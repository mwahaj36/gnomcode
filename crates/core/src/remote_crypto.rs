//! Pure key agreement and authenticated frames for Remote.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use thiserror::Error;
use zeroize::Zeroize;

use crate::EncryptedRemoteFrame;

/// Which endpoint owns a directional key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerRole {
    /// Local daemon.
    Desktop,
    /// Paired mobile client.
    Phone,
}

/// An ephemeral X25519 keypair supplied with random private bytes by the caller.
pub struct RemoteKeyPair {
    private: [u8; 32],
    public: [u8; 32],
}

impl RemoteKeyPair {
    /// Creates an ephemeral keypair from cryptographically random private bytes.
    #[must_use]
    pub fn from_private(private: [u8; 32]) -> Self {
        let public = x25519_dalek::x25519(private, x25519_dalek::X25519_BASEPOINT_BYTES);
        Self { private, public }
    }

    /// Returns the public key exchanged over the untrusted relay.
    #[must_use]
    pub fn public_key(&self) -> [u8; 32] {
        self.public
    }

    /// Returns the public key as lowercase hex for pairing links.
    #[must_use]
    pub fn public_key_hex(&self) -> String {
        encode_hex(&self.public)
    }

    /// Parses a peer public key received over the relay.
    ///
    /// # Errors
    /// Returns [`RemoteCryptoError`] when the key is malformed.
    pub fn parse_public_hex(value: &str) -> Result<[u8; 32], RemoteCryptoError> {
        decode_hex(value)?
            .try_into()
            .map_err(|_| RemoteCryptoError::InvalidPeer)
    }
}

impl Drop for RemoteKeyPair {
    fn drop(&mut self) {
        self.private.zeroize();
    }
}

/// Pairing or frame validation failure.
#[derive(Debug, Error)]
pub enum RemoteCryptoError {
    /// Peer key is invalid or low order.
    #[error("invalid remote public key")]
    InvalidPeer,
    /// Cipher key construction failed.
    #[error("remote key derivation failed")]
    KeyDerivation,
    /// Frame was stale, reordered, malformed, or unauthenticated.
    #[error("invalid encrypted remote frame")]
    InvalidFrame,
    /// Counter exhausted; a fresh pairing is required.
    #[error("remote frame sequence exhausted")]
    SequenceExhausted,
}

/// Directional cipher state for one confirmed pairing.
pub struct RemoteChannel {
    send_key: [u8; 32],
    recv_key: [u8; 32],
    verify_code: String,
    send_seq: u32,
    recv_seq: u32,
}

impl RemoteChannel {
    /// Derives separate directional keys and a short authentication string.
    ///
    /// # Errors
    /// Returns [`RemoteCryptoError`] for a low-order peer key or failed derivation.
    pub fn new(
        local: &RemoteKeyPair,
        peer_public: [u8; 32],
        room_id: &str,
        role: PeerRole,
    ) -> Result<Self, RemoteCryptoError> {
        let shared = x25519_dalek::x25519(local.private, peer_public);
        if shared == [0; 32] {
            return Err(RemoteCryptoError::InvalidPeer);
        }
        let (desktop_public, phone_public) = match role {
            PeerRole::Desktop => (local.public, peer_public),
            PeerRole::Phone => (peer_public, local.public),
        };
        let mut transcript = b"spawnt-remote-v1".to_vec();
        transcript.extend_from_slice(&(room_id.len() as u32).to_be_bytes());
        transcript.extend_from_slice(room_id.as_bytes());
        transcript.extend_from_slice(&desktop_public);
        transcript.extend_from_slice(&phone_public);
        let salt = mac(b"spawnt-remote-salt-v1", &transcript)?;
        let prk = mac(&salt, &shared)?;
        let desktop_to_phone = mac(&prk, b"desktop-to-phone-v1")?;
        let phone_to_desktop = mac(&prk, b"phone-to-desktop-v1")?;
        let sas = mac(&prk, b"verify-v1")?;
        let code = u32::from_be_bytes([sas[0], sas[1], sas[2], sas[3]]) % 1_000_000;
        let (send_key, recv_key) = match role {
            PeerRole::Desktop => (desktop_to_phone, phone_to_desktop),
            PeerRole::Phone => (phone_to_desktop, desktop_to_phone),
        };
        Ok(Self {
            send_key,
            recv_key,
            verify_code: format!("{code:06}"),
            send_seq: 0,
            recv_seq: 0,
        })
    }

    /// Code that both users must compare before any sensitive frame is sent.
    #[must_use]
    pub fn verify_code(&self) -> &str {
        &self.verify_code
    }

    /// Authenticates and encrypts one outbound binary payload.
    ///
    /// # Errors
    /// Returns [`RemoteCryptoError`] if the counter is exhausted or encryption fails.
    pub fn seal(&mut self, plaintext: &[u8]) -> Result<EncryptedRemoteFrame, RemoteCryptoError> {
        let next = self
            .send_seq
            .checked_add(1)
            .ok_or(RemoteCryptoError::SequenceExhausted)?;
        let nonce = sequence_nonce(next);
        let cipher = Aes256Gcm::new_from_slice(&self.send_key)
            .map_err(|_| RemoteCryptoError::KeyDerivation)?;
        let ciphertext = cipher
            .encrypt(&Nonce::from(nonce), plaintext)
            .map_err(|_| RemoteCryptoError::InvalidFrame)?;
        self.send_seq = next;
        Ok(EncryptedRemoteFrame {
            sequence_num: next,
            nonce: encode_hex(&nonce),
            ciphertext: encode_hex(&ciphertext),
        })
    }

    /// Verifies and decrypts the next inbound frame; replay and reordering fail.
    ///
    /// # Errors
    /// Returns [`RemoteCryptoError`] for a malformed or unauthenticated frame.
    pub fn open(&mut self, frame: &EncryptedRemoteFrame) -> Result<Vec<u8>, RemoteCryptoError> {
        let next = self
            .recv_seq
            .checked_add(1)
            .ok_or(RemoteCryptoError::SequenceExhausted)?;
        if frame.sequence_num != next || frame.nonce != encode_hex(&sequence_nonce(next)) {
            return Err(RemoteCryptoError::InvalidFrame);
        }
        let ciphertext = decode_hex(&frame.ciphertext)?;
        let cipher = Aes256Gcm::new_from_slice(&self.recv_key)
            .map_err(|_| RemoteCryptoError::KeyDerivation)?;
        let plaintext = cipher
            .decrypt(&Nonce::from(sequence_nonce(next)), ciphertext.as_slice())
            .map_err(|_| RemoteCryptoError::InvalidFrame)?;
        self.recv_seq = next;
        Ok(plaintext)
    }
}

impl Drop for RemoteChannel {
    fn drop(&mut self) {
        self.send_key.zeroize();
        self.recv_key.zeroize();
    }
}

fn sequence_nonce(sequence: u32) -> [u8; 12] {
    let mut nonce = [0; 12];
    nonce[8..].copy_from_slice(&sequence.to_be_bytes());
    nonce
}

fn mac(key: &[u8], data: &[u8]) -> Result<[u8; 32], RemoteCryptoError> {
    let mut instance = <Hmac<Sha256> as hmac::KeyInit>::new_from_slice(key)
        .map_err(|_| RemoteCryptoError::KeyDerivation)?;
    instance.update(data);
    let mut result = [0; 32];
    result.copy_from_slice(&instance.finalize().into_bytes());
    Ok(result)
}

fn encode_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    out
}

fn decode_hex(input: &str) -> Result<Vec<u8>, RemoteCryptoError> {
    if !input.len().is_multiple_of(2) {
        return Err(RemoteCryptoError::InvalidFrame);
    }
    input
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let hi = (pair[0] as char)
                .to_digit(16)
                .ok_or(RemoteCryptoError::InvalidFrame)?;
            let lo = (pair[1] as char)
                .to_digit(16)
                .ok_or(RemoteCryptoError::InvalidFrame)?;
            Ok(((hi << 4) | lo) as u8)
        })
        .collect()
}

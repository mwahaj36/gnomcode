use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

#[cfg(test)]
const IDLE_SECONDS: i64 = 7 * 24 * 60 * 60;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct DeviceTrust {
    pub room: String,
    pub relay: String,
    pub private: [u8; 32],
    pub secret: [u8; 32],
    pub last_activity: i64,
}

impl DeviceTrust {
    pub(crate) fn valid(&self, now: i64) -> bool {
        self.last_activity <= now.saturating_add(300)
    }

    pub(crate) fn verify(&self, context: &str, proof: &str) -> bool {
        let Ok(bytes) = decode(proof) else {
            return false;
        };
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(&self.secret) else {
            return false;
        };
        mac.update(context.as_bytes());
        mac.verify_slice(&bytes).is_ok()
    }

    pub(crate) fn save_at(&self, path: &std::path::Path) -> Result<(), crate::remote::RemoteError> {
        let bytes =
            serde_json::to_vec(self).map_err(|_| crate::remote::RemoteError::Persistence)?;
        spawnt_platform::write_private_file(path, &bytes)
            .map_err(|_| crate::remote::RemoteError::Persistence)
    }
}

pub(crate) fn path() -> std::path::PathBuf {
    spawnt_platform::spawnt_home().join(obfstr::obfstr!("remote-device.json"))
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode(value: &str) -> Result<Vec<u8>, ()> {
    if value.len() != 64 {
        return Err(());
    }
    (0..value.len())
        .step_by(2)
        .map(|i| {
            value
                .get(i..i + 2)
                .ok_or(())
                .and_then(|part| u8::from_str_radix(part, 16).map_err(|_| ()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trust_survives_inactivity_and_proof_binds_challenge() {
        let mut trust = DeviceTrust {
            room: "room".into(),
            relay: "relay".into(),
            private: [1; 32],
            secret: [2; 32],
            last_activity: 100,
        };
        assert!(trust.valid(100 + IDLE_SECONDS - 1));
        assert!(trust.valid(100 + IDLE_SECONDS));
        assert!(trust.valid(100 + 365 * 24 * 60 * 60));
        trust.last_activity += 60;
        assert!(trust.valid(100 + IDLE_SECONDS));
        let mut mac = Hmac::<Sha256>::new_from_slice(&trust.secret).expect("mac");
        mac.update(b"room:challenge:phone");
        let proof = hex(&mac.finalize().into_bytes());
        assert!(trust.verify("room:challenge:phone", &proof));
        assert!(!trust.verify("room:other:phone", &proof));
        assert!(!trust.verify("room:challenge:phone", "invalid"));
    }
}

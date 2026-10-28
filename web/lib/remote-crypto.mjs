// Browser transport cryptography for the Rust Remote protocol.
const encoder = new TextEncoder();
const decoder = new TextDecoder('utf-8', { fatal: true });

const hex = bytes => Array.from(bytes, byte => byte.toString(16).padStart(2, '0')).join('');
const unhex = value => {
  if (typeof value !== 'string' || value.length % 2 || !/^[0-9a-f]*$/.test(value)) throw new Error('Invalid hex');
  return Uint8Array.from(value.match(/.{2}/g) || [], pair => Number.parseInt(pair, 16));
};
const concat = (...parts) => {
  const bytes = new Uint8Array(parts.reduce((length, part) => length + part.length, 0));
  let offset = 0;
  for (const part of parts) { bytes.set(part, offset); offset += part.length; }
  return bytes;
};
const hmac = async (key, data) => {
  const imported = await crypto.subtle.importKey('raw', key, { name: 'HMAC', hash: 'SHA-256' }, false, ['sign']);
  return new Uint8Array(await crypto.subtle.sign('HMAC', imported, data));
};
const nonceFor = sequence => {
  const nonce = new Uint8Array(12);
  new DataView(nonce.buffer).setUint32(8, sequence);
  return nonce;
};

export async function generateKeyPair() {
  const pair = await crypto.subtle.generateKey('X25519', false, ['deriveBits']);
  const publicHex = hex(new Uint8Array(await crypto.subtle.exportKey('raw', pair.publicKey)));
  return { privateKey: pair.privateKey, publicHex };
}

export async function createChannel(local, peerPublicHex, roomId, role) {
  if (role !== 'desktop' && role !== 'phone') throw new Error('Invalid role');
  const peerBytes = unhex(peerPublicHex);
  if (peerBytes.length !== 32 || !roomId || roomId.length > 128) throw new Error('Invalid pairing');
  const peerKey = await crypto.subtle.importKey('raw', peerBytes, 'X25519', false, []);
  const shared = new Uint8Array(await crypto.subtle.deriveBits({ name: 'X25519', public: peerKey }, local.privateKey, 256));
  if (shared.every(byte => byte === 0)) throw new Error('Invalid peer');
  const room = encoder.encode(roomId);
  const size = new Uint8Array(4);
  new DataView(size.buffer).setUint32(0, room.length);
  const localPublic = unhex(local.publicHex);
  const desktopPublic = role === 'desktop' ? localPublic : peerBytes;
  const phonePublic = role === 'phone' ? localPublic : peerBytes;
  const transcript = concat(encoder.encode('spawnt-remote-v1'), size, room, desktopPublic, phonePublic);
  const salt = await hmac(encoder.encode('spawnt-remote-salt-v1'), transcript);
  const prk = await hmac(salt, shared);
  const d2p = await hmac(prk, encoder.encode('desktop-to-phone-v1'));
  const p2d = await hmac(prk, encoder.encode('phone-to-desktop-v1'));
  const sas = await hmac(prk, encoder.encode('verify-v1'));
  const verifyCode = String(new DataView(sas.buffer).getUint32(0) % 1_000_000).padStart(6, '0');
  const sendKey = await crypto.subtle.importKey('raw', role === 'desktop' ? d2p : p2d, 'AES-GCM', false, ['encrypt']);
  const recvKey = await crypto.subtle.importKey('raw', role === 'desktop' ? p2d : d2p, 'AES-GCM', false, ['decrypt']);
  let sent = 0;
  let received = 0;
  return {
    verifyCode,
    async seal(value) {
      if (sent === 0xffffffff) throw new Error('Sequence exhausted');
      const sequence_num = sent + 1;
      sent = sequence_num;
      const nonce = nonceFor(sequence_num);
      const ciphertext = await crypto.subtle.encrypt({ name: 'AES-GCM', iv: nonce }, sendKey, encoder.encode(JSON.stringify(value)));
      return { sequence_num, nonce: hex(nonce), ciphertext: hex(new Uint8Array(ciphertext)) };
    },
    async open(frame) {
      if (received === 0xffffffff || frame.sequence_num !== received + 1 || frame.nonce !== hex(nonceFor(frame.sequence_num))) throw new Error('Invalid frame order');
      const plaintext = await crypto.subtle.decrypt({ name: 'AES-GCM', iv: nonceFor(frame.sequence_num) }, recvKey, unhex(frame.ciphertext));
      const value = JSON.parse(decoder.decode(plaintext));
      received = frame.sequence_num;
      return value;
    },
  };
}

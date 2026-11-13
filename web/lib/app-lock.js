export const APP_LOCK_STORAGE_KEY = 'gnomcode-pwa-lock-v1';
export const APP_LOCK_SESSION_KEY = 'gnomcode-pwa-lock-session-v1';
export const MAX_LOCK_ATTEMPTS = 10;
export const LOCKOUT_DURATION_MS = 10 * 60 * 1000;
const PBKDF2_ITERATIONS = 210000;

export function emptyAppLockState() {
  return {
    mode: 'off',
    idleMinutes: 5,
    failedAttempts: 0,
    lockedUntil: 0,
    pinSalt: '',
    pinHash: '',
    credentialId: '',
  };
}

function toBase64Url(bytes) {
  let binary = '';
  for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function fromBase64Url(value) {
  const padded = String(value || '').replace(/-/g, '+').replace(/_/g, '/').padEnd(Math.ceil(String(value || '').length / 4) * 4, '=');
  const binary = atob(padded);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

function constantTimeEqual(a, b) {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a[i] ^ b[i];
  return diff === 0;
}

async function derivePinDigest(pin, saltBytes) {
  const subtle = globalThis.crypto?.subtle;
  if (!subtle) throw new Error('WebCrypto is unavailable in this context.');
  const baseKey = await subtle.importKey('raw', new TextEncoder().encode(pin), 'PBKDF2', false, ['deriveBits']);
  const bits = await subtle.deriveBits({ name: 'PBKDF2', hash: 'SHA-256', salt: saltBytes, iterations: PBKDF2_ITERATIONS }, baseKey, 256);
  return new Uint8Array(bits);
}

function assertPinFormat(pin) {
  if (!/^\d{4,8}$/.test(String(pin || ''))) {
    throw new Error('PIN must be 4 to 8 digits.');
  }
}

export async function enrollPinLock(pin, options = {}) {
  if (!options.unlocked) throw new Error('Unlock the app before changing app lock settings.');
  assertPinFormat(pin);
  const salt = new Uint8Array(16);
  globalThis.crypto.getRandomValues(salt);
  const digest = await derivePinDigest(String(pin), salt);
  return {
    ...emptyAppLockState(),
    mode: 'pin',
    idleMinutes: Number.isFinite(options.idleMinutes) ? Math.max(0, Number(options.idleMinutes)) : 5,
    pinSalt: toBase64Url(salt),
    pinHash: toBase64Url(digest),
  };
}

export async function unlockWithPin(state, pin, now = Date.now()) {
  const current = { ...emptyAppLockState(), ...(state || {}) };
  if (current.lockedUntil && now < current.lockedUntil) {
    return { unlocked: false, lockedOut: true, remainingMs: current.lockedUntil - now, nextState: current };
  }
  if (!/^\d{4,8}$/.test(String(pin || '')) || !current.pinSalt || !current.pinHash) {
    const failed = (current.failedAttempts || 0) + 1;
    const lockedOut = failed >= MAX_LOCK_ATTEMPTS;
    const nextState = { ...current, failedAttempts: failed, lockedUntil: lockedOut ? now + LOCKOUT_DURATION_MS : 0 };
    return { unlocked: false, lockedOut, remainingMs: lockedOut ? LOCKOUT_DURATION_MS : 0, nextState };
  }
  const actual = await derivePinDigest(String(pin), fromBase64Url(current.pinSalt));
  const expected = fromBase64Url(current.pinHash);
  if (constantTimeEqual(actual, expected)) {
    return { unlocked: true, lockedOut: false, remainingMs: 0, nextState: { ...current, failedAttempts: 0, lockedUntil: 0 } };
  }
  const failed = (current.failedAttempts || 0) + 1;
  const lockedOut = failed >= MAX_LOCK_ATTEMPTS;
  const nextState = { ...current, failedAttempts: failed, lockedUntil: lockedOut ? now + LOCKOUT_DURATION_MS : 0 };
  return { unlocked: false, lockedOut, remainingMs: lockedOut ? LOCKOUT_DURATION_MS : 0, nextState };
}

export function isWebAuthnSupported() {
  return typeof window !== 'undefined'
    && Boolean(window.isSecureContext)
    && typeof PublicKeyCredential !== 'undefined'
    && Boolean(navigator.credentials?.create)
    && Boolean(navigator.credentials?.get);
}

export async function enrollPasskeyLock(options = {}) {
  if (!options.unlocked) throw new Error('Unlock the app before changing app lock settings.');
  if (!isWebAuthnSupported()) throw new Error('Passkeys require a secure HTTPS or localhost origin.');
  const challenge = new Uint8Array(32);
  const userId = new Uint8Array(16);
  globalThis.crypto.getRandomValues(challenge);
  globalThis.crypto.getRandomValues(userId);
  const created = await navigator.credentials.create({
    publicKey: {
      challenge,
      rp: { name: 'Gnomcode' },
      user: { id: userId, name: options.label || 'gnomcode-device-lock', displayName: 'Gnomcode Device Lock' },
      pubKeyCredParams: [{ alg: -7, type: 'public-key' }, { alg: -257, type: 'public-key' }],
      authenticatorSelection: { authenticatorAttachment: 'platform', userVerification: 'required', residentKey: 'preferred' },
      timeout: 60000,
    },
  });
  if (!created || !('rawId' in created)) throw new Error('Passkey creation was cancelled.');
  return {
    ...emptyAppLockState(),
    mode: 'passkey',
    idleMinutes: Number.isFinite(options.idleMinutes) ? Math.max(0, Number(options.idleMinutes)) : 5,
    credentialId: toBase64Url(new Uint8Array(created.rawId)),
  };
}

export async function unlockWithPasskey(state, now = Date.now()) {
  const current = { ...emptyAppLockState(), ...(state || {}) };
  if (current.lockedUntil && now < current.lockedUntil) {
    return { unlocked: false, lockedOut: true, remainingMs: current.lockedUntil - now, nextState: current };
  }
  if (!isWebAuthnSupported() || !current.credentialId) {
    throw new Error('Passkey unlock is unavailable on this browser.');
  }
  const challenge = new Uint8Array(32);
  globalThis.crypto.getRandomValues(challenge);
  const assertion = await navigator.credentials.get({
    publicKey: {
      challenge,
      allowCredentials: [{ id: fromBase64Url(current.credentialId), type: 'public-key' }],
      userVerification: 'required',
      timeout: 60000,
    },
  });
  if (!assertion || !('response' in assertion)) {
    const failed = (current.failedAttempts || 0) + 1;
    const lockedOut = failed >= MAX_LOCK_ATTEMPTS;
    return { unlocked: false, lockedOut, remainingMs: lockedOut ? LOCKOUT_DURATION_MS : 0, nextState: { ...current, failedAttempts: failed, lockedUntil: lockedOut ? now + LOCKOUT_DURATION_MS : 0 } };
  }
  const resp = assertion.response;
  const clientData = JSON.parse(new TextDecoder().decode(resp.clientDataJSON));
  if (clientData.type !== 'webauthn.get' || clientData.origin !== window.location.origin || !(resp.authenticatorData?.byteLength > 0)) {
    throw new Error('Passkey verification failed origin check.');
  }
  return { unlocked: true, lockedOut: false, remainingMs: 0, nextState: { ...current, failedAttempts: 0, lockedUntil: 0 } };
}

export function loadAppLockState() {
  try {
    const raw = localStorage.getItem(APP_LOCK_STORAGE_KEY);
    if (!raw) return emptyAppLockState();
    return { ...emptyAppLockState(), ...JSON.parse(raw) };
  } catch {
    return emptyAppLockState();
  }
}

export function saveAppLockState(state) {
  try {
    localStorage.setItem(APP_LOCK_STORAGE_KEY, JSON.stringify(state));
  } catch {}
}

export function clearAppLockState() {
  try {
    localStorage.removeItem(APP_LOCK_STORAGE_KEY);
    sessionStorage.removeItem(APP_LOCK_SESSION_KEY);
  } catch {}
}

export function markSessionUnlocked(now = Date.now()) {
  try { sessionStorage.setItem(APP_LOCK_SESSION_KEY, String(now)); } catch {}
}

export function lockSessionNow() {
  try { sessionStorage.removeItem(APP_LOCK_SESSION_KEY); } catch {}
}

export function isSessionUnlocked(state, now = Date.now()) {
  if (!state || state.mode === 'off') return true;
  if (state.lockedUntil && now < state.lockedUntil) return false;
  try {
    const raw = sessionStorage.getItem(APP_LOCK_SESSION_KEY);
    const unlockedAt = Number(raw);
    if (!Number.isFinite(unlockedAt) || unlockedAt <= 0) return false;
    if (state.idleMinutes > 0 && now - unlockedAt > state.idleMinutes * 60 * 1000) return false;
    return true;
  } catch {
    return false;
  }
}

export const APP_LOCK_STORAGE_KEY: string;
export const APP_LOCK_SESSION_KEY: string;
export const MAX_LOCK_ATTEMPTS: number;
export const LOCKOUT_DURATION_MS: number;

export type AppLockMode = 'off' | 'pin' | 'passkey';
export type AppLockState = {
  mode: AppLockMode;
  idleMinutes: number;
  failedAttempts: number;
  lockedUntil: number;
  pinSalt: string;
  pinHash: string;
  credentialId: string;
};

export function emptyAppLockState(): AppLockState;
export function enrollPinLock(pin: string, options?: { unlocked?: boolean; idleMinutes?: number }): Promise<AppLockState>;
export function unlockWithPin(state: AppLockState, pin: string, now?: number): Promise<{ unlocked: boolean; lockedOut: boolean; remainingMs: number; nextState: AppLockState }>;
export function isWebAuthnSupported(): boolean;
export function enrollPasskeyLock(options?: { unlocked?: boolean; idleMinutes?: number; label?: string }): Promise<AppLockState>;
export function unlockWithPasskey(state: AppLockState, now?: number): Promise<{ unlocked: boolean; lockedOut: boolean; remainingMs: number; nextState: AppLockState }>;
export function loadAppLockState(): AppLockState;
export function saveAppLockState(state: AppLockState): void;
export function clearAppLockState(): void;
export function markSessionUnlocked(now?: number): void;
export function lockSessionNow(): void;
export function isSessionUnlocked(state: AppLockState, now?: number): boolean;

# Remote Access Security Threat Model & Cryptographic Specification

- **Document:** 0001-remote-threat-model
- **Date:** 2026-10-28
- **Status:** Preliminary design; independent security review pending

## 1. System Overview

Remote access enables users to securely monitor active coding sessions, read redacted chat history, approve tool permissions, and send prompts from a mobile browser (PWA) to their local desktop daemon.

```
+----------------+          +-------------------------+          +----------------+
|                |  E2EE    |   Zero-Knowledge Relay  |  E2EE    |                |
|   Phone PWA    |<-------->|   (Cloudflare Worker +  |<-------->| Desktop Daemon |
|  (WebCrypto)   |  WS TLS  |     Durable Object)     |  WS TLS  |  (127.0.0.1)   |
+----------------+          +-------------------------+          +----------------+
```

## 2. Threat Analysis & Countermeasures

| Threat | Risk Level | Countermeasure / Architectural Defense |
| :--- | :--- | :--- |
| **Relay Server Compromise** | Critical | End-to-End Encryption (E2EE) with authenticated encryption (AEAD: ChaCha20-Poly1305 / AES-GCM). The relay acts solely as a dumb frame broker; it possesses no decryption keys and stores no message contents. |
| **Man-In-The-Middle (MITM) on Pairing** | High | Ephemeral pairing rooms combined with an out-of-band Short Authentication String (SAS) 6-digit verification code confirmed simultaneously on desktop and phone screens. |
| **Malicious Pairing Attempt / Scanning** | Medium | Room IDs use cryptographically secure random bytes (CSPRNG). Unconfirmed pairing expires after five minutes; revocation closes the room and a new attempt creates fresh keys. |
| **Stolen or Unattended Phone** | High | Desktop revocation closes the active relay connection; a replacement pairing creates fresh keys. |
| **Replay Attacks** | High | Per-message monotonically increasing sequence numbers and unique nonces/IVs for each sealed frame. Stale or duplicate frames are rejected. |
| **Prompt Injection Leading to Destructive Execution** | Critical | The remote phone interface strictly allows status inspection, prompt submission, and low-risk approvals. It **never** allows arbitrary file system browsing, command line execution, or permission bypasses (`--dangerously-skip-permissions`, `--yolo`). |

## 3. Cryptographic Primitives

- **Key Agreement:** X25519 (ECDH)
- **Symmetric Cipher:** AES-256-GCM with separate directional keys
- **Verification Code:** HMAC-SHA256 over the room, roles, and both public keys, reduced to six decimal digits for user comparison
- **Frame Structure:** JSON envelope with monotonic sequence number and authenticated ciphertext; the nonce is derived from the directional sequence

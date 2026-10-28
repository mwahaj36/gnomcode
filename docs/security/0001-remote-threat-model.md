# Remote Access Security Threat Model & Cryptographic Specification

- **Document:** 0001-remote-threat-model
- **Date:** 2026-10-28
- **Status:** Approved for Implementation

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
| **Malicious Pairing Attempt / Scanning** | Medium | One-time room IDs generated with cryptographically secure random bytes (CSPRNG), short time-to-live (5 minutes), and automatic room rotation upon completion or expiration. |
| **Stolen or Unattended Phone** | High | Fast session revocation directly from the desktop tray/UI, rotating encryption keys and disconnecting the mobile client immediately. |
| **Replay Attacks** | High | Per-message monotonically increasing sequence numbers and unique nonces/IVs for each sealed frame. Stale or duplicate frames are rejected. |
| **Prompt Injection Leading to Destructive Execution** | Critical | The remote phone interface strictly allows status inspection, prompt submission, and low-risk approvals. It **never** allows arbitrary file system browsing, command line execution, or permission bypasses (`--dangerously-skip-permissions`, `--yolo`). |

## 3. Cryptographic Primitives

- **Key Agreement:** X25519 (ECDH)
- **Symmetric Cipher:** ChaCha20-Poly1305 or AES-256-GCM
- **Verification Hash:** SHA-256 truncated to 6 decimal digits for user visual verification
- **Frame Structure:** `[4 bytes: sequence_num] || [12 bytes: nonce] || [N bytes: ciphertext] || [16 bytes: auth_tag]`

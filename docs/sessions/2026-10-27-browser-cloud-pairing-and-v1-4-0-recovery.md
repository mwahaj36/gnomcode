## 2026-10-27 — Browser cloud pairing and v1.4.0 recovery

- Goal: remove manual session-file copying, repair the Windows console loading failure, expose cloud presence on native clients and publish working update downloads.
- Root cause reproduced: duplicate `url` declaration in embedded export JavaScript prevented the entire dashboard script from executing. `check-console.mjs` failed on the shipped script and passes with the corrected variable.
- Added isolated browser GitHub PKCE/email login, explicit code comparison and encrypted one-time approval RPCs. User applied the v4 migration; a live anonymous consume probe returned HTTP 200/null, without accessing account data.
- PostgreSQL checks cover repeated migrations, denied anonymous approvals/direct reads, capability verification, one-time consumption and replay rejection. Browser checks exercise the actual console, unauthorized states, pairing initiation/cancel and encrypted independent login transfer; external auth transport is stubbed.
- Native consoles merge remote cloud presence with paging/dedup. Tray separates cloud connectivity from local active work and reports daemon failure. Cloud daemons no longer overwrite desktop summaries. Atomic private credential writes serialize with refresh/disconnect; OS-held daemon locks protect metadata.
- New dependencies: RustCrypto aes-gcm for authenticated WebCrypto-compatible encryption, existing base64 crate as a direct dependency, and existing libc dependency for the Unix flock system call confined to platform code.
- Scope follows Ponytail Ultra: account pairing and reported failures only; no provider billing guesses, hidden-log discovery or generic device-management framework.
- Native Windows GUI and macOS hardware are unavailable in this cloud environment. Windows executable behavior is checked under Wine; macOS packages remain unavailable rather than relabeling old artifacts.

- Verification: 65 Rust tests, Clippy, cargo-deny and MSRV 1.87 passed. Both embedded PostgreSQL migration suites and real-browser email/PKCE approval checks passed. Windows PE/installer payload and PowerShell updater checks passed; Linux/ARM64 versions and Wine Windows collector smoke checks passed. Unix updater rejects corrupt manifests without replacing the existing install. Final Linux x86_64 packaging explicitly selects static OpenSSL to avoid an accidental host shared-library dependency.


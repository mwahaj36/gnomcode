## 2026-10-27 — Automatic collection and web deployment

- Corrected manual startup: the daemon now starts cloud presence automatically from detected Codex metadata or explicit provider/runtime identity bindings for every supported agent. It reuses the existing paired account. Standalone collection remains available for metadata usage imports.
- Serialized in-process account refresh between desktop sync and the automatic collector. Tests isolate the pairing path to a temporary directory and verify automatic registration without a manual collector command.
- Updated the feature branch with current main (v1.2.1), preserving its latest-context-token fix, platform locking, license hardening and daemon improvements. Adjusted the newly enabled MSRV-compatible Clippy integer-multiple lint without changing license verification behavior.
- Pushed the feature branch and created draft PR #38. The connected Vercel Git integration reported a successful preview deployment. Production database migration and initial account pairing remain unconfigured here.

### Deployment follow-up

- Confirmed daemon automatic registration against the actual current cloud runtime. The local gate passed 57 tests and Rust 1.87 workspace checking passed after merging v1.2.1.
- Vercel reports the updated branch preview Ready. GitHub CI dispatch was accepted but every job stopped before runner steps because account payments/spending limits blocked execution; Windows/macOS native checks remain pending.
- The user applied the v3 Supabase migration. A read-only schema probe returned permission denied for the new view under anonymous credentials, confirming it exists and does not grant anonymous access. Authenticated account sync cannot be tested without pairing.
- Promoting the web changes through PR #38 and the existing Vercel main-branch deployment integration; production readiness is checked separately from preview status.


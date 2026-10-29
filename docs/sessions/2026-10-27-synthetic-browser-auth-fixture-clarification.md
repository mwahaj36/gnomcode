## 2026-10-27 — Synthetic browser-auth fixture clarification

GitGuardian flagged the hardcoded JWT header/signature in the browser integration
fixture in PR #44 as a Generic High Entropy Secret. The token is synthetic, uses
an all-zero test account and an invalid signature, and is accepted only by mocked
auth transport. Generate its header and signature from clearly named test values
instead. No real account credential was committed or needs rotating. Production
code and packaged binaries are unchanged. Rerun the browser flow with this fixture.


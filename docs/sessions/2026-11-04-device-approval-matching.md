# 2026-11-04 - Remote device approval matching

## Goal and outcome
Investigate app issues and prevent a pending browser enrollment from retrying when another browser is approved for the same desktop.

## Root cause and decision
The waiting state identified only the desktop, while the account directory can contain multiple device requests for it. Track the enrollment request ID as well and resume only for its own unrevoked approval. No protocol or schema change is needed.

## Verification
- Regression test failed with the previous desktop-only match and passed with request matching.
- Web tests, typecheck, lint and production build passed; lint reports four existing React hook warnings.
- Desktop tests, typecheck and lint passed.
- Windows Rust quality gate passed; cargo-deny reported duplicate dependency warnings.
- Live multi-device pairing was not exercised.

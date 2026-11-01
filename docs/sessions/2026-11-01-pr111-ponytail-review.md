# PR #111 ponytail ultra review

Date: 2026-11-01

What this change does: PR #111 improves navigation, keyboard focus, dialog names, form labels, password/token visibility and account failure feedback. It keeps the existing runtime and data model.

Reviewed head: `ac302d33825685d4b0a30cc098250ae0c8521b60`. Merged as `8057f2c` at the user's explicit request. No blocking product bugs found. Assumed the existing local desktop and paired-browser usage, with one signed-in account per browser profile.

## Nice to have

1. **Keep the UI fix separate from downloaded tooling** (`.agents/skills/unslop/`).
   - What this is: The PR includes an additional design skill, scripts and reference images.
   - Problem: These add 1,290 text lines and roughly 1.3 MB of images without being needed by the shipped UI fixes.
   - Fix: Keep this tooling in a separate tooling change when it next changes. No product abstraction or dependency is needed.
   - If we skip it: The app works, but reviewers and future maintainers carry extra repository scope. Kept intact to preserve the other agent's work; this is not a merge blocker.

Verdict: Ship.

Lean: 1,290 lines plus reference images could be separated from the product patch; no production code deletion identified.

Checked the diff and connected navigation, auth lifecycle, settings loads, dialog effects, CSS and test fixtures. Re-ran typecheck, 18 web tests, production build, desktop browser interactions (including navigation keys and token visibility), account visibility/task switching and failed global sign-out feedback, and mobile console navigation. Verified installed Supabase client sign-out source clears local state even when remote revocation fails; no new credentials, permissions, persistence or cloud data fields were introduced. Existing Rust gate results were supplemented by another full gate on the combined model-picker integration.

Not checked: Physical devices, platform screen readers, live external sign-in/revocation or running the optional downloaded design scripts. The combined encrypted Remote flow is recorded in the model-picker session.

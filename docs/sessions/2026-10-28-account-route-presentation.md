## 2026-10-28 — account route presentation

- Goal: align Auth with the spawnt visual language while retaining GitHub and email sign-in, account creation, password reset, and local-console access.
- Outcome: added keyboard-friendly native mode toggles, clear field labels and focus states, responsive square forms, readable GitHub sign-in, and accessible status/error messaging. OAuth now prevents duplicate clicks and reports a provider-start failure. Existing Supabase calls and callback handling remain in place.
- Verification: Next.js production build and `git diff --check` passed. Chromium review covered light/dark themes, the sign-in/create/reset content states, accessible control names, and the visible GitHub action. No credentials were entered and no external sign-in was submitted; narrow viewport and end-to-end Supabase flows remain pending.

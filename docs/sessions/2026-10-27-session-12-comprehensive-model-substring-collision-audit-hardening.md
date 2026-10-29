## 2026-10-27 — Session 12: Comprehensive Model Substring Collision Audit & Hardening

### 1. Goals & User Requests
- Audit entire model pricing catalog and matching logic to ensure no other model suffers from false substring collisions like `"gemini" -> "mini"`.
- Prevent generic keywords (`o3`, `pro`, `ultra`, `mini`, `4o`, `gpt-5`) from capturing unrelated models or tools (`turbo3`, `combo3`, `prompt-eval`, `production-app`, `protocol-v1`, `minimax-01`, `ultravox-v0.5`).
- Fix `gpt-5.6-luna` version ordering collision (`"5.6-luna"` contained `"6-luna"`).
- Implement bounded word matching (`has_bounded_word`) in Rust with dot, hyphen, space, and non-alphanumeric delimiter support.
- Add regression test asserting that all 39 catalog models resolve cleanly and 14 potential collision strings evaluate strictly to `None` (Unknown Trust Level, $0 cost).
- Recompile native release binary `spawnt.exe`, rebuild `Spawnt-Setup.exe` with verified payload, and push updates.

### 2. Implementation & Root Causes
| Area | Collision / Gap | Resolution |
| :--- | :--- | :--- |
| **OpenAI `o3` Catch-all** | `lower.contains("o3")` matched any word containing the letters `o` and `3` (e.g. `turbo3`, `combo3`, `macro3`, `echo3`) and billed them as OpenAI o3 ($30/$120 per Mtok). | Replaced with `has_bounded_word(&lower, "o3")`. |
| **Gemini `pro` Catch-all** | `lower.contains("pro")` at line 655 matched any word containing `pro` (e.g. `prompt-eval`, `production-app`, `protocol-v1`, `profile`, `program`). | Removed unbounded `contains("pro")`. Gemini Pro is matched strictly under `gemini` or `argon`. |
| **OpenAI `5.6-luna` vs `6-luna`** | `lower.contains("6-luna")` was evaluated before `5.6-luna`. Since `"5.6-luna"` contains the substring `"6-luna"`, `5.6-luna` was misattributed to `6-luna`. | Prioritized `5.6-luna` before `6-luna` and added dot-boundary checks in `has_bounded_word`. |
| **Generic `mini` / `nano`** | `lower.contains("mini")` matched `minimax`, `admin-model`, and `gemini`. | Replaced with `lower.contains("4o-mini") \|\| has_bounded_word(&lower, "mini") \|\| has_bounded_word(&lower, "nano")`. |
| **Generic `ultra`** | `lower.contains("ultra")` matched audio/vision tools like `ultravox-v0.5`. | Replaced with `(lower.contains("gemini") && lower.contains("ultra")) \|\| has_bounded_word(&lower, "ultra")`. |

### 3. Verification & Packaging
- Rust Quality Gate: 58 tests passed across all 9 crates with 0 failures, 0 clippy warnings, clean formatting.
- Comprehensive Test: `model_lookup_resolves_all_catalog_families_and_rejects_false_substring_collisions` verified all 39 catalog models and 14 negative test cases.
- Native Windows Installer: `Spawnt-Setup.exe` compiled and verified with `check-installer-payload.cs`.
- Local Daemon: Restarted with hardened binary; live session context correctly shows `200.5k / 1,000,000 (20.0%)` with healthy green progress bar.

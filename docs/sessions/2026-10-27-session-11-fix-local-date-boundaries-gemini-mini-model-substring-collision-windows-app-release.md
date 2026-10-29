## 2026-10-27 — Session 11: Fix Local Date Boundaries, Gemini/Mini Model Substring Collision & Windows App Release

### 1. Goals & User Requests
- Resolve date boundary issue: "Today" filter was dropping nighttime spend and jumping between dates upon restart because it used UTC midnight (`00:00:00+00:00` = 05:00 AM local time).
- Resolve Gemini Flash model attribution: evaluate why Gemini 3.8 Flash (Medium) in Antigravity was showing a saturated red context bar (`148.5k / 128.0k (100%)`) and inflated cost ($0.91).
- Compile fresh native release binary `spawnt.exe`, build `Spawnt-Setup.exe` with verified installer payload, and push updates.

### 2. Implementation & Root Causes
| Area | Problem / Gap | Resolution |
| :--- | :--- | :--- |
| **Store / Timeline Date Boundaries** | "Today" calculated start-of-day using `Utc::now().date_naive().and_hms_opt(0,0,0)`, which evaluates to UTC midnight (05:00 AM in UTC+5). Work done between 00:00 and 05:00 AM local was grouped under "yesterday", and upon 05:00 AM rollover, "Today" dropped to $0.smth. Explicit `from_ts` was also ignored for "today", "7d", and "30d". | Updated `generate_usage_report_timeline` in `crates/store/src/lib.rs` to compute start-of-day using `chrono::Local::now().date_naive().and_hms_opt(0,0,0)` converted to UTC, and prioritize explicit `from_ts` across all range queries. Added unit test. |
| **Model Matching Collision** | `lookup_model_pricing` matched `lower.contains("mini")` under OpenAI before checking Google Gemini models. Since `"gemini"` contains `"mini"`, `Gemini 3.8 Flash (Medium)` was mistakenly resolved as `OPENAI_MINI` (128k context window, $0.15/$0.075/$0.60 pricing). | Reordered model catalog in `crates/core/src/pricing.rs` so Google Gemini models are resolved before OpenAI reasoning/flagship generic `"mini"` checks. Added unit test assertion. |
| **Console & Web Range Filtering** | `console.html` and `web/app/console/page.tsx` lacked precise local date span display, showing awkward spans like `Today (Oct 26 — Oct 27)`. | Formatted date spans to condense identical local start/end dates (`Today (Oct 27, 2026)`), and passed browser-local start-of-day ISO string in `from` query param. |
| **CLI Range Filter** | `spawnt report` lacked `--range` CLI argument documented in web docs. | Added `--range` option to `Report` subcommand in `crates/cli/src/main.rs` routing to `generate_usage_report_timeline`. |

### 3. Verification & Packaging
- Rust Quality Gate: 57 tests passed across all 9 crates with 0 failures, 0 clippy warnings, clean formatting.
- Native Windows Compilation: Built `target/release/spawnt.exe` (5.3 MB).
- Native Installer: Compiled `Spawnt-Setup.exe` via `csc.exe` with embedded payload verification passing cleanly.
- Web Production Build: Next.js 15.5 production build verified (all 14 static routes compiled cleanly).
- Live verification: `spawnt report --range today` reports exact local day spend ($28.67) and Gemini Flash 1M context window.

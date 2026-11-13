# Handoff original-goal repair

Date: 2026-11-13. Owner: codex-release.

## Goals, cause and decision

The newest-50 fetch discarded the original request before formatting; newest-first budget consumption could also discard it. Session-list pagination could hide an older title. Read original retained request/title independently, reserve their bounded redacted content first and fill remaining space from the newest page. No pins/heuristics/persistent copy; deletion stays authoritative. Also preserve goal/title in legacy summary packets. Newly introduced SQLite work and the existing preparation snapshot run off the async executor.

## Tests and verification

Failing-first long-session eval regression reproduced missing java.time, leap days identical and src/calendar.kt. Regression now passes. Tests cover 5,000+ events, provider switches, deletion, title lookup, redaction, first requests longer than 4,000 characters, newest result retention and total budget. Full fmt/clippy -D warnings and 501 workspace tests passed with CARGO_INCREMENTAL=0, dev/test debug symbols disabled. No Cargo.lock/dependency changes. Remapped stale artifacts from sharing a target with another worktree were cleared; final A gate ran in its exclusive target. No live providers/phones or installer acceptance claimed.

## Before: real CLI, isolated home, no-save

```text
=== HANDOFF QUALITY EVALS (estimated, synthetic cases) ===
  [PASS] 100.0%  dark-mode-css      codex -> antigravity  (packet 87.5%)
         goals 2/2, files 2/2, open tasks 2/2, 809/18000 chars
  [PASS] 100.0%  antigravity-to-claude antigravity -> claude-code  (packet 75.0%)
         goals 2/2, files 2/2, open tasks 2/2, 825/18000 chars
  [PASS] 100.0%  login-redirect     claude-code -> codex  (packet 66.7%)
         goals 2/2, files 3/3, open tasks 3/3, 1008/18000 chars
  [FAIL]  47.5%  long-session       antigravity -> claude-code  (packet 35.0%)
         goals 0/2, files 1/2, open tasks 1/1, 10903/18000 chars
         missing goals: java.time; leap days identical
         missing files: src/calendar.kt
  [PASS] 100.0%  orm-migration      codex -> claude-code  (packet 75.0%)
         goals 2/2, files 2/2, open tasks 2/2, 862/18000 chars
  [PASS] 100.0%  pricing-table      antigravity -> codex  (packet 75.0%)
         goals 3/3, files 2/2, open tasks 2/2, 861/18000 chars
  [PASS] 100.0%  secret-in-prompt   claude-code -> antigravity  (packet 75.0%)
         goals 2/2, files 1/1, open tasks 1/1, 839/18000 chars
  [PASS] 100.0%  small-budget       codex -> antigravity  (packet 75.0%)
         goals 2/2, files 1/1, open tasks 2/2, 525/3000 chars
  [PASS] 100.0%  tool-heavy         claude-code -> antigravity  (packet 75.0%)
         goals 3/3, files 3/3, open tasks 2/2, 672/18000 chars

By receiving provider:
  antigravity  4/4 passed (100.0%), mean score 100.0%
  claude-code  2/3 passed (66.7%), mean score 82.5%
  codex        2/2 passed (100.0%), mean score 100.0%

Total: 8/9 passed (88.9%), mean score 94.2% [EST]
```

## After: real CLI, isolated home, no-save

```text
=== HANDOFF QUALITY EVALS (estimated, synthetic cases) ===
  [PASS] 100.0%  dark-mode-css      codex -> antigravity  (packet 87.5%)
         goals 2/2, files 2/2, open tasks 2/2, 809/18000 chars
  [PASS] 100.0%  antigravity-to-claude antigravity -> claude-code  (packet 75.0%)
         goals 2/2, files 2/2, open tasks 2/2, 825/18000 chars
  [PASS] 100.0%  login-redirect     claude-code -> codex  (packet 66.7%)
         goals 2/2, files 3/3, open tasks 3/3, 1008/18000 chars
  [PASS] 100.0%  long-session       antigravity -> claude-code  (packet 75.0%)
         goals 2/2, files 2/2, open tasks 1/1, 11052/18000 chars
  [PASS] 100.0%  orm-migration      codex -> claude-code  (packet 75.0%)
         goals 2/2, files 2/2, open tasks 2/2, 862/18000 chars
  [PASS] 100.0%  pricing-table      antigravity -> codex  (packet 75.0%)
         goals 3/3, files 2/2, open tasks 2/2, 861/18000 chars
  [PASS] 100.0%  secret-in-prompt   claude-code -> antigravity  (packet 75.0%)
         goals 2/2, files 1/1, open tasks 1/1, 839/18000 chars
  [PASS] 100.0%  small-budget       codex -> antigravity  (packet 75.0%)
         goals 2/2, files 1/1, open tasks 2/2, 525/3000 chars
  [PASS] 100.0%  tool-heavy         claude-code -> antigravity  (packet 75.0%)
         goals 3/3, files 3/3, open tasks 2/2, 672/18000 chars

By receiving provider:
  antigravity  4/4 passed (100.0%), mean score 100.0%
  claude-code  3/3 passed (100.0%), mean score 100.0%
  codex        2/2 passed (100.0%), mean score 100.0%

Total: 9/9 passed (100.0%), mean score 100.0% [EST]
```

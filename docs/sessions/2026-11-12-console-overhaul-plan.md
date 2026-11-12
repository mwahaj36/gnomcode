# 2026-11-12 - Console overhaul plan

## Goals
- Plan a CodexBar-style overhaul of the local console: provider limit cards, a GitHub-style token activity grid instead of the bar chart, a Home board, and navigation reduced from seven places to four.
- Make the plan explicit enough for any implementing agent to follow step by step.

## Outcomes
- Added `docs/design/console-overhaul-plan.md`: design rules, tokens, component markup and CSS, view specs, Rust data contracts (quota state and pace, activity levels and heatmap, Home grouping, provider overview), seven phased PRs, tests, verification and SPEC text per phase.

## Decisions
- Quota state, pace, activity levels, streaks and Home grouping are computed in Rust (invariant 8); JavaScript only formats.
- Heat ramp is blue, not brand red, because red already means an exhausted limit.
- Rail becomes a toggle-only grid column to fix the hover overlay (audit D8).

## Problems and root causes
- None; planning only.

## Verification
- Documentation only. No code changed; SPEC changes land with each implementation phase as listed in the plan.

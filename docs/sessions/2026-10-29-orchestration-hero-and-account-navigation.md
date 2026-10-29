# 2026-10-29 — Orchestration hero and account navigation

## Goals and owner feedback

Owner screenshots identified the support strip incorrectly placed inside hero copy and the main graphic still communicating a reporting dashboard. Requested an independent moving strip, an agent-agnostic question with every supported switching agent named in its answer, and prominent red Log in/Console navigation on desktop and phone. Download becomes secondary on desktop and moves into the phone menu.

## Changes and decisions

- Replaced the hero overview with a code-native illustrative conversation: task, agent handoff/context briefing and encrypted phone instruction. Reporting remains in the existing lower tour. No invented measurements, vendor-specific hero labels or simulated working controls.
- Restored the support marquee as its own full-width sibling section. Preserved the existing infinite motion, smooth hover pause/resume and static reduced-motion behavior.
- The FAQ asks “How do I switch agents mid-task?” and explicitly lists Claude Code, Codex, Gemini CLI and Antigravity CLI. Shared data also updates the landing JSON-LD. Editor/Aider adapters are not advertised as switching targets.
- Reused AccountLink's existing account state. Swapped desktop button order/visual priority, retained account action on phones and added Download to mobile menus in both landing and shared marketing headers.
- Updated SPEC and DESIGN to record the accepted direction. No account, routing, agent, permission or network behavior changed.

## Verification

Browser regressions were observed failing for strip placement and hidden phone account access before the fixes. Acceptance covers conversation/handoff/phone visibility, independent strip, no report graphic, overflow/region bounds at 390/768/1440 in both themes, actual marquee movement and hover pause/resume, reduced motion, and synthetic signed-in/signed-out account navigation. All 18 web tests, the full Rust gate (153 tests plus fmt/clippy/deny/core purity) and browser acceptance pass. The optimized build and deployment checks are recorded in the PR.

## Copy changes

- web/lib/landing-faq.ts:2: “How do I switch from Claude Code to Codex?” becomes “How do I switch agents mid-task?”; its answer now lists every hosted switching CLI before explaining the compact handoff.
- web/app/LandingClient.tsx: hero “spawnt / overview” and numeric sample graphic become the illustrative conversation in web/app/components/HeroWorkflow.tsx. All new sample task/prompt/reply text is contained in that component.
- LandingClient.tsx and MarketingNavbar.tsx: Download remains the same destination/label; Log in/Console retains account-aware wording and becomes the red primary action. Mobile menu account duplicates become Download links.

Earlier marketing/SEO production acceptance passed all 19 public routes after the deployment-ignore fix. Original download artifacts remain untracked and excluded from this work.

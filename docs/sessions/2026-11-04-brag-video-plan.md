# 2026-11-04 - Brag video plan

## Goal
Plan a 4:3 launch video for LinkedIn and marketing with the `/brag` workflow, and keep the plan in the repo so the video can be regenerated (for example after a domain change) without replanning.

## Outcome
- `docs/marketing/brag-video/` holds the storyboard (`brag-plan.md`), the Hyperframes brief (`composition-brief.md`), a step-by-step prompt for a build agent (`handoff-prompt.md`) and a README describing regeneration.
- 1440x1080, 24.5s, polished tone; site address gnomcode.vercel.app on the outro and in the share copy.

## Decisions
- Planning and rendering are split: planning by a stronger model, rendering by a lighter model following the handoff prompt.
- Render output (`brag-output/`) stays local and uncommitted; only the plan is recorded.
- All on-screen product copy comes from the landing page, HeroWorkflow, ConsoleTourView and the Remote notification payload. One invented user prompt ("Run the tests too.") was approved.

## Verification
Documentation only; no code changed. Paths in the prompt were checked against the repo (fonts, mark, agent icons, skill files).

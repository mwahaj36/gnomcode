# Brag video (4:3 launch video)

A 24.5 second, 1440x1080 product video for LinkedIn and other marketing, planned with the `/brag` skill and rendered with Hyperframes.

| File | Purpose |
| :--- | :--- |
| [brag-plan.md](brag-plan.md) | Creative contract: angle, tone, palette, scene-by-scene storyboard with timings, audio direction. |
| [composition-brief.md](composition-brief.md) | Hyperframes brief: verbatim copy, assets, beat locks. |
| [handoff-prompt.md](handoff-prompt.md) | Step-by-step prompt for the build agent. Paste the fenced block into it. |

Rendered output goes to `brag-output/` in the repo root and is not committed (the prompt adds it to `.git/info/exclude`).

## Regenerate

1. Paste the fenced block from `handoff-prompt.md` into the build agent. No replanning is needed.
2. The agent writes `brag-output/brag.mp4`, `brag.jpg` (poster, also baked as frame 0) and `share-copy.txt`.

## Change the domain

The site address appears only as the literal string `gnomcode.vercel.app`. Replace it in all three files, then regenerate:

```powershell
Get-ChildItem docs/marketing/brag-video/*.md | ForEach-Object { (Get-Content $_ -Raw) -replace 'gnomcode\.vercel\.app', 'NEW-DOMAIN' | Set-Content $_ -NoNewline }
```

If the new address is much longer, check the Scene 6 snapshot for overflow; the prompt allows shrinking the address font by up to 10%.

## Change other copy

Product copy must stay grounded in the site (`web/app/LandingClient.tsx`, `web/app/components/HeroWorkflow.tsx`, `web/app/components/ConsoleTourView.tsx`). If the site copy changes, update the verbatim strings in all three files together.

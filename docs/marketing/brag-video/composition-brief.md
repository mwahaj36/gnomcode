# Hyperframes Composition Brief: Gnomcode

## Objective
Create a short, polished launch-style brag video for Gnomcode for LinkedIn and marketing.

## Output
- Composition directory: `brag-output/composition/`
- Rendered video: `brag-output/brag.mp4`
- Format: custom 4:3 — 1440x1080, 30fps
- Duration: 24.5 seconds

## Source Material
- Project root: c:\Users\Wahaj\Documents\GitHub\fleethud
- Primary files read: web/app/LandingClient.tsx, web/app/components/HeroWorkflow.tsx, web/app/components/ConsoleTourView.tsx, web/app/gnomcode-tokens.css, web/app/landing.css, web/components/RemoteCompanion.tsx, PRODUCT.md
- Product name: Gnomcode (wordmark is lowercase "gnomcode")
- Tagline / strongest claim: "Switch agents. Keep the task moving."
- Key UI moment to recreate: the HeroWorkflow conversation window with the "Concise handoff" card (Claude Code -> Codex)
- Copy that must appear verbatim:
  - Hit a limit.
  - Keep the task moving.
  - Switch agents. Keep the task moving.
  - Build the checkout flow
  - Build checkout with saved addresses.
  - Checkout is implemented. Keyboard navigation still needs testing.
  - Concise handoff / Same conversation
  - Task: Finish the checkout flow
  - Context: Changed files and recent activity
  - Next: Test keyboard navigation
  - I'll start with the keyboard navigation tests.
  - Away from your desk. / Still in control.
  - One fleet. One view of usage.
  - Open your next session.
  - Bring your agents into one workspace.
  - Download for Windows
  - gnomcode.vercel.app
- Assets to copy into composition/assets/:
  - web/public/gnomcode-mark.svg
  - web/public/fonts/inter-variable.woff2
  - web/public/fonts/jetbrains-mono.woff2
  - web/public/agents/claudecode.svg, codex.svg, antigravity.svg, gemini.svg, cline.svg, cursor.svg

## Creative Direction
- Tone preset: polished
- Creative direction: Apple keynote product film for a developer tool
- Interpretation: few scenes, long settled holds, soft crossfades and slides, slow push-ins on UI, one red accent, minimal SFX.
- Angle: the moment an agent hits a limit mid-task, answered by Gnomcode's concise handoff, then the same task followed on a phone, then usage with honest trust labels.
- Hook: "Hit a limit." on pure black, then "Keep the task moving."
- Outro / punchline: "Open your next session." then the mark locks on the strongest music hit.
- Avoid:
  - Generic SaaS language ("streamline", "supercharge", "10x")
  - Abstract filler visuals, particles, gradients, gradient text, glows on text
  - Emojis anywhere (product rule)
  - Light mode, purple, or any color not in the palette below

## Visual Identity
- Background: #070708
- Surfaces: #0d0d0f, #141416; borders #242428 / #38383e
- Text: #f6f7f8; secondary #a9b4c2; tertiary #8d99aa
- Accent: #ff2544 (fills and mark); #ff6177 (red text on dark); cool accent #becbd8
- Display font: Inter (local woff2), weight 600, tight letter-spacing (-0.02em to -0.035em)
- Body font: Inter 400-500; mono badges JetBrains Mono
- Visual references: HeroWorkflow window, console usage breakdown, trust badges EXACT / EST, red triangle + square mark
- Safe margin: 72px on all sides

## Storyboard
Use the storyboard in `docs/marketing/brag-video/brag-plan.md` as the creative contract.

1. Hook — 0.0-3.0s — "Hit a limit." / "Keep the task moving." over faint Claude quota bar
2. Reveal — 3.0-6.5s — mark assembles, wordmark, "Switch agents. Keep the task moving."
3. The handoff — 6.5-13.0s — conversation window, handoff card at 8.74s, rows, Codex reply types
4. Away from your desk — 13.0-17.0s — phone notification, tap, type "Run the tests too.", send
5. One view of usage — 17.0-20.5s — three provider rows with EXACT / EST badges
6. Outro — 20.5-24.5s — "Open your next session.", mark locks at 22.93s, Download pill + gnomcode.vercel.app, agent icons

## Audio
- Audio role: warm bed with sparse professional accents
- Audio arc: fade in under hook, low under UI, resolve on a bell accent with the final mark
- Music: happy-beats-business-moves-vol-12-by-ende-dot-app.mp3
- Music treatment: volume 0.30, 0.6s fade-in, 1.2s fade-out at the end
- Music cue guidance: `C:\Users\Wahaj\.agents\skills\brag\assets\music\cues\happy-beats-business-moves-vol-12-by-ende-dot-app.music-cues.json`. Strong locks: 8.74s, 17.47s, 22.93s. Beat grid for handoff rows 9.29 / 9.83 / 10.37; usage rows 17.47 / 18.02 / 18.56 (then hold the set).
- Audio-reactive treatment: subtle; RMS breathes a soft red glow behind the Scene 3 window and the Scene 6 mark only
- Audio-coupled moments:
  - Scene 1 hook line — soft impact
  - Scene 2 square lands — soft drop
  - Scene 3 handoff card — card slide; Codex reply — thinned key ticks
  - Scene 4 notification / tap / send — soft clicks
  - Scene 5 first row — soft drop
  - Scene 6 mark — bell accent
- SFX analysis guidance: `C:\Users\Wahaj\.agents\skills\brag\assets\sfx\sfx-analysis.md`; prefer low high-frequency-risk files
- Exact SFX choice: Hyperframes chooses filenames, timestamps, density, volume (0.55-0.7)
- Audio files: copy chosen music and SFX into `brag-output/composition/assets/`

## Hyperframes Instructions
Load `hyperframes-core`, `hyperframes-animation`, `hyperframes-creative`, `hyperframes-keyframes`, `hyperframes-cli`. Do not enter the generic `hyperframes` intent interview or its promo workflow.

Requirements:
- Show the real handoff UI, usage rows and mark from the project.
- Keep all text readable; honor the holds in the plan.
- Keep the video at 24.5s (must stay within 15-25s).
- Include music and sparse SFX.
- Beat locks within +/-0.15s, beat-grid items within +/-0.10s, marked in code comments.
- Run `npx hyperframes check` before render; fix every error including contrast.
- Keep creation and rendering local.

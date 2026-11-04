# Handoff prompt: render the Gnomcode brag video

Copy everything inside the fence below and paste it into the agent that does the build work. See [README.md](README.md) for how to change the domain and regenerate.

````text
You are building and rendering a 24.5 second marketing video for Gnomcode with Hyperframes. The creative decisions are ALREADY MADE. Your job is to implement them exactly, render, and report. Do not redesign, do not add scenes, do not change copy, do not change colors.

WORKING DIRECTORY: c:\Users\Wahaj\Documents\GitHub\fleethud
OS: Windows, shell: PowerShell. Use PowerShell syntax (Copy-Item, New-Item, Test-Path). Never use absolute paths inside HTML; always relative to the composition folder.

HARD RULES (breaking any of these = failure):
1. No emojis anywhere (HTML, comments, captions, file names).
2. Do NOT git add, commit or push anything. First run:  Add-Content .git\info\exclude "brag-output/"
3. Do not modify any file outside brag-output\ (you may only READ docs\ and web\ files and copy assets out of them). Do not edit the plan files in docs\marketing\brag-video\.
4. Copy text VERBATIM from docs\marketing\brag-video\brag-plan.md. Do not invent product claims, numbers or features.
5. Only these colors: #070708 #0d0d0f #141416 #242428 #38383e #f6f7f8 #a9b4c2 #8d99aa #ff2544 #ff6177 #becbd8 #0c1119. No gradients on text, no purple, no particles, no waveform bars.
6. Canvas is 1440 x 1080 (4:3), 30fps, total duration exactly 24.5s.
7. Only fonts: Inter and JetBrains Mono, loaded from local woff2 files via @font-face.

STEP 0 - READ (do this before writing any code, in this order):
  a. docs\marketing\brag-video\brag-plan.md            (the creative contract - storyboard with exact timings)
  b. docs\marketing\brag-video\composition-brief.md    (the technical brief)
  c. C:\Users\Wahaj\.agents\skills\hyperframes-core\SKILL.md
  d. C:\Users\Wahaj\.agents\skills\hyperframes-animation\SKILL.md
  e. C:\Users\Wahaj\.agents\skills\hyperframes-creative\SKILL.md
  f. C:\Users\Wahaj\.agents\skills\hyperframes-keyframes\SKILL.md
  g. C:\Users\Wahaj\.agents\skills\hyperframes-cli\SKILL.md
  h. C:\Users\Wahaj\.agents\skills\brag\references\audio.md
  i. C:\Users\Wahaj\.agents\skills\brag\references\step-4-deliver.md
  j. web\app\components\HeroWorkflow.tsx and the .sp-workflow-* rules in web\app\landing.css (to copy the look of the conversation window)
Follow the Hyperframes skills for HOW to build (structure, data-* attributes, timeline, check, render). Do NOT run the generic "hyperframes" intent interview or its promo workflow.

STEP 1 - SCAFFOLD AND COPY ASSETS:
  New-Item -ItemType Directory -Force brag-output\composition\assets\music, brag-output\composition\assets\sfx, brag-output\composition\assets\fonts, brag-output\composition\assets\img, brag-output\composition\assets\agents | Out-Null
  Copy-Item C:\Users\Wahaj\.agents\skills\brag\assets\music\happy-beats-business-moves-vol-12-by-ende-dot-app.mp3 brag-output\composition\assets\music\
  Copy-Item web\public\fonts\inter-variable.woff2, web\public\fonts\jetbrains-mono.woff2 brag-output\composition\assets\fonts\
  Copy-Item web\public\gnomcode-mark.svg brag-output\composition\assets\img\
  Copy-Item web\public\agents\claudecode.svg, web\public\agents\codex.svg, web\public\agents\antigravity.svg, web\public\agents\gemini.svg, web\public\agents\cline.svg, web\public\agents\cursor.svg brag-output\composition\assets\agents\
If the Hyperframes CLI has an init/scaffold command (see hyperframes-cli skill), run it inside brag-output\composition, but do not let it overwrite the assets folder.

STEP 2 - BUILD THE SIX SCENES. Timings are absolute seconds on the master timeline. "holds" means fully visible and not moving out.

SCENE 1  HOOK  0.0 - 3.0
  - Background #070708.
  - Faint layer (opacity 0.12): a thin horizontal bar 560px wide, 6px tall, radius 3px, track #242428, fill animates width from 72% to 100% between 0.0 and 2.4s; fill color #a9b4c2 turning #ff2544 in the last 0.3s. Tiny label above it in JetBrains Mono 18px #8d99aa: "Claude / weekly / EST".
  - 0.3s: "Hit a limit." Inter 600, 120px, #f6f7f8, letter-spacing -0.035em, left aligned at x=120, vertically centered slightly above middle. Enters with 0.35s rise (24px) + fade. Holds to 3.0.
  - 1.5s: "Keep the task moving." Inter 500, 64px, #a9b4c2, directly below. 0.4s fade + 12px rise. Holds to 3.0.
  - 2.7 - 3.0: whole scene crossfades out.

SCENE 2  REVEAL  3.0 - 6.5
  - Mark from assets/img/gnomcode-mark.svg, about 140px. Build it as two shapes (red triangle, red square, both #ff2544) so they can animate separately: triangle slides in from left 3.2-3.6s, square drops from above and lands 3.5-3.8s.
  - 3.9s: wordmark "gnomcode" (lowercase) Inter 600, 88px, #f6f7f8, to the right of the mark, 0.4s fade.
  - 4.4s: "Switch agents. Keep the task moving." Inter 600, 64px, #f6f7f8, centered below, 0.45s fade + rise. Holds to 6.3.
  - 6.2 - 6.5: everything slides up 40px and fades while Scene 3 window rises in.

SCENE 3  THE HANDOFF  6.5 - 13.0   (most important scene)
  - A window card ~1180px wide, ~820px tall, centered, background #0d0d0f, border 1px #242428, radius 18px.
  - Chrome row (top, height ~56px, border-bottom #242428): left = small mark + "gnomcode" Inter 600 20px; right = "Workspace / storefront" Inter 18px #8d99aa.
  - Title row: "Build the checkout flow" Inter 600 36px #f6f7f8; right side small tag "Conversation" 16px #8d99aa.
  - Thread (each item fades + rises 16px in 0.35s):
      6.7s  user bubble (right aligned, background #141416, radius 14px): "Build checkout with saved addresses."
      7.5s  agent bubble (left aligned): bold label "Claude Code" then "Checkout is implemented. Keyboard navigation still needs testing."
      8.74s HANDOFF CARD slides up 0.45s (beat-locked: 8.74s, write that in a code comment). Card bg #141416, border 1px #38383e, radius 14px, left edge 3px #ff2544.
            header row: "Concise handoff" (Inter 600 18px #f6f7f8) and right "Same conversation" (16px #8d99aa)
            agents row: "Claude Code"  [arrow SVG icon, stroke #ff6177]  "Codex"  (Inter 600 26px). Use an SVG arrow, not the characters "->".
      9.29s row "Task"    | "Finish the checkout flow"
      9.83s row "Context" | "Changed files and recent activity"
      10.37s row "Next"   | "Test keyboard navigation"
            (labels JetBrains Mono 15px #8d99aa uppercase; values Inter 20px #f6f7f8; comment "beat-grid: 9.29 / 9.83 / 10.37")
      11.0s agent bubble label "Codex", text types character by character, finishing by 12.0s: "I'll start with the keyboard navigation tests."  Use a straight apostrophe or the typographic one consistently.
  - Composer bar fixed at bottom of window: placeholder "Continue the task..." #8d99aa, right side pill "Codex" with a small chevron SVG.
  - As new items arrive, older items move up so everything fits. Nothing may overflow the window.
  - Camera: whole window scales 1.00 -> 1.06 linearly from 6.5 to 13.0.
  - Soft red glow (#ff2544 at low opacity, large blur) behind the window, its opacity gently driven by music RMS (audio-reactive, subtle). If audio extraction is unavailable, use a static glow at 0.10 opacity and note it.
  - 12.7 - 13.0: window slides left 60px and fades.

SCENE 4  AWAY FROM YOUR DESK  13.0 - 17.0
  - Left column at x=120: 13.2s "Away from your desk." Inter 600 72px #f6f7f8; 13.6s "Still in control." Inter 500 56px #a9b4c2. Both hold to 17.0.
  - Right: phone frame 430 x 880, radius 56px, bg #0d0d0f, border 2px #38383e, centered vertically at around x=920. A dark lock-screen look inside.
      13.4s notification banner slides down from top of the phone (0.4s): small mark icon + "Gnomcode" (13px #8d99aa), title "Build the checkout flow" (Inter 600 17px #f6f7f8), body "I'll start with the keyboard navigation tests." (15px #a9b4c2). Banner bg #141416, radius 18px.
      14.5s banner press (scale 0.97 then 1.0 over 0.2s), then phone screen crossfades to a conversation view: header "Build the checkout flow" with a small badge "E2EE" (JetBrains Mono 12px, 1px border #becbd8, color #becbd8); the Codex bubble with the same reply text.
      15.3s composer types "Run the tests too." finishing ~16.0s.
      16.2s send button (circle #ff2544 with dark arrow SVG) press; the message appears as a user bubble in the thread.
  - 16.7 - 17.0: crossfade out.

SCENE 5  ONE VIEW OF USAGE  17.0 - 20.5
  - 17.0s headline top-left x=120: "One fleet. One view of usage." Inter 600 64px #f6f7f8. Holds.
  - Card 1200 wide, bg #0d0d0f, border #242428, radius 18px. Three rows, each: provider name (Inter 600 30px), tokens value (Inter 28px) + badge "EXACT", cost value + badge "EST".
      17.47s Codex   124.0k tokens [EXACT]   $1.84 [EST]     (comment: beat-locked 17.47)
      18.02s Claude   98.0k tokens [EXACT]   $1.42 [EST]
      18.56s Gemini   74.0k tokens [EXACT]   $0.52 [EST]     (comment: beat-grid 17.47 / 18.02 / 18.56)
    Badges: JetBrains Mono 14px, padding 4px 8px, radius 6px. EXACT = border+text #becbd8. EST = border+text #ff6177.
  - Footnote under card, 18px #8d99aa: "Illustrative sample week. Every measurement carries its trust level."
  - All rows hold from 18.6 to 20.5. 20.2 - 20.5 crossfade out.

SCENE 6  OUTRO  20.5 - 24.5
  - 20.7s "Open your next session." Inter 600 96px #f6f7f8, centered, 0.45s fade + rise. Holds.
  - 21.6s "Bring your agents into one workspace." Inter 500 40px #a9b4c2 below. Holds.
  - 22.93s (beat-locked, comment it): both lines shift down ~60px while the mark (~110px, two red shapes) + "gnomcode" wordmark lock in above them. Soft red glow behind the mark driven subtly by music RMS.
  - 23.2s bottom row: pill "Download for Windows" (bg #ff2544, text #0c1119, Inter 600 22px, radius 6px, height 52px, padding 0 24px). To the right of the pill, 24px gap, vertically centered: the site address "gnomcode.vercel.app" in Inter 500 26px #a9b4c2 (lowercase, no "https://"). Under that row the label "WORKS WITH" (JetBrains Mono 14px #8d99aa, letter-spacing 0.12em) and six agent icons from assets/agents/ in a row, 36px each, rendered as #a9b4c2 via CSS mask, 28px gap.
  - Hold everything to 24.5. No fade to black needed.

STEP 3 - AUDIO
  - Music: assets/music/happy-beats-business-moves-vol-12-by-ende-dot-app.mp3, start 0, duration 24.5, volume 0.30, track index 10, fade in 0.6s, fade out over the final 1.2s.
  - Open C:\Users\Wahaj\.agents\skills\brag\assets\sfx\sfx-analysis.md and pick LOW high-frequency-risk files. Copy ONLY the files you use into assets\sfx\<family>\ (keep the family subfolder). Suggested picks (swap only if sfx-analysis flags them):
      0.25s   impact/impactSoft_medium_000.ogg     vol 0.60   (hook line)
      3.75s   interface/drop_001.ogg                vol 0.55   (square lands)
      8.70s   casino/card-slide-1.ogg               vol 0.60   (handoff card)
      11.0-12.0s  4 to 6 random keyboard/keypress-0XX.wav  vol 0.35  (thinned typing, not every character)
      13.40s  interface/drop_002.ogg                vol 0.50   (notification)
      14.50s  ui/click1.ogg                         vol 0.55   (tap banner)
      16.20s  ui/click2.ogg                         vol 0.55   (send)
      17.45s  interface/drop_001.ogg                vol 0.50   (first usage row only)
      22.90s  impact/impactBell_heavy_000.ogg       vol 0.60   (mark lock, let it ring over the fade)
  - Every overlapping SFX gets its own track index starting at 11 and going up. Never reuse an index for overlapping clips.

STEP 4 - CHECK (loop until clean)
  cd brag-output\composition
  npx hyperframes check
  Fix EVERY error, including contrast (apply the suggested compliant color but stay inside the palette family) and text overflow. Re-run until zero errors. Do NOT use --no-contrast.
  Then: npx hyperframes snapshot  and LOOK at the PNGs. Verify for each scene: no clipped text, nothing touching the 72px safe margin, no overlapping elements, fonts are Inter (not a serif fallback). Fix and re-check if anything is wrong.

STEP 5 - RENDER
  From brag-output\composition:
  npx hyperframes render --quality draft --output ..\brag.mp4      (quick sanity render; open it or snapshot it)
  npx hyperframes render --quality high --output ..\brag.mp4       (final)
  Verify: ffprobe -v error -show_entries stream=width,height,r_frame_rate:format=duration -of default=nw=1 ..\brag.mp4
  Expect width=1440, height=1080, 30/1, duration about 24.5. If not, fix and re-render.

STEP 6 - POSTER (from brag-output folder)
  cd ..
  ffmpeg -y -ss 9.0 -i brag.mp4 -frames:v 1 -q:v 2 brag.jpg
  Open brag.jpg. It must show the handoff card fully visible with the "Claude Code -> Codex" row. If it lands mid-animation, try 10.6 or 11.9 instead. Then bake it as frame 0:
  ffmpeg -y -i brag.mp4 -i brag.jpg -filter_complex "[0:v][1:v]overlay=0:0:enable='eq(n,0)'[v]" -map "[v]" -map 0:a? -c:v libx264 -crf 18 -preset slow -pix_fmt yuv420p -c:a copy -movflags +faststart brag.poster.mp4
  Move-Item -Force brag.poster.mp4 brag.mp4

STEP 7 - SHARE COPY
  Write brag-output\share-copy.txt with EXACTLY this text:
  Hit a limit in Claude Code? Gnomcode hands the task to Codex with a concise briefing, lets you follow it from your phone over an encrypted channel, and labels every usage number as exact or estimated. gnomcode.vercel.app

STEP 8 - REPORT BACK (short)
  - Path to brag.mp4, brag.jpg, share-copy.txt
  - ffprobe output (size, fps, duration)
  - Final `hyperframes check` result
  - Which SFX files you actually used and at what times
  - Whether audio-reactive glow worked or fell back to static
  - Anything you could NOT do exactly as specified, and why
Do not claim success without the ffprobe output and a clean check.

IF YOU GET STUCK:
  - Hyperframes command not found -> run it with npx exactly as written; if it still fails, stop and report the error text.
  - A font renders as serif -> the @font-face url is wrong; it must be relative, e.g. url("assets/fonts/inter-variable.woff2").
  - Audio silent in render -> path must be relative (assets/music/...), not absolute.
  - Text overflows -> reduce font size by at most 10% or tighten the bubble padding; NEVER cut verbatim copy.
````

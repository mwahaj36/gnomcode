# Brag Plan: Gnomcode

## What is this app?
Gnomcode is a local-first workspace that launches official AI coding agent CLIs, carries a concise handoff when you switch agents (Claude Code to Codex), lets you continue from your phone over an end-to-end encrypted channel, and shows usage with an explicit trust level on every number.

## The angle
Every developer using coding agents knows the moment: the agent stops because a limit was hit, mid-task. Gnomcode's own copy answers it in one line: "Hit a limit. Keep the task moving." The film is a quiet, Apple-keynote-style product piece: no hype, no stock motion, just the real handoff card doing its job, then the same task followed on a phone, then the usage view with its EXACT / EST badges. Restraint is the brand: near-black canvas, one red accent, Inter, generous space.

## Hook (first 2-3 seconds)
Pure black (#070708). "Hit a limit." lands center-left in large Inter at 0.3s and holds. At 1.5s the second line "Keep the task moving." fades up beneath it in the secondary gray. Behind the type, very faint, a Claude Code quota bar finishes filling. The viewer recognises the pain before they know the product.

## Key moments (the middle)
- The concise handoff card sliding into the conversation: "Claude Code -> Codex", then Task / Context / Next rows, then Codex replying "I'll start with the keyboard navigation tests."
- A phone receiving that reply as a notification titled "Build the checkout flow", tapping in, and sending the next instruction. Small "E2EE" badge on the phone header.
- The usage breakdown: Codex / Claude / Gemini rows arriving one by one, each with tokens marked EXACT and cost marked EST.

## Outro / punchline
"Open your next session." then the red triangle-and-square mark assembles, wordmark "gnomcode", a "Download for Windows" pill, and a quiet row of supported agent icons. The mark lands on the strongest music hit.

## User flow worth showing
1. Working with Claude Code on "Build the checkout flow" -> limit pressure.
2. Concise handoff to Codex inside the same conversation -> Codex continues.
3. Away from the desk: phone notification -> open conversation -> send next instruction.

## Tone
- Preset: polished
- Creative direction: Apple keynote product film for a developer tool
- Interpretation: few scenes, long settled holds, soft crossfades and slides, slow camera push-ins on UI, one accent color, minimal SFX. Confidence through restraint. No jokes, no exclamation marks, no emojis.

## Format: custom 4:3 (LinkedIn / marketing) — 1440x1080
## Duration: 24.5 seconds

## Visual identity (from the project)
- Background: #070708 (--sp-bg)
- Surface: #0d0d0f (--sp-surface), raised surface #141416 (--sp-surface-2)
- Border: #242428 (--sp-border), strong #38383e
- Text: #f6f7f8 (--sp-text); secondary #a9b4c2 (--sp-text-2); tertiary #8d99aa (--sp-text-3)
- Accent / brand: #ff2544 (--sp-primary); red text on dark #ff6177 (--sp-primary-text); cool accent #becbd8 (--sp-accent)
- Display font: Inter (web/public/fonts/inter-variable.woff2), tight tracking, weight 600-650 for headlines
- Body / labels font: Inter; mono labels JetBrains Mono (web/public/fonts/jetbrains-mono.woff2)
- Strongest visual element: the HeroWorkflow conversation window with the "Concise handoff" card (web/app/components/HeroWorkflow.tsx)
- Mark: web/public/gnomcode-mark.svg (red right triangle + red square)

## Share copy (draft)
Hit a limit in Claude Code? Gnomcode hands the task to Codex with a concise briefing, lets you follow it from your phone over an encrypted channel, and labels every usage number as exact or estimated. gnomcode.vercel.app

## Audio direction
- Role: warm, restrained bed with sparse professional accents
- Music: happy-beats-business-moves-vol-12-by-ende-dot-app.mp3 (steady, clean; tagged for polished)
- Music treatment: start at track 0.0s, volume 0.30, 0.6s fade-in, fade out over the last 1.2s while the final logo SFX rings over it
- Music cue guidance: bundled preset `<skill-dir>/assets/music/cues/happy-beats-business-moves-vol-12-by-ende-dot-app.music-cues.json`, ~110 BPM (beats ~0.55s apart). Strong cues to target: 8.74s (handoff card lands), 17.47s (first usage row), 22.93s (mark locks). Beat-grid window for handoff rows: 9.29 / 9.83 / 10.37. Beat-grid window for usage rows: 17.47 / 18.02 / 18.56 (reveal fast, then hold the full set).
- Audio-reactive treatment: subtle; music RMS gently breathes a soft red glow behind the hero window and the final mark only. No waveforms, bars or pulsing text.
- SFX posture: sparse (about 6-8 cues total), all at 0.55-0.7 volume, low high-frequency risk
- Audio-coupled moments: hook line landing, mark assembly, handoff card slide, Codex reply typing, phone tap/send, usage rows, final logo
- Restraint rule: no glitch, punch, casino or comedic sounds. No SFX on every row. Music never above 0.35.

## Storyboard

### Scene 1 — Hook — 0.0-3.0s (3.0s)
Black canvas. "Hit a limit." (Inter 600, ~120px, #f6f7f8) slams in at 0.3s (fast 0.35s rise + fade), holds. "Keep the task moving." (Inter 500, ~64px, #a9b4c2) fades up beneath at 1.5s, holds to 3.0s. Behind, at ~12% opacity, a thin quota bar labelled "Claude / weekly / EST" fills from 72% to full during 0.0-2.4s, red at the end.
Sequential/interaction: yes — two lines appear one after the other.
Audio intent: quiet tension, then resolution.
Audio-coupled idea: soft impact on the first line landing.
Music: fades in under the hook.
Transition mood: soft crossfade -> Scene 2.

### Scene 2 — Reveal — 3.0-6.5s (3.5s)
Mark assembles center: red triangle slides in from left, red square drops into place (3.2-3.8s). Wordmark "gnomcode" (lowercase, Inter 600) fades in right of the mark at 3.9s. Headline "Switch agents. Keep the task moving." fades up below at 4.4s, holds until 6.3s.
Sequential/interaction: yes — triangle, then square, then wordmark, then headline.
Audio intent: confident introduction.
Audio-coupled idea: soft drop when the square lands.
Transition mood: slide up / push into the window -> Scene 3.

### Scene 3 — The handoff (centerpiece) — 6.5-13.0s (6.5s)
Recreate the HeroWorkflow window, large and centered (about 1180px wide). Chrome row: mark + "gnomcode" left, "Workspace / storefront" right. Title "Build the checkout flow", tag "Conversation".
- 6.7s: user bubble "Build checkout with saved addresses."
- 7.5s: agent bubble, label "Claude Code": "Checkout is implemented. Keyboard navigation still needs testing."
- 8.74s (beat-locked): "Concise handoff" card slides up into the thread. Header "Concise handoff" / "Same conversation". Row "Claude Code -> Codex" (arrow icon, not text arrow).
- 9.29 / 9.83 / 10.37 (beat-grid): rows Task "Finish the checkout flow", Context "Changed files and recent activity", Next "Test keyboard navigation". Full card holds afterwards.
- 11.0s: agent bubble, label "Codex", types out: "I'll start with the keyboard navigation tests." (done by ~12.0s, hold to 13.0s).
- Composer at the bottom: "Continue the task..." with model selector reading "Codex".
Slow camera push-in (scale 1.00 -> 1.06) across the whole scene; earlier bubbles scroll up gently to make room.
Sequential/interaction: yes — bubbles and card rows arrive one by one; Codex reply types.
Audio intent: the core payoff, calm and satisfying.
Audio-coupled idea: card-slide on the handoff card, light key ticks on the Codex reply (thinned, not every character).
Transition mood: soft slide-left -> Scene 4.

### Scene 4 — Away from your desk — 13.0-17.0s (4.0s)
Left column: headline "Away from your desk." with "Still in control." beneath (secondary gray), appears at 13.2s and holds. Right: a phone frame (portrait, ~430x880, rounded 56px, #0d0d0f) on the black canvas.
- 13.4s: notification banner slides down on the phone lock screen: app "Gnomcode", title "Build the checkout flow", body "I'll start with the keyboard navigation tests."
- 14.5s: tap on the banner (small press-scale), phone shows the conversation with the Codex reply and a small "E2EE" badge in the header.
- 15.3s: composer types "Run the tests too." then the send button is tapped at 16.2s; bubble appears in the thread.
Sequential/interaction: yes — notification, tap, type, send.
Audio intent: personal, light.
Audio-coupled idea: soft notification tone on the banner, click on tap, click on send.
Transition mood: crossfade -> Scene 5.

### Scene 5 — One view of usage — 17.0-20.5s (3.5s)
Headline "One fleet. One view of usage." top-left at 17.0s. Card with three rows (Inter + mono badges):
- 17.47s (beat-locked): "Codex — 124.0k tokens EXACT — $1.84 EST"
- 18.02s: "Claude — 98.0k tokens EXACT — $1.42 EST"
- 18.56s: "Gemini — 74.0k tokens EXACT — $0.52 EST"
Full set holds 18.6-20.5s. Footnote in tertiary gray: "Illustrative sample week. Every measurement carries its trust level."
EXACT badges in cool accent #becbd8 outline, EST badges in red-text #ff6177 outline.
Sequential/interaction: yes — rows one by one, then hold the set.
Audio intent: precise, quiet.
Audio-coupled idea: one soft drop on the first row only.
Transition mood: soft crossfade -> Scene 6.

### Scene 6 — Outro — 20.5-24.5s (4.0s)
- 20.7s: "Open your next session." (Inter 600, ~96px) fades up center and holds.
- 21.6s: "Bring your agents into one workspace." beneath in secondary gray.
- 22.93s (beat-locked): headline lifts slightly; mark + "gnomcode" wordmark locks in above it with a subtle red glow (audio-reactive).
- 23.2s: "Download for Windows" pill (red #ff2544 fill, dark text #0c1119) with the site address "gnomcode.vercel.app" to its right (Inter 500, #a9b4c2), and a row of agent icons (Claude Code, Codex, Antigravity, Gemini CLI, Cline, Cursor) at 50% opacity under the label "WORKS WITH". Hold to end; music fades.
Sequential/interaction: yes — headline, subline, mark, CTA row.
Audio intent: resolved, confident ending.
Audio-coupled idea: bell-type logo accent at 22.93s ringing over the fade.
Transition mood: end on hold.

**Music mood for this video:** polished, steady, warm
**Audio summary:** a clean bed fades in under the hook, stays low under the UI, and resolves with one bell accent on the final mark.

## Duration check
3.0 + 3.5 + 6.5 + 4.0 + 3.5 + 4.0 = 24.5s

## Grounding notes
All on-screen product copy is taken from web/app/LandingClient.tsx, web/app/components/HeroWorkflow.tsx, web/app/components/ConsoleTourView.tsx and the notification payload in web/components/RemoteCompanion.tsx (title = conversation title, body = last reply). Usage numbers are the site's illustrative sample and are labelled as such. "Run the tests too." is an invented user prompt (framing only, approved by Wahaj). The site address gnomcode.vercel.app was supplied by Wahaj. No real user data, keys, hostnames or emails appear.

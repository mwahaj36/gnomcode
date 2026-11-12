# Road to 1.0.0

Authorized by Wahaj on 2026-11-13. Work in order; one focused PR per item; each item updates SPEC, adds a session record and, where marked, an ADR. Tick items here in the same PR that finishes them.

## A. In flight
- [ ] G1 phone approvals (ADR 0032)
- [ ] Provider cards: height, missing-state fallback, logos
- [ ] Web Devices and Connect redesign
- [ ] Project names fix and project activity list
- [ ] Subscription value tiles and activity cleanup

## B. Research gaps (docs/research/2026-11-13-dialpad-cao-comparison.md)
- [ ] Batch: G5 phone interrupt, G6 agy native resume, G7 agy limits via `/usage`, G11 env scrub, G12 Unix process-group stop, G13 Codex/Gemini native-install discovery
- [ ] G8 Claude usage-limit and API error parsing
- [ ] G3 terminal screen model, then G14 swallowed-Enter recovery

## C. Phone and Remote features (one ADR each, all providers where the agent exposes the data)
- [ ] Quick actions (canned prompts, never raw argv)
- [ ] Changes view: read-only diff of the session's repo on phone and console
- [ ] File viewer on phone: browse and read project files read-only, within the session workspace only
- [ ] Artifacts alongside changes: files the agent created or edited in the turn, plus images/HTML/markdown outputs, shown next to the diff
- [ ] Phone attachments (upload into the workspace with size/type limits)
- [ ] PWA app lock (WebAuthn/passkey or device PIN; never weakens E2EE)

## D. Audit Phase 2 leftovers
- [ ] D3 file watcher and idle back-off for ingest (CPU budget: idle under 1%)
- [ ] Antigravity exhausted cross-check against RESOURCE_EXHAUSTED
- [ ] Failover demo end to end: Claude exhausted, continue in Codex with handoff

## E. Consolidation (audit Phase 3)
- [ ] Local console built from web/ React; delete console-app.js
- [ ] Split api.rs and store/lib.rs by domain
- [ ] Freeze unproven features behind off-by-default toggles (Teams, extra adapters, account sync)

## F. Differentiators (audit Phase 4)
- [ ] Worktree per session with diff review and merge
- [ ] "Continue here" for imported sessions
- [ ] Handoff quality evals

## G. Release 1.0.0
- [ ] Version 1.0.0 everywhere (cut-release skill), CHANGELOG, release notes
- [ ] Packaged-app smoke test (launch, detect agents, limits, tray, Remote pairing)
- [ ] Windows installer built and published; macOS/Linux builds where CI supports them
- [ ] Website download page and version updated (Vercel)
- [ ] Real-device checks: physical phone Remote + approvals, installed PWA; macOS/Linux runs recorded or listed as known gaps
- [ ] Pricing decision (after release)

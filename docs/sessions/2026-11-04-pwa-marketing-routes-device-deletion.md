# 2026-11-04 — Progressive Web App, Route Distinction, AEO and Device Deletion

## Goals
- Establish clean architectural distinction between marketing routes and interactive app routes for faster page loads, isolated app shells, and superior SEO/AEO.
- Convert Gnomcode Web into a full Progressive Web App (PWA) with installability, offline fallback, Apple Touch assets, and background service worker.
- Enable end-to-end desktop and mobile PWA notifications for agent completions, required approvals, and quota resets.
- Implement device removal/deletion across the native daemon console and web devices panel.

## Outcomes
- **Distinct App and Marketing Routes**:
  - Marketing routes (`/`, `/pricing`, `/downloads`, `/docs/*`, `/privacy`, `/terms`) are indexed with canonical metadata and rich Schema.org JSON-LD structured data (`SoftwareApplication`, `FAQPage`, `TechArticle`).
  - App routes (`/app`, `/console`, `/remote`, `/settings`, `/auth`) are isolated under an application shell with `robots: { index: false, follow: false }`, ensuring user privacy and concentrating crawl authority on marketing pages.
  - Created `/app` workspace hub with 1-click launcher for Remote Companion, Fleet Console, and device settings.
- **Answer Engine Optimization (AEO)**:
  - Added `llms.txt` and `llms-full.txt` specifications for LLM web crawlers (Perplexity, ChatGPT, Gemini, Copilot).
  - Injected dynamic JSON-LD structured schemas (`SoftwareApplication`, `FAQPage`, `TechArticle`) across landing and guide pages.
- **Progressive Web App (PWA)**:
  - Created `web/public/manifest.webmanifest` with standalone display mode, orientation, app shortcuts, and branding colors.
  - Generated standard and maskable icons (192x192, 384x384, 512x512) via `scripts/generate-brand-assets.mjs`.
  - Implemented `web/public/sw.js` with offline fallback (`web/public/offline.html`) and push notification handling.
  - Configured Apple mobile web app metadata and viewport fit in `web/app/layout.tsx`.
- **Notifications on Desktop & Mobile PWA**:
  - Implemented `web/lib/notifications.ts` providing permission management, service worker notification dispatch with vibration patterns, desktop browser fallback, Web Audio chimes, and instant test alert triggers.
  - Added notification toggle button and status display in `RemoteCompanion.tsx`, `SettingsPanel.tsx`, and `/app`.
  - Triggered background notifications when agents finish turns or emit automation notices while the page is hidden.
- **Device Deletion**:
  - Added `remove_device` action in `crates/daemon/src/api.rs` and `AccountDevices::remove_device` in `crates/daemon/src/account_devices.rs`.
  - Added "Remove" action button in `crates/daemon/src/console-app.js` with confirmation dialog, room revocation, and cloud request cleanup.
  - Added device removal capability in `web/components/DesktopDevices.tsx`.

## Verification
- `npm --prefix web run typecheck`: clean (0 errors).
- `npm --prefix web test`: all 21 tests pass.
- `npm --prefix web run build`: 30/30 static pages compiled.
- `node scripts/check-release-versions.mjs`: PASS.
- `cargo test -p gnomcode-daemon`: all 68 tests pass (including new `remove_device_deletes_approved_connection_and_persists`).
- `.agents/skills/rust-quality-gate/scripts/check.ps1`: 5/5 quality gates passed (fmt, clippy -D warnings, test workspace, deny, core purity).

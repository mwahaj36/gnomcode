# 0006 — Shared spawnt Console Presentation

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

FleetHUD currently has a daemon-served HTML console, a separately authenticated Next.js cloud console, and a Rust tray controller with platform scripts. The console data and security paths must remain distinct. The rollout guide calls for matching layouts and tokens, but the web build (Next.js) and embedded local console currently have no shared presentation package. SPEC also describes a Tauri tray that is not present in this repository.

## Decision

Use a small framework-neutral shared token stylesheet as the canonical visual source, imported by the Next.js application and embedded into the daemon console at build time. Keep console transport, authentication, aggregation, business rules, and state handling in their existing Rust/local and Supabase/cloud paths. Do not migrate tray or installer architecture as part of the console redesign; first bring the existing Rust tray and platform scripts into visual alignment and revise SPEC to describe the implementation that actually ships. Any later Tauri migration requires a separate decision and parity work.

## Consequences

Both console implementations can share colors, type roles, controls, and layout conventions without bundling cloud code into the daemon or exposing local data to Supabase. Duplicated view markup remains a maintenance cost. Installer and tray changes can land after the console design is stable, with upgrade behavior preserved.

## Alternatives considered

- Move the local console into the Next.js app — rejected because it couples local daemon delivery to the signed-in web deployment and risks the local data boundary.
- Create a shared React package immediately — rejected because the daemon currently embeds a standalone HTML document and a package/bundler migration would delay visual parity without improving data separation.
- Build the Tauri shell described in SPEC during redesign — rejected because the current repository ships a Rust tray controller and platform scripts, and replacing the install/runtime architecture is a distinct high-impact migration.

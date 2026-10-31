# Existing Windows desktop upgrade detection

Date: 2026-11-01

## Problem and root cause

The options installer rejected the user's default registered install directory. Read-only inspection showed an Electron/Squirrel layout (`spawnt-desktop.exe`, `Update.exe`, `app-0.9.0`) rather than the native engine/uninstaller pair assumed by the first check.

## Fix and decisions

Recognize both legacy Electron/Squirrel and native launcher installations, including native installations created without a standalone uninstaller. Keep rejecting unrelated nonempty folders and drive/network roots. No recursive deletion or migration of existing Electron application folders is introduced. Installation continues to ship the native engine/tray payload through the existing release pipeline; this change does not repackage the Electron shell.

## Verification

The new layout regression failed before the fix and passed afterward. Tests cover empty folders, rejection of unrelated content, both previous layouts and unrelated-file preservation. Independently invoked validation against the actual existing desktop directory without modifying it. Release engine/C# builds, embedded payload/version checks, options/registry/cleanup checks, native console/navigation/tray and PowerShell URL checks passed. Copied the hash-verified replacement to `Desktop/Spawnt-Setup-Fixed.exe`. No full installation was executed by the agent. Build output stayed under `.tmp`; original public downloads were untouched.

# Failover selection snapshot follow-up

Date: 2026-11-04

## Goal and root cause

Final review of PR #126 found that the automatic-switch freshness guard compared a saved model selection with the agent's observed model ID. Native aliases and full IDs can legitimately differ, so an unchanged alias could prevent failover.

## Fix and decision

Capture the saved selection when evaluating the rate-limit event and compare that exact optional selection again after catalog discovery. Continue to recheck the current native conversation and routing policy under the control lock. Do not interpret an alias as proof of model activation. A user selection changed during lookup invalidates the pending automatic switch.

## Verification

Extend the isolated terminal regression to cover an unchanged alias with a different observed model ID and a subsequent user selection change. The unchanged-alias assertion failed before the fix. Run the full Rust gate and rebuild the matching Windows artifacts and manifests before publishing. Website behavior and storage/schema remain as documented in ADR 0027; no dependencies added.

## Acceptance limits

Real native model activation, mailbox delivery, physical-phone and macOS/Linux packaged acceptance remain open as recorded in the main session. This follow-up only corrects the selection freshness decision.

## Results

The alias regression, full Rust quality gate, installer payload/options checks and production web build passed. The rebuilt native executable matches the standalone download and ZIP payload.

```text
e6692c6e164dd9aa647442da8fb8cf17f2da25616fbfdce043d55806596abd53  Gnomcode-Setup.exe
90a2292f3b24cbb9f3e973e484c44c9e3b7b719210ec30907cc33774e82ea7de  Gnomcode-v0.9.0-windows-x64.zip
8891c188b253e3deb72d893ab8fde86d01a71ce1e0ad49a318b1cf115f830ff0  gnomcode.exe
```

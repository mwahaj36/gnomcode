# 2026-11-04 - Dependabot updates

## Goal

Review and verify the five open dependency updates.

## Findings and decisions

The AES-GCM update deprecated nonce slice conversion, causing the warnings-as-errors lint to fail. Construct the fixed-size nonce directly; this preserves the existing wire format. CI is manually triggered, so the PR checks alone do not run the Rust quality gate.

## Verification

The focused core clippy check failed on the original update and passed after the nonce conversion change. The Windows quality gate passed: formatting, workspace clippy and tests, cargo-deny, and core purity. Cargo-deny reported duplicate-dependency warnings.

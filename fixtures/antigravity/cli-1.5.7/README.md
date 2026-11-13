# Native usage metadata

Captured from installed agy 1.5.7 with `agy --output-format text -p=/usage` in a disposable workspace on 2026-11-13. The probe exited successfully within eight seconds. Native diagnostic logs showed retrieveUserQuotaSummary and no cascade/generation markers; a separate controlled marker-generation turn logged cascade and streamGenerateContent, providing a positive control. Raw diagnostic logs and account identifiers are not committed.

The anonymizer accepts JSON/JSONL, so each observed TSV row was first represented by its safe metadata fields: cli_version, name, limit_name, numeric remaining_percent and timestamp. Ran `.agents/skills/add-agent-adapter/scripts/anonymize-fixture.mjs` on those rows to produce usage.jsonl, inspected the output, then reconstructed usage.tsv from the anonymized fields. No prompts, paths, credentials, account names or native IDs occur in either fixture. Quota values/timestamps and model-pool labels are preserved as intended by the anonymizer.

Each TSV row is group, window label, percent remaining with a percent suffix, and RFC3339 reset time, separated by actual tabs. Five observed rows cover pooled five-hour/weekly windows and a model-specific weekly label. fake_usage_cli.rs is synthetic test-only source compiled into a temporary executable by the daemon regression; it never accesses native agent configuration.

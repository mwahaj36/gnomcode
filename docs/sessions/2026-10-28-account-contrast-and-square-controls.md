## 2026-10-28 — account contrast and square controls

- Goal: make the GitHub sign-in action readable, remove the cloud-sync tag, and remove pill-shaped UI treatments across current web and local console views.
- Outcome: removed the Cloud Sync badge, gave OAuth a bordered surface with theme-aware foreground color, changed marketing release/reference labels to plain mono text, made pricing billing controls square, placed tier labels in their headers, and changed local console button naming to match its existing square shape. Connection indicators are square markers without pill containers.
- Verification: `npm run build --prefix web`, embedded local-console JavaScript parsing, `git diff --check`, and a source search for pill styles passed. The full account and pricing route visual review will be recorded with the consolidated theme check.

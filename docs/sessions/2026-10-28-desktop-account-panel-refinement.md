## 2026-10-28 — desktop Account panel refinement

- Replaced the oversized technical cloud-roaming card with a connection summary, concise privacy explanation, numbered pairing steps and a masked, labeled token form. Retained the current domain, manual pairing API and private credential handling.
- Fetch account/publication status independently; failed status reads now clear Checking and show retry guidance. Connected devices show their account and Disconnect action; disconnected devices show the pairing flow. Pairing and disconnect prevent concurrent writes, and pairing updates state without reloading the page.
- Runtime regressions execute actual connected/disconnected/failure transitions and verify duplicate pairing calls issue one request, clear the token on success and restore controls. Native script/target checks pass. Account uses the shared responsive tokens and remains keyboard usable; browser screenshot inspection remains unavailable.


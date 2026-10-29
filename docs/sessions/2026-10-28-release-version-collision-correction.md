## 2026-10-28 — release version collision correction

- Verification of the new draft exposed pre-existing published v1.4.0 and v1.4.1 tags, despite this checkout's original 1.3.0 workspace version. The initial assumption of 1.4.0 was incorrect.
- Corrected the redesign release to 1.5.0, rebuilding every available platform artifact and renaming only the draft created in this session. Existing published releases/tags remain untouched. Removed session-generated 1.4.0 archive files so they cannot be mistaken for those older tags.

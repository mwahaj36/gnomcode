## 2026-10-28 — downloads route and platform availability

- Goal: redesign Downloads in the shared spawnt system while keeping published URLs and distinguishing available packages from pending platforms.
- Outcome: organized the verified Windows installer/archive and Linux install/checksum paths into a responsive platform list. Marked macOS unavailable until a signed build exists; included the current release's data-preserving update notes.
- Decision: retain `/downloads/Spawnt-Setup.exe`, the Windows archive, `/sh`, and checksum URLs; no macOS or npm package claims are shown.


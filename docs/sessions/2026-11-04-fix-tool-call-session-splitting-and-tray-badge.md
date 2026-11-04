# Fix Codex tool-call session splitting and tray badge clipping

Date: 2026-11-04

## Goal and root cause

Two issues were identified during live testing:

1. **Spurious ongoing sessions (14+ active/idle sessions):** In `crates/adapters/src/codex.rs`, `self.raw_session_id` was falling back to `payload.get("id")` across all event types. During log tailing where chunks begin at non-zero offsets (missing `session_meta`), custom tool calls (`custom_tool_call`, `id: "ctc_..."`) and function calls (`function_call`, `id: "fc_..."`) had their tool call IDs treated as session IDs. This created dozens of phantom session records in SQLite (`codex:ctc_...`, `codex:fc_...`), each with recent timestamps matching the tool executions ("Just now" / "1m ago"), making Gnomcode report 14+ ongoing sessions.
2. **Tray icon displaying `9` instead of `9+`:** In `scripts/tray.ps1`, when session count > 9, `$textStr` was set to `"9+"`. However, at the fixed font size of `Arial 9.5pt Bold`, the text width was 19.1px, exceeding the 18px badge bounding box (`Rectangle 13, 13, 18, 18`). GDI+ `DrawString` clipped the `+` character and only rendered `9`.

## Fix and decision

1. **Codex Adapter (`crates/adapters/src/codex.rs`):**
   - Initialized `self.raw_session_id` from the file path's rollout UUID at the start of `parse`.
   - Removed `payload.get("id")` fallback from non-metadata lines, restricting `id` extraction exclusively to `session_meta` and filtering out tool-call prefixes (`ctc_`, `fc_`, `ctco_`, `call_`).
   - Outside of `session_meta`, session fallback uses `path_id` or `session_id`/`thread_id`, never raw item `id`.
2. **Store Maintenance (`crates/store/src/lib.rs`):**
   - Added open-time maintenance cleanup purging any orphaned phantom `codex:ctc_%`, `codex:fc_%`, and `codex:ctco_%` sessions and their usage records.
3. **Tray Badge (`scripts/tray.ps1`):**
   - Dynamically set font size to 7.5pt when `$count -gt 9` so `"9+"` renders completely within the 18px badge without clipping.
4. **Testing & Verification:**
   - Added unit test `codex_tool_calls_at_offset_do_not_create_phantom_sessions` in `crates/adapters/src/lib.rs`.
   - Added verification in `scripts/check-tray.ps1` asserting that `Update-TrayIconBadge 10` renders the `+` glyph.

## Verification

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed with zero warnings.
- `cargo test --workspace` passed across all 11 crates.
- `powershell -NoProfile -File scripts/check-tray.ps1` passed.
- Cleaned up existing phantom sessions in local state database.

```text
474612f40ab615b7ef4004b82710b1bb42ff0b320a130a27bd7270bb8c648da7  Gnomcode-Setup.exe
b2ad4199f6fd5e0294cf6e059789c71bba585077ce4b2aa74091e0b0e52d9111  Gnomcode-v0.9.0-windows-x64.zip
bccf73db7ea7ad3aacaf02fb35f121ca833f2401a13dfc0c33270f016b4c048f  gnomcode.exe
```

# 2026-11-02 - Case-sensitive rename follow-up

## Goal and outcome

After rechecking and merging PRs 118 and 119, a case-insensitive tracked-path scan found two active filenames missed by the earlier lowercase substitution. Rename `Install-Spawnt.bat` to `Install-Gnomcode.bat` and `docs/SPAWNT_PHASE_0.md` to `docs/GNOMCODE_PHASE_0.md`. Their contents and current references already use Gnomcode.

## Root cause and decision

The original path substitution handled lowercase names but missed title-case and uppercase paths. Preserve historical ADR/session filenames; rename the two current entry points only. No implementation or behavior changes.

## Verification

Confirmed both new paths exist, both old paths are absent, and both file contents are byte-identical to their pre-rename versions. Checked active local Markdown links containing these names. The full Rust gates, app tests/builds and browser checks passed for the identical implementation tree before PRs 118/119 merged. No installer execution or native platform acceptance is claimed.

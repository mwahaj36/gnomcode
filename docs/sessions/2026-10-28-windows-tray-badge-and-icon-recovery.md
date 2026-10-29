## 2026-10-28 — Windows tray badge and icon recovery

- Reproduced GDI+ DrawIcon/ToBitmap throwing on PNG-compressed small ICO frames; the tray swallowed the failure and never applied its running-count overlay. Generate small Windows ICO frames as DIBs from the same canonical SVG, retaining a compressed large frame.
- Windows ICO frames now use an opaque #070708 canvas behind the red mark. Marketing PNG/SVG marks retain their existing backgrounds. Use ASCII tray separators to avoid Windows PowerShell UTF-8-without-BOM tooltip corruption.
- Native regression renders count one, restores count zero, reads the resulting icon bitmap and asserts an opaque brand-black corner. It failed before the ICO encoding fix and passes afterward. Tray pricing captions also distinguish partial estimates from complete estimates.


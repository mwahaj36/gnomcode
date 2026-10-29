## 2026-10-28 — Overview order and Windows icon inset

- Applied the owner's requested order to both consoles using actual DOM order: activity, spend summary, then limits on the left and ongoing sessions on the right. Narrow layouts preserve this reading/focus order.
- Removed duplicated total cost and pricing-warning text from the activity header; the spend summary remains authoritative and expanded day/hour details retain their own costs.
- Center the Windows canonical asset at 0.65x on its black canvas, without changing marketing marks. The real GDI+ icon regression failed with the previous red bounds and passes with the inset asset, including badge rendering and the zero-count reset.
- Shared presenter checks verify that costs stay out of the chart header but remain in expanded interval details. Native script checks pass; production build, Rust gate and package checks run before push. macOS remains deferred and browser visual tooling remains unavailable.

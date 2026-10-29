## 2026-10-28 — documentation visual system and pricing labels

- Goal: bring the documentation route into the marketing design system and clarify model-family and pricing-basis labels.
- Outcome: removed literal Markdown heading markers, replaced the sidebar/card-heavy monospace presentation with the shared sans-led hierarchy and open rows, and added definitions for Frontier, Tiered, and Exact. Explicitly identified Claude Fable 5/5.1 as Anthropic models, linked to the provider, and added one current provider-sourced Fable 5.1 API rate example.
- Decision: keep rate and quota calculations in Rust; the docs explain trust and catalog terms without reproducing the unverified full pricing table. Distinguish catalog rate basis from live-value trust labels.
- Verification: Next.js production build and `git diff --check` passed. Desktop dark and light renders and the accessibility tree were reviewed, including the corrected labels and Anthropic attribution. Narrow viewport, keyboard, and reduced-motion checks remain pending. `npm run lint --prefix web` could not run because the existing `next lint` script opens an interactive first-time configuration prompt; no lint config was added.

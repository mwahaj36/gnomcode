# 2026-11-04 - Marketing site audit

## Goal and outcome
Review public marketing and documentation pages for search visibility, accessibility, content and performance, including the preceding landing and search edits. Improved the Windows download path, truthful product availability, metadata and sitemap; added a skip link; corrected the documentation download-button contrast and enlarged small labels.

## Evidence and limits
- All 18 public content routes returned HTTP 200 with one H1, a title and description; internal links extracted from their rendered HTML returned successful HEAD responses.
- Impeccable rendered checks covered the four primary page templates on desktop and 14 documentation/legal routes at 390px. The shared low-contrast download label and undersized functional labels found in the main templates were corrected. Detector advisories about the established font, layout and marketing illustrations require design judgment, not automatic replacement.
- Local Lighthouse mobile runs against a production build gave downloads, pricing and documentation 100 for performance, accessibility, best practices and SEO; the initial homepage scored 100/100/100/92, with generic links subsequently renamed. Lab scores are not field measurements or accessibility certification.
- Web tests and typecheck passed; the production build passed with four existing React hook warnings. Windows Rust quality gate passed, with dependency duplication warnings from cargo-deny.
- Live search indexing, real-user performance, physical devices and assistive technology remain unverified. No analytics or invented social proof was added.

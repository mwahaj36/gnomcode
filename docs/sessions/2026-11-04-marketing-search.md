# 2026-11-04 - Marketing, search and answer visibility

## Goal and outcome
Improve the marketing site for Windows downloads and useful search answers without inventing customer evidence or adding analytics. Updated titles and descriptions, clarified prelaunch availability and pricing, emphasized the free download on pricing and guide pages, and made the Claude-to-Codex guide easier to follow.

## Decisions and verification
Excluded the Remote application from indexing and the sitemap while keeping its public guide discoverable. Removed the fixed sitemap date and obsolete FAQ rich-result markup. Kept the existing domain and product evidence, since no alternate domain, testimonials or footage were supplied. Locally verified the sitemap, Remote noindex and guide title; web tests, typecheck and lint passed, with existing React hook warnings. Production indexing and real-user acquisition remain unmeasured.

# 2026-10-29 — Vercel documentation route recovery

## Goal

Make the new public documentation routes available on the deployed site after the marketing/documentation refresh.

## Root cause and fix

Production acceptance found `/docs/getting-started`, `/docs/faq` and other nested guides returned 404, while the homepage and legal pages loaded. Local production builds included every route. The repository `.vercelignore` used unanchored `docs/`, which excludes nested directories named docs as well as engineering documentation. Anchored it to `/docs/` so engineering docs remain excluded while `web/app/docs` stays in deployment inputs.

Added a regression that executes Git's ignore matcher against the actual deployment ignore file and representative engineering/website paths. Observed it fail before the fix and pass after. This tests exclusion behavior rather than a literal config line. Vercel documents gitignore-like semantics at https://vercel.com/docs/deployments/vercel-ignore.

## Verification and boundaries

All 18 web tests and the optimized Next build pass for this fix. Production route/metadata acceptance follows the deployment. No app business logic, installer, schema or security protocol changes. The integrated main Rust gate also passes after the independently merged getrandom dependency update.

## Decision

Keep release caveats in technical docs. A successful deployment status alone is insufficient; request actual new routes on the public domain before claiming the site complete.

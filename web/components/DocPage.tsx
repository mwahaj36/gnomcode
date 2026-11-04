import Link from "next/link";
import type { ReactNode } from "react";

export default function DocPage({ title, answer, children, updated = false }: { title: string; answer: string; children: ReactNode; updated?: boolean }) {
  return <div className="sp-console-shell sp-docs-page sp-guide-page mx-auto px-6 font-sans">
    <article className="sp-docs-article">
      <header className="sp-docs-header"><Link href="/docs">Documentation</Link><h1>{title}</h1>{updated && <p>Last updated October 29, 2026</p>}</header>
      <p>{answer}</p>
      {children}
      <section><h2>Try Gnomcode on Windows</h2><p>Download the unsigned Windows development build and use local features free, without a Gnomcode account.</p><p><Link href="/downloads">Download for Windows</Link> · <Link href="/docs/getting-started">Installation guide</Link></p></section>
      <section><h2>Related guides</h2><nav className="sp-guide-related" aria-label="Related guides"><Link href="/docs/getting-started">Getting started</Link><Link href="/docs/supported-agents">Supported agents</Link><Link href="/docs/model-switching">Model switching</Link><Link href="/docs/remote-access">Remote access</Link><Link href="/docs/quota-and-cost">Quota and cost</Link><Link href="/docs/claude-code-vs-codex">Claude Code vs Codex</Link><Link href="/docs/prompt-from-phone">Prompt from your phone</Link><Link href="/docs/faq">FAQ</Link></nav></section>
    </article>
  </div>;
}

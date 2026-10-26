import type { Metadata } from "next";
import Link from "next/link";
import { BRAND } from "@/lib/brand";

export const metadata: Metadata = {
  title: "Documentation — Setup, Agents & API Reference",
  description:
    "Complete technical guide for spawnt: 1-minute quickstart, log adapter architecture, CLI commands, and REST APIs with zero content storage.",
  alternates: {
    canonical: "https://spawnt.xyz/docs",
  },
};

const docsJsonLd = {
  "@context": "https://schema.org",
  "@type": "TechArticle",
  headline: "spawnt Technical Documentation and Architecture Guide",
  description:
    "Complete manual for monitoring AI coding agents with zero content storage: setup, log adapters, CLI usage, and REST API.",
  author: {
    "@type": "Organization",
    name: "spawnt",
    url: "https://spawnt.xyz",
  },
  dependencies: "Windows 10/11, macOS 12+, Ubuntu/Debian/Fedora",
  articleSection: "Developer Tools",
};

export default function DocsPage() {
  const sections = [
    { id: "quickstart", title: "Quickstart (1 Min)" },
    { id: "supported-agents", title: "Supported Agents & Logs" },
    { id: "privacy", title: "Privacy Invariants" },
    { id: "cli", title: "CLI Command Reference" },
    { id: "api", title: "REST & WS API" },
    { id: "pricing-engine", title: "Pricing & Cache Math" },
    { id: "faq", title: "Troubleshooting & FAQ" },
  ];

  return (
    <div className="max-w-7xl mx-auto px-6 py-12 font-mono text-xs">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(docsJsonLd) }}
      />

      <div className="flex flex-col lg:flex-row gap-12">
        {/* Sticky Sidebar Navigation */}
        <aside className="w-full lg:w-64 shrink-0">
          <div className="sticky top-24 bg-surface border border-border rounded-[6px] p-5">
            <div className="text-foreground font-bold uppercase tracking-wider text-[11px] mb-3">
              DOCUMENTATION //
            </div>
            <nav className="space-y-1">
              {sections.map((s) => (
                <a
                  key={s.id}
                  href={`#${s.id}`}
                  className="block px-2.5 py-1.5 rounded-[4px] text-muted hover:text-primary hover:bg-surface-elevated transition-colors"
                >
                  {s.title}
                </a>
              ))}
            </nav>

            <div className="mt-6 pt-5 border-t border-border">
              <div className="text-[10px] text-muted uppercase tracking-wider mb-2">
                DESKTOP APP //
              </div>
              <Link
                href="/downloads"
                className="text-primary hover:underline flex items-center gap-1 font-bold"
              >
                <span>Download Desktop Client</span>
                <span>&rarr;</span>
              </Link>
            </div>
          </div>
        </aside>

        {/* Main Content Area */}
        <article className="flex-1 max-w-4xl space-y-12">
          {/* Header */}
          <div className="border-b border-border pb-8">
            <div className="inline-flex items-center gap-2 px-3 py-1 rounded-[4px] bg-surface border border-border text-muted uppercase tracking-wider text-[11px] mb-4">
              <span className="w-1.5 h-1.5 rounded-[1px] bg-primary"></span>
              <span>REFERENCE GUIDE // v{BRAND.version}</span>
            </div>
            <h1 className="text-3xl font-bold tracking-tight text-foreground mb-4">
              {BRAND.name} Documentation
            </h1>
            <p className="text-muted text-sm font-sans leading-relaxed">
              {BRAND.name} tracks live status, token spend, and quota headroom for Claude Code, Codex, Antigravity, Gemini, and Cursor. It runs locally as a native background daemon or desktop tray app, reads local agent logs, computes exact costs and prompt cache savings, and tracks per-model rate limit quota pools without ever storing prompts, source code, or tool responses.
            </p>
          </div>

          {/* Section 1: Quickstart */}
          <section id="quickstart" className="scroll-mt-24 space-y-4">
            <div className="text-muted text-[11px] uppercase tracking-wider font-bold">
              01 // QUICKSTART
            </div>
            <h2 className="text-xl font-bold text-foreground">
              Get started in 60 seconds
            </h2>
            <p className="text-muted font-sans text-xs leading-relaxed">
              {BRAND.name} runs as a standalone native binary with zero runtime dependencies. Choose your install method:
            </p>

            <div className="space-y-3">
              <div className="bg-surface border border-border rounded-[6px] p-4">
                <div className="flex justify-between items-center text-muted text-[11px] mb-2">
                  <span className="font-bold text-foreground">WINDOWS (POWERSHELL)</span>
                  <span className="text-primary font-bold">[RECOMMENDED]</span>
                </div>
                <div className="bg-surface-elevated p-3 rounded-[4px] border border-border text-foreground select-all">
                  irm https://spawnt.xyz/win | iex
                </div>
                <p className="text-[11px] text-muted font-sans mt-2">
                  Installs to <code className="text-foreground">%LOCALAPPDATA%\spawnt\</code>, configures start menu shortcuts, and runs a silent background tray service without command prompt popups.
                </p>
              </div>

              <div className="bg-surface border border-border rounded-[6px] p-4">
                <div className="flex justify-between items-center text-muted text-[11px] mb-2">
                  <span className="font-bold text-foreground">MACOS & LINUX (CURL)</span>
                  <span>Darwin & POSIX</span>
                </div>
                <div className="bg-surface-elevated p-3 rounded-[4px] border border-border text-foreground select-all">
                  curl -fsSL https://spawnt.xyz/install.sh | sh
                </div>
              </div>

              <div className="bg-surface border border-border rounded-[6px] p-4">
                <div className="flex justify-between items-center text-muted text-[11px] mb-2">
                  <span className="font-bold text-foreground">NPM GLOBAL</span>
                  <span>Cross-platform CLI</span>
                </div>
                <div className="bg-surface-elevated p-3 rounded-[4px] border border-border text-foreground select-all">
                  npm install -g spawnt
                </div>
              </div>
            </div>

            <p className="text-muted text-xs">
              Verify your setup by running:
            </p>
            <div className="bg-surface-elevated p-3 rounded-[4px] border border-border text-foreground">
              spawnt status
            </div>
          </section>

          {/* Section 2: Supported Agents */}
          <section id="supported-agents" className="scroll-mt-24 space-y-4">
            <div className="text-muted text-[11px] uppercase tracking-wider font-bold">
              02 // AGENT LOG ADAPTERS
            </div>
            <h2 className="text-xl font-bold text-foreground">
              Supported Agents & Log Paths
            </h2>
            <p className="text-muted font-sans text-xs leading-relaxed">
              {BRAND.name} parses standard local outputs with zero agent modification required:
            </p>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div className="bg-surface border border-border rounded-[6px] p-4">
                <div className="flex items-center justify-between mb-2">
                  <span className="font-bold text-foreground text-sm">Claude Code</span>
                  <span className="text-[10px] px-1.5 py-0.5 rounded-[4px] bg-primary/10 text-primary border border-primary/30 font-bold">
                    [EXACT]
                  </span>
                </div>
                <p className="text-muted text-xs font-sans mb-3 leading-relaxed">
                  Extracts token usage, cached input reads, and thinking tokens from <code className="text-foreground">~/.claude/projects/</code>.
                </p>
                <div className="text-[11px] text-muted">
                  Path: <code className="text-foreground">~/.claude/projects/*/*.jsonl</code>
                </div>
              </div>

              <div className="bg-surface border border-border rounded-[6px] p-4">
                <div className="flex items-center justify-between mb-2">
                  <span className="font-bold text-foreground text-sm">OpenAI Codex</span>
                  <span className="text-[10px] px-1.5 py-0.5 rounded-[4px] bg-primary/10 text-primary border border-primary/30 font-bold">
                    [EXACT]
                  </span>
                </div>
                <p className="text-muted text-xs font-sans mb-3 leading-relaxed">
                  Parses rollout logs with session UUID attribution, 5-hour and 7-day rate limit headers, and turn deltas.
                </p>
                <div className="text-[11px] text-muted">
                  Path: <code className="text-foreground">~/.codex/sessions/rollout-*.jsonl</code>
                </div>
              </div>

              <div className="bg-surface border border-border rounded-[6px] p-4">
                <div className="flex items-center justify-between mb-2">
                  <span className="font-bold text-foreground text-sm">Google Antigravity</span>
                  <span className="text-[10px] px-1.5 py-0.5 rounded-[4px] bg-surface-elevated text-foreground border border-border font-bold">
                    [REAL-TIME]
                  </span>
                </div>
                <p className="text-muted text-xs font-sans mb-3 leading-relaxed">
                  Monitors active conversation transcripts in IDE brain directory. Tracks step indexes and tool call states.
                </p>
                <div className="text-[11px] text-muted">
                  Path: <code className="text-foreground">~/.gemini/antigravity/brain/*/transcript.jsonl</code>
                </div>
              </div>

              <div className="bg-surface border border-border rounded-[6px] p-4">
                <div className="flex items-center justify-between mb-2">
                  <span className="font-bold text-foreground text-sm">Gemini CLI & Cursor</span>
                  <span className="text-[10px] px-1.5 py-0.5 rounded-[4px] bg-surface-elevated text-muted border border-border">
                    [SUPPORTED]
                  </span>
                </div>
                <p className="text-muted text-xs font-sans mb-3 leading-relaxed">
                  Tracks CLI commands, turns, and token counts from local Gemini and Cursor state stores.
                </p>
                <div className="text-[11px] text-muted">
                  Path: <code className="text-foreground">~/.gemini/logs/*</code>
                </div>
              </div>
            </div>
          </section>

          {/* Section 3: Privacy Invariants */}
          <section id="privacy" className="scroll-mt-24 space-y-4">
            <div className="text-muted text-[11px] uppercase tracking-wider font-bold">
              03 // PRIVACY INVARIANTS
            </div>
            <h2 className="text-xl font-bold text-foreground">
              Non-negotiable architectural guarantees
            </h2>

            <div className="space-y-3">
              <div className="bg-surface border border-border rounded-[6px] p-4">
                <div className="text-foreground font-bold mb-1">
                  1. Zero Content Storage
                </div>
                <p className="text-muted text-xs font-sans leading-relaxed">
                  Prompts, completions, code snippets, diffs, and tool output payloads NEVER touch disk or network. {BRAND.name} persists only numeric counters, timestamps, status enums, and scrubbed project names.
                </p>
              </div>

              <div className="bg-surface border border-border rounded-[6px] p-4">
                <div className="text-foreground font-bold mb-1">
                  2. Local HMAC-SHA256 Project Masking
                </div>
                <p className="text-muted text-xs font-sans leading-relaxed">
                  Before optional cloud synchronization, local filesystem paths are truncated to directory basenames, and full paths are irreversibly hashed using HMAC-SHA256 with a machine key.
                </p>
              </div>

              <div className="bg-surface border border-border rounded-[6px] p-4">
                <div className="text-foreground font-bold mb-1">
                  3. Fail-Open Architecture (50ms IPC Timeout)
                </div>
                <p className="text-muted text-xs font-sans leading-relaxed">
                  Integrations and hooks run asynchronously. If the {BRAND.name} daemon is stopped or slow, agent execution never blocks.
                </p>
              </div>

              <div className="bg-surface border border-border rounded-[6px] p-4">
                <div className="text-foreground font-bold mb-1">
                  4. Loopback-Only Network Binding
                </div>
                <p className="text-muted text-xs font-sans leading-relaxed">
                  The local REST/WebSocket daemon binds strictly to <code className="text-foreground">127.0.0.1:41738</code> with bearer token authentication and Host/Origin header verification.
                </p>
              </div>
            </div>
          </section>

          {/* Section 4: CLI Reference */}
          <section id="cli" className="scroll-mt-24 space-y-4">
            <div className="text-muted text-[11px] uppercase tracking-wider font-bold">
              04 // CLI REFERENCE
            </div>
            <h2 className="text-xl font-bold text-foreground">
              Terminal command suite
            </h2>

            <div className="space-y-3">
              <div className="bg-surface border border-border rounded-[6px] p-4">
                <div className="font-bold text-primary mb-1">spawnt status</div>
                <div className="text-muted text-xs font-sans mb-2">
                  Inspect currently active and idle agent sessions across all installed tools.
                </div>
                <div className="bg-surface-elevated p-2.5 rounded-[4px] border border-border text-foreground">
                  $ spawnt status<br />
                  Codex: 2 active (cdx-01a17825, cdx-01a176fa)<br />
                  Antigravity: 1 active (agy-b488ea6c)<br />
                  Total: 3 active agents running
                </div>
              </div>

              <div className="bg-surface border border-border rounded-[6px] p-4">
                <div className="font-bold text-primary mb-1">spawnt report [--range today|7d|30d|all]</div>
                <div className="text-muted text-xs font-sans mb-2">
                  Outputs complete API spend breakdown, prompt cache savings, and ROI multiplier against flat subscription plans.
                </div>
                <div className="bg-surface-elevated p-2.5 rounded-[4px] border border-border text-foreground">
                  $ spawnt report --range 7d<br />
                  API-Equivalent Spend: $142.80<br />
                  Prompt Cache Savings: $48.20 (41% cached tokens)<br />
                  Anthropic Plan: $100/mo &rarr; 1.4x ROI
                </div>
              </div>

              <div className="bg-surface border border-border rounded-[6px] p-4">
                <div className="font-bold text-primary mb-1">spawnt daemon</div>
                <div className="text-muted text-xs font-sans">
                  Starts the local HTTP and WebSocket service on port 41738 for tray app and console UI.
                </div>
              </div>
            </div>
          </section>

          {/* Section 5: REST API */}
          <section id="api" className="scroll-mt-24 space-y-4">
            <div className="text-muted text-[11px] uppercase tracking-wider font-bold">
              05 // REST API REFERENCE
            </div>
            <h2 className="text-xl font-bold text-foreground">
              Local endpoints (127.0.0.1:41738)
            </h2>

            <div className="space-y-3">
              <div className="bg-surface border border-border rounded-[6px] p-3.5">
                <div className="flex items-center gap-2 mb-1">
                  <span className="px-1.5 py-0.5 rounded-[4px] bg-surface-elevated border border-border font-bold text-[10px]">GET</span>
                  <span className="text-foreground font-bold">/api/v1/live</span>
                </div>
                <div className="text-muted text-xs font-sans">
                  Real-time snapshot with active agents, attention alerts, KPI totals, quota pools, and active sessions.
                </div>
              </div>

              <div className="bg-surface border border-border rounded-[6px] p-3.5">
                <div className="flex items-center gap-2 mb-1">
                  <span className="px-1.5 py-0.5 rounded-[4px] bg-surface-elevated border border-border font-bold text-[10px]">GET</span>
                  <span className="text-foreground font-bold">/api/v1/usage/daily</span>
                </div>
                <div className="text-muted text-xs font-sans">
                  Daily aggregated token counts and dollar costs for interactive charts, grouped by agent or model.
                </div>
              </div>

              <div className="bg-surface border border-border rounded-[6px] p-3.5">
                <div className="flex items-center gap-2 mb-1">
                  <span className="px-1.5 py-0.5 rounded-[4px] bg-surface-elevated border border-border font-bold text-[10px]">GET</span>
                  <span className="text-foreground font-bold">/api/v1/projects</span>
                </div>
                <div className="text-muted text-xs font-sans">
                  Rollups of token spend and turn activity aggregated by workspace and project directory.
                </div>
              </div>

              <div className="bg-surface border border-border rounded-[6px] p-3.5">
                <div className="flex items-center gap-2 mb-1">
                  <span className="px-1.5 py-0.5 rounded-[4px] bg-surface-elevated border border-border font-bold text-[10px]">GET</span>
                  <span className="text-foreground font-bold">/api/v1/usage/summary</span>
                </div>
                <div className="text-muted text-xs font-sans">
                  High-level usage metrics (total tokens, total cost, trust level, cache savings ratio).
                </div>
              </div>

              <div className="bg-surface border border-border rounded-[6px] p-3.5">
                <div className="flex items-center gap-2 mb-1">
                  <span className="px-1.5 py-0.5 rounded-[4px] bg-surface-elevated border border-border font-bold text-[10px]">GET</span>
                  <span className="text-foreground font-bold">/api/v1/quota</span>
                </div>
                <div className="text-muted text-xs font-sans">
                  Current per-model quota pools (percent used, reset timestamps, window duration).
                </div>
              </div>
            </div>
          </section>

          {/* Section 6: Model Pricing & Cost Ledger */}
          <section id="pricing-engine" className="scroll-mt-24 space-y-4">
            <div className="text-muted text-[11px] uppercase tracking-wider font-bold">
              06 // MODEL PRICING CATALOG
            </div>
            <h2 className="text-xl font-bold text-foreground">
              Official developer pricing ledger
            </h2>
            <p className="text-muted font-sans text-xs leading-relaxed">
              Base rates mirror official provider pricing documentation from Anthropic, OpenAI, and Google.
            </p>

            <div className="overflow-x-auto border border-border rounded-[6px] bg-surface">
              <table className="w-full text-left text-[11px]">
                <thead className="bg-surface-elevated text-muted border-b border-border uppercase">
                  <tr>
                    <th className="py-2.5 px-3">MODEL //</th>
                    <th className="py-2.5 px-3">INPUT / 1M</th>
                    <th className="py-2.5 px-3">OUTPUT / 1M</th>
                    <th className="py-2.5 px-3">CACHE READ</th>
                    <th className="py-2.5 px-3">BASIS</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border/60 text-foreground font-mono">
                  <tr>
                    <td className="py-2 px-3 font-bold">Claude 3.7 Sonnet</td>
                    <td className="py-2 px-3">\$3.00</td>
                    <td className="py-2 px-3">\$15.00</td>
                    <td className="py-2 px-3 text-primary">\$0.30 (90% off)</td>
                    <td className="py-2 px-3 text-muted">[EXACT]</td>
                  </tr>
                  <tr>
                    <td className="py-2 px-3 font-bold">Claude 3.5 Haiku</td>
                    <td className="py-2 px-3">\$0.80</td>
                    <td className="py-2 px-3">\$4.00</td>
                    <td className="py-2 px-3 text-primary">\$0.08 (90% off)</td>
                    <td className="py-2 px-3 text-muted">[EXACT]</td>
                  </tr>
                  <tr>
                    <td className="py-2 px-3 font-bold">gpt-4o</td>
                    <td className="py-2 px-3">\$2.50</td>
                    <td className="py-2 px-3">\$10.00</td>
                    <td className="py-2 px-3 text-primary">\$1.25 (50% off)</td>
                    <td className="py-2 px-3 text-muted">[EXACT]</td>
                  </tr>
                  <tr>
                    <td className="py-2 px-3 font-bold">o3-mini</td>
                    <td className="py-2 px-3">\$1.10</td>
                    <td className="py-2 px-3">\$4.40</td>
                    <td className="py-2 px-3 text-primary">\$0.55 (50% off)</td>
                    <td className="py-2 px-3 text-muted">[EXACT]</td>
                  </tr>
                  <tr>
                    <td className="py-2 px-3 font-bold">gemini-2.0-flash</td>
                    <td className="py-2 px-3">\$0.10</td>
                    <td className="py-2 px-3">\$0.40</td>
                    <td className="py-2 px-3 text-primary">\$0.025 (75% off)</td>
                    <td className="py-2 px-3 text-muted">[EXACT]</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* Bottom CTA */}
          <div className="border-t border-border pt-8 flex items-center justify-between flex-wrap gap-4">
            <span className="text-muted">Ready to inspect your local fleet?</span>
            <div className="flex gap-3">
              <Link
                href="/console"
                className="h-9 px-4 bg-primary text-white font-bold rounded-[4px] hover:brightness-110 transition-all flex items-center"
              >
                Launch Console &rarr;
              </Link>
              <Link
                href="/pricing"
                className="h-9 px-4 bg-surface border border-border text-foreground rounded-[4px] hover:border-primary transition-colors flex items-center"
              >
                View Plans
              </Link>
            </div>
          </div>
        </article>
      </div>
    </div>
  );
}

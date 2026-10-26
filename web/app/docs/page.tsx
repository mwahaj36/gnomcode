import type { Metadata } from "next";
import Link from "next/link";

export const metadata: Metadata = {
  title: "Documentation — Setup, Agents & API Reference",
  description:
    "Complete technical guide for FleetHUD: 1-minute quickstart, log adapter architecture for Claude Code, Codex, and Antigravity, CLI commands, and REST APIs.",
  alternates: {
    canonical: "https://fleethud.vercel.app/docs",
  },
};

const docsJsonLd = {
  "@context": "https://schema.org",
  "@type": "TechArticle",
  headline: "FleetHUD Technical Documentation and Architecture Guide",
  description:
    "Complete manual for monitoring AI coding agents with zero content storage: setup, log adapters, CLI usage, and REST API.",
  author: {
    "@type": "Organization",
    name: "FleetHUD",
    url: "https://fleethud.vercel.app",
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
    <div className="max-w-7xl mx-auto px-6 py-12 font-sans">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(docsJsonLd) }}
      />

      <div className="flex flex-col lg:flex-row gap-12">
        {/* Sticky Sidebar Navigation */}
        <aside className="w-full lg:w-64 shrink-0 font-mono text-xs">
          <div className="sticky top-24 bg-card border border-border rounded-lg p-5">
            <div className="text-foreground font-semibold uppercase tracking-wider text-[11px] mb-3">
              Documentation
            </div>
            <nav className="space-y-1.5">
              {sections.map((s) => (
                <a
                  key={s.id}
                  href={`#${s.id}`}
                  className="block px-2.5 py-1.5 rounded text-muted hover:text-accent hover:bg-bg transition-colors"
                >
                  {s.title}
                </a>
              ))}
            </nav>

            <div className="mt-6 pt-5 border-t border-border">
              <div className="text-[10px] text-muted uppercase tracking-wider mb-2">
                Need Help?
              </div>
              <a
                href="https://github.com/mwahaj36/fleethud/issues"
                target="_blank"
                rel="noreferrer"
                className="text-accent hover:underline flex items-center gap-1"
              >
                <span>Open GitHub Issue</span>
                <span>&rarr;</span>
              </a>
            </div>
          </div>
        </aside>

        {/* Main Content Area */}
        <article className="flex-1 max-w-4xl prose prose-invert">
          {/* Header */}
          <div className="border-b border-border pb-8 mb-10">
            <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-accent/15 border border-accent/30 text-accent font-mono text-xs mb-4">
              <span>FleetHUD Reference Guide &bull; v1.1.1</span>
            </div>
            <h1 className="text-4xl font-extrabold tracking-tight text-foreground mb-4">
              FleetHUD Documentation
            </h1>
            <p className="text-muted text-base leading-relaxed">
              FleetHUD is a live mission control cockpit for AI coding agents. It runs locally as a native background daemon or desktop tray app, reads agent runtime logs in real time, computes exact token spend attribution and prompt cache savings, and tracks per-model rate limit quota pools without ever storing prompts, source code, or tool responses.
            </p>
          </div>

          {/* Section 1: Quickstart */}
          <section id="quickstart" className="mb-14 scroll-mt-24">
            <h2 className="text-2xl font-bold text-foreground mb-4 font-mono">
              # Quickstart (1 Minute)
            </h2>
            <p className="text-muted text-sm leading-relaxed mb-4">
              FleetHUD runs as a standalone native binary with zero external runtime dependencies. Choose your platform below:
            </p>

            <div className="space-y-4 font-mono text-xs">
              <div className="bg-card border border-border rounded-lg p-4">
                <div className="flex justify-between items-center text-muted text-[11px] mb-2">
                  <span>WINDOWS (POWERSHELL / CMD)</span>
                  <span className="text-green">[RECOMMENDED] Native Installer</span>
                </div>
                <div className="bg-bg p-3 rounded border border-border text-foreground overflow-x-auto select-all">
                  irm https://raw.githubusercontent.com/mwahaj36/fleethud/main/scripts/install.ps1 | iex
                </div>
                <p className="text-[11px] text-muted mt-2">
                  Installs <code className="text-accent">Installer.exe</code> to <code className="text-accent">%LOCALAPPDATA%\fleethud\</code>, creates desktop and start menu shortcuts, and sets up a background tray app with zero black console windows.
                </p>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="flex justify-between items-center text-muted text-[11px] mb-2">
                  <span>MACOS (HOMEBREW / CURL)</span>
                  <span>Darwin x64 & Apple Silicon</span>
                </div>
                <div className="bg-bg p-3 rounded border border-border text-foreground overflow-x-auto select-all">
                  curl -fsSL https://raw.githubusercontent.com/mwahaj36/fleethud/main/scripts/install.sh | bash
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="flex justify-between items-center text-muted text-[11px] mb-2">
                  <span>NPM GLOBAL PACKAGE</span>
                  <span>Cross-platform CLI Shim</span>
                </div>
                <div className="bg-bg p-3 rounded border border-border text-foreground overflow-x-auto select-all">
                  npm install -g fleethud
                </div>
              </div>
            </div>

            <p className="text-muted text-xs mt-4">
              Once installed, verify the installation by running:
            </p>
            <div className="bg-card p-3 rounded border border-border font-mono text-xs text-foreground mt-2">
              fleethud status
            </div>
          </section>

          {/* Section 2: Supported Agents */}
          <section id="supported-agents" className="mb-14 scroll-mt-24">
            <h2 className="text-2xl font-bold text-foreground mb-4 font-mono">
              # Supported Agents & Adapters
            </h2>
            <p className="text-muted text-sm leading-relaxed mb-6">
              FleetHUD connects directly to the local file outputs of popular AI coding tools. It requires zero configuration in the target agent.
            </p>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 font-mono text-xs">
              <div className="bg-card border border-border rounded-lg p-5">
                <div className="flex items-center justify-between mb-2">
                  <div className="font-bold text-foreground text-sm">Claude Code</div>
                  <span className="text-[10px] px-2 py-0.5 rounded bg-accent/15 text-accent border border-accent/30 font-semibold">
                    AUTO-DETECT
                  </span>
                </div>
                <p className="text-muted text-[11px] mb-3 leading-relaxed">
                  Extracts token usage, cached input reads, and thinking tokens from <code className="text-foreground">~/.claude/projects/</code>. Detects subscription tier ($20 Pro or $100 Max 5x) from credentials metadata without touching auth keys.
                </p>
                <div className="text-[10px] text-muted">
                  Log Path: <code className="text-foreground">~/.claude/projects/*/*.jsonl</code>
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-5">
                <div className="flex items-center justify-between mb-2">
                  <div className="font-bold text-foreground text-sm">OpenAI Codex</div>
                  <span className="text-[10px] px-2 py-0.5 rounded bg-green/15 text-green border border-green/30 font-semibold">
                    EXACT LIMITS
                  </span>
                </div>
                <p className="text-muted text-[11px] mb-3 leading-relaxed">
                  Parses <code className="text-foreground">~/.codex/sessions/rollout-*.jsonl</code> with session UUID attribution. Detects 5-hour and 7-day exact rate limit headers, turn token usage, and mid-session model shifts.
                </p>
                <div className="text-[10px] text-muted">
                  Log Path: <code className="text-foreground">~/.codex/sessions/rollout-*.jsonl</code>
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-5">
                <div className="flex items-center justify-between mb-2">
                  <div className="font-bold text-foreground text-sm">Google Antigravity</div>
                  <span className="text-[10px] px-2 py-0.5 rounded bg-accent/15 text-accent border border-accent/30 font-semibold">
                    REAL-TIME
                  </span>
                </div>
                <p className="text-muted text-[11px] mb-3 leading-relaxed">
                  Monitors active conversation transcripts in <code className="text-foreground">~/.gemini/antigravity/brain/</code>. Tracks step indexes, tool call execution states (Running/Waiting), model transitions, and prompt tokens.
                </p>
                <div className="text-[10px] text-muted">
                  Log Path: <code className="text-foreground">~/.gemini/antigravity/brain/*/transcript.jsonl</code>
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-5">
                <div className="flex items-center justify-between mb-2">
                  <div className="font-bold text-foreground text-sm">Google Gemini CLI</div>
                  <span className="text-[10px] px-2 py-0.5 rounded bg-muted/20 text-muted border border-border font-semibold">
                    SUPPORTED
                  </span>
                </div>
                <p className="text-muted text-[11px] mb-3 leading-relaxed">
                  Tracks CLI commands, execution turns, and token counts from local Gemini configuration and history logs in <code className="text-foreground">~/.gemini/</code>.
                </p>
                <div className="text-[10px] text-muted">
                  Log Path: <code className="text-foreground">~/.gemini/logs/*</code>
                </div>
              </div>
            </div>
          </section>

          {/* Section 3: Privacy Invariants */}
          <section id="privacy" className="mb-14 scroll-mt-24">
            <h2 className="text-2xl font-bold text-foreground mb-4 font-mono">
              # Non-Negotiable Privacy Invariants
            </h2>
            <p className="text-muted text-sm leading-relaxed mb-4">
              FleetHUD is designed for defense, enterprise, and privacy-conscious engineering teams. We uphold 4 non-negotiable architectural guarantees:
            </p>

            <div className="space-y-3 font-mono text-xs">
              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-accent font-semibold mb-1">
                  1. Zero Content Storage
                </div>
                <div className="text-muted text-[11px] leading-relaxed">
                  Prompts, completions, code snippets, diffs, and tool output payloads NEVER touch disk or network. FleetHUD only persists numeric metadata: turn count, token counts, timestamps, status enums, and scrubbed project identifiers.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-accent font-semibold mb-1">
                  2. Local HMAC-SHA256 Project Masking
                </div>
                <div className="text-muted text-[11px] leading-relaxed">
                  Before any cloud synchronization (e.g., Supabase), local file paths are truncated to directory basenames, and full paths are irreversibly hashed using HMAC-SHA256 with a local machine key.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-accent font-semibold mb-1">
                  3. Fail-Open Architecture (50ms IPC Timeout)
                </div>
                <div className="text-muted text-[11px] leading-relaxed">
                  Integrations and hooks operate asynchronously. If the FleetHUD daemon is stopped or slow, agent execution never blocks.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-accent font-semibold mb-1">
                  4. Loopback-Only Network Binding
                </div>
                <div className="text-muted text-[11px] leading-relaxed">
                  The local REST/WebSocket daemon binds strictly to <code className="text-foreground">127.0.0.1:41738</code> and requires bearer token validation alongside Host/Origin header verification.
                </div>
              </div>
            </div>
          </section>

          {/* Section 4: CLI Reference */}
          <section id="cli" className="mb-14 scroll-mt-24">
            <h2 className="text-2xl font-bold text-foreground mb-4 font-mono">
              # CLI Command Reference
            </h2>
            <p className="text-muted text-sm leading-relaxed mb-4">
              The <code className="text-foreground">fleethud</code> binary provides fast terminal commands for local inspection:
            </p>

            <div className="space-y-4 font-mono text-xs">
              <div className="bg-card border border-border rounded-lg p-4">
                <div className="font-semibold text-accent mb-1">fleethud status</div>
                <div className="text-muted text-[11px] mb-2">
                  Inspect currently active and idle agent sessions across all installed tools.
                </div>
                <div className="bg-bg p-2.5 rounded border border-border text-foreground">
                  $ fleethud status<br />
                  Codex: 2 active (rollout-2026-10-26T17-17-47-01a17825, rollout-2026-10-26T11-51-26-01a176fa)<br />
                  Antigravity: 1 active (brain/b488ea6c-c99d-4b21-9bb7-e42a6d369a0c)<br />
                  Total Fleet: 3 active agents running
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="font-semibold text-accent mb-1">fleethud report [--range today|7d|30d|all]</div>
                <div className="text-muted text-[11px] mb-2">
                  Outputs complete API spend breakdown, prompt cache savings, and ROI multiplier against flat subscription plans.
                </div>
                <div className="bg-bg p-2.5 rounded border border-border text-foreground">
                  $ fleethud report --range 7d<br />
                  API-Equivalent Spend: $142.80<br />
                  Prompt Cache Savings: $48.20 (41% cached tokens)<br />
                  Anthropic Plan: $100/mo &rarr; 1.4x ROI<br />
                  OpenAI Plan: $20/mo &rarr; 4.2x ROI
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="font-semibold text-accent mb-1">fleethud daemon</div>
                <div className="text-muted text-[11px] mb-2">
                  Starts the local HTTP and WebSocket service on port 41738 for tray app and web console polling.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="font-semibold text-accent mb-1">fleethud install</div>
                <div className="text-muted text-[11px] mb-2">
                  Configures autostart, system tray, shortcuts, and background execution on Windows, macOS, or Linux.
                </div>
              </div>
            </div>
          </section>

          {/* Section 5: REST API */}
          <section id="api" className="mb-14 scroll-mt-24">
            <h2 className="text-2xl font-bold text-foreground mb-4 font-mono">
              # REST API Reference
            </h2>
            <p className="text-muted text-sm leading-relaxed mb-4">
              The daemon exposes a local JSON API on <code className="text-foreground">http://127.0.0.1:41738</code>:
            </p>

            <div className="space-y-3 font-mono text-xs">
              <div className="bg-card border border-border rounded-lg p-3.5">
                <div className="flex items-center gap-2 mb-1">
                  <span className="px-1.5 py-0.5 rounded bg-green/20 text-green font-semibold text-[10px]">GET</span>
                  <span className="text-foreground font-semibold">/api/v1/live</span>
                </div>
                <div className="text-muted text-[11px]">
                  Real-time 2-second snapshot containing active agents, KPI totals, quota pools, and recent sessions.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-3.5">
                <div className="flex items-center gap-2 mb-1">
                  <span className="px-1.5 py-0.5 rounded bg-green/20 text-green font-semibold text-[10px]">GET</span>
                  <span className="text-foreground font-semibold">/api/v1/report?range=all|today|7d|30d</span>
                </div>
                <div className="text-muted text-[11px]">
                  Aggregated spend, prompt cache savings, and ROI breakdowns by provider and model.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-3.5">
                <div className="flex items-center gap-2 mb-1">
                  <span className="px-1.5 py-0.5 rounded bg-green/20 text-green font-semibold text-[10px]">GET</span>
                  <span className="text-foreground font-semibold">/api/v1/quota</span>
                </div>
                <div className="text-muted text-[11px]">
                  Current per-model quota pools (used percentage, reset timestamps, window duration).
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-3.5">
                <div className="flex items-center gap-2 mb-1">
                  <span className="px-1.5 py-0.5 rounded bg-accent/20 text-accent font-semibold text-[10px]">POST</span>
                  <span className="text-foreground font-semibold">/api/v1/pricing/custom</span>
                </div>
                <div className="text-muted text-[11px]">
                  Override token pricing for specific models (input, output, and cache read rates per 1M tokens).
                </div>
              </div>
            </div>
          </section>

          {/* Section 6: FAQ */}
          <section id="faq" className="mb-14 scroll-mt-24">
            <h2 className="text-2xl font-bold text-foreground mb-4 font-mono">
              # Troubleshooting & FAQ
            </h2>

            <div className="space-y-4 font-mono text-xs">
              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-foreground font-semibold mb-1">
                  Q: Why did an agent turn status switch from "Thinking" to "Idle"?
                </div>
                <div className="text-muted text-[11px] leading-relaxed">
                  FleetHUD marks an agent session idle if no new turn events or tool calls are detected within 15 minutes. As soon as the agent begins a new step or generates a token, the session immediately snaps back to "Thinking" or "Tool".
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-foreground font-semibold mb-1">
                  Q: How is prompt cache savings computed?
                </div>
                <div className="text-muted text-[11px] leading-relaxed">
                  Prompt caching reduces token rates by up to 90%. FleetHUD calculates savings as: <br />
                  <code className="text-accent">(cached_tokens * raw_input_rate) - (cached_tokens * cached_rate)</code>.
                  This shows the direct dollar savings your subscription or API key realized.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-foreground font-semibold mb-1">
                  Q: Can I use FleetHUD completely offline?
                </div>
                <div className="text-muted text-[11px] leading-relaxed">
                  Yes. The local CLI, daemon, SQLite store, and tray app operate 100% locally with zero internet access required. Connecting to Supabase is strictly optional for cross-device roaming.
                </div>
              </div>
            </div>
          </section>

          {/* Bottom CTA */}
          <div className="border-t border-border pt-8 flex items-center justify-between flex-wrap gap-4 font-mono text-xs">
            <div>
              Ready to explore your live fleet?
            </div>
            <div className="flex gap-3">
              <Link
                href="/console"
                className="px-4 py-2 bg-accent text-[#0d1117] font-semibold rounded hover:opacity-90 transition-opacity"
              >
                Launch Console &rarr;
              </Link>
              <Link
                href="/pricing"
                className="px-4 py-2 bg-card border border-border text-foreground rounded hover:border-accent"
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

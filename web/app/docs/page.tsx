import type { Metadata } from "next";
import Link from "next/link";

export const metadata: Metadata = {
  title: "Documentation — Setup, Agents & API Reference",
  description:
    "Complete technical guide for spawnt: quickstart, agent support, CLI commands, and REST APIs.",
  alternates: {
    canonical: "https://fleethud.vercel.app/docs",
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
    url: "https://fleethud.vercel.app",
  },
  dependencies: "Windows 10/11, Linux x86_64 and ARM64, WSL2",
  articleSection: "Developer Tools",
};

export default function DocsPage() {
  const sections = [
    { id: "quickstart", title: "Quickstart (1 Min)" },
    { id: "supported-agents", title: "Supported Agents & Logs" },
    { id: "privacy", title: "Privacy Invariants" },
    { id: "cli", title: "CLI Command Reference" },
    { id: "api", title: "REST & WS API" },
    { id: "pricing-engine", title: "Cost & trust" },
    { id: "faq", title: "Troubleshooting & FAQ" },
  ];

  return (
    <div className="sp-console-shell sp-docs-page mx-auto max-w-7xl font-sans">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(docsJsonLd) }}
      />

      <div className="sp-docs-layout">
        <aside className="sp-docs-sidebar font-mono text-xs">
            <nav aria-label="On this page">
              {sections.map((s) => (
                <a
                  key={s.id}
                  href={`#${s.id}`}
                  className="sp-docs-nav-link"
                >
                  {s.title}
                </a>
              ))}
            </nav>
            <Link href="/downloads" className="sp-docs-download-link">Get spawnt <span aria-hidden="true">→</span></Link>
        </aside>

        <article className="sp-docs-article">
          <header className="sp-docs-header">
            <h1>
              spawnt Documentation
            </h1>
            <p>
              spawnt is live mission control for AI coding agents. It reads local agent activity and reports usage and quota without storing prompts, source code, or tool responses.
            </p>
          </header>

          {/* Section 1: Quickstart */}
          <section id="quickstart" className="mb-14 scroll-mt-24">
            <h2>
              Quickstart
            </h2>
            <p className="text-muted text-sm leading-relaxed mb-4">
              spawnt runs locally with no external runtime dependencies. Choose an available platform release below:
            </p>

            <div className="sp-docs-install-list font-mono text-xs">
              <div className="sp-docs-install">
                <header><strong>Windows</strong><span>AVAILABLE · x64</span></header>
                <code>irm https://fleethud.vercel.app/win | iex</code>
                <p>Downloads the v1.3.0 installer and creates Start menu and desktop shortcuts. Updates preserve recorded sessions and account pairing.</p>
              </div>
              <div className="sp-docs-install">
                <header><strong>Linux</strong><span>AVAILABLE · x86_64 / ARM64</span></header>
                <code>curl -fsSL https://fleethud.vercel.app/sh | bash</code>
                <p>Downloads the verified tarball, checks its SHA-256 digest, and installs the CLI. WSL2 is supported.</p>
              </div>
              <div className="sp-docs-install sp-docs-install-pending">
                <header><strong>macOS</strong><span>NOT PUBLISHED</span></header>
                <p>A verified macOS package is not available yet. Check the Downloads page for current release availability.</p>
                <Link href="/downloads">View downloads <span aria-hidden="true">→</span></Link>
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
            <h2>
              Supported agents & adapters
            </h2>
            <p className="text-muted text-sm leading-relaxed mb-6">
              spawnt reads local activity files from supported AI coding tools. It requires no configuration in the target agent.
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
              Non-Negotiable Privacy Invariants
            </h2>
            <p className="text-muted text-sm leading-relaxed mb-4">
              spawnt is designed for privacy-conscious developers. It upholds four non-negotiable architectural guarantees:
            </p>

            <div className="space-y-3 font-mono text-xs">
              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-accent font-semibold mb-1">Zero content storage</div>
                <div className="text-muted text-[11px] leading-relaxed">
                  Prompts, completions, code snippets, diffs, and tool output payloads NEVER touch disk or network. spawnt only persists metadata such as turn counts, token counts, timestamps, status, and scrubbed project identifiers.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-accent font-semibold mb-1">Project identifiers are minimized</div>
                <div className="text-muted text-[11px] leading-relaxed">
                  Before any cloud synchronization (e.g., Supabase), local file paths are truncated to directory basenames, and full paths are irreversibly hashed using HMAC-SHA256 with a local machine key.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-accent font-semibold mb-1">Fail-open integrations</div>
                <div className="text-muted text-[11px] leading-relaxed">
                  Integrations and hooks operate asynchronously. If the spawnt daemon is stopped or slow, agent execution never blocks.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-accent font-semibold mb-1">Loopback-only network binding</div>
                <div className="text-muted text-[11px] leading-relaxed">
                  The local REST/WebSocket daemon binds strictly to <code className="text-foreground">127.0.0.1:41738</code> and requires bearer token validation alongside Host/Origin header verification.
                </div>
              </div>
            </div>
          </section>

          {/* Section 4: CLI Reference */}
          <section id="cli" className="mb-14 scroll-mt-24">
            <h2>
              CLI commands
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
                  Shows current session states and activity metadata.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="font-semibold text-accent mb-1">fleethud report [--range today|7d|30d|all]</div>
                <div className="text-muted text-[11px] mb-2">
                  Outputs complete API spend breakdown, prompt cache savings, and ROI multiplier against flat subscription plans.
                </div>
                <div className="bg-bg p-2.5 rounded border border-border text-foreground">
                  $ fleethud report --range 7d<br />
                  Values include the trust level supplied by the local engine.
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
            <h2>
              REST API
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

              <div className="bg-card border border-border rounded-lg p-3.5">
                <div className="flex items-center gap-2 mb-1">
                  <span className="px-1.5 py-0.5 rounded bg-green/20 text-green font-semibold text-[10px]">GET</span>
                  <span className="text-foreground font-semibold">/api/v1/pricing/models</span>
                </div>
                <div className="text-muted text-[11px]">
                  Returns complete JSON catalog of all supported models, per-million token rates, and known context limits.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-3.5">
                <div className="flex items-center gap-2 mb-1">
                  <span className="px-1.5 py-0.5 rounded bg-accent/20 text-accent font-semibold text-[10px]">POST</span>
                  <span className="text-foreground font-semibold">/api/v1/pricing/refresh</span>
                </div>
                <div className="text-muted text-[11px]">
                  Explicitly refreshes the local rate catalog from the upstream registry. This request is opt-in.
                </div>
              </div>
            </div>
          </section>

          {/* Pricing and quota remain in the local Rust engine. */}
          <section id="pricing-engine" className="sp-docs-section mb-14 scroll-mt-24">
            <h2>Cost and trust</h2>
            <p>spawnt calculates token cost, cache savings, quota usage, and rollups in its local Rust engine. This guide does not duplicate model rates or compute totals.</p>
            <h3>Reading model and pricing labels</h3>
            <dl className="sp-docs-label-list">
              <div><dt>Frontier</dt><dd>A model family or capability tier label. It is not a price or a measurement-confidence level.</dd></div>
              <div><dt>Tiered</dt><dd>The provider rate depends on a tier or request conditions such as context length, volume, or service level. A rate-based cost remains estimated unless usage is reconciled against an API or invoice.</dd></div>
              <div><dt>Exact</dt><dd>In a provider pricing catalog, the rate is listed by the provider for the named model and conditions. In spawnt's live views, <code>EXACT</code> separately means the measurement came directly from an exact local record or provider limit response.</dd></div>
            </dl>
            <p><strong>Provider family matters:</strong> Claude Fable 5 and Claude Fable 5.1 are Anthropic Claude models, never Gemini models. See <a href="https://www.anthropic.com/claude/fable" target="_blank" rel="noreferrer">Anthropic's Claude Fable page</a> for current model details.</p>
            <div className="sp-docs-trust-grid" aria-label="Measurement trust levels">
              <div><strong>EXACT</strong><p>Measured from local usage records or an exact provider limit response.</p></div>
              <div><strong>EST</strong><p>Calculated from a rate catalog when provider usage is not an invoice.</p></div>
              <div><strong>API</strong><p>Reported directly by an opted-in provider API.</p></div>
              <div><strong>UNKNOWN</strong><p>Shown when a value cannot be measured reliably; it is never replaced with zero.</p></div>
            </div>
            <p>API-equivalent cost estimates public model rates; it is not a subscription invoice. Unknown model prices remain unknown. Any remote pricing refresh is opt-in; normal local collection does not contact the pricing registry.</p>
          </section>
          {/* Section 7: FAQ */}
          <section id="faq" className="mb-14 scroll-mt-24">
            <h2>
              Troubleshooting & FAQ
            </h2>

            <div className="space-y-4 font-mono text-xs">
              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-foreground font-semibold mb-1">
                  Q: Why did an agent turn status switch from "Thinking" to "Idle"?
                </div>
                <div className="text-muted text-[11px] leading-relaxed">
                  spawnt marks an agent session idle if no new turn events or tool calls are detected within 15 minutes. New activity changes the session state again.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-foreground font-semibold mb-1">
                  Q: How is prompt cache savings computed?
                </div>
                <div className="text-muted text-[11px] leading-relaxed">
                  Prompt caching may reduce provider token rates. The local Rust pricing engine calculates cache savings using the selected model's rates: <br />
                  <code className="text-accent">(cached_tokens * raw_input_rate) - (cached_tokens * cached_rate)</code>.
                  The result is API-equivalent savings, not a subscription invoice or a guaranteed billed amount.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-foreground font-semibold mb-1">
                  Q: Can I use spawnt completely offline?
                </div>
                <div className="text-muted text-[11px] leading-relaxed">
                  Yes. The local CLI, daemon, SQLite store, and tray app operate 100% locally with zero internet access required. Connecting to Supabase is strictly optional for cross-device roaming.
                </div>
              </div>
            </div>
          </section>

          {/* Bottom CTA */}
          <div className="sp-docs-cta">
            <p>
              Ready to explore your live fleet?
            </p>
            <div className="sp-docs-cta-actions">
              <Link
                href="/console"
                className="sp-docs-cta-primary"
              >
                Launch Console &rarr;
              </Link>
              <Link
                href="/pricing"
                className="sp-docs-cta-secondary"
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

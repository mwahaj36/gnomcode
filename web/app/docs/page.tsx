import { pageMetadata } from "../../lib/seo";
import branding from "../../branding.json";
import Link from "next/link";
import { CopyCommand } from "../components/CopyCommand";
import { PlatformDownloads } from "../downloads/PlatformDownloads";

export const metadata = pageMetadata("/docs", "Gnomcode Documentation", "Install Gnomcode, launch AI coding agents, switch models and pair your phone. Guides for agent support, cost, quota and privacy.");

const docsJsonLd = {
  "@context": "https://schema.org",
  "@type": "TechArticle",
  headline: "gnomcode Technical Documentation and Architecture Guide",
  description:
    "Complete manual for monitoring AI coding agents: setup, log adapters, CLI usage, and REST API.",
  author: {
    "@type": "Organization",
    name: "gnomcode",
    url: branding.baseUrl,
  },
  dependencies: "Windows x64, macOS (arm64 & x86_64), and Linux (x86_64 & arm64)",
  articleSection: "Developer Tools",
};

export default function DocsPage() {
  const sections = [
    { id: "quickstart", title: "Getting started" },
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
            <Link href="/downloads" className="sp-docs-download-link">Get gnomcode <span aria-hidden="true">→</span></Link>
        </aside>

        <article className="sp-docs-article">
          <header className="sp-docs-header">
            <h1>
              gnomcode Documentation
            </h1>
            <p>
              Gnomcode is a local-first agent orchestrator. Launch supported official CLIs, work in redacted local conversations, switch with a compact handoff and pair a device for encrypted Remote.
            </p>
          </header>

          {/* Section 1: Quickstart */}
          <section id="guides"><h2>Gnomcode guides</h2><p>Choose a workflow, then check capability and release details.</p><ul className="sp-guide-related"><li><Link href="/docs/getting-started">Getting started</Link></li><li><Link href="/docs/model-switching">Model switching</Link></li><li><Link href="/docs/remote-access">Remote access</Link></li><li><Link href="/docs/supported-agents">Supported agents</Link></li><li><Link href="/docs/quota-and-cost">Quota and cost</Link></li><li><Link href="/docs/model-pricing">Model pricing</Link></li><li><Link href="/docs/automate-multiple-agents">Automate multiple agents</Link></li><li><Link href="/docs/prompt-from-phone">Prompt from phone</Link></li><li><Link href="/docs/faq">FAQ</Link></li><li><Link href="/docs/quota-limits-explained">Quota limits explained</Link></li><li><Link href="/docs/privacy">Privacy</Link></li><li><Link href="/docs/claude-code-vs-codex">Claude Code vs Codex</Link></li></ul></section>
          <section id="quickstart" className="mb-14 scroll-mt-24">
            <h2>
              Quickstart
            </h2>
            <p className="text-muted text-sm leading-relaxed mb-4">
              gnomcode runs locally with no external runtime dependencies. Check the platform guide for current release availability:
            </p>

            <PlatformDownloads />

            <p className="text-muted text-xs mt-4">
              Once installed, verify the installation by running:
            </p>
            <CopyCommand command="gnomcode status" />
          </section>

          {/* Section 2: Supported Agents */}
          <section id="supported-agents" className="mb-14 scroll-mt-24">
            <h2>
              Supported agents & adapters
            </h2>
            <p className="text-muted text-sm leading-relaxed mb-6">
              gnomcode 1.0 supports exactly Claude Code, OpenAI Codex and Google Antigravity. It reads their local activity files and needs no configuration in the agent for observation.
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
                  Extracts token usage, cached input reads, and thinking tokens from <code className="text-foreground">~/.claude/projects/</code>. Set your Claude subscription plan manually in Settings. By default gnomcode does not read Claude credentials; the optional usage-limit source reads Claude Code&apos;s own sign-in only after you enable it.
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
                  Parses <code className="text-foreground">~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl</code> with session UUID attribution. Reads recorded 5-hour and weekly rate-limit windows, turn token usage, and mid-session model changes.
                </p>
                <div className="text-[10px] text-muted">
                  Log Path: <code className="text-foreground">~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl</code>
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-5">
                <div className="flex items-center justify-between mb-2">
                  <div className="font-bold text-foreground text-sm">Google Antigravity</div>
                  <span className="text-[10px] px-2 py-0.5 rounded bg-accent/15 text-accent border border-accent/30 font-semibold">
                    TRANSCRIPTS
                  </span>
                </div>
                <p className="text-muted text-[11px] mb-3 leading-relaxed">
                  Monitors active conversation transcripts in <code className="text-foreground">~/.gemini/antigravity/brain/</code>. Tracks step indexes, tool call execution states (Running/Waiting), model transitions, and token fields where recorded. Antigravity uses Google&apos;s Gemini models.
                </p>
                <div className="text-[10px] text-muted">
                  Log Path: <code className="text-foreground">~/.gemini/antigravity/brain/*/transcript.jsonl</code>
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
              gnomcode is designed for privacy-conscious developers. It upholds four non-negotiable architectural guarantees:
            </p>

            <div className="space-y-3 font-mono text-xs">
              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-accent font-semibold mb-1">Local-first history</div>
                <div className="text-muted text-[11px] leading-relaxed">
                  Gnomcode stores redacted local chat for continuity. Chat events are excluded from non-E2EE account metadata sync. Paired Remote carries conversations over a separate encrypted channel.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-accent font-semibold mb-1">Project identifiers are minimized</div>
                <div className="text-muted text-[11px] leading-relaxed">
                  The live account snapshot includes project/session labels and automation metadata. HMAC rollup helpers do not make it hash-only. Read the privacy guide before enabling sync; field-level minimization remains a launch gate.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-accent font-semibold mb-1">Fail-open integrations</div>
                <div className="text-muted text-[11px] leading-relaxed">
                  Integrations and hooks operate asynchronously. If the gnomcode daemon is stopped or slow, agent execution never blocks.
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
              The gnomcode CLI provides fast terminal commands for local inspection:
            </p>

            <div className="space-y-4 font-mono text-xs">
              <div className="bg-card border border-border rounded-lg p-4">
                <CopyCommand command="gnomcode status" />
                <div className="text-muted text-[11px] mb-2">
                  Inspect currently active and idle agent sessions across all installed tools.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <CopyCommand command="gnomcode report --range 7d" />
                <div className="text-muted text-[11px] mb-2">
                  Shows a seven-day API-equivalent usage report with trust levels supplied by the local engine.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <CopyCommand command="gnomcode daemon" />
                <div className="text-muted text-[11px] mb-2">
                  Starts the local HTTP and WebSocket service on port 41738 for tray app and web console polling.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <CopyCommand command="gnomcode install" />
                <div className="text-muted text-[11px] mb-2">
                  Configures autostart, the system tray and shortcuts for the current platform on Windows, macOS and Linux.
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
                  <span className="px-1.5 py-0.5 rounded bg-card text-foreground border border-border font-semibold text-[10px]">GET</span>
                  <span className="text-foreground font-semibold">/api/v1/live</span>
                </div>
                <div className="text-muted text-[11px]">
                  Live snapshot containing active agents, KPI totals, quota pools, and recent sessions.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-3.5">
                <div className="flex items-center gap-2 mb-1">
                  <span className="px-1.5 py-0.5 rounded bg-card text-foreground border border-border font-semibold text-[10px]">GET</span>
                  <span className="text-foreground font-semibold">/api/v1/report?range=all|today|7d|30d</span>
                </div>
                <div className="text-muted text-[11px]">
                  Aggregated spend, prompt cache savings, and ROI breakdowns by provider and model.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-3.5">
                <div className="flex items-center gap-2 mb-1">
                  <span className="px-1.5 py-0.5 rounded bg-card text-foreground border border-border font-semibold text-[10px]">GET</span>
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
                  <span className="px-1.5 py-0.5 rounded bg-card text-foreground border border-border font-semibold text-[10px]">GET</span>
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
            <p>gnomcode calculates token costs, cache savings and quota usage locally. Costs use the rate catalog available on your desktop; unpriced usage is identified separately.</p>
            <h3>Reading model and pricing labels</h3>
            <dl className="sp-docs-label-list">
              <div><dt>Frontier</dt><dd>A model family or capability tier label. It is not a price or a measurement-confidence level.</dd></div>
              <div><dt>Tiered</dt><dd>The provider rate depends on a tier or request conditions such as context length, volume, or service level. A rate-based cost remains estimated unless usage is reconciled against an API or invoice.</dd></div>
              <div><dt>Exact</dt><dd>In a provider pricing catalog, the rate is listed by the provider for the named model and conditions. In gnomcode&apos;s live views, <code>EXACT</code> separately means the measurement came directly from an exact local record or provider limit response.</dd></div>
            </dl>
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
                  Q: Why did an agent turn status switch from &quot;Thinking&quot; to &quot;Idle&quot;?
                </div>
                <div className="text-muted text-[11px] leading-relaxed">
                  gnomcode stops showing a session as Running when its agent has recorded no recent activity or an open tool step has gone stale. New activity changes the session state again.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-foreground font-semibold mb-1">
                  Q: How is prompt cache savings computed?
                </div>
                <div className="text-muted text-[11px] leading-relaxed">
                  Prompt caching may reduce provider token rates. The local Rust pricing engine calculates cache savings using the selected model&apos;s rates: <br />
                  <code className="text-accent">(cached_tokens * raw_input_rate) - (cached_tokens * cached_rate)</code>.
                  The result is API-equivalent savings, not a subscription invoice or a guaranteed billed amount.
                </div>
              </div>

              <div className="bg-card border border-border rounded-lg p-4">
                <div className="text-foreground font-semibold mb-1">
                  Q: Can I use gnomcode completely offline?
                </div>
                <div className="text-muted text-[11px] leading-relaxed">
                  Yes. The local CLI, daemon, SQLite store, and tray app work without a Gnomcode account or internet access. Agent CLIs still need their own provider network. Account sync is optional for viewing desktop activity across devices.
                </div>
              </div>
            </div>
          </section>

          <section id="legal"><h2>Legal</h2><p><Link href="/privacy">Privacy</Link> and <Link href="/terms">Terms of Service</Link> describe storage, optional services and licensing.</p></section>
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

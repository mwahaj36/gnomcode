import Link from "next/link";

export default function MarketingFooter() {
  return (
    <footer className="border-t border-border bg-[#0d1117] pt-16 pb-12 text-xs font-mono text-muted">
      <div className="max-w-7xl mx-auto px-6">
        <div className="grid grid-cols-2 sm:grid-cols-2 md:grid-cols-4 gap-10 mb-12">
          {/* Column 1: Product */}
          <div>
            <div className="font-semibold text-foreground tracking-wide uppercase text-[11px] mb-4">
              Product
            </div>
            <ul className="space-y-2.5">
              <li>
                <Link href="/console" className="hover:text-accent transition-colors">
                  Mission Control Console
                </Link>
              </li>
              <li>
                <Link href="/pricing" className="hover:text-accent transition-colors">
                  Pricing & Plans
                </Link>
              </li>
              <li>
                <Link href="/downloads" className="hover:text-accent transition-colors">
                  Desktop Tray App
                </Link>
              </li>
              <li>
                <Link href="/docs" className="hover:text-accent transition-colors">
                  Rate Limit Quota Pools
                </Link>
              </li>
              <li>
                <Link href="/docs#roi" className="hover:text-accent transition-colors">
                  Subscription ROI Tracker
                </Link>
              </li>
            </ul>
          </div>

          {/* Column 2: Supported Agents */}
          <div>
            <div className="font-semibold text-foreground tracking-wide uppercase text-[11px] mb-4">
              Supported Agents
            </div>
            <ul className="space-y-2.5">
              <li>
                <Link href="/docs#claude" className="hover:text-accent transition-colors">
                  Anthropic Claude Code
                </Link>
              </li>
              <li>
                <Link href="/docs#codex" className="hover:text-accent transition-colors">
                  OpenAI Codex CLI & App
                </Link>
              </li>
              <li>
                <Link href="/docs#antigravity" className="hover:text-accent transition-colors">
                  Google Antigravity
                </Link>
              </li>
              <li>
                <Link href="/docs#gemini" className="hover:text-accent transition-colors">
                  Google Gemini CLI
                </Link>
              </li>
              <li>
                <Link href="/downloads" className="hover:text-accent transition-colors">
                  Desktop Downloads
                </Link>
              </li>
            </ul>
          </div>

          {/* Column 3: Documentation & Open Source */}
          <div>
            <div className="font-semibold text-foreground tracking-wide uppercase text-[11px] mb-4">
              Resources & Docs
            </div>
            <ul className="space-y-2.5">
              <li>
                <Link href="/docs" className="hover:text-accent transition-colors">
                  Quickstart Guide
                </Link>
              </li>
              <li>
                <Link href="/docs#privacy" className="hover:text-accent transition-colors">
                  Privacy Invariants
                </Link>
              </li>
              <li>
                <Link href="/docs#cli" className="hover:text-accent transition-colors">
                  CLI Reference
                </Link>
              </li>
              <li>
                <Link href="/docs#api" className="hover:text-accent transition-colors">
                  REST & WS API
                </Link>
              </li>
              <li>
                <Link href="/downloads" className="hover:text-accent transition-colors">
                  Download v1.1.1
                </Link>
              </li>
            </ul>
          </div>

          {/* Column 4: Privacy & Guarantees */}
          <div>
            <div className="font-semibold text-foreground tracking-wide uppercase text-[11px] mb-4">
              Non-Negotiable Invariants
            </div>
            <p className="text-[11px] text-muted leading-relaxed mb-3">
              FleetHUD never writes prompts, source code, or tool responses to disk or network. Pure telemetry attribution with zero content retention.
            </p>
            <div className="inline-flex items-center gap-2 px-2.5 py-1 rounded bg-green/10 border border-green/30 text-green text-[10px]">
              <span className="w-1.5 h-1.5 rounded-full bg-green"></span>
              <span>100% PRIVATE & LOOPBACK</span>
            </div>
          </div>
        </div>

        <div className="pt-8 border-t border-border/60 flex flex-col sm:flex-row items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <div className="w-5 h-5 rounded bg-[#0d1117] p-0.5 flex items-center justify-center">
              <img src="/fleethud.svg" alt="FleetHUD Logo" className="w-full h-full object-contain" />
            </div>
            <span className="font-bold text-foreground">FleetHUD</span>
            <span>&bull;</span>
            <span>v1.1.1 Production Release</span>
            <span>&bull;</span>
            <span className="text-green">All Systems Operational</span>
          </div>

          <div className="flex items-center gap-6">
            <Link href="/docs#privacy" className="hover:text-foreground transition-colors">
              Privacy Policy
            </Link>
            <Link href="/pricing" className="hover:text-foreground transition-colors">
              Terms of Service
            </Link>
            <Link href="/downloads" className="hover:text-foreground transition-colors">
              Downloads
            </Link>
          </div>
        </div>
      </div>
    </footer>
  );
}

import Link from "next/link";
import { BRAND } from "@/lib/brand";

export default function MarketingFooter() {
  return (
    <footer className="border-t border-border bg-background pt-16 pb-12 text-xs font-mono text-muted">
      <div className="max-w-7xl mx-auto px-6">
        <div className="grid grid-cols-2 sm:grid-cols-2 md:grid-cols-4 gap-10 mb-12">
          {/* Column 1: Product */}
          <div>
            <div className="font-semibold text-foreground tracking-wider uppercase text-[11px] mb-4">
              PRODUCT //
            </div>
            <ul className="space-y-2.5">
              <li>
                <Link href="/console" className="hover:text-primary transition-colors">
                  Console
                </Link>
              </li>
              <li>
                <Link href="/pricing" className="hover:text-primary transition-colors">
                  Pricing & Plans
                </Link>
              </li>
              <li>
                <Link href="/downloads" className="hover:text-primary transition-colors">
                  Desktop App
                </Link>
              </li>
              <li>
                <Link href="/docs" className="hover:text-primary transition-colors">
                  Rate Limit Quota Pools
                </Link>
              </li>
              <li>
                <Link href="/docs#roi" className="hover:text-primary transition-colors">
                  Subscription ROI Tracker
                </Link>
              </li>
            </ul>
          </div>

          {/* Column 2: Supported Agents */}
          <div>
            <div className="font-semibold text-foreground tracking-wider uppercase text-[11px] mb-4">
              AGENTS //
            </div>
            <ul className="space-y-2.5">
              <li>
                <Link href="/docs#claude" className="hover:text-primary transition-colors">
                  Claude Code
                </Link>
              </li>
              <li>
                <Link href="/docs#codex" className="hover:text-primary transition-colors">
                  OpenAI Codex
                </Link>
              </li>
              <li>
                <Link href="/docs#antigravity" className="hover:text-primary transition-colors">
                  Google Antigravity
                </Link>
              </li>
              <li>
                <Link href="/docs#gemini" className="hover:text-primary transition-colors">
                  Google Gemini CLI
                </Link>
              </li>
              <li>
                <Link href="/docs#cursor" className="hover:text-primary transition-colors">
                  Cursor & Roo Code
                </Link>
              </li>
            </ul>
          </div>

          {/* Column 3: Documentation */}
          <div>
            <div className="font-semibold text-foreground tracking-wider uppercase text-[11px] mb-4">
              RESOURCES //
            </div>
            <ul className="space-y-2.5">
              <li>
                <Link href="/docs" className="hover:text-primary transition-colors">
                  Documentation
                </Link>
              </li>
              <li>
                <Link href="/docs#privacy" className="hover:text-primary transition-colors">
                  Privacy Invariants
                </Link>
              </li>
              <li>
                <Link href="/docs#cli" className="hover:text-primary transition-colors">
                  CLI Reference
                </Link>
              </li>
              <li>
                <Link href="/docs#api" className="hover:text-primary transition-colors">
                  REST & WS API
                </Link>
              </li>
              <li>
                <Link href="/downloads" className="hover:text-primary transition-colors">
                  Downloads
                </Link>
              </li>
            </ul>
          </div>

          {/* Column 4: Privacy & Guarantees */}
          <div>
            <div className="font-semibold text-foreground tracking-wider uppercase text-[11px] mb-4">
              INVARIANTS //
            </div>
            <p className="text-[11px] text-muted leading-relaxed mb-3">
              {BRAND.privacyInvariant}
            </p>
            <div className="inline-flex items-center gap-2 px-2.5 py-1 rounded-[4px] bg-surface-elevated border border-border text-foreground text-[10px]">
              <span className="w-1.5 h-1.5 rounded-[1px] bg-primary"></span>
              <span>100% PRIVATE & LOOPBACK</span>
            </div>
          </div>
        </div>

        <div className="pt-8 border-t border-border flex flex-col sm:flex-row items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <svg
              className="w-4 h-4 text-foreground flex-shrink-0"
              viewBox="0 0 2048 2048"
              fill="currentColor"
              xmlns="http://www.w3.org/2000/svg"
              aria-hidden="true"
            >
              <polygon points="290.67,1756.53 1757.33,1756.53 1757.33,291.47" />
              <rect x="799.39" y="789.88" width="449.23" height="449.23" />
            </svg>
            <span className="font-mono font-bold lowercase text-foreground tracking-tight">
              {BRAND.name}
            </span>
            <span className="text-muted">/</span>
            <span className="text-muted">v{BRAND.version}</span>
            <span className="text-muted">/</span>
            <span className="text-muted">LOCAL-FIRST</span>
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

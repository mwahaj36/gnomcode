"use client";

import { useState } from "react";
import Link from "next/link";
import { BRAND } from "@/lib/brand";

const AGENTS = [
  { name: "Claude Code", vendor: "Anthropic", tag: "CLI" },
  { name: "OpenAI Codex", vendor: "OpenAI", tag: "CLI + APP" },
  { name: "Google Antigravity", vendor: "Google", tag: "IDE" },
  { name: "Google Gemini CLI", vendor: "Google", tag: "CLI" },
  { name: "Cursor", vendor: "Anysphere", tag: "EDITOR" },
  { name: "Roo Code", vendor: "Roo Vet", tag: "EXT" },
  { name: "Cline", vendor: "Cline Bot", tag: "EXT" },
  { name: "Aider", vendor: "Paul Gauthier", tag: "CLI" },
];

export default function LandingPage() {
  const [copied, setCopied] = useState(false);
  const installCmd = "npx spawnt@latest";

  const copyCommand = () => {
    navigator.clipboard.writeText(installCmd);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="max-w-7xl mx-auto px-6 py-16 space-y-24">
      {/* Hero Section */}
      <section className="text-center max-w-4xl mx-auto pt-6">
        <div className="inline-flex items-center gap-2 px-3 py-1 rounded-[4px] bg-surface border border-border text-xs font-mono mb-8">
          <span className="w-2 h-2 rounded-[1px] bg-primary animate-pulse"></span>
          <span className="text-muted">LOCAL-FIRST</span>
          <span className="text-muted">/</span>
          <span className="text-foreground">NO CONTENT STORED</span>
          <span className="text-muted">/</span>
          <span className="text-primary font-bold">v{BRAND.version}</span>
        </div>

        <h1 className="text-4xl sm:text-6xl font-bold tracking-tight text-foreground mb-6 font-mono leading-tight">
          {BRAND.tagline}
        </h1>

        <p className="text-base sm:text-lg text-muted max-w-2xl mx-auto mb-10 font-sans leading-relaxed">
          {BRAND.subline}
        </p>

        {/* Action Buttons */}
        <div className="flex flex-wrap items-center justify-center gap-4 font-mono text-xs mb-8">
          <Link
            href="/console"
            className="h-10 px-6 rounded-[4px] bg-primary text-white font-medium hover:brightness-110 transition-all flex items-center gap-2"
          >
            Launch Console
            <span>&rarr;</span>
          </Link>
          <Link
            href="/downloads"
            className="h-10 px-6 rounded-[4px] bg-surface border border-border text-foreground hover:border-primary transition-colors flex items-center gap-2"
          >
            Download Desktop App
            <span>&darr;</span>
          </Link>
        </div>

        {/* Quick CLI command */}
        <div className="inline-flex items-center gap-3 px-4 py-2 rounded-[4px] bg-surface-elevated border border-border font-mono text-xs text-muted">
          <span className="text-primary">$</span>
          <code className="text-foreground">{installCmd}</code>
          <button
            onClick={copyCommand}
            className="ml-2 text-[10px] uppercase px-2 py-0.5 rounded-[4px] border border-border bg-surface hover:text-primary hover:border-primary transition-colors"
          >
            {copied ? "[COPIED]" : "[COPY]"}
          </button>
        </div>
      </section>

      {/* Agent Marquee Strip */}
      <section className="border-y border-border py-4 overflow-hidden relative">
        <div className="max-w-7xl mx-auto px-6 mb-3 flex items-center justify-between font-mono text-[11px] text-muted tracking-wider uppercase">
          <span>WORKS WITH ALL LEADING CODING AGENTS //</span>
          <span>AUTODISCOVERED LOCALLY</span>
        </div>
        <div className="relative w-full overflow-hidden mask-fade-edges">
          <div className="animate-marquee flex items-center gap-8 py-2">
            {[...AGENTS, ...AGENTS].map((agent, i) => (
              <div
                key={`${agent.name}-${i}`}
                className="group flex items-center gap-3 px-4 py-2 rounded-[4px] border border-border bg-surface hover:border-primary transition-colors cursor-pointer"
              >
                <div className="w-2 h-2 rounded-[1px] bg-border group-hover:bg-primary transition-colors"></div>
                <span className="font-mono text-xs font-semibold text-muted group-hover:text-primary transition-colors">
                  {agent.name}
                </span>
                <span className="font-mono text-[10px] text-muted/70 group-hover:text-primary/70 transition-colors">
                  [{agent.tag}]
                </span>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Console Preview Card (Daytona Hairline Strip) */}
      <section className="border border-border rounded-[6px] bg-surface overflow-hidden">
        {/* Top title bar */}
        <div className="h-9 border-b border-border bg-surface-elevated px-4 flex items-center justify-between font-mono text-xs text-muted">
          <div className="flex items-center gap-3">
            <span className="w-2 h-2 rounded-[1px] bg-primary"></span>
            <span className="text-foreground font-semibold">spawnt // console preview</span>
          </div>
          <div className="flex items-center gap-4 text-[11px]">
            <span>LOOPBACK 127.0.0.1:41738</span>
            <span>TOKEN SECURED</span>
          </div>
        </div>

        {/* Hairline 4-cell metric strip */}
        <div className="grid grid-cols-2 md:grid-cols-4 border-b border-border font-mono">
          <div className="p-4 border-r border-border border-b md:border-b-0">
            <div className="text-[10px] uppercase text-muted tracking-wider">ACTIVE AGENTS //</div>
            <div className="text-2xl font-bold text-foreground mt-1">3 RUNNING</div>
            <div className="text-[10px] text-muted mt-1">[EXACT] &bull; 0 WAITING APPROVAL</div>
          </div>
          <div className="p-4 border-r md:border-r border-border border-b md:border-b-0">
            <div className="text-[10px] uppercase text-muted tracking-wider">TODAY TOKEN SPEND //</div>
            <div className="text-2xl font-bold text-primary mt-1">2.41M TOKENS</div>
            <div className="text-[10px] text-muted mt-1">[EXACT] &bull; \$4.12 ACCRUED</div>
          </div>
          <div className="p-4 border-r border-border">
            <div className="text-[10px] uppercase text-muted tracking-wider">CLAUDE 5H QUOTA //</div>
            <div className="text-2xl font-bold text-foreground mt-1">68% REMAINING</div>
            <div className="text-[10px] text-muted mt-1">[EST] &bull; RESETS IN 1H 42M</div>
          </div>
          <div className="p-4">
            <div className="text-[10px] uppercase text-muted tracking-wider">CODEX 7D LIMIT //</div>
            <div className="text-2xl font-bold text-foreground mt-1">84% REMAINING</div>
            <div className="text-[10px] text-muted mt-1">[API] &bull; NO BOTTLENECKS</div>
          </div>
        </div>

        {/* Live sessions table preview */}
        <div className="overflow-x-auto p-4 font-mono text-xs">
          <table className="w-full text-left">
            <thead>
              <tr className="border-b border-border text-[11px] text-muted uppercase">
                <th className="pb-2 font-normal">AGENT //</th>
                <th className="pb-2 font-normal">SESSION //</th>
                <th className="pb-2 font-normal">MODEL //</th>
                <th className="pb-2 font-normal">STATE //</th>
                <th className="pb-2 font-normal text-right">TOKENS //</th>
                <th className="pb-2 font-normal text-right">COST //</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border/60">
              <tr>
                <td className="py-2.5 font-bold text-foreground">Claude Code</td>
                <td className="py-2.5 text-muted">cli:cc-9f4a</td>
                <td className="py-2.5 text-muted">claude-3-7-sonnet</td>
                <td className="py-2.5">
                  <span className="px-1.5 py-0.5 rounded-[4px] bg-primary/10 border border-primary/30 text-primary text-[10px]">
                    [RUN] tool:read_file
                  </span>
                </td>
                <td className="py-2.5 text-right text-foreground">1,482,910</td>
                <td className="py-2.5 text-right font-bold text-foreground">\$2.48</td>
              </tr>
              <tr>
                <td className="py-2.5 font-bold text-foreground">OpenAI Codex</td>
                <td className="py-2.5 text-muted">app:cdx-881b</td>
                <td className="py-2.5 text-muted">gpt-4o</td>
                <td className="py-2.5">
                  <span className="px-1.5 py-0.5 rounded-[4px] bg-surface-elevated border border-border text-foreground text-[10px]">
                    [WAIT] prompt_input
                  </span>
                </td>
                <td className="py-2.5 text-right text-foreground">620,114</td>
                <td className="py-2.5 text-right font-bold text-foreground">\$1.12</td>
              </tr>
              <tr>
                <td className="py-2.5 font-bold text-foreground">Google Antigravity</td>
                <td className="py-2.5 text-muted">ide:agy-24c1</td>
                <td className="py-2.5 text-muted">gemini-2.5-pro</td>
                <td className="py-2.5">
                  <span className="px-1.5 py-0.5 rounded-[4px] bg-surface-elevated border border-border text-muted text-[10px]">
                    [IDLE] completed
                  </span>
                </td>
                <td className="py-2.5 text-right text-foreground">308,400</td>
                <td className="py-2.5 text-right font-bold text-foreground">\$0.52</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      {/* 4-Feature Hairline Grid */}
      <section>
        <div className="text-center max-w-xl mx-auto mb-12">
          <div className="font-mono text-xs text-primary font-bold uppercase tracking-wider mb-2">
            DESIGNED FOR ENGINEERING TEAMS //
          </div>
          <h2 className="text-3xl font-bold font-mono tracking-tight text-foreground">
            Zero noise. Full visibility.
          </h2>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-px bg-border border border-border rounded-[6px] overflow-hidden">
          <div className="p-8 bg-surface space-y-3">
            <div className="font-mono text-xs text-primary font-bold">01 // LIVE STATUS</div>
            <h3 className="font-mono text-lg font-bold text-foreground">See it live</h3>
            <p className="text-sm text-muted font-sans leading-relaxed">
              Tail multiple CLI agents, IDE plugins, and desktop coding apps in a single unified view. Watch turn progress, active tool executions, and file modifications token by token.
            </p>
          </div>

          <div className="p-8 bg-surface space-y-3">
            <div className="font-mono text-xs text-primary font-bold">02 // FINANCIAL ATTRIBUTION</div>
            <h3 className="font-mono text-lg font-bold text-foreground">Know the cost</h3>
            <p className="text-sm text-muted font-sans leading-relaxed">
              Every turn calculates exact input, cache write, cache read, output, and reasoning tokens. Tag costs by repository, git branch, and feature.
            </p>
          </div>

          <div className="p-8 bg-surface space-y-3">
            <div className="font-mono text-xs text-primary font-bold">03 // RATE LIMIT PROTECTION</div>
            <h3 className="font-mono text-lg font-bold text-foreground">Stay under limits</h3>
            <p className="text-sm text-muted font-sans leading-relaxed">
              Track independent 5-hour and 7-day quota windows for Claude Code, Codex, and Gemini. Know your exact headroom before hitting a mid-task rate limit cliff.
            </p>
          </div>

          <div className="p-8 bg-surface space-y-3">
            <div className="font-mono text-xs text-primary font-bold">04 // LOCAL-FIRST PRIVACY</div>
            <h3 className="font-mono text-lg font-bold text-foreground">Zero content stored</h3>
            <p className="text-sm text-muted font-sans leading-relaxed">
              No prompts, source code, or tool responses ever touch disk or the network. All metrics are aggregated purely from local metadata and bound to loopback.
            </p>
          </div>
        </div>
      </section>

      {/* Pricing Teaser */}
      <section className="border border-border rounded-[6px] bg-surface p-8 sm:p-12">
        <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-6 border-b border-border pb-8 mb-8">
          <div>
            <div className="font-mono text-xs text-primary font-bold uppercase tracking-wider mb-2">
              SIMPLE PRICING //
            </div>
            <h2 className="text-2xl sm:text-3xl font-bold font-mono text-foreground">
              Free forever locally. Upgrade for teams.
            </h2>
          </div>
          <Link
            href="/pricing"
            className="h-10 px-5 rounded-[4px] bg-surface-elevated border border-border text-foreground hover:border-primary font-mono text-xs font-semibold flex items-center gap-2"
          >
            View all plans &rarr;
          </Link>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 font-mono">
          <div className="p-6 rounded-[4px] border border-border bg-surface-elevated">
            <div className="text-xs text-muted">COMMUNITY //</div>
            <div className="text-2xl font-bold text-foreground mt-2">\$0</div>
            <div className="text-[11px] text-muted mb-4">Free forever</div>
            <ul className="text-xs space-y-2 text-muted">
              <li>&bull; All local adapters</li>
              <li>&bull; Full console & CLI</li>
              <li>&bull; Unlimited local history</li>
            </ul>
          </div>

          <div className="p-6 rounded-[4px] border border-primary bg-surface-elevated relative">
            <div className="text-xs text-primary font-bold">PRO //</div>
            <div className="text-2xl font-bold text-foreground mt-2">\$15</div>
            <div className="text-[11px] text-muted mb-4">Per month</div>
            <ul className="text-xs space-y-2 text-foreground">
              <li>&bull; Everything in Free</li>
              <li>&bull; Real-time budget alerts</li>
              <li>&bull; Multi-machine sync</li>
            </ul>
          </div>

          <div className="p-6 rounded-[4px] border border-border bg-surface-elevated">
            <div className="text-xs text-muted">TEAM //</div>
            <div className="text-2xl font-bold text-foreground mt-2">\$35</div>
            <div className="text-[11px] text-muted mb-4">Per seat / month</div>
            <ul className="text-xs space-y-2 text-muted">
              <li>&bull; Everything in Pro</li>
              <li>&bull; Shared org quotas</li>
              <li>&bull; Centralized audit logs</li>
            </ul>
          </div>
        </div>
      </section>

      {/* Final Call to Action */}
      <section className="text-center max-w-3xl mx-auto py-8">
        <h2 className="text-3xl font-bold font-mono text-foreground mb-4">
          Ready to see what your agents are doing?
        </h2>
        <p className="text-sm text-muted mb-8 font-sans">
          Install the desktop tray or run one command to launch your local spawnt console.
        </p>
        <div className="flex flex-wrap justify-center gap-4 font-mono text-xs">
          <Link
            href="/console"
            className="h-10 px-6 rounded-[4px] bg-primary text-white font-medium hover:brightness-110 transition-all flex items-center gap-2"
          >
            Launch Console &rarr;
          </Link>
          <Link
            href="/downloads"
            className="h-10 px-6 rounded-[4px] bg-surface border border-border text-foreground hover:border-primary transition-colors flex items-center gap-2"
          >
            Get Desktop App &darr;
          </Link>
        </div>
      </section>
    </div>
  );
}

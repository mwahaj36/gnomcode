"use client";

import { useState } from "react";
import Link from "next/link";

const pricingJsonLd = {
  "@context": "https://schema.org",
  "@type": "Product",
  name: "FleetHUD",
  description:
    "Live mission control, token spend attribution, and per-model quota monitoring for AI coding agents with zero content storage.",
  offers: [
    {
      "@type": "Offer",
      name: "Community",
      price: "0",
      priceCurrency: "USD",
      description: "100% local monitoring on one machine forever.",
    },
    {
      "@type": "Offer",
      name: "Pro",
      price: "19",
      priceCurrency: "USD",
      description: "Cross-device Supabase cloud sync, multi-plan ROI, and custom model pricing.",
    },
    {
      "@type": "Offer",
      name: "Teams",
      price: "49",
      priceCurrency: "USD",
      description: "Organization-wide spend caps, masked project keys, and role-based access.",
    },
  ],
};

export default function PricingPage() {
  const [annual, setAnnual] = useState(false);

  const tiers = [
    {
      name: "Free / Community",
      price: "$0",
      cadence: "forever",
      desc: "Everything you need to monitor and attribute your local AI coding agents on one workstation.",
      badge: "100% LOCAL",
      features: [
        "Unlimited local agent sessions",
        "Full support for Claude Code, Codex, & Antigravity",
        "Real-time token cost & cache savings attribution",
        "Per-model 5h & 7d quota monitoring",
        "Native desktop background tray app (zero terminal)",
        "Zero telemetry & zero content storage",
        "Local SQLite database (~/.fleethud/state.db)",
      ],
      ctaText: "Download Free App",
      ctaHref: "/downloads",
      primary: false,
    },
    {
      name: "Pro",
      price: annual ? "$15" : "$19",
      cadence: annual ? "per month, billed annually" : "per month",
      desc: "For engineers running agent fleets across multiple laptops and workstations.",
      badge: "MOST POPULAR",
      features: [
        "Everything in Free, plus:",
        "Real-time cross-device cloud sync (Supabase)",
        "Multi-Plan ROI calculator (Claude + OpenAI + Google)",
        "Custom model pricing overrides manager",
        "Rate limit Resume Queue & smart continuation",
        "Daily, weekly, and monthly rollup exports",
        "Priority model pricing catalog updates",
      ],
      ctaText: "Start Pro Free Trial",
      ctaHref: "/auth",
      primary: true,
    },
    {
      name: "Teams",
      price: annual ? "$39" : "$49",
      cadence: annual ? "per seat / mo, billed annually" : "per seat / mo",
      desc: "Full visibility and spend governance across your entire engineering organization.",
      badge: "ENTERPRISE",
      features: [
        "Everything in Pro, plus:",
        "Organization-wide team dashboard & member rollups",
        "Privacy-preserving HMAC-SHA256 project masking",
        "Spend caps & team budget guardrails",
        "Slack & Discord webhook notifications",
        "Role-based access control (Admin, Member, Billing)",
        "Dedicated onboarding & priority SLA",
      ],
      ctaText: "Contact for Teams",
      ctaHref: "/auth",
      primary: false,
    },
  ];

  const comparisonRows = [
    { feature: "Supported Agents (Claude, Codex, Antigravity)", free: "Yes", pro: "Yes", team: "Yes" },
    { feature: "Token Spend & Cache Savings Attribution", free: "Yes", pro: "Yes", team: "Yes" },
    { feature: "Per-Model Quota Pool Monitoring (5h & 7d)", free: "Yes", pro: "Yes", team: "Yes" },
    { feature: "Desktop Background Tray App", free: "Yes", pro: "Yes", team: "Yes" },
    { feature: "Zero Content Storage Invariant", free: "Guaranteed", pro: "Guaranteed", team: "Guaranteed" },
    { feature: "Multi-Workstation Cloud Sync (Supabase)", free: "No", pro: "Yes", team: "Yes" },
    { feature: "Multi-Plan Subscription ROI Calculator", free: "Single Plan", pro: "Multi-Plan", team: "Org-Wide" },
    { feature: "Custom Token Pricing Overrides", free: "No", pro: "Yes", team: "Yes" },
    { feature: "Team Budget Caps & Alert Webhooks", free: "No", pro: "No", team: "Yes" },
    { feature: "HMAC-SHA256 Project Key Masking", free: "Local Only", pro: "Yes", team: "Yes" },
  ];

  const faqs = [
    {
      q: "Do you ever see or store my prompts, code, or tool outputs?",
      a: "Never. Non-negotiable product invariant #1: FleetHUD never stores or transmits content. Activity summaries are strictly scrubbed locally before persistence, and project keys are HMAC-hashed before cloud sync.",
    },
    {
      q: "Can I use FleetHUD completely offline?",
      a: "Yes. The Free tier runs 100% locally on your machine with zero network calls. You only connect an account if you want cross-device cloud sync or team rollups.",
    },
    {
      q: "How does the Subscription ROI calculation work?",
      a: "We compute what your exact token usage would have cost at raw provider API rates (including cached input discounts and reasoning token premiums), and compare it against your monthly flat subscription fee ($20/mo, $100/mo, etc.).",
    },
    {
      q: "Which coding agents are currently supported?",
      a: "Claude Code, OpenAI Codex (CLI and Desktop App), Google Antigravity (IDE and autonomous tasks), and Gemini CLI. More agents are added regularly.",
    },
  ];

  return (
    <div className="sp-console-shell max-w-7xl mx-auto px-6 py-16 font-sans">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(pricingJsonLd) }}
      />

      {/* Header */}
      <div className="text-center max-w-3xl mx-auto mb-12">
        <p className="font-mono text-xs uppercase tracking-wider text-accent mb-4">Simple, transparent pricing</p>
        <h1 className="text-4xl sm:text-5xl font-extrabold tracking-tight text-foreground mb-4">
          Invest in Your Agent Productivity
        </h1>
        <p className="text-muted text-base leading-relaxed">
          Start free on your local workstation. Upgrade only when you want cross-device roaming, team spend governance, or custom rate overrides.
        </p>

        {/* Annual / Monthly Toggle */}
        <div className="mt-8 inline-flex items-stretch border border-border font-mono text-xs">
          <button
            onClick={() => setAnnual(false)}
            className={`min-h-11 px-4 transition-colors ${
              !annual
                ? "bg-accent text-bg font-bold"
                : "text-muted hover:bg-card hover:text-foreground"
            }`}
          >
            Monthly Billing
          </button>
          <button
            onClick={() => setAnnual(true)}
            className={`min-h-11 px-4 border-l border-border transition-colors flex items-center gap-2 ${
              annual
                ? "bg-accent text-bg font-bold"
                : "text-muted hover:bg-card hover:text-foreground"
            }`}
          >
            <span>Annual Billing</span>
            <span className={annual ? "text-bg" : "text-green"}>SAVE 20%</span>
          </button>
        </div>
      </div>

      {/* Pricing Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-8 mb-20 font-mono">
        {tiers.map((t) => (
          <div
            key={t.name}
            className={`rounded-xl p-8 flex flex-col justify-between transition-all ${
              t.primary
              ? "bg-card border-2 border-accent shadow-xl shadow-accent/10"
                : "bg-card/50 border border-border hover:border-border/80"
            }`}
          >
            <div>
              <div className="flex justify-between items-center mb-3">
                <span className="text-sm font-bold text-foreground uppercase tracking-wider">
                  {t.name}
                </span>
                <span className={`font-mono text-[10px] uppercase tracking-wider ${t.primary ? "text-accent" : "text-muted"}`}>
                  {t.badge}
                </span>
              </div>

              <div className="mb-4">
                <span className="text-4xl font-extrabold text-foreground">{t.price}</span>
                <span className="text-xs text-muted ml-2">/ {t.cadence}</span>
              </div>

              <p className="text-xs text-muted mb-6 leading-relaxed min-h-[40px]">
                {t.desc}
              </p>

              <div className="border-t border-border/80 pt-6 mb-8">
                <div className="text-[11px] uppercase tracking-wider text-muted font-semibold mb-3">
                  Included Capabilities:
                </div>
                <ul className="space-y-2.5 text-xs text-muted">
                  {t.features.map((f, i) => (
                    <li key={i} className="flex items-start gap-2">
                      <span className="text-green font-bold">&check;</span>
                      <span className={i === 0 && t.primary ? "text-foreground font-semibold" : ""}>
                        {f}
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
            </div>

            <Link
              href={t.ctaHref}
              className={`w-full py-3 rounded-lg text-center text-xs font-bold transition-all ${
                t.primary
                  ? "bg-accent text-bg hover:opacity-90 shadow-md shadow-accent/20"
                  : "bg-bg border border-border text-foreground hover:border-accent"
              }`}
            >
              {t.ctaText} &rarr;
            </Link>
          </div>
        ))}
      </div>

      {/* Feature Comparison Matrix */}
      <div className="mb-20 font-mono">
        <div className="text-center mb-8">
          <h2 className="text-2xl font-bold text-foreground">Detailed Feature Comparison</h2>
          <p className="text-muted text-xs mt-1">Side-by-side feature matrix across editions</p>
        </div>

        <div className="bg-card border border-border rounded-xl overflow-x-auto">
          <table className="w-full text-left text-xs border-collapse">
            <thead>
              <tr className="border-b border-border bg-card">
                <th className="p-4 text-muted font-semibold">Capability</th>
                <th className="p-4 text-muted font-semibold">Free</th>
                <th className="p-4 text-accent font-semibold">Pro</th>
                <th className="p-4 text-foreground font-semibold">Teams</th>
              </tr>
            </thead>
            <tbody>
              {comparisonRows.map((r, i) => (
                <tr key={i} className="border-b border-border/60 hover:bg-bg/40 transition-colors">
                  <td className="p-4 font-semibold text-foreground">{r.feature}</td>
                  <td className="p-4 text-muted">{r.free}</td>
                  <td className="p-4 text-accent font-semibold">{r.pro}</td>
                  <td className="p-4 text-foreground">{r.team}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* FAQ Section */}
      <div className="max-w-3xl mx-auto font-mono text-xs">
        <h2 className="text-2xl font-bold text-center text-foreground mb-8">
          Frequently Asked Questions
        </h2>
        <div className="space-y-4">
          {faqs.map((f, i) => (
            <div key={i} className="bg-card border border-border rounded-lg p-5">
              <div className="text-foreground font-semibold text-sm mb-2">{f.q}</div>
              <div className="text-muted leading-relaxed">{f.a}</div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

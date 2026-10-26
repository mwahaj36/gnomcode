"use client";

import { useState } from "react";
import Link from "next/link";
import { BRAND } from "@/lib/brand";

const pricingJsonLd = {
  "@context": "https://schema.org",
  "@type": "Product",
  name: "spawnt",
  description:
    "See what your coding agents are doing. Live status, cost and limits for your agents. Local-first. Nothing leaves your machine.",
  offers: [
    {
      "@type": "Offer",
      name: "Free",
      price: "0",
      priceCurrency: "USD",
      description: "100% local monitoring on one machine forever.",
    },
    {
      "@type": "Offer",
      name: "Pro",
      price: "15",
      priceCurrency: "USD",
      description: "Multi-machine sync, real-time budget alerts, and priority pricing catalogs.",
    },
    {
      "@type": "Offer",
      name: "Team",
      price: "35",
      priceCurrency: "USD",
      description: "Organization-wide spend caps, masked project keys, and role-based access.",
    },
  ],
};

export default function PricingPage() {
  const [annual, setAnnual] = useState(true);

  const tiers = [
    {
      name: "FREE",
      price: "$0",
      cadence: "forever",
      desc: "Everything you need to monitor and attribute your local AI coding agents on one workstation.",
      badge: "[LOCAL-FIRST]",
      features: [
        "Unlimited local agent sessions",
        "Full support for Claude Code, Codex, Antigravity, Gemini, Cursor",
        "Real-time token cost & cache savings attribution",
        "Per-model 5h & 7d quota pool monitoring",
        "Native desktop background tray app",
        "Zero telemetry & zero content storage",
        "Local SQLite database (~/.spawnt/state.db)",
      ],
      ctaText: "Download Free App",
      ctaHref: "/downloads",
      primary: false,
    },
    {
      name: "PRO",
      price: annual ? "$12" : "$15",
      cadence: annual ? "per month, billed annually" : "per month",
      desc: "For engineers running agent fleets across multiple laptops and workstations.",
      badge: "[MOST POPULAR]",
      features: [
        "Everything in Free, plus:",
        "Real-time cross-device cloud sync",
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
      name: "TEAM",
      price: annual ? "$28" : "$35",
      cadence: annual ? "per seat / mo, billed annually" : "per seat / mo",
      desc: "Full visibility and spend governance across your entire engineering organization.",
      badge: "[ENTERPRISE]",
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
    { feature: "Supported Agents (Claude, Codex, Antigravity, Gemini, Cursor)", free: "Yes", pro: "Yes", team: "Yes" },
    { feature: "Token Spend & Cache Savings Attribution", free: "Yes", pro: "Yes", team: "Yes" },
    { feature: "Per-Model Quota Pool Monitoring (5h & 7d)", free: "Yes", pro: "Yes", team: "Yes" },
    { feature: "Desktop Background Tray App", free: "Yes", pro: "Yes", team: "Yes" },
    { feature: "Zero Content Storage Invariant", free: "Guaranteed", pro: "Guaranteed", team: "Guaranteed" },
    { feature: "Multi-Workstation Cloud Sync", free: "No", pro: "Yes", team: "Yes" },
    { feature: "Multi-Plan Subscription ROI Calculator", free: "Single Plan", pro: "Multi-Plan", team: "Org-Wide" },
    { feature: "Custom Token Pricing Overrides", free: "No", pro: "Yes", team: "Yes" },
    { feature: "Team Budget Caps & Alert Webhooks", free: "No", pro: "No", team: "Yes" },
    { feature: "HMAC-SHA256 Project Key Masking", free: "Local Only", pro: "Yes", team: "Yes" },
  ];

  const faqs = [
    {
      q: "Do you ever see or store my prompts, code, or tool outputs?",
      a: "Never. Non-negotiable product invariant #1: spawnt never stores or transmits content. Activity summaries are strictly scrubbed locally before persistence, and project keys are HMAC-hashed before cloud sync.",
    },
    {
      q: "Can I use spawnt completely offline?",
      a: "Yes. The Free tier runs 100% locally on your machine with zero network calls. You only connect an account if you want cross-device cloud sync or team rollups.",
    },
    {
      q: "How does the Subscription ROI calculation work?",
      a: "We compute what your exact token usage would have cost at raw provider API rates (including cached input discounts and reasoning token premiums), and compare it against your monthly flat subscription fee ($20/mo, $100/mo, etc.).",
    },
    {
      q: "Which coding agents are currently supported?",
      a: "Claude Code, OpenAI Codex (CLI and Desktop App), Google Antigravity (IDE and autonomous tasks), Gemini CLI, Cursor, and Roo Code/Cline. More agents are added regularly.",
    },
  ];

  return (
    <div className="max-w-7xl mx-auto px-6 py-16 font-mono text-xs">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(pricingJsonLd) }}
      />

      {/* Header */}
      <div className="text-center max-w-3xl mx-auto mb-12">
        <div className="inline-flex items-center gap-2 px-3 py-1 rounded-[4px] bg-surface border border-border text-muted uppercase tracking-wider text-[11px] mb-4">
          <span>PRICING & PLANS //</span>
        </div>
        <h1 className="text-3xl sm:text-5xl font-bold tracking-tight text-foreground mb-4">
          Invest in your agent productivity
        </h1>
        <p className="text-muted text-sm font-sans leading-relaxed">
          Start free on your local workstation. Upgrade only when you want cross-device roaming, team spend governance, or custom rate overrides.
        </p>

        {/* Annual / Monthly Toggle */}
        <div className="mt-8 inline-flex items-center gap-3 p-1 rounded-[4px] border border-border bg-surface">
          <button
            onClick={() => setAnnual(false)}
            className={`px-3 py-1.5 rounded-[4px] text-xs font-semibold transition-colors ${
              !annual ? "bg-surface-elevated text-foreground border border-border" : "text-muted hover:text-foreground"
            }`}
          >
            MONTHLY
          </button>
          <button
            onClick={() => setAnnual(true)}
            className={`px-3 py-1.5 rounded-[4px] text-xs font-semibold transition-colors flex items-center gap-2 ${
              annual ? "bg-primary text-white" : "text-muted hover:text-foreground"
            }`}
          >
            <span>ANNUAL</span>
            <span className="text-[10px] opacity-90">[SAVE 20%]</span>
          </button>
        </div>
      </div>

      {/* Tiers Grid */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-20">
        {tiers.map((tier) => (
          <div
            key={tier.name}
            className={`rounded-[6px] p-6 border flex flex-col justify-between ${
              tier.primary
                ? "bg-surface border-primary"
                : "bg-surface border-border"
            }`}
          >
            <div>
              <div className="flex items-center justify-between mb-4">
                <span className="font-bold text-foreground tracking-wider">{tier.name} //</span>
                <span
                  className={`text-[10px] px-2 py-0.5 rounded-[4px] border ${
                    tier.primary
                      ? "border-primary/40 text-primary bg-primary/10 font-bold"
                      : "border-border text-muted bg-surface-elevated"
                  }`}
                >
                  {tier.badge}
                </span>
              </div>
              <div className="mb-4">
                <span className="text-3xl font-bold text-foreground">{tier.price}</span>
                <span className="text-muted ml-2 text-[11px]">/{tier.cadence}</span>
              </div>
              <p className="text-muted text-xs font-sans mb-6 leading-relaxed">{tier.desc}</p>
              <ul className="space-y-2.5 text-xs border-t border-border pt-6 mb-6">
                {tier.features.map((feat) => (
                  <li key={feat} className="flex items-start gap-2 text-foreground/90">
                    <span className="text-primary font-bold">&bull;</span>
                    <span className="font-sans text-xs">{feat}</span>
                  </li>
                ))}
              </ul>
            </div>
            <Link
              href={tier.ctaHref}
              className={`w-full py-2.5 rounded-[4px] text-center font-bold text-xs transition-colors block ${
                tier.primary
                  ? "bg-primary text-white hover:brightness-110"
                  : "bg-surface-elevated border border-border text-foreground hover:border-primary"
              }`}
            >
              {tier.ctaText} &rarr;
            </Link>
          </div>
        ))}
      </div>

      {/* Feature Comparison Table */}
      <div className="mb-20 border border-border rounded-[6px] bg-surface overflow-hidden">
        <div className="p-4 bg-surface-elevated border-b border-border">
          <div className="text-xs text-muted uppercase tracking-wider">FEATURE MATRIX //</div>
          <div className="text-lg font-bold text-foreground mt-1">Detailed Tier Comparison</div>
        </div>
        <div className="overflow-x-auto p-4">
          <table className="w-full text-left">
            <thead>
              <tr className="border-b border-border text-muted uppercase text-[11px]">
                <th className="pb-3 font-semibold">CAPABILITY //</th>
                <th className="pb-3 font-semibold text-center w-28">FREE</th>
                <th className="pb-3 font-semibold text-center w-28 text-primary">PRO</th>
                <th className="pb-3 font-semibold text-center w-28">TEAM</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border/60">
              {comparisonRows.map((row) => (
                <tr key={row.feature} className="hover:bg-surface-elevated/40">
                  <td className="py-3 text-foreground font-sans text-xs">{row.feature}</td>
                  <td className="py-3 text-center text-muted">{row.free}</td>
                  <td className="py-3 text-center text-primary font-bold">{row.pro}</td>
                  <td className="py-3 text-center text-foreground font-semibold">{row.team}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* FAQs */}
      <div className="max-w-3xl mx-auto space-y-6">
        <div className="text-center mb-8">
          <div className="text-muted uppercase tracking-wider text-[11px] mb-1">FREQUENTLY ASKED //</div>
          <h2 className="text-2xl font-bold text-foreground">Common questions</h2>
        </div>
        <div className="space-y-4">
          {faqs.map((faq) => (
            <div key={faq.q} className="border border-border rounded-[6px] bg-surface p-5">
              <h3 className="font-bold text-foreground text-sm mb-2">{faq.q}</h3>
              <p className="text-muted font-sans text-xs leading-relaxed">{faq.a}</p>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

import type { Metadata } from "next";
import Link from "next/link";

export const metadata: Metadata = {
  title: "Pricing — spawnt",
  description: "Start with local agent monitoring for free. Pro is a one-time license; Teams is planned.",
};

const tiers = [
  {
    name: "Free",
    note: "LOCAL · FOREVER",
    price: "$0",
    cadence: "free forever",
    description: "Live agent state, usage, and quota on one machine.",
    features: [
      "Local CLI, daemon, and dashboard",
      "Live sessions and approval state",
      "Token usage and quota tracking",
      "Desktop tray and notifications",
      "No telemetry or content storage",
    ],
    action: "Download free",
    href: "/downloads",
  },
  {
    name: "Pro",
    note: "PERSONAL LICENSE",
    price: "$19",
    cadence: "one-time",
    description: "More control for developers running agents across machines.",
    features: [
      "Everything in Free",
      "Budget and loop guardrails",
      "Custom alert rules",
      "Custom model pricing overrides",
      "Resume Queue and Context Relay",
      "Advanced analytics and exports",
      "Personal multi-machine sync",
    ],
    action: "Sign in",
    href: "/auth",
  },
  {
    name: "Teams",
    note: "PLANNED",
    price: "$12",
    cadence: "per seat / month",
    description: "Shared visibility and controls for small engineering teams.",
    features: [
      "Everything in Pro",
      "Team dashboard and member rollups",
      "Shared spend caps",
      "Role-based access",
      "Team notifications",
    ],
    action: "Teams is planned",
    href: null,
  },
];

const questions = [
  {
    question: "Does spawnt store prompts, code, or tool output?",
    answer: "No. spawnt stores redacted activity metadata and usage details. Prompt and tool-output content is never stored or sent.",
  },
  {
    question: "Can I use spawnt without an account?",
    answer: "Yes. The Free tier runs locally. An account is only needed for personal multi-machine sync.",
  },
  {
    question: "Is Pro a subscription?",
    answer: "No. Pro is a $19 one-time license. Teams is planned at $12 per seat per month.",
  },
];

export default function PricingPage() {
  return (
    <div className="sp-console-shell sp-pricing-page mx-auto max-w-7xl px-6 py-16 font-sans">
      <header className="sp-pricing-intro">
        <h1>Start local. Add control when you need it.</h1>
        <p>One workstation is free forever. Pro is a one-time personal license; team features are planned.</p>
      </header>

      <section className="sp-pricing-grid" aria-label="Plans">
        {tiers.map((tier, index) => (
          <article className={index === 1 ? "sp-pricing-tier sp-pricing-featured" : "sp-pricing-tier"} key={tier.name}>
            <div className="sp-pricing-tier-head">
              <h2>{tier.name}</h2>
              <span>{tier.note}</span>
            </div>
            <p className="sp-pricing-description">{tier.description}</p>
            <p className="sp-pricing-price"><strong>{tier.price}</strong><span>{tier.cadence}</span></p>
            <ul>
              {tier.features.map((feature) => <li key={feature}>{feature}</li>)}
            </ul>
            {tier.href ? (
              <Link href={tier.href} className={index === 1 ? "sp-pricing-action sp-pricing-primary" : "sp-pricing-action"}>
                {tier.action}<span aria-hidden="true">→</span>
              </Link>
            ) : (
              <span className="sp-pricing-action sp-pricing-unavailable">{tier.action}</span>
            )}
          </article>
        ))}
      </section>

      <section className="sp-pricing-included" aria-labelledby="included-heading">
        <header>
          <h2 id="included-heading">Included in every plan</h2>
          <p>The same local-first foundation, whether you use Free or Pro.</p>
        </header>
        <dl>
          <div><dt>Agent activity</dt><dd>See running, waiting, idle, and error states.</dd></div>
          <div><dt>Usage and quota</dt><dd>Every measurement carries its source and trust level.</dd></div>
          <div><dt>Privacy</dt><dd>Prompts, code, and tool output stay out of spawnt storage.</dd></div>
        </dl>
      </section>

      <section className="sp-pricing-faq" aria-labelledby="pricing-faq-heading">
        <h2 id="pricing-faq-heading">Pricing questions</h2>
        {questions.map((item) => (
          <details key={item.question}>
            <summary>{item.question}</summary>
            <p>{item.answer}</p>
          </details>
        ))}
      </section>
    </div>
  );
}

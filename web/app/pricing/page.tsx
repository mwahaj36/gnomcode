import type { Metadata } from "next";
import Link from "next/link";

export const metadata: Metadata = {
  title: "Pricing — spawnt",
  description: "All local features are free. Remote and Teams are planned paid features.",
};

const tiers = [
  {
    name: "Local", note: "FREE · NO ACCOUNT", price: "$0", cadence: "free forever",
    description: "Monitor and manage agents on your own machine.",
    features: ["CLI, daemon, and local dashboard", "Live sessions, usage, and quota", "Guardrails and alert rules", "Resume Queue and Context Relay", "Analytics and exports"],
    action: "Download free", href: "/downloads",
  },
  {
    name: "Remote", note: "PLANNED", price: "Planned", cadence: "paid feature",
    description: "Access your own sessions from your phone.",
    features: ["Encrypted remote access is planned", "Pair and revoke devices", "View sessions and respond to requests"],
    action: "Remote is planned", href: null,
  },
  {
    name: "Teams", note: "PLANNED", price: "Planned", cadence: "paid feature",
    description: "Shared visibility and controls for teams.",
    features: ["Team dashboard and member rollups", "Shared budgets and policies", "Role-based access", "Team notifications"],
    action: "Teams is planned", href: null,
  },
];

const questions = [
  {
    question: "Does spawnt store prompts, code, or tool output?",
    answer: "Current monitoring stores redacted metadata. Local chat history is planned for session continuity and model switching.",
  },
  {
    question: "Can I use spawnt without an account?",
    answer: "Yes. All local features run without an account. Existing cloud sync is optional and is not end-to-end encrypted.",
  },
  {
    question: "What will be paid?",
    answer: "Remote and Teams are planned paid features. Pricing has not been announced.",
  },
];

export default function PricingPage() {
  return (
    <div className="sp-console-shell sp-pricing-page mx-auto max-w-7xl px-6 py-16 font-sans">
      <header className="sp-pricing-intro">
        <h1>All local features are free.</h1>
        <p>Use the CLI and local dashboard without an account. Remote and Teams are planned paid features.</p>
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
          <p>The same local-first foundation across every plan.</p>
        </header>
        <dl>
          <div><dt>Agent activity</dt><dd>See running, waiting, idle, and error states.</dd></div>
          <div><dt>Usage and quota</dt><dd>Every measurement carries its source and trust level.</dd></div>
          <div><dt>Privacy</dt><dd>Monitoring metadata is redacted. Local chat history is planned.</dd></div>
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

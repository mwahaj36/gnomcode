import { pageMetadata } from "../../lib/seo";
import Link from "next/link";

export const metadata = pageMetadata("/pricing", "Gnomcode Pricing — Free Local, Paid Remote & Teams", "All local Gnomcode features are free. Remote access and Teams are paid categories. Your agent provider bills inference separately.");

const tiers = [
  {
    name: "Local", note: "FREE · NO ACCOUNT", price: "$0", cadence: "free forever",
    description: "Monitor and manage agents on your own machine.",
    features: ["CLI, daemon, and local dashboard", "Live sessions, usage, and quota", "Guardrails and alert rules", "Resume Queue and Context Relay", "Analytics and exports"],
    action: "Download free", href: "/downloads",
  },
  {
    name: "Remote", note: "PAID ACCESS", price: "Remote", cadence: "terms in documentation",
    description: "Access your own sessions from your phone.",
    features: ["End-to-end encrypted access", "Pair and revoke devices", "View sessions and respond to requests"],
    action: "Explore Remote", href: "/docs/remote-access",
  },
  {
    name: "Teams", note: "PAID CATEGORY", price: "Teams", cadence: "product direction in docs",
    description: "Shared visibility and controls for teams.",
    features: ["Shared visibility is the Teams direction", "Scope and service terms in documentation"],
    action: "Read the FAQ", href: "/docs/faq",
  },
];

const questions = [
  {
    question: "Does gnomcode store prompts, code, or tool output?",
    answer: "Gnomcode stores redacted local history for continuity. Account metadata sync excludes chat events; paired Remote carries conversations through E2EE.",
  },
  {
    question: "Can I use gnomcode without an account?",
    answer: "Yes. All local features run without an account. Existing cloud sync is optional and is not end-to-end encrypted.",
  },
  {
    question: "Where can I read service terms?",
    answer: "Read the documentation for Remote and Teams service details. Your agent provider bills inference separately.",
  },
];

export default function PricingPage() {
  return (
    <div className="sp-console-shell sp-pricing-page mx-auto max-w-7xl px-6 py-16 font-sans">
      <header className="sp-pricing-intro">
        <h1>All local features are free.</h1>
        <p>Use the CLI and local dashboard without an account. Remote access and Teams are paid categories. Provider inference is billed separately.</p>
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
          <div><dt>Privacy</dt><dd>Redacted local history; optional metadata sync; encrypted Remote.</dd></div>
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

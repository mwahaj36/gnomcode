import LandingClient from "./LandingClient";
import { pageMetadata } from "../lib/seo";
import { landingFaq } from "../lib/landing-faq";
import branding from "../branding.json";

export const metadata = pageMetadata("", "Spawnt — The Runtime for AI Coding Agents", "Launch AI coding agents, switch models mid-task and control supported sessions from your phone. Local-first orchestration across providers.");

const schema = { "@context": "https://schema.org", "@graph": [
  { "@type": "SoftwareApplication", name: "Spawnt", applicationCategory: "DeveloperApplication", operatingSystem: "Windows, macOS, Linux", url: branding.baseUrl, author: { "@type": "Person", name: "Wahaj" }, description: "Local-first orchestration for AI coding agents: launch, switch models and control supported sessions remotely.", offers: { "@type": "Offer", price: "0", priceCurrency: "USD", description: "All local features are free" } },
  { "@type": "FAQPage", mainEntity: landingFaq.map(item => ({ "@type": "Question", name: item.question, acceptedAnswer: { "@type": "Answer", text: item.answer } })) },
] };

export default function LandingPage() {
  return <><script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify(schema).replace(/</g, "\\u003c") }} /><LandingClient /></>;
}

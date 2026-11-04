import LandingClient from "./LandingClient";
import { pageMetadata } from "../lib/seo";
import branding from "../branding.json";

export const metadata = pageMetadata("", "Gnomcode — Switch AI Coding Agents and Keep Working", "Launch Claude Code and Codex, switch agents with a concise handoff, and manage coding sessions locally. Free local features; Windows development build available.");

const schema = { "@context": "https://schema.org", "@graph": [
  { "@type": "SoftwareApplication", name: "Gnomcode", applicationCategory: "DeveloperApplication", operatingSystem: "Windows, macOS, Linux", url: branding.baseUrl, author: { "@type": "Person", name: "Wahaj" }, description: "Local-first orchestration for AI coding agents: launch, switch models and control supported sessions remotely.", offers: { "@type": "Offer", price: "0", priceCurrency: "USD", description: "All local features are free" } },
  ] };

export default function LandingPage() {
  return <><script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify(schema).replace(/</g, "\\u003c") }} /><LandingClient /></>;
}

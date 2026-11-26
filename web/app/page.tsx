import LandingClient from "./LandingClient";
import { pageMetadata, buildSoftwareAppJsonLd, buildFaqJsonLd, buildWebSiteJsonLd } from "../lib/seo";
import { landingFaq } from "../lib/landing-faq";

export const metadata = pageMetadata(
  "",
  "Gnomcode — Switch AI Coding Agents and Keep Working",
  "Open source runtime for Claude Code, Codex and Antigravity. Switch agents with a concise handoff. Windows, macOS and Linux."
);

const softwareApp = buildSoftwareAppJsonLd();
const faqSchema = buildFaqJsonLd(landingFaq);
const webSite = buildWebSiteJsonLd();

export default function LandingPage() {
  return (
    <>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(softwareApp).replace(/</g, "\\u003c") }}
      />
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(faqSchema).replace(/</g, "\\u003c") }}
      />
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(webSite).replace(/</g, "\\u003c") }}
      />
      <LandingClient />
    </>
  );
}

import LandingClient from "./LandingClient";
import { pageMetadata, buildSoftwareAppJsonLd, buildFaqJsonLd } from "../lib/seo";
import { landingFaq } from "../lib/landing-faq";

export const metadata = pageMetadata(
  "",
  "Gnomcode — Switch AI Coding Agents and Keep Working",
  "Launch Claude Code, Codex and Antigravity, switch with a concise handoff and manage sessions locally. Local sessions and encrypted Remote; Windows build available."
);

const softwareApp = buildSoftwareAppJsonLd();
const faqSchema = buildFaqJsonLd(landingFaq);

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
      <LandingClient />
    </>
  );
}

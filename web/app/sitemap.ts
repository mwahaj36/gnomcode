import { MetadataRoute } from "next";
import branding from "../branding.json";

export default function sitemap(): MetadataRoute.Sitemap {
  return ["", "/downloads", "/docs", "/pricing", "/privacy", "/terms", "/docs/automate-multiple-agents", "/docs/claude-code-vs-codex", "/docs/faq", "/docs/getting-started", "/docs/model-pricing", "/docs/model-switching", "/docs/privacy", "/docs/prompt-from-phone", "/docs/quota-and-cost", "/docs/quota-limits-explained", "/docs/remote-access", "/docs/supported-agents"].map(path => ({
    url: `${branding.baseUrl}${path}`,
  }));
}

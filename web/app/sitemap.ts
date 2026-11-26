import { MetadataRoute } from "next";
import branding from "../branding.json";

const pages: Array<[string, number]> = [
  ["", 1], ["/downloads", 0.9], ["/docs", 0.8], ["/docs/getting-started", 0.8], ["/docs/faq", 0.7],
  ["/docs/supported-agents", 0.7], ["/docs/model-switching", 0.7], ["/docs/claude-code-vs-codex", 0.7],
  ["/docs/prompt-from-phone", 0.6], ["/docs/remote-access", 0.6], ["/docs/quota-and-cost", 0.6],
  ["/docs/quota-limits-explained", 0.6], ["/docs/model-pricing", 0.6], ["/docs/automate-multiple-agents", 0.6],
  ["/docs/privacy", 0.4], ["/privacy", 0.3], ["/terms", 0.3],
];

export default function sitemap(): MetadataRoute.Sitemap {
  const lastModified = new Date();
  return pages.map(([path, priority]) => ({
    url: `${branding.baseUrl}${path}`,
    lastModified,
    changeFrequency: path === "/downloads" || path === "" ? "weekly" : "monthly",
    priority,
  }));
}

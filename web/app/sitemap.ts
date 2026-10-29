import { MetadataRoute } from "next";
import branding from "../branding.json";

export default function sitemap(): MetadataRoute.Sitemap {
  // Content review date, not the time a crawler requests the sitemap.
  const lastModified = new Date("2026-10-29T00:00:00Z");
  return ["", "/downloads", "/docs", "/pricing", "/remote", "/privacy", "/terms", "/docs/automate-multiple-agents", "/docs/claude-code-vs-codex", "/docs/faq", "/docs/getting-started", "/docs/model-pricing", "/docs/model-switching", "/docs/privacy", "/docs/prompt-from-phone", "/docs/quota-and-cost", "/docs/quota-limits-explained", "/docs/remote-access", "/docs/supported-agents"].map(path => ({
    url: `${branding.baseUrl}${path}`, lastModified,
    changeFrequency: path === "/privacy" || path === "/terms" ? "yearly" : path === "/pricing" || path === "/remote" ? "monthly" : "weekly",
    priority: path === "" ? 1 : path === "/privacy" || path === "/terms" ? .4 : .8,
  }));
}

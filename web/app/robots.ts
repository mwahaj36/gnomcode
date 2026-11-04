import { MetadataRoute } from "next";
import branding from "../branding.json";
export default function robots(): MetadataRoute.Robots {
  const disallow = ["/api/", "/_next/", "/app", "/app/", "/console", "/remote", "/auth", "/settings"];
  return { rules: [
    { userAgent: "*", allow: "/", disallow },
    ...["GPTBot", "PerplexityBot", "ClaudeBot"].map(userAgent => ({ userAgent, allow: "/", disallow })),
  ], sitemap: `${branding.baseUrl}/sitemap.xml` };
}

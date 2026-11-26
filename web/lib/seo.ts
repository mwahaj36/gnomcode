import type { Metadata } from "next";
import branding from "../branding.json";
import packageInfo from "../package.json";

export function pageMetadata(path: string, title: string, description: string, index = true): Metadata {
  const url = `${branding.baseUrl}${path}`;
  const images = [{ url: `${branding.baseUrl}/og-image.png`, width: 1200, height: 630 }];
  return {
    title: { absolute: title }, description,
    alternates: { canonical: url },
    openGraph: { title, description, url, siteName: "Gnomcode", type: "website", images },
    twitter: { card: "summary_large_image", title, description, images: images.map(image => image.url) },
    robots: { index, follow: index },
  };
}

export function appMetadata(path: string, title: string, description: string): Metadata {
  return {
    title: { absolute: `${title} | Gnomcode` },
    description,
    manifest: "/manifest.webmanifest",
    robots: {
      index: false,
      follow: false,
      noarchive: true,
      nosnippet: true,
      googleBot: {
        index: false,
        follow: false,
      },
    },
    appleWebApp: {
      capable: true,
      statusBarStyle: "black-translucent",
      title: "Gnomcode",
    },
  };
}

export function buildSoftwareAppJsonLd() {
  return {
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    "name": "Gnomcode",
    "operatingSystem": "Windows 10, Windows 11, macOS, Linux",
    "applicationCategory": "DeveloperApplication",
    "applicationSubCategory": "AI coding agent manager",
    "offers": { "@type": "Offer", "price": "0", "priceCurrency": "USD" },
    "license": "https://www.apache.org/licenses/LICENSE-2.0",
    "isAccessibleForFree": true,
    "downloadUrl": `${branding.baseUrl}/downloads`,
    "installUrl": `${branding.baseUrl}/docs/getting-started`,
    "softwareHelp": `${branding.baseUrl}/docs`,
    "image": `${branding.baseUrl}/og-image.png`,
    "author": { "@type": "Person", "name": "Wahaj", "url": "https://github.com/mwahaj36" },
    "sameAs": ["https://github.com/mwahaj36/gnomcode", "https://www.npmjs.com/package/gnomcode"],
    "description": "Local-first runtime, model handoff orchestrator, and encrypted remote control for AI coding agents including Claude Code, OpenAI Codex and Antigravity.",
    "url": branding.baseUrl,
    "softwareVersion": packageInfo.version,
    "featureList": [
      "Switch models mid-conversation across Claude Code, Codex and Antigravity",
      "Control agents from your phone over an end-to-end encrypted channel",
      "Local-first architecture with zero telemetry and fail-open guarantees",
      "Unified token, cost and quota usage tracking with declared trust levels"
    ]
  };
}

export function buildWebSiteJsonLd() {
  return {
    "@context": "https://schema.org",
    "@type": "WebSite",
    "name": "Gnomcode",
    "url": branding.baseUrl,
    "description": "Free, open source local runtime for Claude Code, OpenAI Codex and Antigravity: switch agents with a handoff, track usage and quota, and control sessions from your phone.",
    "publisher": { "@type": "Organization", "name": "Gnomcode", "url": branding.baseUrl, "logo": `${branding.baseUrl}/icons/gnomcode-192.png`, "sameAs": ["https://github.com/mwahaj36/gnomcode"] }
  };
}

export function buildFaqJsonLd(items: Array<{ question: string; answer: string }>) {
  return {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    "mainEntity": items.map((item) => ({
      "@type": "Question",
      "name": item.question,
      "acceptedAnswer": {
        "@type": "Answer",
        "text": item.answer
      }
    }))
  };
}

export function buildHowToJsonLd(name: string, description: string, steps: Array<{ name: string; text: string }>) {
  return {
    "@context": "https://schema.org",
    "@type": "HowTo",
    "name": name,
    "description": description,
    "step": steps.map((s, idx) => ({
      "@type": "HowToStep",
      "position": idx + 1,
      "name": s.name,
      "text": s.text
    }))
  };
}


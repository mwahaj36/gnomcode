import type { Metadata } from "next";
import branding from "../branding.json";

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
    "operatingSystem": "Windows, macOS, Linux, iOS (PWA), Android (PWA)",
    "applicationCategory": "DeveloperApplication",
    "description": "Local-first runtime, model handoff orchestrator, and encrypted remote control for AI coding agents including Claude Code, OpenAI Codex, and Gemini CLI.",
    "url": branding.baseUrl,
    "softwareVersion": "0.9.0",
    "offers": [
      {
        "@type": "Offer",
        "price": "0",
        "priceCurrency": "USD",
        "description": "Local runtime, CLI observer, and usage tracking are free forever."
      },
      {
        "@type": "Offer",
        "price": "20",
        "priceCurrency": "USD",
        "description": "Pro plan with encrypted mobile remote companion."
      }
    ],
    "featureList": [
      "Switch models mid-conversation across Claude Code, Codex and Gemini CLI",
      "Control agents from your phone over an end-to-end encrypted channel",
      "Local-first architecture with zero telemetry and fail-open guarantees",
      "Unified token, cost and quota usage tracking with declared trust levels"
    ]
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


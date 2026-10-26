import type { Metadata } from "next";
import "./globals.css";
import Navbar from "../components/Navbar";
import FooterWrapper from "../components/FooterWrapper";

export const metadata: Metadata = {
  metadataBase: new URL("https://fleethud.vercel.app"),
  title: {
    default: "FleetHUD — Mission Control for AI Coding Agents",
    template: "%s | FleetHUD",
  },
  description:
    "Live mission control, token cost attribution, per-model quota pools, and flight recorder for Claude Code, OpenAI Codex, and Google Antigravity across Windows, macOS, and Linux.",
  keywords: [
    "AI coding agent monitoring",
    "Claude Code token tracker",
    "OpenAI Codex rate limit monitor",
    "Google Antigravity mission control",
    "LLM spend attribution",
    "Prompt cache savings calculator",
    "AI developer tooling",
    "Local agent flight recorder",
  ],
  authors: [{ name: "FleetHUD Team" }],
  creator: "FleetHUD",
  publisher: "FleetHUD",
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      "max-video-preview": -1,
      "max-image-preview": "large",
      "max-snippet": -1,
    },
  },
  openGraph: {
    type: "website",
    locale: "en_US",
    url: "https://fleethud.vercel.app",
    title: "FleetHUD — Live Mission Control for AI Coding Agents",
    description:
      "Monitor token usage, per-model quota pools, and live session states for Claude Code, Codex, and Antigravity with zero content storage.",
    siteName: "FleetHUD",
  },
  twitter: {
    card: "summary_large_image",
    title: "FleetHUD — Live Mission Control for AI Coding Agents",
    description:
      "Monitor token usage, per-model quota pools, and live session states for Claude Code, Codex, and Antigravity with zero content storage.",
    creator: "@fleethud",
  },
  alternates: {
    canonical: "https://fleethud.vercel.app",
  },
  icons: {
    icon: [
      { url: "/favicon.ico", sizes: "any" },
      { url: "/icon.svg", type: "image/svg+xml" },
    ],
    apple: "/fleethud.svg",
  },
};

const jsonLd = {
  "@context": "https://schema.org",
  "@graph": [
    {
      "@type": "SoftwareApplication",
      name: "FleetHUD",
      operatingSystem: "Windows, macOS, Linux",
      applicationCategory: "DeveloperApplication",
      offers: {
        "@type": "Offer",
        price: "0",
        priceCurrency: "USD",
      },
      description:
        "Live mission control, token spend attribution, and per-model quota monitoring for AI coding agents with zero content storage.",
    },
    {
      "@type": "Organization",
      name: "FleetHUD",
      url: "https://fleethud.vercel.app",
    },
    {
      "@type": "FAQPage",
      mainEntity: [
        {
          "@type": "Question",
          name: "Does FleetHUD store my source code or prompt contents?",
          acceptedAnswer: {
            "@type": "Answer",
            text: "No. FleetHUD enforces a non-negotiable invariant: zero content storage. Prompts, code, and tool outputs never touch disk or network.",
          },
        },
        {
          "@type": "Question",
          name: "Which AI coding agents does FleetHUD support?",
          acceptedAnswer: {
            "@type": "Answer",
            text: "FleetHUD supports Claude Code, OpenAI Codex (CLI & Desktop App), Google Antigravity (IDE & Autonomous Agent), and Google Gemini CLI.",
          },
        },
      ],
    },
  ],
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <head>
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
        />
      </head>
      <body className="bg-bg text-[#c9d1d9] antialiased min-h-screen flex flex-col font-sans">
        <Navbar />
        <main className="flex-1">{children}</main>
        <FooterWrapper />
      </body>
    </html>
  );
}

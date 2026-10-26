import type { Metadata } from "next";
import "./globals.css";
import Navbar from "../components/Navbar";
import FooterWrapper from "../components/FooterWrapper";
import { BRAND } from "@/lib/brand";

export const metadata: Metadata = {
  metadataBase: new URL("https://spawnt.xyz"),
  title: {
    default: "spawnt — See what your coding agents are doing",
    template: "%s | spawnt",
  },
  description:
    "Live status, cost and limits for your agents. Local-first. Nothing leaves your machine. Supports Claude Code, Codex, Antigravity, Gemini, and Cursor.",
  keywords: [
    "spawnt",
    "AI coding agent monitoring",
    "Claude Code token tracker",
    "OpenAI Codex rate limit monitor",
    "Google Antigravity live status",
    "LLM spend attribution",
    "Prompt cache savings calculator",
    "Local agent flight recorder",
  ],
  authors: [{ name: "spawnt" }],
  creator: "spawnt",
  publisher: "spawnt",
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
    url: "https://spawnt.xyz",
    title: "spawnt — See what your coding agents are doing",
    description:
      "Live status, cost and limits for your agents. Local-first. Nothing leaves your machine.",
    siteName: "spawnt",
  },
  twitter: {
    card: "summary_large_image",
    title: "spawnt — See what your coding agents are doing",
    description:
      "Live status, cost and limits for your agents. Local-first. Nothing leaves your machine.",
    creator: "@spawnt_xyz",
  },
  alternates: {
    canonical: "https://spawnt.xyz",
  },
  icons: {
    icon: [
      { url: "/favicon.ico", sizes: "any" },
      { url: "/icon.svg", type: "image/svg+xml" },
    ],
    apple: "/icon.svg",
  },
};

const jsonLd = {
  "@context": "https://schema.org",
  "@graph": [
    {
      "@type": "SoftwareApplication",
      name: "spawnt",
      operatingSystem: "Windows, macOS, Linux",
      applicationCategory: "DeveloperApplication",
      offers: {
        "@type": "Offer",
        price: "0",
        priceCurrency: "USD",
      },
      description:
        "See what your coding agents are doing. Live status, cost and limits for your agents. Local-first. Nothing leaves your machine.",
    },
    {
      "@type": "Organization",
      name: "spawnt",
      url: "https://spawnt.xyz",
    },
    {
      "@type": "FAQPage",
      mainEntity: [
        {
          "@type": "Question",
          name: "Does spawnt store my source code or prompt contents?",
          acceptedAnswer: {
            "@type": "Answer",
            text: "No. spawnt enforces a non-negotiable invariant: zero content storage. Prompts, code, and tool outputs never touch disk or network.",
          },
        },
        {
          "@type": "Question",
          name: "Which AI coding agents does spawnt support?",
          acceptedAnswer: {
            "@type": "Answer",
            text: "spawnt supports Claude Code, OpenAI Codex, Google Antigravity, Google Gemini CLI, and Cursor.",
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
      <body className="bg-background text-foreground antialiased min-h-screen flex flex-col font-sans">
        <Navbar />
        <main className="flex-1">{children}</main>
        <FooterWrapper />
      </body>
    </html>
  );
}

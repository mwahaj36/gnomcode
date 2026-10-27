import type { Metadata } from "next";
import "./globals.css";
import "./spawnt-console.css";
import Navbar from "../components/Navbar";
import FooterWrapper from "../components/FooterWrapper";

export const metadata: Metadata = {
  metadataBase: new URL("https://fleethud.vercel.app"),
  title: {
    default: "spawnt — Mission Control for AI Coding Agents",
    template: "%s | spawnt",
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
  authors: [{ name: "spawnt Team" }],
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
    url: "https://fleethud.vercel.app",
    title: "spawnt — Live Mission Control for AI Coding Agents",
    description:
      "Monitor token usage, per-model quota pools, and live session states for Claude Code, Codex, and Antigravity with zero content storage.",
    siteName: "spawnt",
  },
  twitter: {
    card: "summary_large_image",
    title: "spawnt — Live Mission Control for AI Coding Agents",
    description:
      "Monitor token usage, per-model quota pools, and live session states for Claude Code, Codex, and Antigravity with zero content storage.",
  },
  alternates: {
    canonical: "https://fleethud.vercel.app",
  },
  icons: {
    icon: "/spawnt-mark.svg",
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
        "Live mission control, token spend attribution, and per-model quota monitoring for AI coding agents with zero content storage.",
    },
    {
      "@type": "Organization",
      name: "spawnt",
      url: "https://fleethud.vercel.app",
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
            text: "spawnt supports Claude Code, OpenAI Codex (CLI & Desktop App), Google Antigravity (IDE & Autonomous Agent), and Google Gemini CLI.",
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
        <script dangerouslySetInnerHTML={{ __html: `(function(){try{var t=localStorage.getItem('spawnt-theme');if(t==='light'||t==='dark')document.documentElement.dataset.theme=t;}catch(_){}})();` }} />
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
        />
      </head>
      <body className="bg-bg text-[var(--sp-text)] antialiased min-h-screen flex flex-col font-sans">
        <Navbar />
        <main className="flex-1">{children}</main>
        <FooterWrapper />
      </body>
    </html>
  );
}

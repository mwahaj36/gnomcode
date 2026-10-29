import branding from "../branding.json";
import type { Metadata } from "next";
import "./globals.css";
import "./spawnt-console.css";
import Navbar from "../components/Navbar";
import FooterWrapper from "../components/FooterWrapper";

export const metadata: Metadata = {
  metadataBase: new URL(branding.baseUrl),
  title: { default: "Spawnt — The Runtime for AI Coding Agents", template: "%s | Spawnt" },
  description: "Launch, switch and remotely control AI coding agents. Work across Claude Code, Codex and Gemini CLI from one place.",
  keywords: ["AI coding agents", "Claude Code", "OpenAI Codex", "Gemini CLI", "agent orchestrator", "switch AI models", "remote AI agent control", "AI developer tools", "multi-agent"],
  authors: [{ name: "Wahaj", url: "https://github.com/mwahaj36" }], creator: "Wahaj", publisher: "Spawnt",
  robots: { index: true, follow: true, googleBot: { index: true, follow: true } },
  icons: { icon: "/spawnt-mark.svg" },
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

      </head>
      <body className="bg-bg text-[var(--sp-text)] antialiased min-h-screen flex flex-col font-sans">
        <Navbar />
        <main className="flex-1">{children}</main>
        <FooterWrapper />
      </body>
    </html>
  );
}

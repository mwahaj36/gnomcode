import branding from "../branding.json";
import type { Metadata, Viewport } from "next";
import "./globals.css";
import "./gnomcode-console.css";
import "./controls.css";
import Navbar from "../components/Navbar";
import FooterWrapper from "../components/FooterWrapper";

export const metadata: Metadata = {
  metadataBase: new URL(branding.baseUrl),
  title: { default: "Gnomcode — The Runtime for AI Coding Agents", template: "%s | Gnomcode" },
  description: "Launch, switch and remotely control AI coding agents. Work across Claude Code, Codex and Antigravity from one place.",
  keywords: ["AI coding agents", "Claude Code", "OpenAI Codex", "Antigravity", "agent orchestrator", "switch AI models", "remote AI agent control", "AI developer tools", "multi-agent"],
  authors: [{ name: "Wahaj", url: "https://github.com/mwahaj36" }], creator: "Wahaj", publisher: "Gnomcode",
  robots: { index: true, follow: true, googleBot: { index: true, follow: true } },
  manifest: "/manifest.webmanifest",
  icons: { icon: "/gnomcode-mark.svg", apple: "/icons/gnomcode-192.png" },
  appleWebApp: {
    capable: true,
    statusBarStyle: "black-translucent",
    title: "Gnomcode",
  },
};

// viewport-fit=cover lets the installed app draw under the notch; the console pads with env(safe-area-inset-*).
export const viewport: Viewport = { width: "device-width", initialScale: 1, viewportFit: "cover" };

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <head>
        <meta name="theme-color" content="#070708" />
        <meta name="apple-mobile-web-app-capable" content="yes" />
        <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent" />
        <link rel="apple-touch-icon" href="/icons/gnomcode-192.png" />
        <script dangerouslySetInnerHTML={{ __html: `(function(){try{var t=localStorage.getItem('gnomcode-theme');if(t==='light'||t==='dark')document.documentElement.dataset.theme=t;}catch(_){}})();` }} />
        <script dangerouslySetInnerHTML={{ __html: `(function(){if(typeof window!=='undefined'&&'serviceWorker' in navigator){window.addEventListener('load',function(){navigator.serviceWorker.register('/sw.js').catch(function(){});});}})();` }} />
      </head>
      <body className="bg-bg text-[var(--sp-text)] antialiased min-h-screen flex flex-col font-sans">
        <a className="sp-global-skip" href="#main-content">Skip to content</a>
        <Navbar />
        <main id="main-content" className="flex-1">{children}</main>
        <FooterWrapper />
      </body>
    </html>
  );
}

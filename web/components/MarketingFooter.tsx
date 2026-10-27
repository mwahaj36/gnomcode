"use client";

import Link from "next/link";
import { useEffect, useState } from "react";

type Theme = "system" | "light" | "dark";

export default function MarketingFooter() {
  const [theme, setTheme] = useState<Theme>("system");
  useEffect(() => {
    const stored = localStorage.getItem("spawnt-theme");
    const next: Theme = stored === "light" || stored === "dark" ? stored : "system";
    setTheme(next);
    if (next === "system") delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = next;
  }, []);
  const cycleTheme = () => {
    const next: Theme = theme === "system" ? "light" : theme === "light" ? "dark" : "system";
    setTheme(next);
    if (next === "system") { localStorage.removeItem("spawnt-theme"); delete document.documentElement.dataset.theme; }
    else { localStorage.setItem("spawnt-theme", next); document.documentElement.dataset.theme = next; }
  };

  return <footer className="sp-site-footer" id="site-footer">
    <div className="sp-site-footer-grid">
      <div className="sp-site-footer-intro"><Link className="sp-site-brand" href="/"><span className="sp-brand-mark" aria-hidden="true" /><span>spawnt</span></Link><h2>Know what is running.</h2><p>spawnt reads local logs and stores metadata only. Prompts, code and output are never saved or sent.</p></div>
      <div><span>PRODUCT</span><Link href="/auth">Console</Link><Link href="/pricing">Pricing</Link><Link href="/downloads">Downloads</Link></div>
      <div><span>RESOURCES</span><Link href="/docs">Documentation</Link><Link href="/docs#privacy">Privacy</Link></div>
      <div><span>MORE</span><Link href="/auth">Account</Link><button className="sp-site-theme" type="button" onClick={cycleTheme} aria-label={`Theme: ${theme}. Switch mode`}>Theme / {theme}</button></div>
    </div>
    <div className="sp-site-wordmark" aria-hidden="true">spawnt<span>.</span></div>
    <div className="sp-site-footer-bottom"><span>© 2026 spawnt</span><span>LOCAL-FIRST SOFTWARE</span><a href="#site-footer" onClick={(event) => { event.preventDefault(); window.scrollTo({ top: 0, behavior: "smooth" }); }}>Back to top <span aria-hidden="true">↑</span></a></div>
  </footer>;
}

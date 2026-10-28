"use client";

import Link from "next/link";
import ThemeControl from "./ThemeControl";

export default function MarketingFooter() {
  return <footer className="sp-site-footer" id="site-footer">
    <div className="sp-site-footer-grid">
      <div className="sp-site-footer-intro"><Link className="sp-site-brand" href="/"><span className="sp-brand-mark" aria-hidden="true" /><span>spawnt</span></Link><h2>Know what is running.</h2><p>spawnt reads local logs and stores redacted monitoring metadata. Local chat history is planned.</p></div>
      <div><span>PRODUCT</span><Link href="/auth">Console</Link><Link href="/pricing">Pricing</Link><Link href="/downloads">Downloads</Link></div>
      <div><span>RESOURCES</span><Link href="/docs">Documentation</Link><Link href="/docs#privacy">Privacy</Link></div>
      <div><span>MORE</span><Link href="/auth">Account</Link><ThemeControl className="sp-site-theme" /></div>
    </div>
    <div className="sp-site-wordmark" aria-hidden="true">spawnt<span>.</span></div>
    <div className="sp-site-footer-bottom"><span>© 2026 spawnt</span><span>LOCAL-FIRST SOFTWARE</span><a href="#site-footer" onClick={(event) => { event.preventDefault(); window.scrollTo({ top: 0, behavior: "smooth" }); }}>Back to top <span aria-hidden="true">↑</span></a></div>
  </footer>;
}

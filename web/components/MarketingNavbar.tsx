"use client";

import Link from "next/link";
import AccountLink from "./AccountLink";
import { useEffect, useRef, useState } from "react";
import ThemeControl from "./ThemeControl";

export default function MarketingNavbar({ showTheme = false }: { showTheme?: boolean }) {
  const header = useRef<HTMLElement>(null);
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    let frame = 0;
    const update = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const progress = Math.min(window.scrollY / 72, 1);
        const inset = Math.max(16, (window.innerWidth - 1240) / 2) * progress;
        header.current?.style.setProperty("--nav-top", `${12 * progress}px`);
        header.current?.style.setProperty("--nav-inset", `${inset}px`);
        header.current?.style.setProperty("--nav-radius", `${6 * progress}px`);
      });
    };
    update();
    window.addEventListener("scroll", update, { passive: true });
    window.addEventListener("resize", update);
    return () => { cancelAnimationFrame(frame); window.removeEventListener("scroll", update); window.removeEventListener("resize", update); };
  }, []);

  return <header onKeyDown={event => { if (event.key === "Escape") { setMenuOpen(false); event.currentTarget.querySelector<HTMLButtonElement>(".sp-site-menu")?.focus(); } }} className="sp-site-header" ref={header}>
    <div className="sp-site-nav">
      <Link className="sp-site-brand" href="/"><span className="sp-brand-mark" aria-hidden="true" /><span>gnomcode</span></Link>
      <span className="sp-site-divider" aria-hidden="true">/</span>
      <nav id="marketing-navigation" onClick={() => setMenuOpen(false)} className={menuOpen ? "sp-site-links open" : "sp-site-links"} aria-label="Main navigation">
        <Link href="/#features" onClick={() => setMenuOpen(false)}>Product</Link>
        <Link href="/#agents" onClick={() => setMenuOpen(false)}>Agents</Link>
        <Link href="/docs">Docs</Link>
        <Link href="/docs/getting-started">Get started</Link>
        <Link className="sp-site-mobile-download" href="/downloads" onClick={() => setMenuOpen(false)}>Download</Link>
        {showTheme && <ThemeControl className="sp-mobile-theme" />}
      </nav>
      <div className="sp-site-actions">{showTheme && <ThemeControl className="sp-nav-theme" />}<Link className="sp-site-download" href="/downloads">Download <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M5 19 19 5M5 5h14v14" /></svg></Link><AccountLink className="sp-site-login sp-account-cta" arrow /></div>
      <button className="sp-site-menu" type="button" aria-controls="marketing-navigation" aria-label="Toggle menu" aria-expanded={menuOpen} onClick={() => setMenuOpen((value) => !value)}><span /><span /><span /></button>
    </div>
  </header>;
}

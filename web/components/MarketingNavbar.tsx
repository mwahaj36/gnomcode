"use client";

import Link from "next/link";
import { useEffect, useRef, useState } from "react";

export default function MarketingNavbar() {
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

  return <header className="sp-site-header" ref={header}>
    <div className="sp-site-nav">
      <Link className="sp-site-brand" href="/"><span className="sp-brand-mark" aria-hidden="true" /><span>spawnt</span></Link>
      <span className="sp-site-divider" aria-hidden="true">/</span>
      <nav className={menuOpen ? "sp-site-links open" : "sp-site-links"} aria-label="Main navigation">
        <Link href="/#features" onClick={() => setMenuOpen(false)}>Product</Link>
        <Link href="/#agents" onClick={() => setMenuOpen(false)}>Agents</Link>
        <Link href="/docs">Docs</Link>
        <Link href="/pricing">Pricing</Link>
        <Link className="sp-site-mobile-login" href="/auth" onClick={() => setMenuOpen(false)}>Log in</Link>
      </nav>
      <div className="sp-site-actions"><Link className="sp-site-login" href="/auth">Log in <span aria-hidden="true">↗</span></Link><Link className="sp-site-download" href="/downloads">Download <span aria-hidden="true">↗</span></Link></div>
      <button className="sp-site-menu" type="button" aria-label="Toggle menu" aria-expanded={menuOpen} onClick={() => setMenuOpen((value) => !value)}><span /><span /><span /></button>
    </div>
  </header>;
}

"use client";

import Link from "next/link";
import AccountLink from "../components/AccountLink";
import { useEffect, useRef, useState } from "react";
import "./landing.css";
import ThemeControl from "../components/ThemeControl";
import { PlatformDownloads } from "./downloads/PlatformDownloads";
import { landingFaq } from "../lib/landing-faq";
import { ConsoleTourView } from "./components/ConsoleTourView";
import HeroWorkflow from "./components/HeroWorkflow";

const agents = [
  { name: "Claude Code", icons: ["claudecode"] },
  { name: "Codex", icons: ["codex"] },
  { name: "Antigravity", icons: ["antigravity"] },
];
const tours = [
  { name: "Overview", title: "Everything happening now.", detail: "See active sessions, spend and the limits that need attention.", kind: "overview" },
  { name: "Sessions", title: "Every run, in one place.", detail: "Find the agent waiting for you and open its session.", kind: "sessions" },
  { name: "Usage", title: "Know where the tokens went.", detail: "Sample seven-day usage and API-equivalent cost by agent.", kind: "usage" },
  { name: "Limits", title: "See what is left.", detail: "Quota usage, remaining capacity and reset times at a glance.", kind: "limits" },
];
const features = [
  ["Hit a limit. Keep the task moving.", "Switch supported agents with a compact briefing of objectives and changed files. Keep the original history and continue with another model."],
  ["Away from your desk. Still in control.", "Read the conversation, send the next instruction and approve Claude Code permission prompts from your phone over an end-to-end encrypted channel."],
  ["One fleet. One view of usage.", "Compare recorded tokens, estimated API-equivalent spend and quota across agents. Every measurement carries its trust level."],
];
const plans = [
  { name: "Local", price: "$0", term: "no account", promise: "Run locally without an account.", points: ["Session and quota view", "Guardrails and resume", "Analytics and export"], href: "/downloads", action: "Download" },
  { name: "Remote", price: "Remote", term: "prototype", promise: "Connect to supported sessions from your phone.", points: ["End-to-end encrypted access", "Linked devices you can revoke", "Session controls"], href: "/pricing", action: "Explore Remote" },
];

function Mark() { return <span className="sp-brand-mark sp-mark" aria-hidden="true" />; }

export default function LandingPage() {
  const [tab, setTab] = useState(0);
  const tourTabs = useRef<Array<HTMLButtonElement | null>>([]);
  const [tourVisible, setTourVisible] = useState(false);
  const headerRef = useRef<HTMLElement>(null);
  const tourScreenRef = useRef<HTMLDivElement>(null);
  const marqueeRef = useRef<HTMLDivElement>(null);
  const speedFrame = useRef(0);
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    let frame = 0;
    const update = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const progress = Math.min(window.scrollY / 72, 1);
        const inset = Math.max(16, (window.innerWidth - 1240) / 2) * progress;
        headerRef.current?.style.setProperty("--nav-top", `${12 * progress}px`);
        headerRef.current?.style.setProperty("--nav-inset", `${inset}px`);
        headerRef.current?.style.setProperty("--nav-radius", `${6 * progress}px`);
        headerRef.current?.style.setProperty("--nav-shadow", `${0.5 * progress}`);
        headerRef.current?.style.setProperty("--nav-border", `${progress}`);
      });
    };
    update();
    window.addEventListener("scroll", update, { passive: true });
    window.addEventListener("resize", update);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("scroll", update);
      window.removeEventListener("resize", update);
    };
  }, []);

  useEffect(() => {
    if (tourVisible) return;
    const screen = tourScreenRef.current;
    if (!screen) return;
    const observer = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting) {
        setTourVisible(true);
        observer.disconnect();
      }
    }, { threshold: 0.35 });
    observer.observe(screen);
    return () => observer.disconnect();
  }, [tab, tourVisible]);

  function setMarqueeSpeed(target: number) {
    const animation = marqueeRef.current?.getAnimations()[0];
    if (!animation) return;
    cancelAnimationFrame(speedFrame.current);
    const start = animation.playbackRate;
    const started = performance.now();
    const step = (now: number) => {
      const progress = Math.min((now - started) / 450, 1);
      animation.playbackRate = start + (target - start) * (1 - Math.pow(1 - progress, 3));
      if (progress < 1) speedFrame.current = requestAnimationFrame(step);
    };
    speedFrame.current = requestAnimationFrame(step);
  }

  useEffect(() => () => cancelAnimationFrame(speedFrame.current), []);

  useEffect(() => {
    const sections = document.querySelectorAll<HTMLElement>(".sp-page [data-reveal]");
    const observer = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          entry.target.classList.add("revealed");
          observer.unobserve(entry.target);
        }
      });
    }, { threshold: 0.08, rootMargin: "0px 0px -32px 0px" });
    sections.forEach((section) => {
      if (section.getBoundingClientRect().top > window.innerHeight) section.classList.add("reveal-pending");
      observer.observe(section);
    });
    return () => observer.disconnect();
  }, []);

  return <div className="sp-page">
    <a className="sp-skip" href="#content">Skip to content</a>
    <header onKeyDown={event => { if (event.key === "Escape") { setMenuOpen(false); event.currentTarget.querySelector<HTMLButtonElement>(".sp-menu")?.focus(); } }} className="sp-header" ref={headerRef}>
      <div className="sp-nav sp-width">
        <Link className="sp-brand" href="/"><Mark /><span>gnomcode</span></Link>
        <span className="sp-nav-slash" aria-hidden="true">/</span>
        <nav id="marketing-navigation" onClick={() => setMenuOpen(false)} className={menuOpen ? "sp-nav-links open" : "sp-nav-links"} aria-label="Main navigation">
          <a href="#features" onClick={() => setMenuOpen(false)}>Product</a>
          <a href="#agents" onClick={() => setMenuOpen(false)}>Agents</a>
          <Link href="/docs">Docs</Link>
          <Link href="/pricing">Pricing</Link>
          <Link className="sp-nav-download-mobile" href="/downloads" onClick={() => setMenuOpen(false)}>Download</Link>
        </nav>
        <div className="sp-nav-right"><Link href="/downloads" className="sp-nav-cta">Download <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M5 19 19 5M5 5h14v14" /></svg></Link><AccountLink className="sp-nav-login sp-account-cta" arrow /></div>
        <button className="sp-menu" type="button" onClick={() => setMenuOpen(!menuOpen)} aria-expanded={menuOpen} aria-controls="marketing-navigation" aria-label="Toggle menu"><span /><span /><span /></button>
      </div>
    </header>

    <div id="content">
      <section className="sp-hero sp-width">
        <div className="sp-hero-copy"><h1>Switch agents. Keep the task moving.</h1><p>Start with Claude Code, continue with Codex. Gnomcode carries a concise handoff so the next agent can pick up the task.</p><div className="sp-actions"><Link className="sp-button-primary" href="/downloads">Download for Windows <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M4 12h16m-6-6 6 6-6 6" /></svg></Link><Link className="sp-button-outline" href="/docs">View docs <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M5 19 19 5M5 5h14v14" /></svg></Link></div><small>Windows development build available. Local features are free.</small></div>
        <HeroWorkflow />
      </section>



      <section className="sp-marquee" id="agents" aria-label="Supported agents"><div className="sp-width"><span className="sp-label">WORKS WITH</span><div className="sp-marquee-window" onMouseEnter={() => setMarqueeSpeed(0)} onMouseLeave={() => setMarqueeSpeed(1)}><div className="sp-marquee-track" ref={marqueeRef}>{[false, true].map((duplicate) => <div className="sp-agent-list" aria-hidden={duplicate || undefined} key={String(duplicate)}>{agents.map((agent) => <span className="sp-agent-item" key={agent.name}><span className="sp-agent-icons" aria-hidden="true">{agent.icons.map((icon) => <span className="sp-agent-icon" style={{ maskImage: `url(/agents/${icon}.svg)`, WebkitMaskImage: `url(/agents/${icon}.svg)` }} key={icon} />)}</span><strong>{agent.name}</strong></span>)}</div>)}</div></div></div></section>

      <section className="sp-features sp-width" id="features" data-reveal><div className="sp-section-head"><h2>Keep working across agents.</h2><span>LAUNCH / SWITCH / CONTINUE</span></div><div className="sp-feature-grid">{features.map(([title, copy]) => <article key={title}><h3>{title}</h3><p>{copy}</p><span className="sp-feature-line" aria-hidden="true" /></article>)}</div></section>

      <section className="sp-tour sp-width" aria-labelledby="tour-title" data-reveal>
        <div className="sp-section-head"><div><h2 id="tour-title">A closer look at the console.</h2><p>Find what matters without opening every agent window.</p></div><span>ILLUSTRATIVE PRODUCT TOUR</span></div>
        <div className="sp-tour-tabs" role="tablist" aria-label="Console views">{tours.map((view, index) => <button type="button" role="tab" id={`tour-tab-${index}`} aria-controls="tour-panel" tabIndex={tab === index ? 0 : -1} ref={node => { tourTabs.current[index] = node; }} aria-selected={tab === index} key={view.name} onClick={() => setTab(index)} onKeyDown={event => {
          if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
          event.preventDefault();
          const next = event.key === "Home" ? 0 : event.key === "End" ? tours.length - 1 : (index + (event.key === "ArrowRight" ? 1 : tours.length - 1)) % tours.length;
          setTab(next); tourTabs.current[next]?.focus();
        }}>{view.name}</button>)}</div>
        <div className="sp-tour-panel" role="tabpanel" id="tour-panel" aria-labelledby={`tour-tab-${tab}`} tabIndex={0} key={tab}>
          <div className="sp-tour-copy"><span className="sp-label">{tours[tab].name.toUpperCase()} / CONSOLE</span><h3>{tours[tab].title}</h3><p>{tours[tab].detail}</p><Link href="/auth">Open console <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M4 12h16m-6-6 6 6-6 6" /></svg></Link></div>
          <div className="sp-tour-screen" ref={tourScreenRef}>
            <div className="sp-tour-screen-head"><span><Mark /> gnomcode / {tours[tab].name.toLowerCase()}</span><span>RUN · LOCAL</span></div>
            <div className={tourVisible ? "sp-tour-screen-body visible" : "sp-tour-screen-body"} key={`${tab}-${tourVisible}`}>
              <small>{tours[tab].name.toUpperCase()} / ILLUSTRATIVE DATA</small><h4>{tours[tab].title}</h4>
              <ConsoleTourView kind={tours[tab].kind} visible={tourVisible} />
            </div>
          </div>
        </div>
      </section>

      <section className="sp-privacy" data-reveal><div className="sp-width"><div className="sp-section-head"><h2>Local by default. Encrypted when remote.</h2><span>PRIVACY / BY DESIGN</span></div><div className="sp-privacy-grid">{[["Local history", "Keep redacted conversations on your desktop."], ["Encrypted Remote", "Conversations reach your paired device through E2EE."], ["No telemetry", "No product analytics or tracking."], ["Your choice", "Account metadata sync is optional and separately disclosed."]].map(([title, copy]) => <div key={title}><h3>{title}</h3><p>{copy}</p></div>)}</div></div></section>

      <section className="sp-pricing sp-width" data-reveal aria-labelledby="pricing-title"><div className="sp-section-head"><div><h2 id="pricing-title">All local features are free.</h2><p>Remote is a prototype and its pricing is not finalized. Provider inference is billed separately.</p></div><Link href="/pricing">Compare plans <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M5 19 19 5M5 5h14v14" /></svg></Link></div><div className="sp-plan-grid">{plans.map((plan) => <article key={plan.name}><span className="sp-label">{plan.name}</span><div className="sp-plan-price"><strong>{plan.price}</strong><span>{plan.term}</span></div><p>{plan.promise}</p><ul>{plan.points.map((point) => <li key={point}>{point}</li>)}</ul><Link href={plan.href}>{plan.action} <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M4 12h16m-6-6 6 6-6 6" /></svg></Link></article>)}</div></section>

      <section className="sp-width sp-landing-downloads" data-reveal><div className="sp-section-head"><h2>Choose your platform.</h2></div><PlatformDownloads /></section>

      <section className="sp-width sp-landing-faq" aria-labelledby="faq-title"><h2 id="faq-title">Questions before you start?</h2>{landingFaq.map(item => <details key={item.question}><summary>{item.question}</summary><p>{item.answer}</p></details>)}</section>
      <section className="sp-final sp-width" data-reveal><h2>Open your next session.</h2><p>Bring your agents into one workspace.</p><Link className="sp-button-primary" href="/downloads">Download <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M4 12h16m-6-6 6 6-6 6" /></svg></Link></section>
    </div>
    <footer className="sp-footer" data-reveal>
      <div className="sp-width sp-footer-grid">
        <div className="sp-footer-intro"><Link className="sp-brand" href="/"><Mark /><span>gnomcode</span></Link><h2>Keep your agents moving.</h2><p>Launch, switch and control AI coding agents. Local-first, with encrypted access from paired devices.</p></div>
        <div><span>PRODUCT</span><Link href="/auth">Console</Link><Link href="/pricing">Pricing</Link><Link href="/downloads">Downloads</Link></div>
        <div><span>RESOURCES</span><Link href="/docs">Documentation</Link><Link href="/docs/supported-agents">Supported agents</Link></div>
        <div><span>LEGAL</span><Link href="/privacy">Privacy</Link><Link href="/terms">Terms</Link><Link href="/auth">Account</Link><ThemeControl className="sp-footer-theme" /></div>
      </div>
      <div className="sp-width sp-footer-wordmark" aria-hidden="true">gnomcode<span>.</span></div>
      <div className="sp-width sp-footer-bottom"><span>© 2026 gnomcode</span><span>LOCAL-FIRST SOFTWARE</span><a href="#content">Back to top <span aria-hidden="true">↑</span></a></div>
    </footer>
  </div>;
}

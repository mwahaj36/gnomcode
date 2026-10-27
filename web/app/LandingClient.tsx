"use client";

import Link from "next/link";
import { useEffect, useRef, useState } from "react";
import "./landing.css";
import ThemeControl from "../components/ThemeControl";

const agents = [
  { name: "Claude Code", icons: ["claudecode"] },
  { name: "Codex", icons: ["codex"] },
  { name: "Antigravity", icons: ["antigravity"] },
  { name: "Gemini CLI", icons: ["gemini"] },
  { name: "Cline / Roo", icons: ["cline", "roocode"] },
  { name: "Cursor", icons: ["cursor"] },
];
const tours = [
  { name: "Overview", title: "Everything happening now.", detail: "See active sessions, spend and the limits that need attention.", kind: "overview", bars: [42, 58, 35, 75, 52, 65, 88], rows: [["Codex", "[RUN]", "18.4k", "EXACT"], ["Claude Code", "[WAIT]", "12.8k", "EST"], ["Antigravity", "[IDLE]", "8.1k", "EST"]] },
  { name: "Sessions", title: "Every run, in one place.", detail: "Find the agent waiting for you and open its session.", kind: "sessions", bars: [30, 72, 48, 90, 62, 38, 76], rows: [["Claude Code", "[WAIT]", "repo: api", "UNKNOWN"], ["Codex", "[RUN]", "repo: web", "EXACT"], ["Gemini CLI", "[IDLE]", "repo: docs", "EST"]] },
  { name: "Usage", title: "Know where the tokens went.", detail: "Sample seven-day usage and API-equivalent cost by agent.", kind: "usage", bars: [48, 32, 66, 44, 86, 58, 72], rows: [["Codex · GPT-5", "124.0k tokens", "$1.84", "API"], ["Claude · Sonnet", "98.0k tokens", "$1.42", "EST"], ["Gemini · Flash", "74.0k tokens", "$0.52", "EST"]] },
  { name: "Limits", title: "See what is left.", detail: "Quota usage, remaining capacity and reset times at a glance.", kind: "limits", bars: [64, 38, 80, 53, 72, 46, 58], rows: [["Codex · 5 hour", "[API]", "62% left · 2h 14m", "API"], ["Claude · weekly", "[EST]", "28% left · 3d 6h", "EST"], ["Gemini · daily", "[API]", "44% left · 8h 20m", "API"]] },
];
const features = [
  ["See it live", "Every session, its status and which ones need you, in one list."],
  ["Know the cost", "Tokens and spend per agent, model and repo. Every number says how sure we are."],
  ["Stay under limits", "Track what is left in each plan and pick up where you stopped when it resets."],
];
const plans = [
  { name: "Free", price: "$0", term: "forever", promise: "See your agents on one machine.", points: ["Local session view", "Usage and cost", "Quota tracking"], href: "/downloads", action: "Download" },
  { name: "Pro", price: "$19", term: "one-time", promise: "More control for your own work.", points: ["Everything in Free", "Resume queue", "Guardrails and export"], href: "/pricing", action: "View Pro" },
  { name: "Teams", price: "$12", term: "/ seat / month", promise: "Shared visibility for a team.", points: ["Team dashboard", "Shared caps", "Team alerts"], href: "/pricing", action: "View Teams" },
];

function Mark() { return <span className="sp-brand-mark sp-mark" aria-hidden="true" />; }

export default function LandingPage() {
  const [tab, setTab] = useState(0);
  const [selectedDay, setSelectedDay] = useState(6);
  const [selectedRow, setSelectedRow] = useState(0);
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
    <header className="sp-header" ref={headerRef}>
      <div className="sp-nav sp-width">
        <Link className="sp-brand" href="/"><Mark /><span>spawnt</span></Link>
        <span className="sp-nav-slash" aria-hidden="true">/</span>
        <nav className={menuOpen ? "sp-nav-links open" : "sp-nav-links"} aria-label="Main navigation">
          <a href="#features" onClick={() => setMenuOpen(false)}>Product</a>
          <a href="#agents" onClick={() => setMenuOpen(false)}>Agents</a>
          <Link href="/docs">Docs</Link>
          <Link href="/pricing">Pricing</Link>
          <Link className="sp-nav-login-mobile" href="/auth" onClick={() => setMenuOpen(false)}>Log in</Link>
        </nav>
        <div className="sp-nav-right"><Link href="/auth" className="sp-nav-login">Log in <span aria-hidden="true">↗</span></Link><Link href="/downloads" className="sp-nav-cta">Download <span aria-hidden="true">↗</span></Link></div>
        <button className="sp-menu" type="button" onClick={() => setMenuOpen(!menuOpen)} aria-expanded={menuOpen} aria-label="Toggle menu"><span /><span /><span /></button>
      </div>
    </header>

    <main id="content">
      <section className="sp-hero sp-width">
        <div className="sp-hero-copy"><span className="sp-eyebrow">WINDOWS · MACOS · LINUX</span><h1>See, stop, and control your agents.</h1><p>Know who is blocked on approval, halt runaway loops before they drain your API bill, and auto-resume at quota reset.</p><div className="sp-actions"><Link className="sp-button-primary" href="/downloads">Download for Windows <span aria-hidden="true">→</span></Link><Link className="sp-button-outline" href="/docs">View docs <span aria-hidden="true">↗</span></Link></div><small>Local-first. Nothing leaves your machine.</small></div>
        <div className="sp-hero-shot" aria-label="Illustrative console preview">
          <div className="sp-shot-head"><span><Mark /> spawnt <b>/</b> overview</span><span className="sp-demo-label">ILLUSTRATIVE PREVIEW</span></div>
          <div className="sp-shot-body"><div className="sp-shot-top"><div><small>OVERVIEW / LIVE</small><h2>What your agents are doing now.</h2></div><span className="sp-connection">[RUN] LOCAL</span></div><div className="sp-kpis"><div><small>SPEND / TODAY</small><strong>$4.86</strong><span>EST</span></div><div><small>TOKENS / TODAY</small><strong>39.3k</strong><span>EST</span></div><div><small>ACTIVE SESSIONS</small><strong>2</strong><span>EXACT</span></div></div><div className="sp-shot-table-title">LIVE SESSIONS <span>VIEW ALL ↗</span></div><div className="sp-shot-table">{tours[0].rows.map((row) => <div key={row[0]}><b>{row[0]}</b><span className={row[1] === "[WAIT]" ? "sp-wait" : ""}>{row[1]}</span><span>{row[2]} tokens</span><em>{row[3]}</em></div>)}</div></div>
        </div>
      </section>

      <section className="sp-marquee" id="agents" aria-label="Supported agents"><div className="sp-width"><span className="sp-label">WORKS WITH</span><div className="sp-marquee-window" onMouseEnter={() => setMarqueeSpeed(0)} onMouseLeave={() => setMarqueeSpeed(1)}><div className="sp-marquee-track" ref={marqueeRef}>{[false, true].map((duplicate) => <div className="sp-agent-list" aria-hidden={duplicate || undefined} key={String(duplicate)}>{agents.map((agent) => <span className="sp-agent-item" key={agent.name}><span className="sp-agent-icons" aria-hidden="true">{agent.icons.map((icon) => <span className="sp-agent-icon" style={{ maskImage: `url(/agents/${icon}.svg)`, WebkitMaskImage: `url(/agents/${icon}.svg)` }} key={icon} />)}</span><strong>{agent.name}</strong></span>)}</div>)}</div></div></div></section>

      <section className="sp-features sp-width" id="features" data-reveal><div className="sp-section-head"><h2>One place to see it all.</h2><span>STATUS / COST / LIMITS</span></div><div className="sp-feature-grid">{features.map(([title, copy], index) => <article key={title}><span className="sp-feature-index">0{index + 1}</span><h3>{title}</h3><p>{copy}</p><span className="sp-feature-line" aria-hidden="true" /></article>)}</div></section>

      <section className="sp-tour sp-width" aria-labelledby="tour-title" data-reveal>
        <div className="sp-section-head"><div><h2 id="tour-title">A closer look at the console.</h2><p>Find what matters without opening every agent window.</p></div><span>ILLUSTRATIVE PRODUCT TOUR</span></div>
        <div className="sp-tour-tabs" role="tablist" aria-label="Console views">{tours.map((view, index) => <button type="button" role="tab" aria-selected={tab === index} key={view.name} onClick={() => { setTab(index); setSelectedRow(0); setSelectedDay(6); }}>{view.name}</button>)}</div>
        <div className="sp-tour-panel" role="tabpanel" key={tab}>
          <div className="sp-tour-copy"><span className="sp-label">{tours[tab].name.toUpperCase()} / CONSOLE</span><h3>{tours[tab].title}</h3><p>{tours[tab].detail}</p><Link href="/auth">Open console <span aria-hidden="true">→</span></Link></div>
          <div className="sp-tour-screen" ref={tourScreenRef}>
            <div className="sp-tour-screen-head"><span><Mark /> spawnt / {tours[tab].name.toLowerCase()}</span><span>[RUN] LOCAL</span></div>
            <div className={tourVisible ? "sp-tour-screen-body visible" : "sp-tour-screen-body"} key={`${tab}-${tourVisible}`}>
              <small>{tours[tab].name.toUpperCase()} / ILLUSTRATIVE DATA</small><h4>{tours[tab].title}</h4>
              {tours[tab].kind === "overview" && <div className="sp-tour-kpis"><div><small>ACTIVE</small><strong>2 <em>EXACT</em></strong></div><div><small>TOKENS / TODAY</small><strong>39.3k <em>EST</em></strong></div><div><small>COST / TODAY</small><strong>$4.86 <em>API</em></strong></div></div>}
              {tours[tab].kind === "sessions" && <div className="sp-tour-summary"><span>4 RUNS</span><span>1 WAITING</span><span>3 AGENTS</span></div>}
              {tours[tab].kind === "usage" && <div className="sp-tour-usage-summary"><span>296.0k TOKENS <em>EST</em></span><strong>$3.78 <em>API</em></strong><small>API-equivalent cost · sample week</small></div>}
              {tours[tab].kind === "limits" && <div className="sp-tour-summary"><span>3 POOLS</span><span>62% MOST REMAINING</span><span>1 ESTIMATE</span></div>}
              <div className="sp-tour-bars" role="group" aria-label={tours[tab].kind === "limits" ? "Illustrative quota remaining by pool" : "Illustrative daily activity, tokens, or quota by day"}>{["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].map((day, index) => <button type="button" key={day} style={{ "--bar-height": `${tours[tab].bars[index]}%` } as React.CSSProperties} className={selectedDay === index ? "selected" : ""} onClick={() => setSelectedDay(index)} aria-label={`${day}, sample ${tours[tab].kind === "limits" ? "quota" : "usage"}`} aria-pressed={selectedDay === index} title={`${day}, sample ${tours[tab].kind === "limits" ? "quota" : "usage"}`} />)}</div>
              {tours[tab].rows.map((row, index) => <button type="button" className={selectedRow === index ? "sp-tour-data selected" : "sp-tour-data"} aria-pressed={selectedRow === index} onClick={() => setSelectedRow(index)} key={row[0]}><b>{row[0]}</b><span>{row[1]}</span><span>{row[2]}</span><em>{row[3]}</em></button>)}
              <div className="sp-tour-selection" aria-live="polite"><span>{["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"][selectedDay]} / SAMPLE PERIOD</span><strong>{tours[tab].rows[selectedRow][0]}</strong><span>{tours[tab].rows[selectedRow][2]} · {tours[tab].rows[selectedRow][3]}</span></div>
            </div>
          </div>
        </div>
      </section>

      <section className="sp-privacy" data-reveal><div className="sp-width"><div className="sp-section-head"><h2>Your logs never leave your machine.</h2><span>PRIVACY / BY DESIGN</span></div><div className="sp-privacy-grid">{[["Local only", "Reads logs on your machine."], ["Metadata only", "No prompts, code or output stored."], ["No telemetry", "Nothing phones home."], ["Loopback", "Only your machine can reach it."]].map(([title, copy]) => <div key={title}><h3>{title}</h3><p>{copy}</p></div>)}</div></div></section>

      <section className="sp-pricing sp-width" data-reveal aria-labelledby="pricing-title"><div className="sp-section-head"><div><h2 id="pricing-title">Start free. Add control when you need it.</h2><p>Plans from the current product specification.</p></div><Link href="/pricing">Compare plans <span aria-hidden="true">↗</span></Link></div><div className="sp-plan-grid">{plans.map((plan) => <article key={plan.name}><span className="sp-label">{plan.name}</span><div className="sp-plan-price"><strong>{plan.price}</strong><span>{plan.term}</span></div><p>{plan.promise}</p><ul>{plan.points.map((point) => <li key={point}>{point}</li>)}</ul><Link href={plan.href}>{plan.action} <span aria-hidden="true">→</span></Link></article>)}</div></section>

      <section className="sp-final sp-width" data-reveal><h2>Start in a minute.</h2><p>Download spawnt and open your first session.</p><Link className="sp-button-primary" href="/downloads">Download <span aria-hidden="true">→</span></Link></section>
    </main>
    <footer className="sp-footer" data-reveal>
      <div className="sp-width sp-footer-grid">
        <div className="sp-footer-intro"><Link className="sp-brand" href="/"><Mark /><span>spawnt</span></Link><h2>Know what is running.</h2><p>spawnt reads local logs and stores metadata only. Prompts, code and output are never saved or sent.</p></div>
        <div><span>PRODUCT</span><Link href="/auth">Console</Link><Link href="/pricing">Pricing</Link><Link href="/downloads">Downloads</Link></div>
        <div><span>RESOURCES</span><Link href="/docs">Documentation</Link><Link href="/docs#privacy">Privacy</Link></div>
        <div><span>MORE</span><Link href="/auth">Account</Link><ThemeControl className="sp-footer-theme" /></div>
      </div>
      <div className="sp-width sp-footer-wordmark" aria-hidden="true">spawnt<span>.</span></div>
      <div className="sp-width sp-footer-bottom"><span>© 2026 spawnt</span><span>LOCAL-FIRST SOFTWARE</span><a href="#content">Back to top <span aria-hidden="true">↑</span></a></div>
    </footer>
  </div>;
}

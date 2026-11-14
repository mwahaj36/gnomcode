import { ProviderLogo } from "./ProviderLogo";

const nav = ["Home", "Chats", "Usage", "Settings"];
const chats = [
  { title: "Build the checkout flow", meta: "storefront · Codex", group: "running", active: true },
  { title: "Fix flaky checkout test", meta: "storefront · Claude Code", group: "needs_you" },
  { title: "Build pipeline cleanup", meta: "infra · Antigravity", group: "running", wsl: "Ubuntu" },
];

/** Illustrative console window: the 268px sidebar beside a conversation that hands off from Claude Code to Codex. */
export default function HeroWorkflow() {
  return <figure className="sp-workflow-visual" aria-label="Illustrative console showing a concise handoff from Claude Code to Codex">
    <div className="sp-workflow-desktop sp-hero-console">
      <aside className="sp-hero-side" aria-hidden="true">
        <div className="sp-hero-brand"><i className="sp-brand-mark" /> gnomcode</div>
        <ul className="sp-hero-nav">{nav.map(item => <li key={item} className={item === "Chats" ? "is-active" : undefined}>{item}</li>)}</ul>
        <div className="sp-hero-search">Search sessions</div>
        <ul className="sp-hero-chats">{chats.map(chat => <li key={chat.title} className={chat.active ? "is-active" : undefined}>
          <span className={`sp-session-dot sp-session-dot--${chat.group}`} />
          <span><strong>{chat.title}</strong><small>{chat.wsl && <b className="sp-badge sp-wsl-badge">WSL: {chat.wsl}</b>}{chat.meta}</small></span>
        </li>)}</ul>
      </aside>
      <div className="sp-hero-main">
        <div className="sp-workflow-task"><h2>Build the checkout flow</h2><span className="sp-chip sp-chip--ok">Running</span></div>
        <div className="sp-workflow-thread">
          <div className="sp-workflow-message is-user">Build checkout with saved addresses.</div>
          <div className="sp-workflow-message is-agent"><strong><ProviderLogo provider="claude-code" size={14} /> Claude Code</strong>Checkout is implemented. Keyboard navigation still needs testing.</div>
          <div className="sp-workflow-handoff">
            <div className="sp-handoff-heading"><span>Concise handoff</span><span>Same conversation</span></div>
            <div className="sp-handoff-agents"><strong><ProviderLogo provider="claude-code" size={14} /> Claude Code</strong><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true"><path d="M4 12h16m-6-6 6 6-6 6" /></svg><strong><ProviderLogo provider="codex" size={14} /> Codex</strong></div>
            <div className="sp-handoff-details"><div><span>Task</span><p>Finish the checkout flow</p></div><div><span>Context</span><p>Changed files and recent activity</p></div><div><span>Next</span><p>Test keyboard navigation</p></div></div>
          </div>
          <div className="sp-workflow-message is-agent sp-handoff-reply"><strong><ProviderLogo provider="codex" size={14} /> Codex</strong>I will start with the keyboard navigation tests.</div>
        </div>
        <div className="sp-workflow-composer"><span>Continue the task...</span><span><ProviderLogo provider="codex" size={14} /> Codex <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" aria-hidden="true"><path d="m4 6 4 4 4-4" /></svg></span></div>
      </div>
    </div>
    <figcaption>One task. A concise handoff. <span>ILLUSTRATIVE</span></figcaption>
  </figure>;
}

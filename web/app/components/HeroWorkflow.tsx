export default function HeroWorkflow() {
  return <figure className="sp-workflow-visual" aria-label="Illustrative workflow: launch an agent, carry context to another agent and continue from your phone">
    <div className="sp-workflow-desktop">
      <div className="sp-workflow-chrome"><span><i className="sp-brand-mark" aria-hidden="true" /> gnomcode</span><span>Workspace / storefront</span></div>
      <div className="sp-workflow-task"><h2>Build the checkout flow</h2><span>Conversation</span></div>
      <div className="sp-workflow-thread">
        <div className="sp-workflow-message is-user">Build checkout with saved addresses.</div>
        <div className="sp-workflow-message is-agent">The checkout flow is ready. Next up: tests and keyboard navigation.</div>
        <div className="sp-workflow-handoff"><span aria-hidden="true"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M4 7h15m-4-4 4 4-4 4M20 17H5m4-4-4 4 4 4" /></svg></span><div><strong>Agent switched</strong><span>Context carried forward</span></div></div>
        <div className="sp-workflow-message is-agent">I have the task and changed files. I’ll pick up from here.</div>
      </div>
      <div className="sp-workflow-composer"><span>Continue the task…</span><span>Agent / model <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" aria-hidden="true"><path d="m4 6 4 4 4-4" /></svg></span></div>
    </div>
    <div className="sp-workflow-phone">
      <div className="sp-workflow-phone-title"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true"><rect x="6" y="2" width="12" height="20" rx="2" /><path d="M10 18h4" /></svg><span>From your phone</span><span className="sp-workflow-encrypted"><svg viewBox="0 0 16 16" fill="none" stroke="currentColor" aria-hidden="true"><rect x="3" y="7" width="10" height="7" rx="1" /><path d="M5 7V5a3 3 0 0 1 6 0v2" /></svg>Encrypted</span></div>
      <div className="sp-workflow-phone-prompt"><span>Test keyboard navigation too.</span><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true"><path d="M12 19V5m-6 6 6-6 6 6" /></svg></div>
    </div>
    <figcaption>One task. Across agents and devices. <span>Illustrative workflow</span></figcaption>
  </figure>;
}

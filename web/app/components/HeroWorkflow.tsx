export default function HeroWorkflow() {
  return <figure className="sp-workflow-visual" aria-label="Illustrative workflow showing a concise handoff from Claude Code to Codex">
    <div className="sp-workflow-desktop">
      <div className="sp-workflow-chrome"><span><i className="sp-brand-mark" aria-hidden="true" /> gnomcode</span><span>Workspace / storefront</span></div>
      <div className="sp-workflow-task"><h2>Build the checkout flow</h2><span>Conversation</span></div>
      <div className="sp-workflow-thread">
        <div className="sp-workflow-message is-user">Build checkout with saved addresses.</div>
        <div className="sp-workflow-message is-agent"><strong>Claude Code</strong>Checkout is implemented. Keyboard navigation still needs testing.</div>
        <div className="sp-workflow-handoff">
          <div className="sp-handoff-heading"><span>Concise handoff</span><span>Same conversation</span></div>
          <div className="sp-handoff-agents"><strong>Claude Code</strong><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true"><path d="M4 12h16m-6-6 6 6-6 6" /></svg><strong>Codex</strong></div>
          <div className="sp-handoff-details"><div><span>Task</span><p>Finish the checkout flow</p></div><div><span>Context</span><p>Changed files and recent activity</p></div><div><span>Next</span><p>Test keyboard navigation</p></div></div>
        </div>
        <div className="sp-workflow-message is-agent sp-handoff-reply"><strong>Codex</strong>I’ll start with the keyboard navigation tests.</div>
      </div>
      <div className="sp-workflow-composer"><span>Continue the task…</span><span>Codex <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" aria-hidden="true"><path d="m4 6 4 4 4-4" /></svg></span></div>
    </div>
    <figcaption>One task. A concise handoff. <span>Illustrative workflow</span></figcaption>
  </figure>;
}

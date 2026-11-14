import assert from 'node:assert/strict';
import { test } from 'node:test';

const presentation = await import(new URL('../lib/console-presentation.js',import.meta.url));

test('session sidebar searches titles and escapes session metadata', () => {
  const sessions = [
    { id: 'one" onclick="bad', title: '<Review auth>', agent: 'codex', state: 'idle', last_event_at: '2026-10-29T00:00:00Z' },
    { id: 'two', title: 'Other work', agent: 'claude-code', state: 'thinking' },
  ];
  const markup = presentation.renderSessionList(sessions, sessions[0].id, 'auth');
  assert.match(markup, /&lt;Review auth&gt;/);
  assert.match(markup, /aria-current="true"/);
  assert.match(markup, /data-action="load-chat"/);
  assert.doesNotMatch(markup, /Other work|<Review|id="one" onclick/);
  assert.match(presentation.renderSessionList(sessions, '', 'missing'), /No matching sessions/);
});

test('conversation view excludes tool and system output', () => {
  const markup = presentation.renderChatCards([
    { role: 'user', content: 'Review auth', created_at: '2026-10-29T00:00:00Z' },
    { role: 'assistant', content: 'I found a bug.', created_at: '2026-10-29T00:00:01Z' },
    { role: 'tool', content: 'Command output: secret', created_at: '2026-10-29T00:00:02Z' },
    { role: 'system', content: 'Task: metadata', created_at: '2026-10-29T00:00:03Z' },
  ]);
  assert.match(markup, /Review auth/);
  assert.match(markup, /I found a bug/);
  assert.doesNotMatch(markup, /Command output|Task: metadata/);
});

test('agent replies render Markdown while user messages retain literal text', () => {
  const markup = presentation.renderChatCards([
    { role: 'user', content: '**Keep this literal**', created_at: '2026-10-29' },
    { role: 'assistant', content: '# Result\n\n**Fixed** with `cargo test`.\n\n- First\n- Second\n\n```rust\nlet x = "<script>";\n```\n\n| Check | Result |\n| --- | --- |\n| Tests | Pass |', created_at: '2026-10-29' },
  ]);
  assert.match(markup, /<h1>Result<\/h1>/);
  assert.match(markup, /<strong>Fixed<\/strong>/);
  assert.match(markup, /<code>cargo test<\/code>/);
  assert.match(markup, /<ul>\s*<li>First<\/li>\s*<li>Second<\/li>\s*<\/ul>/);
  assert.match(markup, /<pre tabindex="0"><code><span class="sp-hl-keyword">let<\/span> x = <span class="sp-hl-string">&quot;&lt;script&gt;&quot;<\/span>;/);
  assert.match(markup, /<th>Check<\/th>/);
  assert.match(markup, /<td data-label="Result">Pass<\/td>/);
  assert.match(markup, /\*\*Keep this literal\*\*/);
  assert.doesNotMatch(markup, /<script>/);
});

test('CommonMark emphasis, escaped punctuation and quoted Markdown remain correct', () => {
  const html=presentation.renderChatCards([{role:'assistant',content:'__strong__ and _emphasis_ with snake_case.\\*literal\\*\n\n> ## Quoted heading\n>\n> - Item\n\n- [x] Complete\n- [ ] Pending',created_at:'2026-11-02'}]);
  assert.match(html,/<strong>strong<\/strong>/);
  assert.match(html,/<em>emphasis<\/em>/);
  assert.match(html,/snake_case/);
  assert.match(html,/\*literal\*/);
  assert.match(html,/<blockquote>\s*<h2>Quoted heading<\/h2>/);
  assert.match(html,/<input[^>]+checked/);
});

test('reply formatting never enables HTML or executable links', () => {
  const markup = presentation.renderChatCards([{ role: 'assistant', content: '<img src=x onerror=alert(1)>\n\n[bad](javascript:alert) [safe](https://example.com)\n\n```html\n**literal**\n```', created_at: '2026-10-29' }]);
  assert.doesNotMatch(markup, /<img|href="javascript:/);
  assert.match(markup, /href="https:\/\/example.com"/);
  assert.match(markup, /<code[^>]*>\*\*literal\*\*/);
});

test('nested steps and multiline paragraphs retain their structure', () => {
  const markup = presentation.renderChatCards([{role:'assistant', content:'First line\nsecond line\n\n1. Run checks\n   - Unit tests\n   - Browser tests\n2. Review changes', created_at:'2026-10-29'}]);
  assert.match(markup, /<p>First line<br>\s*second line<\/p>/);
  assert.match(markup, /<ol>\s*<li>Run checks\s*<ul>\s*<li>Unit tests<\/li>\s*<li>Browser tests<\/li>\s*<\/ul>\s*<\/li>\s*<li>Review changes<\/li>\s*<\/ol>/);
});

test('continued numbered steps keep the agent numbering', () => {
  const markup = presentation.renderChatCards([{role:'assistant',content:'4. Build\n5. Verify',created_at:'2026-10-29'}]);
  assert.match(markup, /<ol start="4">\s*<li>Build<\/li>\s*<li>Verify<\/li>\s*<\/ol>/);
});

test('confirmed model changes appear as notices instead of chat bubbles', () => {
  const markup = presentation.renderChatCards([
    {role:'system',status:'model_changed',content:'Model changed from gpt-6.1-sol to o3',created_at:'2026-10-29'},
    {role:'system',content:'Internal metadata',created_at:'2026-10-29'},
  ]);
  assert.match(markup, /sp-chat-notice/);
  assert.match(markup, /Model changed from gpt-6.1-sol to o3/);
  assert.doesNotMatch(markup, /sp-chat-card|Internal metadata/);
});

test('activity identifies the selected total and the plotted date window', () => {
  const markup = presentation.renderActivity({range_label:'All time',total_tokens:2800000000,daily:[{date:'2026-08-24',total_tokens:118000000,chart_pct:20,turns:1},{date:'2026-10-29',total_tokens:128000000,chart_pct:30,turns:1}]});
  assert.doesNotMatch(markup, /Daily activity/);
  assert.match(markup,/All time/);
  assert.match(markup,/sp-activity-caption/);
  assert.match(markup,/Aug 24, 2026/);
  assert.match(markup,/Oct 29, 2026/);
});

test('wide Markdown tables keep escaped column labels for phone layout', () => {
  const html = presentation.renderChatCards([{event_id:'wide',turn_index:1,role:'assistant',content:'| Name | Value | Unit | <img> |\n| --- | --- | --- | --- |\n| Test | 12 | ms | safe |',created_at:'2026-10-29T00:00:00Z'}]);
  assert.match(html, /data-wide="true"/);
  assert.match(html, /data-label="&lt;img&gt;"/);
  assert.doesNotMatch(html, /<img>/);
});

test('pending prompts survive empty history and reconcile without duplicate bubbles', () => {
  const pending = [{id:'local-1',text:'Hello',created_at:'2026-10-29T00:00:01Z',baseline:['old'],delivered:false}];
  const initial = presentation.reconcileChat([],pending,'idle');
  assert.equal(initial.events[0].content,'Hello');
  assert.equal(initial.events.at(-1).content,'Sending...');
  pending[0].delivered = true;
  const recorded = [{event_id:'user',role:'user',content:'Hello',created_at:'2026-10-29T00:00:02Z'}];
  const waiting = presentation.reconcileChat(recorded,pending,'thinking');
  assert.equal(waiting.events.filter(event=>event.role==='user').length,1);
  assert.equal(waiting.events.at(-1).content,'Thinking...');
  const replied = presentation.reconcileChat([...recorded,{event_id:'reply',role:'assistant',content:'Hi',created_at:'2026-10-29T00:00:03Z'}],pending,'idle');
  assert.equal(replied.pending.length,0);
  assert.equal(replied.events.length,2);
});

test('an unrecorded old prompt stays in chronological order without an endless response spinner', () => {
  const pending=[{id:'missing',text:'heyo',created_at:'2026-11-01T18:38:39Z',baseline:[],delivered:true}];
  const events=[{event_id:'hey',role:'user',content:'hey',created_at:'2026-11-01T18:39:19Z'},{event_id:'reply',role:'assistant',content:'Hello',created_at:'2026-11-01T18:39:20Z'}];
  const result=presentation.reconcileChat(events,pending,'idle');
  assert.deepEqual(result.events.filter(e=>e.role==='user').map(e=>e.content),['heyo','hey']);
  assert.equal(result.events.some(e=>e.waiting),false);
  assert.match(presentation.renderChatCards(result.events),/Not confirmed by agent/);
  assert.equal(result.pending.length,1,'keep the unconfirmed text until its native turn appears');
});

test('identical pending prompts consume distinct logged turns and ended sessions stop waiting', () => {
 const one={id:'one',text:'again',created_at:'2026-10-29',baseline:[],delivered:true};
 const two={...one,id:'two'};
 const events=[{event_id:'u1',role:'user',content:'again',created_at:'2026-10-29'},{event_id:'a1',role:'assistant',content:'done',created_at:'2026-10-29'}];
 const result=presentation.reconcileChat(events,[one,two],'idle');
 assert.equal(result.pending.length,1);assert.equal(result.pending[0].id,'two');
 assert.match(presentation.reconcileChat([], [one], 'ended').events.at(-1).content,/ended/);
});
test('identical pending prompts keep ownership across repeated polls and empty history', () => {
 const pending=['one','two'].map(id=>({id,text:'again',created_at:'2026-10-29',baseline:[],delivered:true}));
 assert.equal(presentation.reconcileChat([],pending).events.filter(e=>e.role==='user').length,2);
 const events=[{event_id:'u1',role:'user',content:'again'},{event_id:'a1',role:'assistant',content:'done'}];
 const first=presentation.reconcileChat(events,pending);
 const second=presentation.reconcileChat(events,first.pending);
 assert.equal(second.pending.length,1);
 assert.equal(second.pending[0].id,'two');
 assert.equal(second.pending[0],pending[1]);
 pending[1].delivered=false;
 assert.equal(presentation.reconcileChat([],pending).pending[1],pending[1]);
 assert.equal(second.events.filter(e=>e.role==='user').length,2);
});
test('calendar week labels use ISO week-year at the year boundary', () => {
 const html=presentation.renderActivity({range_label:'All time',activity_interval:'week',total_tokens:10,daily:[{date:'2025-12-29',total_tokens:10,turns:1,chart_pct:100}]});
 assert.match(html,/2026 · Week 1/);assert.match(html,/weekly totals/);
});

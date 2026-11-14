import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const { renderHomeSessions, renderSessionList, renderOngoing, renderActivity, renderProviders, renderQuota, renderSessions, renderProjects, renderSummary, updateSummary } = await import(new URL('../lib/console-presentation.js',import.meta.url));
const {renderConversationLimits}=await import(new URL('../lib/console-presentation.js',import.meta.url));

test("shared console escaping, unknown data, trust, reset and freshness states", async () => {
  assert.match(renderConversationLimits([{label:'5h',remaining_pct:75,basis:'exact',observed_at:new Date().toISOString(),stale:false}]),/75%/);
  assert.match(renderConversationLimits([{label:'Weekly',remaining_pct:12,basis:'estimated',stale:true}]),/Estimated/);
  assert.match(renderConversationLimits([{label:'Weekly',remaining_pct:12,basis:'estimated',stale:true}]),/Last seen/);
  assert.match(renderConversationLimits([]),/Limits unavailable/);
  assert.doesNotMatch(renderConversationLimits([{label:'<script>',remaining_pct:null,basis:'unknown'}]),/<script>|0%/);
  assert.match(renderSummary({ report: { cost_basis: 'unknown', total_cost_usd: 0 } }), /Unknown/);
  assert.doesNotMatch(renderSummary({ report: { cost_basis: 'unknown', total_cost_usd: 0 } }), /\$0\.00/);
  assert.match(renderSummary({activeCount:3}), /inferred from recent records/);
  assert.match(renderActivity(null), /unavailable/);
  assert.match(renderActivity({ daily: [] }), /No recorded usage/);
  const activity = renderActivity({ total_tokens: 100, daily: [{ date: '2026-10-28', total_tokens: 100, chart_pct: 100, cost_basis: 'unknown', cost_usd: 0, turns: 1 }] });
  assert.match(activity, /Recorded tokens/);
  assert.match(activity, /Unknown/);
  assert.doesNotMatch(activity, /\$0\.00/);
  const quota = renderQuota([{ provider: '<script>', limit_id: 'sample', window: 'weekly', used_pct: 75, remaining_pct: 25, resets_at: '2020-01-01', reset_passed: true, basis: 'exact' }], null);
  assert.match(quota, /sp-provider-card/);
  assert.match(quota, /Reset, not confirmed/);
  assert.doesNotMatch(quota, /<script>/);
  const fullQuota = renderQuota([{ provider: 'codex', limit_id: 'sample', window: 'weekly', used_pct: 75, remaining_pct: 25, resets_at: '2099-01-01', basis: 'exact', state: 'ok' }], new Date().toISOString());
  assert.match(fullQuota, /Last received/);
  assert.match(fullQuota, /25% left/);
  assert.match(fullQuota, /Resets /);
  assert.doesNotMatch(fullQuota, /<script>/);
  const sessions = renderSessions([{ id: 'sample" onmouseover="bad', project_name: '<img>', agent: 'codex', model: 'sample', state: 'idle', last_event_at: '2026-10-28', input_tokens: 100, output_tokens: 10, cost_basis: 'unknown', total_cost_usd: 0 }], { rotate: true });
  assert.doesNotMatch(sessions, /<img>|data-key="sample" onmouseover=/);
  assert.match(sessions, /Unknown/);
  assert.doesNotMatch(sessions, /\$0\.00/);
  const localSessions = renderSessions([{ id: 'codex:session-1', project_name: 'Demo', agent: 'codex', model: 'sample', state: 'idle', last_event_at: '2026-10-28' }], { focus: true });
  assert.match(localSessions, /data-action="load-chat"[^>]*data-arg="codex:session-1"/);
  assert.match(renderProjects(undefined), /remain local/);
});

test("partial pricing, independent provider coverage, local hourly labels and ongoing session identity", async () => {
  const providers=renderProviders({cost_basis:'unknown',providers:[{provider:'<script>',total_cost_usd:0,models:[{model:'<img onerror=x>',input_tokens:100,output_tokens:10,cost_usd:0}]}]});
  assert.doesNotMatch(providers,/<script>|<img|\$0\.00/);
  assert.match(providers,/title="Price unavailable">Unknown/);

  let writes=0, values=[];
  globalThis.document={activeElement:null};
  const element={ contains:()=>false, querySelector:()=>null, querySelectorAll:selector=>selector==='strong'?values:[], set innerHTML(markup){writes++;values=[...markup.matchAll(/<strong(?:\s[^>]*)?>([\s\S]*?)<\/strong>/g)].map(match=>({textContent:match[1],changed:false,classList:{add(){this.owner.changed=true;}}}));values.forEach(value=>value.classList.owner=value);} };
  updateSummary(element,{activeCount:1});
  assert.equal(writes,1);assert.equal(values.some(value=>value.changed),false);
  updateSummary(element,{activeCount:1});assert.equal(writes,1,'unchanged polling must not rewrite markup');
  updateSummary(element,{activeCount:2});assert.equal(writes,2);assert.equal(values.filter(value=>value.changed).length,1,'only changed values animate');

  const webConsole = await readFile(new URL('../app/console/Client.tsx', import.meta.url), 'utf8');
  assert.doesNotMatch(webConsole, /gnomcode_cloud_sessions_live/, 'retired hosted sessions must not be queried');
  assert.match(webConsole, /spawnt_desktops/, 'desktop summaries use per-desktop storage');

  const partial={cost_basis:'unknown',priced_turns:2,unpriced_turns:1,total_cost_usd:12,total_cache_savings_usd:4,range_label:'This month'};
  assert.match(renderSummary({report:partial}),/\$12.00/);
  assert.match(renderSummary({report:partial}),/Partial estimate/);
  const perProvider=renderProviders({cost_basis:'unknown',providers:[{provider:'Claude',priced_turns:1,unpriced_turns:0,total_cost_usd:12,total_cache_savings_usd:4,models:[]}]});
  assert.match(perProvider,/\$12.00/);assert.doesNotMatch(perProvider,/Unknown/);
  const hourly=renderActivity({...partial,range_label:'Today',total_tokens:100,daily:[{date:'2026-10-28T00:00:00Z',total_tokens:100,chart_pct:100,priced_turns:1,cost_usd:12,turns:1}]});
  assert.match(hourly,/Today, by hour/);assert.match(hourly,/Since local midnight/);assert.doesNotMatch(hourly,/>10-28</);
  assert.doesNotMatch(hourly.split('class="sp-heatmap sp-heatmap--hours"')[0], /\$12\.00|Partial estimate/, 'activity header must not duplicate the spend summary');
  assert.match(hourly, /\$12\.00/, 'expanded interval details retain their own cost');
  const inferredSession=renderOngoing([{id:'unknown',title:'unknown',project_name:'unknown',agent:'codex',is_ongoing:true,state:'thinking',last_event_at:new Date().toISOString()}]);
  assert.match(inferredSession, /Inferred from recorded activity/);
  assert.match(inferredSession, /<b>Running<\/b>/);
  assert.match(inferredSession, /Private session/);
  assert.match(renderOngoing([{id:'active-1',title:'Current session',agent:'codex',is_active:true,state:'thinking',last_event_at:'2026-10-28T00:00:00Z'},{id:'idle-1',title:'Old session',is_active:false}]),/Current session/);
  assert.doesNotMatch(renderOngoing([{id:'idle-1',title:'Old session',is_active:false}]),/Old session/);
});

test("project rows, shared styles and ongoing sessions keep their contracts", async () => {
  const projectPartial=renderProjects([{id:'project"safe',name:'Project name',total_tokens:100,sessions:2,cost_basis:'unknown',priced_turns:1,unpriced_turns:1,cost_usd:12}]);
  assert.match(projectPartial,/\$12.00/);assert.match(projectPartial,/Partial estimate/);assert.match(projectPartial,/data-project-id="project&quot;safe"/);

  const sharedCss=await readFile(new URL('../app/gnomcode-console.css',import.meta.url),'utf8');
  assert.doesNotMatch(sharedCss,/var\(--(?:text(?:-\d)?|border|primary-text|surface|bg)\)/,'shared console styles must use shared tokens, not marketing-only aliases');

  const recentIdle=renderOngoing([{id:'idle-grace',title:'Recent idle',is_active:false,is_ongoing:true,presence:'recently_idle',state:'idle',last_event_at:new Date().toISOString()}]);
  assert.match(recentIdle,/Recent idle/);assert.match(recentIdle,/<b>Idle<\/b>/);assert.match(recentIdle,/Last activity/);
  const ongoingWithTokens = renderOngoing([{
    id: 'subagent-1',
    title: 'Goodall (control) · Subagent',
    agent: 'codex',
    parent_session_id: 'parent-1',
    is_active: true,
    state: 'thinking',
    total_tokens: 23177,
    last_event_at: '2026-10-28T00:00:00Z',
  }]);
  assert.match(ongoingWithTokens, /Goodall \(control\) · Subagent/);
  assert.match(ongoingWithTokens, /23\.2K tokens/);
  assert.match(ongoingWithTokens, /Subagent/);
  assert.match(ongoingWithTokens, /23\.2K recorded/);
  assert.match(ongoingWithTokens, /parent-1/);
  const windows=renderQuota([{provider:'codex',window:'five_hour',used_pct:50,remaining_pct:50,basis:'exact'},{provider:'codex',window:'weekly',used_pct:8,remaining_pct:92,basis:'exact'}],new Date().toISOString());
  assert.equal((windows.match(/<details /g)||[]).length,1,'one card groups both provider windows');
  assert.match(windows,/50% left/);assert.match(windows,/92% left/);
  const stale=renderQuota([{provider:'claude-code',limit_id:'claude',window:'weekly',used_pct:2,remaining_pct:98,resets_at:'2099-01-01T00:00:00Z',basis:'exact',observed_at:new Date(Date.now()-9*3600000).toISOString(),source_issue:'Claude sign-in expired; run `claude` in a terminal to refresh it'}],new Date().toISOString());
  assert.match(stale,/Claude sign-in expired/,'a stale live source is named instead of showing a current-looking reset time');
  assert.match(stale,/Seen 9h ago/i);

});

test('session rows name the product surface from Rust and fall back to the agent', () => {
  const row = { id: 'a', agent: 'antigravity', title: 'T', project_name: 'p', model: 'm', state: 'idle', home_group: 'recent', last_event_at: '2026-10-28T00:00:00Z' };
  assert.match(renderHomeSessions([{ ...row, surface_label: 'Antigravity IDE' }]), /p · Antigravity IDE · m/);
  assert.match(renderHomeSessions([row]), /p · Antigravity · m/);
  assert.match(renderSessionList([{ ...row, agent: 'claude-code', surface_label: 'Claude desktop app' }]), /<span>Claude desktop app · /);
  assert.match(renderSessionList([{ ...row, agent: 'claude-code' }]), /<span>claude-code · /);
  assert.doesNotMatch(renderSessionList([{ ...row, surface_label: '<img src=x>' }]), /<img/);
});

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
const { renderActivity, renderProviders, renderQuota, renderSessions, renderProjects, renderSummary, updateSummary } = await import(new URL('../web/lib/console-presentation.js',import.meta.url));
const {renderConversationLimits}=await import(new URL('../web/lib/console-presentation.js',import.meta.url));
assert.match(renderConversationLimits([{label:'5h',remaining_pct:75,basis:'exact',observed_at:new Date().toISOString(),stale:false}]),/75%/);
assert.match(renderConversationLimits([{label:'Weekly',remaining_pct:12,basis:'estimated',stale:true}]),/EST/);
assert.match(renderConversationLimits([{label:'Weekly',remaining_pct:12,basis:'estimated',stale:true}]),/Last seen/);
assert.match(renderConversationLimits([]),/Limits unavailable/);
assert.doesNotMatch(renderConversationLimits([{label:'<script>',remaining_pct:null,basis:'unknown'}]),/<script>|0%/);
assert.match(renderSummary({ report: { cost_basis: 'unknown', total_cost_usd: 0 } }), /Unknown/);
assert.doesNotMatch(renderSummary({ report: { cost_basis: 'unknown', total_cost_usd: 0 } }), /\$0\.00/);
assert.match(renderActivity(null), /unavailable/);
assert.match(renderActivity({ daily: [] }), /No recorded usage/);
const activity = renderActivity({ total_tokens: 100, daily: [{ date: '2026-10-28', total_tokens: 100, chart_pct: 100, cost_basis: 'unknown', cost_usd: 0, turns: 1 }] });
assert.match(activity, /Recorded tokens/);
assert.match(activity, /Unknown/);
assert.doesNotMatch(activity, /\$0\.00/);
const quota = renderQuota([{ provider: '<script>', limit_id: 'sample', window: 'weekly', used_pct: 75, remaining_pct: 25, resets_at: '2020-01-01', basis: 'exact' }], null);
assert.match(quota, /STALE/);
assert.match(quota, /awaiting provider update/);
assert.match(quota, /25\.0% remaining/);
assert.doesNotMatch(quota, /<script>/);
const sessions = renderSessions([{ id: 'sample" onmouseover="bad', project_name: '<img>', agent: 'codex', model: 'sample', state: 'idle', last_event_at: '2026-10-28', input_tokens: 100, output_tokens: 10, cost_basis: 'unknown', total_cost_usd: 0 }], { rotate: true });
assert.doesNotMatch(sessions, /<img>|data-key="sample" onmouseover=/);
assert.match(sessions, /Unknown/);
assert.doesNotMatch(sessions, /\$0\.00/);
const localSessions = renderSessions([{ id: 'codex:session-1', project_name: 'Demo', agent: 'codex', model: 'sample', state: 'idle', last_event_at: '2026-10-28' }], { focus: true });
assert.match(localSessions, /data-action="load-chat"[^>]*data-arg="codex:session-1"/);
const localConsole = await readFile(new URL('../crates/daemon/src/console.html', import.meta.url), 'utf8');
assert.match(localConsole, /id="chat-stream"/);
assert.match(renderProjects(undefined), /remain local/);
console.log('PASS: shared console escaping, unknown data, trust, reset and freshness states');

const providers=renderProviders({cost_basis:'unknown',providers:[{provider:'<script>',total_cost_usd:0,models:[{model:'<img onerror=x>',input_tokens:100,output_tokens:10,cost_usd:0}]}]});
assert.doesNotMatch(providers,/<script>|<img|\$0\.00/);
assert.match(providers,/Unknown · Price unavailable/);

let writes=0, values=[];
globalThis.document={activeElement:null};
const element={ contains:()=>false, querySelector:()=>null, querySelectorAll:selector=>selector==='strong'?values:[], set innerHTML(markup){writes++;values=[...markup.matchAll(/<strong(?:\s[^>]*)?>([\s\S]*?)<\/strong>/g)].map(match=>({textContent:match[1],changed:false,classList:{add(){this.owner.changed=true;}}}));values.forEach(value=>value.classList.owner=value);} };
updateSummary(element,{activeCount:1});
assert.equal(writes,1);assert.equal(values.some(value=>value.changed),false);
updateSummary(element,{activeCount:1});assert.equal(writes,1,'unchanged polling must not rewrite markup');
updateSummary(element,{activeCount:2});assert.equal(writes,2);assert.equal(values.filter(value=>value.changed).length,1,'only changed values animate');

const webConsole = await readFile(new URL('../web/app/console/Client.tsx', import.meta.url), 'utf8');
assert.doesNotMatch(webConsole, /spawnt_cloud_sessions_live/, 'retired hosted sessions must not be queried');
assert.match(webConsole, /spawnt_sync_state/, 'desktop summary sync remains available');

const partial={cost_basis:'unknown',priced_turns:2,unpriced_turns:1,total_cost_usd:12,total_cache_savings_usd:4,range_label:'This month'};
assert.match(renderSummary({report:partial}),/\$12.00/);
assert.match(renderSummary({report:partial}),/Partial estimate/);
const perProvider=renderProviders({cost_basis:'unknown',providers:[{provider:'Claude',priced_turns:1,unpriced_turns:0,total_cost_usd:12,total_cache_savings_usd:4,models:[]}]});
assert.match(perProvider,/\$12.00/);assert.doesNotMatch(perProvider,/Unknown/);
const hourly=renderActivity({...partial,range_label:'Today',total_tokens:100,daily:[{date:'2026-10-28T00:00:00Z',total_tokens:100,chart_pct:100,priced_turns:1,cost_usd:12,turns:1}]});
assert.match(hourly,/Today, by hour/);assert.match(hourly,/Since local midnight/);assert.doesNotMatch(hourly,/>10-28</);
assert.doesNotMatch(hourly.split('class="sp-activity-bars"')[0], /\$12\.00|Partial estimate/, 'activity header must not duplicate the spend summary');
assert.match(hourly, /\$12\.00/, 'expanded interval details retain their own cost');
const {renderOngoing}=await import(new URL('../web/lib/console-presentation.js',import.meta.url));
assert.match(renderOngoing([{id:'active-1',title:'Current session',agent:'codex',is_active:true,state:'thinking',last_event_at:'2026-10-28T00:00:00Z'},{id:'idle-1',title:'Old session',is_active:false}]),/Current session/);
assert.doesNotMatch(renderOngoing([{id:'idle-1',title:'Old session',is_active:false}]),/Old session/);
console.log('PASS: partial pricing, independent provider coverage, local hourly labels and ongoing session identity');

const projectPartial=renderProjects([{id:'project"safe',name:'Project name',total_tokens:100,sessions:2,cost_basis:'unknown',priced_turns:1,unpriced_turns:1,cost_usd:12}]);
assert.match(projectPartial,/\$12.00/);assert.match(projectPartial,/Partial estimate/);assert.match(projectPartial,/data-project-id="project&quot;safe"/);

const sharedCss=await readFile(new URL('../web/app/spawnt-console.css',import.meta.url),'utf8');
assert.doesNotMatch(sharedCss,/var\(--(?:text(?:-\d)?|border|primary-text|surface|bg)\)/,'shared console styles must use shared tokens, not marketing-only aliases');

const recentIdle=renderOngoing([{id:'idle-grace',title:'Recent idle',is_active:false,is_ongoing:true,presence:'recently_idle',state:'idle',last_event_at:new Date().toISOString()}]);
assert.match(recentIdle,/Recent idle/);assert.match(recentIdle,/<b>Idle<\/b>/);assert.match(recentIdle,/Last activity/);
const windows=renderQuota([{provider:'codex',window:'five_hour',used_pct:50,remaining_pct:50,basis:'exact'},{provider:'codex',window:'weekly',used_pct:8,remaining_pct:92,basis:'exact'}],new Date().toISOString());
assert.equal((windows.match(/class="sp-limit-windows"/g)||[]).length,1,'one compact row groups both provider windows');
assert.match(windows,/50.0%/);assert.match(windows,/8.0%/);

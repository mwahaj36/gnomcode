import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
const source = await readFile(new URL('../web/lib/console-presentation.js', import.meta.url));
const { renderActivity, renderProviders, renderQuota, renderSessions, renderProjects, renderSummary, updateSummary } = await import(`data:text/javascript;base64,${source.toString('base64')}`);
assert.match(renderSummary({ report: { cost_basis: 'unknown', total_cost_usd: 0 } }), /Unknown/);
assert.doesNotMatch(renderSummary({ report: { cost_basis: 'unknown', total_cost_usd: 0 } }), /\$0\.00/);
assert.match(renderActivity(null), /unavailable/);
assert.match(renderActivity({ daily: [] }), /No recorded usage/);
const activity = renderActivity({ total_tokens: 100, daily: [{ date: '2026-10-28', total_tokens: 100, chart_pct: 100, cost_basis: 'unknown', cost_usd: 0, turns: 1 }] });
assert.match(activity, /EXACT tokens/);
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
assert.match(renderProjects(undefined), /remain local/);
console.log('PASS: shared console escaping, unknown data, trust, reset and freshness states');

const providers=renderProviders({cost_basis:'unknown',providers:[{provider:'<script>',total_cost_usd:0,models:[{model:'<img onerror=x>',input_tokens:100,output_tokens:10,cost_usd:0}]}]});
assert.doesNotMatch(providers,/<script>|<img|\$0\.00/);
assert.match(providers,/Unknown · UNKNOWN/);

let writes=0, values=[];
globalThis.document={activeElement:null};
const element={ contains:()=>false, querySelector:()=>null, querySelectorAll:selector=>selector==='strong'?values:[], set innerHTML(markup){writes++;values=[...markup.matchAll(/<strong>([\s\S]*?)<\/strong>/g)].map(match=>({textContent:match[1],changed:false,classList:{add(){this.owner.changed=true;}}}));values.forEach(value=>value.classList.owner=value);} };
updateSummary(element,{activeCount:1});
assert.equal(writes,1);assert.equal(values.some(value=>value.changed),false);
updateSummary(element,{activeCount:1});assert.equal(writes,1,'unchanged polling must not rewrite markup');
updateSummary(element,{activeCount:2});assert.equal(writes,2);assert.equal(values.filter(value=>value.changed).length,1,'only changed values animate');

const webConsole = await readFile(new URL('../web/app/console/page.tsx', import.meta.url), 'utf8');
assert.doesNotMatch(webConsole, /fleethud_cloud_sessions_live/, 'retired hosted sessions must not be queried');
assert.match(webConsole, /fleethud_sync_state/, 'desktop summary sync remains available');

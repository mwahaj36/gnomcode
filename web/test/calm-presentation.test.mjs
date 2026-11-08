import test from 'node:test';
import assert from 'node:assert/strict';
import {renderSummary,renderOngoing,renderConversationLimits,renderActivity} from '../lib/console-presentation.js';

test('summary keeps provenance available in one disclosure instead of repeated badges',()=>{
 const html=renderSummary({activeCount:2,quotaCount:1,report:{priced_turns:1,total_cost_usd:12,total_cache_savings_usd:4}});
 assert.match(html,/<details class="sp-data-details"/);
 assert.match(html,/Estimated spend/);
 assert.doesNotMatch(html,/>EXACT|>EST ·/);
});
test('observed activity uses short status with its inference explained in details',()=>{
 const html=renderOngoing([{id:'one',title:'Long project title',is_ongoing:true,state:'thinking',agent:'codex'}]);
 assert.doesNotMatch(html,/Possibly running/);
 assert.match(html,/Status source/);
});
test('conversation quotas keep provenance in their accessible detail, without exact badges',()=>{
 const html=renderConversationLimits([{label:'Weekly',remaining_pct:50,basis:'exact'}]);
 assert.match(html,/50% left/);
 assert.match(html,/Provider measured/);
 assert.doesNotMatch(html,/<small>EXACT/);
});
test('activity breakdown is optional while the chart is available immediately',()=>{
 const html=renderActivity({total_tokens:100,daily:[{date:'2026-11-07',total_tokens:100,chart_pct:100}]});
 assert.match(html,/<details class="sp-activity-breakdown"><summary>Activity details/);
});

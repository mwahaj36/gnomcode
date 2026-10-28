import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const html=await readFile(new URL('../crates/daemon/src/console.html',import.meta.url),'utf8');
for(const match of html.matchAll(/<script(?:\s[^>]*)?>([\s\S]*?)<\/script>/g)) new Function(match[1]);
const escapeSource=html.match(/function escapeHtml\(value\) \{([^\n]+)\}/)[1];
const escapeHtml=new Function('value',escapeSource);
const argumentSource=html.match(/function jsArg\(value\) \{([^\n]+)\}/)[1];
const jsArg=new Function('escapeHtml','value',argumentSource).bind(null,escapeHtml);
const attack='" onclick="alert(1)"\'><script>';
assert.doesNotMatch(escapeHtml(attack),/[<>"']/);
const decoded=jsArg(attack).replace(/&quot;/g,'"').replace(/&#39;/g,"'").replace(/&lt;/g,'<').replace(/&gt;/g,'>').replace(/&amp;/g,'&');
assert.equal(JSON.parse(decoded),attack);
assert.doesNotMatch(html,/onclick="(?:executeRelay|dismissRelay|deleteRule|disarmResume|deleteCustomPricing)\('\$\{/);
console.log('PASS: local inline script syntax, metadata escaping and action argument roundtrip');

for(const id of ['overview-summary','providers-container','auth-cloud-status','connection-status','cloud-sync-status']) assert.ok(html.includes('id=' + String.fromCharCode(34) + id + String.fromCharCode(34)), 'required runtime status target: '+id);

const markup=html.replace(/<script[^>]*>[\s\S]*?<\/script>/g,'');
const declaredIds=new Set([...markup.matchAll(/\bid="([^"]+)"/g)].map(match=>match[1]));
for(const match of html.matchAll(/getElementById\('([^']+)'\)\./g)) assert.ok(declaredIds.has(match[1]),'native runtime target: '+match[1]);

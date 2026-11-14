import assert from 'node:assert/strict';
import test from 'node:test';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import vm from 'node:vm';
const require=createRequire(new URL('../package.json',import.meta.url));
const ts=require('typescript');const React=require('react');const {renderToStaticMarkup}=require('react-dom/server');
const presenters=await import(new URL('../lib/console-presentation.js',import.meta.url));
const code=ts.transpileModule(readFileSync(new URL('../app/components/ConsoleTourView.tsx',import.meta.url),'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX}}).outputText;
const exports={};vm.runInNewContext(code,{exports,require(name){return name.includes('console-presentation')?presenters:require(name);},Intl});
const render=kind=>renderToStaticMarkup(React.createElement(exports.ConsoleTourView,{kind}));
test('the Overview tour is the console Home: limits, then activity and sessions beside today', () => {
  const html=render('overview');
  assert.match(html,/sp-home-grid/);
  assert.equal((html.match(/class="sp-card sp-provider-card"/g)||[]).length,3);
  assert.equal((html.match(/sp-provider-mark--logo/g)||[]).length,3,'every provider uses the real logo, never a letter');
  assert.match(html,/sp-heatmap-months|sp-heat-month|>Nov</);
  assert.match(html,/Tokens today/);
  assert.match(html,/Needs you/);
});
test('the Sessions tour shows status groups, subagents and a WSL badge', () => {
  const html=render('sessions');
  assert.match(html,/Needs you/);assert.match(html,/Running/);
  assert.match(html,/<details open class="sp-subagents"/);
  assert.match(html,/WSL: Ubuntu/);
});
test('the Limits tour expands one card and shows Low and Out states with trust levels', () => {
  const html=render('limits');
  assert.equal((html.match(/<details open class="sp-card sp-provider-card"/g)||[]).length,1);
  assert.match(html,/sp-chip--low/);assert.match(html,/1 pool out/);
  assert.match(html,/Provider measured/);assert.match(html,/Status line/);
});
test('the Usage tour uses the 53-week brand-red heatmap', () => {
  const html=render('usage');
  assert.match(html,/data-level="4"/);assert.match(html,/Est\. spend/);
});
test('the landing tour does not repeat the tab title inside the illustrative screen', () => {
  const landing=readFileSync(new URL('../app/LandingClient.tsx',import.meta.url),'utf8');
  assert.doesNotMatch(landing,/<h4>\{tours\[tab\]\.title\}<\/h4>/);
  assert.doesNotMatch(landing,/\/agents\/\$\{/,'agent row uses the console provider logos');
  assert.doesNotMatch(landing,/approve Claude Code permission prompts from your phone/);
});

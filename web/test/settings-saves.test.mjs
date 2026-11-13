import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import vm from 'node:vm';
const require = createRequire(new URL('../package.json', import.meta.url));
const ts = require('typescript');
const source = readFileSync(new URL('../components/SettingsPanel.tsx', import.meta.url), 'utf8');
const code = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }).outputText;
async function scenario(error, signedIn = true) {
  let index = 0, message = '', writes = 0;
  const states = [{ id: 'test-user', email: 'test@example.invalid' }];
  const fakeReact = { useState(initial) { const i=index++; return [i===0 ? signedIn ? states[0] : null : initial, value => { if (i===3) message=value; }]; }, useEffect() {}, useCallback: f=>f, useRef: value=>({ current:value }) };
  const exports = {};
  const sandbox = { exports, require(name) {
    if (name==='react') return fakeReact;
    if (name==='react/jsx-runtime') return { jsx:(type, props)=>({type,props}), jsxs:(type,props)=>({type,props}) };
    if (name==='next/link') return { default:'a' };
    if (name.includes('notifications')) return {};
    if (name.includes('AppLockGate')) return { AppLockSettings: 'div' };
    if (name.includes('supabase')) return { supabase:{ from:()=>({ upsert:async()=>{ writes++; return {error}; } }) } };
    throw new Error(name);
  }, Date, Number, Error };
  vm.runInNewContext(code,sandbox);
  const tree=exports.default();
  function find(node) { if (!node || typeof node!=='object') return; if (node.type==='form' && node.props.className==='sp-settings-plans-form') return node; for(const child of [node.props?.children].flat(Infinity)) { const result=find(child); if(result) return result; } }
  await find(tree).props.onSubmit({preventDefault(){}});
  return {message,writes};
}
test('rejected cloud settings never report success; signed-out writes are blocked; no local transport', async () => {
  assert.doesNotMatch(source,/127\.0\.0\.1/,'cloud settings must not probe or write local data');
  const failed=await scenario({message:'Write rejected'});
  assert.match(failed.message,/Save failed.*Write rejected/);
  assert.doesNotMatch(failed.message,/saved/i);
  const success=await scenario(null);
  assert.match(success.message,/Account display name saved/);
  const signedOut=await scenario(null,false);
  assert.equal(signedOut.writes,0);
  assert.match(signedOut.message,/Sign in/);
});

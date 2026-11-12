import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {runInNewContext} from 'node:vm';

const html=readFileSync('crates/daemon/src/console.html','utf8');
const source=readFileSync('crates/daemon/src/console-app.js','utf8');
const start=source.indexOf('// First-run guide');
assert.ok(start>=0,'console includes first-run guide');
const guide=source.slice(start,source.indexOf('// End first-run guide',start));

// Enforce zero emojis and strict copy limits in tour HTML + JS.
const dialogStart=html.indexOf('<dialog id="onboarding-dialog"');
const dialogEnd=html.indexOf('</dialog>',dialogStart)+'</dialog>'.length;
const tipStart=html.indexOf('<div id="tour-tip"');
assert.ok(dialogStart>=0 && tipStart>=0,'console.html contains #onboarding-dialog and #tour-tip');
const tourMarkup=html.slice(tipStart,dialogEnd);

assert.equal(/[\u{1F300}-\u{1FAFF}\u{2600}-\u{27BF}]/u.test(tourMarkup+guide),false,'no emojis in tour markup or JS');
assert.equal(/https?:\/\/|sendBeacon|analytics|gtag|posthog/i.test(guide),false,'no analytics or external requests in guide JS');

const stripTags=s=>s.replace(/<[^>]+>/g,' ').replace(/\s+/g,' ').trim();
const countWords=s=>s.trim()?s.trim().split(/\s+/).length:0;

for(const match of tourMarkup.matchAll(/<(strong|button|h2|dt)\b[^>]*>([\s\S]*?)<\/\1>/g)){
  const label=stripTags(match[2]);
  assert.ok(countWords(label)<=6,`label exceeds 6 words (${countWords(label)}): "${label}"`);
}
for(const match of tourMarkup.matchAll(/<(p|dd|li)\b[^>]*>([\s\S]*?)<\/\1>/g)){
  const text=stripTags(match[2]);
  if(!text)continue;
  for(const sentence of text.split(/[.!?]+/).map(part=>part.trim()).filter(Boolean)){
    assert.ok(countWords(sentence)<=12,`sentence exceeds 12 words (${countWords(sentence)}): "${sentence}"`);
  }
}

const steps=[...tourMarkup.matchAll(/<section data-onboarding-step\b[^>]*>([\s\S]*?)<\/section>/g)].map(m=>stripTags(m[1]));
assert.equal(steps.length,4,'tour has four steps');
const allTourText=steps.join(' ');
for(const required of ['Home','Workspace','Usage','Settings','New','Claude','Antigravity','Providers','tray','Remote']){
  assert.ok(allTourText.includes(required),`tour covers "${required}"`);
}
assert.match(html,/id="settings-guide"[\s\S]*data-action="onboarding-start"/,'tour is replayable from Settings');

function makeHarness(){
  const nodes=new Map();
  const node=id=>nodes.get(id) || nodes.set(id,{id,hidden:id==='tour-tip',textContent:'',disabled:false,focus(){this.focused=true;},open:false,showModal(){this.open=true;},close(){this.open=false;},addEventListener(type,fn){this[type]=fn;}}).get(id);
  const pages=Array.from({length:4},(_,i)=>node(`page-${i}`));
  const saved=new Map();
  const state={launches:0,view:''};
  const context={
    document:{getElementById:node,querySelectorAll:()=>pages,activeElement:node('origin')},
    localStorage:{getItem:key=>saved.get(key)??null,setItem:(key,value)=>saved.set(key,value)},
    setView:value=>{state.view=value;},
    showLauncher:()=>{state.launches++;},
  };
  runInNewContext(guide,context);
  return {context,node,pages,saved,state};
}

// 1. Zero sessions opens the first-run modal automatically and keeps tip hidden.
{
  const {context,node,pages,saved,state}=makeHarness();
  context.offerOnboarding(0);
  assert.equal(node('onboarding-dialog').open,true);
  assert.equal(node('tour-tip').hidden,true);
  assert.equal(pages[0].hidden,false);
  assert.equal(node('onboarding-back').disabled,true);
  context.moveOnboarding(1);
  assert.equal(pages[0].hidden,true);
  assert.equal(pages[1].hidden,false);
  context.moveOnboarding(-1);
  assert.equal(pages[0].hidden,false);
  for(let i=0;i<4;i++)context.moveOnboarding(1);
  assert.equal(state.launches,1);
  assert.equal(state.view,'sessions');
  assert.equal(saved.get('gnomcode-onboarding-v1'),'dismissed');
  context.startOnboarding();
  node('onboarding-dialog').cancel({preventDefault(){}});
  assert.equal(node('onboarding-dialog').open,false);
  assert.equal(state.launches,1,'skip never launches an agent');
}

// 2. Existing sessions (> 0) show the dismissible tip instead of the modal.
{
  const {context,node,saved}=makeHarness();
  context.offerOnboarding(3);
  assert.equal(node('onboarding-dialog').open,false,'modal stays closed when sessions exist');
  assert.equal(node('tour-tip').hidden,false,'dismissible tip is shown when sessions exist');
  context.dismissTourTip();
  assert.equal(node('tour-tip').hidden,true,'tip hides when dismissed');
  assert.equal(saved.get('gnomcode-onboarding-v1'),'dismissed');
  // Re-offering after dismissal stays hidden.
  context.offerOnboarding(0);
  assert.equal(node('onboarding-dialog').open,false);
  // Explicit replay from Settings still opens the modal regardless of session count.
  context.startOnboarding();
  assert.equal(node('onboarding-dialog').open,true);
}

// 3. Blocked storage does not crash guide or tip dismissal.
{
  const {context}=makeHarness();
  context.localStorage.getItem=()=>{throw Error('storage blocked');};
  context.localStorage.setItem=()=>{throw Error('storage blocked');};
  assert.doesNotThrow(()=>{context.offerOnboarding(2);context.dismissTourTip();context.startOnboarding();context.finishOnboarding(false);});
}

console.log('PASS: tour refresh coverage, copy limits, zero-session gate, tip fallback, keyboard/skip, blocked storage');

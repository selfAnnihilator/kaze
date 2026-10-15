import test from 'node:test';
import assert from 'node:assert/strict';
import { registerHooks } from 'node:module';
import { readFileSync, existsSync } from 'node:fs';
import ts from 'typescript';

// Execute the actual Settings component and its handlers with controlled hook
// scheduling and IPC responses. No copied production handler or native writes.
const realReact = import.meta.resolve('react');
const hooksUrl = 'data:text/javascript,' + encodeURIComponent(`
import React from ${JSON.stringify(realReact)};
export default React;
export const useState = (...a) => globalThis.closeSettingsHarness.useState(...a);
export const useRef = (...a) => globalThis.closeSettingsHarness.useRef(...a);
export const useEffect = (...a) => globalThis.closeSettingsHarness.useEffect(...a);
`);
const apiUrl = 'data:text/javascript,' + encodeURIComponent(`
export const dispatchCommand = (...a) => globalThis.closeSettingsHarness.dispatch(...a);
export const executeQuery = async () => ({ total_size_bytes: 0 });
`);
registerHooks({
  resolve(specifier, context, next) {
    if (context.parentURL?.endsWith('/SettingsView.tsx')) {
      if (specifier === 'react') return {url: hooksUrl, shortCircuit: true};
      if (specifier === '../../services/api') return {url: apiUrl, shortCircuit: true};
    }
    if (specifier.startsWith('.')) {
      const base = new URL(specifier, context.parentURL);
      for (const suffix of ['.ts', '.tsx']) if (existsSync(new URL(base.href + suffix))) return next(base.href + suffix, context);
    }
    return next(specifier, context);
  },
  load(url, context, next) {
    if (url.endsWith('.tsx') || url.endsWith('.ts')) return {format:'module', source:ts.transpileModule(readFileSync(new URL(url),'utf8'), {compilerOptions:{module:ts.ModuleKind.ESNext, jsx:ts.JsxEmit.ReactJSX, target:ts.ScriptTarget.ES2022}}).outputText, shortCircuit:true};
    return next(url, context);
  },
});
const {SettingsView} = await import('../src/components/views/SettingsView.tsx');
const text = node => Array.isArray(node) ? node.map(text).join('') : typeof node === 'string' ? node : node?.props ? text(node.props.children) : '';
function find(node, predicate) {
  if (Array.isArray(node)) {for (const child of node) {const found=find(child,predicate); if(found) return found;}}
  else if (node?.props) {if(predicate(node)) return node; return find(node.props.children,predicate);}
}
function setup(dispatch, reload=async()=>{}) {
  const values=[], effects=[]; let cursor=0;
  globalThis.closeSettingsHarness = {
    dispatch,
    useState(initial) {const i=cursor++; if(!(i in values)) values[i]=typeof initial==='function'?initial():initial; return [values[i], value=>{values[i]=typeof value==='function'?value(values[i]):value;}];},
    useRef(initial) {const i=cursor++; return values[i]??=( {current:initial} );},
    useEffect(effect,deps) {const i=cursor++; if(!values[i] || deps.some((v,j)=>!Object.is(v,values[i][j]))) {values[i]=deps; effects.push(effect);}},
  };
  const props={settings:{close_button_behavior:'minimize_to_tray'},configuredFolders:[],onAddFolder:async()=>{},onRemoveFolder:async()=>{},onRescanLibrary:async()=>{},onRerunOnboarding(){},onLaunchSoulseek:async()=>{},onImportSoulseek:async()=>{},onReloadSettings:reload};
  function render() {cursor=0; const tree=SettingsView(props); while(effects.length) effects.shift()(); return tree;}
  const button=(tree,label)=>find(tree,n=>n.type==='button'&&text(n).includes(label));
  button(render(),'System').props.onClick();
  return {render,button};
}

test('failed persistence leaves the previous close choice active and shows an error', async(t)=>{
  t.mock.method(console, 'error', () => {});
  const h=setup(async()=>{throw Error('disk full');});
  await h.button(h.render(),'Quit Application').props.onClick();
  const tree=h.render();
  assert.equal(h.button(tree,'Minimize to Tray').props['aria-pressed'],true);
  assert.equal(h.button(tree,'Quit Application').props['aria-pressed'],false);
  assert.match(text(find(tree,n=>n.props.role==='alert')),/Couldn’t save/);
});
test('a pending save does not claim success and blocks duplicate clicks',async()=>{
  let resolve, count=0, reloads=0;
  const h=setup(command=>{count++; assert.deepEqual(command,{command:'SetCloseButtonBehavior',payload:{behavior:'quit_app'}}); return new Promise(r=>{resolve=r;});},async()=>{reloads++;});
  const quit=h.button(h.render(),'Quit Application');
  const pending=quit.props.onClick();
  await quit.props.onClick();
  assert.equal(count,1);
  assert.equal(h.button(h.render(),'Quit Application').props.disabled,true);
  assert.equal(h.button(h.render(),'Minimize to Tray').props['aria-pressed'],true);
  resolve({}); await pending;
  assert.equal(reloads,1);
  assert.equal(h.button(h.render(),'Quit Application').props['aria-pressed'],true);
  assert.equal(h.button(h.render(),'Quit Application').props.disabled,false);
});
test('a refresh failure reports a saved choice accurately',async()=>{
  const h=setup(async()=>({}),async()=>{throw Error('refresh failed');});
  await h.button(h.render(),'Quit Application').props.onClick();
  const tree=h.render();
  assert.equal(h.button(tree,'Quit Application').props['aria-pressed'],true);
  assert.match(text(find(tree,n=>n.props.role==='alert')),/choice was saved/);
});

import assert from "node:assert/strict";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";
const { chromium } = await import(process.env.KAZE_PLAYWRIGHT_MODULE || "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs");
const browser=await chromium.launch({executablePath:'/usr/bin/chromium',headless:true,args:['--no-sandbox']});
try {
 for(const style of ['brutalist','spatial']){
  const context=await browser.newContext({viewport:{width:1440,height:900}});
  await context.addInitScript({content:`sessionStorage.setItem('test.interface-style','${style}');(${installDualInterfaceFixture.toString()})();`});
  await context.route('**/src/services/api.ts*',route=>route.fulfill({contentType:'text/javascript',body:`export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return ()=>window.__kazeFixture.listeners.delete(cb);};`}));
  const page=await context.newPage(),errors=[];page.setDefaultTimeout(10000);page.on('pageerror',e=>errors.push(e.message));
  await page.goto(process.env.KAZE_TEST_URL||'http://127.0.0.1:5196');
  await page.waitForFunction(()=>window.__kazeFixture.listeners.size===1);
  const next=page.locator(style==='brutalist'?'.press-deck':'.kaze-interface > .player-bar').getByRole('button',{name:'Next Track',exact:true});
  await next.evaluate(el=>window.focusButton=el);
  await next.click();
  await page.keyboard.press('x');
  const inspect=()=>page.evaluate(()=>{const el=window.focusButton,s=getComputedStyle(el);return {connected:el.isConnected,focused:document.activeElement===el,visible:el.matches(':focus-visible'),outline:s.outlineStyle==='none'?0:parseFloat(s.outlineWidth)};});
  let state=await inspect();console.log(style,'after click and x',state);
  assert.ok(state.connected && state.focused,'mouse focus is retained, without replacing the button');
  assert.equal(state.outline,0,'an unrelated key must not outline the mouse-clicked button');
  await page.keyboard.press('Tab');await page.keyboard.press('Shift+Tab');
  state=await inspect();assert.ok(state.focused && state.outline>0,'Tab navigation still has a visible focus indicator');
  await next.click();await page.keyboard.press('Control');await page.keyboard.press('ArrowUp');
  state=await inspect();assert.equal(state.outline,0,'clicking a keyboard-focused button restores pointer styling');
  const search=page.getByRole('textbox',{name:'Search music online'});
  await search.click();await page.keyboard.type('Asmani');assert.equal(await search.inputValue(),'Asmani','typing remains intact');
  assert.deepEqual(errors,[]);console.log(`PASS ${style}: click/key highlight suppressed; Tab/Shift+Tab and typing preserved`);
  await context.close();
 }
}finally{await browser.close();}

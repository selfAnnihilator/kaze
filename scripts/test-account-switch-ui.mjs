import assert from 'node:assert/strict';
import {installDualInterfaceFixture} from '../tests/fixtures/dualInterfaceBackend.mjs';
const {chromium}=await import(process.env.KAZE_PLAYWRIGHT_MODULE||'playwright');
const browser=await chromium.launch({executablePath:process.env.KAZE_CHROMIUM||'/usr/bin/chromium',headless:true,args:['--no-sandbox']});
try {for(const style of ['spatial','brutalist']) {
 const context=await browser.newContext();await context.addInitScript({content:`sessionStorage.setItem('test.interface-style','${style}');(${installDualInterfaceFixture.toString()})();const f=window.__kazeFixture,q=f.query.bind(f);f.account='u1';f.hold=false;f.pending=[];f.query=async r=>{if(r.query==='GetPlaylists'){const result={type:'Playlists',data:[{id:f.account+'-playlist',name:f.account==='u1'?'Old private playlist':'New private playlist',is_smart_mix:0,track_count:0}]};if(f.hold)return new Promise(resolve=>f.pending.push(()=>resolve(result)));return result;}return q(r);};`});
 await context.route('**/src/services/api.ts*',r=>r.fulfill({contentType:'text/javascript',body:`export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return()=>window.__kazeFixture.listeners.delete(cb);};`}));
 const page=await context.newPage();await page.goto(process.env.KAZE_TEST_URL||'http://127.0.0.1:5194');await page.locator('.home-flow').waitFor();
 await page.locator('.sidebar,.press-header,.press-account-navigation').getByRole('button',{name:'Library',exact:true}).click();await page.getByRole('tab',{name:'Playlists',exact:true}).click();await page.getByText('Old private playlist',{exact:true}).waitFor();
 await page.evaluate(()=>{const f=window.__kazeFixture;f.hold=true;f.emit({event:'PlaylistsUpdated',payload:{}});});await page.waitForFunction(()=>window.__kazeFixture.pending.length>0);
 await page.evaluate(()=>{const f=window.__kazeFixture;f.hold=false;f.account='u2';f.emit({event:'SessionChanged',payload:{user:{id:'u2',username:'second',created_at:1}}});});await page.waitForTimeout(100);await page.locator('.sidebar,.press-header,.press-account-navigation').getByRole('button',{name:'Library',exact:true}).click();await page.getByRole('tab',{name:'Playlists',exact:true}).click();await page.getByText('New private playlist',{exact:true}).waitFor();
 await page.evaluate(()=>{for(const done of window.__kazeFixture.pending.splice(0))done();});await page.waitForTimeout(250);
 const old=await page.getByText('Old private playlist',{exact:true}).count();
 if(process.env.KAZE_REPRO_ONLY)console.log('REPRO',style,'stale old-account playlist visible',old);
 else {assert.equal(old,0);await page.getByText('New private playlist',{exact:true}).waitFor();console.log('PASS',style,'old account playlist response rejected after delayed switch');}

 await page.getByText('New private playlist',{exact:true}).click();
 await page.locator('.collection-view').waitFor();
 await page.evaluate(()=>{const f=window.__kazeFixture;f.account='u3';f.emit({event:'SessionChanged',payload:{user:{id:'u3',username:'third',created_at:1}}});});
 await page.waitForTimeout(150);
 if(process.env.KAZE_REPRO_COLLECTION)console.log('REPRO',style,'old private collection persists',await page.locator('.collection-view').count());
 else {
  assert.equal(await page.locator('.collection-view').count(),0);
  assert.equal(await page.getByRole('button',{name:'Go back',exact:true}).isDisabled(),true);
  await page.keyboard.press('Alt+ArrowLeft');assert.equal(await page.locator('.collection-view').count(),0);
  console.log('PASS',style,'private collection and back-stack discarded on account switch');
 }
 await context.close();
}}finally{await browser.close();}

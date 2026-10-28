import assert from 'node:assert/strict';
import {installDualInterfaceFixture} from '../tests/fixtures/dualInterfaceBackend.mjs';
const {chromium}=await import(process.env.KAZE_PLAYWRIGHT_MODULE||'/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs');
const browser=await chromium.launch({executablePath:'/usr/bin/chromium',headless:true,args:['--no-sandbox']});
try {
 for(const style of ['brutalist','spatial']) {
  const context=await browser.newContext({viewport:{width:1440,height:900}});
  await context.addInitScript({content:`sessionStorage.setItem('test.interface-style','${style}');(${installDualInterfaceFixture.toString()})();
   const f=window.__kazeFixture,query=f.query.bind(f);f.refreshCount=0;f.recs.splice(0,f.recs.length,{...f.recs[0],title:"Initial relevant pick",artist:"Favorite Artist",external_track_id:"online:initial-pick",recommendation_reason:"Based on your listening habit"});
   f.query=async q=>{if(q.query==='GetDiscoveryRecommendations'&&q.payload?.force_refresh){f.queries.push(q);f.refreshCount++;await new Promise(resolve=>f.finishRefresh=resolve);f.recs[0]={...f.recs[0],title:'Manual fresh pick',external_track_id:'online:manual-fresh'};return {type:'DiscoveryRecommendations',data:f.recs.map(r=>({...r}))};}return query(q);};`});
  await context.route('**/src/services/api.ts*',r=>r.fulfill({contentType:'text/javascript',body:`export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return()=>window.__kazeFixture.listeners.delete(cb);};`}));
  const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(10000);
  await page.goto(process.env.KAZE_TEST_URL||'http://127.0.0.1:5196');await page.waitForFunction(()=>window.__kazeFixture.listeners.size===1);
  await page.locator('.spatial-navigation,.press-header').getByRole('button',{name:'Discover Music',exact:true}).click();await page.locator('.discovery-chapters').waitFor();
  if(style==='spatial')await page.locator('.chapter-nav button').nth(1).click();else await page.getByRole('combobox',{name:'Discovery selection'}).selectOption('1');
  const refresh=page.getByRole('button',{name:'Refresh For you',exact:true});await refresh.waitFor();
  await page.waitForTimeout(350);
  assert.equal(await page.evaluate(()=>window.__kazeFixture.refreshCount),0,'background loading must not request a new manual selection');
  const before=await page.evaluate(()=>({charts:window.__kazeFixture.queries.filter(q=>q.query==='GetWorldTrending').length,commands:window.__kazeFixture.commands.length}));
  await refresh.click();await page.waitForFunction(()=>window.__kazeFixture.refreshCount===1);
  assert.equal(await refresh.isDisabled(),true);assert.equal(await refresh.getAttribute('aria-busy'),'true');assert.equal(await refresh.locator('.spin-animation').count(),1);
  assert.ok(await page.locator('.discovery-chapter .discovery-track-card').count()>0,'keep existing picks while waiting for fresh ones');
  await page.evaluate(()=>window.__kazeFixture.finishRefresh());await page.getByText('Manual fresh pick',{exact:true}).first().waitFor();await page.waitForFunction(()=>!document.querySelector('[aria-label="Refresh For you"]').disabled);
  await page.evaluate(()=>window.__kazeFixture.emit({event:'TasteProfileChanged',payload:{user_id:'u1'}}));await page.waitForTimeout(450);
  assert.equal(await page.evaluate(()=>window.__kazeFixture.refreshCount),1,'automatic taste updates must not rotate the manual batch');
  assert.equal(await page.evaluate(()=>window.__kazeFixture.queries.filter(q=>q.query==='GetWorldTrending').length),before.charts,'personal refresh leaves public charts alone');
  assert.equal(await page.evaluate(()=>window.__kazeFixture.commands.length),before.commands,'refresh never changes playback or feedback');
  assert.deepEqual(errors,[]);console.log(`PASS ${style}: manual refresh requests a fresh selection, keeps cards and busy icon while pending; background refresh and playback stay independent`);await context.close();
 }
}finally{await browser.close();}

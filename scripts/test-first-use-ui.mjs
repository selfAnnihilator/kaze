/** Fresh-account setup, empty-player guard and notice timers in the real app. */
import assert from 'node:assert/strict';
import { installDualInterfaceFixture } from '../tests/fixtures/dualInterfaceBackend.mjs';
const { chromium } = await import(process.env.KAZE_PLAYWRIGHT_MODULE || '/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs');
const browser = await chromium.launch({executablePath:'/usr/bin/chromium',headless:true,args:['--no-sandbox']});
try {
 for(const style of ['spatial','brutalist']) {
  const context=await browser.newContext({viewport:{width:800,height:600}});
  await context.addInitScript({content:`sessionStorage.setItem('test.interface-style','${style}');(${installDualInterfaceFixture.toString()})();`});
  await context.addInitScript(()=>{
   const f=window.__kazeFixture,command=f.command.bind(f);
   f.empty=true;f.flow.is_active=false;
   Object.assign(f.playback,{current_track:null,current_track_id:null,current_queue_index:null,queue_track_ids:[],is_playing:false,is_paused:false});
   f.musicTaste=JSON.parse(sessionStorage.getItem('taste.fixture')||'null')||{status:'pending',artists:[],genres:[],revision:0,should_prompt:true};
   f.command=async c=>{
    if(c.command==='SaveRecommendationSetup'){
     f.commands.push(c);if(f.failSave)throw Error('Fixture save failure');
     f.musicTaste={...c.payload,status:c.payload.skipped?'skipped':'completed',should_prompt:false,revision:1};
     sessionStorage.setItem('taste.fixture',JSON.stringify(f.musicTaste));
     f.emit({event:'TasteProfileChanged',payload:{user_id:'u1'}});return {status:'Ok'};
    }
    if(c.command==='RemoveTrackFeedback'&&f.failUndo)throw Error('Fixture undo failure');
    if(c.command==='TogglePlayPause'&&!f.playback.current_track&&f.playback.queue_track_ids.length){
     f.commands.push(c);return command({command:'PlayTrack',payload:{track_id:f.playback.queue_track_ids[0]}});
    }
    return command(c);
   };
  });
  await context.route('**/src/services/api.ts*',route=>route.fulfill({contentType:'text/javascript',body:'export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return()=>window.__kazeFixture.listeners.delete(cb);};'}));
  const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(10000);
  const nav=name=>page.locator('.sidebar,.press-header,.press-account-navigation').getByRole('button',{name,exact:true}).click();
  await page.goto(process.env.KAZE_TEST_URL||'http://127.0.0.1:5194');
  const dialog=page.getByRole('dialog',{name:'Make Kaze yours'});
  try { await dialog.waitFor(); } catch (error) {
   console.log('SETUP STATE',await page.evaluate(()=>({queries:window.__kazeFixture.queries.map(q=>q.query),setup:window.__kazeFixture.musicTaste,body:document.body.innerText.slice(-1500)})));
   await page.screenshot({path:`/tmp/kaze-first-use-${style}-failure.png`});throw error;
  }
  assert.equal(await dialog.getByRole('button',{name:'Save music taste',exact:true}).isDisabled(),true);
  const box=await dialog.boundingBox();assert.ok(box.x>=0&&box.y>=0&&box.x+box.width<=800&&box.y+box.height<=600,'setup fits compact viewport');
  await dialog.getByRole('button',{name:'Rock',exact:true}).click();
  await page.evaluate(()=>window.__kazeFixture.failSave=true);
  await dialog.getByRole('button',{name:'Save music taste',exact:true}).click();
  await dialog.getByRole('alert').waitFor();
  assert.ok(await dialog.isVisible(),'failed save retains setup choices');
  await page.evaluate(()=>window.__kazeFixture.failSave=false);
  await dialog.getByRole('button',{name:'Save music taste',exact:true}).click();
  await dialog.waitFor({state:'detached'});
  assert.deepEqual(await page.evaluate(()=>window.__kazeFixture.musicTaste.genres),['Rock']);
  await page.reload();await page.locator('.home-flow').waitFor();await page.waitForTimeout(120);
  assert.equal(await dialog.count(),0,'saved setup is not shown again');
  await nav('Settings');await page.getByRole('button',{name:/Music taste.*Manage/}).click();
  const search=page.getByRole('textbox',{name:'Find an artist'});await search.fill('Nujabes');
  await page.locator('[aria-label="Artist suggestions"]').getByRole('button',{name:'Nujabes',exact:true}).click();
  await page.getByRole('button',{name:'Save music taste',exact:true}).click();
  await page.locator('.settings-view--main').waitFor();
  assert.ok(await page.evaluate(()=>window.__kazeFixture.musicTaste.artists.includes('Nujabes')));
  await nav('Home');
  const play=page.locator('.kaze-interface > .player-bar,.press-deck-controls').getByRole('button',{name:'Play',exact:true});
  const before=await page.evaluate(()=>window.__kazeFixture.commands.length);
  await play.click();await page.getByText('Select a song to play',{exact:true}).waitFor();
  await page.waitForTimeout(1100);await play.click();await page.waitForTimeout(1100);
  assert.ok(await page.locator('.playback-notice').isVisible(),'repeat click refreshes one two-second notice');
  assert.equal(await page.locator('.playback-notice').count(),1);
  await page.waitForTimeout(1100);assert.equal(await page.locator('.playback-notice').count(),0);
  assert.equal(await page.evaluate(()=>window.__kazeFixture.commands.length),before,'empty Play sends no transport command');
  assert.equal(await page.evaluate(()=>window.__kazeFixture.playback.is_playing),false);
  await page.keyboard.press('Space');await page.locator('.playback-notice').waitFor();
  await page.keyboard.press('f');await page.locator('.fullscreen-player').waitFor();
  await page.locator('.fullscreen-player').getByRole('button',{name:'Play',exact:true}).click();
  assert.equal(await page.evaluate(()=>window.__kazeFixture.playback.is_playing),false);
  await page.getByRole('button',{name:'Exit full screen',exact:true}).click();
  await page.evaluate(()=>{const f=window.__kazeFixture;f.playback.queue_track_ids=['t1'];f.emit({event:'LibraryMetadataUpdated',payload:{}});});await page.waitForTimeout(180);
  await play.click();await page.waitForFunction(()=>window.__kazeFixture.playback.current_track_id==='t1');
  const current=await page.evaluate(()=>window.__kazeFixture.playback.current_track_id);
  await page.locator('.kaze-interface > .player-bar,.press-deck-controls').getByRole('button',{name:'Pause',exact:true}).click();
  await play.click();assert.equal(await page.evaluate(()=>window.__kazeFixture.playback.current_track_id),current);
  await page.clock.install();
  const dislike=async(id,title)=>{await page.evaluate(({id,title})=>window.__kazeFixture.emit({event:'RecommendationFeedbackChanged',payload:{user_id:'u1',track_id:id,track_ids:[id],title,disliked:true,manual_like:-1}}),{id,title});await page.waitForTimeout(30);};
  await dislike('one','One');await page.mouse.move(790,10);await page.clock.fastForward(5100);
  assert.equal(await page.locator('.recommendation-feedback-toast').count(),0,'confirmation expires without clearing feedback');
  await dislike('one','One');await page.clock.fastForward(3000);await dislike('two','Two');await page.clock.fastForward(2500);
  assert.ok(await page.getByText('Won’t recommend “Two” again.',{exact:true}).isVisible(),'older expiry cannot dismiss replacement');
  await page.evaluate(()=>window.__kazeFixture.emit({event:'RecommendationFeedbackChanged',payload:{user_id:'u1',track_id:'one',track_ids:['one'],title:'One',disliked:false,manual_like:0}}));
  assert.ok(await page.getByText('Won’t recommend “Two” again.',{exact:true}).isVisible(),'an older undo event must not dismiss the replacement');
  const toast=page.locator('.recommendation-feedback-toast');await toast.hover();await page.clock.fastForward(6000);assert.ok(await toast.isVisible(),'hover pauses expiry');
  await toast.getByRole('button',{name:'Allow again',exact:true}).focus();await page.mouse.move(790,10);await page.clock.fastForward(6000);assert.ok(await toast.isVisible(),'keyboard focus pauses expiry');
  await toast.getByRole('button',{name:'Allow again',exact:true}).click();await toast.waitFor({state:'detached'});
  await dislike('three','Three');await page.evaluate(()=>window.__kazeFixture.failUndo=true);
  await page.getByRole('button',{name:'Allow again',exact:true}).click();await page.getByText('Couldn’t update recommendations',{exact:true}).waitFor();
  await page.getByRole('button',{name:'Dismiss recommendation message',exact:true}).click();
  await page.evaluate(()=>sessionStorage.removeItem('taste.fixture'));await page.reload();await dialog.waitFor();
  await dialog.getByRole('button',{name:'Skip for now',exact:true}).click();await dialog.waitFor({state:'detached'});
  assert.equal(await page.evaluate(()=>window.__kazeFixture.musicTaste.status),'skipped');
  assert.deepEqual(errors,[]);console.log(`PASS ${style}: setup persistence/search/skip/retry, compact bounds, empty/queued/fullscreen Play and timed feedback notices`);
  await context.close();
 }
}finally{await browser.close();}

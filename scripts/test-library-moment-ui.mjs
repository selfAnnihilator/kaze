import assert from 'node:assert/strict';
import {installDualInterfaceFixture} from '../tests/fixtures/dualInterfaceBackend.mjs';
const {chromium}=await import(process.env.KAZE_PLAYWRIGHT_MODULE||'/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs');
const browser=await chromium.launch({executablePath:'/usr/bin/chromium',headless:true,args:['--no-sandbox']});
try {
 for(const style of ['brutalist','spatial']) {
  const context=await browser.newContext({viewport:{width:1440,height:900}});
  await context.addInitScript({content:`sessionStorage.setItem('test.interface-style','${style}');(${installDualInterfaceFixture.toString()})();
   const f=window.__kazeFixture,query=f.query.bind(f),title='No Lie (feat. Dua Lipa) - Single';
   f.playlists.push({...f.playlists[1],id:'p2',name:'Second list'},{...f.playlists[1],id:'p3',name:'Third list'});f.tracks[2].manual_like=1;
   f.tracks.push({...f.tracks[0],id:'moment-song',title:'Exact album song',artist_name:'Sean Paul',album_title:title,file_path:'/fixture/moment.flac'});
   f.query=async q=>{if(q.query==='GetAlbumDetails'){f.queries.push(q);return {type:'AlbumDetails',data:{album:{id:q.payload.album_id,title},tracks:[f.tracks.at(-1)]}};}const r=await query(q);if(q.query==='GetHomeOverview')r.data.library_moment={album_id:'outside-first-200',album_title:title,artist_name:'Sean Paul',total_tracks:1,first_track_id:'moment-song'};return r;};`});
  await context.route('**/src/services/api.ts*',r=>r.fulfill({contentType:'text/javascript',body:`export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return()=>window.__kazeFixture.listeners.delete(cb);};`}));
  const page=await context.newPage(),errors=[];page.on('pageerror',e=>{errors.push(e.message);console.error('App error:',e.message);});page.setDefaultTimeout(8000);
  await page.goto(process.env.KAZE_TEST_URL||'http://127.0.0.1:5196');await page.waitForFunction(()=>window.__kazeFixture.listeners.size===1);
  const home=page.locator(style==='brutalist'?'.press-home':'.spatial-home');
  const moment=page.locator(style==='brutalist'?'.press-home-moment':'.spatial-library-moment');
  const cover=moment.locator(style==='brutalist'?'.press-home-moment-art':'.spatial-music-cover');
  await cover.click();
  await page.waitForFunction(()=>window.__kazeFixture.commands.some(c=>c.command==='PlayTrack'&&c.payload.track_id==='moment-song'));
  assert.equal(await home.count(),1,'cover plays directly without opening another view');
  assert.equal(await page.evaluate(()=>window.__kazeFixture.queries.filter(q=>q.query==='GetAlbumDetails').length),0);
  await page.waitForFunction(()=>document.querySelector('[aria-label^="Pause No Lie"]'));
  await cover.click();await page.waitForFunction(()=>window.__kazeFixture.commands.at(-1)?.command==='Pause');
  await cover.click();await page.waitForFunction(()=>window.__kazeFixture.commands.at(-1)?.command==='Resume');
  const commandCount=await page.evaluate(()=>window.__kazeFixture.commands.length);
  await moment.getByRole('button',{name:'Open record',exact:true}).click();
  await page.locator('.library-view .track-row').first().waitFor();
  assert.equal(await page.locator('.library-view .track-row').count(),1,'exact album members only');
  if(style==='brutalist')assert.equal(await page.locator('.press-library-heading .press-big-index').textContent(),'01','album detail shows its own track count');
  assert.equal(await page.evaluate(()=>window.__kazeFixture.commands.length),commandCount,'Open record does not change playback');
  const detailQueries=await page.evaluate(()=>window.__kazeFixture.queries.filter(q=>q.query==='GetAlbumDetails').length);
  await page.evaluate(()=>window.__kazeFixture.emit({event:'LibraryMetadataUpdated',payload:{}}));
  await page.waitForFunction(count=>window.__kazeFixture.queries.filter(q=>q.query==='GetAlbumDetails').length>count,detailQueries);
  await page.waitForTimeout(100);
  assert.equal(await page.locator('.library-view .track-row').count(),1,'background library refresh retains the exact album');
  assert.ok(await page.evaluate(()=>window.__kazeFixture.queries.some(q=>q.query==='GetAlbumDetails'&&q.payload.album_id==='outside-first-200')),'open by ID even when absent from album cache');
  await page.locator('.spatial-navigation,.press-header').getByRole('button',{name:'Home',exact:true}).click();
  await page.evaluate(()=>{const f=window.__kazeFixture,q=f.query;f.query=request=>{if(request.query==='GetAlbumDetails')throw Error('Fixture album removed');return q(request);};});
  await moment.getByRole('button',{name:'Open record',exact:true}).click();
  await page.getByText('Couldn’t open album',{exact:true}).waitFor();
  assert.equal(await page.locator('.library-view .track-row').count(),0,'missing album must not retain another album’s rows');
  if(style==='brutalist') {
   const tabs=page.getByRole('tablist',{name:'Library categories'});
   for(const [name,count] of [['Albums','130'],['Tracks','1037'],['Playlists','03'],['Liked','02']]) {
    await tabs.getByRole('tab',{name,exact:true}).click();
    await page.waitForFunction(value=>document.querySelector('.press-library-heading .press-big-index')?.textContent===value,count);
    assert.equal(await tabs.getByRole('tab',{name,exact:true}).getAttribute('aria-selected'),'true');
   }
   console.log('PASS archive count follows Albums, Tracks, Playlists and Liked, including exact album scope');
  }
  assert.deepEqual(errors,[]);console.log(`PASS ${style}: Library Moment cover plays/pauses/resumes directly; Open record opens its uncached album by ID without interrupting`);await context.close();
 }
}finally{await browser.close();}

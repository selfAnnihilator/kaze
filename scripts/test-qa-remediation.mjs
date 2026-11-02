/** Real App, controlled IPC. KAZE_PLAYWRIGHT_MODULE may point to a Playwright installation. */
import assert from 'node:assert/strict';
import { installDualInterfaceFixture } from '../tests/fixtures/dualInterfaceBackend.mjs';
const {chromium}=await import(process.env.KAZE_PLAYWRIGHT_MODULE || 'playwright');
const browser=await chromium.launch({executablePath:process.env.KAZE_CHROMIUM || '/usr/bin/chromium',headless:true,args:['--no-sandbox']});
try {for(const style of ['spatial','brutalist']) {
 const context=await browser.newContext({viewport:{width:1440,height:900}});
 await context.addInitScript({content:`sessionStorage.setItem('test.interface-style','${style}');(${installDualInterfaceFixture.toString()})();
 const f=window.__kazeFixture,q=f.query.bind(f),cmd=f.command.bind(f);f.albumCount=201;f.failPage=true;f.createMode='fail';f.failMembership=true;f.members={};f.failAlbums=true;f.heldPages=[];f.holdPages=false;
 f.query=async r=>{if(r.query==='GetPlaylists'&&f.failListRefresh){f.failListRefresh=false;throw Error('Audit list refresh failure');}if(r.query==='GetTrackPlaylistMemberships')return{type:'TrackPlaylistMemberships',data:structuredClone(f.members)};if(r.query==='GetLibraryAlbumsPage'&&r.payload.offset===100&&f.failAlbums){f.queries.push(r);throw Error('Audit album page failure');}if(r.query==='GetLibraryTracksPage'&&f.holdPages){f.queries.push(r);return new Promise(resolve=>f.heldPages.push(()=>q(r).then(resolve)));}if(r.query==='GetLibraryTracksPage'&&r.payload.offset===200&&f.failPage){f.queries.push(r);throw Error('Audit page failure');}return q(r);};
 f.command=async c=>{if(c.command==='CreatePlaylist'){f.commands.push(c);await new Promise(r=>setTimeout(r,350));if(f.createMode==='fail')throw Error('Audit create failure');if(f.createMode==='cancel')return{status:'Ok',data:null};const id='created-'+f.commands.length;f.playlists.push({id,name:c.payload.name,is_smart_mix:0,track_count:0});return{status:'PlaylistCreated',data:id};}
 if(['AddTrackToPlaylist','RemoveTrackFromPlaylist'].includes(c.command)){f.commands.push(c);if(f.failMembership)throw Error('Audit membership failure');const ids=f.members[c.payload.track_id]||[];f.members[c.payload.track_id]=c.command==='AddTrackToPlaylist'?[...new Set([...ids,c.payload.playlist_id])]:ids.filter(id=>id!==c.payload.playlist_id);return{status:'Ok'};}return cmd(c);};`});
 await context.route('**/src/services/api.ts*',r=>r.fulfill({contentType:'text/javascript',body:`export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return()=>window.__kazeFixture.listeners.delete(cb);};`}));
 const page=await context.newPage(), errors=[];page.setDefaultTimeout(12000);page.on('pageerror',e=>errors.push(e.message));
 await page.goto(process.env.KAZE_TEST_URL||'http://127.0.0.1:5194');await page.locator('.home-flow').waitFor();
 const nav=name=>page.locator('.sidebar,.press-header,.press-account-navigation').getByRole('button',{name,exact:true}).click();
 await nav('Library');await page.getByRole('tab',{name:'Tracks',exact:true}).click();await page.locator('.library-view .track-row').first().waitFor();
 const scroll=()=>page.locator('.library-view').evaluate(root=>{const el=[...root.querySelectorAll('div')].find(el=>el.scrollHeight>el.clientHeight+100&&getComputedStyle(el).overflowY==='auto');if(!el)throw Error('Missing list');el.scrollTop=el.scrollHeight;});
 await scroll();await page.getByRole('alert').filter({hasText:'Audit page failure'}).waitFor();assert.ok(await page.locator('.track-row').count()>0);await page.evaluate(()=>window.__kazeFixture.failPage=false);await page.getByRole('button',{name:'Retry loading',exact:true}).click();
 for(let i=0;i<8;i++){await scroll();await page.waitForTimeout(130);}
 await page.getByText('Library track 1035',{exact:true}).waitFor();assert.ok(await page.locator('.track-row').count()<35);
 assert.match(await page.locator('.library-view').textContent(), /1036\s*local audio tracks/);
 assert.match(await page.locator(style==='spatial'?'.spatial-library-header':'.press-library-heading').innerText(), /1036|1,036/);
 console.log('PASS',style,'1036 tracks, subsequent-page failure/retry and bounded rows');
 await page.evaluate(()=>{const f=window.__kazeFixture;f.holdPages=true;f.emit({event:'LibraryMetadataUpdated',payload:{}});});
 await page.waitForFunction(()=>window.__kazeFixture.heldPages.length>0);
 await page.getByRole('tab',{name:'Albums',exact:true}).click();
 await page.getByRole('button',{name:'Open album migration',exact:true}).first().click();
 await page.locator('.library-view .track-row').first().waitFor();
 const exact=await page.locator('.library-view').textContent();
 await page.evaluate(async()=>{const f=window.__kazeFixture;f.holdPages=false;for(const done of f.heldPages.splice(0))await done();});
 await page.waitForTimeout(200);assert.equal(await page.locator('.library-view').textContent(),exact,'old general page cannot overwrite exact album');
 console.log('PASS',style,'delayed general refresh cannot overwrite album destination');
 const kerala=()=>page.locator('.library-view .track-row').filter({has:page.getByRole('button',{name:/^(Play|Pause|Resume) Kerala$/})});
 await kerala().getByRole('button',{name:'Like track',exact:true}).click();
 await kerala().getByRole('button',{name:'Unlike track',exact:true}).waitFor();
 await page.getByRole('tab',{name:'Tracks',exact:true}).click();
 await kerala().getByRole('button',{name:'Unlike track',exact:true}).waitFor();
 await kerala().getByRole('button',{name:'Unlike track',exact:true}).click();
 await page.getByRole('tab',{name:'Albums',exact:true}).click();
 await page.getByRole('button',{name:'Open album migration',exact:true}).first().click();
 await kerala().getByRole('button',{name:'Like track',exact:true}).waitFor();
 console.log('PASS',style,'feedback remains consistent across cached general and album lists');


 await page.getByRole('tab',{name:'Albums',exact:true}).click();
 for(let i=0;i<10;i++){
  await page.locator(style==='brutalist'?'[aria-label="Album spines"]':'[aria-label="Album shelves"]').evaluate(el=>{el.style.scrollBehavior='auto';el.scrollTo({left:el.scrollWidth,top:el.scrollHeight,behavior:'instant'});});
  await page.waitForTimeout(100);if(await page.getByRole('alert').filter({hasText:'Audit album page failure'}).count())break;
 }
 await page.getByRole('alert').filter({hasText:'Audit album page failure'}).waitFor();
 await page.evaluate(()=>window.__kazeFixture.failAlbums=false);await page.getByRole('button',{name:'Retry loading',exact:true}).click();
 for(let i=0;i<35;i++){await page.locator(style==='brutalist'?'[aria-label="Album spines"]':'[aria-label="Album shelves"]').evaluate(el=>{el.style.scrollBehavior="auto";el.scrollTo({top:el.scrollHeight,left:el.scrollWidth,behavior:"instant"});});await page.waitForTimeout(80);}
 await page.waitForFunction(()=>window.__kazeFixture.queries.some(q=>q.query==='GetLibraryAlbumsPage'&&q.payload.offset===200));
 console.log('PASS',style,'201 albums loaded through bounded presentation');
 await page.getByRole('tab',{name:'Playlists',exact:true}).click();await page.getByRole('button',{name:'New Playlist',exact:true}).first().click();
 const input=page.getByPlaceholder('Playlist name'),submit=page.locator('.modal-content').getByRole('button',{name:'Create Playlist',exact:true});
 await input.fill('Audit draft');await submit.click();assert.equal(await submit.isDisabled(),true);assert.equal(await page.locator('.modal-content').getByRole('button',{name:'Cancel',exact:true}).isDisabled(),true);
 await page.getByRole('alert').filter({hasText:'Audit create failure'}).waitFor();assert.equal(await input.inputValue(),'Audit draft');
 await page.evaluate(()=>window.__kazeFixture.createMode='cancel');await submit.click();await page.waitForTimeout(450);assert.equal(await input.inputValue(),'Audit draft');
 await page.evaluate(()=>{window.__kazeFixture.createMode='success';window.__kazeFixture.failListRefresh=true;});await submit.click();await input.waitFor({state:'detached'});
 assert.equal(await page.evaluate(()=>window.__kazeFixture.playlists.filter(p=>p.name==='Audit draft').length),1);
 await page.getByText('Playlist created; list refresh failed',{exact:true}).first().waitFor();
 const creates=await page.evaluate(()=>window.__kazeFixture.commands.filter(c=>c.command==='CreatePlaylist').length);
 await page.evaluate(()=>window.__kazeFixture.failListRefresh=true);await page.getByRole('button',{name:'Refresh playlists',exact:true}).click();
 await page.getByRole('alert').filter({hasText:'Couldn’t refresh playlists'}).waitFor();
 await page.getByRole('button',{name:'Refresh playlists',exact:true}).click();
 await page.waitForTimeout(100);assert.equal(await page.evaluate(()=>window.__kazeFixture.commands.filter(c=>c.command==='CreatePlaylist').length),creates);
 console.log('PASS',style,'saved playlist survives list refresh failure and refresh retry never creates another');

 console.log('PASS',style,'playlist rejection, cancellation, pending guards and successful retry');
 await page.getByRole('tab',{name:'Tracks',exact:true}).click();await page.locator('.track-row').first().waitFor();await page.getByRole('button',{name:/Add to playlist|In playlist \(click to manage\)/}).first().click();
 const modal=page.locator('.modal-content');await modal.getByText('After hours',{exact:true}).click();await modal.getByRole('alert').filter({hasText:'Audit membership failure'}).waitFor();
 await modal.getByText('Not in any playlist',{exact:true}).waitFor();
 await page.evaluate(()=>window.__kazeFixture.failMembership=false);await modal.getByText('After hours',{exact:true}).click();
 await modal.getByText('In 1 playlist',{exact:true}).waitFor();
 await page.evaluate(()=>window.__kazeFixture.failMembership=true);await modal.getByText('After hours',{exact:true}).click();
 await modal.getByRole('alert').filter({hasText:'Audit membership failure'}).waitFor();
 await modal.getByText('In 1 playlist',{exact:true}).waitFor();
 await page.evaluate(()=>window.__kazeFixture.failMembership=false);await modal.getByText('After hours',{exact:true}).click();
 await modal.getByText('Not in any playlist',{exact:true}).waitFor();
 assert.deepEqual(errors,[]);console.log('PASS',style,'membership rejection displayed and retry handled without page errors');
 await context.close();
}} finally {await browser.close();}

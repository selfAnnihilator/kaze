import assert from 'node:assert/strict';
import { installDualInterfaceFixture } from '../tests/fixtures/dualInterfaceBackend.mjs';
const {chromium}=await import(process.env.KAZE_PLAYWRIGHT_MODULE||'/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs');
const browser=await chromium.launch({executablePath:'/usr/bin/chromium',headless:true,args:['--no-sandbox']});
const base=process.env.KAZE_TEST_URL||'http://127.0.0.1:5196';
try {
 for(const style of ['brutalist','spatial']) {
  const context=await browser.newContext({viewport:{width:1440,height:900}});
  await context.addInitScript({content:`sessionStorage.setItem('test.interface-style','${style}');(${installDualInterfaceFixture.toString()})();
   const original=window.__kazeFixture.query;
   window.__kazeFixture.query=async function(q){const r=await original(q);if(q.query==='GetAlbums')r.data=r.data.map((a,i)=>({...a,title:i%2?'Seasons Change - Violet Evergarden Piano Memories - Original Soundtrack Deluxe Edition':'A'.repeat(220),artist_name:'A very long artist name that must never overlap the album title'}));return r;};`});
  await context.route('**/src/services/api.ts*',r=>r.fulfill({contentType:'text/javascript',body:`export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return()=>window.__kazeFixture.listeners.delete(cb);};`}));
  const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto(base);await page.waitForFunction(()=>window.__kazeFixture.listeners.size===1);
  if(style==='brutalist') {
   await page.locator('.press-header').getByRole('button',{name:'Library',exact:true}).click();
   await page.locator('.press-record-spine').first().waitFor();
   for(const [width,height] of [[1920,1080],[1440,900],[1024,600],[800,700]]) {
    await page.setViewportSize({width,height});await page.waitForTimeout(80);
    const issues=await page.locator('.press-record-spine').evaluateAll(els=>els.flatMap(el=>{
     const title=el.querySelector('strong'),artist=el.querySelector('small'),t=title.getBoundingClientRect(),a=artist.getBoundingClientRect(),b=el.getBoundingClientRect(),s=getComputedStyle(title),as=getComputedStyle(artist);
     return t.right>a.left+1||t.bottom>b.bottom+1||a.right>b.right+1||s.whiteSpace!=='nowrap'||as.whiteSpace!=='nowrap'||parseFloat(s.fontSize)>20?[{t:t.toJSON(),a:a.toJSON(),b:b.toJSON(),wrap:s.whiteSpace,font:s.fontSize}]:[];
    }));
    assert.equal(issues.length,0,`long album labels stay in separate clipped lanes at ${width}x${height}: ${JSON.stringify(issues.slice(0,2))}`);
   }
   await page.setViewportSize({width:1440,height:900});
   const shelf=page.getByRole('region',{name:'Album spines'});await shelf.evaluate(el=>el.scrollLeft=0);await page.waitForTimeout(100);
   await page.screenshot({path:'/tmp/kaze-album-spines-fixed.png'});
   const b=await shelf.boundingBox();await page.mouse.move(b.x+b.width*.75,b.y+90);await page.mouse.down();await page.mouse.move(b.x+80,b.y+90,{steps:16});await page.mouse.up();await page.waitForTimeout(100);
   assert.ok(await shelf.evaluate(el=>el.scrollLeft)>300,'mouse drag scrolls album shelf');assert.equal(await page.locator('.press-record-spine').count()<30,true,'drag preserves bounded album DOM');
   assert.equal(await page.locator('.press-albums').count(),1,'drag must not open an album');
   const card=page.locator('.press-record-spine').filter({visible:true}).first();await card.click();await page.locator('.library-view').waitFor();
   console.log('PASS brutalist: long album metadata clipped at four sizes; mouse drag scrolls without opening; normal click still opens');
  }
  const nav=style==='brutalist'?page.locator('.press-header'):page.locator('.spatial-navigation');
  await nav.getByRole('button',{name:'Discover Music',exact:true}).click();
  const shelf=page.locator('.discovery-chapter .horizontal-scroll-row').first();await shelf.waitFor();await shelf.evaluate(el=>el.scrollLeft=0);
  const b=await shelf.boundingBox(),before=await page.evaluate(()=>window.__kazeFixture.commands.length);
  await page.mouse.move(b.x+b.width*.75,b.y+80);await page.mouse.down();await page.mouse.move(b.x+60,b.y+80,{steps:16});await page.mouse.up();await page.waitForTimeout(100);
  assert.ok(await shelf.evaluate(el=>el.scrollLeft)>100,`${style} discovery mouse drag scrolls`);
  assert.equal(await page.evaluate(()=>window.__kazeFixture.commands.length),before,'drag does not play, download or queue a card');
  assert.deepEqual(errors,[]);console.log(`PASS ${style}: discovery dragging preserved without accidental actions`);await context.close();
 }
}finally{await browser.close();}

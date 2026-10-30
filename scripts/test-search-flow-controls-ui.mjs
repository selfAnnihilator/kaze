import assert from "node:assert/strict";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";
const { chromium } = await import(process.env.KAZE_PLAYWRIGHT_MODULE || "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs");
const browser = await chromium.launch({ executablePath: "/usr/bin/chromium", headless: true, args: ["--no-sandbox"] });
try {
  for (const style of ["brutalist", "spatial"]) {
    const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
    await context.addInitScript({ content: `sessionStorage.setItem('test.interface-style','${style}');(${installDualInterfaceFixture.toString()})();
      const f=window.__kazeFixture, query=f.query.bind(f), command=f.command.bind(f);
      f.matches=Array.from({length:125},(_,i)=>({...f.recs[0],external_track_id:'jiosaavn:match'+i,provider:'jiosaavn',provider_id:'match'+i,playback_source:{provider:'jiosaavn',track_id:'match'+i},title:'Asmani '+i,artist:'Test Artist',album:'Album'}));
      f.query=async q=>q.query==='SearchOnlineMusic'? (f.queries.push(q),{type:'SearchResults',data:f.matches}):query(q);
      f.command=async c=>{const r=await command(c);if(c.command==='PlayOnlineTrack'){await command({command:'StopFlow'});f.emit({event:'QueueUpdated',payload:{items:[],current_index:0}});}return r;};` });
    await context.route("**/src/services/api.ts*", route => route.fulfill({ contentType: "text/javascript", body: `export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return ()=>window.__kazeFixture.listeners.delete(cb);};` }));
    const page=await context.newPage(), errors=[];
    page.setDefaultTimeout(10000);
    page.on('pageerror',error=>errors.push(error.message));
    await page.goto(process.env.KAZE_TEST_URL || 'http://127.0.0.1:5196');
    const home=page.locator('.home-flow');
    await home.getByRole('button',{name:'Pause Flow',exact:true}).click();
    await home.getByRole('button',{name:'Start Flow',exact:true}).click();
    await home.getByRole('button',{name:'Pause Flow',exact:true}).waitFor();
    assert.equal(await page.getByRole('button',{name:'Stop Flow',exact:true}).count(),0);
    const flowCommands=await page.evaluate(()=>window.__kazeFixture.commands.map(c=>c.command));
    assert.ok(flowCommands.includes('Pause') && flowCommands.includes('ContinueFlow'),'Start resumes paused Flow');
    assert.ok(!flowCommands.includes('StopFlow'),'pausing does not disable Flow');
    if(style==='brutalist'){
      const deck=page.locator('.press-deck-controls');
      await deck.getByRole('button',{name:'Pause Flow',exact:true}).click();
      await deck.getByRole('button',{name:'Start Flow',exact:true}).click();
      await deck.getByRole('button',{name:'Pause Flow',exact:true}).waitFor();
    }
    const search=page.getByRole('textbox',{name:'Search music online'});
    await search.fill('Asmani');
    await search.press('Enter');
    const list=page.locator('[aria-label="Search songs"]');
    await list.locator('.online-search-song').first().waitFor();
    assert.equal(await page.getByRole('button',{name:/show all songs/i}).count(),0);
    assert.equal(await page.getByText(/More Matching Results/).count(),0);
    const measure=()=>page.evaluate(()=>{
      const list=document.querySelector('[aria-label="Search songs"]'),panel=document.querySelector('.online-search-results'),main=document.querySelector('.main-content');
      return {height:list.clientHeight,gap:main.getBoundingClientRect().bottom-list.getBoundingClientRect().bottom,panelGap:main.getBoundingClientRect().bottom-panel.getBoundingClientRect().bottom,rows:list.querySelectorAll('.online-search-song').length,scrollHeight:list.scrollHeight};
    });
    let size=await measure();
    assert.ok(size.height>450,JSON.stringify(size));
    assert.ok(Math.abs(size.panelGap)<3 && size.gap<35,JSON.stringify(size));
    assert.ok(size.rows<30 && size.scrollHeight>=125*80,'all results available with bounded DOM');
    await list.evaluate(el=>{el.scrollTop=el.scrollHeight;});
    await list.locator('.online-search-song[data-track-id="jiosaavn:match124"]').waitFor();
    await list.locator('.online-search-song[data-track-id="jiosaavn:match124"]').click();
    await page.waitForFunction(()=>window.__kazeFixture.commands.some(c=>c.command==='PlayOnlineTrack' && c.payload.track_id==='jiosaavn:match124'));
    assert.equal(await page.evaluate(()=>window.__kazeFixture.flow.is_active),false,'outside song ends Flow');
    await page.getByRole('button',{name:'Songs (125)',exact:true}).click();
    await list.locator('.online-search-song').first().waitFor();
    size=await measure(); assert.ok(size.height>450 && size.gap<35,JSON.stringify(size));
    const tallHeight=size.height;
    await page.setViewportSize({width:1440,height:780});
    await page.waitForFunction(tall=>document.querySelector('[aria-label="Search songs"]').clientHeight<tall-150,tallHeight);
    size=await measure();assert.ok(size.height>240 && size.rows<30 && size.gap<35,JSON.stringify(size));
    await list.evaluate(el=>{el.scrollTop=el.scrollHeight;});
    await list.locator('.online-search-song[data-track-id="jiosaavn:match124"]').waitFor();
    await page.getByRole('button',{name:'All',exact:true}).click();
    await page.setViewportSize({width:1440,height:1080});
    await page.waitForFunction(()=>document.querySelector('[aria-label="Search songs"]').clientHeight>600);
    await page.screenshot({path:`/tmp/kaze-search-full-${style}.png`});
    await page.setViewportSize({width:800,height:900});
    await page.waitForFunction(()=>getComputedStyle(document.querySelector('.online-search-top-result')).display==='none');
    size=await measure();assert.ok(size.height>300 && size.gap<35,JSON.stringify(size));
    await list.evaluate(el=>{el.scrollTop=el.scrollHeight;});
    await list.locator('.online-search-song[data-track-id="jiosaavn:match124"]').waitFor();
    await page.setViewportSize({width:800,height:600});
    await page.waitForTimeout(120);
    size=await measure();assert.ok(size.height>100 && size.gap<35,JSON.stringify(size));
    await list.evaluate(el=>{el.scrollTop=el.scrollHeight;});
    await list.locator('.online-search-song[data-track-id="jiosaavn:match124"]').waitFor();
    assert.deepEqual(errors,[]);
    console.log(`PASS ${style}: Start/Pause only, full default search, final result reachable, bounded rows, viewport resize and outside-song Flow exit`);
    await context.close();
  }
} finally {await browser.close();}

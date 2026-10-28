import assert from "node:assert/strict";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";
const { chromium } = await import(process.env.KAZE_PLAYWRIGHT_MODULE || "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs");
const browser = await chromium.launch({ executablePath: process.env.KAZE_CHROMIUM || "/usr/bin/chromium", headless: true, args: ["--no-sandbox"] });
try {
  for (const style of ["brutalist", "spatial"]) {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    await context.addInitScript({ content: `sessionStorage.setItem('test.interface-style','${style}');(${installDualInterfaceFixture.toString()})();
      const f=window.__kazeFixture, query=f.query.bind(f), command=f.command.bind(f);
      command({command:'StopFlow'}); f.playback.is_playing=false; f.playback.is_paused=true;
      f.cache=sessionStorage.getItem('test.warmCache') ? f.recs : []; f.holdHome=false; f.homeWaiters=[]; f.trendWaiters=[]; f.startWaiters=[]; f.personalWaiters=[];
      f.query=async q=>{
        if(q.query==='GetCachedWorldTrending') {f.queries.push(q);return {type:'DiscoveryRecommendations',data:f.cache};}
        if(q.query==='GetWorldTrending') {f.queries.push(q);return new Promise((resolve,reject)=>f.trendWaiters.push({resolve,reject}));}
        if(q.query==='GetHomeOverview' && f.holdHome) return new Promise(resolve=>f.homeWaiters.push(()=>query(q).then(resolve)));
        if(q.query==='GetDiscoveryRecommendations' && f.holdPersonal) return new Promise(resolve=>f.personalWaiters.push(()=>query(q).then(resolve)));
        return query(q);
      };
      f.command=async c=>{
        if(c.command==='StartFlow') {f.commands.push(c);return new Promise(resolve=>f.startWaiters.push(async()=>{
          await command(c);f.playback.is_playing=true;f.playback.is_paused=false;
          f.emit({event:'PlaybackResumed',payload:{track_id:'t0',position_secs:47}});resolve({status:'Ok'});
        }));}
        return command(c);
      };` });
    await context.route("**/src/services/api.ts*", route => route.fulfill({ contentType: "text/javascript", body: `export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return ()=>window.__kazeFixture.listeners.delete(cb);};` }));
    const page = await context.newPage(), errors = [];
    page.setDefaultTimeout(15000);
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(process.env.KAZE_TEST_URL || "http://127.0.0.1:5196", { waitUntil: "domcontentloaded" });
    await page.waitForFunction(() => window.__kazeFixture.listeners.size === 1);
    const start = page.getByRole('button',{name:'Start Flow',exact:true}).first();
    await start.click();
    const starting = page.getByRole('button',{name:'Starting Flow…',exact:true});
    assert.equal(await starting.isDisabled(),true,'Start gives immediate busy feedback');
    // A background Home response must not clear the pending command.
    const homeRequests = await page.evaluate(()=>window.__kazeFixture.queries.filter(q=>q.query==='GetHomeOverview').length);
    await page.evaluate(()=>{const f=window.__kazeFixture;f.emit({event:'TasteProfileChanged',payload:{user_id:'u1'}});});
    await page.waitForFunction(previous=>window.__kazeFixture.queries.filter(q=>q.query==='GetHomeOverview').length>previous,homeRequests);
    assert.equal(await starting.isDisabled(),true,'background Home does not clear action busy');
    await page.evaluate(async()=>{const f=window.__kazeFixture;f.holdHome=true;await f.startWaiters.shift()();});
    await page.getByRole('button',{name:'Pause Flow',exact:true}).first().waitFor();
    assert.equal(await page.getByRole('button',{name:'Pause Flow',exact:true}).first().isEnabled(),true,'artwork refresh does not hold Start busy');
    await page.evaluate(()=>{const f=window.__kazeFixture;f.holdHome=false;for(const done of f.homeWaiters.splice(0))done();});
    await page.getByRole('button',{name:'Pause Flow',exact:true}).first().click();
    await page.getByRole('button',{name:'Start Flow',exact:true}).first().waitFor();
    assert.equal(await page.getByRole('button',{name:'Stop Flow',exact:true}).count(),0);
    // Direct backend Stop remains an internal action; Home exposes only Start/Pause.
    await page.evaluate(async()=>{const f=window.__kazeFixture;await f.command({command:'StopFlow'});f.emit({event:'QueueUpdated',payload:{items:[],current_index:null}});});
    await page.getByRole('button',{name:'Start Flow',exact:true}).first().click();
    await page.evaluate(async()=>{await window.__kazeFixture.startWaiters.shift()();});
    await page.getByRole('button',{name:'Pause Flow',exact:true}).first().waitFor();
    const commands = await page.evaluate(()=>window.__kazeFixture.commands.map(c=>c.command));
    assert.equal(commands.filter(c=>c==='ContinueFlow'||c==='Resume').length,0,'Start never needs a second resume command');
    // Trending request is still deliberately pending from startup.
    await page.locator(".sidebar, .press-header, .press-account-navigation").getByRole('button',{name:'Discover Music',exact:true}).click();
    await page.getByText('Loading trending songs…',{exact:true}).waitFor();
    assert.equal(await page.getByText('No trending songs found',{exact:true}).count(),0);
    await page.evaluate(()=>{const f=window.__kazeFixture;for(const w of f.trendWaiters.splice(0))w.resolve({type:'DiscoveryRecommendations',data:f.recs});});
    await page.getByText('Loading trending songs…',{exact:true}).waitFor({state:'hidden'});
    const trendingCards = page.locator('.windowed-strip[aria-label="Trending Songs"]');
    assert.ok(await trendingCards.count());
    assert.equal(await page.getByRole('button',{name:/^Refresh Trending songs$/i}).count(),1,'one refresh control');
    await page.evaluate(()=>{window.__kazeFixture.holdPersonal=true;});
    await page.getByRole('button',{name:/^Refresh Trending songs$/i}).click();
    await page.waitForFunction(()=>window.__kazeFixture.trendWaiters.length>0);
    assert.ok(await trendingCards.isVisible(),'old songs stay visible while updating');
    await page.evaluate(()=>{for(const w of window.__kazeFixture.trendWaiters.splice(0))w.reject(Error('network unavailable'));});
    await page.waitForFunction(()=>document.querySelector('[aria-label="Refresh Trending songs"]')?.disabled===false);
    assert.ok(await trendingCards.isVisible(),'refresh failure retains old songs');
    assert.equal(await page.evaluate(()=>window.__kazeFixture.personalWaiters.length),0,'Trending refresh never waits on personal recommendations');
    // A new renderer shows saved cards before the deliberately held network reply.
    await page.evaluate(()=>sessionStorage.setItem('test.warmCache','1'));
    await page.reload({waitUntil:'domcontentloaded'});
    await page.waitForFunction(()=>window.__kazeFixture.listeners.size===1);
    await page.locator(".sidebar, .press-header, .press-account-navigation").getByRole('button',{name:'Discover Music',exact:true}).click();
    await page.locator('.windowed-strip[aria-label="Trending Songs"]').waitFor();
    await page.locator('.discovery-refresh-control button[aria-busy="true"]').waitFor();
    assert.equal(await page.getByText('Updating trending songs…',{exact:true}).count(),0,'background refresh uses the existing icon instead of a loose text line');
    assert.ok(await page.evaluate(()=>window.__kazeFixture.trendWaiters.length>0),'saved cards appear before network');
    assert.equal(await page.getByText('No trending songs found',{exact:true}).count(),0);
    assert.deepEqual(errors,[]);
    console.log(`PASS ${style}: immediate Flow feedback, background refresh isolation, stop/start autoplay, truthful Trending loading, retained songs and independent refresh`);
    await page.screenshot({path:`/tmp/kaze-flow-startup-${style}.png`});
    await context.close();
  }
} finally { await browser.close(); }

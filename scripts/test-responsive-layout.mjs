/** Real production tree: compact windows, scroll ownership and resize continuity. */
import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
import { installDualInterfaceFixture } from '../tests/fixtures/dualInterfaceBackend.mjs';
const { chromium } = await import(process.env.KAZE_PLAYWRIGHT_MODULE || '/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs');
const out = process.env.KAZE_TEST_OUTPUT || '/tmp/kaze-responsive';
await mkdir(out, { recursive: true });
const browser = await chromium.launch({ executablePath: process.env.KAZE_CHROMIUM || '/usr/bin/chromium', headless: true, args: ['--no-sandbox'] });
const sizes = [[800,600],[800,700],[1024,600],[1366,768],[1440,900],[1920,1080],[940,1000],[800,1000]];
try {
  for (const theme of ['spatial', 'brutalist']) {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    await context.addInitScript({ content: `sessionStorage.setItem('test.interface-style','${theme}');(${installDualInterfaceFixture.toString()})();` });
    await context.addInitScript(() => {
      const mode = sessionStorage.getItem('responsive.state'), f = window.__kazeFixture;
      const query = f.query.bind(f);
      f.empty = mode === 'empty';
      if(mode === 'long') {
        for(const t of f.tracks) {t.title = 'A very long recording title '.repeat(8); t.artist_name = 'Artist '.repeat(20);}
        for(const r of f.recs) {r.title = 'A very long discovery title '.repeat(8); r.artist = 'Artist '.repeat(20);}
      }
      f.query = async request => {
        if(request.query === 'GetWorldTrending') {
          if(mode === 'error') throw Error('Fixture trending failure');
          if(mode === 'loading') await new Promise(resolve => setTimeout(resolve, 2000));
        }
        return query(request);
      };
    });
    await context.route('**/src/services/api.ts*', route => route.fulfill({ contentType:'text/javascript', body:'export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return()=>window.__kazeFixture.listeners.delete(cb);};' }));
    const page = await context.newPage(), errors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.setDefaultTimeout(10000);
    const nav = async name => {
      if (name === 'Now Playing') return page.locator('.kaze-interface > .player-bar,.press-deck-controls').getByRole('button',{name:'Open Now Playing',exact:true}).click();
      const direct = page.locator('.sidebar,.press-header,.press-account-navigation').getByRole('button',{name,exact:true});
      if(await direct.count()) return direct.click();
      if(['Albums','Downloaded Tracks','Playlists'].includes(name)) {
        await nav('Library');
        return page.getByRole('tab',{name:name === 'Downloaded Tracks' ? 'Tracks' : name,exact:true}).click();
      }
      await page.getByRole('button',{name:'Library and more',exact:true}).click();
      await page.getByRole('menuitem',{name,exact:true}).click();
    };
    const chapter = async index => theme === 'spatial'
      ? page.locator('.chapter-nav button').nth(index).click()
      : page.getByRole('combobox',{name:'Discovery selection'}).selectOption(String(index));
    const active = () => page.locator('.discovery-chapter').getAttribute('data-chapter-index');
    const bounds = async label => {
      const g = await page.evaluate(() => {
        const main = document.querySelector('.main-content');
        const player = document.querySelector('.kaze-interface > .player-bar,.press-deck');
        const r = player.getBoundingClientRect();
        return { bodyOverflow:document.documentElement.scrollWidth>innerWidth+1, mainOverflow:main.scrollWidth>main.clientWidth+1, player:{x:r.x,y:r.y,right:r.right,bottom:r.bottom}, w:innerWidth,h:innerHeight };
      });
      assert.equal(g.bodyOverflow,false,`${label}: page width ${JSON.stringify(g)}`);
      assert.equal(g.mainOverflow,false,`${label}: content width ${JSON.stringify(g)}`);
      assert.ok(g.player.x>=-1 && g.player.y>=-1 && g.player.right<=g.w+1 && g.player.bottom<=g.h+1,`${label}: player clipped ${JSON.stringify(g)}`);
    };
    await page.goto(process.env.KAZE_TEST_URL || 'http://127.0.0.1:5194');
    await page.locator('.home-flow').waitFor();
    const playback = await page.evaluate(() => JSON.stringify(window.__kazeFixture.playback));
    for(const [width,height] of sizes) {
      await page.setViewportSize({width,height});
      await nav('Discover Music');
      for(let index=0;index<4;index++) {
        await chapter(index);
        await page.waitForTimeout(100);
        await bounds(`${theme} ${width}x${height} chapter ${index}`);
        const geometry = await page.evaluate(() => {
          const root = document.querySelector('.discovery-chapters');
          const stage = root.querySelector('.chapter-stage');
          const owners = [root,stage];
          const owner = owners.find(e => e.scrollHeight>e.clientHeight+1 && /^(auto|scroll)$/.test(getComputedStyle(e).overflowY));
          const strips = [...root.querySelectorAll('.windowed-strip')].map(e=>({h:e.clientHeight,sh:e.scrollHeight}));
          return { overflow:!!owner, strips };
        });
        for(const strip of geometry.strips) assert.ok(strip.sh<=strip.h+2,`${theme} ${width}x${height} chapter ${index}: shelf clips its own content ${JSON.stringify(strip)}`);
        // Wheel over the heading must scroll the overflowing region without changing bins.
        if(geometry.overflow) {
          const heading = page.locator('.spatial-discovery-copy,.press-bin-heading').first();
          await heading.hover();
          await page.mouse.wheel(0,180);
          await page.waitForTimeout(220);
          assert.equal(await active(),String(index),'overflow wheel must not switch chapters');
          const scrolled = await page.locator('.discovery-chapters').evaluate(root=>Math.max(root.scrollTop,root.querySelector('.chapter-stage').scrollTop));
          assert.ok(scrolled>0,'heading wheel reaches the overflowing shelf');
          await page.locator('.discovery-chapters').evaluate(root=>{root.scrollTop=root.scrollHeight;const s=root.querySelector('.chapter-stage');s.scrollTop=s.scrollHeight;});
          await page.mouse.wheel(0,180);await page.waitForTimeout(220);
          assert.equal(await active(),String(index),'bottom boundary must not switch chapters');
          await page.mouse.wheel(0,-180);await page.waitForTimeout(220);
          assert.equal(await active(),String(index),'upward scrolling must not switch chapters');
          await page.locator('.discovery-chapters').focus();
          await page.keyboard.press('PageDown');
          assert.equal(await active(),String(index),'PageDown scrolls content when overflowing');
        }
        if(width===800&&height===600) await page.screenshot({path:`${out}/${theme}-discovery-${index}.png`});
      }
      for(const route of ['Home','Now Playing','Albums','Downloaded Tracks','Playlists','Queue','Settings','Profile']) {
        await nav(route);
        const surface = {'Home':'.home-flow','Now Playing':'.playback-stage,.press-listen','Albums':'.albums-view','Downloaded Tracks':'.library-view','Playlists':'.playlists-view','Queue':'.queue-view','Settings':'.settings-view','Profile':'.stats-view'}[route];
        await page.locator(surface).waitFor();
        await page.waitForTimeout(90);
        await bounds(`${theme} ${width}x${height} ${route}`);
        if(width===800&&height===600) await page.screenshot({path:`${out}/${theme}-${route.replaceAll(' ','-')}.png`});
      }
      assert.equal(await page.evaluate(()=>window.__kazeFixture.listeners.size),1,'one backend subscription through resizes');
      assert.equal(await page.evaluate(()=>JSON.stringify(window.__kazeFixture.playback)),playback,'resizing/navigation preserve playback');
      console.log(`PASS ${theme} ${width}x${height}: four chapters, overflow gestures, eight routes and playback continuity`);
    }
    await page.setViewportSize({width:1920,height:1080});await nav('Discover Music');await chapter(0);await page.waitForTimeout(120);
    await page.locator('.discovery-chapters').focus();await page.keyboard.press('PageDown');
    assert.equal(await active(),'1','PageDown switches chapters when the stage fits');
    await chapter(0);await page.locator('.spatial-discovery-copy,.press-bin-heading').first().hover();
    await page.mouse.wheel(0,100);await page.waitForTimeout(220);
    assert.equal(await active(),'1','wheel switches chapters when the stage fits');
    await chapter(0);
    await page.locator('.discovery-chapter .windowed-strip').evaluate(el=>el.scrollTo({left:235,behavior:'instant'}));
    await page.waitForTimeout(120);
    await chapter(1);await chapter(0);
    await page.waitForFunction(()=>Math.abs(document.querySelector('.discovery-chapter .windowed-strip').scrollLeft-235)<2);
    await page.setViewportSize({width:800,height:600});
    await nav('Downloaded Tracks');
    await page.locator('.library-view .track-row').first().waitFor();
    assert.ok(await page.locator('.library-view .track-row').count()<35,'large library stays windowed');
    await page.getByRole('button',{name:/Add to playlist|In playlist \(click to manage\)/}).first().click();
    await page.locator('.modal-content').waitFor();
    const dialog = await page.locator('.modal-content').boundingBox();
    assert.ok(dialog.x>=0 && dialog.y>=0 && dialog.x+dialog.width<=800 && dialog.y+dialog.height<=600,'playlist dialog fits the viewport');
    await page.getByRole('button',{name:'Done',exact:true}).click();
    await nav('Albums');
    await page.locator('.album-card,.press-record-spine').first().click();
    await page.locator('.library-view,.collection-view').waitFor();
    await page.locator('.track-row,.collection-track-row').first().waitFor();
    await bounds(`${theme} compact album collection`);
    for(const mode of ['long','empty','error','loading']) {
      await page.evaluate(mode=>sessionStorage.setItem('responsive.state',mode),mode);
      await page.reload();await page.locator('.home-flow').waitFor();
      await nav('Discover Music');await page.locator('.discovery-chapters').waitFor();
      if(mode==='loading') await page.getByText('Loading trending songs…',{exact:true}).waitFor();
      if(mode==='error') await page.getByText('Trending songs unavailable',{exact:true}).waitFor();
      if(mode==='empty') await page.getByText('No trending songs found',{exact:true}).waitFor();
      await bounds(`${theme} compact ${mode} state`);
      for(let index=0;index<4;index++) {await chapter(index);await bounds(`${theme} ${mode} chapter ${index}`);}
      await nav('Settings');await page.locator('.settings-view').waitFor();
      await bounds(`${theme} ${mode} settings remain reachable`);
    }
    console.log(`PASS ${theme}: compact dialog, large library/album collection, long metadata and empty/error/loading states`);
    assert.deepEqual(errors,[]);
    await context.close();
  }
} finally { await browser.close(); }

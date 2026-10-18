#!/usr/bin/env python3
"""Opt-in, disposable WebKitGTK frontend benchmark; no production instrumentation.

KAZE_PRIVATE_PERF_TEST=1 dbus-run-session -- python3 scripts/profile-frontend-native.py BINARY OUTPUT.json [--soak-seconds 1260]
Uses the shared controlled catalogue for repeatable UI pressure, actual Rust audio,
and the real tray. Run GUI suites sequentially. Does not use the user's profile.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import socket
import sqlite3
import subprocess
import sys
import tempfile
import threading
import time
import wave

sys.dont_write_bytecode = True
BASE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('tray_fixture', BASE/'test-tray-lifecycle.py')
tray = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tray)


def protocol(browser, method, params):
    browser.next_id += 1
    identity = browser.next_id
    event, values = threading.Event(), []
    browser.pending[identity] = (event, values)
    if method == 'Heap.snapshot': browser.ws.set_max_incoming_payload_size(128*1024*1024)
    message = {'id': identity, 'method': method, 'params': params}
    browser.ws.send_text(json.dumps({'id': identity+100000, 'method': 'Target.sendMessageToTarget', 'params': {'targetId': browser.target, 'message': json.dumps(message)}}))
    assert event.wait(30 if method == "Heap.snapshot" else 5), method
    del browser.pending[identity]
    assert 'error' not in values[0], values[0]
    return values[0].get('result')


def process_sample(root_pid):
    todo, result = [root_pid], {}
    while todo:
        pid = todo.pop()
        try:
            proc = Path(f'/proc/{pid}')
            fields = (proc/'stat').read_text().rsplit(')', 1)[1].split()
            todo.extend(int(value) for value in (proc/'task'/str(pid)/'children').read_text().split())
            memory = {line.split(':')[0]: int(line.split()[1]) for line in (proc/'smaps_rollup').read_text().splitlines() if line.split() and line.split()[0].endswith(':')}
            result[pid] = {'name': (proc/'comm').read_text().strip(), 'cpu': (int(fields[11])+int(fields[12]))/os.sysconf('SC_CLK_TCK'), 'private_kib': memory.get('Private_Clean', 0)+memory.get('Private_Dirty', 0)}
        except (OSError, ValueError):
            pass
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('binary'); parser.add_argument('output')
    parser.add_argument('--soak-seconds', type=int, default=0)
    parser.add_argument('--memory-diagnostic', action='store_true')
    args = parser.parse_args()
    assert os.environ.get('KAZE_PRIVATE_PERF_TEST') == '1', 'Use a private D-Bus session and explicit opt-in'
    output = Path(args.output); output.parent.mkdir(parents=True, exist_ok=True)
    report = {'target': 'native WebKitGTK', 'catalogue': 'controlled 1036 tracks / 130 albums; actual Rust audio', 'measurements': {}}
    watcher = tray.Watcher(); loop = tray.GLib.MainLoop()
    threading.Thread(target=loop.run, daemon=True).start()
    app = browser = None
    with tempfile.TemporaryDirectory(prefix='kaze-frontend-perf-') as directory:
        root = Path(directory); log = (root/'native.log').open('w+')
        def save(): output.write_text(json.dumps(report, indent=2))
        def launch():
            with socket.socket() as sock: sock.bind(('127.0.0.1', 0)); port = sock.getsockname()[1]
            env = dict(os.environ, XDG_DATA_HOME=str(root/'data'), XDG_CACHE_HOME=str(root/'cache'), WEBKIT_INSPECTOR_HTTP_SERVER=f'127.0.0.1:{port}', HTTP_PROXY='http://127.0.0.1:9', HTTPS_PROXY='http://127.0.0.1:9', NO_PROXY='127.0.0.1,localhost', no_proxy='127.0.0.1,localhost', NO_AT_BRIDGE='1', GIO_USE_VFS='local')
            proc = subprocess.Popen([str(Path(args.binary).resolve())], env=env, stdout=log, stderr=log)
            tray.wait_for(lambda: watcher.owner() and watcher.item(proc.pid), 'own native app', timeout=25)
            return proc, tray.Inspector(port)
        def click(label):
            expr = f"[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==={json.dumps(label)}||b.getAttribute('aria-label')==={json.dumps(label)})"
            tray.wait_for(lambda: browser.evaluate(f'Boolean({expr})'), f'button {label}')
            browser.evaluate(f'{expr}.click();true')
        def route(view):
            labels = {'home':'Home', 'discovery':'Discover Music', 'albums':'Library' if browser.evaluate('document.documentElement.dataset.interface')=='spatial' else 'Albums', 'library':'Library' if browser.evaluate('document.documentElement.dataset.interface')=='spatial' else 'Downloaded Tracks', 'now_playing':'Open Now Playing', 'queue':'Queue', 'settings':'Settings'}
            click(labels[view])
            if view == 'library' and browser.evaluate('document.documentElement.dataset.interface') == 'spatial': click('Tracks')
            tray.wait_for(lambda: browser.evaluate("document.querySelector('.main-content').dataset.view") == view, view)
        def style(value):
            route('settings'); browser.evaluate(f"[...document.querySelectorAll('.interface-options button')].find(b=>b.querySelector('strong').textContent==={json.dumps(value.title())}).click();true")
            tray.wait_for(lambda: browser.evaluate('document.documentElement.dataset.interface') == value, value)
            tray.wait_for(lambda: browser.evaluate("!document.querySelector('.interface-options button:disabled')"), 'theme save')
        def snap(name, inventory=False):
            report['measurements'][name] = browser.evaluate(f'window.__kazePerf.snapshot({str(inventory).lower()})')
            report['measurements'][name]['processes'] = process_sample(app.pid)
            save(); print(f'MEASURE {name}: '+json.dumps({key: report['measurements'][name].get(key) for key in ['commits','dom','images','visibility']}), flush=True)
        def period(name, seconds=5):
            browser.evaluate('window.__kazePerf.reset();true')
            before = process_sample(app.pid); started = time.monotonic(); time.sleep(seconds); elapsed = time.monotonic()-started
            snap(name)
            after = report['measurements'][name]['processes']
            report['measurements'][name]['cpu_percent_one_core'] = round(100*sum(row['cpu']-before.get(pid, row)['cpu'] for pid,row in after.items())/elapsed, 2)
            report['measurements'][name]['renderer_cpu_percent'] = round(100*sum(row['cpu']-before.get(pid, row)['cpu'] for pid,row in after.items() if 'WebKitWeb' in row['name'])/elapsed, 2)
            save()
        def frames(name, action='true'):
            browser.evaluate('window.__kazePerf.reset();window.__kazePerf.frames(2500);true')
            browser.evaluate(action)
            tray.wait_for(lambda: browser.evaluate('window.__kazePerf.frameResult'), name, timeout=7)
            snap(name, True); report['measurements'][name]['frames'] = browser.evaluate('window.__kazePerf.frameResult'); save()
        try:
            app,browser=launch(); browser.close(); browser=None; watcher.menu(app.pid,'Quit Kaze');app.wait(timeout=15)
            tray.wait_for(lambda: not watcher.owner(), 'warmup exit')
            database=root/'data/music-player/music_player.db'
            with sqlite3.connect(database) as db:
                db.execute("INSERT OR REPLACE INTO application_settings (key,value,updated_at) VALUES ('onboarding_completed','true',1)")
                path=root/'audio.wav'
                with wave.open(str(path),'wb') as audio:
                    audio.setnchannels(1);audio.setsampwidth(2);audio.setframerate(8000)
                    for _ in range(1800): audio.writeframesraw(b'\x00\x00'*8000)
                for i in range(4):
                    link=root/f'audio-{i}.wav';os.link(path,link)
                    db.execute("INSERT INTO tracks (id,file_path,file_size,modified_timestamp,title,normalized_title,duration_secs,format,created_at,updated_at) VALUES (?,?,?,1,?,?,1800,'wav',1,1)",(f't{i}',str(link),link.stat().st_size,f'Performance Track {i}',f'performance track {i}'))
            app,browser=launch()
            fixture=(BASE.parent/'tests/fixtures/dualInterfaceBackend.mjs').read_text().replace('export function installDualInterfaceFixture()', 'function installDualInterfaceFixture()')
            bootstrap=(BASE/'performance-probe.js').read_text()+'\n'+fixture+'''
installDualInterfaceFixture();
// Keep unrelated desktop input out of this disposable benchmark window.
for (const type of ['click','keydown','wheel']) document.addEventListener(type,event=>{
  if(event.isTrusted){event.preventDefault();event.stopImmediatePropagation();}
}, {capture:true,passive:false});
const define = Object.defineProperty;
Object.defineProperty = function(target,key,descriptor) {
  if (target===window.__TAURI_INTERNALS__ && key==='invoke' && typeof descriptor.value==='function') {
    const invoke=descriptor.value;
    descriptor={...descriptor,value:function(method,args,...rest) {
      if(method==='plugin:event|listen'&&args.event==='backend-event') window.__perfEmit=payload=>window.__TAURI_INTERNALS__.runCallback(args.handler,{event:'backend-event',id:0,payload});
      const name=args?.query?.query||args?.command?.command||method;
      const count=window.__kazePerf.counters.ipc; count[name]=(count[name]||0)+1;
      if (method==='execute_query' && args.query.query!=='GetPlaybackState') return window.__kazeFixture.query(args.query);
      if (method==='execute_command' && args.command.command==='SetInterfaceStyle') return window.__kazeFixture.command(args.command);
      return invoke.call(this,method,args,...rest);
    }};
    Object.defineProperty=define;
  }
  return define.call(Object,target,key,descriptor);
};
'''
            protocol(browser,'Page.enable',{});protocol(browser,'Page.setBootstrapScript',{'source':bootstrap})
            browser.evaluate('location.reload();true')
            tray.wait_for(lambda: browser.evaluate("Boolean(window.__kazePerf&&window.__kazeFixture&&document.querySelector('.home-flow'))"), 'instrumented real App', timeout=20)
            tray.wait_for(lambda:browser.evaluate('window.__kazeFixture.queries.length>10'), 'controlled catalogue loaded before measuring')
            browser.evaluate('window.__kazePerf.attach();true');time.sleep(2)
            if args.memory_diagnostic:
                protocol(browser,'Heap.enable',{})
                def heap(name):
                    protocol(browser,'Heap.gc',{})
                    raw=protocol(browser,'Heap.snapshot',{})['snapshotData']
                    data=json.loads(raw);nodes=data['nodes'];names=data['nodeClassNames'];classes={}
                    assert data['version']==3
                    for i in range(0,len(nodes),4):
                        label=names[nodes[i+2]]
                        count,size=classes.get(label,(0,0));classes[label]=(count+1,size+nodes[i+1])
                    snap(name)
                    report.setdefault('heap',{})[name]={'nodes':len(nodes)//4,'live_bytes':sum(nodes[1::4]),'classes':classes,'snapshot_bytes':len(raw)}
                    save();print('HEAP '+name+' '+json.dumps({k:report['heap'][name][k] for k in ['nodes','live_bytes','snapshot_bytes']}),flush=True)
                def burst(name):
                    browser.evaluate("window.__perfBurstDone=false;let ticks=0;const timer=setInterval(()=>{window.__perfEmit({event:'PlaybackPositionChanged',payload:{position_secs:(++ticks*.25)%1700,duration_secs:1800}});if(ticks===5000){clearInterval(timer);window.__perfBurstDone=true}},10);true")
                    tray.wait_for(lambda:browser.evaluate('window.__perfBurstDone'),name,timeout=90)
                heap('warm_heap')
                burst('profiled tick burst');heap('profiled_5000_ticks')
                browser.evaluate('window.__REACT_DEVTOOLS_GLOBAL_HOOK__.onCommitFiberRoot=()=>{};window.__kazePerf.pauseMutationCounters();true')
                burst('unprofiled tick burst');heap('unprofiled_5000_ticks')
                browser.evaluate("const style=document.createElement('style');style.textContent='.kaze-interface input[aria-label=Seek]{background:var(--border)!important}';document.head.append(style);true")
                burst('static track tick burst');heap('static_track_5000_ticks')
                protocol(browser,'Heap.disable',{})
                report['passed']=True;save();print('PASS memory diagnostics',flush=True)
                return
            snap('initial',True); period('spatial_idle')
            browser.command('PlayTrack',track_id='t0',source='library')
            tray.wait_for(lambda: watcher.status()=='Playing','real audio')
            time.sleep(1);period('spatial_home_playing');frames('spatial_home_frames')
            route('discovery');time.sleep(1);snap('spatial_discovery',True)
            frames('discovery_chapters',"""(() => {let n=0;window.perfChapter=setInterval(()=>{document.querySelectorAll('.chapter-nav button')[++n%4]?.click();if(n===16)clearInterval(window.perfChapter)},100);return true;})()""")
            browser.evaluate("document.querySelectorAll('.chapter-nav button')[0].click();true");time.sleep(.3)
            frames('discovery_horizontal',"""(() => {const el=document.querySelector('.discovery-chapter:not([hidden]) .horizontal-scroll-row');let n=0;window.perfScroll=setInterval(()=>{el.scrollLeft=(++n*9)%500;if(n===100)clearInterval(window.perfScroll)},16);return true;})()""")
            route('now_playing');time.sleep(.3)
            frames('spatial_track_changes', """(() => {let n=0;const timer=setInterval(()=>{window.__TAURI_INTERNALS__.invoke('execute_command',{command:{command:'PlayTrack',payload:{track_id:'t'+(++n%4),source:'library'}}});if(n===16)clearInterval(timer)},100);return true;})()""")
            browser.command('PlayTrack',track_id='t0',source='library')
            route('albums');time.sleep(.5);snap('spatial_albums',True)
            route('settings'); time.sleep(.3);snap('before_theme_cycles')
            timings=[]
            for i in range(20):
                started=time.monotonic();style('brutalist' if i%2==0 else 'spatial');timings.append((time.monotonic()-started)*1000)
            report['theme_switch_ms']=timings;snap('after_theme_cycles')
            for i in range(6):
                for view in ['home','discovery','albums']: route(view);time.sleep(.15)
                browser.evaluate("document.querySelector('.album-card').click();true")
                tray.wait_for(lambda: browser.evaluate("document.querySelector('.main-content').dataset.view==='library'"),'album detail')
            route('settings');time.sleep(1);snap('after_navigation_cycles')
            style('brutalist');route('library');time.sleep(1);snap('brutalist_library',True)
            frames('brutalist_library_scroll',"""(() => {const el=[...document.querySelectorAll('.library-view div')].find(el=>el.scrollHeight>el.clientHeight+100 && getComputedStyle(el).overflowY==='auto');if(!el)throw Error('missing virtual track list');let n=0;window.perfList=setInterval(()=>{el.scrollTop=++n*40;if(n===100)clearInterval(window.perfList)},16);return true;})()""")
            route('home');time.sleep(1);period('brutalist_home_playing')
            style('spatial');route('home');time.sleep(.5)
            before_playback=browser.query('GetPlaybackState');browser.evaluate('window.__kazePerf.reset();true')
            window=tray.wait_for(lambda:tray.windows(app.pid),'own visible window')[0]
            subprocess.run(['niri','msg','action','close-window','--id',str(window['id'])],check=True,capture_output=True)
            tray.wait_for(lambda:not tray.windows(app.pid),'own hidden window')
            period('hidden_one_minute',60)
            if os.environ.get('KAZE_ASSERT_PERFORMANCE')=='1':
                assert report['measurements']['hidden_one_minute']['commits']==0, 'hidden visual commits must stop'
            assert watcher.status()=='Playing'; assert browser.query('GetPlaybackState')['position_secs']>before_playback['position_secs']+50
            started=time.monotonic();watcher.menu(app.pid,'Open Kaze');tray.wait_for(lambda:tray.windows(app.pid),'restore')
            browser.evaluate('window.__kazePerf.frames(1000);true');tray.wait_for(lambda:browser.evaluate('window.__kazePerf.frameResult'),'restore frames')
            report['restore_ms']=(time.monotonic()-started)*1000; snap('restored',True)
            if args.soak_seconds:
                started=time.monotonic();report['soak_start']=browser.query('GetPlaybackState');snap('soak_start')
                while time.monotonic()-started<args.soak_seconds:
                    time.sleep(max(0,min(30,args.soak_seconds-(time.monotonic()-started))))
                    if args.soak_seconds/2 < time.monotonic()-started < args.soak_seconds/2+31:
                        style('brutalist');route('home')
                    assert app.poll() is None and watcher.status()=='Playing'
                    report['soak_elapsed']=round(time.monotonic()-started,1);save();print(f'SOAK {report["soak_elapsed"]}s / {args.soak_seconds}s',flush=True)
                style('spatial');route('home');time.sleep(1);snap('soak_end',True);report['soak_end']=browser.query('GetPlaybackState');frames('soak_end_frames')
            browser.command('Pause');tray.wait_for(lambda:watcher.status()=='Paused','actual pause');period('final_paused')
            report['passed']=True;save(); print('PASS native performance scenarios',flush=True)
        except Exception:
            report['failed']=True;save();log.flush();print((root/'native.log').read_text()[-4000:],file=sys.stderr);raise
        finally:
            if browser:browser.close()
            if app and app.poll() is None:
                try:watcher.menu(app.pid,'Quit Kaze');app.wait(timeout=15)
                except Exception:app.terminate();app.wait(timeout=8)
            log.close();loop.quit()

if __name__=='__main__':main()

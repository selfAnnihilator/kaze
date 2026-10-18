#!/usr/bin/env python3
"""Opt-in Queue-to-Home frame regression on native WebKitGTK/Niri.

KAZE_PRIVATE_QUEUE_TEST=1 dbus-run-session -- python3 scripts/test-queue-frames-native.py \
    /absolute/path/to/embedded/kaze --artwork-database /path/to/music_player.db \
    --output /tmp/kaze-queue-frames.json

Requires Python GI, Niri and wtype. Reads at most 21 local-track metadata rows
from the explicitly supplied database, read-only. Writes only disposable XDG
state; artwork uses real Rust extraction, other IPC and playback use fixtures.
Run GUI suites sequentially. Frame progress must be asserted while focused:
DOM navigation and responsive JavaScript alone miss a frozen native surface.
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

sys.dont_write_bytecode = True
BASE = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('native_perf', BASE / 'scripts/profile-frontend-native.py')
perf = importlib.util.module_from_spec(spec)
spec.loader.exec_module(perf)
tray = perf.tray


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary')
    parser.add_argument('--artwork-database', required=True, type=Path)
    parser.add_argument('--rounds', default=6, type=int)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    if os.environ.get('KAZE_PRIVATE_QUEUE_TEST') != '1':
        raise SystemExit('Use explicit opt-in and a private dbus-run-session')
    assert 1 <= args.rounds <= 100
    # Only catalogue metadata is read, never account/session tables.
    with sqlite3.connect(args.artwork_database.resolve().as_uri() + '?mode=ro', uri=True) as source:
        songs = source.execute('''
            SELECT id,file_path,file_size,modified_timestamp,title,normalized_title,
                   duration_secs,format,has_cover_art,created_at,updated_at
            FROM tracks WHERE format != 'online' AND has_cover_art = 1 LIMIT 21
        ''').fetchall()
    assert len(songs) >= 12, 'Supply a library with at least twelve local tracks with artwork'
    tracks = [dict(id=row[0], title=row[4], file_path=row[1], duration_secs=row[6],
                   format=row[7], has_cover_art=row[8]) for row in songs]
    report = {'passed': False, 'target': 'native WebKitGTK', 'rounds': [], 'tracks': len(tracks)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    watcher = tray.Watcher()
    loop = tray.GLib.MainLoop()
    threading.Thread(target=loop.run, daemon=True).start()
    app = browser = None
    with tempfile.TemporaryDirectory(prefix='kaze-queue-frames-') as directory:
        root = Path(directory)
        with (root / 'native.log').open('w+') as log:
            try:
                with socket.socket() as sock:
                    sock.bind(('127.0.0.1', 0))
                    port = sock.getsockname()[1]
                env = dict(os.environ, XDG_DATA_HOME=str(root / 'data'),
                           XDG_CACHE_HOME=str(root / 'cache'), XDG_CONFIG_HOME=str(root / 'config'),
                           WEBKIT_INSPECTOR_HTTP_SERVER=f'127.0.0.1:{port}',
                           HTTP_PROXY='http://127.0.0.1:9', HTTPS_PROXY='http://127.0.0.1:9',
                           NO_PROXY='127.0.0.1,localhost', no_proxy='127.0.0.1,localhost',
                           NO_AT_BRIDGE='1', GIO_USE_VFS='local')
                app = subprocess.Popen([str(Path(args.binary).resolve())], env=env, stdout=log, stderr=log)
                tray.wait_for(lambda: watcher.owner() and watcher.item(app.pid), 'own private application', timeout=30)
                browser = tray.Inspector(port)
                tray.wait_for(lambda: browser.evaluate("location.origin==='tauri://localhost' && document.body.innerText.length>0"),
                              'embedded initial page', timeout=30)
                with sqlite3.connect(root / 'data/music-player/music_player.db') as database:
                    database.executemany('''
                        INSERT OR REPLACE INTO tracks
                        (id,file_path,file_size,modified_timestamp,title,normalized_title,
                         duration_secs,format,has_cover_art,created_at,updated_at)
                        VALUES (?,?,?,?,?,?,?,?,?,?,?)
                    ''', songs)
                fixture = (BASE / 'tests/fixtures/dualInterfaceBackend.mjs').read_text().replace(
                    'export function installDualInterfaceFixture()', 'function installDualInterfaceFixture()')
                bootstrap = fixture + '\nwindow.__actualSongs=' + json.dumps(tracks) + ';\n' + '''
installDualInterfaceFixture();
window.__kazeFixture.tracks.splice(0,window.__kazeFixture.tracks.length,...window.__actualSongs);
Object.assign(window.__kazeFixture.playback,{current_track:window.__actualSongs[0],current_track_id:window.__actualSongs[0].id,queue_track_ids:window.__actualSongs.slice(1).map(t=>t.id)});
window.__queueErrors=[];window.__nativeCoverReads=0;
window.addEventListener('error',e=>window.__queueErrors.push(e.message));
const define=Object.defineProperty;
Object.defineProperty=function(target,key,descriptor){
 if(target===window.__TAURI_INTERNALS__&&key==='invoke'&&typeof descriptor.value==='function'){
  const invoke=descriptor.value;
  descriptor={...descriptor,value:function(method,args,...rest){
   if(method==='execute_query'){
    if(args.query.query!=='GetTrackCoverArt')return window.__kazeFixture.query(args.query);
    window.__nativeCoverReads++;
   }
   if(method==='execute_command')return window.__kazeFixture.command(args.command);
   if(method==='plugin:event|listen'&&args.event==='backend-event')window.__kazeFixture.listeners.add(payload=>window.__TAURI_INTERNALS__.runCallback(args.handler,{event:'backend-event',id:0,payload}));
   return invoke.call(this,method,args,...rest);
  }};
  Object.defineProperty=define;
 }
 return define.call(this,target,key,descriptor);
};
'''
                perf.protocol(browser, 'Page.enable', {})
                perf.protocol(browser, 'Page.setBootstrapScript', {'source': bootstrap})
                browser.evaluate('location.reload();true')
                tray.wait_for(lambda: browser.evaluate("Boolean(document.querySelector('.home-flow'))"), 'fixture Home', timeout=25)
                browser.invoke('plugin:window|set_fullscreen', {'value': True})
                tray.wait_for(lambda: browser.evaluate('innerWidth>=1440'), 'full desktop viewport')
                browser.evaluate("window.__queueTick=0;window.__queueTimer=setInterval(()=>{window.__queueTick++;window.__kazeFixture.emit({event:'PlaybackPositionChanged',payload:{position_secs:window.__queueTick*.25,duration_secs:240}})},10);true")

                def focus():
                    windows = json.loads(subprocess.check_output(['niri', 'msg', '-j', 'windows'], text=True))
                    own = next(window for window in windows if window.get('pid') == app.pid)
                    subprocess.run(['niri', 'msg', 'action', 'focus-window', '--id', str(own['id'])],
                                   stdout=subprocess.DEVNULL, check=True)
                    tray.wait_for(lambda: browser.evaluate('document.hasFocus() && document.visibilityState===\"visible\"'), 'focused visible window')

                for iteration in range(args.rounds):
                    for label in ['Queue', 'Home']:
                        scroll_top = None
                        if label == 'Home':
                            focus()
                            browser.evaluate("const q=document.querySelector('[aria-label=\"Upcoming queue\"]');q.tabIndex=0;q.focus();true")
                            time.sleep(.1)
                            # WebKit's PageDown targets the outer page even with the
                            # nested list focused. Exercise that native input, then
                            # explicitly scroll the inner viewport too.
                            subprocess.run(['wtype', '-k', 'Page_Down'], check=True)
                            browser.evaluate("const q=document.querySelector('[aria-label=\"Upcoming queue\"]');q.scrollTop=Math.min(400,q.scrollHeight-q.clientHeight);true")
                            time.sleep(.15)
                            scroll_top = browser.evaluate("document.querySelector('[aria-label=\"Upcoming queue\"]').scrollTop")
                            assert scroll_top > 0, 'Queue viewport did not scroll'
                        focus()
                        browser.evaluate("window.__drawCount=0;window.__drawProbe=requestAnimationFrame(function tick(){window.__drawCount++;if(window.__drawCount<60)window.__drawProbe=requestAnimationFrame(tick)});true")
                        browser.evaluate(f"[...document.querySelectorAll('button')].find(b=>b.getAttribute('aria-label')==={json.dumps(label)}||b.textContent.trim()==={json.dumps(label)}).click();true")
                        time.sleep(1)
                        frames = browser.evaluate('window.__drawCount')
                        browser.evaluate('cancelAnimationFrame(window.__drawProbe);true')
                        state = browser.evaluate("({view:document.querySelector('.main-content').dataset.view,errors:window.__queueErrors,coverReads:window.__nativeCoverReads,rows:document.querySelectorAll('.queue-entry').length})")
                        report['rounds'].append(dict(iteration=iteration, destination=label, frames=frames, scroll_top=scroll_top, **state))
                        print(f'FRAMES {iteration} {label}: {frames}', flush=True)
                        assert state['view'] == label.lower(), state
                        assert frames > 5, f'Frame delivery stalled on {label}: {frames}'
                        assert not state['errors'], state['errors']
                assert state['coverReads'] > 0, 'Real Rust artwork extraction was not exercised'
                report['passed'] = True
                print('PASS native Queue scrolling and Home frame delivery', flush=True)
            finally:
                args.output.write_text(json.dumps(report, indent=2))
                if browser:
                    browser.close()
                if app and app.poll() is None:
                    try:
                        watcher.menu(app.pid, 'Quit Kaze')
                        app.wait(timeout=15)
                    except Exception:
                        # This PID belongs only to the disposable test process.
                        app.terminate()
                        app.wait(timeout=5)


if __name__ == '__main__':
    main()

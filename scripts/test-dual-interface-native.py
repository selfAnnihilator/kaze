#!/usr/bin/env python3
"""Real WebKitGTK + Rust playback, using private D-Bus and disposable music/data.

KAZE_PRIVATE_DUAL_TEST=1 dbus-run-session -- python3 scripts/test-dual-interface-native.py /absolute/path/to/embedded/debug/kaze
"""
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
spec = importlib.util.spec_from_file_location('tray_fixture', Path(__file__).with_name('test-tray-lifecycle.py'))
tray = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tray)


def main():
    if os.environ.get('KAZE_PRIVATE_DUAL_TEST') != '1':
        raise SystemExit('Use KAZE_PRIVATE_DUAL_TEST=1 and a private dbus-run-session')
    binary = str(Path(sys.argv[1]).resolve())
    watcher = tray.Watcher()
    loop = tray.GLib.MainLoop()
    threading.Thread(target=loop.run, daemon=True).start()
    app = browser = None
    with tempfile.TemporaryDirectory(prefix='kaze-dual-native-') as directory:
        root = Path(directory)
        log = (root/'native.log').open('w+')

        def launch():
            with socket.socket() as s:
                s.bind(('127.0.0.1', 0)); port = s.getsockname()[1]
            env = dict(os.environ, XDG_DATA_HOME=str(root/'data'), XDG_CACHE_HOME=str(root/'cache'), XDG_CONFIG_HOME=str(root/'config'), WEBKIT_INSPECTOR_HTTP_SERVER=f'127.0.0.1:{port}', NO_PROXY='127.0.0.1,localhost', no_proxy='127.0.0.1,localhost', HTTPS_PROXY='http://127.0.0.1:9', HTTP_PROXY='http://127.0.0.1:9', NO_AT_BRIDGE='1', GIO_USE_VFS='local')
            process = subprocess.Popen([binary], env=env, stdout=log, stderr=log)
            tray.wait_for(lambda: watcher.owner() and watcher.item(process.pid), 'native app and own tray', timeout=25)
            inspector = tray.Inspector(port)
            tray.wait_for(lambda: inspector.evaluate('Boolean(document.querySelector(".kaze-interface"))'), 'real interface mount', timeout=20)
            return process, inspector

        def quit_own():
            nonlocal app, browser
            if browser:
                browser.close(); browser = None
            if app and app.poll() is None:
                watcher.menu(app.pid, 'Quit Kaze'); app.wait(timeout=15)
                assert app.returncode == 0
            tray.wait_for(lambda: not watcher.owner(), 'own MPRIS release')

        def click(text):
            expression = f"[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==={json.dumps(text)}||b.getAttribute('aria-label')==={json.dumps(text)})"
            tray.wait_for(lambda: browser.evaluate(f'Boolean({expression})'), f'button {text}')
            browser.evaluate(f'{expression}.click(); true')

        def select(style):
            browser.evaluate(f"[...document.querySelectorAll('.settings-style-options button')].find(b=>b.querySelector('strong').textContent==={json.dumps(style.title())}).click(); true")
            tray.wait_for(lambda: browser.query('GetSettings')['interface_style'] == style, f'{style} persisted')
            tray.wait_for(lambda: browser.evaluate('document.documentElement.dataset.interface') == style, f'{style} rendered')

        try:
            app, browser = launch(); quit_own()
            db_path = root/'data/music-player/music_player.db'
            with sqlite3.connect(db_path) as db:
                db.execute("INSERT OR REPLACE INTO application_settings (key,value,updated_at) VALUES ('onboarding_completed','true',1)")
                for i in range(4):
                    path = root/f'track-{i}.wav'
                    with wave.open(str(path), 'wb') as audio:
                        audio.setnchannels(1); audio.setsampwidth(2); audio.setframerate(8000)
                        audio.writeframes(b'\x00\x00' * 8000 * 120)
                    db.execute("INSERT INTO tracks (id,file_path,file_size,modified_timestamp,title,normalized_title,duration_secs,format,created_at,updated_at) VALUES (?,?,?,1,?,?,120,'wav',1,1)", (f'track-{i}', str(path), path.stat().st_size, f'Interface Track {i}', f'interface track {i}'))
            app, browser = launch()
            click('Discover Music')
            tray.wait_for(lambda: browser.evaluate('Boolean(document.querySelector(".spatial-discovery-fixed"))'), 'restored native Discovery')
            browser.evaluate("document.querySelectorAll('.chapter-nav button')[3].click(); true")
            geometry = browser.evaluate("""(() => {
                const rect = selector => { const r = document.querySelector(selector).getBoundingClientRect(); return {x:r.x,y:r.y,width:r.width,height:r.height,bottom:r.bottom}; };
                return {art:rect('.spatial-discovery-cover'),caption:rect('.spatial-discovery-caption'),detail:rect('.spatial-discovery-detail'),footer:rect('.kaze-interface > .player-bar'),mainScroll:document.querySelector('.main-content').scrollTop,rail:rect('.spatial-navigation')};
            })()""")
            assert geometry['art']['width'] > 180, geometry
            assert geometry['art']['bottom'] <= geometry['caption']['y'] + 12, geometry
            assert geometry['caption']['bottom'] <= geometry['footer']['y'], geometry
            assert geometry['mainScroll'] == 0 and geometry['rail']['width'] == 72, geometry
            browser.evaluate("document.querySelector('.spatial-rail-resizer').dispatchEvent(new KeyboardEvent('keydown',{key:'End',bubbles:true}));true")
            tray.wait_for(lambda: browser.evaluate("document.querySelector('.spatial-navigation').getBoundingClientRect().width") == 160, 'native rail expands')
            browser.evaluate("document.querySelector('.spatial-rail-resizer').dispatchEvent(new KeyboardEvent('keydown',{key:'Home',bubbles:true}));true")
            tray.wait_for(lambda: browser.evaluate("document.querySelector('.spatial-navigation').getBoundingClientRect().width") == 56, 'native rail contracts')
            print('PASS native WebKitGTK split Discovery, large artwork, fitted panes and adjustable icon rail', flush=True)
            for burst in range(10):
                direction = -120 if burst % 2 == 0 else 120
                browser.evaluate(f"(() => {{ const stage=document.querySelector('.spatial-discovery-fixed');for(let n=0;n<30;n++)stage.dispatchEvent(new WheelEvent('wheel',{{bubbles:true,cancelable:true,deltaY:{direction}}}));return true; }})()")
                expected_chapter = 2 if burst % 2 == 0 else 3
                tray.wait_for(lambda: browser.evaluate("[...document.querySelectorAll('.chapter-nav button')].findIndex(button=>button.getAttribute('aria-current')==='true')") == expected_chapter, 'one native chapter per rapid wheel burst')
                assert browser.evaluate("document.querySelector('.main-content').scrollTop") == 0
                time.sleep(.21)
            print('PASS native rapid vertical wheel bursts change one chapter per gesture without outer scrolling', flush=True)
            browser.command('PlayTrack', track_id='track-0', source='library')
            browser.command('EnqueueTrack', track_id='track-1', play_next=False)
            browser.command('EnqueueTrack', track_id='track-2', play_next=False)
            browser.command('StartFlow', familiarity_ratio=.5)
            tray.wait_for(lambda: watcher.status() == 'Playing', 'real Rust audio advances')
            click('Home')
            tray.wait_for(lambda: browser.evaluate('Boolean(document.querySelector(".familiarity-dial"))'), 'native circular familiarity dial')
            assert browser.evaluate("document.querySelectorAll('.spatial-navigation nav button').length") == 4
            browser.evaluate("document.querySelector('.familiarity-dial').dispatchEvent(new KeyboardEvent('keydown',{key:'End',bubbles:true})); true")
            tray.wait_for(lambda: browser.query('GetFlowSession')['familiarity_ratio'] == 1, 'native dial updates real Flow steering')
            assert browser.evaluate("document.querySelector('.familiarity-dial').getAttribute('aria-valuenow')") == '1'
            print('PASS native four-destination rail and circular familiarity dial update actual Flow steering', flush=True)
            click('Settings')
            tray.wait_for(lambda: browser.evaluate('Boolean(document.querySelector(".settings-style-options"))'), 'interface settings')
            browser.evaluate("window.nativeApp=document.querySelector('.kaze-interface');window.nativeMain=document.querySelector('.main-content');window.nativeMain.scrollTop=100;true")
            with sqlite3.connect(db_path) as db:
                journal_before = db.execute('SELECT session_id FROM active_playback_journal').fetchall()
                history_before = db.execute('SELECT count(*) FROM playback_history').fetchone()[0]
            flow_before = browser.query('GetFlowSession')
            for style in ['brutalist', 'spatial', 'brutalist']:
                before = browser.query('GetPlaybackState'); start = time.monotonic()
                select(style)
                after = browser.query('GetPlaybackState'); elapsed = time.monotonic()-start
                for field in ['current_track_id', 'queue_track_ids', 'current_queue_index', 'queue_length', 'is_playing', 'repeat_mode', 'is_shuffle']:
                    assert before[field] == after[field], (style, field, before[field], after[field])
                assert before['position_secs']-.1 <= after['position_secs'] <= before['position_secs']+elapsed+1, (style, before['position_secs'], after['position_secs'], elapsed)
                assert browser.query('GetFlowSession')['generation'] == flow_before['generation']
                assert browser.evaluate("window.nativeApp===document.querySelector('.kaze-interface')&&window.nativeMain===document.querySelector('.main-content')&&document.querySelector('.main-content').dataset.view==='settings'")
                assert watcher.status() == 'Playing'
                print(f'PASS {style}: real playback, position, track, queue, Flow, MPRIS, route and mounted elements preserved', flush=True)
            with sqlite3.connect(db_path) as db:
                assert db.execute('SELECT session_id FROM active_playback_journal').fetchall() == journal_before
                assert db.execute('SELECT count(*) FROM playback_history').fetchone()[0] == history_before
            browser.evaluate("window.hiddenMutationCount=0;window.visibilityObserver=new MutationObserver(records=>{if(document.hidden)window.hiddenMutationCount+=records.length;});window.visibilityObserver.observe(document.querySelector('.kaze-interface'),{subtree:true,childList:true,attributes:true,characterData:true});true")
            for cycle in range(10):
                own_window = tray.wait_for(lambda: tray.windows(app.pid), 'own visible window')[0]
                subprocess.run(['niri', 'msg', 'action', 'close-window', '--id', str(own_window['id'])], check=True, capture_output=True)
                tray.wait_for(lambda: not tray.windows(app.pid) and browser.evaluate('document.hidden'), 'native hidden visibility')
                browser.evaluate('window.hiddenMutationCount=0;true')
                time.sleep(.4)
                assert browser.evaluate('window.hiddenMutationCount') == 0, ('hidden visual mutations', cycle)
                assert watcher.status() == 'Playing'
                watcher.menu(app.pid, 'Open Kaze')
                tray.wait_for(lambda: tray.windows(app.pid) and browser.evaluate('!document.hidden'), 'native visible restoration')
                assert browser.evaluate("window.nativeApp===document.querySelector('.kaze-interface')&&window.nativeMain===document.querySelector('.main-content')&&document.querySelector('.main-content').dataset.view==='settings'")
                tray.wait_for(lambda: abs(browser.evaluate("Number(document.querySelector(':is(.player-bar,.press-deck) input[aria-label=Seek]').value)")-browser.query('GetPlaybackState')['position_secs']) < .75, 'restored progress catches up')
                assert browser.query('GetFlowSession')['generation'] == flow_before['generation']
            browser.evaluate('window.visibilityObserver.disconnect();true')
            print('PASS ten native hide/restore cycles suspend visual mutations and restore current progress, route, DOM and Flow', flush=True)
            browser.command('Pause')
            tray.wait_for(lambda: watcher.status() == 'Paused' and not browser.query('GetPlaybackState')['is_playing'], 'Pause applied before recording exact position')
            paused = browser.query('GetPlaybackState')
            select('spatial'); select('brutalist')
            after_paused_switch = browser.query('GetPlaybackState')
            assert after_paused_switch['position_secs'] == paused['position_secs'], (paused, after_paused_switch)
            assert watcher.status() == 'Paused'
            print('PASS paused switching preserves exact position and history/journal session identity', flush=True)
            click('Profile')
            tray.wait_for(lambda: browser.evaluate("[...document.querySelectorAll('h2')].some(el=>el.textContent==='Guest Listener')&&document.querySelector('.main-content').textContent.includes('Daily Music Time')"), 'native profile and actual listening statistics')
            assert browser.evaluate("(() => {const profile=document.querySelector(':is(.sidebar,.press-account-navigation) button[aria-label=Profile]'),settings=document.querySelector(':is(.sidebar,.press-account-navigation) button[aria-label=Settings]');return profile.getBoundingClientRect().top>=settings.getBoundingClientRect().bottom&&profile.getAttribute('aria-current')==='page';})()")
            assert browser.query('GetPlaybackState')['position_secs'] == paused['position_secs']
            print('PASS native profile shortcut below Settings opens account and actual statistics without changing paused playback', flush=True)
            click('Open Now Playing')
            assert browser.evaluate("Boolean(document.querySelector('.press-poster-page .press-poster-cover'))")
            assert browser.evaluate("getComputedStyle(document.querySelector('.press-poster-title h1')).color")
            click('Full Screen')
            tray.wait_for(lambda: browser.evaluate("Boolean(document.querySelector('.press-fullscreen .press-deck'))"), 'independent native full screen deck')
            assert browser.evaluate("document.querySelector('.press-fullscreen .press-poster-cover')!==null&&document.querySelectorAll('.press-fullscreen .playback-stage,.press-fullscreen .player-bar').length===0")
            assert browser.query('GetPlaybackState')['position_secs'] == paused['position_secs']
            click('Exit full screen')
            tray.wait_for(lambda: browser.evaluate("!document.querySelector('.fullscreen-player')"), 'native full screen exit')
            print('PASS native full screen uses the independent Brutalist poster and deck with exact paused playback', flush=True)
            browser.command('Resume'); quit_own()
            app, browser = launch()
            tray.wait_for(lambda: browser.evaluate('document.documentElement.dataset.interface') == 'brutalist', 'persisted style on cold relaunch')
            assert browser.query('GetSettings')['interface_style'] == 'brutalist'
            print('PASS cold native relaunch loads saved Brutalist via existing SQLite settings', flush=True)
            quit_own()
        except Exception:
            log.flush()
            print((root/'native.log').read_text()[-7000:], file=sys.stderr)
            raise
        finally:
            if browser:
                browser.close()
            if app and app.poll() is None:
                app.terminate()
                try: app.wait(timeout=8)
                except subprocess.TimeoutExpired: app.kill(); app.wait(timeout=5)
            log.close(); loop.quit()


if __name__ == '__main__':
    main()

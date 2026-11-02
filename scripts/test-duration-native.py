import pathlib
import os
import sys
import json
import subprocess
import threading
import time
import sqlite3
import shutil
import tempfile
import importlib.util
import socket
import select
import re
import wave
import hashlib
sys.dont_write_bytecode = True
if os.environ.get('KAZE_PRIVATE_DEFECT_TEST') != '1':
    raise SystemExit('Run with KAZE_PRIVATE_DEFECT_TEST=1 dbus-run-session -- python3 scripts/test-duration-native.py')
REPO = pathlib.Path(__file__).resolve().parents[1]
ROOT = pathlib.Path(os.environ['KAZE_EVIDENCE_DIR']) if os.environ.get('KAZE_EVIDENCE_DIR') else pathlib.Path(tempfile.mkdtemp(prefix='kaze-native-defects-'))
ROOT.mkdir(parents=True, exist_ok=True)
WORK = pathlib.Path(tempfile.mkdtemp(prefix='kaze-review-resume-'))
sys.path.insert(0, str(REPO / 'scripts'))

def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m
tray = module('review_tray', REPO / 'scripts/test-tray-lifecycle.py')
watcher = tray.Watcher()
loop = tray.GLib.MainLoop()
threading.Thread(target=loop.run, daemon=True).start()
xvfb = subprocess.Popen(['Xvfb', '-displayfd', '1', '-screen', '0', '1440x900x24', '-nolisten', 'tcp', '-noreset'], stdout=subprocess.PIPE, stderr=(ROOT / 'xvfb.log').open('w'), text=True)
assert select.select([xvfb.stdout], [], [], 10)[0]
DISPLAY = ':' + xvfb.stdout.readline().strip()
os.environ['DISPLAY'] = DISPLAY
os.environ['GDK_BACKEND'] = 'x11'
os.environ.pop('WAYLAND_DISPLAY', None)
os.environ.pop('XAUTHORITY', None)
BINARY = WORK / 'kaze-review'
shutil.copy2(REPO / 'target/debug/kaze', BINARY)
(ROOT / 'native-provenance.json').write_text(json.dumps({'binary_sha256': hashlib.sha256(BINARY.read_bytes()).hexdigest(), 'assets': 'unchanged embedded production frontend; custom-protocol debug Rust build', 'display': 'private Xvfb GTK X11 1440x900; not the original Wayland workload'}, indent=2))
STATE = {'phase': 'seed', 'cases': []}

def save():
    (ROOT / 'native-status.json').write_text(json.dumps(STATE, indent=2) + '\n')

def env(profile, mode):
    e = dict(os.environ, XDG_DATA_HOME=str(profile / 'data'), XDG_CACHE_HOME=str(profile / 'cache'), XDG_CONFIG_HOME=str(profile / 'config'), HTTP_PROXY='http://127.0.0.1:9', HTTPS_PROXY='http://127.0.0.1:9', NO_PROXY='127.0.0.1,localhost', NO_AT_BRIDGE='1', GIO_USE_VFS='local', RUST_LOG='music_player_backend::core::processor=info,kaze=warn', __GL_SHADER_DISK_CACHE_PATH=str(profile / 'cache/nvidia'))
    e.pop('WEBKIT_INSPECTOR_HTTP_SERVER', None)
    if mode == 'profile':
        with socket.socket() as s:
            s.bind(('127.0.0.1', 0))
            e['WEBKIT_INSPECTOR_HTTP_SERVER'] = '127.0.0.1:' + str(s.getsockname()[1])
    return e

def win(pid):
    ids = subprocess.check_output(['xdotool', 'search', '--onlyvisible', '--pid', str(pid)], text=True).split()
    rows = []
    for xid in ids:
        raw = subprocess.check_output(['xwininfo', '-id', xid], text=True)
        d = {'xid': xid, 'viewable': 'Map State: IsViewable' in raw}
        for label, key in [('Width', 'width'), ('Height', 'height')]:
            d[key] = int(re.search(label + ':\\s*(\\d+)', raw).group(1))
        rows.append(d)
    d = max(rows, key=lambda a: a['width'] * a['height'])
    d['focused'] = subprocess.run(['xdotool', 'getwindowfocus'], capture_output=True, text=True).stdout.strip() == d['xid']
    return d

def launch(profile, mode, name):
    log = (ROOT / (name + '.log')).open('w')
    e = env(profile, mode)
    app = subprocess.Popen([str(BINARY)], env=e, stdout=log, stderr=log)
    try:
        tray.wait_for(lambda: watcher.owner() and watcher.item(app.pid), 'private app', timeout=35)
        w = tray.wait_for(lambda: win(app.pid), 'mapped app', timeout=20)
        subprocess.run(['xdotool', 'windowfocus', '--sync', w['xid']], check=True)
        browser = tray.Inspector(int(e['WEBKIT_INSPECTOR_HTTP_SERVER'].split(':')[-1])) if mode == 'profile' else None
        return (app, browser, log)
    except BaseException:
        app.terminate()
        app.wait(timeout=10)
        log.close()
        raise

def quit(app, browser, log):
    if browser:
        browser.close()
    if app.poll() is None:
        try:
            watcher.menu(app.pid, 'Quit Kaze')
            app.wait(timeout=15)
        except Exception:
            app.terminate()
            app.wait(timeout=10)
    tray.wait_for(lambda: not watcher.owner(), 'private owner released', timeout=15)
    log.close()

def clone(seed, name, style='spatial'):
    profile = WORK / name
    shutil.copytree(seed, profile)
    with sqlite3.connect(profile / 'data/music-player/music_player.db') as db:
        db.execute("INSERT OR REPLACE INTO application_settings(key,value,updated_at)VALUES('interface_style',?,1)", (style,))
    return profile

def native_duration(seed, style, zero=True):
    name = style + ('-unknown-duration' if zero else '-duration-control')
    STATE['phase'] = name
    save()
    profile = clone(seed, name, style)
    with sqlite3.connect(profile / 'data/music-player/music_player.db') as db:
        db.execute("DELETE FROM application_settings WHERE key='playback_session_v1'")
        for i, secs in [(0, 6), (1, 2), (2, 6)]:
            db.execute('UPDATE tracks SET file_path=?,duration_secs=? WHERE id=?', (str(WORK / ('short' + str(i) + '.wav')), 0 if i == 1 and zero else secs, 't' + str(i).zfill(4)))
    app, browser, log = launch(profile, 'profile', name)
    record = {'name': name, 'observations': []}
    try:
        tray.wait_for(lambda: browser.evaluate("!!document.querySelector('.home-flow')"), 'Home', timeout=20)
        browser.command('PlayTrack', track_id='t0000', source='library')
        browser.command('EnqueueTrack', track_id='t0001', play_next=False)
        browser.command('EnqueueTrack', track_id='t0002', play_next=False)
        time.sleep(0.5)

        def snap(phase):
            record['observations'].append({'phase': phase, 'state': browser.query('GetPlaybackState'), 'seek': browser.evaluate("(()=>{const s=document.querySelector('input[aria-label=Seek]');return s?{max:s.max,disabled:s.disabled,value:s.value}:null})()")})
        snap('known')
        browser.command('NextTrack')
        time.sleep(0.2)
        snap('second-start')
        if zero:
            assert record['observations'][-1]['state']['current_track_id'] == 't0001'
            assert record['observations'][-1]['state']['is_playing']
            assert record['observations'][-1]['seek']['disabled']
            browser.command('Pause')
            snap('paused')
            assert record['observations'][-1]['state']['current_track_id'] == 't0001'
            browser.command('Resume')
        time.sleep(3)
        snap('after-eof')
        end = record['observations'][-1]['state']
        assert end['current_queue_index'] == 2, end
        record['completed'] = True
        record['passed'] = True
    finally:
        quit(app, browser, log)
        (ROOT / (name + '.json')).write_text(json.dumps(record, indent=2))
        shutil.rmtree(profile)
    STATE['cases'].append({'name': name, 'completed': True})
    save()
try:
    save()
    seed = WORK / 'seed'
    seed.mkdir()
    app, browser, log = launch(seed, 'none', 'seed')
    time.sleep(3)
    quit(app, browser, log)
    for i, secs in [(0, 6), (1, 2), (2, 6), ('long', 1800)]:
        p = WORK / ('short' + str(i) + '.wav' if i != 'long' else 'long.wav')
        with wave.open(str(p), 'wb') as w:
            w.setnchannels(1)
            w.setsampwidth(2)
            w.setframerate(8000)
            for _ in range(secs):
                w.writeframesraw(b'\x00\x00' * 8000)
    with sqlite3.connect(seed / 'data/music-player/music_player.db') as db:
        for k, v in [('onboarding_completed', 'true'), ('enable_jiosaavn', 'false'), ('scan_on_startup', 'false'), ('auto_fill_metadata', 'false')]:
            db.execute('INSERT OR REPLACE INTO application_settings(key,value,updated_at)VALUES(?,?,1)', (k, v))
        db.execute("INSERT OR REPLACE INTO recommendation_setup VALUES('default','skipped','[]','[]',1,1)")
        for i in range(201):
            db.execute('INSERT INTO albums(id,title,normalized_title,created_at)VALUES(?,?,?,1)', ('a' + str(i), f'Review album {i:04}', f'review album {i:04}'))
        for i in range(1036):
            path = WORK / ('t' + str(i) + '.wav')
            os.link(WORK / 'long.wav', path)
            db.execute("INSERT INTO tracks(id,file_path,file_size,modified_timestamp,title,normalized_title,duration_secs,format,album_id,has_cover_art,created_at,updated_at)VALUES(?,?,?,1,?,?,1800,'wav',?,0,1,1)", ('t' + str(i).zfill(4), str(path), path.stat().st_size, f'Review track {i:04}', f'review track {i:04}', 'a' + str(i % 201)))
        items = [{'queue_id': 'review-q' + str(i), 'track_id': 't' + str(i).zfill(4), 'title': f'Review track {i:04}', 'artist': 'Unknown Artist', 'duration_secs': 1800} for i in range(2)]
        bookmark = {'version': 1, 'queue': {'items': items, 'current_index': 0, 'shuffle_indices': [0, 1], 'shuffle_enabled': False, 'repeat_mode': 'off'}, 'has_current_track': True, 'position_secs': 0, 'duration_secs': 1800, 'volume': 0.8}
        db.execute("INSERT OR REPLACE INTO application_settings(key,value,updated_at)VALUES('playback_session_v1',?,1)", (json.dumps(bookmark),))
    shutil.rmtree(seed / 'cache', ignore_errors=True)
    for style in ['spatial', 'brutalist']:
        native_duration(seed, style)
    native_duration(seed, 'spatial', False)
    STATE.update(completed=True, passed=True, phase='complete')
    save()
except BaseException as error:
    STATE.update(completed=True, passed=False, error=repr(error))
    save()
    raise
finally:
    loop.quit()
    xvfb.terminate()
    xvfb.wait(timeout=10)
    shutil.rmtree(WORK)

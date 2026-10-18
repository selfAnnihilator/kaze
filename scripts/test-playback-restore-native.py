#!/usr/bin/env python3
"""Cold-launch playback restoration using private data/bus and real silent audio.

KAZE_PRIVATE_RESTORE_TEST=1 dbus-run-session -- python3 scripts/test-playback-restore-native.py /absolute/embedded/kaze
Requires the same GI/Niri/native dependencies as test-tray-lifecycle.py.
"""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import socket
import sqlite3
import subprocess
import sys
import tempfile
import threading
import time
import wave

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location('tray', Path(__file__).with_name('test-tray-lifecycle.py'))
tray = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tray)


def main():
    if os.environ.get('KAZE_PRIVATE_RESTORE_TEST') != '1':
        raise SystemExit('Use explicit opt-in and a private dbus-run-session')
    binary = str(Path(sys.argv[1]).resolve())
    watcher = tray.Watcher()
    loop = tray.GLib.MainLoop()
    threading.Thread(target=loop.run, daemon=True).start()
    app = browser = None
    with tempfile.TemporaryDirectory(prefix='kaze-restore-native-') as directory:
        root = Path(directory)
        with (root / 'native.log').open('w+') as log:
            def launch():
                nonlocal app, browser
                with socket.socket() as sock:
                    sock.bind(('127.0.0.1', 0))
                    port = sock.getsockname()[1]
                env = dict(os.environ, XDG_DATA_HOME=str(root/'data'), XDG_CACHE_HOME=str(root/'cache'),
                           XDG_CONFIG_HOME=str(root/'config'), WEBKIT_INSPECTOR_HTTP_SERVER=f'127.0.0.1:{port}',
                           HTTP_PROXY='http://127.0.0.1:9', HTTPS_PROXY='http://127.0.0.1:9',
                           NO_PROXY='127.0.0.1,localhost', no_proxy='127.0.0.1,localhost',
                           NO_AT_BRIDGE='1', GIO_USE_VFS='local')
                app = subprocess.Popen([binary], env=env, stdout=log, stderr=log)
                tray.wait_for(lambda: watcher.owner() and watcher.item(app.pid), 'private app', timeout=30)
                browser = tray.Inspector(port)
                tray.wait_for(lambda: browser.evaluate("Boolean(document.querySelector('.kaze-interface'))"), 'real frontend', timeout=25)

            def quit_own():
                nonlocal app, browser
                if browser:
                    browser.close()
                    browser = None
                if app and app.poll() is None:
                    watcher.menu(app.pid, 'Quit Kaze')
                    app.wait(timeout=15)
                    assert app.returncode == 0
                tray.wait_for(lambda: not watcher.owner(), 'MPRIS released')

            def prop(name):
                return watcher.call('org.mpris.MediaPlayer2.kaze', '/org/mpris/MediaPlayer2',
                                    'org.freedesktop.DBus.Properties', 'Get',
                                    tray.GLib.Variant('(ss)', ('org.mpris.MediaPlayer2.Player', name)))[0]

            try:
                launch()
                quit_own()
                database = root/'data/music-player/music_player.db'
                with wave.open(str(root/'silent.wav'), 'wb') as audio:
                    audio.setnchannels(1)
                    audio.setsampwidth(2)
                    audio.setframerate(8000)
                    audio.writeframes(b'\0\0' * 8000 * 120)
                with sqlite3.connect(database) as db:
                    db.execute("INSERT OR REPLACE INTO application_settings (key,value,updated_at) VALUES ('onboarding_completed','true',1)")
                    for identity, title in [('one', 'Saved Track'), ('two', 'Next Saved Track')]:
                        path = root/f'{identity}.wav'
                        os.link(root/'silent.wav', path)
                        db.execute("INSERT INTO tracks (id,file_path,file_size,modified_timestamp,title,normalized_title,duration_secs,format,created_at,updated_at) VALUES (?,?,?,1,?,?,120,'wav',1,1)",
                                   (identity, str(path), path.stat().st_size, title, title.lower()))
                launch()
                browser.command('PlayTrack', track_id='one', source='library')
                browser.command('EnqueueTrack', track_id='two', play_next=False)
                browser.command('Seek', position_secs=37)
                browser.command('SetVolume', volume=.43)
                # Quit while actually playing. Every cold launch must still pause.
                quit_own()
                with sqlite3.connect(database) as db:
                    history_before = db.execute('SELECT count(*) FROM playback_history').fetchone()[0]
                for style in ['spatial', 'brutalist']:
                    with sqlite3.connect(database) as db:
                        db.execute("INSERT OR REPLACE INTO application_settings (key,value,updated_at) VALUES ('interface_style',?,1)", (style,))
                    launch()
                    saved = browser.query('GetPlaybackState')
                    windows = json.loads(subprocess.check_output(['niri', 'msg', '-j', 'windows'], text=True))
                    own = next(window for window in windows if window.get('pid') == app.pid)
                    subprocess.run(['niri', 'msg', 'action', 'focus-window', '--id', str(own['id'])], stdout=subprocess.DEVNULL, check=True)
                    tray.wait_for(lambda: browser.evaluate('document.hasFocus()'), 'focused restored window')
                    browser.evaluate("window.__restoreFrames=0;window.__restoreRaf=requestAnimationFrame(function tick(){window.__restoreFrames++;if(window.__restoreFrames<60)window.__restoreRaf=requestAnimationFrame(tick)});true")
                    time.sleep(.5)
                    assert browser.evaluate('window.__restoreFrames') > 2, 'restored native frame is stalled'
                    browser.evaluate('cancelAnimationFrame(window.__restoreRaf);true')
                    assert saved['current_track']['title'] == 'Saved Track', saved
                    assert saved['is_paused'] and not saved['is_playing'], saved
                    assert 36.9 <= saved['position_secs'] < 40, saved
                    assert saved['queue_track_ids'] == ['two'], saved
                    assert abs(saved['volume']-.43) < .001
                    tray.wait_for(lambda: prop('PlaybackStatus') == 'Paused', 'status bar paused')
                    metadata = prop('Metadata')
                    assert metadata['xesam:title'] == 'Saved Track', metadata
                    assert 36_900_000 <= prop('Position') < 40_000_000
                    tray.wait_for(lambda: browser.evaluate("document.querySelector('.player-bar,.press-deck').textContent.includes('Saved Track')"), f'{style} player title')
                    time.sleep(.5)
                    still = browser.query('GetPlaybackState')
                    assert still['position_secs'] == saved['position_secs'], 'paused position drifted'
                    with sqlite3.connect(database) as db:
                        assert db.execute('SELECT count(*) FROM playback_history').fetchone()[0] == history_before
                        assert db.execute('SELECT count(*) FROM active_playback_journal').fetchone()[0] == 0
                    print(f'PASS {style}: cold restore, player/status bar metadata, queue and stable paused progress; no listening session', flush=True)
                    quit_own()
                # Reuse actual complete cached audio for the online restore path.
                cache = root/'cache/music-player/remote-audio'
                cache.mkdir(parents=True, exist_ok=True)
                key = hashlib.sha256(b'full:v2:one:unknown artist:saved track').hexdigest()
                shutil.copyfile(root/'silent.wav', cache/f'{key}.audio')
                with sqlite3.connect(database) as db:
                    db.execute("UPDATE tracks SET format='online',file_path='online://one' WHERE id='one'")
                launch()
                saved = browser.query('GetPlaybackState')
                assert saved['current_track']['format'] == 'online' and saved['is_paused']
                started = time.monotonic()
                watcher.call('org.mpris.MediaPlayer2.kaze', '/org/mpris/MediaPlayer2', 'org.mpris.MediaPlayer2.Player', 'Play')
                assert time.monotonic()-started < 1, 'status bar Play blocked on media preparation'
                tray.wait_for(lambda: browser.query('GetPlaybackState')['is_playing'], 'status bar resumes saved audio')
                resumed = browser.query('GetPlaybackState')
                assert saved['position_secs'] <= resumed['position_secs'] < saved['position_secs']+4, resumed
                print('PASS cached online audio: status-bar Play resumes real Rust audio at saved position', flush=True)
                quit_own()
            finally:
                if browser:
                    browser.close()
                if app and app.poll() is None:
                    try:
                        watcher.menu(app.pid, 'Quit Kaze')
                        app.wait(timeout=15)
                    except Exception:
                        app.terminate()
                        app.wait(timeout=5)


if __name__ == '__main__':
    main()

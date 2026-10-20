#!/usr/bin/env python3
"""Verify login registration and hidden startup on Niri with disposable XDG data.

Build the embedded desktop binary first, then run:
KAZE_PRIVATE_AUTOSTART_TEST=1 dbus-run-session -- python3 scripts/test-autostart-native.py /path/to/kaze
No real login settings, installed executable or live library are modified.
"""
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

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location('tray_test', Path(__file__).with_name('test-tray-lifecycle.py'))
tray = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tray)


def main():
    if os.environ.get('KAZE_PRIVATE_AUTOSTART_TEST') != '1':
        raise SystemExit('Use KAZE_PRIVATE_AUTOSTART_TEST=1 on a private dbus-run-session')
    watcher = tray.Watcher()
    loop = tray.GLib.MainLoop()
    threading.Thread(target=loop.run, daemon=True).start()
    processes, inspectors, logs = [], [], []
    with tempfile.TemporaryDirectory(prefix='kaze-autostart-') as directory:
        root = Path(directory)
        binary = root / 'Kaze Player'
        shutil.copy2(Path(sys.argv[1]).resolve(), binary)
        env = dict(os.environ, XDG_CONFIG_HOME=str(root/'config'), XDG_DATA_HOME=str(root/'data'), XDG_CACHE_HOME=str(root/'cache'), NO_PROXY='127.0.0.1,localhost', no_proxy='127.0.0.1,localhost', HTTP_PROXY='http://127.0.0.1:9', HTTPS_PROXY='http://127.0.0.1:9', NO_AT_BRIDGE='1', GIO_USE_VFS='local')
        entry = root/'config/autostart/org.intelligentmusicplayer.app.desktop'

        def launch(login=False, visible=True):
            with socket.socket() as reservation:
                reservation.bind(('127.0.0.1', 0))
                port = reservation.getsockname()[1]
            log = (root/f'process-{len(processes)}.log').open('w+')
            logs.append(log)
            app = subprocess.Popen([str(binary)] + (['--autostart'] if login else []), env=dict(env, WEBKIT_INSPECTOR_HTTP_SERVER=f'127.0.0.1:{port}'), stdout=log, stderr=log)
            processes.append(app)
            if watcher.host:
                tray.wait_for(lambda: watcher.item(app.pid), 'tray registration')
            browser = tray.Inspector(port)
            inspectors.append(browser)
            if visible:
                tray.wait_for(lambda: tray.windows(app.pid), 'visible window')
            else:
                # Give queued show/focus callbacks time to run before asserting.
                time.sleep(.4)
                assert not tray.windows(app.pid), 'login startup unexpectedly showed the window'
                assert app.poll() is None
            return app, browser

        def quit_app(app):
            if watcher.item(app.pid):
                watcher.menu(app.pid, 'Quit Kaze')
            else:
                window = tray.wait_for(lambda: tray.windows(app.pid), 'window to quit')[0]
                subprocess.run(['niri', 'msg', 'action', 'close-window', '--id', str(window['id'])], check=True, capture_output=True)
            app.wait(timeout=12)
            assert app.returncode == 0
            tray.wait_for(lambda: not watcher.owner(), 'MPRIS cleanup')

        def open_settings(browser):
            tray.wait_for(lambda: browser.evaluate("Boolean(document.querySelector('.home-flow, .press-home')) || [...document.querySelectorAll('button')].some(b=>b.getAttribute('aria-label')==='Settings' || b.title==='Settings' || b.textContent.trim()==='Settings')"), 'React ready')
            browser.evaluate("[...document.querySelectorAll('button')].find(b=>b.getAttribute('aria-label')==='Settings' || b.title==='Settings' || b.textContent.trim()==='Settings').click(); true")
            tray.wait_for(lambda: browser.evaluate("!!document.querySelector('.settings-startup-switch input:not(:disabled)')"), 'startup settings loaded')

        def checked(browser):
            return browser.evaluate("document.querySelector('.settings-startup-switch input')?.checked")

        def click_switch(browser):
            browser.evaluate("document.querySelector('.settings-startup-switch input').click(); true")
            tray.wait_for(lambda: browser.evaluate("!document.querySelector('.settings-startup-switch input').disabled"), 'startup save finished')

        try:
            # Initialize schema, then skip onboarding only in this disposable DB.
            warmup, _ = launch()
            quit_app(warmup)
            with sqlite3.connect(root/'data/music-player/music_player.db') as db:
                db.execute("INSERT OR REPLACE INTO application_settings (key,value,updated_at) VALUES ('onboarding_completed','true',1)")
            app, browser = launch()
            open_settings(browser)
            assert checked(browser) is False
            click_switch(browser)
            assert checked(browser) is True
            assert entry.is_file()
            data = entry.read_text()
            assert f'Exec="{binary}" --autostart' in data
            subprocess.run(['desktop-file-validate', str(entry)], check=True, capture_output=True)
            click_switch(browser)
            assert checked(browser) is False and not entry.exists()
            print('PASS: real Settings switch enables/disables an XDG login entry with a quoted executable path', flush=True)

            # Force an ordinary filesystem error without changing permissions or live paths.
            entry.parent.rmdir()
            entry.parent.write_text('blocked')
            click_switch(browser)
            assert checked(browser) is False
            assert browser.evaluate("document.querySelector('.settings-startup [role=alert]').textContent.includes(\"Couldn't change\")")
            entry.parent.unlink()
            click_switch(browser)
            assert checked(browser) is True
            browser.command('SetInterfaceStyle', style='brutalist')
            quit_app(app)
            print('PASS: failed registration leaves the confirmed switch off; retry succeeds', flush=True)

            # An enabled login entry must not hide an ordinary manual launch.
            app, browser = launch()
            open_settings(browser)
            assert browser.evaluate("document.documentElement.dataset.interface==='brutalist'")
            assert checked(browser) is True
            click_switch(browser)
            assert checked(browser) is False and not entry.exists()
            click_switch(browser)
            assert checked(browser) is True and entry.is_file()
            print('PASS: persisted preference and real switch saves also work in Brutalist', flush=True)
            quit_app(app)
            app, browser = launch(login=True, visible=False)
            browser.evaluate("window.__startupMarker='preserved'; true")
            assert browser.query('GetPlaybackState')['is_playing'] is False
            watcher.menu(app.pid, 'Open Kaze')
            tray.wait_for(lambda: tray.windows(app.pid), 'tray Open restores login window')
            assert browser.evaluate("window.__startupMarker==='preserved'")
            window = tray.windows(app.pid)[0]
            subprocess.run(['niri', 'msg', 'action', 'close-window', '--id', str(window['id'])], check=True, capture_output=True)
            tray.wait_for(lambda: not tray.windows(app.pid), 'window hides again')
            for args, should_show in [(['--autostart'], False), ([], True)]:
                secondary = subprocess.Popen([str(binary), *args], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                processes.append(secondary)
                secondary.wait(timeout=6)
                assert secondary.returncode == 0
                if should_show:
                    tray.wait_for(lambda: len(tray.windows(app.pid)) == 1, 'manual second launch restores existing window')
                else:
                    time.sleep(.3)
                    assert not tray.windows(app.pid), 'late login launch interrupted existing instance'
            assert browser.evaluate("window.__startupMarker==='preserved'")
            quit_app(app)
            print('PASS: hidden cold login, tray restore, quiet duplicate login and normal-launch restore preserve one webview', flush=True)

            watcher.host = False
            app, browser = launch(login=True, visible=True)
            assert app.poll() is None
            quit_app(app)
            print('PASS: no tray host falls back to an accessible window', flush=True)

            with sqlite3.connect(root/'data/music-player/music_player.db') as db:
                assert db.execute('SELECT COUNT(*) FROM playback_history').fetchone()[0] == 0
            assert entry.is_file()
            print('PASS: login preference persists and startup creates no playback history', flush=True)
        finally:
            for browser in inspectors:
                try: browser.close()
                except Exception: pass
            for app in processes:
                if app.poll() is None:
                    app.kill()
                    app.wait(timeout=6)
            for log in logs:
                log.seek(0)
                if sys.exc_info()[0] is not None:
                    print(log.read()[-4000:], file=sys.stderr)
                log.close()
            loop.quit()


if __name__ == '__main__':
    main()

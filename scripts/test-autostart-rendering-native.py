#!/usr/bin/env python3
"""Assert real painted pixels after hidden login, before attaching any inspector.

KAZE_PRIVATE_AUTOSTART_TEST=1 dbus-run-session -- python3 scripts/test-autostart-rendering-native.py /path/to/kaze
Requires Niri, Pillow and the native tray test's PyGObject dependencies. Only
windows owned by the disposable test process are captured. Live settings and
installed binaries are never changed. Screenshots/logs are retained in /tmp.
"""
import importlib.util
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import threading
import time

from PIL import Image, ImageStat

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location('tray_test', Path(__file__).with_name('test-tray-lifecycle.py'))
tray = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tray)


def main():
    if os.environ.get('KAZE_PRIVATE_AUTOSTART_TEST') != '1':
        raise SystemExit('Use explicit KAZE_PRIVATE_AUTOSTART_TEST=1 with a private dbus-run-session')
    binary = str(Path(sys.argv[1]).resolve())
    output = Path(tempfile.mkdtemp(prefix='kaze-autostart-rendering-'))
    print(f'Evidence: {output}', flush=True)
    watcher = tray.Watcher()
    loop = tray.GLib.MainLoop()
    threading.Thread(target=loop.run, daemon=True).start()
    processes, logs = [], []
    with tempfile.TemporaryDirectory(prefix='kaze-autostart-profile-') as directory:
        root = Path(directory)
        env = dict(os.environ, XDG_CONFIG_HOME=str(root/'config'), XDG_DATA_HOME=str(root/'data'),
                   XDG_CACHE_HOME=str(root/'cache'), NO_AT_BRIDGE='1', GIO_USE_VFS='local',
                   HTTP_PROXY='http://127.0.0.1:9', HTTPS_PROXY='http://127.0.0.1:9')
        # The inspector's DOM state alone cannot certify that GTK children paint.
        env.pop('WEBKIT_INSPECTOR_HTTP_SERVER', None)

        def launch(login=False):
            log = (output/f'process-{len(processes)}.log').open('w')
            logs.append(log)
            app = subprocess.Popen([binary] + (['--autostart'] if login else []), env=env, stdout=log, stderr=log)
            processes.append(app)
            if watcher.host:
                tray.wait_for(lambda: watcher.item(app.pid), 'private tray registration', timeout=30)
            else:
                tray.wait_for(lambda: tray.windows(app.pid), 'no-host fallback window', timeout=30)
            return app

        def close_window(app):
            own = tray.wait_for(lambda: tray.windows(app.pid), 'visible test window')[0]
            subprocess.run(['niri', 'msg', 'action', 'close-window', '--id', str(own['id'])], check=True, capture_output=True)

        def quit_app(app):
            if watcher.item(app.pid):
                watcher.menu(app.pid, 'Quit Kaze')
            else:
                close_window(app)
            app.wait(timeout=12)
            assert app.returncode == 0
            tray.wait_for(lambda: not watcher.owner(), 'private MPRIS cleanup')

        def assert_painted(app, name):
            own = tray.wait_for(lambda: tray.windows(app.pid), 'test window maps')[0]
            result = None
            # Allow the existing two-second introduction to finish; retry
            # late frames on slow hosts, without evaluating JS or forcing layout.
            for attempt, delay in enumerate((3, 2, 2, 3)):
                time.sleep(delay)
                # Niri writes captures asynchronously. A distinct filename for
                # each attempt prevents accepting the previous capture's pixels.
                path = output/f'{name}-{attempt}.png'
                subprocess.run(['niri', 'msg', 'action', 'screenshot-window', '--id', str(own['id']), '--path', str(path)], check=True, capture_output=True)
                tray.wait_for(path.exists, 'screenshot saved')
                with Image.open(path) as source:
                    source.load()
                    image = source.convert('RGB')
                    crop = image.crop((20, 20, image.width-20, image.height-20))
                stddev = max(ImageStat.Stat(crop).stddev)
                colors = crop.getcolors(crop.width*crop.height)
                dominant = max(count for count, _ in colors)/(crop.width*crop.height)
                result = {'name': name, 'stddev': stddev, 'dominant': dominant}
                if stddev > 5 and dominant < .98:
                    print('PASS painted: ' + json.dumps(result), flush=True)
                    return
            raise AssertionError(f'Blank restored window: {result}; screenshot: {path}')

        try:
            warmup = launch()
            assert_painted(warmup, 'normal-launch-control')
            quit_app(warmup)
            database = root/'data/music-player/music_player.db'
            for style in ('spatial', 'brutalist'):
                with sqlite3.connect(database) as db:
                    db.execute("INSERT OR REPLACE INTO application_settings (key,value,updated_at) VALUES ('onboarding_completed','true',1)")
                    db.execute("INSERT OR REPLACE INTO application_settings (key,value,updated_at) VALUES ('interface_style',?,1)", (style,))
                for method in ('tray', 'manual-launch', 'no-host'):
                    watcher.host = method != 'no-host'
                    app = launch(login=True)
                    if watcher.host:
                        time.sleep(2)
                        assert not tray.windows(app.pid), 'login flashed a window before Open'
                        if method == 'tray':
                            watcher.menu(app.pid, 'Open Kaze')
                        else:
                            second = subprocess.Popen([binary], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                            processes.append(second)
                            second.wait(timeout=6)
                            assert second.returncode == 0
                    assert_painted(app, f'{style}-{method}-first-open')
                    assert app.poll() is None
                    if watcher.host:
                        close_window(app)
                        tray.wait_for(lambda: not tray.windows(app.pid), 'hide existing window')
                        watcher.menu(app.pid, 'Open Kaze')
                        assert_painted(app, f'{style}-{method}-second-open')
                    quit_app(app)
            with sqlite3.connect(database) as db:
                assert db.execute('SELECT COUNT(*) FROM playback_history').fetchone()[0] == 0
            print('PASS: both themes paint on first Open, manual launch, repeat restore and no-host fallback; no inspector or playback history', flush=True)
        finally:
            for app in processes:
                if app.poll() is None:
                    app.kill()
                    app.wait(timeout=6)
            for log in logs:
                log.close()
            loop.quit()


if __name__ == '__main__':
    main()

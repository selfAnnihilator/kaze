#!/usr/bin/env python3
"""Real tray Quit while the real decoder waits on a controlled HTTP stream.
Requires a copied binary built with tauri/custom-protocol,acceptance-fixtures.
Run with KAZE_PRIVATE_TRAY_TEST=1 in a private dbus-run-session.
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
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

spec = importlib.util.spec_from_file_location('tray_support', Path(__file__).with_name('test-tray-lifecycle.py'))
tray = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tray)


def main():
    assert os.environ.get('KAZE_PRIVATE_TRAY_TEST') == '1'
    binary = str(Path(sys.argv[1]).resolve())
    watcher = tray.Watcher()
    loop = tray.GLib.MainLoop()
    threading.Thread(target=loop.run, daemon=True).start()
    received = threading.Event()
    release = threading.Event()

    class Audio(BaseHTTPRequestHandler):
        def do_GET(self):
            self.send_response(200)
            self.send_header('Content-Type', 'audio/wav')
            self.send_header('Content-Length', '294196')
            self.end_headers()
            # RIFF with a large legal JUNK chunk. No fmt/data arrive.
            self.wfile.write(b'RIFF' + (294188).to_bytes(4, 'little') + b'WAVEJUNK' + (262144).to_bytes(4, 'little') + bytes(65516))
            self.wfile.flush()
            received.set()
            release.wait(60)

        def log_message(self, *_args):
            pass

    server = ThreadingHTTPServer(('127.0.0.1', 0), Audio)
    server.daemon_threads = True
    threading.Thread(target=server.serve_forever, daemon=True).start()
    results = []
    try:
        with tempfile.TemporaryDirectory(prefix='kaze-stalled-tray-') as directory:
            root = Path(directory)
            env = dict(os.environ, XDG_DATA_HOME=str(root/'data'), XDG_CACHE_HOME=str(root/'cache'), XDG_CONFIG_HOME=str(root/'config'),
                       NO_PROXY='127.0.0.1,localhost', no_proxy='127.0.0.1,localhost', HTTP_PROXY='http://127.0.0.1:9', HTTPS_PROXY='http://127.0.0.1:9',
                       KAZE_PREPARATION_FIXTURE_URL=f'http://127.0.0.1:{server.server_port}/fixture.wav', NO_AT_BRIDGE='1', GIO_USE_VFS='local')
            for mode in ['warmup', 'spatial', 'brutalist']:
                with socket.socket() as sock:
                    sock.bind(('127.0.0.1', 0))
                    port = sock.getsockname()[1]
                env['WEBKIT_INSPECTOR_HTTP_SERVER'] = f'127.0.0.1:{port}'
                with (root/f'{mode}.log').open('w+') as log:
                    app = subprocess.Popen([binary], env=env, stdout=log, stderr=log)
                    inspector = None
                    try:
                        tray.wait_for(lambda: watcher.item(app.pid), 'tray registration')
                        if mode != 'warmup':
                            inspector = tray.Inspector(port)
                            if mode == 'brutalist':
                                inspector.command('SetInterfaceStyle', style='brutalist')
                            def start_stalled():
                                received.clear()
                                inspector.evaluate("window.__TAURI_INTERNALS__.invoke('execute_command',{command:{command:'PlayTrack',payload:{track_id:'jiosaavn:preparation-fixture',source:'library'}}}).catch(()=>{}); true")
                                assert received.wait(5), 'stream request never reached local fixture'
                                tray.wait_for(lambda: inspector.query('GetPlaybackState')['is_buffering'], 'preparation buffering')
                                time.sleep(.1)

                            for _ in range(10):
                                start_stalled()
                                inspector.command('Stop')
                                assert not inspector.query('GetPlaybackState')['is_buffering']
                                tray.wait_for(lambda: not list((root/'cache').rglob('*.part')), 'partial cleanup after Stop')
                            received.clear()
                            inspector.evaluate("window.__TAURI_INTERNALS__.invoke('execute_command',{command:{command:'PlayTrack',payload:{track_id:'jiosaavn:preparation-fixture',source:'library'}}}).catch(()=>{}); true")
                            assert received.wait(5), 'stream request never reached local fixture'
                            time.sleep(.4)
                        start = time.monotonic()
                        watcher.menu(app.pid, 'Quit Kaze')
                        app.wait(timeout=5)
                        assert app.returncode == 0
                        elapsed = time.monotonic()-start
                        results.append({'case':mode,'quit_seconds':elapsed,'exit_code':app.returncode,'repeated_preparations':0 if mode == 'warmup' else 10})
                        print(json.dumps(results[-1]), flush=True)
                    finally:
                        if inspector:
                            inspector.close()
                        if app.poll() is None:
                            app.terminate()
                            app.wait(timeout=5)
                if os.environ.get('KAZE_EVIDENCE_DIR'):
                    import shutil
                    shutil.copyfile(root/f'{mode}.log', Path(os.environ['KAZE_EVIDENCE_DIR'])/f'native-preparation-{mode}.log')
                db_path = root/'data/music-player/music_player.db'
                if mode == 'warmup':
                    with sqlite3.connect(db_path) as db:
                        db.execute("INSERT INTO tracks(id,file_path,file_size,modified_timestamp,title,normalized_title,duration_secs,format,created_at,updated_at) VALUES('jiosaavn:preparation-fixture','online://fixture',1,1,'fixture','fixture',2,'online',1,1)")
                else:
                    with sqlite3.connect(db_path) as db:
                        assert db.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
                    assert not list((root/'cache').rglob('*.part')), 'partial files leaked after Quit'
    finally:
        release.set()
        server.shutdown()
        loop.quit()
    output = os.environ.get('KAZE_EVIDENCE_DIR')
    if output:
        Path(output, 'stalled-tray.json').write_text(json.dumps(results, indent=2))


if __name__ == '__main__':
    main()

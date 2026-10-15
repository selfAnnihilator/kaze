#!/usr/bin/env python3
"""Linux single-instance regression against the real desktop binary.

Uses the tray harness's private SNI host, silent audio and WebKit inspector.
Use KAZE_TEST_NIRI_LAUNCH=1 KAZE_REQUIRE_FOCUS=1 to launch through niri with
real activation tokens and require compositor focus. Tokenless launches may
be denied activation; tray focus is observed separately from launcher focus.
Build with --features tauri/custom-protocol, then run:
KAZE_PRIVATE_TRAY_TEST=1 dbus-run-session -- python3 scripts/test-single-instance.py /absolute/path/to/kaze
"""
import importlib.util
import json
import os
import socket
import signal
import sqlite3
import subprocess
import sys
import tempfile
import threading
import time
import wave
from pathlib import Path

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location('tray_test', Path(__file__).with_name('test-tray-lifecycle.py'))
tray = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tray)
GLib, wait_for = tray.GLib, tray.wait_for
NAME = 'org.intelligentmusicplayer.app.SingleInstance'
MPRIS = 'org.mpris.MediaPlayer2.kaze'


class LauncherChild:
    """A test child started by niri, with exit status collected by its helper."""
    def __init__(self, state):
        self.state = state
        self.pid = wait_for(lambda: json.loads(state.read_text()).get('pid'), 'launcher child PID')
        try:
            self.pidfd = os.pidfd_open(self.pid)
        except ProcessLookupError:
            self.pidfd = None

    def poll(self):
        return json.loads(self.state.read_text()).get('returncode')

    def wait(self, timeout=15):
        wait_for(lambda: self.poll() is not None, 'launcher child exit', timeout=timeout)
        return self.poll()

    def kill(self):
        if self.pidfd is not None:
            signal.pidfd_send_signal(self.pidfd, signal.SIGKILL)


def launcher_child(spec_path):
    spec = json.loads(Path(spec_path).read_text())
    state = Path(spec['state'])
    def report(value):
        temporary = state.with_suffix('.tmp')
        temporary.write_text(json.dumps(value)); temporary.replace(state)
    with Path(spec['log']).open('w') as log:
        # niri supplies a real activation token to this launcher process. Only
        # disposable test settings are overridden; never serialize host env.
        env = dict(os.environ, **spec['env'])
        log.write(f'Launcher activation context present: {bool(env.get("XDG_ACTIVATION_TOKEN"))}\n')
        log.flush()
        app = subprocess.Popen([spec['binary']], env=env, stdout=log, stderr=log)
        report({'pid': app.pid})
        report({'pid': app.pid, 'returncode': app.wait()})


def focus_fixture():
    import gi
    gi.require_version('Gtk', '3.0')
    from gi.repository import Gtk
    window = Gtk.Window(title='Kaze focus test fixture')
    window.set_default_size(320, 160)
    window.show_all()
    Gtk.main()


def main():
    if os.environ.get('KAZE_PRIVATE_TRAY_TEST') != '1':
        raise SystemExit('Run on a private dbus-run-session with KAZE_PRIVATE_TRAY_TEST=1')
    binary = str(Path(sys.argv[1]).resolve())
    watcher = tray.Watcher()
    loop = GLib.MainLoop()
    threading.Thread(target=loop.run, daemon=True).start()
    children, inspectors, focus_limits, fixtures = [], [], [], []
    with tempfile.TemporaryDirectory(prefix='kaze-single-instance-') as directory:
        root = Path(directory)
        profile = root/'primary'

        def owner(name=NAME):
            try:
                return watcher.call('org.freedesktop.DBus', '/org/freedesktop/DBus', 'org.freedesktop.DBus', 'GetConnectionUnixProcessID', GLib.Variant('(s)', (name,)))[0]
            except GLib.Error:
                return None

        def spawn(data=profile):
            with socket.socket() as reservation:
                reservation.bind(('127.0.0.1', 0))
                port = reservation.getsockname()[1]
            log_path = root/f'child-{len(children)}.log'
            overrides = dict(DBUS_SESSION_BUS_ADDRESS=os.environ['DBUS_SESSION_BUS_ADDRESS'], XDG_DATA_HOME=str(data/'data'), XDG_CACHE_HOME=str(data/'cache'), WEBKIT_INSPECTOR_HTTP_SERVER=f'127.0.0.1:{port}', NO_PROXY='127.0.0.1,localhost', no_proxy='127.0.0.1,localhost', HTTPS_PROXY='http://127.0.0.1:9', HTTP_PROXY='http://127.0.0.1:9', NO_AT_BRIDGE='1', GIO_USE_VFS='local', RUST_LOG='music_player_backend=info,kaze=info,warn')
            if os.environ.get('KAZE_TEST_NIRI_LAUNCH') == '1':
                spec_path = root/f'launch-{len(children)}.json'
                state = spec_path.with_suffix('.state')
                spec_path.write_text(json.dumps({'binary': binary, 'env': overrides, 'log': str(log_path), 'state': str(state)}))
                subprocess.run(['niri', 'msg', 'action', 'spawn', '--', sys.executable, str(Path(__file__).resolve()), '--launcher-child', str(spec_path)], check=True, capture_output=True)
                app = LauncherChild(state)
                assert 'Launcher activation context present: True' in log_path.read_text(), 'niri did not issue an activation token'
            else:
                env = dict(os.environ, **overrides)
                env.pop('XDG_ACTIVATION_TOKEN', None)
                env.pop('DESKTOP_STARTUP_ID', None)
                with log_path.open('w') as log:
                    app = subprocess.Popen([binary], env=env, stdout=log, stderr=log)
            child = (app, port, log_path)
            children.append(child)
            return child

        def ready(child):
            app, port, _ = child
            wait_for(lambda: len(tray.windows(app.pid)) == 1, 'one primary window')
            wait_for(lambda: watcher.item(app.pid), 'one primary tray')
            wait_for(lambda: owner(MPRIS) == app.pid, 'primary MPRIS owner')
            browser = tray.Inspector(port)
            inspectors.append(browser)
            return browser

        def quit_app(child):
            app = child[0]
            watcher.menu(app.pid, 'Quit Kaze')
            assert app.wait(timeout=15) == 0
            wait_for(lambda: owner() is None and owner(MPRIS) is None, 'clean release of both names')

        def hide(app):
            window = wait_for(lambda: tray.windows(app.pid), 'window before hide')[0]
            subprocess.run(['niri', 'msg', 'action', 'close-window', '--id', str(window['id'])], check=True, capture_output=True)
            wait_for(lambda: not tray.windows(app.pid), 'primary hidden')
            assert app.poll() is None

        def secondary_done(child):
            app, _, log = child
            assert app.wait(timeout=15) == 0, log.read_text()[-3000:]
            assert 'Opening SQLite database' not in log.read_text(), 'secondary initialized database/backend'
            assert not tray.windows(app.pid), 'secondary created a window'
            assert watcher.item(app.pid) is None, 'secondary created a tray'

        try:
            # Start each burst without waiting for any contender to acquire the
            # name or finish setup. Test both the first DB creation and relaunch.
            for round_number in range(3):
                burst = [spawn() for _ in range(5)]
                primary_pid = wait_for(owner, 'cold-start name arbitration')
                primary = next(child for child in burst if child[0].pid == primary_pid)
                browser = ready(primary)
                for child in burst:
                    if child is not primary:
                        secondary_done(child)
                assert sum(child[2].read_text().count('Opening SQLite database') for child in burst) == 1
                assert sum(watcher.item(child[0].pid) is not None for child in burst) == 1
                assert owner() == owner(MPRIS) == primary_pid
                browser.close(); inspectors.remove(browser)
                quit_app(primary)
                print(f'PASS cold burst {round_number + 1}: five launches, one backend/window/tray/MPRIS, four clean secondary exits', flush=True)

            database = profile/'data/music-player/music_player.db'
            with sqlite3.connect(database) as db:
                for index in range(3):
                    path = root/f'track-{index}.wav'
                    with wave.open(str(path), 'wb') as audio:
                        audio.setnchannels(1); audio.setsampwidth(2); audio.setframerate(8000)
                        audio.writeframes(b'\x00\x00' * 8000 * 120)
                    db.execute("INSERT INTO tracks (id,file_path,file_size,modified_timestamp,title,normalized_title,duration_secs,format,created_at,updated_at) VALUES (?,?,?,1,?,?,120,'wav',1,1)", (f'track-{index}', str(path), path.stat().st_size, f'Instance Track {index}', f'instance track {index}'))

            primary = spawn()
            app = primary[0]
            browser = ready(primary)
            browser.command('SetCloseButtonBehavior', behavior='minimize_to_tray')
            wait_for(lambda: browser.evaluate("Boolean([...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Settings'))"), 'sidebar ready')
            browser.evaluate("[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Settings').click(); true")
            wait_for(lambda: browser.evaluate("document.body.innerText.includes('Configured Music Folders')"), 'Settings route')
            browser.evaluate("window.__instanceMarker={}; window.__instanceMain=document.querySelector('main'); const spacer=document.createElement('div'); spacer.style.height='2000px'; window.__instanceMain.append(spacer); window.__instanceMain.scrollTop=180; true")
            browser.command('PlayTrack', track_id='track-0', source='library')
            browser.command('EnqueueTrack', track_id='track-1', play_next=False)
            browser.command('Seek', position_secs=12.0)

            def journal_sessions():
                with sqlite3.connect(database) as db:
                    return db.execute('SELECT session_id, track_id, started_at FROM active_playback_journal ORDER BY session_id').fetchall()

            def restore_burst(label, progressing=False):
                before = browser.query('GetPlaybackState')
                flow = browser.query('GetFlowSession')
                if before['is_playing'] or before['is_paused']:
                    wait_for(lambda: len(journal_sessions()) == 1 and journal_sessions()[0][1] == before['current_track_id'], 'real active playback journal')
                else:
                    wait_for(lambda: not journal_sessions(), 'stopped session finalized')
                sessions = journal_sessions()
                with sqlite3.connect(database) as db:
                    history_count = db.execute('SELECT count(*) FROM playback_history').fetchone()[0]
                started = time.monotonic()
                tray_item = watcher.item(app.pid)
                mpris_owner = watcher.call('org.freedesktop.DBus', '/org/freedesktop/DBus', 'org.freedesktop.DBus', 'GetNameOwner', GLib.Variant('(s)', (MPRIS,)))[0]
                hide(app)
                secondary_profiles = [root/f'{label}-secondary-{index}' for index in range(5)]
                burst = [spawn(data) for data in secondary_profiles]
                for child in burst:
                    secondary_done(child)
                wait_for(lambda: len(tray.windows(app.pid)) == 1, 'existing primary restored')
                if os.environ.get('KAZE_REQUIRE_FOCUS') == '1':
                    wait_for(lambda: tray.windows(app.pid)[0]['is_focused'], 'compositor focus from launcher token')
                focused = bool(tray.windows(app.pid)[0]['is_focused'])
                print(f'OBSERVED {label} compositor focus after restore: {focused}', flush=True)
                if not focused:
                    # Distinguish activation policy from focusing before the
                    # native show request was processed. This is diagnostic;
                    # restore/state assertions below remain mandatory.
                    watcher.menu(app.pid, 'Open Kaze')
                    time.sleep(.3)
                    print(f'OBSERVED {label} focus requested again while visible: {tray.windows(app.pid)[0]["is_focused"]}', flush=True)
                    focus_limits.append(label)
                    print(f'LIMITATION {label}: secondary restored the window without compositor focus', flush=True)
                after = browser.query('GetPlaybackState')
                assert browser.query('GetFlowSession') == flow, 'Flow session changed'
                for key in before:
                    if key != 'position_secs':
                        assert after[key] == before[key], (key, before[key], after[key])
                elapsed = time.monotonic() - started
                delta = after['position_secs'] - before['position_secs']
                if progressing:
                    assert .04 < delta <= elapsed + 0.5, 'playing position stopped/reset/jumped'
                else:
                    assert abs(delta) < .05, 'paused/stopped position changed'
                assert journal_sessions() == sessions, 'real playback session changed'
                with sqlite3.connect(database) as db:
                    assert db.execute('SELECT count(*) FROM playback_history').fetchone()[0] == history_count, 'restore finalized or duplicated history'
                assert browser.evaluate("Boolean(window.__instanceMarker) && window.__instanceMain===document.querySelector('main') && document.querySelector('main').scrollTop===180 && document.body.innerText.includes('Configured Music Folders')"), 'webview/route/scroll reset'
                assert owner() == owner(MPRIS) == app.pid
                assert watcher.item(app.pid) == tray_item
                assert watcher.call('org.freedesktop.DBus', '/org/freedesktop/DBus', 'org.freedesktop.DBus', 'GetNameOwner', GLib.Variant('(s)', (MPRIS,)))[0] == mpris_owner
                assert all(not data.exists() for data in secondary_profiles), 'secondary wrote app data/cache'
                print(f'PASS hidden {label}: five secondary exits, same process/webview/route/scroll/queue/position/Flow/tray/MPRIS', flush=True)

            restore_burst('playing', progressing=True)
            browser.command('Pause')
            restore_burst('paused')
            browser.command('Stop')
            restore_burst('stopped')
            browser.command('StartFlow', familiarity_ratio=.5)
            browser.command('Pause')
            assert browser.query('GetFlowSession')['is_active']
            restore_burst('flow')
            browser.command('Resume')
            restore_burst('flow-playing', progressing=True)
            browser.command('Pause')

            fixture = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), '--focus-fixture'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            fixtures.append(fixture)
            fixture_window = wait_for(lambda: tray.windows(fixture.pid), 'own background-focus fixture')[0]['id']

            def background_primary():
                subprocess.run(['niri', 'msg', 'action', 'focus-window', '--id', str(fixture_window)], check=True, capture_output=True)
                wait_for(lambda: tray.windows(fixture.pid)[0]['is_focused'], 'fixture gets focus')

            paused_state = browser.query('GetPlaybackState')
            flow_state = browser.query('GetFlowSession')
            tray_item = watcher.item(app.pid)
            for index in range(10):
                background_primary()
                child = spawn(root/f'behind-secondary-{index}')
                secondary_done(child)
                if os.environ.get('KAZE_REQUIRE_FOCUS') == '1':
                    wait_for(lambda: tray.windows(app.pid)[0]['is_focused'], 'visible primary gets keyboard focus')
                assert browser.query('GetPlaybackState') == paused_state
                assert browser.query('GetFlowSession') == flow_state
                assert owner() == owner(MPRIS) == app.pid
                assert watcher.item(app.pid) == tray_item
                assert primary[2].read_text().count('Opening SQLite database') == 1
            print(f'PASS visible behind another app: ten launches, same backend/queue/Flow/tray; focus required={os.environ.get("KAZE_REQUIRE_FOCUS") == "1"}', flush=True)
            background_primary()
            time.sleep(.3)
            assert tray.windows(fixture.pid)[0]['is_focused'], 'Kaze stole focus again without another launch'
            print('PASS no focus loop after repeated launches', flush=True)

            hide(app)
            background_primary()
            watcher.menu(app.pid, 'Open Kaze')
            wait_for(lambda: tray.windows(app.pid), 'tray Open restores existing primary')
            time.sleep(.2)
            tray_focused = bool(tray.windows(app.pid)[0]['is_focused'])
            print(f'OBSERVED tray Open without launcher token: focused={tray_focused}', flush=True)
            assert browser.query('GetPlaybackState') == paused_state
            assert browser.query('GetFlowSession') == flow_state
            print('PASS tray Open: same window/backend/state; activation remains compositor-controlled', flush=True)

            # Hold a real SQLite writer so history drain keeps Quit in progress.
            # The plugin name must stay held, and new launches must not revive UI.
            hide(app)
            blocker = sqlite3.connect(database)
            blocker.execute('BEGIN IMMEDIATE')
            try:
                watcher.menu(app.pid, 'Quit Kaze')
                wait_for(lambda: owner(MPRIS) is None, 'Quit started and MPRIS closed')
                assert app.poll() is None and owner() == app.pid, 'Quit drain was not held'
                for child in [spawn(root/f'quitting-secondary-{i}') for i in range(5)]:
                    secondary_done(child)
                assert not tray.windows(app.pid), 'secondary revived UI during Quit'
                assert owner() == app.pid, 'singleton channel released before drain'
            finally:
                blocker.rollback(); blocker.close()
            assert app.wait(timeout=15) == 0
            wait_for(lambda: owner() is None, 'singleton release after drain')
            print('PASS shutdown race: five launches during pending history drain cannot reopen UI or initialize another backend', flush=True)

            primary = spawn(); browser = ready(primary)
            browser.command('SetCloseButtonBehavior', behavior='quit_app')
            window = tray.windows(primary[0].pid)[0]
            subprocess.run(['niri', 'msg', 'action', 'close-window', '--id', str(window['id'])], check=True, capture_output=True)
            assert primary[0].wait(timeout=15) == 0
            wait_for(lambda: owner() is None and owner(MPRIS) is None, 'X Quit name release')
            print('PASS close-to-quit: immediate relaunch reacquires both names', flush=True)

            primary = spawn(); browser = ready(primary)
            primary[0].kill(); primary[0].wait(timeout=5)
            wait_for(lambda: owner() is None and owner(MPRIS) is None, 'SIGKILL name release')
            primary = spawn(); browser = ready(primary)
            quit_app(primary)
            print('PASS SIGKILL: automatic D-Bus release, immediate relaunch, clean final quit', flush=True)
            if os.environ.get('KAZE_REQUIRE_FOCUS') == '1':
                assert not focus_limits, f'Compositor focus was not granted for: {focus_limits}'
            if os.environ.get('KAZE_REQUIRE_TRAY_FOCUS') == '1':
                assert tray_focused, 'Tray Open restored the window but compositor focus was not granted'
        except Exception:
            for app, _, log in children:
                print(f'PID {app.pid}: {log.read_text()[-2500:]}', file=sys.stderr)
            raise
        finally:
            for browser in inspectors:
                try: browser.close()
                except Exception: pass
            for app, _, _ in children:
                if app.poll() is None:
                    app.kill(); app.wait(timeout=5)
                if isinstance(app, LauncherChild) and app.pidfd is not None:
                    os.close(app.pidfd)
            for fixture in fixtures:
                if fixture.poll() is None:
                    fixture.kill(); fixture.wait(timeout=5)
            loop.quit()


if __name__ == '__main__':
    if len(sys.argv) == 3 and sys.argv[1] == '--launcher-child':
        launcher_child(sys.argv[2])
    elif len(sys.argv) == 2 and sys.argv[1] == '--focus-fixture':
        focus_fixture()
    else:
        main()

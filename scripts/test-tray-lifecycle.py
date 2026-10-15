#!/usr/bin/env python3
"""Native niri tray regression using disposable data and a private D-Bus session.

Prerequisites: niri, PyGObject with Gio/Soup 3, and a debug Kaze build embedding
frontend assets (--features tauri/custom-protocol). The WebKit inspector is
loopback-only and exists only in this test child. Uses silent WAV fixtures.

KAZE_PRIVATE_TRAY_TEST=1 dbus-run-session -- python3 scripts/test-tray-lifecycle.py /absolute/path/to/kaze
"""
import json
import os
import re
import socket
import sqlite3
import subprocess
import sys
import tempfile
import threading
import time
import urllib.request
import wave
from pathlib import Path

import gi
gi.require_version('Gio', '2.0')
gi.require_version('Soup', '3.0')
from gi.repository import Gio, GLib, Soup


def wait_for(test, message, timeout=12):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        try:
            result = test()
            if result:
                return result
        except Exception as error:
            last = error
        time.sleep(.04)
    raise AssertionError(f'{message}: {last}')


class Inspector:
    def __init__(self, port):
        def target():
            page = urllib.request.urlopen(f'http://127.0.0.1:{port}', timeout=1).read().decode()
            found = re.search(r"/socket/\d+/\d+/WebPage", page)
            return found.group(0) if found else None
        path = wait_for(target, 'WebKit inspector target')
        self.pending = {}
        self.target = None
        self.next_id = 0
        self.session = Soup.Session.new()
        done = threading.Event()
        error = []
        def connected(session, result, _data):
            try:
                self.ws = session.websocket_connect_finish(result)
                self.ws.connect('message', self.received)
            except Exception as problem:
                error.append(problem)
            done.set()
        self.session.websocket_connect_async(Soup.Message.new('GET', f'ws://127.0.0.1:{port}{path}'), None, None, GLib.PRIORITY_DEFAULT, None, connected, None)
        assert done.wait(5), 'inspector websocket connection timed out'
        if error:
            raise error[0]
        wait_for(lambda: self.target, 'inspector page target')
        wait_for(lambda: self.evaluate('typeof window.__TAURI_INTERNALS__') == 'object', 'Tauri frontend ready')

    def received(self, _ws, _kind, data):
        message = json.loads(data.get_data().decode())
        if message.get('method') == 'Target.targetCreated':
            self.target = message['params']['targetInfo']['targetId']
        if message.get('method') == 'Target.didCommitProvisionalTarget':
            self.target = message['params']['newTargetId']
        if message.get('method') == 'Target.dispatchMessageFromTarget':
            message = json.loads(message['params']['message'])
        if message.get('id') in self.pending:
            event, values = self.pending[message['id']]
            values.append(message)
            event.set()

    def evaluate(self, expression):
        self.next_id += 1
        event, values = threading.Event(), []
        self.pending[self.next_id] = (event, values)
        command = {'id': self.next_id, 'method': 'Runtime.evaluate', 'params': {'expression': expression, 'returnByValue': True}}
        self.ws.send_text(json.dumps({'id': self.next_id + 100000, 'method': 'Target.sendMessageToTarget', 'params': {'targetId': self.target, 'message': json.dumps(command)}}))
        if not event.wait(5):
            raise TimeoutError('Runtime.evaluate')
        del self.pending[self.next_id]
        message = values[0]
        if 'error' in message:
            raise RuntimeError(message['error'])
        result = message['result']
        if result.get('wasThrown'):
            raise RuntimeError(result)
        return result['result'].get('value')

    def invoke(self, method, args):
        expression = f"window.__trayResult=null; window.__TAURI_INTERNALS__.invoke({json.dumps(method)},{json.dumps(args)}).then(value=>window.__trayResult={{ok:true,value}}).catch(error=>window.__trayResult={{ok:false,error}}); true"
        self.evaluate(expression)
        result = wait_for(lambda: self.evaluate('window.__trayResult'), method)
        if not result['ok']:
            raise RuntimeError(result['error'])
        return result.get('value')

    def command(self, command, **payload):
        return self.invoke('execute_command', {'command': {'command': command, **({'payload': payload} if payload else {})}})

    def query(self, query):
        result = self.invoke('execute_query', {'query': {'query': query}})
        return result.get('data', result)

    def close(self):
        self.ws.close(1000, None)


class Watcher:
    XML = '''<node><interface name="org.kde.StatusNotifierWatcher">
    <method name="RegisterStatusNotifierItem"><arg direction="in" type="s"/></method>
    <method name="RegisterStatusNotifierHost"><arg direction="in" type="s"/></method>
    <property name="RegisteredStatusNotifierItems" type="as" access="read"/>
    <property name="IsStatusNotifierHostRegistered" type="b" access="read"/>
    <property name="ProtocolVersion" type="i" access="read"/>
    <signal name="StatusNotifierItemRegistered"><arg type="s"/></signal>
    </interface></node>'''

    def __init__(self):
        self.bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
        self.items = []
        self.host = True
        self.bus.register_object('/StatusNotifierWatcher', Gio.DBusNodeInfo.new_for_xml(self.XML).interfaces[0], self.method, self.property, None)
        self.call('org.freedesktop.DBus', '/org/freedesktop/DBus', 'org.freedesktop.DBus', 'RequestName', GLib.Variant('(su)', ('org.kde.StatusNotifierWatcher', 4)))

    def call(self, destination, path, interface, method, args=None):
        return self.bus.call_sync(destination, path, interface, method, args, None, Gio.DBusCallFlags.NONE, 3000, None).unpack()

    def method(self, _conn, sender, _path, _iface, name, args, reply):
        if name == 'RegisterStatusNotifierItem':
            path = args.unpack()[0]
            self.items.append((sender, path if path.startswith('/') else '/StatusNotifierItem'))
        reply.return_value(None)

    def property(self, _conn, _sender, _path, _iface, name):
        return {'RegisteredStatusNotifierItems': GLib.Variant('as', [s+p for s,p in self.items]), 'IsStatusNotifierHostRegistered': GLib.Variant('b', self.host), 'ProtocolVersion': GLib.Variant('i', 0)}[name]

    def owner(self):
        return self.call('org.freedesktop.DBus', '/org/freedesktop/DBus', 'org.freedesktop.DBus', 'NameHasOwner', GLib.Variant('(s)', ('org.mpris.MediaPlayer2.kaze',)))[0]

    def status(self):
        return self.call('org.mpris.MediaPlayer2.kaze', '/org/mpris/MediaPlayer2', 'org.freedesktop.DBus.Properties', 'Get', GLib.Variant('(ss)', ('org.mpris.MediaPlayer2.Player', 'PlaybackStatus')))[0]

    def item(self, pid):
        for sender, path in self.items:
            try:
                owner = self.call('org.freedesktop.DBus', '/org/freedesktop/DBus', 'org.freedesktop.DBus', 'GetConnectionUnixProcessID', GLib.Variant('(s)', (sender,)))[0]
                if owner == pid:
                    return sender, path
            except GLib.Error:
                pass
        return None

    def menu(self, pid, label):
        sender, path = wait_for(lambda: self.item(pid), 'native tray registration')
        menu = self.call(sender, path, 'org.freedesktop.DBus.Properties', 'Get', GLib.Variant('(ss)', ('org.kde.StatusNotifierItem', 'Menu')))[0]
        tree = self.call(sender, menu, 'com.canonical.dbusmenu', 'GetLayout', GLib.Variant('(iias)', (0, -1, ['label'])))[1]
        def find(node):
            if node[1].get('label', '').replace('_', '') == label:
                return node[0]
            for child in node[2]:
                result = find(child)
                if result is not None:
                    return result
        node = find(tree)
        assert node is not None, f'menu item {label!r} missing'
        self.call(sender, menu, 'com.canonical.dbusmenu', 'Event', GLib.Variant('(isvu)', (node, 'clicked', GLib.Variant('i', 0), 0)))


def windows(pid):
    return [window for window in json.loads(subprocess.check_output(['niri', 'msg', '-j', 'windows'], text=True)) if window.get('pid') == pid]


def main():
    if os.environ.get('KAZE_PRIVATE_TRAY_TEST') != '1':
        raise SystemExit('Run on a private dbus-run-session with KAZE_PRIVATE_TRAY_TEST=1')
    binary = str(Path(sys.argv[1]).resolve())
    watcher = Watcher()
    loop = GLib.MainLoop()
    threading.Thread(target=loop.run, daemon=True).start()
    processes, inspectors, logs = [], [], []
    with tempfile.TemporaryDirectory(prefix='kaze-native-tray-') as directory:
        root = Path(directory)
        def launch(inspect=True, require_tray=True):
            with socket.socket() as reservation:
                reservation.bind(('127.0.0.1', 0))
                port = reservation.getsockname()[1]
            env = dict(os.environ, XDG_DATA_HOME=str(root/'data'), XDG_CACHE_HOME=str(root/'cache'), WEBKIT_INSPECTOR_HTTP_SERVER=f'127.0.0.1:{port}', NO_PROXY='127.0.0.1,localhost', no_proxy='127.0.0.1,localhost', HTTPS_PROXY='http://127.0.0.1:9', HTTP_PROXY='http://127.0.0.1:9', NO_AT_BRIDGE='1', GIO_USE_VFS='local')
            log = (root/f'process-{len(processes)}.log').open('w+')
            logs.append(log)
            app = subprocess.Popen([binary], env=env, stdout=log, stderr=log)
            processes.append(app)
            wait_for(lambda: windows(app.pid), 'native main window')
            if require_tray:
                wait_for(lambda: watcher.item(app.pid), 'native tray')
            inspector = Inspector(port) if inspect else None
            if inspector:
                inspectors.append(inspector)
            return app, inspector

        def close_window(app):
            window = wait_for(lambda: windows(app.pid), 'window before close')[0]
            subprocess.run(['niri', 'msg', 'action', 'close-window', '--id', str(window['id'])], check=True, capture_output=True)

        def hidden(app):
            wait_for(lambda: not windows(app.pid), 'window hides')
            assert app.poll() is None, 'process exited instead of hiding'

        def quit_app(app):
            watcher.menu(app.pid, 'Quit Kaze')
            app.wait(timeout=12)
            assert app.returncode == 0

        try:
            warmup, browser = launch()
            quit_app(warmup)
            wait_for(lambda: not watcher.owner(), 'warmup releases MPRIS')
            database = root/'data/music-player/music_player.db'
            assert database.is_file()
            with sqlite3.connect(database) as db:
                for index in range(3):
                    path = root/f'track-{index}.wav'
                    with wave.open(str(path), 'wb') as audio:
                        audio.setnchannels(1); audio.setsampwidth(2); audio.setframerate(8000)
                        audio.writeframes(b'\x00\x00' * 8000 * 120)
                    db.execute("INSERT INTO tracks (id,file_path,file_size,modified_timestamp,title,normalized_title,duration_secs,format,created_at,updated_at) VALUES (?,?,?,1,?,?,120,'wav',1,1)", (f'track-{index}',str(path),path.stat().st_size,f'Tray Track {index}',f'tray track {index}'))
            app, browser = launch()
            assert watcher.owner()
            wait_for(lambda: browser.evaluate("Boolean([...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Settings'))"), 'React sidebar ready')
            browser.evaluate("[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Settings').click(); true")
            wait_for(lambda: browser.evaluate("document.body.innerText.includes('Configured Music Folders')"), 'Settings route ready')
            browser.evaluate("window.__trayAuditMarker='preserved'; window.__trayAuditMain=document.querySelector('main'); const spacer=document.createElement('div'); spacer.style.height='2000px'; window.__trayAuditMain.append(spacer); window.__trayAuditMain.scrollTop=180; true")
            for _ in range(10):
                close_window(app); hidden(app)
                watcher.menu(app.pid, 'Open Kaze')
                wait_for(lambda: len(windows(app.pid)) == 1, 'existing window restores')
                assert browser.evaluate("window.__trayAuditMarker==='preserved' && window.__trayAuditMain===document.querySelector('main') && document.querySelector('main').scrollTop===180 && document.body.innerText.includes('Configured Music Folders')")
            assert sum(1 for item in watcher.items if watcher.item(app.pid) == item) == 1
            print('PASS A/B/I: ten hide/show cycles, same webview/Settings route/scroll, one window and tray', flush=True)
            browser.command('PlayTrack', track_id='track-0', source='library')
            browser.command('EnqueueTrack', track_id='track-1', play_next=False)
            wait_for(lambda: watcher.status() == 'Playing', 'real backend starts silent WAV')
            close_window(app); hidden(app)
            watcher.menu(app.pid, 'Play/Pause')
            wait_for(lambda: watcher.status() == 'Paused', 'hidden tray Pause')
            watcher.menu(app.pid, 'Open Kaze')
            wait_for(lambda: windows(app.pid), 'restore paused')
            close_window(app); hidden(app)
            assert watcher.status() == 'Paused'
            watcher.menu(app.pid, 'Play/Pause')
            wait_for(lambda: watcher.status() == 'Playing', 'hidden tray Resume')
            watcher.menu(app.pid, 'Next')
            wait_for(lambda: browser.query('GetPlaybackState')['current_track_id'] == 'track-1', 'hidden Next')
            watcher.menu(app.pid, 'Previous')
            wait_for(lambda: browser.query('GetPlaybackState')['current_track_id'] == 'track-0', 'hidden Previous')
            browser.command('Seek', position_secs=12.0)
            print('PASS C/E: native hidden media controls, Paused retention, position and seeking', flush=True)
            browser.command('StartFlow', familiarity_ratio=.5)
            before = browser.query('GetFlowSession')
            assert before['is_active']
            watcher.menu(app.pid, 'Next')
            wait_for(lambda: watcher.status() == 'Playing', 'hidden Flow keeps playing')
            after = browser.query('GetFlowSession')
            assert after['is_active'] and before['generation'] == after['generation']
            print('PASS D: Flow active and advances while hidden without restarting its session', flush=True)
            quit_app(app)
            wait_for(lambda: not watcher.owner(), 'tray Quit releases MPRIS')
            print('PASS F/J: tray Quit exits cleanly and next launch reacquires MPRIS', flush=True)
            app, browser = launch()
            browser.command('SetCloseButtonBehavior', behavior='quit_app')
            close_window(app); app.wait(timeout=12)
            assert app.returncode == 0
            wait_for(lambda: not watcher.owner(), 'X Quit releases MPRIS')
            app, browser = launch()
            assert browser.query('GetSettings')['close_button_behavior'] == 'quit_app'
            browser.command('SetCloseButtonBehavior', behavior='minimize_to_tray')
            close_window(app); hidden(app)
            print('PASS G/H: X quits with saved quit_app; switching back hides immediately', flush=True)
            env_second = dict(os.environ, XDG_DATA_HOME=str(root/'data'), XDG_CACHE_HOME=str(root/'cache'), NO_AT_BRIDGE='1', GIO_USE_VFS='local')
            second = subprocess.Popen([binary], env=env_second, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            second.wait(timeout=10)
            assert second.returncode == 0, 'second process must exit 0'
            assert app.poll() is None, 'primary instance must remain alive'
            wait_for(lambda: windows(app.pid), 'primary instance window restored')
            owner = watcher.call('org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus','GetConnectionUnixProcessID',GLib.Variant('(s)',('org.mpris.MediaPlayer2.kaze',)))[0]
            assert owner == app.pid, 'primary instance retains MPRIS'
            print('PASS K: second launch restores hidden primary window; second process exits immediately', flush=True)
            quit_app(app)
            wait_for(lambda: not watcher.owner(), 'all instances quit')
            watcher.host = False
            app, browser = launch(require_tray=False)
            close_window(app); app.wait(timeout=12)
            assert app.returncode == 0
            print('PASS: absent tray host falls back to a clean quit rather than an inaccessible hidden app', flush=True)
            wait_for(lambda: not watcher.owner(), 'no-host quit releases MPRIS')
            watcher.host = True
            app, browser = launch()
            browser.command('PlayTrack', track_id='track-0', source='library')
            browser.command('Pause')
            def journal_count():
                with sqlite3.connect(database) as db:
                    return db.execute('SELECT count(*) FROM active_playback_journal').fetchone()[0]
            wait_for(lambda: journal_count() > 0, 'real session journal exists before forced kill')
            app.kill(); app.wait(timeout=5)
            wait_for(lambda: not watcher.owner(), 'OS releases bus name after forced kill')
            app, browser = launch()
            wait_for(lambda: journal_count() == 0, 'interrupted session recovery')
            quit_app(app)
            print('PASS: SIGKILL releases D-Bus ownership; immediate relaunch recovers the real journal', flush=True)
            with sqlite3.connect(database) as db:
                assert db.execute('SELECT count(*) FROM active_playback_journal').fetchone()[0] == 0
                assert db.execute('SELECT count(*) FROM playback_history WHERE completed=1').fetchone()[0] == 0
            print('PASS: shutdown drains SQLite journal without inventing completed tracks', flush=True)
        except Exception:
            for log in logs:
                log.flush(); log.seek(0)
                print(log.read()[-4000:], file=sys.stderr)
            raise
        finally:
            for inspector in inspectors:
                try: inspector.close()
                except Exception: pass
            for app in processes:
                if app.poll() is None:
                    try: watcher.menu(app.pid, 'Quit Kaze'); app.wait(timeout=8)
                    except Exception: app.terminate(); app.wait(timeout=5)
            for log in logs: log.close()
            loop.quit()


if __name__ == '__main__':
    main()

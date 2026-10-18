#!/usr/bin/env python3
"""Saved theme first-commit check in embedded WebKitGTK with real SQLite settings.

KAZE_PRIVATE_THEME_TEST=1 dbus-run-session -- python3 scripts/test-interface-startup-native.py /absolute/embedded/kaze
Uses disposable data and a private D-Bus; requires the existing native test helpers.
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

sys.dont_write_bytecode = True
BASE = Path(__file__).resolve().parent


def helper(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


tray = helper('theme_tray', BASE / 'test-tray-lifecycle.py')
protocol = helper('theme_protocol', BASE / 'profile-frontend-native.py').protocol

# Delay delivery of the real saved settings; no replacement backend or theme cache.
BOOTSTRAP = r"""
window.themeTrace = [];
window.introTrace = [];
window.settingsAttempts = 0;
new MutationObserver(() => {
  const intro = document.querySelector('.app-startup[data-startup-style]');
  if (intro && intro.dataset.startupStyle !== 'pending') window.introTrace.push(intro.dataset.startupStyle);
  const shell = document.querySelector('.kaze-interface');
  if (shell) window.themeTrace.push({root:document.documentElement.dataset.interface,shell:shell.dataset.interface});
}).observe(document, {subtree:true,childList:true,attributes:true,attributeFilter:['data-interface','data-startup-style']});
const define = Object.defineProperty;
Object.defineProperty = function(target,key,descriptor) {
  if (target===window.__TAURI_INTERNALS__ && key==='invoke' && typeof descriptor.value==='function') {
    const invoke = descriptor.value;
    descriptor = {...descriptor,value:function(method,args,...rest) {
      const response = invoke.call(this,method,args,...rest);
      if (method==='execute_query' && args.query.query==='GetSettings') {
        window.settingsAttempts++;
        return response.then(value => new Promise(resolve => {
          window.releaseSettings = () => resolve(value);
        }));
      }
      return response;
    }};
    Object.defineProperty = define;
  }
  return define.call(Object,target,key,descriptor);
};
"""


def main():
    if os.environ.get('KAZE_PRIVATE_THEME_TEST') != '1':
        raise SystemExit('Use explicit opt-in and a private dbus-run-session')
    binary = str(Path(sys.argv[1]).resolve())
    watcher = tray.Watcher()
    loop = tray.GLib.MainLoop()
    threading.Thread(target=loop.run, daemon=True).start()
    app = browser = None
    with tempfile.TemporaryDirectory(prefix='kaze-theme-startup-') as directory:
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
                tray.wait_for(lambda: browser.evaluate("Boolean(document.querySelector('.kaze-interface'))"), 'real shell', timeout=25)

            def quit_own():
                nonlocal app, browser
                if browser:
                    browser.close()
                    browser = None
                if app and app.poll() is None:
                    watcher.menu(app.pid, 'Quit Kaze')
                    app.wait(timeout=15)
                    assert app.returncode == 0
                tray.wait_for(lambda: not watcher.owner(), 'private MPRIS released')

            try:
                launch()
                quit_own()
                database = root/'data/music-player/music_player.db'
                for style in ['brutalist', 'spatial']:
                    with sqlite3.connect(database) as db:
                        db.execute("INSERT OR REPLACE INTO application_settings (key,value,updated_at) VALUES ('onboarding_completed','true',1)")
                        db.execute("INSERT OR REPLACE INTO application_settings (key,value,updated_at) VALUES ('interface_style',?,1)", (style,))
                    launch()
                    assert browser.evaluate('document.documentElement.dataset.interface') == style
                    protocol(browser, 'Page.enable', {})
                    protocol(browser, 'Page.setBootstrapScript', {'source': BOOTSTRAP})
                    browser.evaluate('location.reload();true')
                    tray.wait_for(lambda: browser.evaluate("typeof window.releaseSettings==='function'"), 'delayed real settings')
                    assert browser.evaluate("document.querySelector('.app-startup').dataset.startupStyle") == 'pending'
                    assert browser.evaluate("document.querySelector('.app-startup-brand').getAnimations({subtree:true}).length") == 0
                    assert not browser.evaluate("Boolean(document.querySelector('.kaze-interface'))"), 'default shell mounted early'
                    browser.evaluate('window.releaseSettings();true')
                    tray.wait_for(lambda: browser.evaluate("document.querySelector('.app-startup')?.dataset.startupStyle") == style, 'saved startup theme')
                    tray.wait_for(lambda: browser.evaluate("document.querySelector('.app-startup-icon')?.naturalWidth>0"), 'startup app icon')
                    windows = json.loads(subprocess.check_output(['niri', 'msg', '-j', 'windows'], text=True))
                    own = next(window for window in windows if window.get('pid') == app.pid)
                    subprocess.run(['niri', 'msg', 'action', 'focus-window', '--id', str(own['id'])], stdout=subprocess.DEVNULL, check=True)
                    tray.wait_for(lambda: browser.evaluate('document.hasFocus()'), 'focused startup')
                    indicator = '.app-startup-ring' if style == 'spatial' else '.app-startup-rail>span'
                    before = browser.evaluate(f"getComputedStyle(document.querySelector({json.dumps(indicator)})).transform")
                    browser.evaluate('window.introFrames=0;requestAnimationFrame(function tick(){window.introFrames++;if(window.introFrames<60)requestAnimationFrame(tick)});true')
                    time.sleep(.3)
                    after = browser.evaluate(f"getComputedStyle(document.querySelector({json.dumps(indicator)})).transform")
                    assert before != after and browser.evaluate('window.introFrames') > 2, 'startup animation is stalled'
                    assert browser.evaluate("Boolean(document.querySelector('.app-startup'))"), 'neutral startup screen missing'
                    assert not browser.evaluate("Boolean(document.querySelector('.kaze-interface'))"), 'default shell mounted early'
                    samples = []
                    for stamp in [0, 600, 900, 2000]:
                        browser.evaluate(f"document.querySelector('.app-startup-brand').getAnimations({{subtree:true}}).forEach(animation=>{{animation.pause();animation.currentTime={stamp}}});true")
                        samples.append(browser.evaluate("[...document.querySelectorAll('.app-startup-name>span')].map(letter=>{const style=getComputedStyle(letter);return {opacity:Number(style.opacity),x:new DOMMatrixReadOnly(style.transform).m41}})"))
                    assert all(letter['opacity'] == 0 for letter in samples[0])
                    assert all(samples[1][index]['opacity'] > samples[1][index+1]['opacity'] for index in range(3)), samples[1]
                    assert all(0 < letter['opacity'] < 1 and -12 < letter['x'] < 0 for letter in samples[2]), samples[2]
                    assert all(letter['opacity'] == 1 and abs(letter['x']) < .01 for letter in samples[3]), samples[3]
                    assert not browser.evaluate("Boolean(document.querySelector('.app-startup-cursor'))")
                    tray.wait_for(lambda: browser.evaluate("Boolean(document.querySelector('.home-flow'))"), 'hydrated Home')
                    trace = browser.evaluate('window.themeTrace')
                    assert trace and all(item == {'root': style, 'shell': style} for item in trace), json.dumps(trace)
                    intro_trace = browser.evaluate('window.introTrace')
                    assert intro_trace and all(value == style for value in intro_trace), intro_trace
                    assert browser.evaluate('window.settingsAttempts') == 1
                    assert not browser.evaluate("Boolean(document.querySelector('.app-startup'))"), 'startup did not unmount'
                    print(f'PASS {style}: native theme-specific motion, advancing frames and saved theme from first intro/shell commit', flush=True)
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

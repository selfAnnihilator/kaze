#!/usr/bin/env python3
"""Fresh-account setup / transient notices in ephemeral native WebKitGTK windows."""
import http.server
import json
import os
from pathlib import Path
import threading
import time
import urllib.request
import gi
gi.require_version('Gtk', '3.0')
gi.require_version('WebKit2', '4.1')
from gi.repository import GLib, Gtk, WebKit2
BASE = os.environ.get('KAZE_TEST_URL', 'http://127.0.0.1:5194').rstrip('/')
FIXTURE = Path(__file__).resolve().parent.parent.joinpath('tests/fixtures/dualInterfaceBackend.mjs').read_text().replace('export function installDualInterfaceFixture()', 'function installDualInterfaceFixture()')
API = b'export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return()=>window.__kazeFixture.listeners.delete(cb);};'
THEME = 'spatial'
def pump(seconds=.05):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        while GLib.MainContext.default().pending():
            GLib.MainContext.default().iteration(False)
        time.sleep(.005)
def main():
    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path.split('?')[0] == '/src/services/api.ts':
                body, content_type = API, 'text/javascript'
            else:
                with urllib.request.urlopen(BASE + self.path, timeout=10) as response:
                    body, content_type = response.read(), response.headers.get('Content-Type', 'application/octet-stream')
                if self.path == '/':
                    setup = f"<script>sessionStorage.setItem('test.interface-style','{THEME}');" + FIXTURE + ";installDualInterfaceFixture();" + """
                    const f=window.__kazeFixture,command=f.command.bind(f);
                    f.empty=true;f.flow.is_active=false;
                    Object.assign(f.playback,{current_track:null,current_track_id:null,current_queue_index:null,queue_track_ids:[],is_playing:false,is_paused:false});
                    f.musicTaste={status:'pending',artists:[],genres:[],revision:0,should_prompt:true};
                    f.command=async c=>{if(c.command==='SaveRecommendationSetup'){f.commands.push(c);f.musicTaste={...c.payload,status:c.payload.skipped?'skipped':'completed',revision:1,should_prompt:false};return{status:'Ok'};}return command(c);};</script>"""
                    body = body.replace(b'<head>', b'<head>' + setup.encode())
            self.send_response(200)
            self.send_header('Content-Type', content_type)
            self.end_headers()
            self.wfile.write(body)
        def log_message(self, *_args): pass
    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    window = None
    view = None
    def evaluate(expression):
        results = []
        def done(widget, task, *_args):
            try: results.append(json.loads(widget.evaluate_javascript_finish(task).to_json(0)))
            except Exception as error: results.append({'error': str(error)})
        view.evaluate_javascript(expression, -1, None, None, None, done, None)
        for _ in range(300):
            pump(.01)
            if results: return results[0]
        raise TimeoutError(expression)
    def wait(expression):
        for _ in range(300):
            if evaluate(expression) is True: return
            pump(.025)
        raise AssertionError(f'Expected native UI state: {expression}')
    global THEME
    try:
        for theme in ['spatial', 'brutalist']:
            THEME = theme
            for width, height in [(800, 600), (1440, 900)]:
                if window: window.destroy()
                window = Gtk.OffscreenWindow()
                window.set_default_size(width, height)
                view = WebKit2.WebView.new_with_context(WebKit2.WebContext.new_ephemeral())
                view.get_settings().set_hardware_acceleration_policy(WebKit2.HardwareAccelerationPolicy.NEVER)
                window.add(view)
                window.show_all()
                view.load_uri(f'http://127.0.0.1:{server.server_port}/')
                wait('!!document.querySelector(".taste-dialog[open]")')
                geometry = evaluate('''(() => {const r=document.querySelector('.taste-dialog').getBoundingClientRect();return {x:r.x,y:r.y,right:r.right,bottom:r.bottom,width:innerWidth,height:innerHeight};})()''')
                assert geometry['width'] == width and geometry['height'] == height, geometry
                assert geometry['x'] >= 0 and geometry['y'] >= 0 and geometry['right'] <= width + 1 and geometry['bottom'] <= height + 1, geometry
                evaluate("[...document.querySelectorAll('.taste-dialog button')].find(e=>e.textContent==='Rock').click();true")
                evaluate("document.querySelector('.taste-actions .btn-primary').click();true")
                wait('!document.querySelector(".taste-dialog")')
                assert evaluate('window.__kazeFixture.musicTaste.genres.includes("Rock")') is True
                evaluate("document.querySelector('.kaze-interface > .player-bar .play-pause-btn,.press-play-main').click();true")
                wait('!!document.querySelector(".playback-notice")')
                assert evaluate('window.__kazeFixture.playback.is_playing') is False
                assert evaluate("window.__kazeFixture.commands.filter(c=>['Resume','TogglePlayPause'].includes(c.command)).length") == 0
                pump(2.2)
                assert evaluate('!document.querySelector(".playback-notice")') is True
                evaluate("window.__kazeFixture.emit({event:'RecommendationFeedbackChanged',payload:{user_id:'u1',track_id:'t1',track_ids:['t1'],title:'Feather',disliked:true,manual_like:-1}});true")
                wait('!!document.querySelector(".recommendation-feedback-toast")')
                pump(5.3)
                assert evaluate('!document.querySelector(".recommendation-feedback-toast")') is True
                assert evaluate("window.__kazeFixture.commands.filter(c=>c.command==='RemoveTrackFeedback').length") == 0
                print(f'PASS WebKitGTK {theme} {width}x{height}: taste dialog, empty Play and 2s/5s notice expiry', flush=True)
    finally:
        if window: window.destroy()
        server.shutdown()
        server.server_close()
if __name__ == '__main__': main()

#!/usr/bin/env python3
"""Real App/CSS narrow-window geometry in an ephemeral offscreen WebKitGTK view.
Requires Vite at KAZE_TEST_URL, Gtk3/WebKit2 4.1 and a display; no real app/data.
"""
import http.server
import json
import os
from pathlib import Path
import threading
import time
import urllib.request
import gi

gi.require_version('Gtk','3.0')
gi.require_version('WebKit2','4.1')
from gi.repository import GLib, Gtk, WebKit2

BASE=os.environ.get('KAZE_TEST_URL','http://127.0.0.1:5196').rstrip('/')
FIXTURE=Path(__file__).resolve().parent.parent.joinpath('tests/fixtures/dualInterfaceBackend.mjs').read_text().replace('export function installDualInterfaceFixture()', 'function installDualInterfaceFixture()')
API=b'export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return()=>window.__kazeFixture.listeners.delete(cb);};'

def pump(seconds=.05):
    end=time.monotonic()+seconds
    while time.monotonic()<end:
        while GLib.MainContext.default().pending():GLib.MainContext.default().iteration(False)
        time.sleep(.005)

def main():
    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path.split('?')[0]=='/src/services/api.ts':
                body=API;content_type='text/javascript'
            else:
                try:
                    with urllib.request.urlopen(BASE+self.path,timeout=10) as response:
                        body=response.read();content_type=response.headers.get('Content-Type','application/octet-stream')
                except Exception:
                    self.send_error(502);return
                if self.path=='/':
                    setup="<script>sessionStorage.setItem('test.interface-style','spatial');"+FIXTURE+";installDualInterfaceFixture();const originalQuery=window.__kazeFixture.query.bind(window.__kazeFixture);window.__kazeFixture.query=async q=>{const r=await originalQuery(q);if(q.query==='GetDiscoveryRecommendations')r.data=r.data.map(t=>({...t,title:'New '+t.title,recommendation_reason:'Based on your listening habit'}));return r;};</script>"
                    body=body.replace(b'<head>',b'<head>'+setup.encode())
            self.send_response(200);self.send_header('Content-Type',content_type);self.end_headers();self.wfile.write(body)
        def log_message(self,*_args):pass
    server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler)
    threading.Thread(target=server.serve_forever,daemon=True).start()
    window=Gtk.OffscreenWindow();window.set_default_size(940,1000)
    view=WebKit2.WebView.new_with_context(WebKit2.WebContext.new_ephemeral())
    view.get_settings().set_hardware_acceleration_policy(WebKit2.HardwareAccelerationPolicy.NEVER)
    window.add(view);window.show_all()
    def evaluate(expression):
        results=[]
        def done(widget,task,*_args):
            try:results.append(json.loads(widget.evaluate_javascript_finish(task).to_json(0)))
            except Exception as error:results.append({'error':str(error)})
        view.evaluate_javascript(expression,-1,None,None,None,done,None)
        for _ in range(200):
            pump(.01)
            if results:return results[0]
        raise TimeoutError(expression)
    def wait(expression):
        for _ in range(300):
            if evaluate(expression) is True:return
            pump(.05)
        raise AssertionError(f'App did not reach expected state: {expression}')
    try:
        view.load_uri(f'http://127.0.0.1:{server.server_port}/')
        wait('!!document.querySelector(".spatial-navigation")')
        evaluate("document.querySelector('.spatial-navigation button[aria-label=\"Discover Music\"]').click();true")
        wait('document.querySelectorAll(".chapter-nav button").length===4')
        for width,height in [(940,1000),(800,1000),(1024,600),(1440,900)]:
            if width != 940:
                # Offscreen windows do not reliably apply compositor resize requests.
                # Use a new real viewport rather than measuring the previous width.
                window.destroy()
                window=Gtk.OffscreenWindow();window.set_default_size(width,height)
                view=WebKit2.WebView.new_with_context(WebKit2.WebContext.new_ephemeral())
                view.get_settings().set_hardware_acceleration_policy(WebKit2.HardwareAccelerationPolicy.NEVER)
                window.add(view);window.show_all()
                view.load_uri(f'http://127.0.0.1:{server.server_port}/')
                wait('!!document.querySelector(".spatial-navigation")')
                evaluate("document.querySelector('.spatial-navigation button[aria-label=\"Discover Music\"]').click();true")
                wait('document.querySelectorAll(".chapter-nav button").length===4')
            for chapter in range(4):
                evaluate(f'document.querySelectorAll(".chapter-nav button")[{chapter}].click();true');pump(.15)
                geometry=evaluate('''(() => {
                    const rect=s=>{const r=document.querySelector(s).getBoundingClientRect();return {x:r.x,y:r.y,right:r.right,bottom:r.bottom,width:r.width,height:r.height}};
                    return {viewport:innerWidth,art:rect('.spatial-discovery-cover'),column:rect('.spatial-discovery-artwork'),panel:rect('.spatial-discovery-panel'),strip:rect('.discovery-chapter .horizontal-scroll-row')};
                })()''')
                assert geometry['viewport']==width,geometry
                assert geometry['art']['right']<=geometry['panel']['x']-8,f'{width}x{height} chapter {chapter}: preview overlaps right panel: {geometry}'
                assert geometry['art']['width']>180,geometry
                assert geometry['strip']['x']>=geometry['panel']['x']-1,geometry
            print(f'PASS WebKitGTK {width}x{height}: all four real App chapters keep the cover and song shelf in separate columns',flush=True)
    finally:
        window.destroy();server.shutdown();server.server_close()
if __name__=='__main__':main()

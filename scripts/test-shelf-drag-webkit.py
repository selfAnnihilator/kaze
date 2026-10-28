#!/usr/bin/env python3
"""Trusted WebKitGTK shelf gestures in an isolated offscreen renderer.
Requires Vite (KAZE_TEST_URL), Gtk3/WebKit2 4.1 and a display. No live profile.
"""
import http.server
import json
import os
import threading
import time
import gi

gi.require_version('Gtk', '3.0')
gi.require_version('Gdk', '3.0')
gi.require_version('WebKit2', '4.1')
from gi.repository import Gdk, GLib, Gtk, WebKit2

HTML = '''<!doctype html><html><head><style>
body {margin:30px} .horizontal-scroll-row {overflow-x:auto;width:640px;height:240px;scroll-behavior:smooth;user-select:none;-webkit-user-select:none} .inner {display:flex;width:2400px} button {flex:0 0 180px;height:210px} img {width:160px;height:160px}
</style></head><body><div id="root" class="kaze-interface"></div><script type="module">
import RefreshRuntime from '$BASE/@react-refresh';
RefreshRuntime.injectIntoGlobalHook(window); window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>type=>type;window.__vite_plugin_react_preamble_installed__=true;
const React=(await import('$BASE/node_modules/.vite/deps/react.js')).default;
const {createRoot}=(await import('$BASE/node_modules/.vite/deps/react-dom_client.js')).default;
const {useHorizontalDrag}=await import('$BASE/src/presentation/useHorizontalDrag.ts');
window.clicks=0;window.events=[];
for(const name of ['pointerdown','pointermove','pointerup'])document.addEventListener(name,e=>events.push([name,e.isTrusted]),true);
function App(){useHorizontalDrag();return React.createElement('div',{className:'horizontal-scroll-row'},React.createElement('div',{className:'inner'},Array.from({length:13},(_,i)=>React.createElement('button',{key:i,onClick:()=>clicks++},React.createElement('img',{src:'data:image/svg+xml,'+encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="160" height="160"><rect width="160" height="160" fill="coral"/></svg>')}),'Album '+i))));}
createRoot(document.getElementById('root')).render(React.createElement(App));
</script></body></html>'''

def pump(seconds=.04):
    end=time.monotonic()+seconds
    while time.monotonic()<end:
        while GLib.MainContext.default().pending(): GLib.MainContext.default().iteration(False)
        time.sleep(.005)

def main():
    html=HTML.replace('$BASE',os.environ.get('KAZE_TEST_URL','http://127.0.0.1:5196').rstrip('/'))
    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            self.send_response(200);self.send_header('Content-Type','text/html');self.end_headers();self.wfile.write(html.encode())
        def log_message(self,*_args): pass
    server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler)
    threading.Thread(target=server.serve_forever,daemon=True).start()
    window=Gtk.OffscreenWindow();window.set_default_size(800,350)
    view=WebKit2.WebView();view.get_settings().set_hardware_acceleration_policy(WebKit2.HardwareAccelerationPolicy.NEVER)
    window.add(view);window.show_all()
    def evaluate(expression):
        result=[]
        def done(widget,task,*_args):
            try: result.append(json.loads(widget.evaluate_javascript_finish(task).to_json(0)))
            except Exception as error: result.append({'error':str(error)})
        view.evaluate_javascript(expression,-1,None,None,None,done,None)
        for _ in range(200):
            pump(.01)
            if result:return result[0]
        raise TimeoutError(expression)
    device=Gdk.Display.get_default().get_default_seat().get_pointer()
    def send(kind,x,y=100,pressed=False):
        event=Gdk.Event.new(kind);event.window=view.get_window();event.send_event=True;event.time=Gdk.CURRENT_TIME
        event.x,event.y=x,y;event.x_root,event.y_root=x,y;event.set_device(device)
        event.state=Gdk.ModifierType.BUTTON1_MASK if pressed else Gdk.ModifierType(0)
        if kind in (Gdk.EventType.BUTTON_PRESS,Gdk.EventType.BUTTON_RELEASE):event.button=1
        view.event(event);pump()
    snapshot='({left:document.querySelector(".horizontal-scroll-row").scrollLeft,clicks,dragging:document.querySelector(".horizontal-scroll-row").hasAttribute("data-dragging"),behavior:document.querySelector(".horizontal-scroll-row").style.scrollBehavior})'
    try:
        view.load_uri(f'http://127.0.0.1:{server.server_port}')
        for _ in range(200):
            pump(.03)
            if evaluate('!!document.querySelector("button")'):break
        else:raise AssertionError('shelf did not mount; check Vite')
        send(Gdk.EventType.MOTION_NOTIFY,520)
        send(Gdk.EventType.BUTTON_PRESS,520)
        for x in range(500,99,-20):send(Gdk.EventType.MOTION_NOTIFY,x,pressed=True)
        during=evaluate(snapshot);assert during['left']>=350 and during['dragging'],during
        send(Gdk.EventType.BUTTON_RELEASE,100,pressed=True);pump(.1)
        after=evaluate(snapshot);assert after['left']>=350 and after['clicks']==0 and not after['dragging'] and after['behavior']=='',after
        assert evaluate('events.every(e=>e[1])'),'must use trusted native pointer events'
        print('PASS WebKitGTK: dragging actual images scrolls immediately, captures release and suppresses card activation')
        send(Gdk.EventType.MOTION_NOTIFY,120);send(Gdk.EventType.BUTTON_PRESS,120);send(Gdk.EventType.BUTTON_RELEASE,120,pressed=True)
        assert evaluate('clicks')==1,'normal click after dragging must activate once'
        print('PASS WebKitGTK: next ordinary click still activates the card')
        send(Gdk.EventType.BUTTON_PRESS,500);send(Gdk.EventType.MOTION_NOTIFY,400,pressed=True)
        evaluate('window.dispatchEvent(new Event("blur"));true')
        cancelled=evaluate(snapshot);assert not cancelled['dragging'] and cancelled['behavior']=='',cancelled
        send(Gdk.EventType.BUTTON_RELEASE,400,pressed=True)
        assert evaluate('clicks')==1,'cancelled drag must not activate'
        print('PASS WebKitGTK: focus loss cancels dragging and restores scroll behavior')
    finally:
        window.destroy();server.shutdown();server.server_close()
if __name__=='__main__':main()

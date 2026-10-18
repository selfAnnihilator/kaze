#!/usr/bin/env python3
"""Native WebKitGTK drag regression, using an offscreen control and trusted GDK events.
Requires a running Vite server (KAZE_TEST_URL, default http://127.0.0.1:5194),
PyGObject/Gtk3/WebKit2 4.1 and an available display. Never opens the user's player.
"""
import http.server
import json
import os
import threading
import time

import gi

gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
gi.require_version("WebKit2", "4.1")
from gi.repository import Gdk, GLib, Gtk, WebKit2

HTML = """<!doctype html><html><head>
<link rel="stylesheet" href="http://127.0.0.1:5194/src/presentation/brutalist.css">
</head><body style="margin:40px;background:#111;color:#eee">
<div id="root" class="kaze-interface" data-interface="brutalist"></div>
<script type="module">
import RefreshRuntime from 'http://127.0.0.1:5194/@react-refresh';
RefreshRuntime.injectIntoGlobalHook(window);
window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>type=>type;
window.__vite_plugin_react_preamble_installed__=true;
const React=(await import('http://127.0.0.1:5194/node_modules/.vite/deps/react.js')).default;
const {createRoot}=(await import('http://127.0.0.1:5194/node_modules/.vite/deps/react-dom_client.js')).default;
const {FamiliarityBar}=await import('http://127.0.0.1:5194/src/presentation/FamiliarityBar.tsx');
window.commits=[];window.events=[];
for(const name of ['pointerdown','pointermove','pointerup'])
  document.addEventListener(name,e=>events.push([name,e.isTrusted]),true);
function App(){
  const [value,setValue]=React.useState(.5);
  return React.createElement(FamiliarityBar,{value,onChange:v=>{commits.push(v);setValue(v)}});
}
createRoot(document.getElementById('root')).render(React.createElement(App));
</script></body></html>"""


def pump(seconds=.05):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        while GLib.MainContext.default().pending():
            GLib.MainContext.default().iteration(False)
        time.sleep(.005)


def main():
    html = HTML.replace("http://127.0.0.1:5194", os.environ.get("KAZE_TEST_URL", "http://127.0.0.1:5194").rstrip("/"))

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            self.send_response(200)
            self.send_header("Content-Type", "text/html")
            self.end_headers()
            self.wfile.write(html.encode())

        def log_message(self, *_args):
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    window = Gtk.OffscreenWindow()
    window.set_default_size(800, 350)
    view = WebKit2.WebView()
    # An offscreen widget has no hardware rendering target. Production is unchanged.
    view.get_settings().set_hardware_acceleration_policy(WebKit2.HardwareAccelerationPolicy.NEVER)
    window.add(view)
    window.show_all()

    def evaluate(expression):
        result = []

        def done(widget, task, *_args):
            try:
                result.append(json.loads(widget.evaluate_javascript_finish(task).to_json(0)))
            except Exception as error:
                result.append({"error": str(error)})

        view.evaluate_javascript(expression, -1, None, None, None, done, None)
        for _ in range(200):
            pump(.01)
            if result:
                return result[0]
        raise TimeoutError(expression)

    try:
        view.load_uri(f"http://127.0.0.1:{server.server_port}")
        for _ in range(200):
            pump(.05)
            if evaluate('Boolean(document.querySelector("#press-flow-steering"))'):
                break
        else:
            raise AssertionError("control did not mount; check Vite is running")
        box = evaluate('''(() => {
            const r=document.querySelector('#press-flow-steering').getBoundingClientRect();
            return {x:r.x,y:r.y,w:r.width,h:r.height};
        })()''')
        device = Gdk.Display.get_default().get_default_seat().get_pointer()

        def send(kind, fraction, pressed=False, outside=False):
            event = Gdk.Event.new(kind)
            event.window = view.get_window()
            event.send_event = True
            event.time = Gdk.CURRENT_TIME
            event.x = box["x"] + box["w"] * fraction
            event.y = box["y"] + box["h"] * (.5 if not outside else 2)
            event.x_root, event.y_root = event.x, event.y
            event.set_device(device)
            event.state = Gdk.ModifierType.BUTTON1_MASK if pressed else Gdk.ModifierType(0)
            if kind in (Gdk.EventType.BUTTON_PRESS, Gdk.EventType.BUTTON_RELEASE):
                event.button = 1
            view.event(event)
            pump(.06)

        def snapshot():
            return evaluate('({value:Number(document.querySelector("#press-flow-steering").value),commits:[...commits]})')

        send(Gdk.EventType.MOTION_NOTIFY, .5)
        send(Gdk.EventType.BUTTON_PRESS, .5)
        for step in range(1, 11):
            send(Gdk.EventType.MOTION_NOTIFY, .5 + .3 * step / 10, True)
        during = snapshot()
        assert during["value"] >= .7, f"native drag must preview its new value: {during}"
        assert during["commits"] == [], "preview must not persist during drag"
        send(Gdk.EventType.BUTTON_RELEASE, .8, True)
        after = snapshot()
        assert after["commits"] == [.8], f"release commits once: {after}"
        assert evaluate("events.every(event=>event[1])"), "test must exercise trusted native events"
        print("PASS WebKitGTK: trusted mouse drag previews live and commits once on release")

        send(Gdk.EventType.BUTTON_PRESS, .8)
        send(Gdk.EventType.MOTION_NOTIFY, -.1, True, outside=True)
        assert snapshot() == {"value": 0, "commits": [.8]}
        send(Gdk.EventType.BUTTON_RELEASE, -.1, True, outside=True)
        assert snapshot() == {"value": 0, "commits": [.8, 0]}
        print("PASS WebKitGTK: capture preserves outside dragging/release and clamps the endpoint")

        send(Gdk.EventType.BUTTON_PRESS, .3)
        send(Gdk.EventType.MOTION_NOTIFY, .7, True)
        evaluate('document.querySelector("#press-flow-steering").blur();true')
        assert snapshot() == {"value": 0, "commits": [.8, 0]}
        send(Gdk.EventType.BUTTON_RELEASE, .7, True)
        assert snapshot() == {"value": 0, "commits": [.8, 0]}
        print("PASS WebKitGTK: cancelling via blur restores the controlled value without committing")
    finally:
        window.destroy()
        server.shutdown()


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Repeated startup reduced-motion check in ephemeral offscreen WebKitGTK windows."""

from qa_support import open_loopback_url

import http.server
import json
import os
from pathlib import Path
import threading
import time
import gi

gi.require_version("Gtk", "3.0")
gi.require_version("WebKit2", "4.1")
from gi.repository import GLib, Gtk, WebKit2  # noqa: E402 -- select GI versions before importing repositories.

BASE = os.environ.get("KAZE_TEST_URL", "http://127.0.0.1:5194").rstrip("/")
FIXTURE = (
    Path(__file__)
    .resolve()
    .parent.parent.joinpath("tests/fixtures/dualInterfaceBackend.mjs")
    .read_text()
    .replace(
        "export function installDualInterfaceFixture()",
        "function installDualInterfaceFixture()",
    )
)
API = b"export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return()=>window.__kazeFixture.listeners.delete(cb);};"
THEME = "spatial"
APP_GATE = threading.Event()


def pump(seconds=0.05):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        while GLib.MainContext.default().pending():
            GLib.MainContext.default().iteration(False)
        time.sleep(0.005)


def main():
    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path.split("?")[0] == "/src/App.tsx":
                APP_GATE.wait(timeout=8)
            if self.path.split("?")[0] == "/src/services/api.ts":
                body, content_type = API, "text/javascript"
            else:
                with open_loopback_url(BASE + self.path, timeout=10) as response:
                    body, content_type = (
                        response.read(),
                        response.headers.get(
                            "Content-Type", "application/octet-stream"
                        ),
                    )
                if self.path == "/":
                    setup = (
                        f"<script>sessionStorage.setItem('test.interface-style','{THEME}');"
                        + FIXTURE
                        + ";installDualInterfaceFixture();"
                        + """
                    const f=window.__kazeFixture,q=f.query.bind(f);
                    f.query=async r=>{const result=await q(r);if(r.query==='GetSettings')await new Promise(resolve=>setTimeout(resolve,500));return result;};</script>"""
                    )
                    body = body.replace(b"<head>", b"<head>" + setup.encode())
            self.send_response(200)
            self.send_header("Content-Type", content_type)
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *_args):
            # Routine fixture access logs are intentionally quiet.
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    window = None
    view = None

    def evaluate(expression):
        results = []

        def done(widget, task, *_args):
            try:
                results.append(
                    json.loads(widget.evaluate_javascript_finish(task).to_json(0))
                )
            except Exception as error:
                results.append({"error": str(error)})

        view.evaluate_javascript(expression, -1, None, None, None, done, None)
        for _ in range(300):
            pump(0.01)
            if results:
                return results[0]
        raise TimeoutError(expression)

    def wait(expression):
        for _ in range(300):
            if evaluate(expression) is True:
                return
            pump(0.025)
        raise AssertionError(f"Expected native UI state: {expression}")

    global THEME
    try:
        Gtk.Settings.get_default().set_property("gtk-enable-animations", False)
        for theme in ["spatial", "brutalist"]:
            THEME = theme
            for repeat in range(int(os.environ.get("KAZE_STARTUP_REPEATS", "5"))):
                if window:
                    window.destroy()
                APP_GATE.clear()
                window = Gtk.OffscreenWindow()
                window.set_default_size(1000, 700)
                view = WebKit2.WebView.new_with_context(
                    WebKit2.WebContext.new_ephemeral()
                )
                view.get_settings().set_hardware_acceleration_policy(
                    WebKit2.HardwareAccelerationPolicy.NEVER
                )
                window.add(view)
                window.show_all()
                view.load_uri(f"http://127.0.0.1:{server.server_port}/")
                wait('!!document.querySelector(".app-startup-brand")')
                assert (
                    evaluate("matchMedia('(prefers-reduced-motion: reduce)').matches")
                    is True
                ), "WebKit did not receive reduced-motion preference"
                wait(
                    "document.querySelector('.app-startup')?.dataset.startupStyle === "
                    + json.dumps(theme)
                )
                for _ in range(10):
                    active = evaluate(
                        "document.querySelector('.app-startup-brand').getAnimations({subtree:true}).map(a=>({name:a.animationName,property:a.transitionProperty,state:a.playState,target:a.effect.target?.className,computed:a.effect.target?getComputedStyle(a.effect.target).transitionProperty:null,duration:a.effect.target?getComputedStyle(a.effect.target).transitionDuration:null}))"
                    )
                    assert active == [], active
                    css = evaluate(
                        "(() => {const s=getComputedStyle(document.querySelector('.app-startup-brand'));return {transition:s.transitionProperty,animation:s.animationName};})()"
                    )
                    assert css == {"transition": "none", "animation": "none"}, css
                    pump(0.03)
                APP_GATE.set()
                wait('!!document.querySelector(".kaze-interface")')
                print(
                    f"PASS reduced motion WebKitGTK {theme} repeat {repeat + 1}: native preference, no active transition/animation, usable App",
                    flush=True,
                )
    finally:
        APP_GATE.set()
        if window:
            window.destroy()
        server.shutdown()
        server.server_close()


if __name__ == "__main__":
    main()

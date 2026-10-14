#!/usr/bin/env python3
"""Exercise the actual Navbar MPRIS section against disposable D-Bus players.

Run: KAZE_PRIVATE_MPRIS_TEST=1 dbus-run-session -- python3
     scripts/test-status-bar-mpris.py /absolute/path/to/Navbar.qml
Requires system PyGObject, Quickshell and Qt's offscreen platform.
Never registers test players on the user's real session bus.
"""
import os
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib

XML = """<node>
<interface name="org.mpris.MediaPlayer2">
 <property name="Identity" type="s" access="read"/>
 <property name="DesktopEntry" type="s" access="read"/>
 <property name="CanQuit" type="b" access="read"/>
 <property name="CanRaise" type="b" access="read"/>
 <property name="HasTrackList" type="b" access="read"/>
 <property name="SupportedUriSchemes" type="as" access="read"/>
 <property name="SupportedMimeTypes" type="as" access="read"/>
</interface>
<interface name="org.mpris.MediaPlayer2.Player">
 <property name="PlaybackStatus" type="s" access="read"/>
 <property name="Metadata" type="a{sv}" access="read"/>
 <property name="CanControl" type="b" access="read"/>
 <property name="CanPlay" type="b" access="read"/>
 <property name="CanPause" type="b" access="read"/>
 <property name="CanGoNext" type="b" access="read"/>
 <property name="CanGoPrevious" type="b" access="read"/>
 <property name="CanSeek" type="b" access="read"/>
 <property name="Position" type="x" access="read"/>
 <property name="Volume" type="d" access="read"/>
 <property name="Rate" type="d" access="read"/>
 <property name="MinimumRate" type="d" access="read"/>
 <property name="MaximumRate" type="d" access="read"/>
 <property name="LoopStatus" type="s" access="read"/>
 <property name="Shuffle" type="b" access="read"/>
 <signal name="Seeked"><arg type="x"/></signal>
</interface></node>"""
PATH = "/org/mpris/MediaPlayer2"
PLAYER = "org.mpris.MediaPlayer2.Player"


class Player:
    def __init__(self, suffix, entry, state, title=""):
        self.props = {
            "Identity": GLib.Variant("s", "Kaze" if "kaze" in suffix else entry),
            "DesktopEntry": GLib.Variant("s", entry),
            "PlaybackStatus": GLib.Variant("s", state),
            "Metadata": GLib.Variant("a{sv}", {"xesam:title": GLib.Variant("s", title)}),
            "Position": GLib.Variant("x", 0),
            "LoopStatus": GLib.Variant("s", "None"),
            "SupportedUriSchemes": GLib.Variant("as", []),
            "SupportedMimeTypes": GLib.Variant("as", []),
        }
        for key in ["CanControl", "CanPlay", "CanPause", "CanGoNext", "CanGoPrevious", "CanSeek"]:
            self.props[key] = GLib.Variant("b", True)
        for key in ["CanQuit", "CanRaise", "HasTrackList", "Shuffle"]:
            self.props[key] = GLib.Variant("b", False)
        for key in ["Volume", "Rate", "MinimumRate", "MaximumRate"]:
            self.props[key] = GLib.Variant("d", 1.0)
        self.conn = Gio.DBusConnection.new_for_address_sync(
            os.environ["DBUS_SESSION_BUS_ADDRESS"],
            Gio.DBusConnectionFlags.AUTHENTICATION_CLIENT | Gio.DBusConnectionFlags.MESSAGE_BUS_CONNECTION,
            None, None,
        )
        self.conn.set_exit_on_close(False)
        for iface in Gio.DBusNodeInfo.new_for_xml(XML).interfaces:
            self.conn.register_object(PATH, iface, None,
                lambda _conn, _sender, _path, _iface, key: self.props[key], None)
        self.conn.call_sync("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus",
            "RequestName", GLib.Variant("(su)", ("org.mpris.MediaPlayer2." + suffix, 4)), None,
            Gio.DBusCallFlags.NONE, 2000, None)

    def state(self, state):
        self.props["PlaybackStatus"] = GLib.Variant("s", state)
        self.conn.emit_signal(None, PATH, "org.freedesktop.DBus.Properties", "PropertiesChanged",
            GLib.Variant("(sa{sv}as)", (PLAYER, {"PlaybackStatus": self.props["PlaybackStatus"]}, [])))
        self.conn.flush_sync(None)

    def close(self):
        self.conn.close_sync(None)


def main():
    if os.environ.get("KAZE_PRIVATE_MPRIS_TEST") != "1":
        raise SystemExit("Run inside a private dbus-run-session with KAZE_PRIVATE_MPRIS_TEST=1")
    if sys.argv[1:] == ["--hold-kaze"]:
        player = Player("kaze", "kaze", "Stopped")
        GLib.MainLoop().run()
        return
    source = Path(sys.argv[1]).resolve()
    section = source.read_text().split("    // ---------- MPRIS (now playing) ----------", 1)[1]
    # Preserve the complete selector, signal subscriptions and music IPC.
    qml = "import QtQuick\nimport Quickshell\nimport Quickshell.Io\nimport Quickshell.Services.Mpris\nShellRoot {\n Item {\n id: root\n" + section + "\n}\n"
    loop = GLib.MainLoop()
    thread = threading.Thread(target=loop.run, daemon=True)
    thread.start()
    players = []
    shell = crash = None
    with tempfile.TemporaryDirectory(prefix="kaze-bar-mpris-") as temp:
        config = Path(temp) / "shell.qml"
        config.write_text(qml)
        env = dict(os.environ, QT_QPA_PLATFORM="offscreen")
        with (Path(temp) / "quickshell.log").open("w+") as log:
            def query(method):
                result = subprocess.run(["qs", "-p", str(config), "ipc", "call", "music", method],
                    env=env, capture_output=True, text=True, timeout=3)
                return result.stdout.strip() if result.returncode == 0 else "<not-ready>"

            def expect(method, expected):
                deadline = time.monotonic() + 5
                while time.monotonic() < deadline:
                    actual = query(method)
                    if actual == expected:
                        return
                    time.sleep(0.05)
                log.flush()
                log.seek(0)
                raise AssertionError(f"{method}: expected {expected!r}, got {actual!r}\n{log.read()}")

            try:
                shell = subprocess.Popen(["qs", "-p", str(config)], env=env, stdout=log, stderr=log)
                expect("player", "none")
                mpd = Player("mpd", "mpd-mpris", "Paused", "MPD track")
                players.append(mpd)
                expect("player", "mpd-mpris")
                browser = Player("firefox.instance_test", "zen", "Playing", "Browser track")
                players.append(browser)
                expect("player", "zen")
                impostor = Player("kazette", "kaze", "Stopped")
                players.append(impostor)
                expect("player", "zen")
                kaze = Player("kaze", "kaze", "Stopped")
                players.append(kaze)
                expect("player", "kaze")
                expect("title", "Kaze")
                expect("playing", "false")
                for state in ["Playing", "Paused", "Stopped", "Paused"]:
                    kaze.state(state)
                    expect("playing", "true" if state == "Playing" else "false")
                    mpd.state("Playing")
                    browser.state("Paused")
                    browser.state("Playing")
                    expect("player", "kaze")
                kaze.close()
                players.remove(kaze)
                expect("player", "mpd-mpris")  # MPD now playing, first in original list.
                crash = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "--hold-kaze"], env=env)
                expect("player", "kaze")
                crash.kill()
                crash.wait(timeout=3)
                expect("player", "mpd-mpris")
                relaunched = Player("kaze", "kaze", "Stopped")
                players.append(relaunched)
                expect("player", "kaze")
                relaunched.close()
                players.remove(relaunched)
                expect("player", "mpd-mpris")
                for player in players:
                    player.close()
                players.clear()
                expect("player", "none")
                expect("title", "")
                print("PASS: real Quickshell selector: empty bus, MPD/browser fallback, exact service matching, all Kaze states, competitors, graceful close, SIGKILL, relaunch, final removal")
            finally:
                if crash and crash.poll() is None:
                    crash.kill()
                    crash.wait(timeout=3)
                if shell:
                    shell.terminate()
                    shell.wait(timeout=3)
                for player in players:
                    player.close()
                loop.quit()


if __name__ == "__main__":
    main()

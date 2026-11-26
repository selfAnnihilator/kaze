#!/usr/bin/env python3
"""Exercise Kaze's real native media service in a separate, silent QA package."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess  # nosec B404 - local device-test driver with explicit argument vectors
import time

PACKAGE = "org.intelligentmusicplayer.lifecycleqa"
COMPONENT = f"{PACKAGE}/.MainActivity"
ROOT = Path(__file__).resolve().parents[1]


def run(*args):
    # Commands are local SDK tools; the APK package is checked before any device mutation.
    result = subprocess.run(args, text=True, capture_output=True, timeout=60, shell=False)  # nosec B603
    if result.returncode:
        raise RuntimeError(f"{' '.join(args)}: {result.stderr or result.stdout}")
    return result.stdout


def adb(*args):
    return run("adb", *args)


def action(name):
    adb("shell", "am", "start", "-W", "-n", COMPONENT, "--es", "qaAction", name)


def service():
    return adb("shell", "dumpsys", "activity", "services", PACKAGE)


def has_service():
    return "* ServiceRecord" in service()


def has_session():
    return f"package={PACKAGE}\n" in adb("shell", "dumpsys", "media_session")


def has_notification():
    return f"|{PACKAGE}|" in adb("shell", "cmd", "notification", "list")


def wait_for(condition, description):
    deadline = time.monotonic() + 6
    while time.monotonic() < deadline:
        if condition():
            return
        time.sleep(0.15)
    raise AssertionError(description)


def absent():
    return not has_service() and not has_session() and not has_notification()


def fresh_play():
    adb("shell", "am", "force-stop", PACKAGE)
    action("play")
    wait_for(lambda: "isForeground=true" in service(), "fixture must start foreground playback")


def stopped_state():
    fresh_play()
    action("stop")
    wait_for(absent, "stopped playback must release its session and service")


def background_and_pause():
    fresh_play()
    adb("shell", "input", "keyevent", "3")
    assert "isForeground=true" in service(), "Home must preserve background playback"
    action("pause")
    wait_for(lambda: "isForeground=true" not in service(), "paused media must leave the foreground")
    assert has_session(), "a paused track remains controllable while the app is open"
    action("dismiss")
    wait_for(absent, "dismissing a paused app must remove the media service")


def dismissed_playback(action_name):
    fresh_play()
    process_id = adb("shell", "pidof", PACKAGE).strip()
    action(action_name)
    wait_for(absent, f"{action_name} must release playing media")
    time.sleep(0.6)
    assert absent(), "a late event must not revive dismissed playback"
    logs = adb("logcat", "-d", f"--pid={process_id}", "-s", "KazeLifecycleQA:I", "*:S")
    assert "backend command: stop" in logs, "dismissal must stop the Rust player too"


def restart_and_external_stop():
    fresh_play()
    action("restart")
    wait_for(lambda: "isForeground=true" in service(), "immediate stop/play must recreate the service")
    action("externalStop")
    wait_for(absent, "an external controller's Stop must reach the backend and release media")


def unavailable_bridge():
    fresh_play()
    action("brokenSinkDismiss")
    wait_for(absent, "a closed command bridge must not prevent native session cleanup")


def idle_and_reopen():
    adb("shell", "am", "force-stop", PACKAGE)
    action("idleService")
    wait_for(absent, "an empty service started by a media button must not stay active")
    action("play")
    wait_for(lambda: "isForeground=true" in service(), "a subsequent explicit play must still work")
    action("dismiss")
    wait_for(absent, "reopened app must still clean up")


def validate_apk(parser, apk):
    if not apk.is_file():
        parser.error("Build the separate Android media QA fixture first (see docs/ANDROID_LIFECYCLE.md)")
    sdk = Path(os.environ.get("ANDROID_HOME", "/opt/android-sdk"))
    tools = sorted(sdk.glob("build-tools/*/aapt2"))
    if not tools:
        parser.error("Set ANDROID_HOME to an SDK with aapt2 installed")
    badging = run(str(tools[-1]), "dump", "badging", str(apk))
    package = re.search(r"package: name='([^']+)'", badging)
    if not package or package[1] != PACKAGE:
        parser.error("Refusing to install anything except the separate lifecycle QA package")
def run_cases(results):
    cases = [
            ("terminal playback removes empty media", stopped_state),
            ("Home preserves playback; paused dismissal cleans up", background_and_pause),
            ("playing task dismissal stops backend and media", lambda: dismissed_playback("dismiss")),
            ("finishing Activity stops backend and media", lambda: dismissed_playback("close")),
            ("late playback cannot revive a dismissed task", lambda: dismissed_playback("dismissWithLateEvent")),
            ("closed bridge cannot strand the native player", unavailable_bridge),
            ("immediate restart and external Stop", restart_and_external_stop),
            ("empty service and reopening", idle_and_reopen),
    ]
    for name, case in cases:
        try:
            case()
        except Exception as error:
            results.append({"case": name, "passed": False, "error": str(error), **diagnostics()})
            raise
        results.append({"case": name, "passed": True})
        print(f"PASS {name}", flush=True)


def diagnostics():
    try:
        return {"service": service(), "session_present": has_session(),
                "notification_present": has_notification()}
    except Exception as error:
        return {"diagnostics_unavailable": str(error)}


def cleanup_fixture():
    errors = []
    # Owns only the disposable QA package; disconnection must not hide the test result.
    for command in [("shell", "am", "force-stop", PACKAGE), ("uninstall", PACKAGE)]:
        try:
            adb(*command)
        except Exception as error:
            errors.append(str(error))
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apk", type=Path, default=ROOT / "tests/fixtures/androidMediaQa/app/build/outputs/apk/debug/app-debug.apk")
    parser.add_argument("--output", type=Path, default=ROOT / "tests/fixtures/androidMediaQa/build/lifecycle-results.json")
    args = parser.parse_args()
    validate_apk(parser, args.apk)
    results = []
    failure = None
    try:
        run("android", "install", f"--apks={args.apk}")
        run_cases(results)
    except Exception as error:
        failure = str(error)
        raise
    finally:
        cleanup_errors = cleanup_fixture()
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps({"package": PACKAGE, "results": results,
                                          "complete": failure is None and not cleanup_errors,
                                          "error": failure, "cleanup_errors": cleanup_errors}, indent=2) + "\n")
        if cleanup_errors and failure is None:
            raise RuntimeError("QA cleanup failed; see the result file")


if __name__ == "__main__":
    main()

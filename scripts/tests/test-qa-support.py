"""Exercise URL boundaries, redirects and cleanup errors without live providers."""

import contextlib
import http.server
import importlib.util
import io
from pathlib import Path
import threading
import unittest
import urllib.error
from unittest.mock import Mock

SPEC = importlib.util.spec_from_file_location(
    "qa_support", Path(__file__).resolve().parents[1] / "qa_support.py"
)
support = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(support)


class QaSupportTests(unittest.TestCase):
    def test_nonlocal_urls_and_credentials_fail_before_opening(self):
        for url in [
            "file:///etc/passwd",
            "ftp://127.0.0.1/test",
            "http://example.invalid/",
            "http://user@127.0.0.1/",
            "http://127.0.0.1:0/",
            "http://127.0.0.1:65536/",
        ]:
            with self.subTest(url=url), self.assertRaises(ValueError):
                support.open_loopback_url(url, timeout=1)

    def test_gaana_requires_exact_https_song_origin(self):
        for url in [
            "http://gaana.com/song/test",
            "https://gaana.com.evil.invalid/song/test",
            "https://user@gaana.com/song/test",
            "https://gaana.com:8443/song/test",
            "https://gaana.com/album/test",
        ]:
            with self.subTest(url=url), self.assertRaises(ValueError):
                support.validate_gaana_url(url)
        support.validate_gaana_url("https://www.gaana.com/song/test")

    def test_local_response_and_redirect_policy(self):
        class Handler(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                if self.path == "/allowed":
                    self.send_response(302)
                    self.send_header("Location", "/ok")
                elif self.path == "/blocked":
                    self.send_response(302)
                    self.send_header("Location", "http://example.invalid/song")
                else:
                    self.send_response(200)
                self.end_headers()
                self.wfile.write(b"fixture")

            def log_message(self, *_args):
                # Routine fixture access logs are intentionally quiet.
                return

        with http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler) as server:
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            origin = f"http://127.0.0.1:{server.server_port}"
            try:
                with support.open_loopback_url(
                    origin + "/allowed", timeout=2
                ) as response:
                    self.assertEqual(response.read(), b"fixture")
                with self.assertRaisesRegex(urllib.error.URLError, "Blocked redirect"):
                    support.open_loopback_url(origin + "/blocked", timeout=2)
            finally:
                server.shutdown()
                thread.join(timeout=3)

    def test_cleanup_reports_errors_closes_remaining_and_preserves_primary_failure(
        self,
    ):
        failed, remaining = Mock(), Mock()
        failed.close.side_effect = RuntimeError("injected close failure")
        output = io.StringIO()
        with contextlib.redirect_stderr(output):
            with self.assertRaisesRegex(ValueError, "primary failure"):
                try:
                    raise ValueError("primary failure")
                finally:
                    support.close_inspectors([failed, remaining])
        remaining.close.assert_called_once()
        self.assertIn(
            "Inspector cleanup failed: RuntimeError: injected close failure",
            output.getvalue(),
        )


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Check the linked GaanaAPI's metadata gate; this is not a playback provider.

Reference: cyberboysumanjay/GaanaAPI, revision
0488e812a9ac7d740d785c6e09f1e83e527eebcc, gaana.py.
The legacy JSON-block pattern and required field contract are adapted under MIT;
see docs/third-party/GaanaAPI-LICENSE. No proxy fallback or account access is used.
"""
import argparse
import json
import re
import sys
import unittest
import urllib.error
import urllib.parse
import urllib.request

REFERENCE = "0488e812a9ac7d740d785c6e09f1e83e527eebcc"
MAX_PAGE_BYTES = 2 * 1024 * 1024
LEGACY_BLOCK = re.compile(r"> ({[^<]*}) <")
HEADERS = {
    "User-Agent": "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:49.0) Gecko/20100101 Firefox/49.0"
}


def inspect_page(page):
    blocks = LEGACY_BLOCK.findall(page)
    tracks = 0
    for block in set(blocks):
        try:
            item = json.loads(block)
        except (ValueError, TypeError):
            continue
        if not isinstance(item, dict) or not isinstance(item.get("path"), dict):
            continue
        for quality in ("high", "medium", "normal", "auto"):
            entries = item["path"].get(quality)
            if isinstance(entries, list) and entries and isinstance(entries[0], dict):
                if entries[0].get("message") and entries[0].get("bitRate"):
                    required = ("title", "albumtitle", "albumartwork_large", "language", "share_url", "duration", "artist", "release_date")
                    if all(key in item for key in required):
                        tracks += 1
                        break
    return {
        "legacy_json_blocks": len(blocks),
        "reference_track_records": tracks,
        "has_current_preloaded_state": 'id="preloaded-state"' in page or "id='preloaded-state'" in page,
    }


def probe(url, timeout):
    parsed = urllib.parse.urlsplit(url)
    if parsed.scheme != "https" or parsed.hostname not in {"gaana.com", "www.gaana.com"} or parsed.username or parsed.password or not parsed.path.startswith("/song/"):
        raise ValueError("Use an HTTPS gaana.com/song/... URL")
    result = {"url": url, "reference_revision": REFERENCE, "full_playback_verified": False}
    try:
        request = urllib.request.Request(url, headers=HEADERS)
        with urllib.request.urlopen(request, timeout=timeout) as response:
            body = response.read(MAX_PAGE_BYTES + 1)
            result["http_status"] = response.status
            if len(body) > MAX_PAGE_BYTES:
                result.update(gate="failed", reason="page_exceeds_size_limit")
                return result
            result.update(inspect_page(body.decode("utf-8", errors="replace")))
            result["gate"] = "metadata_only" if result["reference_track_records"] else "failed"
            result["reason"] = "full_playback_still_requires_verification" if result["reference_track_records"] else "reference_parser_found_no_tracks"
    except urllib.error.HTTPError as error:
        result.update(gate="failed", reason="http_error", http_status=error.code)
    except (urllib.error.URLError, TimeoutError, OSError) as error:
        result.update(gate="failed", reason="network_error", error_type=type(error).__name__)
    return result


class ParserTests(unittest.TestCase):
    def test_recognizes_reference_shape_without_claiming_playback(self):
        track = {"title": "Song", "albumtitle": "Album", "albumartwork_large": "https://example.test/art.jpg", "language": "Hindi", "share_url": "/song/song", "duration": "180", "artist": "Artist#123", "release_date": "2020", "path": {"high": [{"message": "envelope", "bitRate": "128"}]}}
        self.assertEqual(inspect_page("<div> " + json.dumps(track) + " </div>")["reference_track_records"], 1)

    def test_changed_page_and_unrelated_json_do_not_pass(self):
        page = '<script id="preloaded-state">{"song":{"tracks":[]}}</script><div> {"not":"a song"} </div>'
        result = inspect_page(page)
        self.assertTrue(result["has_current_preloaded_state"])
        self.assertEqual(result["reference_track_records"], 0)

    def test_missing_or_malformed_media_fields_do_not_pass(self):
        for data in ('{"path":null}', '{"path":{"high":[]}}', '{"path":{"high":[null]}}', '{malformed}'):
            self.assertEqual(inspect_page("<div> " + data + " </div>")["reference_track_records"], 0)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("urls", nargs="*", default=["https://gaana.com/song/alone-1435", "https://gaana.com/song/kesariya-from-brahmastra"])
    parser.add_argument("--timeout", type=float, default=10)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return 0 if unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(ParserTests)).wasSuccessful() else 1
    if not 0 < args.timeout <= 30:
        parser.error("timeout must be greater than zero and at most 30 seconds")
    try:
        results = [probe(url, args.timeout) for url in args.urls]
    except ValueError as error:
        parser.error(str(error))
    print(json.dumps(results, indent=2))
    # Metadata alone never passes the full-track acceptance gate.
    return 2


if __name__ == "__main__":
    sys.exit(main())

"""Shared network boundaries and cleanup diagnostics for disposable QA harnesses."""

import sys
import urllib.error
import urllib.parse
import urllib.request


def validate_loopback_url(url):
    parsed = urllib.parse.urlsplit(url)
    if (
        parsed.scheme not in {"http", "https"}
        or parsed.hostname not in {"127.0.0.1", "::1", "localhost"}
        or parsed.username is not None
        or parsed.password is not None
        or parsed.fragment
    ):
        raise ValueError(
            "QA server URL must use HTTP(S) on loopback without credentials"
        )
    # Accessing port also rejects malformed or out-of-range port numbers.
    if parsed.port == 0:
        raise ValueError("QA server URL must use a nonzero port")


def validate_gaana_url(url):
    parsed = urllib.parse.urlsplit(url)
    if (
        parsed.scheme != "https"
        or parsed.hostname not in {"gaana.com", "www.gaana.com"}
        or parsed.username is not None
        or parsed.password is not None
        or parsed.port not in {None, 443}
        or not parsed.path.startswith("/song/")
    ):
        raise ValueError("Use an HTTPS gaana.com/song/... URL")


class _CheckedRedirect(urllib.request.HTTPRedirectHandler):
    def __init__(self, validate):
        super().__init__()
        self.validate = validate

    def redirect_request(self, request, fp, code, message, headers, new_url):
        try:
            self.validate(new_url)
        except ValueError as error:
            raise urllib.error.URLError(f"Blocked redirect: {error}") from error
        return super().redirect_request(request, fp, code, message, headers, new_url)


def open_loopback_url(url, *, timeout):
    validate_loopback_url(url)
    # Never send disposable local fixture requests through an environment proxy.
    opener = urllib.request.build_opener(
        urllib.request.ProxyHandler({}), _CheckedRedirect(validate_loopback_url)
    )
    return opener.open(url, timeout=timeout)


def open_gaana_url(request, *, timeout):
    validate_gaana_url(request.full_url)
    opener = urllib.request.build_opener(_CheckedRedirect(validate_gaana_url))
    return opener.open(request, timeout=timeout)


def close_inspectors(inspectors):
    """Attempt every close, retaining cleanup errors without masking a test failure."""
    for inspector in inspectors:
        try:
            inspector.close()
        except Exception as error:
            print(
                f"Inspector cleanup failed: {type(error).__name__}: {error}",
                file=sys.stderr,
            )

# Gaana provider compatibility gate

Gaana was evaluated as an additional source on `codex/jiosaavn-playback-experiment`. It is **not enabled in Kaze**: the supplied reference does not currently pass its metadata/media acceptance gate. JioSaavn remains the default.

**Scope:** this is a compatibility finding about the linked wrapper, not a finding that Gaana's service is universally unavailable. The original metadata probe did not execute the wrapper's Flask route. The independent recheck below closes that gap and states the remaining limits.

The user clarified that the audio-quality concern came from an older source's cached song. No JioSaavn audio-processing or cache changes were made on that basis.

## Reference and live findings

Reference: [cyberboysumanjay/GaanaAPI](https://github.com/cyberboysumanjay/GaanaAPI/tree/0488e812a9ac7d740d785c6e09f1e83e527eebcc), revision `0488e812a9ac7d740d785c6e09f1e83e527eebcc`. Its `gaana.py` extracts specially formatted JSON blocks from song pages, expects `path.high/medium/normal/auto` entries, and decodes a media URL envelope. The README explicitly requires song links and does not implement search. The code's MIT attribution is preserved in [GaanaAPI-LICENSE](third-party/GaanaAPI-LICENSE).

Direct requests with the reference User-Agent returned HTTP 200 for its documented `https://gaana.com/song/alone-1435` example, a Kesariya song URL and the homepage. The current pages contain a `preloaded-state` application snapshot instead of the reference's JSON-block/`path` shape. The original extraction pattern returned zero track records.

The current Alone metadata identifies track `21232014`, duration 159 seconds, and exposes a different `urls` structure. Applying the reference's original URL decoder to those fields failed: the medium value was not valid padded Base64, while high/auto failed AES padding validation. No full-track media URL, successful Rust decoding or native seeking was obtained. These findings establish incompatibility with this reference revision, rather than proving Gaana cannot be integrated through any other interface or region.

The reference's example output is an HLS `.m3u8` playlist. Kaze's current direct-stream cache cannot treat a playlist document as a complete audio file. A working Gaana integration therefore needs both a verified current media interface and an appropriate Rust HLS preparation path if that interface still returns HLS. It must preserve exact provider identity, bounded caching, cancellation, full-track decoding and seeking. Preview playback does not pass the gate.

No third-party proxy fallback, account credentials, browser audio engine, silent alternative-source substitution or broken Gaana menu option was added.

## Independent recheck — 2026-10-27

The remote HEAD/master still resolves to the pinned revision. Installed the missing Flask and PyCryptodome dependencies in a disposable environment and imported the actual `gaana.py` and `app.py`. Both files' Git blob IDs match the repository revision; source was not rewritten for the test. Called `downloadAndParsePage(url, False)` and the actual `/result/?url=...` route via Flask's test client:

| Live page | HTTP page response | Original function | Actual Flask route |
| --- | --- | --- | --- |
| README example: `/song/alone-1435` | 200 | 0 tracks | 500, HTML error response |
| `/song/kesariya-from-brahmastra` | 200 | 0 tracks | 500, HTML error response |

After the function returns no songs, the route raises `ArithmeticError`. Its exception handler passes that exception object to `jsonify`, which fails with `TypeError: Object of type ArithmeticError is not JSON serializable` under Flask 3.1.3. The requirements file does not pin an older Flask version. This confirms an extraction failure plus an error-handling failure in the tested environment; the 500 alone is not evidence about the media service.

Requests had a ten-second timeout. The code's third-party proxy-list fallback was explicitly excluded in memory, rather than contacting arbitrary proxies; direct origin retrieval was tested. Other proxy/region combinations are unverified. [Exact-wrapper evidence](evidence/gaana-wrapper-recheck-20261027.json) records the dependency versions, unchanged source identity and per-request observations.

A separate REA-controlled Chromium session with a fresh profile attempted the public song page and its normal Play action. The page request received HTTP 403, so the Play button never became available and the action was **not executed**. The owned browser was terminated cleanly. [Complete browser evidence](evidence/gaana-browser-recheck-20261027.json), Evidence `ev_f357797394b1bcda6fe27c8986aaa6dd9e21deb717aa86e3b3e38b13785bdc7d`, reports incomplete action coverage. It does not prove a user-operated browser, logged-in session, other network or a current replacement API cannot play Gaana.

Conclusion: the linked revision does not supply usable playback through the tested direct path unchanged. A working current Gaana integration remains a separate, unverified possibility and would require a current interface and the full native playback gate.

## Reproduce the compatibility check

```bash
python3 scripts/probe-gaana.py --self-test
python3 scripts/probe-gaana.py
```

The second command checks the two public song pages, prints bounded metadata-only diagnostics and exits 2 because full playback is unverified. Optional positional arguments select other HTTPS Gaana song links. It uses the standard library, finite timeouts and a page-size limit; this is a developer probe, not an application runtime dependency.

Before exposing Gaana as a source, repeat the full acceptance gate with multiple tracks/languages: current exact-song metadata, valid artwork, ordinary accessible full media, complete decoding through Kaze's production Rust decoder, and native pause/resume/seek. Then wire provider identity and cache keys into the existing unified player and add the source controls.

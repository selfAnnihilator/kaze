# Cover art: resized images over a `kaze-art:` protocol

Local covers crossed IPC as full-resolution base64 data URLs. A 1400px cover is about
2 MB of base64 text, and the webview decoded it to an 8 MB bitmap even for a 48px row
thumbnail. The backend kept every data URL it had ever served in an unbounded map,
keyed per track, so an album's tracks each held their own copy.

Decision:

| Area | Rule |
| --- | --- |
| IPC | `GetTrackCoverArt` still returns one optional string. For local art it is now `kaze-art://localhost/<hex track id>?v=<version>` (on Windows `http://kaze-art.localhost/…`). Online art is still its https URL. |
| Sizes | The frontend appends `s=<px>`: the frame's CSS width times the device pixel ratio, rounded up to 128, 256, 512 or 1024 (`coverSize` in `services/coverArt.ts`). The playing track uses 1024, because one image serves the bar, the stage and full screen. Images are never upscaled. |
| Serving | `register_asynchronous_uri_scheme_protocol` resolves the track's source (picture tag, folder image or enrichment download), resizes it to a JPEG (quality 90, Catmull-Rom), and answers with `Cache-Control: immutable`. At most two images are resized at once. |
| Cache | Resized files live in `<cache dir>/artwork/`, named by a hash of the original image and the size, so an album's tracks share one file. The cache is trimmed to 64 MB, oldest files first, when it passes 96 MB (checked at startup). |
| Invalidation | `LibraryMetadataUpdated` and scan completion bump a version number that is part of every URL, so the webview never shows a stale cover. |

Consequences: the backend keeps only short URLs and source paths per track. The webview
decodes images close to their display size. A first view of a cover costs one resize
(tens of milliseconds); later sessions read the disk cache.

Alternatives: resizing to base64 thumbnails (still copies text through IPC and JS
memory), or serving originals through Tauri's asset protocol (no resizing, and it
exposes file paths).

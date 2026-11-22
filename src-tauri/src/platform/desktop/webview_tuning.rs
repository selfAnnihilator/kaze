//! WebKitGTK settings for an app webview, not a browser.
//!
//! Kaze plays all audio in Rust (AGENTS.md: no HTML5 audio) and draws no
//! WebGL/canvas, so the engines behind those features never need to start.
//! A single-page app gains nothing from the back/forward page cache or from
//! browser-sized resource caches. None of this changes what is drawn.

use webkit2gtk::{CacheModel, SettingsExt, WebContextExt, WebViewExt};

pub fn apply<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) {
    let result = window.with_webview(|webview| {
        let view = webview.inner();
        if let Some(settings) = WebViewExt::settings(&view) {
            settings.set_enable_webgl(false);
            settings.set_enable_webaudio(false);
            settings.set_enable_media(false);
            settings.set_enable_mediasource(false);
            settings.set_enable_media_stream(false);
            settings.set_enable_webrtc(false);
            settings.set_enable_encrypted_media(false);
            settings.set_enable_page_cache(false);
            settings.set_enable_dns_prefetching(false);
            settings.set_enable_hyperlink_auditing(false);
            settings.set_enable_offline_web_application_cache(false);
            settings.set_enable_html5_database(false);
        }
        // A middle ground: the web-browser model keeps large decoded-image
        // and page caches; document-viewer disables the resource cache, which
        // would re-fetch artwork when virtual lists remount rows.
        if let Some(context) = view.context() {
            context.set_cache_model(CacheModel::DocumentBrowser);
        }
    });
    if let Err(error) = result {
        tracing::warn!(%error, "Could not tune the webview");
    }
}

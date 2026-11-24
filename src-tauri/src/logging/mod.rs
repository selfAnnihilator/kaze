use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[cfg(target_os = "android")]
mod logcat;

/// Release builds log at `info`; `RUST_LOG` still enables more detail.
#[cfg(debug_assertions)]
const DEFAULT_FILTER: &str = "music_player_backend=debug,info";
#[cfg(all(not(debug_assertions), not(target_os = "android")))]
const DEFAULT_FILTER: &str = "info";
/// The phone logs problems and the sync summary, not every query.
#[cfg(all(not(debug_assertions), target_os = "android"))]
const DEFAULT_FILTER: &str = "warn,music_player_backend::cloud=info,music_player_backend::core::processor::account=info";

/// Initializes structured logging with environment filter and formatting.
pub fn init_logging() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));

    let fmt_layer = tracing_subscriber::fmt::layer();
    #[cfg(target_os = "android")]
    let fmt_layer = fmt_layer.with_writer(|| logcat::Logcat).with_ansi(false).without_time();
    let fmt_layer = fmt_layer
        .with_target(true)
        .with_thread_ids(true)
        .with_level(true);

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .try_init();
}

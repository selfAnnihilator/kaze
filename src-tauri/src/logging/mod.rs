use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Release builds log at `info`; `RUST_LOG` still enables more detail.
#[cfg(debug_assertions)]
const DEFAULT_FILTER: &str = "music_player_backend=debug,info";
#[cfg(not(debug_assertions))]
const DEFAULT_FILTER: &str = "info";

/// Initializes structured logging with environment filter and formatting.
pub fn init_logging() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_level(true);

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .try_init();
}

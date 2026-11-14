//! One HTTP client for one-off requests (lyrics, artwork downloads).
//!
//! Building a reqwest client sets up a TLS configuration and a connection
//! pool; doing that per request wasted CPU and memory and threw away
//! keep-alive connections. Callers set their own timeout per request.

use std::sync::OnceLock;

pub fn shared_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .pool_idle_timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default()
    })
}

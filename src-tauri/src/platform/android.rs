//! Android shell. Stub: wires the shared IPC and artwork handler; the
//! processor setup, kaze-media plugin and event bridge follow in later steps.

use super::shared;
use crate::library::artwork;

pub fn run() {
    shared::init_async_runtime();
    tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol(artwork::SCHEME, shared::serve_artwork)
        .invoke_handler(tauri::generate_handler![
            shared::execute_command,
            shared::execute_query
        ])
        .run(tauri::generate_context!())
        .expect("error while running Kaze");
}

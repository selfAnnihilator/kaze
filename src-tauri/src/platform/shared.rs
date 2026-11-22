//! Shell pieces shared by every platform: IPC commands, the artwork scheme
//! handler and the async runtime setup.

use crate::core::command::{Command, CommandResponse};
use crate::core::processor::CoreProcessor;
use crate::core::query::{Query, QueryResponse};
use crate::library::artwork;
use std::sync::Arc;
use tauri::Manager;

#[tauri::command]
pub async fn execute_command(
    processor: tauri::State<'_, Arc<CoreProcessor>>,
    command: serde_json::Value,
) -> Result<CommandResponse, String> {
    let mut val = command;
    if let Some(obj) = val.as_object_mut() {
        if (!obj.contains_key("payload") || obj.get("payload") == Some(&serde_json::Value::Null))
            && serde_json::from_value::<Command>(serde_json::Value::Object(obj.clone())).is_err() {
                obj.insert("payload".to_string(), serde_json::json!({}));
            }
    }
    let parsed_command: Command =
        serde_json::from_value(val).map_err(|e| format!("Command parse error: {}", e))?;
    processor
        .dispatch_command(parsed_command)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn execute_query(
    processor: tauri::State<'_, Arc<CoreProcessor>>,
    query: serde_json::Value,
) -> Result<QueryResponse, String> {
    let mut val = query;
    if let Some(obj) = val.as_object_mut() {
        if obj.get("query").and_then(|q| q.as_str()) == Some("GetStatsOverview")
            && (!obj.contains_key("payload") || obj.get("payload") == Some(&serde_json::Value::Null))
            {
                obj.insert("payload".to_string(), serde_json::json!({}));
            }
    }
    let parsed_query: Query =
        serde_json::from_value(val).map_err(|e| format!("Query parse error: {}", e))?;
    processor
        .execute_query(parsed_query)
        .await
        .map_err(|e| e.to_string())
}

/// Installs the shared Tokio runtime as Tauri's async runtime.
pub fn init_async_runtime() {
    // A music player's async work is I/O-bound; a worker per core (Tokio's
    // default) only adds idle threads. Heavy work uses spawn_blocking.
    match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("kaze-async")
        .enable_all()
        .build()
    {
        Ok(runtime) => tauri::async_runtime::set(Box::leak(Box::new(runtime)).handle().clone()),
        Err(error) => tracing::warn!(%error, "Using the default async runtime"),
    }
}

/// Serves `kaze-art:` cover URLs: resized local artwork from the disk cache.
pub fn serve_artwork<R: tauri::Runtime>(
    ctx: tauri::UriSchemeContext<'_, R>,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    let app = ctx.app_handle().clone();
    tauri::async_runtime::spawn(async move {
        let uri = request.uri();
        let image = match (
            artwork::parse_request(uri.path(), uri.query()),
            app.try_state::<Arc<CoreProcessor>>(),
        ) {
            (Some((track_id, size)), Some(processor)) => {
                processor.track_artwork_image(&track_id, size).await
            }
            _ => None,
        };
        let response = match image {
            // The URL carries a version that changes with the library, so
            // the webview may keep the image for the whole session.
            Some((bytes, mime)) => tauri::http::Response::builder()
                .header("Content-Type", mime)
                .header("Cache-Control", "max-age=31536000, immutable")
                .body(bytes),
            None => tauri::http::Response::builder().status(404).body(Vec::new()),
        };
        responder.respond(response.unwrap_or_default());
    });
}

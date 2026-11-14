//! Processor handlers for app settings.

use super::routing::misrouted;
use super::*;

impl CoreProcessor {
    pub(super) async fn settings_command(&self, cmd: Command) -> AppResult<CommandResponse> {
        match cmd {
            Command::SetCloseButtonBehavior { behavior } => {
                self.set_close_button_behavior(behavior).await?;
                Ok(CommandResponse::Ok)
            }
            Command::SetInterfaceStyle { style } => {
                if !matches!(style.as_str(), "spatial" | "brutalist") {
                    return Err(AppError::Validation("Unknown interface style".to_string()));
                }
                self.settings_repo
                    .set_setting("interface_style", &style)
                    .await?;
                Ok(CommandResponse::Ok)
            }
            other => Err(misrouted(&other)),
        }
    }

    pub(super) async fn settings_query(&self, query: Query) -> AppResult<QueryResponse> {
        match query {
            Query::GetSettings => {
                self.initialize_close_behavior().await?;
                let mut cfg = self.config.read().await.clone();
                cfg.close_button_behavior = self.close_button_behavior().to_string();
                let mut val =
                    serde_json::to_value(&cfg).map_err(|e| AppError::Internal(e.to_string()))?;
                let style = self.settings_repo.get_setting("interface_style").await?;
                val["interface_style"] = serde_json::json!(match style.as_deref() {
                    Some("brutalist") => "brutalist",
                    _ => "spatial",
                });
                for (key, default) in [
                    ("enable_jiosaavn", true),
                    ("auto_fill_metadata", true),
                    ("enable_lastfm_similarity", true),
                ] {
                    val["metadata"][key] =
                        serde_json::json!(self.gap_enrichment.enabled(key, default).await);
                }
                // Canonical, so it matches the stored track paths exactly.
                let dir = self.download_service.download_dir();
                val["downloads"]["resolved_dir"] = serde_json::json!(dir
                    .canonicalize()
                    .unwrap_or_else(|_| dir.to_path_buf())
                    .to_string_lossy());
                Ok(QueryResponse::Settings(val))
            }
            other => Err(misrouted(&other)),
        }
    }
}

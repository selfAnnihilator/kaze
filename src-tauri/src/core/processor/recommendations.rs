//! Processor handlers for Flow, taste setup, recommendations and their feedback.

use super::routing::misrouted;
use super::*;

impl CoreProcessor {
    pub(super) async fn recommendations_command(&self, cmd: Command) -> AppResult<CommandResponse> {
        match cmd {
            Command::SaveRecommendationSetup {
                expected_user_id,
                artists,
                genres,
                skipped,
            } => {
                let user = self.current_user_id().await;
                if expected_user_id
                    .as_deref()
                    .is_some_and(|expected| expected != user)
                {
                    return Err(AppError::Validation(
                        "Account changed. Reopen music taste and try again.".into(),
                    ));
                }
                crate::recommendations::setup::save(&self.db_pool, &user, artists, genres, skipped)
                    .await?;
                self.flow_engine.invalidate_home_picks(&user).await;
                sqlx::query("UPDATE playlists SET expires_at=0 WHERE user_id=? AND is_smart_mix=1")
                    .bind(&user)
                    .execute(&self.db_pool)
                    .await?;
                let _ = self
                    .event_bus
                    .publish(Event::TasteProfileChanged { user_id: user });
                Ok(CommandResponse::Ok)
            }
            Command::StartFlow { familiarity_ratio } => {
                let user_id = self.current_user_id().await;
                self.playback_service
                    .start_flow(&self.flow_engine, &user_id, familiarity_ratio)
                    .await?;
                Ok(CommandResponse::Ok)
            }
            Command::ContinueFlow => {
                let user_id = self.current_user_id().await;
                self.playback_service.continue_flow(&user_id).await?;
                Ok(CommandResponse::Ok)
            }
            Command::StopFlow => {
                self.playback_service.stop_flow().await;
                Ok(CommandResponse::Ok)
            }
            Command::SetFlowSteering { familiarity_ratio } => {
                let user_id = self.current_user_id().await;
                self.playback_service
                    .set_flow_steering(&self.flow_engine, &user_id, familiarity_ratio)
                    .await?;
                Ok(CommandResponse::Ok)
            }
            Command::BlockArtist { track_id } => Ok(CommandResponse::EntityFeedback(
                self.set_entity_feedback(&track_id, "block").await?,
            )),
            Command::LessLikeThis { track_id } => Ok(CommandResponse::EntityFeedback(
                self.set_entity_feedback(&track_id, "less").await?,
            )),
            Command::ClearEntityFeedback { kind, key } => {
                self.clear_entity_feedback(&kind, &key).await?;
                Ok(CommandResponse::Ok)
            }
            Command::GenerateSmartMix { mix_type } => {
                let user_id = self.current_user_id().await;
                let playlist = self
                    .smart_mix_generator
                    .generate_mix(&user_id, &mix_type)
                    .await?;
                let tracks = self.playlist_repo.get_playlist_tracks(&playlist.id).await?;
                let track_count = tracks.len();
                let mix_type_name = playlist
                    .mix_type
                    .clone()
                    .unwrap_or_else(|| "custom".to_string());

                let _ = self.event_bus.publish(Event::SmartMixGenerated {
                    playlist_id: playlist.id.clone(),
                    mix_type: mix_type_name,
                    track_count,
                });

                Ok(CommandResponse::MixGenerated {
                    playlist_id: playlist.id,
                    track_count,
                })
            }
            other => Err(misrouted(&other)),
        }
    }

    pub(super) async fn recommendations_query(&self, query: Query) -> AppResult<QueryResponse> {
        match query {
            Query::GetRecommendationSetup => {
                let user = self.current_user_id().await;
                let mut setup = crate::recommendations::setup::load(&self.db_pool, &user).await?;
                setup.should_prompt = setup.status == "pending"
                    && !self
                        .taste_engine
                        .compute_taste_profile(&user)
                        .await?
                        .has_learned_taste;
                Ok(QueryResponse::RecommendationSetup(setup))
            }
            Query::GetTasteProfile => {
                let user_id = self.current_user_id().await;
                let profile = self.taste_engine.compute_taste_profile(&user_id).await?;
                let val = serde_json::to_value(&profile)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                Ok(QueryResponse::TasteProfile(val))
            }
            Query::GetLocalRecommendations { limit } => {
                let user_id = self.current_user_id().await;
                let recs = self.recommender.recommend(&user_id, limit as usize).await?;
                let val: Vec<serde_json::Value> = recs
                    .into_iter()
                    .filter_map(|r| serde_json::to_value(r).ok())
                    .collect();
                Ok(QueryResponse::Recommendations(
                    self.present_recommendations(serde_json::Value::Array(val), &user_id)
                        .await?
                        .as_array()
                        .cloned()
                        .unwrap_or_default(),
                ))
            }
            Query::GetDiscoveryRecommendations {
                limit,
                force_refresh,
            } => {
                let user_id = self.current_user_id().await;
                let recs = self
                    .discovery_coordinator
                    .get_discovery_recommendations(
                        &user_id,
                        limit as usize,
                        force_refresh.unwrap_or(false),
                    )
                    .await?;
                self.flow_engine.invalidate_home_picks(&user_id).await;
                let val: Vec<serde_json::Value> = recs
                    .into_iter()
                    .filter_map(|r| serde_json::to_value(r).ok())
                    .collect();
                Ok(QueryResponse::DiscoveryRecommendations(
                    self.present_recommendations(
                        serde_json::Value::Array(val),
                        &self.current_user_id().await,
                    )
                    .await?
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
                ))
            }
            Query::GetEntityFeedback => Ok(QueryResponse::EntityFeedback(
                crate::recommendations::entity_feedback::load_rows(
                    &self.db_pool,
                    &self.current_user_id().await,
                )
                .await?,
            )),
            Query::GetRecommendationFeedback => {
                let signals = crate::recommendations::signals::SongSignals::load(
                    &self.db_pool,
                    &self.current_user_id().await,
                )
                .await?;
                let ids = signals
                    .identities
                    .keys()
                    .filter(|id| signals.blocked_id(id))
                    .cloned()
                    .collect();
                Ok(QueryResponse::RecommendationFeedback(ids))
            }
            Query::GetHomeOverview => {
                let user_id = self.current_user_id().await;
                self.discovery_coordinator
                    .ensure_starter_candidates(&user_id, false)
                    .await?;
                let mut overview = self.flow_engine.get_home_overview(&user_id).await?;
                overview.flow_session = self
                    .playback_service
                    .flow_session(&self.flow_engine, &user_id)
                    .await?;
                let val = serde_json::to_value(overview).unwrap_or(serde_json::Value::Null);
                Ok(QueryResponse::HomeOverview(
                    self.present_recommendations(val, &user_id).await?,
                ))
            }
            Query::GetFlowQueue {
                familiarity_ratio,
                limit,
                exclude_track_ids,
            } => {
                let user_id = self.current_user_id().await;
                let ratio = familiarity_ratio.unwrap_or(0.5);
                let lim = limit.unwrap_or(20) as usize;
                let exclude = exclude_track_ids.unwrap_or_default();
                let tracks = self
                    .flow_engine
                    .generate_flow_tracks(&user_id, ratio, lim, &exclude)
                    .await?;
                let val: Vec<serde_json::Value> = tracks
                    .into_iter()
                    .filter_map(|t| serde_json::to_value(t).ok())
                    .collect();
                Ok(QueryResponse::FlowQueue(
                    self.present_recommendations(
                        serde_json::Value::Array(val),
                        &self.current_user_id().await,
                    )
                    .await?
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
                ))
            }
            Query::GetFlowTrackReason { track_id } => {
                let user_id = self.current_user_id().await;
                let reason = match self.playback_service.flow_reason(&user_id, &track_id).await {
                    Some(reason) => Some(reason),
                    None => Some(
                        self.flow_engine
                            .get_track_explanation(&user_id, &track_id)
                            .await?,
                    ),
                };
                Ok(QueryResponse::FlowTrackReason(reason))
            }
            Query::GetFlowSession => {
                let user_id = self.current_user_id().await;
                let session = self
                    .playback_service
                    .flow_session(&self.flow_engine, &user_id)
                    .await?;
                let val = session.and_then(|s| serde_json::to_value(s).ok());
                Ok(QueryResponse::FlowSession(val))
            }
            other => Err(misrouted(&other)),
        }
    }
}

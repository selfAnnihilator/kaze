use super::models::*;
use crate::core::error::{AppError, AppResult};
use reqwest::Client;
use std::time::Duration;

#[derive(Clone)]
pub struct CloudClient {
    client: Client,
}

impl CloudClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    pub async fn register(
        &self,
        worker_url: &str,
        username: &str,
        password: &str,
    ) -> AppResult<AuthResponse> {
        let url = format!("{}/api/auth/register", worker_url.trim_end_matches('/'));
        let res = self
            .client
            .post(&url)
            .json(&AuthRequest {
                username: username.to_string(),
                password: password.to_string(),
            })
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to connect to cloud auth: {}", e)))?;

        let auth_res: AuthResponse = res
            .json()
            .await
            .map_err(|e| AppError::Network(format!("Invalid response from cloud auth: {}", e)))?;

        if !auth_res.success {
            return Err(AppError::Validation(
                auth_res
                    .error
                    .unwrap_or_else(|| "Registration failed".to_string()),
            ));
        }

        Ok(auth_res)
    }

    pub async fn login(
        &self,
        worker_url: &str,
        username: &str,
        password: &str,
    ) -> AppResult<AuthResponse> {
        let url = format!("{}/api/auth/login", worker_url.trim_end_matches('/'));
        let res = self
            .client
            .post(&url)
            .json(&AuthRequest {
                username: username.to_string(),
                password: password.to_string(),
            })
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to connect to cloud auth: {}", e)))?;

        let auth_res: AuthResponse = res
            .json()
            .await
            .map_err(|e| AppError::Network(format!("Invalid response from cloud auth: {}", e)))?;

        if !auth_res.success {
            return Err(AppError::Validation(
                auth_res
                    .error
                    .unwrap_or_else(|| "Login failed".to_string()),
            ));
        }

        Ok(auth_res)
    }

    pub async fn logout(&self, worker_url: &str, token: &str) -> AppResult<()> {
        let url = format!("{}/api/auth/logout", worker_url.trim_end_matches('/'));
        let _ = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await;
        Ok(())
    }

    pub async fn get_me(&self, worker_url: &str, token: &str) -> AppResult<CloudUser> {
        let url = format!("{}/api/auth/me", worker_url.trim_end_matches('/'));
        let res = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to connect to cloud auth: {}", e)))?;

        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| AppError::Network(format!("Invalid JSON from cloud auth: {}", e)))?;

        if json["success"].as_bool().unwrap_or(false) {
            let user: CloudUser = serde_json::from_value(json["user"].clone())
                .map_err(|e| AppError::Network(format!("Failed to parse user profile: {}", e)))?;
            Ok(user)
        } else {
            Err(AppError::Validation(
                json["error"]
                    .as_str()
                    .unwrap_or("Session invalid")
                    .to_string(),
            ))
        }
    }

    pub async fn pull_sync(&self, worker_url: &str, token: &str) -> AppResult<SyncPayload> {
        let url = format!("{}/api/sync", worker_url.trim_end_matches('/'));
        let res = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to pull cloud sync: {}", e)))?;

        let pull_res: SyncPullResponse = res
            .json()
            .await
            .map_err(|e| AppError::Network(format!("Invalid response from cloud sync: {}", e)))?;

        if !pull_res.success {
            return Err(AppError::Validation(
                pull_res
                    .error
                    .unwrap_or_else(|| "Sync pull failed".to_string()),
            ));
        }

        Ok(pull_res.data.unwrap_or_default())
    }

    pub async fn push_sync(
        &self,
        worker_url: &str,
        token: &str,
        payload: &SyncPayload,
    ) -> AppResult<i64> {
        let url = format!("{}/api/sync", worker_url.trim_end_matches('/'));
        let res = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(payload)
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to push cloud sync: {}", e)))?;

        let push_res: SyncPushResponse = res
            .json()
            .await
            .map_err(|e| AppError::Network(format!("Invalid response from cloud sync: {}", e)))?;

        if !push_res.success {
            return Err(AppError::Validation(
                push_res
                    .error
                    .unwrap_or_else(|| "Sync push failed".to_string()),
            ));
        }

        Ok(push_res
            .synced_at
            .unwrap_or_else(|| chrono::Utc::now().timestamp()))
    }
}

impl Default for CloudClient {
    fn default() -> Self {
        Self::new()
    }
}

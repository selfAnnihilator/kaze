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
        device_id: Option<&str>,
        device_name: Option<&str>,
        client_version: Option<&str>,
    ) -> AppResult<AuthResponse> {
        let url = format!("{}/api/auth/register", worker_url.trim_end_matches('/'));
        let res = self
            .client
            .post(&url)
            .json(&AuthRequest {
                username: username.to_string(),
                password: password.to_string(),
                device_id: device_id.map(|s| s.to_string()),
                device_name: device_name.map(|s| s.to_string()),
                client_version: client_version.map(|s| s.to_string()),
            })
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to connect to cloud auth: {}", e)))?;

        let status = res.status();
        let auth_res: AuthResponse = res
            .json()
            .await
            .map_err(|e| AppError::Network(format!("Invalid response from cloud auth: {}", e)))?;

        if !auth_res.success {
            let msg = auth_res
                .error
                .unwrap_or_else(|| "Registration failed".to_string());
            if status.is_client_error() {
                return Err(AppError::Validation(msg));
            } else {
                return Err(AppError::Network(msg));
            }
        }

        Ok(auth_res)
    }

    pub async fn login(
        &self,
        worker_url: &str,
        username: &str,
        password: &str,
        device_id: Option<&str>,
        device_name: Option<&str>,
        client_version: Option<&str>,
    ) -> AppResult<AuthResponse> {
        let url = format!("{}/api/auth/login", worker_url.trim_end_matches('/'));
        let res = self
            .client
            .post(&url)
            .json(&AuthRequest {
                username: username.to_string(),
                password: password.to_string(),
                device_id: device_id.map(|s| s.to_string()),
                device_name: device_name.map(|s| s.to_string()),
                client_version: client_version.map(|s| s.to_string()),
            })
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to connect to cloud auth: {}", e)))?;

        let status = res.status();
        let auth_res: AuthResponse = res
            .json()
            .await
            .map_err(|e| AppError::Network(format!("Invalid response from cloud auth: {}", e)))?;

        if !auth_res.success {
            let msg = auth_res
                .error
                .unwrap_or_else(|| "Login failed".to_string());
            if status.is_client_error() {
                return Err(AppError::Validation(msg));
            } else {
                return Err(AppError::Network(msg));
            }
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

    pub async fn logout_all(&self, worker_url: &str, token: &str) -> AppResult<()> {
        let url = format!("{}/api/auth/logout-all", worker_url.trim_end_matches('/'));
        let res = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to connect to logout-all: {}", e)))?;

        let status = res.status();
        if !status.is_success() {
            let json: serde_json::Value = res.json().await.unwrap_or_default();
            let err = json["error"].as_str().unwrap_or("Logout all failed").to_string();
            return Err(AppError::Validation(err));
        }

        Ok(())
    }

    pub async fn list_sessions(&self, worker_url: &str, token: &str) -> AppResult<Vec<SessionInfo>> {
        let url = format!("{}/api/auth/sessions", worker_url.trim_end_matches('/'));
        let res = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to list sessions: {}", e)))?;

        let status = res.status();
        let list_res: SessionListResponse = res
            .json()
            .await
            .map_err(|e| AppError::Network(format!("Invalid response from sessions list: {}", e)))?;

        if !list_res.success {
            let err = list_res.error.unwrap_or_else(|| "Failed to fetch sessions".to_string());
            if status.is_client_error() {
                return Err(AppError::Validation(err));
            } else {
                return Err(AppError::Network(err));
            }
        }

        Ok(list_res.sessions)
    }

    pub async fn revoke_session(&self, worker_url: &str, token: &str, session_id: &str) -> AppResult<()> {
        let url = format!("{}/api/auth/sessions/{}", worker_url.trim_end_matches('/'), session_id);
        let res = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to revoke session: {}", e)))?;

        let status = res.status();
        if !status.is_success() {
            let json: serde_json::Value = res.json().await.unwrap_or_default();
            let err = json["error"].as_str().unwrap_or("Failed to revoke session").to_string();
            if status == reqwest::StatusCode::FORBIDDEN {
                return Err(AppError::Validation(format!("Forbidden: {}", err)));
            }
            return Err(AppError::Validation(err));
        }

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

        let status = res.status();
        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| AppError::Network(format!("Invalid JSON from cloud auth: {}", e)))?;

        if json["success"].as_bool().unwrap_or(false) {
            let user: CloudUser = serde_json::from_value(json["user"].clone())
                .map_err(|e| AppError::Network(format!("Failed to parse user profile: {}", e)))?;
            Ok(user)
        } else {
            let err_msg = json["error"]
                .as_str()
                .unwrap_or("Session invalid")
                .to_string();
            if status.is_client_error() {
                Err(AppError::Validation(err_msg))
            } else {
                Err(AppError::Network(err_msg))
            }
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

    pub async fn get_profile(&self, worker_url: &str, token: &str) -> AppResult<CloudUser> {
        let url = format!("{}/api/profile", worker_url.trim_end_matches('/'));
        let res = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to fetch profile: {}", e)))?;

        let status = res.status();
        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|e| AppError::Network(format!("Invalid profile response: {}", e)))?;

        if !status.is_success() || body["success"] != true {
            let msg = body["error"].as_str().unwrap_or("Failed to fetch profile");
            return Err(AppError::Network(msg.to_string()));
        }

        let user: CloudUser = serde_json::from_value(body["user"].clone())
            .map_err(|e| AppError::Validation(format!("Invalid profile data: {}", e)))?;

        Ok(user)
    }

    pub async fn upload_avatar(
        &self,
        worker_url: &str,
        token: &str,
        webp_bytes: Vec<u8>,
    ) -> AppResult<CloudAvatarResponse> {
        let url = format!("{}/api/profile/avatar", worker_url.trim_end_matches('/'));
        let res = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "image/webp")
            .body(webp_bytes)
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to connect to avatar upload: {}", e)))?;

        let status = res.status();
        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|e| AppError::Network(format!("Invalid response from avatar upload: {}", e)))?;

        if !status.is_success() || body["success"] != true {
            let msg = body["error"].as_str().unwrap_or("Avatar upload failed");
            return Err(AppError::Network(msg.to_string()));
        }

        let avatar_resp: CloudAvatarResponse = serde_json::from_value(body)
            .map_err(|e| AppError::Validation(format!("Invalid avatar response format: {}", e)))?;

        Ok(avatar_resp)
    }

    pub async fn delete_avatar(&self, worker_url: &str, token: &str) -> AppResult<()> {
        let url = format!("{}/api/profile/avatar", worker_url.trim_end_matches('/'));
        let res = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to connect to delete avatar: {}", e)))?;

        let status = res.status();
        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|e| AppError::Network(format!("Invalid response from delete avatar: {}", e)))?;

        if !status.is_success() || body["success"] != true {
            let msg = body["error"].as_str().unwrap_or("Failed to delete avatar");
            return Err(AppError::Network(msg.to_string()));
        }

        Ok(())
    }

    pub async fn download_avatar(
        &self,
        worker_url: &str,
        token: Option<&str>,
        user_id: &str,
    ) -> AppResult<Option<Vec<u8>>> {
        let url = format!(
            "{}/api/profile/avatar?user_id={}",
            worker_url.trim_end_matches('/'),
            user_id
        );
        let mut req = self.client.get(&url);
        if let Some(tok) = token {
            req = req.header("Authorization", format!("Bearer {}", tok));
        }

        let res = req
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to download avatar: {}", e)))?;

        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !res.status().is_success() {
            return Ok(None);
        }

        let bytes = res
            .bytes()
            .await
            .map_err(|e| AppError::Network(format!("Failed to read avatar bytes: {}", e)))?;

        Ok(Some(bytes.to_vec()))
    }
}

impl Default for CloudClient {
    fn default() -> Self {
        Self::new()
    }
}

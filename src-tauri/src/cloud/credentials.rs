use crate::core::error::{AppError, AppResult};
use directories::ProjectDirs;
use keyring::Entry;
use std::fs;
use std::path::PathBuf;

const KEYRING_SERVICE: &str = "soundflow-music-player";

fn fallback_credential_path(user_id: &str) -> Option<PathBuf> {
    ProjectDirs::from("org", "intelligentmusicplayer", "music-player").map(|dirs| {
        let dir = dirs.data_dir();
        let _ = fs::create_dir_all(dir);
        dir.join(format!(".session_token_{}", user_id))
    })
}

/// Store session bearer token securely using OS keyring, with restricted file fallback.
pub fn store_session_token(user_id: &str, token: &str) -> AppResult<()> {
    let key = format!("token_{}", user_id);
    let mut stored_in_keyring = false;

    if let Ok(entry) = Entry::new(KEYRING_SERVICE, &key) {
        if entry.set_password(token).is_ok() {
            if let Ok(fresh_entry) = Entry::new(KEYRING_SERVICE, &key) {
                if let Ok(pwd) = fresh_entry.get_password() {
                    if pwd == token {
                        stored_in_keyring = true;
                    }
                }
            }
        }
    }

    if !stored_in_keyring {
        // Fallback: Restricted file (0600 on Unix) in private app data directory
        if let Some(path) = fallback_credential_path(user_id) {
            fs::write(&path, token.trim()).map_err(|e| {
                AppError::Database(format!("Failed to write secure credential fallback: {}", e))
            })?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
            }
        }
    } else {
        // If stored successfully in OS keyring, ensure any legacy fallback file is removed
        if let Some(path) = fallback_credential_path(user_id) {
            if path.exists() {
                let _ = fs::remove_file(path);
            }
        }
    }

    Ok(())
}

/// Retrieve session bearer token from OS keyring, falling back to restricted file.
pub fn get_session_token(user_id: &str) -> Option<String> {
    let key = format!("token_{}", user_id);

    // 1. Try OS Keyring
    if let Ok(entry) = Entry::new(KEYRING_SERVICE, &key) {
        if let Ok(password) = entry.get_password() {
            if !password.trim().is_empty() {
                return Some(password);
            }
        }
    }

    // 2. Try Fallback file
    if let Some(path) = fallback_credential_path(user_id) {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                let trimmed = content.trim().to_string();
                if !trimmed.is_empty() {
                    return Some(trimmed);
                }
            }
        }
    }

    None
}

/// Delete session bearer token from OS keyring and fallback storage.
pub fn delete_session_token(user_id: &str) -> AppResult<()> {
    let key = format!("token_{}", user_id);

    if let Ok(entry) = Entry::new(KEYRING_SERVICE, &key) {
        let _ = entry.delete_credential();
    }

    if let Some(path) = fallback_credential_path(user_id) {
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_round_trip() {
        let user_id = "test_user_unit_123";
        let token = "my_sample_bearer_token";

        store_session_token(user_id, token).expect("store session token");
        let retrieved = get_session_token(user_id);
        assert_eq!(retrieved.as_deref(), Some(token));

        delete_session_token(user_id).expect("delete session token");
        let after_del = get_session_token(user_id);
        assert!(after_del.is_none());
    }
}

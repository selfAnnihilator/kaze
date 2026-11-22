use crate::core::error::{AppError, AppResult};
use directories::ProjectDirs;
use super::secret_store::{self, SecretStore};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Set only by tests: tokens then live in a private temporary folder and the
/// OS keyring and the real profile are never touched.
static ISOLATED_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Tests only. Keeps session tokens out of the OS keyring and the user's real
/// profile for the rest of this process. Safe to call more than once.
pub fn use_isolated_store() {
    let dir = ISOLATED_DIR.get_or_init(|| {
        std::env::temp_dir().join(format!("kaze-test-credentials-{}", std::process::id()))
    });
    let _ = fs::create_dir_all(dir);
}

/// The platform secret store, unless tests isolated the store.
fn platform_store() -> Option<Box<dyn SecretStore>> {
    if ISOLATED_DIR.get().is_some() {
        return None;
    }
    secret_store::platform_store()
}

fn fallback_credential_path(user_id: &str) -> Option<PathBuf> {
    if let Some(dir) = ISOLATED_DIR.get() {
        return Some(dir.join(format!(".session_token_{}", user_id)));
    }
    if let Some(dir) = secret_store::base_dir() {
        let _ = fs::create_dir_all(dir);
        return Some(dir.join(format!(".session_token_{}", user_id)));
    }
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

    if let Some(store) = platform_store() {
        stored_in_keyring = store.set(&key, token).is_ok();
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
    if let Some(password) = platform_store().and_then(|store| store.get(&key)) {
        if !password.trim().is_empty() {
            return Some(password);
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

    if let Some(store) = platform_store() {
        store.delete(&key);
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
        use_isolated_store();
        let user_id = "test_user_unit_123";
        let token = "my_sample_bearer_token";

        store_session_token(user_id, token).expect("store session token");
        let retrieved = get_session_token(user_id);
        assert_eq!(retrieved.as_deref(), Some(token));

        delete_session_token(user_id).expect("delete session token");
        let after_del = get_session_token(user_id);
        assert!(after_del.is_none());
        let real = ProjectDirs::from("org", "intelligentmusicplayer", "music-player")
            .map(|dirs| dirs.data_dir().join(format!(".session_token_{}", user_id)));
        assert!(real.is_none_or(|p| !p.exists()), "tests never write the real profile");
    }
}

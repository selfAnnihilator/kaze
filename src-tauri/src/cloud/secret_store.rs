//! Secret-store seam: the OS keyring on desktop, the kaze-media Keystore
//! bridge on Android. Callers fall back to the private 0600 file when the
//! store is unavailable.

use std::path::PathBuf;
use std::sync::OnceLock;

/// A minimal key/value secret backend.
pub trait SecretStore {
    fn set(&self, key: &str, value: &str) -> Result<(), String>;
    fn get(&self, key: &str) -> Option<String>;
    fn delete(&self, key: &str);
}

/// Private data directory injected on platforms where `directories` has no
/// answer (Android). Desktop never sets it.
static BASE_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn set_base_dir(dir: PathBuf) {
    let _ = BASE_DIR.set(dir);
}

pub fn base_dir() -> Option<&'static PathBuf> {
    BASE_DIR.get()
}

#[cfg(not(target_os = "android"))]
mod imp {
    use super::SecretStore;
    use keyring::Entry;

    const SERVICE: &str = "soundflow-music-player";

    pub struct KeyringStore;

    impl SecretStore for KeyringStore {
        fn set(&self, key: &str, value: &str) -> Result<(), String> {
            let entry = Entry::new(SERVICE, key).map_err(|e| e.to_string())?;
            entry.set_password(value).map_err(|e| e.to_string())?;
            match entry.get_password() {
                Ok(read) if read == value => Ok(()),
                _ => Err("keyring read-back mismatch".into()),
            }
        }

        fn get(&self, key: &str) -> Option<String> {
            Entry::new(SERVICE, key).ok()?.get_password().ok()
        }

        fn delete(&self, key: &str) {
            if let Ok(entry) = Entry::new(SERVICE, key) {
                let _ = entry.delete_credential();
            }
        }
    }

    pub fn platform_store() -> Option<Box<dyn SecretStore>> {
        Some(Box::new(KeyringStore))
    }
}

#[cfg(target_os = "android")]
mod imp {
    use super::SecretStore;

    /// Stub until the kaze-media Keystore bridge lands: reports unavailable so
    /// callers use the app-private file fallback.
    pub struct KeystoreStore;

    impl SecretStore for KeystoreStore {
        fn set(&self, _key: &str, _value: &str) -> Result<(), String> {
            Err("Keystore bridge not wired yet".into())
        }
        fn get(&self, _key: &str) -> Option<String> {
            None
        }
        fn delete(&self, _key: &str) {}
    }

    pub fn platform_store() -> Option<Box<dyn SecretStore>> {
        Some(Box::new(KeystoreStore))
    }
}

pub use imp::platform_store;

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Memory(RefCell<HashMap<String, String>>);

    impl SecretStore for Memory {
        fn set(&self, key: &str, value: &str) -> Result<(), String> {
            self.0.borrow_mut().insert(key.into(), value.into());
            Ok(())
        }
        fn get(&self, key: &str) -> Option<String> {
            self.0.borrow().get(key).cloned()
        }
        fn delete(&self, key: &str) {
            self.0.borrow_mut().remove(key);
        }
    }

    #[test]
    fn trait_round_trip() {
        let store: Box<dyn SecretStore> = Box::new(Memory::default());
        store.set("k", "v").unwrap();
        assert_eq!(store.get("k").as_deref(), Some("v"));
        store.delete("k");
        assert!(store.get("k").is_none());
    }
}

use std::sync::{Mutex, OnceLock, RwLock};

use thiserror::Error;

const KEYRING_SERVICE: &str = "echo-build";
const KEYRING_ACCOUNT: &str = "openrouter-api-key";

static CACHED_API_KEY: OnceLock<RwLock<Option<String>>> = OnceLock::new();
static CREDENTIAL_OPERATION_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Error)]
pub enum ApiKeyStoreError {
    #[error("the operating system credential store is unavailable: {0}")]
    CredentialStore(String),
    #[error("the stored OpenRouter API key is not valid UTF-8")]
    InvalidUtf8,
    #[error(
        "the API key was saved securely, but the legacy plaintext API key could not be removed: {0}"
    )]
    LegacyCleanup(#[source] std::io::Error),
    #[error("the legacy plaintext API key could not be removed during logout: {0}")]
    LegacyDelete(#[source] std::io::Error),
}

trait CredentialStore {
    fn load(&self) -> Result<Option<Vec<u8>>, ApiKeyStoreError>;
    fn save(&self, secret: &[u8]) -> Result<(), ApiKeyStoreError>;
    fn delete(&self) -> Result<(), ApiKeyStoreError>;
}

struct SystemCredentialStore;

impl SystemCredentialStore {
    fn entry(&self) -> Result<keyring::v1::Entry, ApiKeyStoreError> {
        keyring::v1::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
            .map_err(|error| ApiKeyStoreError::CredentialStore(error.to_string()))
    }
}

impl CredentialStore for SystemCredentialStore {
    fn load(&self) -> Result<Option<Vec<u8>>, ApiKeyStoreError> {
        match self.entry()?.get_secret() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::v1::Error::NoEntry) => Ok(None),
            Err(error) => Err(ApiKeyStoreError::CredentialStore(error.to_string())),
        }
    }

    fn save(&self, secret: &[u8]) -> Result<(), ApiKeyStoreError> {
        self.entry()?
            .set_secret(secret)
            .map_err(|error| ApiKeyStoreError::CredentialStore(error.to_string()))
    }

    fn delete(&self) -> Result<(), ApiKeyStoreError> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(keyring::v1::Error::NoEntry) => Ok(()),
            Err(error) => Err(ApiKeyStoreError::CredentialStore(error.to_string())),
        }
    }
}

fn cache() -> &'static RwLock<Option<String>> {
    CACHED_API_KEY.get_or_init(|| RwLock::new(None))
}

fn cache_key(api_key: Option<String>) {
    *cache()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = api_key;
}

fn load_with(store: &impl CredentialStore) -> Result<Option<String>, ApiKeyStoreError> {
    store
        .load()?
        .map(|secret| String::from_utf8(secret).map_err(|_| ApiKeyStoreError::InvalidUtf8))
        .transpose()
}

fn save_with(
    store: &impl CredentialStore,
    grok_home: &std::path::Path,
    api_key: &str,
) -> Result<(), ApiKeyStoreError> {
    store.save(api_key.as_bytes())?;
    super::storage::clear_api_key_strict(grok_home).map_err(ApiKeyStoreError::LegacyCleanup)?;
    Ok(())
}

fn delete_with(store: &impl CredentialStore) -> Result<(), ApiKeyStoreError> {
    store.delete()
}

fn delete_all_with(
    store: &impl CredentialStore,
    grok_home: &std::path::Path,
) -> Result<(), ApiKeyStoreError> {
    let secure_delete = delete_with(store);
    let legacy_delete = super::storage::clear_api_key_strict(grok_home);

    secure_delete?;
    legacy_delete.map_err(ApiKeyStoreError::LegacyDelete)
}

fn save_serialized_with(
    store: &impl CredentialStore,
    grok_home: &std::path::Path,
    api_key: &str,
    operation_lock: &Mutex<()>,
) -> Result<(), ApiKeyStoreError> {
    let _operation = operation_lock
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    save_with(store, grok_home, api_key)
}

fn save_and_cache_with(
    store: &impl CredentialStore,
    grok_home: &std::path::Path,
    api_key: &str,
) -> Result<(), ApiKeyStoreError> {
    save_with(store, grok_home, api_key)?;
    cache_key(Some(api_key.to_owned()));
    Ok(())
}

fn delete_all_and_clear_cache_with(
    store: &impl CredentialStore,
    grok_home: &std::path::Path,
) -> Result<(), ApiKeyStoreError> {
    delete_all_with(store, grok_home)?;
    cache_key(None);
    Ok(())
}

fn delete_all_serialized_with(
    store: &impl CredentialStore,
    grok_home: &std::path::Path,
    operation_lock: &Mutex<()>,
) -> Result<(), ApiKeyStoreError> {
    let _operation = operation_lock
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    delete_all_with(store, grok_home)
}

pub fn load_api_key(grok_home: &std::path::Path) -> Result<Option<String>, ApiKeyStoreError> {
    let _operation = CREDENTIAL_OPERATION_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cache_key(None);
    let api_key = load_with(&SystemCredentialStore)?;
    if api_key.is_some() {
        super::storage::clear_api_key_strict(grok_home).map_err(ApiKeyStoreError::LegacyCleanup)?;
    }
    cache_key(api_key.clone());
    Ok(api_key)
}

pub fn save_api_key(grok_home: &std::path::Path, api_key: &str) -> Result<(), ApiKeyStoreError> {
    let _operation = CREDENTIAL_OPERATION_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    save_and_cache_with(&SystemCredentialStore, grok_home, api_key)
}

pub fn delete_api_key() -> Result<(), ApiKeyStoreError> {
    let _operation = CREDENTIAL_OPERATION_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    delete_with(&SystemCredentialStore)?;
    cache_key(None);
    Ok(())
}

pub fn delete_api_key_and_legacy(grok_home: &std::path::Path) -> Result<(), ApiKeyStoreError> {
    let _operation = CREDENTIAL_OPERATION_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    delete_all_and_clear_cache_with(&SystemCredentialStore, grok_home)
}

pub fn cached_api_key() -> Option<String> {
    cache()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Condvar, Mutex};

    use serial_test::serial;

    use super::*;

    #[derive(Default)]
    struct MockStore {
        secret: Mutex<Option<Vec<u8>>>,
        failure: Mutex<Option<&'static str>>,
        calls: Mutex<HashMap<&'static str, usize>>,
    }

    impl MockStore {
        fn fail_with(message: &'static str) -> Self {
            Self {
                failure: Mutex::new(Some(message)),
                ..Self::default()
            }
        }

        fn note(&self, call: &'static str) {
            *self.calls.lock().unwrap().entry(call).or_default() += 1;
        }

        fn check_failure(&self) -> Result<(), ApiKeyStoreError> {
            match *self.failure.lock().unwrap() {
                Some(message) => Err(ApiKeyStoreError::CredentialStore(message.to_owned())),
                None => Ok(()),
            }
        }
    }

    impl CredentialStore for MockStore {
        fn load(&self) -> Result<Option<Vec<u8>>, ApiKeyStoreError> {
            self.note("load");
            self.check_failure()?;
            Ok(self.secret.lock().unwrap().clone())
        }

        fn save(&self, secret: &[u8]) -> Result<(), ApiKeyStoreError> {
            self.note("save");
            self.check_failure()?;
            *self.secret.lock().unwrap() = Some(secret.to_vec());
            Ok(())
        }

        fn delete(&self) -> Result<(), ApiKeyStoreError> {
            self.note("delete");
            self.check_failure()?;
            *self.secret.lock().unwrap() = None;
            Ok(())
        }
    }

    #[test]
    fn missing_key_loads_as_none() {
        assert_eq!(load_with(&MockStore::default()).unwrap(), None);
    }

    #[test]
    fn stored_key_loads_and_replaces() {
        let store = MockStore::default();
        let home = tempfile::tempdir().unwrap();

        save_with(&store, home.path(), "first-key").unwrap();
        save_with(&store, home.path(), "second-key").unwrap();

        assert_eq!(load_with(&store).unwrap().as_deref(), Some("second-key"));
    }

    #[test]
    fn invalid_utf8_is_rejected() {
        let store = MockStore::default();
        *store.secret.lock().unwrap() = Some(vec![0xff]);

        assert!(matches!(
            load_with(&store),
            Err(ApiKeyStoreError::InvalidUtf8)
        ));
    }

    #[test]
    fn unavailable_store_fails_closed() {
        let store = MockStore::fail_with("locked");
        let home = tempfile::tempdir().unwrap();

        assert!(load_with(&store).is_err());
        assert!(save_with(&store, home.path(), "never-written").is_err());
        assert!(delete_with(&store).is_err());
        assert!(!home.path().join("auth.json").exists());
    }

    #[test]
    fn unavailable_store_errors_never_include_key_material() {
        let store = MockStore::fail_with("locked");
        let home = tempfile::tempdir().unwrap();
        let secret = "do-not-print-this-key";

        let error = save_with(&store, home.path(), secret)
            .unwrap_err()
            .to_string();

        assert!(!error.contains(secret));
    }

    #[test]
    fn delete_is_idempotent() {
        let store = MockStore::default();

        delete_with(&store).unwrap();
        delete_with(&store).unwrap();

        assert_eq!(*store.calls.lock().unwrap().get("delete").unwrap(), 2);
    }

    #[test]
    fn legacy_api_key_is_removed_after_secure_save() {
        let store = MockStore::default();
        let home = tempfile::tempdir().unwrap();
        let mut auth = super::super::model::AuthStore::new();
        auth.insert(
            super::super::model::API_KEY_SCOPE.to_owned(),
            super::super::model::GrokAuth {
                key: "plaintext-key".to_owned(),
                ..Default::default()
            },
        );
        auth.insert(
            "keep-me".to_owned(),
            super::super::model::GrokAuth {
                key: "session-token".to_owned(),
                ..Default::default()
            },
        );
        super::super::storage::write_auth_json(&home.path().join("auth.json"), &auth).unwrap();

        save_with(&store, home.path(), "secure-key").unwrap();

        let remaining =
            super::super::storage::read_auth_json(&home.path().join("auth.json")).unwrap();
        assert!(!remaining.contains_key(super::super::model::API_KEY_SCOPE));
        assert_eq!(remaining.get("keep-me").unwrap().key, "session-token");
    }

    #[test]
    fn secure_save_reports_legacy_cleanup_as_partial_success() {
        let store = MockStore::default();
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("auth.json"), b"not json").unwrap();

        let error = save_with(&store, home.path(), "secure-key").unwrap_err();

        assert!(matches!(error, ApiKeyStoreError::LegacyCleanup(_)));
        assert!(error.to_string().contains("saved securely"));
        assert_eq!(load_with(&store).unwrap().as_deref(), Some("secure-key"));
    }

    struct BlockingSaveStore {
        secret: Mutex<Option<Vec<u8>>>,
        save_started: Condvar,
        save_state: Mutex<(bool, bool)>,
    }

    impl BlockingSaveStore {
        fn new() -> Self {
            Self {
                secret: Mutex::new(None),
                save_started: Condvar::new(),
                save_state: Mutex::new((false, false)),
            }
        }

        fn wait_for_save(&self) {
            let state = self.save_state.lock().unwrap();
            drop(
                self.save_started
                    .wait_while(state, |(started, _)| !*started)
                    .unwrap(),
            );
        }

        fn release_save(&self) {
            let mut state = self.save_state.lock().unwrap();
            state.1 = true;
            self.save_started.notify_all();
        }
    }

    impl CredentialStore for BlockingSaveStore {
        fn load(&self) -> Result<Option<Vec<u8>>, ApiKeyStoreError> {
            Ok(self.secret.lock().unwrap().clone())
        }

        fn save(&self, secret: &[u8]) -> Result<(), ApiKeyStoreError> {
            let mut state = self.save_state.lock().unwrap();
            state.0 = true;
            self.save_started.notify_all();
            drop(
                self.save_started
                    .wait_while(state, |(_, released)| !*released)
                    .unwrap(),
            );
            *self.secret.lock().unwrap() = Some(secret.to_vec());
            Ok(())
        }

        fn delete(&self) -> Result<(), ApiKeyStoreError> {
            *self.secret.lock().unwrap() = None;
            Ok(())
        }
    }

    #[test]
    fn concurrent_save_then_logout_finishes_with_no_key() {
        let store = Arc::new(BlockingSaveStore::new());
        let operation_lock = Arc::new(Mutex::new(()));
        let home = tempfile::tempdir().unwrap();
        let home_path = Arc::new(home.path().to_path_buf());

        let save_store = store.clone();
        let save_lock = operation_lock.clone();
        let save_home = home_path.clone();
        let save = std::thread::spawn(move || {
            save_serialized_with(
                save_store.as_ref(),
                save_home.as_ref(),
                "concurrent-key",
                save_lock.as_ref(),
            )
        });

        store.wait_for_save();

        let logout_store = store.clone();
        let logout_lock = operation_lock.clone();
        let logout_home = home_path.clone();
        let (logout_started, logout_waiting) = std::sync::mpsc::channel();
        let logout = std::thread::spawn(move || {
            logout_started.send(()).unwrap();
            delete_all_serialized_with(
                logout_store.as_ref(),
                logout_home.as_ref(),
                logout_lock.as_ref(),
            )
        });

        logout_waiting.recv().unwrap();
        store.release_save();
        save.join().unwrap().unwrap();
        logout.join().unwrap().unwrap();

        assert_eq!(load_with(store.as_ref()).unwrap(), None);
    }

    #[test]
    #[serial(api_key_cache)]
    fn failed_replacement_keeps_previous_cached_key() {
        let home = tempfile::tempdir().unwrap();
        cache_key(Some("previous-key".to_string()));

        let result = save_and_cache_with(
            &MockStore::fail_with("save unavailable"),
            home.path(),
            "replacement-key",
        );

        assert!(result.is_err());
        assert_eq!(cached_api_key().as_deref(), Some("previous-key"));
        cache_key(None);
    }

    #[test]
    #[serial(api_key_cache)]
    fn failed_logout_keeps_previous_cached_key() {
        let home = tempfile::tempdir().unwrap();
        cache_key(Some("previous-key".to_string()));

        let result = delete_all_and_clear_cache_with(
            &MockStore::fail_with("delete unavailable"),
            home.path(),
        );

        assert!(result.is_err());
        assert_eq!(cached_api_key().as_deref(), Some("previous-key"));
        cache_key(None);
    }
}

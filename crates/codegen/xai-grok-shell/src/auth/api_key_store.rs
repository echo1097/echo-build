use std::sync::{OnceLock, RwLock};

use thiserror::Error;

const KEYRING_SERVICE: &str = "echo-build";
const KEYRING_ACCOUNT: &str = "openrouter-api-key";

static CACHED_API_KEY: OnceLock<RwLock<Option<String>>> = OnceLock::new();

#[derive(Debug, Error)]
pub enum ApiKeyStoreError {
    #[error("the operating system credential store is unavailable: {0}")]
    CredentialStore(String),
    #[error("the stored OpenRouter API key is not valid UTF-8")]
    InvalidUtf8,
    #[error("the legacy plaintext API key could not be removed: {0}")]
    LegacyCleanup(#[source] std::io::Error),
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

pub fn load_api_key(grok_home: &std::path::Path) -> Result<Option<String>, ApiKeyStoreError> {
    cache_key(None);
    let api_key = load_with(&SystemCredentialStore)?;
    if api_key.is_some() {
        super::storage::clear_api_key_strict(grok_home).map_err(ApiKeyStoreError::LegacyCleanup)?;
    }
    cache_key(api_key.clone());
    Ok(api_key)
}

pub fn save_api_key(grok_home: &std::path::Path, api_key: &str) -> Result<(), ApiKeyStoreError> {
    cache_key(None);
    save_with(&SystemCredentialStore, grok_home, api_key)?;
    cache_key(Some(api_key.to_owned()));
    Ok(())
}

pub fn delete_api_key() -> Result<(), ApiKeyStoreError> {
    cache_key(None);
    delete_with(&SystemCredentialStore)?;
    Ok(())
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
    use std::sync::Mutex;

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

        assert!(save_with(&store, home.path(), "never-written").is_err());
        assert!(!home.path().join("auth.json").exists());
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
}

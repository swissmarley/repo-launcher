//! Secrets (GitHub sign-in, project keys, the user's Anthropic key) live only in the OS keychain:
//! macOS Keychain, Windows Credential Manager or the Linux Secret Service. There is no plain-text
//! fallback (DECISIONS D-007). Every key we write is also recorded in the database so uninstall
//! can delete it; the keychain itself is never enumerated.

use crate::problem::{Problem, ProblemCode, Result};
use crate::redact;
use crate::store::Store;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

/// The keychain "service" every entry is filed under.
pub const SERVICE: &str = "app.repolauncher.desktop";

/// Windows Credential Manager's blob limit. Applied on every OS so a value that works on one
/// computer works on all of them.
pub const MAX_SECRET_BYTES: usize = 2560;

pub const GITHUB_TOKEN: &str = "github.token";
pub const ANTHROPIC_KEY: &str = "ai.anthropic_key";

pub fn project_env_key(project_id: &str, name: &str) -> String {
    format!("project.{project_id}.env.{name}")
}

/// A secret value. `Debug` and `Display` never show it; call [`Secret::expose`] where the
/// value is really needed (a request header, a child process environment).
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(hidden)")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[hidden]")
    }
}

/// Where secrets are kept. The app uses [`KeychainStore`]; tests use [`MemoryStore`].
pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<Secret>>;
    fn set(&self, key: &str, value: &Secret) -> Result<()>;
    /// Deleting a key that doesn't exist is not an error.
    fn delete(&self, key: &str) -> Result<()>;
    /// Fails with `KEYCHAIN_UNAVAILABLE` when secrets can't be stored on this computer.
    fn check(&self) -> Result<()>;
}

fn unavailable(detail: impl fmt::Display) -> Problem {
    Problem::new(ProblemCode::KeychainUnavailable).detail(detail)
}

pub fn validate(value: &Secret) -> Result<()> {
    let len = value.expose().len();
    if len > MAX_SECRET_BYTES {
        return Err(Problem::new(ProblemCode::SecretTooLarge)
            .param("limit", MAX_SECRET_BYTES.to_string())
            .param("size", len.to_string()));
    }
    Ok(())
}

/// The operating system's credential store, through `keyring-core`.
pub struct KeychainStore {
    inner: std::result::Result<Arc<keyring_core::CredentialStore>, String>,
}

impl KeychainStore {
    pub fn new() -> Self {
        Self {
            inner: platform_store(),
        }
    }

    fn entry(&self, key: &str) -> Result<keyring_core::Entry> {
        let store = self.inner.as_ref().map_err(unavailable)?;
        store.build(SERVICE, key, None).map_err(keyring_problem)
    }
}

impl Default for KeychainStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "macos")]
fn platform_store() -> std::result::Result<Arc<keyring_core::CredentialStore>, String> {
    apple_native_keyring_store::keychain::Store::new()
        .map(|s| s as Arc<keyring_core::CredentialStore>)
        .map_err(|e| e.to_string())
}

#[cfg(windows)]
fn platform_store() -> std::result::Result<Arc<keyring_core::CredentialStore>, String> {
    windows_native_keyring_store::Store::new()
        .map(|s| s as Arc<keyring_core::CredentialStore>)
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
fn platform_store() -> std::result::Result<Arc<keyring_core::CredentialStore>, String> {
    zbus_secret_service_keyring_store::Store::new()
        .map(|s| s as Arc<keyring_core::CredentialStore>)
        .map_err(|e| e.to_string())
}

#[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
fn platform_store() -> std::result::Result<Arc<keyring_core::CredentialStore>, String> {
    Err("no supported keychain on this operating system".into())
}

/// Maps keychain errors to problems. Only `Display` is used: some variants carry the secret's
/// bytes, which their `Debug` output would print.
fn keyring_problem(e: keyring_core::Error) -> Problem {
    use keyring_core::Error as E;
    match e {
        E::TooLong(..) => {
            Problem::new(ProblemCode::SecretTooLarge).param("limit", MAX_SECRET_BYTES.to_string())
        }
        E::BadEncoding(_) | E::BadDataFormat(..) => {
            Problem::internal("a saved secret couldn't be read")
        }
        other => unavailable(other),
    }
}

impl SecretStore for KeychainStore {
    fn get(&self, key: &str) -> Result<Option<Secret>> {
        match self.entry(key)?.get_secret() {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(s) => Ok(Some(Secret(s))),
                Err(_) => Err(Problem::internal("a saved secret couldn't be read")),
            },
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(e) => Err(keyring_problem(e)),
        }
    }

    fn set(&self, key: &str, value: &Secret) -> Result<()> {
        validate(value)?;
        self.entry(key)?
            .set_secret(value.expose().as_bytes())
            .map_err(keyring_problem)
    }

    fn delete(&self, key: &str) -> Result<()> {
        match self.entry(key)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(e) => Err(keyring_problem(e)),
        }
    }

    fn check(&self) -> Result<()> {
        // A lookup that finds nothing proves the store answers (and, on Linux, is unlocked).
        self.get("probe").map(|_| ())
    }
}

/// Keeps secrets in memory. For tests, and as a stand-in nowhere else.
#[derive(Default)]
pub struct MemoryStore {
    map: Mutex<HashMap<String, Secret>>,
    unavailable: bool,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// A store that behaves like a Linux desktop without a Secret Service.
    pub fn unavailable() -> Self {
        Self {
            unavailable: true,
            ..Self::default()
        }
    }

    pub fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self
            .map
            .lock()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        keys.sort();
        keys
    }

    fn guard(&self) -> Result<std::sync::MutexGuard<'_, HashMap<String, Secret>>> {
        if self.unavailable {
            return Err(unavailable("memory store configured as unavailable"));
        }
        Ok(self.map.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

impl SecretStore for MemoryStore {
    fn get(&self, key: &str) -> Result<Option<Secret>> {
        Ok(self.guard()?.get(key).cloned())
    }
    fn set(&self, key: &str, value: &Secret) -> Result<()> {
        validate(value)?;
        self.guard()?.insert(key.to_owned(), value.clone());
        Ok(())
    }
    fn delete(&self, key: &str) -> Result<()> {
        self.guard()?.remove(key);
        Ok(())
    }
    fn check(&self) -> Result<()> {
        self.guard().map(|_| ())
    }
}

/// The secret store plus its index in the database. Every value read or written is
/// registered with the log scrubber so it can never appear in a log or diagnostics.
pub struct Vault {
    store: Arc<dyn SecretStore>,
}

impl Vault {
    pub fn new(store: Arc<dyn SecretStore>) -> Self {
        Self { store }
    }

    pub fn check(&self) -> Result<()> {
        self.store.check()
    }

    pub fn get(&self, key: &str) -> Result<Option<Secret>> {
        let value = self.store.get(key)?;
        if let Some(v) = &value {
            redact::remember(v.expose());
        }
        Ok(value)
    }

    /// Saves a secret. `project_id` ties it to a project so removing the project deletes it.
    pub fn set(
        &self,
        db: &Store,
        key: &str,
        project_id: Option<&str>,
        value: &Secret,
    ) -> Result<()> {
        validate(value)?;
        redact::remember(value.expose());
        // Index first, so a crash between the two steps can't leave an entry uninstall
        // doesn't know about. If the keychain write fails, undo a fresh index row.
        let existed = db.has_secret(key)?;
        db.index_secret(key, project_id)?;
        let result = self.store.set(key, value);
        if result.is_err() && !existed {
            let _ = db.unindex_secret(key);
        }
        result
    }

    pub fn delete(&self, db: &Store, key: &str) -> Result<()> {
        self.store.delete(key)?;
        db.unindex_secret(key)
    }

    /// Deletes every secret belonging to a project.
    pub fn delete_project(&self, db: &Store, project_id: &str) -> Result<()> {
        for key in db.secret_keys_for(project_id)? {
            self.delete(db, &key)?;
        }
        Ok(())
    }

    /// Deletes everything Repo Launcher ever stored in the keychain (used by full uninstall).
    pub fn delete_all(&self, db: &Store) -> Result<usize> {
        let keys = db.all_secret_keys()?;
        for key in &keys {
            self.delete(db, key)?;
        }
        Ok(keys.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_never_formats_its_value() {
        let s = Secret::new("sk-ant-supersecretvalue");
        assert!(!format!("{s:?} {s}").contains("supersecret"));
    }

    #[test]
    fn rejects_values_over_the_windows_limit() {
        let store = MemoryStore::new();
        let big = Secret::new("x".repeat(MAX_SECRET_BYTES + 1));
        let err = store.set("k", &big).unwrap_err();
        assert_eq!(err.code, ProblemCode::SecretTooLarge);
        assert!(
            store
                .set("k", &Secret::new("x".repeat(MAX_SECRET_BYTES)))
                .is_ok()
        );
    }

    #[test]
    fn vault_indexes_and_deletes_by_project() {
        let db = Store::in_memory().unwrap();
        let mem = Arc::new(MemoryStore::new());
        let vault = Vault::new(mem.clone());
        vault
            .set(
                &db,
                &project_env_key("p1", "A"),
                Some("p1"),
                &Secret::new("aaaa1"),
            )
            .unwrap();
        vault
            .set(
                &db,
                &project_env_key("p2", "B"),
                Some("p2"),
                &Secret::new("bbbb2"),
            )
            .unwrap();
        vault
            .set(&db, GITHUB_TOKEN, None, &Secret::new("ghu_x"))
            .unwrap();
        vault.delete_project(&db, "p1").unwrap();
        assert_eq!(
            mem.keys(),
            vec![GITHUB_TOKEN.to_owned(), "project.p2.env.B".into()]
        );
        assert_eq!(vault.delete_all(&db).unwrap(), 2);
        assert!(mem.keys().is_empty());
    }

    #[test]
    fn unavailable_store_reports_a_clear_problem() {
        let store = MemoryStore::unavailable();
        assert_eq!(
            store.check().unwrap_err().code,
            ProblemCode::KeychainUnavailable
        );
    }

    /// Talks to the real keychain of the computer running the tests.
    /// `cargo test -p launcher-core --lib secrets -- --ignored`
    #[test]
    #[ignore]
    fn real_keychain_round_trip() {
        let store = KeychainStore::new();
        store.check().expect("keychain available");
        let key = format!("test.{}", uuid::Uuid::new_v4().simple());
        store.set(&key, &Secret::new("round-trip-value")).unwrap();
        assert_eq!(
            store.get(&key).unwrap().map(|s| s.expose().to_owned()),
            Some("round-trip-value".into())
        );
        store.delete(&key).unwrap();
        assert!(store.get(&key).unwrap().is_none());
        store.delete(&key).unwrap();
    }
}

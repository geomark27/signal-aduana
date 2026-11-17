//! Where the token is kept between sessions.
//!
//! - [`SystemSecretStore`]: Windows Credential Manager (the kernel keyring on Linux).
//! - [`MemorySecretStore`]: memory only; used in simulated mode and in tests.
//!
//! The site and email are not secrets and go in `config.toml` (see `config.rs`).

use std::collections::HashMap;
use std::sync::Mutex;

use secrecy::{ExposeSecret, SecretString};

use crate::error::AppResult;

pub trait SecretStore: Send + Sync {
    fn save(&self, service: &str, account: &str, secret: &SecretString) -> AppResult<()>;
    fn read(&self, service: &str, account: &str) -> AppResult<Option<SecretString>>;
    fn delete(&self, service: &str, account: &str) -> AppResult<()>;
}

/// Name under which the credential appears in Windows: `signal-aduana:empresa.atlassian.net`.
pub fn service_for(host: &str) -> String {
    format!("signal-aduana:{host}")
}

pub struct SystemSecretStore;

impl SecretStore for SystemSecretStore {
    fn save(&self, service: &str, account: &str, secret: &SecretString) -> AppResult<()> {
        keyring::Entry::new(service, account)?.set_password(secret.expose_secret())?;
        Ok(())
    }

    fn read(&self, service: &str, account: &str) -> AppResult<Option<SecretString>> {
        match keyring::Entry::new(service, account)?.get_password() {
            Ok(s) => Ok(Some(SecretString::from(s))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn delete(&self, service: &str, account: &str) -> AppResult<()> {
        match keyring::Entry::new(service, account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Default)]
pub struct MemorySecretStore {
    entries: Mutex<HashMap<(String, String), SecretString>>,
}

impl SecretStore for MemorySecretStore {
    fn save(&self, service: &str, account: &str, secret: &SecretString) -> AppResult<()> {
        self.entries
            .lock()
            .expect("mutex")
            .insert((service.into(), account.into()), secret.clone());
        Ok(())
    }

    fn read(&self, service: &str, account: &str) -> AppResult<Option<SecretString>> {
        Ok(self
            .entries
            .lock()
            .expect("mutex")
            .get(&(service.into(), account.into()))
            .cloned())
    }

    fn delete(&self, service: &str, account: &str) -> AppResult<()> {
        self.entries.lock().expect("mutex").remove(&(service.into(), account.into()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_saves_reads_and_deletes() {
        let store = MemorySecretStore::default();
        let s = service_for("x.atlassian.net");
        assert!(store.read(&s, "a@b.com").unwrap().is_none());
        store.save(&s, "a@b.com", &SecretString::from("t1")).unwrap();
        assert_eq!(store.read(&s, "a@b.com").unwrap().unwrap().expose_secret(), "t1");
        store.delete(&s, "a@b.com").unwrap();
        assert!(store.read(&s, "a@b.com").unwrap().is_none());
        // Deleting something that does not exist is not an error.
        store.delete(&s, "a@b.com").unwrap();
    }
}

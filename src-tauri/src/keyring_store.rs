use keyring::Entry;
use std::sync::Mutex;

const SERVICE_NAME: &str = "ingot-launcher";
const CHUNK_SERVICE: &str = "ingot-launcher-secret-chunks-v1";
const CHUNK_PREFIX: &str = "ingot-keyring-chunks-v1:";
// Keep every entry small in both encodings: Windows uses UTF-16 while macOS
// Keychain and Linux Secret Service use UTF-8 bytes.
const MAX_UTF8_BYTES: usize = 2560;
const MAX_UTF16_UNITS: usize = 1280;
const MAX_CHUNKS: usize = 4096;
static STORE_LOCK: Mutex<()> = Mutex::new(());

trait SecretStore {
    fn read(&self, service: &str, user: &str) -> Result<Option<String>, String>;
    fn write(&self, service: &str, user: &str, secret: &str) -> Result<(), String>;
    fn delete(&self, service: &str, user: &str) -> Result<(), String>;
}

struct OsStore;

fn entry(service: &str, user: &str) -> Result<Entry, String> {
    Entry::new(service, user).map_err(|e| format!("Failed to access OS keyring: {e}"))
}

impl SecretStore for OsStore {
    fn read(&self, service: &str, user: &str) -> Result<Option<String>, String> {
        match entry(service, user)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("Failed to read secret from OS keyring: {e}")),
        }
    }

    fn write(&self, service: &str, user: &str, secret: &str) -> Result<(), String> {
        entry(service, user)?
            .set_password(secret)
            .map_err(|e| format!("Failed to save secret in OS keyring: {e}"))
    }

    fn delete(&self, service: &str, user: &str) -> Result<(), String> {
        match entry(service, user)?.delete_credential() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("Failed to delete secret from OS keyring: {e}")),
        }
    }
}

struct Manifest {
    generation: uuid::Uuid,
    count: usize,
}

impl Manifest {
    fn parse(value: &str) -> Result<Option<Self>, String> {
        let Some(rest) = value.strip_prefix(CHUNK_PREFIX) else {
            return Ok(None);
        };
        let invalid = || "Invalid chunked credential metadata in OS keyring".to_string();
        let (generation, count) = rest.split_once(':').ok_or_else(invalid)?;
        let generation = uuid::Uuid::parse_str(generation).map_err(|_| invalid())?;
        let count = count.parse::<usize>().map_err(|_| invalid())?;
        if count == 0 || count > MAX_CHUNKS {
            return Err(invalid());
        }
        Ok(Some(Self { generation, count }))
    }

    fn key(&self, index: usize) -> String {
        format!("{}:{index}", self.generation)
    }

    fn delete_chunks(&self, store: &impl SecretStore) -> Result<(), String> {
        // Attempt every deletion even if one entry is unavailable.
        let mut result = Ok(());
        for index in 0..self.count {
            if let Err(error) = store.delete(CHUNK_SERVICE, &self.key(index)) {
                result = Err(error);
            }
        }
        result
    }
}

fn split_secret(secret: &str) -> Vec<&str> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut units = 0;
    for (offset, ch) in secret.char_indices() {
        if units + ch.len_utf16() > MAX_UTF16_UNITS
            || offset + ch.len_utf8() - start > MAX_UTF8_BYTES
        {
            chunks.push(&secret[start..offset]);
            start = offset;
            units = 0;
        }
        units += ch.len_utf16();
    }
    chunks.push(&secret[start..]);
    chunks
}

fn save(store: &impl SecretStore, account_id: &str, secret: &str) -> Result<(), String> {
    let old = store.read(SERVICE_NAME, account_id)?;
    let old_manifest = old.as_deref().map(Manifest::parse).transpose()?.flatten();
    let chunks = split_secret(secret);
    // Escape even short values that start with our reserved metadata prefix.
    if chunks.len() == 1 && !secret.starts_with(CHUNK_PREFIX) {
        store.write(SERVICE_NAME, account_id, secret)?;
    } else {
        if chunks.len() > MAX_CHUNKS {
            return Err("Account secret exceeds supported keyring storage size".to_string());
        }
        let manifest = Manifest {
            generation: uuid::Uuid::new_v4(),
            count: chunks.len(),
        };
        // Publish the pointer only after every chunk is stored. Failed writes leave
        // the previous session intact, including during refresh-token rotation.
        let result = (|| {
            for (index, chunk) in chunks.iter().enumerate() {
                store.write(CHUNK_SERVICE, &manifest.key(index), chunk)?;
            }
            store.write(
                SERVICE_NAME,
                account_id,
                &format!("{CHUNK_PREFIX}{}:{}", manifest.generation, manifest.count),
            )
        })();
        if let Err(error) = result {
            let _ = manifest.delete_chunks(store);
            return Err(error);
        }
    }
    if let Some(old_manifest) = old_manifest {
        // The new session is already committed; cleanup must not fail sign-in.
        let _ = old_manifest.delete_chunks(store);
    }
    Ok(())
}

fn get(store: &impl SecretStore, account_id: &str) -> Result<Option<String>, String> {
    let Some(value) = store.read(SERVICE_NAME, account_id)? else {
        return Ok(None);
    };
    let Some(manifest) = Manifest::parse(&value)? else {
        return Ok(Some(value));
    };
    let mut secret = String::new();
    for index in 0..manifest.count {
        let chunk = store
            .read(CHUNK_SERVICE, &manifest.key(index))?
            .ok_or("Stored account credentials are incomplete. Please sign in again.")?;
        secret.push_str(&chunk);
    }
    Ok(Some(secret))
}

fn delete(store: &impl SecretStore, account_id: &str) -> Result<(), String> {
    if let Some(value) = store.read(SERVICE_NAME, account_id)? {
        if let Some(manifest) = Manifest::parse(&value)? {
            manifest.delete_chunks(store)?;
        }
    }
    store.delete(SERVICE_NAME, account_id)
}

/// Stores credentials in the OS vault, splitting large tokens into secure entries.
pub fn save_secret(account_id: &str, secret: &str) -> Result<(), String> {
    let _guard = STORE_LOCK.lock().map_err(|_| "OS keyring lock poisoned")?;
    save(&OsStore, account_id, secret)
}

/// Reads both legacy single-entry credentials and chunked credentials.
pub fn get_secret(account_id: &str) -> Result<Option<String>, String> {
    let _guard = STORE_LOCK.lock().map_err(|_| "OS keyring lock poisoned")?;
    get(&OsStore, account_id)
}

/// Deletes an account secret from the OS Credential Vault
pub fn delete_secret(account_id: &str) -> Result<(), String> {
    let _guard = STORE_LOCK.lock().map_err(|_| "OS keyring lock poisoned")?;
    delete(&OsStore, account_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;

    #[derive(Default)]
    struct TestStore {
        entries: RefCell<HashMap<(String, String), String>>,
        writes: Cell<usize>,
        fail_write: Cell<Option<usize>>,
    }

    impl SecretStore for TestStore {
        fn read(&self, service: &str, user: &str) -> Result<Option<String>, String> {
            Ok(self
                .entries
                .borrow()
                .get(&(service.into(), user.into()))
                .cloned())
        }

        fn write(&self, service: &str, user: &str, secret: &str) -> Result<(), String> {
            let attempt = self.writes.get() + 1;
            self.writes.set(attempt);
            if self.fail_write.get() == Some(attempt) {
                return Err("Injected write failure".into());
            }
            if secret.encode_utf16().count() * 2 > 2560 {
                return Err("Windows credential blob limit exceeded".into());
            }
            if secret.len() > MAX_UTF8_BYTES {
                return Err("UTF-8 credential blob limit exceeded".into());
            }
            self.entries
                .borrow_mut()
                .insert((service.into(), user.into()), secret.into());
            Ok(())
        }

        fn delete(&self, service: &str, user: &str) -> Result<(), String> {
            self.entries
                .borrow_mut()
                .remove(&(service.into(), user.into()));
            Ok(())
        }
    }

    #[test]
    fn microsoft_tokens_round_trip_refresh_and_delete() {
        let store = TestStore::default();
        let credentials = serde_json::json!({
            "accessToken": "a".repeat(4000),
            "refreshToken": "r".repeat(2000),
            "expiresAt": 1234567890,
        })
        .to_string();
        save(&store, "microsoft:test", &credentials).unwrap();
        assert_eq!(get(&store, "microsoft:test").unwrap(), Some(credentials));
        let updated = "new token".repeat(900);
        save(&store, "microsoft:test", &updated).unwrap();
        assert_eq!(
            get(&store, "microsoft:test").unwrap(),
            Some(updated.clone())
        );
        assert_eq!(
            store.entries.borrow().len(),
            split_secret(&updated).len() + 1
        );
        delete(&store, "microsoft:test").unwrap();
        assert!(store.entries.borrow().is_empty());
        assert_eq!(get(&store, "microsoft:test").unwrap(), None);
        delete(&store, "microsoft:test").unwrap();
    }

    #[test]
    fn handles_utf16_boundaries_and_legacy_credentials() {
        let store = TestStore::default();
        for value in [
            String::new(),
            "a".repeat(1280),
            "a".repeat(1281),
            "界".repeat(1280),
            format!("{}😀é{}", "a".repeat(1279), "😀".repeat(1400)),
            format!("{CHUNK_PREFIX}literal secret"),
        ] {
            save(&store, "account", &value).unwrap();
            assert_eq!(get(&store, "account").unwrap(), Some(value));
        }
        save(&store, "account", "legacy JSON").unwrap();
        assert_eq!(
            store.read(SERVICE_NAME, "account").unwrap().as_deref(),
            Some("legacy JSON")
        );
        assert_eq!(store.entries.borrow().len(), 1);
    }

    #[test]
    fn failed_chunk_or_manifest_write_preserves_previous_session() {
        for old in ["legacy secret".to_string(), "old".repeat(2000)] {
            // New value has three chunks; fail each chunk or the final pointer.
            for fail_offset in 1..=4 {
                let store = TestStore::default();
                save(&store, "account", &old).unwrap();
                let old_entries = store.entries.borrow().clone();
                store.fail_write.set(Some(store.writes.get() + fail_offset));
                assert!(save(&store, "account", &"a".repeat(3000)).is_err());
                assert_eq!(get(&store, "account").unwrap(), Some(old.clone()));
                assert_eq!(*store.entries.borrow(), old_entries);
            }
        }
    }

    #[test]
    fn incomplete_credentials_fail_instead_of_returning_partial_tokens() {
        let store = TestStore::default();
        save(&store, "account", &"a".repeat(3000)).unwrap();
        let value = store.read(SERVICE_NAME, "account").unwrap().unwrap();
        let manifest = Manifest::parse(&value).unwrap().unwrap();
        store.delete(CHUNK_SERVICE, &manifest.key(1)).unwrap();
        assert!(get(&store, "account").unwrap_err().contains("incomplete"));
    }

    #[test]
    fn rejects_invalid_metadata() {
        for value in [
            format!("{CHUNK_PREFIX}invalid:1"),
            format!("{CHUNK_PREFIX}{}:0", uuid::Uuid::nil()),
            format!("{CHUNK_PREFIX}{}:{}", uuid::Uuid::nil(), MAX_CHUNKS + 1),
        ] {
            assert!(Manifest::parse(&value).is_err());
        }
    }

    #[test]
    fn chunks_fit_both_utf8_and_utf16_limits() {
        for value in [
            "a".repeat(10000),
            "é".repeat(10000),
            "界".repeat(10000),
            "😀".repeat(10000),
            "aé界😀".repeat(10000),
        ] {
            let chunks = split_secret(&value);
            assert_eq!(chunks.concat(), value);
            for chunk in chunks {
                assert!(chunk.len() <= MAX_UTF8_BYTES);
                assert!(chunk.encode_utf16().count() <= MAX_UTF16_UNITS);
            }
        }
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    #[test]
    #[ignore = "requires an available, unlocked OS vault; run explicitly in platform CI"]
    fn native_vault_round_trip() {
        let account_id = format!("keyring-regression-test:{}", uuid::Uuid::new_v4());
        let result = (|| -> Result<(), String> {
            // Includes the maximum Windows blob and three-byte UTF-8 characters.
            for secret in [
                "a".repeat(MAX_UTF16_UNITS),
                format!("{}{}", "synthetic-token".repeat(500), "😀界".repeat(700)),
                "rotated-token".repeat(800),
            ] {
                save_secret(&account_id, &secret)?;
                assert_eq!(get_secret(&account_id)?, Some(secret));
            }
            save_secret(&account_id, "refreshed-token")?;
            assert_eq!(get_secret(&account_id)?.as_deref(), Some("refreshed-token"));
            Ok(())
        })();
        let cleanup = delete_secret(&account_id);
        result.unwrap();
        cleanup.unwrap();
        assert_eq!(get_secret(&account_id).unwrap(), None);
    }
}

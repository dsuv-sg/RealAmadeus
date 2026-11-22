//! `localStorage`-backed [`amadeus_core::storage::KvStore`]. OWNED BY: ui-foundation.
//!
//! Keys and value formats are V1's (raw strings in `window.localStorage`), so users keep their
//! settings, memory and achievements when V2 replaces V1 on the same origin. The desktop webview
//! uses the same store (it has its own persistent localStorage origin).
//!
//! V1 threw when `localStorage` was unavailable or full. V2 degrades gracefully instead: when
//! storage cannot be opened (privacy mode, sandboxed iframe, blocked site data) or a write fails
//! (quota exceeded), the value is kept in an in-memory overlay for the rest of the session, so
//! reads stay consistent with writes. A warning is logged on the first failure. The exception is
//! [`KvStore::try_set`]: it keeps the same fallback but logs nothing and returns the error, so the
//! caller can report it. The memory manager uses it to log V1's
//! `[MemoryManager] Failed to persist memory to localStorage:` warning on every failed write.
//!
//! Use the shared instance from [`local_store`] everywhere (settings, memory, achievements, vector
//! store), like V1's single `window.localStorage`.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fmt;

use amadeus_core::storage::KvStore;

/// Raw string storage (the browser's `Storage`, or a fake in tests). Errors are messages.
trait RawStore {
    fn get(&self, key: &str) -> Result<Option<String>, String>;
    fn set(&self, key: &str, value: &str) -> Result<(), String>;
    fn remove(&self, key: &str) -> Result<(), String>;
    fn keys(&self) -> Result<Vec<String>, String>;
}

impl RawStore for web_sys::Storage {
    fn get(&self, key: &str) -> Result<Option<String>, String> {
        self.get_item(key).map_err(|e| crate::env::js_to_string(&e))
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        self.set_item(key, value)
            .map_err(|e| crate::env::js_to_string(&e))
    }

    fn remove(&self, key: &str) -> Result<(), String> {
        self.remove_item(key)
            .map_err(|e| crate::env::js_to_string(&e))
    }

    fn keys(&self) -> Result<Vec<String>, String> {
        let len = self.length().map_err(|e| crate::env::js_to_string(&e))?;
        let mut keys = Vec::with_capacity(len as usize);
        for index in 0..len {
            if let Ok(Some(key)) = self.key(index) {
                keys.push(key);
            }
        }
        Ok(keys)
    }
}

/// A [`RawStore`] (optional) shadowed by in-memory values that could not be persisted.
struct Overlay<B> {
    backend: Option<B>,
    memory: RefCell<HashMap<String, String>>,
    warned: Cell<bool>,
}

impl<B: RawStore> Overlay<B> {
    fn new(backend: Option<B>) -> Self {
        Self {
            backend,
            memory: RefCell::new(HashMap::new()),
            warned: Cell::new(false),
        }
    }

    fn report(&self, action: &str, key: &str, error: &str) {
        if self.warned.replace(true) {
            log::debug!("localStorage {action} failed for {key:?}: {error}");
        } else {
            log::warn!("localStorage {action} failed for {key:?}: {error}; keeping the value in memory for this session");
        }
    }

    /// Writes `value` through to the backend. When the backend rejects it, the value is kept in
    /// memory and the backend's error is returned (nothing is logged here). Without a backend the
    /// value lives in memory only, which [`LocalStorageKv::new`] already reported, so that is `Ok`.
    fn write(&self, key: &str, value: &str) -> Result<(), String> {
        match self.backend.as_ref().map(|backend| backend.set(key, value)) {
            Some(Ok(())) => {
                self.memory.borrow_mut().remove(key);
                Ok(())
            }
            // Rejected by the backend (`Some(Err)`) or memory-only (`None`).
            outcome => {
                self.memory
                    .borrow_mut()
                    .insert(key.to_owned(), value.to_owned());
                outcome.unwrap_or(Ok(()))
            }
        }
    }
}

impl<B: RawStore> KvStore for Overlay<B> {
    fn get(&self, key: &str) -> Option<String> {
        if let Some(value) = self.memory.borrow().get(key) {
            return Some(value.clone());
        }
        match self.backend.as_ref()?.get(key) {
            Ok(value) => value,
            Err(error) => {
                self.report("read", key, &error);
                None
            }
        }
    }

    fn set(&self, key: &str, value: &str) {
        if let Err(error) = self.write(key, value) {
            self.report("write", key, &error);
        }
    }

    fn try_set(&self, key: &str, value: &str) -> Result<(), String> {
        self.write(key, value)
    }

    fn remove(&self, key: &str) {
        self.memory.borrow_mut().remove(key);
        if let Some(backend) = &self.backend {
            if let Err(error) = backend.remove(key) {
                self.report("remove", key, &error);
            }
        }
    }

    fn keys(&self) -> Vec<String> {
        let mut keys = match self.backend.as_ref().map(RawStore::keys) {
            Some(Ok(keys)) => keys,
            Some(Err(error)) => {
                self.report("key listing", "*", &error);
                Vec::new()
            }
            None => Vec::new(),
        };
        let mut seen: HashSet<String> = keys.iter().cloned().collect();
        for key in self.memory.borrow().keys() {
            if seen.insert(key.clone()) {
                keys.push(key.clone());
            }
        }
        keys
    }
}

/// `window.localStorage` as a [`KvStore`], with the in-memory fallback described in the module
/// docs. Prefer the shared instance [`local_store`].
pub struct LocalStorageKv {
    inner: Overlay<web_sys::Storage>,
}

impl LocalStorageKv {
    /// Opens `window.localStorage` (once); memory-only when it is unavailable.
    pub fn new() -> Self {
        let storage = browser_local_storage();
        if storage.is_none() {
            log::warn!("localStorage is unavailable; settings and data are kept in memory for this session");
        }
        Self {
            inner: Overlay::new(storage),
        }
    }

    /// A store that never touches `localStorage` (tests, previews).
    pub fn memory_only() -> Self {
        Self {
            inner: Overlay::new(None),
        }
    }

    /// True when backed by `window.localStorage` (writes may still fall back to memory).
    pub fn is_persistent(&self) -> bool {
        self.inner.backend.is_some()
    }
}

impl Default for LocalStorageKv {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for LocalStorageKv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalStorageKv")
            .field("persistent", &self.is_persistent())
            .field("memory_entries", &self.inner.memory.borrow().len())
            .finish()
    }
}

impl KvStore for LocalStorageKv {
    fn get(&self, key: &str) -> Option<String> {
        self.inner.get(key)
    }

    fn set(&self, key: &str, value: &str) {
        self.inner.set(key, value);
    }

    fn try_set(&self, key: &str, value: &str) -> Result<(), String> {
        self.inner.try_set(key, value)
    }

    fn remove(&self, key: &str) {
        self.inner.remove(key);
    }

    fn keys(&self) -> Vec<String> {
        self.inner.keys()
    }
}

#[cfg(target_arch = "wasm32")]
fn browser_local_storage() -> Option<web_sys::Storage> {
    // The getter itself throws (SecurityError) when site data is blocked.
    web_sys::window().and_then(|w| w.local_storage().ok().flatten())
}

#[cfg(not(target_arch = "wasm32"))]
fn browser_local_storage() -> Option<web_sys::Storage> {
    None // host unit tests: no browser, memory only
}

thread_local! {
    static LOCAL_STORE: &'static LocalStorageKv = Box::leak(Box::new(LocalStorageKv::new()));
}

/// The page-wide store (V1's `window.localStorage`). Lives for the whole page; created on first
/// use. Pass it to every core manager (`AppSettings::new(local_store())`, …).
pub fn local_store() -> &'static LocalStorageKv {
    LOCAL_STORE.with(|store| *store)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fake backend: `fail_writes` makes every `set` fail (quota exceeded).
    #[derive(Default)]
    struct Fake {
        data: RefCell<Vec<(String, String)>>,
        fail_writes: Cell<bool>,
    }

    impl Fake {
        fn value(&self, key: &str) -> Option<String> {
            self.data
                .borrow()
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
        }
    }

    impl RawStore for &Fake {
        fn get(&self, key: &str) -> Result<Option<String>, String> {
            Ok(self.value(key))
        }
        fn set(&self, key: &str, value: &str) -> Result<(), String> {
            if self.fail_writes.get() {
                return Err("QuotaExceededError".to_owned());
            }
            let mut data = self.data.borrow_mut();
            match data.iter_mut().find(|(k, _)| k == key) {
                Some(entry) => entry.1 = value.to_owned(),
                None => data.push((key.to_owned(), value.to_owned())),
            }
            Ok(())
        }
        fn remove(&self, key: &str) -> Result<(), String> {
            self.data.borrow_mut().retain(|(k, _)| k != key);
            Ok(())
        }
        fn keys(&self) -> Result<Vec<String>, String> {
            Ok(self.data.borrow().iter().map(|(k, _)| k.clone()).collect())
        }
    }

    #[test]
    fn persists_through_backend() {
        let fake = Fake::default();
        let store = Overlay::new(Some(&fake));
        assert_eq!(store.get("Config_Language"), None);
        store.set("Config_Language", "1");
        assert_eq!(store.get("Config_Language").as_deref(), Some("1"));
        assert_eq!(fake.value("Config_Language").as_deref(), Some("1"));
        assert!(store.memory.borrow().is_empty());
        store.set("Config_ApiKey", "");
        assert_eq!(
            store.get("Config_ApiKey").as_deref(),
            Some(""),
            "empty values are kept"
        );
        store.remove("Config_Language");
        assert_eq!(store.get("Config_Language"), None);
        assert_eq!(store.keys(), vec!["Config_ApiKey".to_owned()]);
    }

    #[test]
    fn failed_writes_shadow_the_backend() {
        let fake = Fake::default();
        let store = Overlay::new(Some(&fake));
        store.set("RealAmadeus.Memory", "old");
        fake.fail_writes.set(true);
        store.set("RealAmadeus.Memory", "new");
        assert_eq!(
            store.get("RealAmadeus.Memory").as_deref(),
            Some("new"),
            "reads follow writes"
        );
        assert_eq!(fake.value("RealAmadeus.Memory").as_deref(), Some("old"));
        store.set("only_in_memory", "x");
        let mut keys = store.keys();
        keys.sort();
        assert_eq!(
            keys,
            vec!["RealAmadeus.Memory".to_owned(), "only_in_memory".to_owned()]
        );
        // Once writes work again the backend becomes authoritative.
        fake.fail_writes.set(false);
        store.set("RealAmadeus.Memory", "newest");
        assert!(!store.memory.borrow().contains_key("RealAmadeus.Memory"));
        assert_eq!(store.get("RealAmadeus.Memory").as_deref(), Some("newest"));
        store.remove("only_in_memory");
        assert_eq!(store.get("only_in_memory"), None);
    }

    #[test]
    fn try_set_reports_failures_without_the_generic_warning() {
        let fake = Fake::default();
        let store = Overlay::new(Some(&fake));
        assert_eq!(store.try_set("RealAmadeus.Memory", "old"), Ok(()));
        assert_eq!(fake.value("RealAmadeus.Memory").as_deref(), Some("old"));
        fake.fail_writes.set(true);
        // Every failure is returned to the caller, which logs it (V1 MemoryManager).
        for value in ["new", "newer"] {
            assert_eq!(
                store.try_set("RealAmadeus.Memory", value),
                Err("QuotaExceededError".to_owned())
            );
            assert_eq!(store.get("RealAmadeus.Memory").as_deref(), Some(value));
        }
        assert_eq!(fake.value("RealAmadeus.Memory").as_deref(), Some("old"));
        assert!(
            !store.warned.get(),
            "try_set leaves the once-per-session warning to the other keys"
        );
        // A plain `set` of another key still logs the generic warning once.
        store.set("Config_Language", "1");
        assert!(store.warned.get());
        assert_eq!(store.get("Config_Language").as_deref(), Some("1"));
        // Writes work again: the backend becomes authoritative.
        fake.fail_writes.set(false);
        assert_eq!(store.try_set("RealAmadeus.Memory", "newest"), Ok(()));
        assert!(!store.memory.borrow().contains_key("RealAmadeus.Memory"));
        assert_eq!(fake.value("RealAmadeus.Memory").as_deref(), Some("newest"));
        // Memory-only: nothing to persist to (reported once at startup), so `Ok`.
        let memory_only = LocalStorageKv::memory_only();
        assert_eq!(memory_only.try_set("RealAmadeus.Memory", "{}"), Ok(()));
        assert_eq!(memory_only.get("RealAmadeus.Memory").as_deref(), Some("{}"));
    }

    #[test]
    fn memory_only_store() {
        let store = LocalStorageKv::memory_only();
        assert!(!store.is_persistent());
        store.set("k", "v");
        assert_eq!(store.get("k").as_deref(), Some("v"));
        assert_eq!(store.keys(), vec!["k".to_owned()]);
        store.remove("k");
        assert_eq!(store.get("k"), None);
        // The shared instance is memory-only on the host and is a singleton.
        assert!(std::ptr::eq(local_store(), local_store()));
        assert!(!local_store().is_persistent());
    }
}

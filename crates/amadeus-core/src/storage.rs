//! Key/value persistence abstraction.
//!
//! The web frontend implements this over `window.localStorage`; tests use
//! [`MemoryStore`]. Keys and value formats MUST stay identical to V1 so that
//! existing users keep their settings when they move to V2 on the same origin.

use std::cell::RefCell;
use std::collections::HashMap;

/// Minimal synchronous string key/value store (mirrors the Web Storage API).
pub trait KvStore {
    fn get(&self, key: &str) -> Option<String>;
    /// `setItem`. Never fails: a store that cannot persist the value handles that itself (the
    /// browser store keeps it in memory for the session and logs a warning).
    fn set(&self, key: &str, value: &str);
    /// `setItem` for a caller that reports failed writes itself (V1 `MemoryManager` caught the
    /// `setItem` exception and logged it). `Err` carries the error text (JS `String(error)`,
    /// e.g. `"QuotaExceededError: …"`). The store applies the same fallback as [`set`] but logs
    /// nothing for a failed write. The default is `set` followed by `Ok`, for stores that cannot
    /// fail.
    ///
    /// [`set`]: KvStore::set
    fn try_set(&self, key: &str, value: &str) -> Result<(), String> {
        self.set(key, value);
        Ok(())
    }
    fn remove(&self, key: &str);
    /// All keys currently stored (order unspecified).
    fn keys(&self) -> Vec<String>;
}

/// In-memory [`KvStore`] used by tests and as a fallback when storage is unavailable.
#[derive(Default)]
pub struct MemoryStore {
    inner: RefCell<HashMap<String, String>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl KvStore for MemoryStore {
    fn get(&self, key: &str) -> Option<String> {
        self.inner.borrow().get(key).cloned()
    }
    fn set(&self, key: &str, value: &str) {
        self.inner
            .borrow_mut()
            .insert(key.to_owned(), value.to_owned());
    }
    fn remove(&self, key: &str) {
        self.inner.borrow_mut().remove(key);
    }
    fn keys(&self) -> Vec<String> {
        self.inner.borrow().keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_try_set_defaults_to_set() {
        let store = MemoryStore::new();
        assert_eq!(store.try_set("k", "v"), Ok(()));
        assert_eq!(store.get("k").as_deref(), Some("v"));
        assert_eq!(store.try_set("k", ""), Ok(()));
        assert_eq!(store.get("k").as_deref(), Some(""));
        // Through a trait object too (the managers take `S: KvStore + ?Sized`).
        let dyn_store: &dyn KvStore = &store;
        assert_eq!(dyn_store.try_set("other", "x"), Ok(()));
        let mut keys = store.keys();
        keys.sort();
        assert_eq!(keys, ["k", "other"]);
    }
}

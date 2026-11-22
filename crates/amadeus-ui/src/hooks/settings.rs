//! Reactive settings: the V1 `AppSettings` singleton plus `useAppSettingInt` /
//! `useAppSettingFloat` (`RealAmadeusApp.tsx` L423–443).
//!
//! * One page-wide [`AppSettings`] over [`local_store`] ([`app_settings`]). Always write through
//!   it (or the `set_app_setting_*` helpers): every `set_*` emits the core
//!   `settings_changed` emitter, exactly like V1's `settingsChanged`, so services that filter
//!   by key prefix (TTS/STT/RAG, achievements) see the change.
//! * A reactive revision counter (a Leptos signal) is bumped by that emitter after **every**
//!   write — any key, even an unchanged value ([`SettingsCtx::track`] follows it).
//! * Tracked reads ([`SettingsCtx`] getters, the `use_*` memos below, `crate::i18n::t`)
//!   subscribe to **their key** only: after every settings write the key's raw `localStorage`
//!   value is re-read, and readers re-run only when it changed (V1's per-key
//!   `useSyncExternalStore` snapshot + `Object.is` bail-out). So an effect may read one key and
//!   write another (or rewrite the same value) without re-triggering itself. The memos notify
//!   only when their *parsed* value changed.
//! * The state is global (not a context), so it works in every component, effect, callback and
//!   service without a provider. [`SettingsCtx`] is a zero-sized `Copy + Send + Sync` handle,
//!   usable inside Leptos `Callback`s.
//!
//! ```ignore
//! let eye_tracking = use_app_setting_bool(keys::EYE_TRACKING.key, false);   // Memo<bool>
//! let speed = use_setting(keys::AUTO_SPEED);                                 // Memo<f64>
//! set_app_setting_int("Config_ScreenMode", 0);
//! keys::EYE_TRACKING.get(app_settings());                                    // untracked read
//! ```

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;

use amadeus_core::settings::{AppSettings, BoolSetting, FloatSetting, IntSetting, StringSetting};
use leptos::prelude::*;

use crate::storage::{local_store, LocalStorageKv};

/// The concrete settings type of the UI (V1 `AppSettings` over `window.localStorage`).
pub type Settings = AppSettings<'static, LocalStorageKv>;

struct SettingsGlobal {
    settings: &'static Settings,
    revision: ArcRwSignal<u64>,
    /// Raw stored value of every key a tracked getter has read: re-read after each settings
    /// write, notifying only on a change. Created on first read, one small memo per key for the
    /// page's lifetime.
    raw_values: RefCell<HashMap<String, ArcMemo<Option<String>>>>,
}

impl SettingsGlobal {
    fn new() -> Self {
        let settings: &'static Settings = Box::leak(Box::new(AppSettings::new(local_store())));
        // Not arena-allocated (nor the memos): never disposed with the component that happened
        // to touch the settings first.
        let revision = ArcRwSignal::new(0_u64);
        let bump = revision.clone();
        settings.settings_changed().connect(move |_key: &str| {
            bump.update(|rev| *rev = rev.wrapping_add(1));
        });
        Self {
            settings,
            revision,
            raw_values: RefCell::new(HashMap::new()),
        }
    }

    fn raw_value(&self, key: &str) -> ArcMemo<Option<String>> {
        if let Some(memo) = self.raw_values.borrow().get(key) {
            return memo.clone();
        }
        let revision = self.revision.clone();
        let owned = key.to_owned();
        let memo = ArcMemo::new(move |_| {
            revision.track();
            app_settings().read_raw(&owned)
        });
        self.raw_values
            .borrow_mut()
            .insert(key.to_owned(), memo.clone());
        memo
    }
}

thread_local! {
    static GLOBAL: SettingsGlobal = SettingsGlobal::new();
}

/// The page-wide V1 `AppSettings` singleton (untracked reads, writes, `settings_changed()`,
/// `initialize_defaults()`, typed descriptors such as `keys::EYE_TRACKING.get(app_settings())`).
pub fn app_settings() -> &'static Settings {
    GLOBAL.with(|g| g.settings)
}

/// Handle to the global settings state: V1 `AppSettings` + the reactive revision.
///
/// Zero-sized, `Copy + Send + Sync`: capture it freely in closures and `Callback`s. Getters
/// are **tracked per key** (the current reactive scope re-runs when that key's stored value
/// changed); setters write through [`app_settings`] (emit `settings_changed`, bump the
/// revision). [`revision`](Self::revision) / [`track`](Self::track) follow *every* write: an
/// effect that uses them must not write settings unconditionally, or it re-triggers itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SettingsCtx;

impl SettingsCtx {
    /// The V1 `AppSettings` singleton ([`app_settings`]).
    pub fn settings(self) -> &'static Settings {
        app_settings()
    }

    /// Number of settings writes so far (tracked: changes on every write).
    pub fn revision(self) -> u64 {
        GLOBAL.with(|g| g.revision.get())
    }

    /// Number of settings writes so far (untracked).
    pub fn revision_untracked(self) -> u64 {
        GLOBAL.with(|g| g.revision.get_untracked())
    }

    /// Subscribes the current reactive scope to every settings write.
    pub fn track(self) {
        GLOBAL.with(|g| g.revision.track());
    }

    /// Subscribes the current reactive scope to `key`: it re-runs after a settings write only
    /// when the raw stored value of `key` changed.
    pub fn track_key(self, key: &str) {
        let memo = GLOBAL.with(|g| g.raw_value(key));
        // `with` subscribes *and* brings the memo up to date (a never-read memo has no sources).
        memo.with(|_| ());
    }

    /// The revision as a Leptos signal (allocated in the current owner).
    pub fn revision_signal(self) -> Signal<u64> {
        Signal::from(GLOBAL.with(|g| g.revision.clone()))
    }

    /// Tracked V1 `getInt(key, default)`.
    pub fn get_int(self, key: &str, default: i64) -> i64 {
        self.track_key(key);
        app_settings().get_int(key, default)
    }

    /// Tracked V1 `getFloat(key, default)`.
    pub fn get_float(self, key: &str, default: f64) -> f64 {
        self.track_key(key);
        app_settings().get_float(key, default)
    }

    /// Tracked V1 `getString(key, default)` (default only when the key is absent).
    pub fn get_string(self, key: &str, default: &str) -> String {
        self.track_key(key);
        app_settings().get_string(key, default)
    }

    /// Tracked V1 bool-int read (`getInt(key, default ? 1 : 0) === 1`).
    pub fn get_bool(self, key: &str, default: bool) -> bool {
        self.track_key(key);
        app_settings().get_bool(key, default)
    }

    /// V1 `setInt(key, value)`.
    pub fn set_int(self, key: &str, value: i64) {
        app_settings().set_int(key, value);
    }

    /// V1 `setFloat(key, value)` (JS number formatting).
    pub fn set_float(self, key: &str, value: f64) {
        app_settings().set_float(key, value);
    }

    /// V1 `setString(key, value)`.
    pub fn set_string(self, key: &str, value: &str) {
        app_settings().set_string(key, value);
    }

    /// V1 bool-int write (`setInt(key, value ? 1 : 0)`).
    pub fn set_bool(self, key: &str, value: bool) {
        app_settings().set_bool(key, value);
    }
}

/// The settings handle (see [`SettingsCtx`]); works without any provider.
pub fn use_settings() -> SettingsCtx {
    SettingsCtx
}

/// V1 `useAppSettingInt(key, default)`: `parseInt` semantics, re-read when the key's stored
/// value changed, notifies only when the parsed value changed. `key` may be `&'static str` or a
/// `String` (per-provider keys such as `keys::api_key_for(p)`).
pub fn use_app_setting_int(key: impl Into<Cow<'static, str>>, default: i64) -> Memo<i64> {
    let key = key.into();
    Memo::new(move |_| SettingsCtx.get_int(&key, default))
}

/// V1 `useAppSettingFloat(key, default)` (`parseFloat`; non-finite → default).
pub fn use_app_setting_float(key: impl Into<Cow<'static, str>>, default: f64) -> Memo<f64> {
    let key = key.into();
    Memo::new(move |_| SettingsCtx.get_float(&key, default))
}

/// Reactive `getString(key, default)`.
pub fn use_app_setting_string(
    key: impl Into<Cow<'static, str>>,
    default: impl Into<Cow<'static, str>>,
) -> Memo<String> {
    let key = key.into();
    let default = default.into();
    Memo::new(move |_| SettingsCtx.get_string(&key, &default))
}

/// Reactive bool-int read: V1 `useAppSettingInt(key, d ? 1 : 0) === 1`.
pub fn use_app_setting_bool(key: impl Into<Cow<'static, str>>, default: bool) -> Memo<bool> {
    let key = key.into();
    Memo::new(move |_| SettingsCtx.get_bool(&key, default))
}

/// A typed settings descriptor of `amadeus_core::settings::keys` (`BoolSetting`,
/// `IntSetting`, `FloatSetting`, `StringSetting`), readable reactively with [`use_setting`].
pub trait SettingDescriptor: Copy + Send + Sync + 'static {
    /// Value type of the setting.
    type Value: Clone + PartialEq + Send + Sync + 'static;
    /// The `localStorage` key.
    fn key(&self) -> &'static str;
    /// Untracked read with the descriptor's V1 default.
    fn read(&self, settings: &Settings) -> Self::Value;
}

impl SettingDescriptor for BoolSetting {
    type Value = bool;
    fn key(&self) -> &'static str {
        self.key
    }
    fn read(&self, settings: &Settings) -> bool {
        self.get(settings)
    }
}

impl SettingDescriptor for IntSetting {
    type Value = i64;
    fn key(&self) -> &'static str {
        self.key
    }
    fn read(&self, settings: &Settings) -> i64 {
        self.get(settings)
    }
}

impl SettingDescriptor for FloatSetting {
    type Value = f64;
    fn key(&self) -> &'static str {
        self.key
    }
    fn read(&self, settings: &Settings) -> f64 {
        self.get(settings)
    }
}

impl SettingDescriptor for StringSetting {
    type Value = String;
    fn key(&self) -> &'static str {
        self.key
    }
    fn read(&self, settings: &Settings) -> String {
        self.get(settings)
    }
}

/// Reactive read of a typed descriptor, e.g. `use_setting(keys::AUTO_MODE)` → `Memo<bool>`.
pub fn use_setting<D: SettingDescriptor>(setting: D) -> Memo<D::Value> {
    Memo::new(move |_| {
        SettingsCtx.track_key(setting.key());
        setting.read(app_settings())
    })
}

/// V1 `AppSettings.setInt(key, value)` (emits `settings_changed`, bumps the revision).
pub fn set_app_setting_int(key: &str, value: i64) {
    app_settings().set_int(key, value);
}

/// V1 `AppSettings.setFloat(key, value)`.
pub fn set_app_setting_float(key: &str, value: f64) {
    app_settings().set_float(key, value);
}

/// V1 `AppSettings.setString(key, value)`.
pub fn set_app_setting_string(key: &str, value: &str) {
    app_settings().set_string(key, value);
}

/// V1 bool-int write (`setInt(key, value ? 1 : 0)`).
pub fn set_app_setting_bool(key: &str, value: bool) {
    app_settings().set_bool(key, value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use amadeus_core::settings::keys;
    use amadeus_core::storage::KvStore;

    // Host tests: the shared store is memory-only and thread-local, so every test thread starts
    // from an empty store.

    fn reset(keys: &[&str]) {
        // Tests may share a thread (and so the thread-local store): start from a known state.
        for key in keys {
            local_store().remove(key);
        }
    }

    #[test]
    fn memos_follow_writes_and_dedupe() {
        reset(&[
            keys::LANGUAGE.key,
            "Config_AutoSpeed",
            keys::EYE_TRACKING.key,
            "Config_ApiKey_8",
        ]);
        let language = use_app_setting_int(keys::LANGUAGE.key, 0);
        let auto_speed = use_app_setting_float("Config_AutoSpeed", 3.0);
        let eye = use_setting(keys::EYE_TRACKING);
        let api_key = use_app_setting_string(keys::api_key_for(8), "fallback");
        assert_eq!(language.get_untracked(), 0);
        assert_eq!(auto_speed.get_untracked(), 3.0);
        assert!(!eye.get_untracked());
        assert_eq!(api_key.get_untracked(), "fallback");

        let before = SettingsCtx.revision_untracked();
        set_app_setting_int(keys::LANGUAGE.key, 1);
        set_app_setting_float("Config_AutoSpeed", 2.5);
        set_app_setting_bool(keys::EYE_TRACKING.key, true);
        set_app_setting_string(&keys::api_key_for(8), "");
        assert_eq!(SettingsCtx.revision_untracked(), before + 4);
        assert_eq!(language.get_untracked(), 1);
        assert_eq!(auto_speed.get_untracked(), 2.5);
        assert!(eye.get_untracked());
        assert_eq!(
            api_key.get_untracked(),
            "",
            "a stored empty string is not the default"
        );
        assert_eq!(
            local_store().get("Config_AutoSpeed").as_deref(),
            Some("2.5")
        );

        // Raw writes behind the store's back become visible on the next settings write. (V1's
        // `useSyncExternalStore` re-read the snapshot on `settingsChanged` and on every render;
        // V2 components do not re-render, so the next write is the re-read point. Nothing
        // writes `Config_*` keys outside `AppSettings` anyway.)
        local_store().set(keys::LANGUAGE.key, "3abc");
        assert_eq!(language.get_untracked(), 1);
        app_settings().set_int("Config_Unrelated", 7);
        assert_eq!(language.get_untracked(), 3);
    }

    #[test]
    fn tracked_reads_follow_only_their_key() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        reset(&["Config_TestTracked", "Config_TestOther"]);
        let runs = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&runs);
        let reader = Memo::new(move |_| {
            counter.fetch_add(1, Ordering::SeqCst);
            SettingsCtx.get_int("Config_TestTracked", 0)
        });
        let runs = move || runs.load(Ordering::SeqCst);
        assert_eq!(reader.get_untracked(), 0);
        assert_eq!(runs(), 1);

        set_app_setting_int("Config_TestOther", 5);
        assert_eq!(reader.get_untracked(), 0);
        assert_eq!(
            runs(),
            1,
            "a write of another key does not re-run the reader"
        );

        set_app_setting_int("Config_TestTracked", 7);
        assert_eq!(reader.get_untracked(), 7);
        assert_eq!(runs(), 2);

        set_app_setting_int("Config_TestTracked", 7);
        assert_eq!(reader.get_untracked(), 7);
        assert_eq!(
            runs(),
            2,
            "rewriting the same value does not re-run the reader"
        );

        // Absent → stored default: the raw value changed, so the reader re-runs once.
        reset(&["Config_TestTracked"]);
        set_app_setting_int("Config_TestOther", 6);
        assert_eq!(reader.get_untracked(), 0);
        assert_eq!(runs(), 3);
        set_app_setting_int("Config_TestTracked", 0);
        assert_eq!(reader.get_untracked(), 0);
        assert_eq!(runs(), 4);

        // `revision()` still follows every write.
        let any_write = Memo::new(|_| SettingsCtx.revision());
        let before = any_write.get_untracked();
        set_app_setting_int("Config_TestOther", 6);
        assert_eq!(any_write.get_untracked(), before + 1);
        assert_eq!(runs(), 4);
    }

    #[test]
    fn bool_int_semantics() {
        reset(&["Config_TestFlag"]);
        let flag = use_app_setting_bool("Config_TestFlag", true);
        assert!(flag.get_untracked(), "absent → default");
        SettingsCtx.set_string("Config_TestFlag", "2");
        assert!(!flag.get_untracked(), "only exactly 1 is true");
        SettingsCtx.set_int("Config_TestFlag", 1);
        assert!(flag.get_untracked());
        assert!(SettingsCtx.get_bool("Config_TestFlag", false));
    }
}

//! Port of V1 `lib/amadeus/app-settings.ts` (the `AppSettings` store over `localStorage`) plus
//! the settings-derived payload builder of `ai-service.ts` (`buildPayload`).
//!
//! Keys and value formats are byte-identical to V1 so existing users keep their data:
//! ints are written as decimal strings (`String(Math.trunc(v))`), floats with JavaScript
//! number formatting (`String(0.8)` == `"0.8"`, `String(1.0)` == `"1"`), strings verbatim.
//! Reads use `parseInt(raw, 10)` / `parseFloat(raw)` prefix semantics ([`jsnum`]).
//!
//! # Change notification
//! Like V1's `settingsChanged: Signal<[string]>`, every `set_*` call writes the value and then
//! synchronously emits [`AppSettings::settings_changed`] with the key — **even when the value
//! did not change**. [`AppSettings::initialize_defaults`] writes without emitting. In addition
//! a [`revision`](AppSettings::revision) counter is bumped before each emission so a reactive UI
//! can poll or mirror it. Use ONE shared `AppSettings` instance for writes, otherwise listeners
//! registered on another instance are not notified.

pub mod defaults;
pub mod display;
pub mod forms;
pub mod jsnum;
pub mod keys;

use std::cell::Cell;

use crate::jscompat::js_trim;
use crate::protocol::{ChatMessage, ChatRequestPayload};
use crate::storage::KvStore;
use crate::types::Emitter;

pub use defaults::{DefaultValue, DEFAULT_SETTINGS};
pub use display::{is_mobile_viewport, resolve_resolution, Resolution};
pub use forms::{ConfigApplyOutcome, ConfigFormState, MobileSettingsState, ProviderBuffers};
pub use keys::{BoolSetting, FloatSetting, IntSetting, StringSetting};

/// V1 `AppSettings` over a [`KvStore`] (the browser's `localStorage` in production).
///
/// Generic over `S: ?Sized`, so `AppSettings<'_, dyn KvStore>` works as well. The store is
/// borrowed: several managers (settings, memory, achievements, …) can share one store.
pub struct AppSettings<'a, S: KvStore + ?Sized> {
    store: &'a S,
    settings_changed: Emitter<str>,
    revision: Cell<u64>,
}

impl<S: KvStore + ?Sized> std::fmt::Debug for AppSettings<'_, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppSettings")
            .field("revision", &self.revision.get())
            .field("listeners", &self.settings_changed.len())
            .finish_non_exhaustive()
    }
}

impl<'a, S: KvStore + ?Sized> AppSettings<'a, S> {
    /// Wraps `store`. Nothing is read or written until a getter/setter is called (V1 constructs
    /// the singleton without touching storage).
    pub fn new(store: &'a S) -> Self {
        Self {
            store,
            settings_changed: Emitter::new(),
            revision: Cell::new(0),
        }
    }

    /// The underlying store.
    pub fn store(&self) -> &'a S {
        self.store
    }

    /// V1 `settingsChanged` — emitted with the key after every `set_*` call.
    pub fn settings_changed(&self) -> &Emitter<str> {
        &self.settings_changed
    }

    /// Number of `set_*` calls so far (bumped right before each `settings_changed` emission).
    pub fn revision(&self) -> u64 {
        self.revision.get()
    }

    /// V1 `hasKey`: the key is present (even with an empty value).
    pub fn has_key(&self, key: &str) -> bool {
        self.store.get(key).is_some()
    }

    /// V1 `readRaw`: the stored string, `None` when absent.
    pub fn read_raw(&self, key: &str) -> Option<String> {
        self.store.get(key)
    }

    fn write_raw(&self, key: &str, value: &str) {
        self.store.set(key, value);
    }

    fn notify(&self, key: &str) {
        self.revision.set(self.revision.get().wrapping_add(1));
        self.settings_changed.emit(key);
    }

    /// V1 `initializeDefaults()`: for every entry of [`DEFAULT_SETTINGS`] (insertion order)
    /// whose key is absent, writes `String(value)`. Existing values — even `""` — are kept.
    /// Does **not** emit `settings_changed` (nor bump the revision). Returns how many keys
    /// were written. V1 calls it once, in the root component's mount effect (after the first
    /// render, after the service singletons already read their settings with call-site
    /// defaults that equal these defaults).
    pub fn initialize_defaults(&self) -> usize {
        let mut written = 0;
        for (key, value) in DEFAULT_SETTINGS.iter() {
            if !self.has_key(key) {
                self.write_raw(key, &value.to_storage_string());
                written += 1;
            }
        }
        written
    }

    /// V1 `getInt(key, default)`: absent → `default`; otherwise `parseInt(raw, 10)`, and a
    /// non-finite result (`NaN`) → `default`. `"1.9"` → 1, `"  42abc"` → 42, `""` → default.
    /// Values beyond ±2^63 saturate (V1 would keep e.g. `1e20` as a float).
    pub fn get_int(&self, key: &str, default: i64) -> i64 {
        match self.read_raw(key) {
            None => default,
            Some(raw) => match jsnum::js_parse_int(&raw) {
                Some(v) if v.is_finite() => jsnum::js_number_to_i64(v),
                _ => default,
            },
        }
    }

    /// V1 `getFloat(key, default)`: absent → `default`; otherwise `parseFloat(raw)`, and a
    /// non-finite result (`NaN`, `"Infinity"`, `"1e400"`) → `default`.
    pub fn get_float(&self, key: &str, default: f64) -> f64 {
        match self.read_raw(key) {
            None => default,
            Some(raw) => match jsnum::js_parse_float(&raw) {
                Some(v) if v.is_finite() => v,
                _ => default,
            },
        }
    }

    /// V1 `getString(key, default)`: the raw value, `default` only when the key is absent.
    pub fn get_string(&self, key: &str, default: &str) -> String {
        self.read_raw(key).unwrap_or_else(|| default.to_owned())
    }

    /// V1 "bool-int" read: `getInt(key, default ? 1 : 0) === 1`.
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.get_int(key, i64::from(default)) == 1
    }

    /// V1 `setInt(key, value)`: writes the decimal string, then emits.
    pub fn set_int(&self, key: &str, value: i64) {
        self.write_raw(key, &value.to_string());
        self.notify(key);
    }

    /// V1 `setInt` with a JS number argument: writes `String(Math.trunc(value))`
    /// (`1.9` → `"1"`, `-0.5` → `"0"`, `NaN` → `"NaN"`), then emits.
    pub fn set_int_f64(&self, key: &str, value: f64) {
        self.write_raw(key, &jsnum::js_trunc_to_string(value));
        self.notify(key);
    }

    /// V1 `setFloat(key, value)`: writes `String(value)` with JS formatting, then emits.
    pub fn set_float(&self, key: &str, value: f64) {
        self.write_raw(key, &jsnum::js_number_to_string(value));
        self.notify(key);
    }

    /// V1 `setString(key, value)`: writes the raw value, then emits.
    pub fn set_string(&self, key: &str, value: &str) {
        self.write_raw(key, value);
        self.notify(key);
    }

    /// V1 "bool-int" write: `setInt(key, value ? 1 : 0)`.
    pub fn set_bool(&self, key: &str, value: bool) {
        self.set_int(key, i64::from(value));
    }

    /// V1 `save()`: a no-op ("Browser localStorage is already persistent"). Kept as a flush
    /// hook for API compatibility.
    pub fn save(&self) {}

    /// `getInt("Config_ApiProvider", 8)`.
    pub fn provider(&self) -> i64 {
        keys::API_PROVIDER.get(self)
    }

    /// `getInt("Config_Language", 0)` — the display language index.
    pub fn language(&self) -> i64 {
        keys::LANGUAGE.get(self)
    }

    /// True when the first-launch display-language selector should be shown (evaluate before
    /// [`Self::initialize_defaults`] writes `Config_Language`).
    ///
    /// * `Config_LanguageSelected` present → true iff it is not `1` (stays true across reloads
    ///   until the user actually selects a language).
    /// * `Config_LanguageSelected` absent and `Config_Language` already present → `false`
    ///   (returning V1/V2 user upgrading without clearing storage).
    /// * Neither key present → `true` (brand-new first launch).
    pub fn needs_initial_language_selection(&self) -> bool {
        if self.has_key(keys::LANGUAGE_SELECTED.key) {
            !keys::LANGUAGE_SELECTED.get(self)
        } else {
            !self.has_key(keys::LANGUAGE.key)
        }
    }

    /// Per-provider API key with the legacy fallback used by `buildPayload`, the mobile sheet
    /// and the status line: `getString("Config_ApiKey_{p}", "") || getString("Config_ApiKey", "")`.
    pub fn api_key_for_provider(&self, provider: i64) -> String {
        let own = self.get_string(&keys::api_key_for(provider), "");
        if own.is_empty() {
            keys::API_KEY.get(self)
        } else {
            own
        }
    }

    /// Per-provider model name with the legacy fallback:
    /// `getString("Config_ModelName_{p}", "") || getString("Config_ModelName", "")`.
    pub fn model_name_for_provider(&self, provider: i64) -> String {
        let own = self.get_string(&keys::model_name_for(provider), "");
        if own.is_empty() {
            keys::MODEL_NAME.get(self)
        } else {
            own
        }
    }
}

/// V1 `ai-service.ts` `buildPayload(messages)`: the body of `POST {base}/app/api/chat` and
/// `/chat/stream`, read from the current settings (provider default 8, per-provider key/model
/// with the legacy fallbacks, bool-ints compared with `=== 1`).
pub fn chat_request_payload<S: KvStore + ?Sized>(
    settings: &AppSettings<'_, S>,
    messages: Vec<ChatMessage>,
) -> ChatRequestPayload {
    let provider = settings.provider();
    ChatRequestPayload {
        messages,
        provider,
        api_key: settings.api_key_for_provider(provider),
        model: settings.model_name_for_provider(provider),
        web_search: keys::WEB_SEARCH.get(settings),
        vertex_project: keys::VERTEX_PROJECT.get(settings),
        vertex_location: keys::VERTEX_LOCATION.get(settings),
        local_proxy_host: keys::LOCAL_PROXY_HOST.get(settings),
        local_proxy_port: keys::LOCAL_PROXY_PORT.get(settings),
        use_gcloud_cli: keys::GCLOUD_CLI.get(settings),
        ollama_host: keys::OLLAMA_HOST.get(settings),
        openai_compatible: keys::OPENAI_COMPATIBLE.get(settings),
        openai_base_url: keys::OPENAI_BASE_URL.get(settings),
    }
}

/// The value V1 settings dialogs write for the Ollama host input:
/// `value.trim() || "http://localhost:11434"` (JS trim).
pub fn normalize_ollama_host(value: &str) -> String {
    let trimmed = js_trim(value);
    if trimmed.is_empty() {
        keys::DEFAULT_OLLAMA_HOST.to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStore;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn initialize_defaults_writes_v1_strings_in_order() {
        // Golden: the localStorage entries after V1 `initializeDefaults()` on an empty store.
        let golden: [(&str, &str); 58] = [
            ("Config_SkipLoading", "0"),
            ("Config_SkipLogin", "0"),
            ("Config_RightClickMenu", "1"),
            ("Config_EyeTracking", "0"),
            ("Config_DesktopNotifications", "1"),
            ("Config_LightweightMode", "0"),
            ("Config_TextSpeed", "1"),
            ("Config_AutoMode", "0"),
            ("Config_AutoSpeed", "3"),
            ("Config_MasterVol", "1"),
            ("Config_BGMVol", "0.8"),
            ("Config_SEVol", "1"),
            ("Config_VoiceVol", "1"),
            ("Config_ScreenMode", "1"),
            ("Config_Resolution", "0"),
            ("Config_ApiProvider", "8"),
            ("Config_ApiKey", ""),
            ("Config_ModelName", ""),
            ("Config_Language", "0"),
            ("Config_ChatLanguage", "0"),
            ("Config_WebSearch", "0"),
            ("Config_VertexProject", ""),
            ("Config_VertexLocation", "us-central1"),
            ("Config_GCloudCLI", "1"),
            ("Config_LocalProxyHost", "127.0.0.1"),
            ("Config_LocalProxyPort", ""),
            ("Config_OllamaHost", "http://localhost:11434"),
            ("Config_OpenAICompatible", "0"),
            ("Config_OpenAIBaseUrl", ""),
            ("Config_Experimental_TTS_Enabled", "0"),
            ("Config_Experimental_TTS_Speaker", "christina-jp"),
            ("Config_Experimental_TTS_ServerUrl", "http://localhost:5100"),
            ("Config_Experimental_STT_Enabled", "0"),
            ("Config_Experimental_STT_Provider", "groq"),
            ("Config_Experimental_STT_ServerUrl", "http://localhost:5101"),
            ("Config_Experimental_STT_Language", "ja"),
            ("Config_Experimental_RAG_Enabled", "0"),
            ("Config_Experimental_RAG_TopK", "5"),
            ("Config_Experimental_RAG_Threshold", "0.7"),
            ("Config_Experimental_RAG_ServerUrl", "http://localhost:5102"),
            ("Config_ApiKey_0", ""),
            ("Config_ModelName_0", ""),
            ("Config_ApiKey_1", ""),
            ("Config_ModelName_1", ""),
            ("Config_ApiKey_2", ""),
            ("Config_ModelName_2", ""),
            ("Config_ApiKey_3", ""),
            ("Config_ModelName_3", ""),
            ("Config_ApiKey_4", ""),
            ("Config_ModelName_4", ""),
            ("Config_ApiKey_5", ""),
            ("Config_ModelName_5", ""),
            ("Config_ApiKey_6", ""),
            ("Config_ModelName_6", ""),
            ("Config_ApiKey_7", ""),
            ("Config_ModelName_7", ""),
            ("Config_ApiKey_8", ""),
            ("Config_ModelName_8", ""),
        ];
        for ((key, value), (gk, gv)) in DEFAULT_SETTINGS.iter().zip(golden.iter()) {
            assert_eq!(key, gk);
            assert_eq!(value.to_storage_string(), *gv, "{key}");
        }
        let store = MemoryStore::new();
        let settings = AppSettings::new(&store);
        let hits = Rc::new(RefCell::new(0));
        let h = hits.clone();
        settings
            .settings_changed()
            .connect(move |_| *h.borrow_mut() += 1);
        assert_eq!(settings.initialize_defaults(), 58);
        assert_eq!(*hits.borrow(), 0, "initializeDefaults must not emit");
        assert_eq!(settings.revision(), 0);
        for (key, value) in golden {
            assert_eq!(store.get(key).as_deref(), Some(value), "{key}");
        }
        assert_eq!(store.keys().len(), 58);
        // Existing values (even empty strings) are never overwritten.
        store.set("Config_ApiProvider", "");
        store.set("Config_Language", "3");
        store.remove("Config_TextSpeed");
        assert_eq!(settings.initialize_defaults(), 1);
        assert_eq!(store.get("Config_ApiProvider").as_deref(), Some(""));
        assert_eq!(store.get("Config_Language").as_deref(), Some("3"));
        assert_eq!(store.get("Config_TextSpeed").as_deref(), Some("1"));
    }

    #[test]
    fn every_default_key_is_catalogued() {
        for (key, value) in DEFAULT_SETTINGS.iter() {
            assert!(keys::is_known_setting_key(key), "{key}");
            if let Some(info) = keys::ALL_SETTINGS.iter().find(|i| i.key == *key) {
                assert!(info.in_default_settings, "{key}");
                let kind_ok = matches!(
                    (info.kind, value),
                    (
                        keys::SettingKind::BoolInt | keys::SettingKind::Int,
                        DefaultValue::Int(_)
                    ) | (keys::SettingKind::Float, DefaultValue::Float(_))
                        | (keys::SettingKind::String, DefaultValue::Str(_))
                );
                assert!(kind_ok, "{key}");
            } else {
                assert!(keys::is_per_provider_key(key), "{key}");
            }
        }
        assert_eq!(
            keys::ALL_SETTINGS
                .iter()
                .filter(|i| i.in_default_settings)
                .count(),
            40
        );
        assert!(keys::is_per_provider_key("Config_ApiKey_-1"));
        assert!(!keys::is_per_provider_key("Config_ApiKey_"));
        assert!(!keys::is_per_provider_key("Config_ApiKey_x"));
        // Typed descriptor defaults equal DEFAULT_SETTINGS.
        assert_eq!(
            defaults::default_for(keys::API_PROVIDER.key),
            Some(DefaultValue::Int(keys::API_PROVIDER.default))
        );
        assert_eq!(
            defaults::default_for(keys::BGM_VOL.key),
            Some(DefaultValue::Float(keys::BGM_VOL.default))
        );
        assert_eq!(
            defaults::default_for(keys::OLLAMA_HOST.key),
            Some(DefaultValue::Str(keys::OLLAMA_HOST.default))
        );
        for bool_setting in [
            keys::SKIP_LOADING,
            keys::SKIP_LOGIN,
            keys::RIGHT_CLICK_MENU,
            keys::EYE_TRACKING,
            keys::DESKTOP_NOTIFICATIONS,
            keys::LIGHTWEIGHT_MODE,
            keys::AUTO_MODE,
            keys::WEB_SEARCH,
            keys::GCLOUD_CLI,
            keys::OPENAI_COMPATIBLE,
            keys::TTS_ENABLED,
            keys::STT_ENABLED,
            keys::RAG_ENABLED,
        ] {
            assert_eq!(
                defaults::default_for(bool_setting.key),
                Some(DefaultValue::Int(i64::from(bool_setting.default))),
                "{}",
                bool_setting.key
            );
        }
        for int_setting in [
            keys::LANGUAGE,
            keys::CHAT_LANGUAGE,
            keys::SCREEN_MODE,
            keys::RESOLUTION,
            keys::RAG_TOP_K,
        ] {
            assert_eq!(
                defaults::default_for(int_setting.key),
                Some(DefaultValue::Int(int_setting.default)),
                "{}",
                int_setting.key
            );
        }
        for float_setting in [
            keys::TEXT_SPEED,
            keys::AUTO_SPEED,
            keys::MASTER_VOL,
            keys::SE_VOL,
            keys::VOICE_VOL,
            keys::RAG_THRESHOLD,
        ] {
            assert_eq!(
                defaults::default_for(float_setting.key),
                Some(DefaultValue::Float(float_setting.default)),
                "{}",
                float_setting.key
            );
        }
        for string_setting in [
            keys::API_KEY,
            keys::MODEL_NAME,
            keys::VERTEX_PROJECT,
            keys::VERTEX_LOCATION,
            keys::LOCAL_PROXY_HOST,
            keys::LOCAL_PROXY_PORT,
            keys::OPENAI_BASE_URL,
            keys::TTS_SPEAKER,
            keys::TTS_SERVER_URL,
            keys::STT_PROVIDER,
            keys::STT_SERVER_URL,
            keys::STT_LANGUAGE,
            keys::RAG_SERVER_URL,
        ] {
            assert_eq!(
                defaults::default_for(string_setting.key),
                Some(DefaultValue::Str(string_setting.default)),
                "{}",
                string_setting.key
            );
        }
        assert_eq!(defaults::default_for(keys::VERTEX_ENABLED.key), None);
    }

    #[test]
    fn getters_follow_js_parsing() {
        let store = MemoryStore::new();
        let s = AppSettings::new(&store);
        // Golden: V1 getInt("K", 7).
        for (raw, expected) in [
            (None, 7),
            (Some(""), 7),
            (Some("1.5"), 1),
            (Some("abc"), 7),
            (Some("  42abc"), 42),
            (Some("-0"), 0),
            (Some("1e3"), 1),
        ] {
            match raw {
                None => store.remove("K"),
                Some(r) => store.set("K", r),
            }
            assert_eq!(s.get_int("K", 7), expected, "{raw:?}");
        }
        store.set("K", "99999999999999999999");
        assert_eq!(s.get_int("K", 7), i64::MAX, "saturates (V1: 1e20)");
        store.set("K", &"9".repeat(400));
        assert_eq!(s.get_int("K", 7), 7, "Infinity is not finite");
        // Golden: V1 getFloat("K", 0.25).
        for (raw, expected) in [
            (None, 0.25),
            (Some(""), 0.25),
            (Some("0.8"), 0.8),
            (Some("Infinity"), 0.25),
            (Some("1e400"), 0.25),
            (Some(".5"), 0.5),
            (Some("abc"), 0.25),
            (Some("1e-7x"), 1e-7),
        ] {
            match raw {
                None => store.remove("K"),
                Some(r) => store.set("K", r),
            }
            assert_eq!(s.get_float("K", 0.25), expected, "{raw:?}");
        }
        store.remove("S");
        assert_eq!(s.get_string("S", "d"), "d");
        store.set("S", "");
        assert_eq!(
            s.get_string("S", "d"),
            "",
            "stored empty string is returned as-is"
        );
        // bool-int: only exactly 1 is true.
        for (raw, expected) in [
            ("1", true),
            ("1.9", true),
            ("2", false),
            ("0", false),
            ("x", true),
            ("", true),
        ] {
            store.set("B", raw);
            assert_eq!(s.get_bool("B", true), expected, "{raw:?}");
        }
        store.remove("B");
        assert!(!s.get_bool("B", false));
        assert!(s.get_bool("B", true));
    }

    #[test]
    fn setters_write_js_strings_and_always_emit() {
        let store = MemoryStore::new();
        let s = AppSettings::new(&store);
        let seen = Rc::new(RefCell::new(Vec::<String>::new()));
        let seen2 = seen.clone();
        let id = s
            .settings_changed()
            .connect(move |k| seen2.borrow_mut().push(k.to_owned()));
        s.set_int_f64("A", 1.9);
        assert_eq!(store.get("A").as_deref(), Some("1"));
        s.set_int_f64("A", -0.5);
        assert_eq!(store.get("A").as_deref(), Some("0"));
        s.set_float("A", 1.2000000000000002);
        assert_eq!(store.get("A").as_deref(), Some("1.2000000000000002"));
        s.set_float("A", 1e21);
        assert_eq!(store.get("A").as_deref(), Some("1e+21"));
        s.set_float("A", 0.1 + 0.2);
        assert_eq!(store.get("A").as_deref(), Some("0.30000000000000004"));
        s.set_float("A", 3.0);
        assert_eq!(store.get("A").as_deref(), Some("3"));
        s.set_int("I", -12);
        assert_eq!(store.get("I").as_deref(), Some("-12"));
        s.set_bool("B", true);
        assert_eq!(store.get("B").as_deref(), Some("1"));
        s.set_string("S", "x");
        s.set_string("S", "x"); // unchanged value still emits
        assert_eq!(seen.borrow().len(), 10);
        assert_eq!(s.revision(), 10);
        assert_eq!(seen.borrow().last().map(String::as_str), Some("S"));
        assert!(s.settings_changed().disconnect(id));
        s.set_string("S", "y");
        assert_eq!(seen.borrow().len(), 10);
        assert_eq!(s.revision(), 11);
        // Typed descriptors.
        keys::AUTO_SPEED.set(&s, 2.5);
        assert_eq!(store.get("Config_AutoSpeed").as_deref(), Some("2.5"));
        assert_eq!(keys::AUTO_SPEED.get(&s), 2.5);
        keys::EYE_TRACKING.set(&s, true);
        assert!(keys::EYE_TRACKING.get(&s));
    }

    #[test]
    fn works_with_trait_objects() {
        let store = MemoryStore::new();
        let dyn_store: &dyn KvStore = &store;
        let s: AppSettings<'_, dyn KvStore> = AppSettings::new(dyn_store);
        s.initialize_defaults();
        assert_eq!(s.provider(), 8);
        assert_eq!(
            normalize_ollama_host("  \u{3000}"),
            "http://localhost:11434"
        );
        assert_eq!(normalize_ollama_host(" http://h:1 "), "http://h:1");
    }

    fn payload_json<S: KvStore + ?Sized>(
        s: &AppSettings<'_, S>,
        messages: Vec<ChatMessage>,
    ) -> String {
        serde_json::to_string(&chat_request_payload(s, messages)).unwrap_or_default()
    }

    /// Golden: V1 `buildPayload` output (JSON.stringify) for several storage states.
    #[test]
    fn payload_matches_v1_build_payload() {
        let store = MemoryStore::new();
        let s = AppSettings::new(&store);
        assert_eq!(
            payload_json(&s, vec![ChatMessage::user("hi")]),
            r#"{"messages":[{"role":"user","content":"hi"}],"provider":8,"apiKey":"","model":"","webSearch":false,"vertexProject":"","vertexLocation":"us-central1","localProxyHost":"127.0.0.1","localProxyPort":"","useGCloudCLI":true,"ollamaHost":"http://localhost:11434","openaiCompatible":false,"openaiBaseUrl":""}"#
        );
        s.initialize_defaults();
        assert_eq!(
            payload_json(&s, vec![ChatMessage::system("S"), ChatMessage::user("hi")]),
            r#"{"messages":[{"role":"system","content":"S"},{"role":"user","content":"hi"}],"provider":8,"apiKey":"","model":"","webSearch":false,"vertexProject":"","vertexLocation":"us-central1","localProxyHost":"127.0.0.1","localProxyPort":"","useGCloudCLI":true,"ollamaHost":"http://localhost:11434","openaiCompatible":false,"openaiBaseUrl":""}"#
        );
        store.set("Config_ApiProvider", "2");
        store.set("Config_ApiKey", "GLOBAL");
        store.set("Config_ModelName", "GM");
        store.set("Config_ModelName_2", "claude-x");
        store.set("Config_WebSearch", "1");
        store.set("Config_GCloudCLI", "0");
        store.set("Config_OpenAICompatible", "2");
        store.set("Config_VertexLocation", "");
        assert_eq!(
            payload_json(&s, vec![]),
            r#"{"messages":[],"provider":2,"apiKey":"GLOBAL","model":"claude-x","webSearch":true,"vertexProject":"","vertexLocation":"","localProxyHost":"127.0.0.1","localProxyPort":"","useGCloudCLI":false,"ollamaHost":"http://localhost:11434","openaiCompatible":false,"openaiBaseUrl":""}"#
        );
        store.set("Config_ApiProvider", " 5.7xyz");
        store.set("Config_ApiKey_5", "K5");
        assert_eq!(
            payload_json(&s, vec![]),
            r#"{"messages":[],"provider":5,"apiKey":"K5","model":"GM","webSearch":true,"vertexProject":"","vertexLocation":"","localProxyHost":"127.0.0.1","localProxyPort":"","useGCloudCLI":false,"ollamaHost":"http://localhost:11434","openaiCompatible":false,"openaiBaseUrl":""}"#
        );
        store.set("Config_ApiProvider", "-1");
        assert_eq!(
            payload_json(&s, vec![]),
            r#"{"messages":[],"provider":-1,"apiKey":"GLOBAL","model":"GM","webSearch":true,"vertexProject":"","vertexLocation":"","localProxyHost":"127.0.0.1","localProxyPort":"","useGCloudCLI":false,"ollamaHost":"http://localhost:11434","openaiCompatible":false,"openaiBaseUrl":""}"#
        );
    }

    #[test]
    fn initial_language_selection_rules() {
        let store = MemoryStore::new();
        let s = AppSettings::new(&store);
        assert!(s.needs_initial_language_selection());
        assert!(keys::is_known_setting_key(keys::LANGUAGE_SELECTED.key));
        assert_eq!(defaults::default_for(keys::LANGUAGE_SELECTED.key), None);

        // Simulated reload before the user chooses a language keeps the picker open even after
        // initialize_defaults() writes Config_Language.
        store.set(keys::LANGUAGE_SELECTED.key, "0");
        s.initialize_defaults();
        assert!(s.needs_initial_language_selection());

        // Choosing a language persists both keys and closes the picker on subsequent launches.
        keys::LANGUAGE.set(&s, 7);
        keys::LANGUAGE_SELECTED.set(&s, true);
        assert!(!s.needs_initial_language_selection());
        assert_eq!(s.language(), 7);

        // Returning V1/V2 user store with only Config_Language present skips the first-launch picker.
        let returning = MemoryStore::new();
        returning.set(keys::LANGUAGE.key, "1");
        let rs = AppSettings::new(&returning);
        assert!(!rs.needs_initial_language_selection());
    }
}

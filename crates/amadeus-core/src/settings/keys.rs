//! Every localStorage key V1 uses, with its V1 type and default.
//!
//! The `Config_*` keys are read and written through [`AppSettings`]. Each typed descriptor
//! carries the raw key (`.key`) and the default V1 passes at its call sites (identical to
//! `DEFAULT_SETTINGS` for every key; the single exception is noted on [`API_PROVIDER`]).
//!
//! Type conventions (V1 `app-settings.ts`):
//! * [`BoolSetting`] — "bool-int": stored `"0"`/`"1"`, read as `getInt(key, d) === 1`, so any
//!   other stored number (e.g. `"2"`) reads as `false`.
//! * [`IntSetting`] — `getInt` (`parseInt(raw, 10)`) / `setInt` (`String(Math.trunc(v))`).
//! * [`FloatSetting`] — `getFloat` (`parseFloat`) / `setFloat` (`String(v)`, JS formatting).
//! * [`StringSetting`] — raw string; the default is used only when the key is absent.

use super::AppSettings;
use crate::storage::KvStore;

/// A "bool-int" setting: stored `"0"`/`"1"`, true iff the parsed integer is exactly 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoolSetting {
    /// localStorage key.
    pub key: &'static str,
    /// Default used when the key is missing or does not parse.
    pub default: bool,
}

/// An integer setting (`getInt` / `setInt`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntSetting {
    /// localStorage key.
    pub key: &'static str,
    /// Default used when the key is missing or does not parse.
    pub default: i64,
}

/// A float setting (`getFloat` / `setFloat`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatSetting {
    /// localStorage key.
    pub key: &'static str,
    /// Default used when the key is missing or does not parse to a finite number.
    pub default: f64,
}

/// A string setting (`getString` / `setString`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StringSetting {
    /// localStorage key.
    pub key: &'static str,
    /// Default used only when the key is absent (a stored `""` is returned as `""`).
    pub default: &'static str,
}

impl BoolSetting {
    const fn new(key: &'static str, default: bool) -> Self {
        Self { key, default }
    }
    /// `AppSettings.getInt(key, default ? 1 : 0) === 1`.
    pub fn get<S: KvStore + ?Sized>(&self, settings: &AppSettings<'_, S>) -> bool {
        settings.get_bool(self.key, self.default)
    }
    /// `AppSettings.setInt(key, value ? 1 : 0)` (emits `settingsChanged`).
    pub fn set<S: KvStore + ?Sized>(&self, settings: &AppSettings<'_, S>, value: bool) {
        settings.set_bool(self.key, value);
    }
}

impl IntSetting {
    const fn new(key: &'static str, default: i64) -> Self {
        Self { key, default }
    }
    /// `AppSettings.getInt(key, default)`.
    pub fn get<S: KvStore + ?Sized>(&self, settings: &AppSettings<'_, S>) -> i64 {
        settings.get_int(self.key, self.default)
    }
    /// `AppSettings.setInt(key, value)` (emits `settingsChanged`).
    pub fn set<S: KvStore + ?Sized>(&self, settings: &AppSettings<'_, S>, value: i64) {
        settings.set_int(self.key, value);
    }
}

impl FloatSetting {
    const fn new(key: &'static str, default: f64) -> Self {
        Self { key, default }
    }
    /// `AppSettings.getFloat(key, default)`.
    pub fn get<S: KvStore + ?Sized>(&self, settings: &AppSettings<'_, S>) -> f64 {
        settings.get_float(self.key, self.default)
    }
    /// `AppSettings.setFloat(key, value)` (emits `settingsChanged`).
    pub fn set<S: KvStore + ?Sized>(&self, settings: &AppSettings<'_, S>, value: f64) {
        settings.set_float(self.key, value);
    }
}

impl StringSetting {
    const fn new(key: &'static str, default: &'static str) -> Self {
        Self { key, default }
    }
    /// `AppSettings.getString(key, default)`.
    pub fn get<S: KvStore + ?Sized>(&self, settings: &AppSettings<'_, S>) -> String {
        settings.get_string(self.key, self.default)
    }
    /// `AppSettings.setString(key, value)` (emits `settingsChanged`).
    pub fn set<S: KvStore + ?Sized>(&self, settings: &AppSettings<'_, S>, value: &str) {
        settings.set_string(self.key, value);
    }
}

// --- System ------------------------------------------------------------------------------

/// `Config_SkipLogin` — 1 skips the ID/password fields on the login screen and shows a prompt + arrow button.
pub const SKIP_LOGIN: BoolSetting = BoolSetting::new("Config_SkipLogin", false);
/// `Config_SkipLoading` — 1 skips the boot/loading animation.
pub const SKIP_LOADING: BoolSetting = BoolSetting::new("Config_SkipLoading", false);
/// `Config_RightClickMenu` — 1 lets right-click toggle the menu panel.
pub const RIGHT_CLICK_MENU: BoolSetting = BoolSetting::new("Config_RightClickMenu", true);
/// `Config_EyeTracking` — Live2D eyes follow the pointer.
pub const EYE_TRACKING: BoolSetting = BoolSetting::new("Config_EyeTracking", false);
/// `Config_DesktopNotifications` — "通知を表示"; stored only (no V1 consumer).
pub const DESKTOP_NOTIFICATIONS: BoolSetting =
    BoolSetting::new("Config_DesktopNotifications", true);
/// `Config_LightweightMode` — "軽量化モード"; stored only (no V1 consumer).
pub const LIGHTWEIGHT_MODE: BoolSetting = BoolSetting::new("Config_LightweightMode", false);
/// `Config_Language` — display language index 0..=12 (0 ja, 1 en, …; see `i18n`).
pub const LANGUAGE: IntSetting = IntSetting::new("Config_Language", 0);
/// `Config_LanguageSelected` — NOT in `DEFAULT_SETTINGS`; `"1"` once the user has chosen
/// a display language in the first-launch picker.
pub const LANGUAGE_SELECTED: BoolSetting = BoolSetting::new("Config_LanguageSelected", false);
/// `Config_ChatLanguage` — 0 = same as the display language, n ≥ 1 = language n − 1.
pub const CHAT_LANGUAGE: IntSetting = IntSetting::new("Config_ChatLanguage", 0);

// --- Text ----------------------------------------------------------------------------------

/// `Config_TextSpeed` — typewriter speed multiplier (dialog slider 0.1–3, step 0.1).
pub const TEXT_SPEED: FloatSetting = FloatSetting::new("Config_TextSpeed", 1.0);
/// `Config_AutoMode` — auto-advance dialogue.
pub const AUTO_MODE: BoolSetting = BoolSetting::new("Config_AutoMode", false);
/// `Config_AutoSpeed` — auto-advance wait in seconds (slider 1–10, step 0.1).
pub const AUTO_SPEED: FloatSetting = FloatSetting::new("Config_AutoSpeed", 3.0);

// --- Sound ---------------------------------------------------------------------------------

/// `Config_MasterVol` — 0..1; stored only in V1.
pub const MASTER_VOL: FloatSetting = FloatSetting::new("Config_MasterVol", 1.0);
/// `Config_BGMVol` — 0..1; stored only in V1.
pub const BGM_VOL: FloatSetting = FloatSetting::new("Config_BGMVol", 0.8);
/// `Config_SEVol` — 0..1; stored only in V1.
pub const SE_VOL: FloatSetting = FloatSetting::new("Config_SEVol", 1.0);
/// `Config_VoiceVol` — 0..1; read once by the audio service at startup.
pub const VOICE_VOL: FloatSetting = FloatSetting::new("Config_VoiceVol", 1.0);

// --- Graphics ------------------------------------------------------------------------------

/// `Config_ScreenMode` — 0 = fullscreen, 1 = windowed (see `display`).
pub const SCREEN_MODE: IntSetting = IntSetting::new("Config_ScreenMode", 1);
/// `Config_Resolution` — 0 1920×1080, 1 1600×900, 2 1280×720 (see `display`).
pub const RESOLUTION: IntSetting = IntSetting::new("Config_Resolution", 0);

// --- LLM API -------------------------------------------------------------------------------

/// `Config_ApiProvider` — provider index 0..=8 (default 8 = FreeAPI).
///
/// V1 quirk: the streaming decision in `submitMessage` reads this key with default **0**
/// (see `providers::should_stream`); every other call site uses 8.
pub const API_PROVIDER: IntSetting = IntSetting::new("Config_ApiProvider", 8);
/// `Config_ApiKey` — legacy/global API key, fallback for an empty `Config_ApiKey_{n}`.
pub const API_KEY: StringSetting = StringSetting::new("Config_ApiKey", "");
/// `Config_ModelName` — legacy/global model name, fallback for an empty `Config_ModelName_{n}`.
pub const MODEL_NAME: StringSetting = StringSetting::new("Config_ModelName", "");
/// `Config_WebSearch` — `payload.webSearch`.
pub const WEB_SEARCH: BoolSetting = BoolSetting::new("Config_WebSearch", false);
/// `Config_VertexProject` — GCP project id.
pub const VERTEX_PROJECT: StringSetting = StringSetting::new("Config_VertexProject", "");
/// `Config_VertexLocation` — Vertex region.
pub const VERTEX_LOCATION: StringSetting =
    StringSetting::new("Config_VertexLocation", "us-central1");
/// `Config_GCloudCLI` — `payload.useGCloudCLI`; never written by the V1 UI.
pub const GCLOUD_CLI: BoolSetting = BoolSetting::new("Config_GCloudCLI", true);
/// `Config_LocalProxyHost` — `payload.localProxyHost`; never written by the V1 UI.
pub const LOCAL_PROXY_HOST: StringSetting =
    StringSetting::new("Config_LocalProxyHost", "127.0.0.1");
/// `Config_LocalProxyPort` — `payload.localProxyPort`; never written by the V1 UI.
pub const LOCAL_PROXY_PORT: StringSetting = StringSetting::new("Config_LocalProxyPort", "");
/// `Config_OllamaHost` — Ollama base URL (UI writes `value.trim() || default`).
pub const OLLAMA_HOST: StringSetting = StringSetting::new("Config_OllamaHost", DEFAULT_OLLAMA_HOST);
/// `Config_OpenAICompatible` — treat provider 0 as an OpenAI-compatible endpoint.
pub const OPENAI_COMPATIBLE: BoolSetting = BoolSetting::new("Config_OpenAICompatible", false);
/// `Config_OpenAIBaseUrl` — base URL of the compatible endpoint.
pub const OPENAI_BASE_URL: StringSetting = StringSetting::new("Config_OpenAIBaseUrl", "");
/// `Config_VertexEnabled` — hidden developer flag (NOT in `DEFAULT_SETTINGS`, never written):
/// 1 shows "Vertex AI" (index 4) in the provider pickers.
pub const VERTEX_ENABLED: BoolSetting = BoolSetting::new("Config_VertexEnabled", false);

/// Default of [`OLLAMA_HOST`]; also the fallback the settings dialogs write for an empty input.
pub const DEFAULT_OLLAMA_HOST: &str = "http://localhost:11434";
/// Default of [`VERTEX_LOCATION`]; the mobile sheet writes it for an empty input.
pub const DEFAULT_VERTEX_LOCATION: &str = "us-central1";

/// Prefix of the per-provider API key keys (`Config_ApiKey_{provider}`).
pub const API_KEY_PREFIX: &str = "Config_ApiKey_";
/// Prefix of the per-provider model keys (`Config_ModelName_{provider}`).
pub const MODEL_NAME_PREFIX: &str = "Config_ModelName_";
/// Provider indices whose per-provider keys `initializeDefaults` seeds (0..=8).
pub const PER_PROVIDER_DEFAULT_RANGE: std::ops::RangeInclusive<i64> = 0..=8;

/// `` `Config_ApiKey_${provider}` `` (decimal, e.g. `-1` → `"Config_ApiKey_-1"`).
pub fn api_key_for(provider: i64) -> String {
    format!("{API_KEY_PREFIX}{provider}")
}

/// `` `Config_ModelName_${provider}` ``.
pub fn model_name_for(provider: i64) -> String {
    format!("{MODEL_NAME_PREFIX}{provider}")
}

// --- Experimental TTS / STT / RAG (no V1 UI; round-tripped by the desktop config dialog) ----

/// Prefix the TTS service filters `settingsChanged` with.
pub const EXPERIMENTAL_TTS_PREFIX: &str = "Config_Experimental_TTS_";
/// Prefix the STT service filters `settingsChanged` with.
pub const EXPERIMENTAL_STT_PREFIX: &str = "Config_Experimental_STT_";
/// Prefix the vector database filters `settingsChanged` with.
pub const EXPERIMENTAL_RAG_PREFIX: &str = "Config_Experimental_RAG_";

/// `Config_Experimental_TTS_Enabled`.
pub const TTS_ENABLED: BoolSetting = BoolSetting::new("Config_Experimental_TTS_Enabled", false);
/// `Config_Experimental_TTS_Speaker` — `"christina-jp"` or `"christina"`.
pub const TTS_SPEAKER: StringSetting =
    StringSetting::new("Config_Experimental_TTS_Speaker", "christina-jp");
/// `Config_Experimental_TTS_ServerUrl`.
pub const TTS_SERVER_URL: StringSetting =
    StringSetting::new("Config_Experimental_TTS_ServerUrl", "http://localhost:5100");
/// `Config_Experimental_STT_Enabled`.
pub const STT_ENABLED: BoolSetting = BoolSetting::new("Config_Experimental_STT_Enabled", false);
/// `Config_Experimental_STT_Provider` — `"groq"`, `"openai"` or `"local"`.
pub const STT_PROVIDER: StringSetting =
    StringSetting::new("Config_Experimental_STT_Provider", "groq");
/// `Config_Experimental_STT_ServerUrl`.
pub const STT_SERVER_URL: StringSetting =
    StringSetting::new("Config_Experimental_STT_ServerUrl", "http://localhost:5101");
/// `Config_Experimental_STT_Language` — free text.
pub const STT_LANGUAGE: StringSetting =
    StringSetting::new("Config_Experimental_STT_Language", "ja");
/// `Config_Experimental_RAG_Enabled`.
pub const RAG_ENABLED: BoolSetting = BoolSetting::new("Config_Experimental_RAG_Enabled", false);
/// `Config_Experimental_RAG_TopK` — the vector DB setter clamps to 1..=20.
pub const RAG_TOP_K: IntSetting = IntSetting::new("Config_Experimental_RAG_TopK", 5);
/// `Config_Experimental_RAG_Threshold` — the vector DB setter clamps to 0..=1.
pub const RAG_THRESHOLD: FloatSetting = FloatSetting::new("Config_Experimental_RAG_Threshold", 0.7);
/// `Config_Experimental_RAG_ServerUrl`.
pub const RAG_SERVER_URL: StringSetting =
    StringSetting::new("Config_Experimental_RAG_ServerUrl", "http://localhost:5102");

// --- Chat panel ----------------------------------------------------------------------------

/// `Config_AudioInputEnabled` — NOT in `DEFAULT_SETTINGS` and never written by V1; 1 shows the
/// microphone button in the chat panel.
pub const AUDIO_INPUT_ENABLED: BoolSetting = BoolSetting::new("Config_AudioInputEnabled", false);

// --- Non-settings storage keys (formats owned by their modules) -----------------------------

/// `RealAmadeus.Memory` — long-term memory JSON (memory module).
pub const MEMORY_STORAGE_KEY: &str = "RealAmadeus.Memory";
/// `amadeus_achievements` — achievement state JSON (achievements module).
pub const ACHIEVEMENTS_STORAGE_KEY: &str = "amadeus_achievements";
/// `RealAmadeus.VectorStore` — client-side RAG store JSON (vector module).
pub const VECTOR_STORE_STORAGE_KEY: &str = "RealAmadeus.VectorStore";
/// `atchannel_posts` — belongs to a different app on the same origin (`app/atchannel`); RealAmadeus
/// must never read or write it.
pub const FOREIGN_ATCHANNEL_POSTS_KEY: &str = "atchannel_posts";

/// Kind of a setting (V1 getter/setter pair).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingKind {
    /// `getInt(..) === 1` / `setInt(b ? 1 : 0)`.
    BoolInt,
    /// `getInt` / `setInt`.
    Int,
    /// `getFloat` / `setFloat`.
    Float,
    /// `getString` / `setString`.
    String,
}

/// One row of [`ALL_SETTINGS`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SettingInfo {
    /// localStorage key.
    pub key: &'static str,
    /// Getter/setter family.
    pub kind: SettingKind,
    /// Whether V1 `DEFAULT_SETTINGS` contains (and `initializeDefaults` seeds) the key.
    pub in_default_settings: bool,
}

const fn info(key: &'static str, kind: SettingKind, in_default_settings: bool) -> SettingInfo {
    SettingInfo {
        key,
        kind,
        in_default_settings,
    }
}

/// Every fixed-name `Config_*` key V1/V2 references (43). The per-provider keys
/// `Config_ApiKey_{n}` / `Config_ModelName_{n}` (strings, seeded for n = 0..=8) come on top.
pub const ALL_SETTINGS: [SettingInfo; 43] = [
    info(SKIP_LOADING.key, SettingKind::BoolInt, true),
    info(SKIP_LOGIN.key, SettingKind::BoolInt, true),
    info(RIGHT_CLICK_MENU.key, SettingKind::BoolInt, true),
    info(EYE_TRACKING.key, SettingKind::BoolInt, true),
    info(DESKTOP_NOTIFICATIONS.key, SettingKind::BoolInt, true),
    info(LIGHTWEIGHT_MODE.key, SettingKind::BoolInt, true),
    info(TEXT_SPEED.key, SettingKind::Float, true),
    info(AUTO_MODE.key, SettingKind::BoolInt, true),
    info(AUTO_SPEED.key, SettingKind::Float, true),
    info(MASTER_VOL.key, SettingKind::Float, true),
    info(BGM_VOL.key, SettingKind::Float, true),
    info(SE_VOL.key, SettingKind::Float, true),
    info(VOICE_VOL.key, SettingKind::Float, true),
    info(SCREEN_MODE.key, SettingKind::Int, true),
    info(RESOLUTION.key, SettingKind::Int, true),
    info(API_PROVIDER.key, SettingKind::Int, true),
    info(API_KEY.key, SettingKind::String, true),
    info(MODEL_NAME.key, SettingKind::String, true),
    info(LANGUAGE.key, SettingKind::Int, true),
    info(CHAT_LANGUAGE.key, SettingKind::Int, true),
    info(WEB_SEARCH.key, SettingKind::BoolInt, true),
    info(VERTEX_PROJECT.key, SettingKind::String, true),
    info(VERTEX_LOCATION.key, SettingKind::String, true),
    info(GCLOUD_CLI.key, SettingKind::BoolInt, true),
    info(LOCAL_PROXY_HOST.key, SettingKind::String, true),
    info(LOCAL_PROXY_PORT.key, SettingKind::String, true),
    info(OLLAMA_HOST.key, SettingKind::String, true),
    info(OPENAI_COMPATIBLE.key, SettingKind::BoolInt, true),
    info(OPENAI_BASE_URL.key, SettingKind::String, true),
    info(TTS_ENABLED.key, SettingKind::BoolInt, true),
    info(TTS_SPEAKER.key, SettingKind::String, true),
    info(TTS_SERVER_URL.key, SettingKind::String, true),
    info(STT_ENABLED.key, SettingKind::BoolInt, true),
    info(STT_PROVIDER.key, SettingKind::String, true),
    info(STT_SERVER_URL.key, SettingKind::String, true),
    info(STT_LANGUAGE.key, SettingKind::String, true),
    info(RAG_ENABLED.key, SettingKind::BoolInt, true),
    info(RAG_TOP_K.key, SettingKind::Int, true),
    info(RAG_THRESHOLD.key, SettingKind::Float, true),
    info(RAG_SERVER_URL.key, SettingKind::String, true),
    info(AUDIO_INPUT_ENABLED.key, SettingKind::BoolInt, false),
    info(VERTEX_ENABLED.key, SettingKind::BoolInt, false),
    info(LANGUAGE_SELECTED.key, SettingKind::BoolInt, false),
];

/// True for a key of the form `Config_ApiKey_{int}` or `Config_ModelName_{int}`.
pub fn is_per_provider_key(key: &str) -> bool {
    let suffix = key
        .strip_prefix(API_KEY_PREFIX)
        .or_else(|| key.strip_prefix(MODEL_NAME_PREFIX));
    match suffix {
        Some(rest) => {
            let digits = rest.strip_prefix('-').unwrap_or(rest);
            !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
        }
        None => false,
    }
}

/// True for every `Config_*` key V1 knows ([`ALL_SETTINGS`] plus the per-provider keys).
pub fn is_known_setting_key(key: &str) -> bool {
    ALL_SETTINGS.iter().any(|s| s.key == key) || is_per_provider_key(key)
}

//! V1 `DEFAULT_SETTINGS` (`lib/amadeus/app-settings.ts`), verbatim and in insertion order.

use super::jsnum::js_number_to_string;
use super::keys;

/// A default value as written in V1's `DEFAULT_SETTINGS` object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DefaultValue {
    /// An integer literal (`0`, `1`, `8`, …).
    Int(i64),
    /// A float literal (`1.0`, `0.8`, …). JS has a single number type, so `1.0` is stored as `"1"`.
    Float(f64),
    /// A string literal.
    Str(&'static str),
}

impl DefaultValue {
    /// `String(value)` — the exact string `initializeDefaults` stores
    /// (`Float(1.0)` → `"1"`, `Float(0.8)` → `"0.8"`).
    pub fn to_storage_string(&self) -> String {
        match *self {
            DefaultValue::Int(i) => i.to_string(),
            DefaultValue::Float(f) => js_number_to_string(f),
            DefaultValue::Str(s) => s.to_owned(),
        }
    }
}

/// V1 `DEFAULT_SETTINGS` in insertion order (39 named keys, then `Config_ApiKey_i` /
/// `Config_ModelName_i` interleaved for i = 0..=8). `AppSettings::initialize_defaults` writes
/// `String(value)` for every key in this order that is not present yet.
pub const DEFAULT_SETTINGS: [(&str, DefaultValue); 58] = [
    (keys::SKIP_LOADING.key, DefaultValue::Int(0)),
    (keys::SKIP_LOGIN.key, DefaultValue::Int(0)),
    (keys::RIGHT_CLICK_MENU.key, DefaultValue::Int(1)),
    (keys::EYE_TRACKING.key, DefaultValue::Int(0)),
    (keys::DESKTOP_NOTIFICATIONS.key, DefaultValue::Int(1)),
    (keys::LIGHTWEIGHT_MODE.key, DefaultValue::Int(0)),
    (keys::TEXT_SPEED.key, DefaultValue::Float(1.0)),
    (keys::AUTO_MODE.key, DefaultValue::Int(0)),
    (keys::AUTO_SPEED.key, DefaultValue::Float(3.0)),
    (keys::MASTER_VOL.key, DefaultValue::Float(1.0)),
    (keys::BGM_VOL.key, DefaultValue::Float(0.8)),
    (keys::SE_VOL.key, DefaultValue::Float(1.0)),
    (keys::VOICE_VOL.key, DefaultValue::Float(1.0)),
    (keys::SCREEN_MODE.key, DefaultValue::Int(1)),
    (keys::RESOLUTION.key, DefaultValue::Int(0)),
    (keys::API_PROVIDER.key, DefaultValue::Int(8)),
    (keys::API_KEY.key, DefaultValue::Str("")),
    (keys::MODEL_NAME.key, DefaultValue::Str("")),
    (keys::LANGUAGE.key, DefaultValue::Int(0)),
    (keys::CHAT_LANGUAGE.key, DefaultValue::Int(0)),
    (keys::WEB_SEARCH.key, DefaultValue::Int(0)),
    (keys::VERTEX_PROJECT.key, DefaultValue::Str("")),
    (keys::VERTEX_LOCATION.key, DefaultValue::Str("us-central1")),
    (keys::GCLOUD_CLI.key, DefaultValue::Int(1)),
    (keys::LOCAL_PROXY_HOST.key, DefaultValue::Str("127.0.0.1")),
    (keys::LOCAL_PROXY_PORT.key, DefaultValue::Str("")),
    (
        keys::OLLAMA_HOST.key,
        DefaultValue::Str("http://localhost:11434"),
    ),
    (keys::OPENAI_COMPATIBLE.key, DefaultValue::Int(0)),
    (keys::OPENAI_BASE_URL.key, DefaultValue::Str("")),
    (keys::TTS_ENABLED.key, DefaultValue::Int(0)),
    (keys::TTS_SPEAKER.key, DefaultValue::Str("christina-jp")),
    (
        keys::TTS_SERVER_URL.key,
        DefaultValue::Str("http://localhost:5100"),
    ),
    (keys::STT_ENABLED.key, DefaultValue::Int(0)),
    (keys::STT_PROVIDER.key, DefaultValue::Str("groq")),
    (
        keys::STT_SERVER_URL.key,
        DefaultValue::Str("http://localhost:5101"),
    ),
    (keys::STT_LANGUAGE.key, DefaultValue::Str("ja")),
    (keys::RAG_ENABLED.key, DefaultValue::Int(0)),
    (keys::RAG_TOP_K.key, DefaultValue::Int(5)),
    (keys::RAG_THRESHOLD.key, DefaultValue::Float(0.7)),
    (
        keys::RAG_SERVER_URL.key,
        DefaultValue::Str("http://localhost:5102"),
    ),
    ("Config_ApiKey_0", DefaultValue::Str("")),
    ("Config_ModelName_0", DefaultValue::Str("")),
    ("Config_ApiKey_1", DefaultValue::Str("")),
    ("Config_ModelName_1", DefaultValue::Str("")),
    ("Config_ApiKey_2", DefaultValue::Str("")),
    ("Config_ModelName_2", DefaultValue::Str("")),
    ("Config_ApiKey_3", DefaultValue::Str("")),
    ("Config_ModelName_3", DefaultValue::Str("")),
    ("Config_ApiKey_4", DefaultValue::Str("")),
    ("Config_ModelName_4", DefaultValue::Str("")),
    ("Config_ApiKey_5", DefaultValue::Str("")),
    ("Config_ModelName_5", DefaultValue::Str("")),
    ("Config_ApiKey_6", DefaultValue::Str("")),
    ("Config_ModelName_6", DefaultValue::Str("")),
    ("Config_ApiKey_7", DefaultValue::Str("")),
    ("Config_ModelName_7", DefaultValue::Str("")),
    ("Config_ApiKey_8", DefaultValue::Str("")),
    ("Config_ModelName_8", DefaultValue::Str("")),
];

/// The `DEFAULT_SETTINGS` entry for `key`, if any.
pub fn default_for(key: &str) -> Option<DefaultValue> {
    DEFAULT_SETTINGS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
}

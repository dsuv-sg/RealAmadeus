//! LLM provider metadata of V1 (`RealAmadeusApp.tsx` 325–328 and 3810–3871, provider ids of
//! `app/amadeus/app/api/chat/shared.ts`) and the TTS speaker / STT provider name tables.

use crate::settings::{keys, AppSettings};
use crate::storage::KvStore;
use crate::types::LlmStatus;

/// Provider 0 — OpenAI (or an OpenAI-compatible endpoint with `Config_OpenAICompatible`).
pub const PROVIDER_OPENAI: i64 = 0;
/// Provider 1 — Google Gemini (AI Studio).
pub const PROVIDER_GEMINI: i64 = 1;
/// Provider 2 — Anthropic Claude.
pub const PROVIDER_CLAUDE: i64 = 2;
/// Provider 3 — Groq.
pub const PROVIDER_GROQ: i64 = 3;
/// Provider 4 — Vertex AI (hidden in pickers unless `Config_VertexEnabled == 1`).
pub const PROVIDER_VERTEX: i64 = 4;
/// Provider 5 — Ollama.
pub const PROVIDER_OLLAMA: i64 = 5;
/// Provider 6 — OpenRouter.
pub const PROVIDER_OPENROUTER: i64 = 6;
/// Provider 7 — local proxy (empty picker name; never selectable in the UI).
pub const PROVIDER_LOCAL: i64 = 7;
/// Provider 8 — FreeAPI (hosted, server-side credentials; the default provider).
pub const PROVIDER_FREEAPI: i64 = 8;
/// V1 `FREEAPI_PROVIDER_INDEX`.
pub const FREEAPI_PROVIDER_INDEX: i64 = PROVIDER_FREEAPI;

/// V1 `PROVIDER_NAMES` (picker labels, index = provider id; index 7 is empty and skipped).
pub const PROVIDER_NAMES: [&str; 9] = [
    "OpenAI",
    "Google Gemini",
    "Anthropic Claude",
    "Groq",
    "Vertex AI",
    "Ollama",
    "OpenRouter",
    "",
    "FreeAPI",
];

/// Providers the V1 client talks to via `/chat/stream` (`sendChatStreaming`); all others
/// (0, 2, 7, 8) use the non-streaming `/chat`.
pub const STREAMING_PROVIDERS: [i64; 5] = [
    PROVIDER_GEMINI,
    PROVIDER_GROQ,
    PROVIDER_VERTEX,
    PROVIDER_OLLAMA,
    PROVIDER_OPENROUTER,
];

/// V1 `TTS_SPEAKER_NAMES` (`Config_Experimental_TTS_Speaker` values).
pub const TTS_SPEAKER_NAMES: [&str; 2] = ["christina-jp", "christina"];
/// V1 `STT_PROVIDER_NAMES` (`Config_Experimental_STT_Provider` values).
pub const STT_PROVIDER_NAMES: [&str; 3] = ["groq", "openai", "local"];

/// Picker label of a provider (`PROVIDER_NAMES[i]`), `None` outside 0..=8.
pub fn provider_name(provider: i64) -> Option<&'static str> {
    usize::try_from(provider)
        .ok()
        .and_then(|i| PROVIDER_NAMES.get(i).copied())
}

/// V1 picker filter (mobile `<select>` and desktop `ConfigSelectRow`): skip empty labels
/// (index 7) and index 4 unless `Config_VertexEnabled == 1`.
///
/// Note: V1's desktop `ConfigSelectRow` applies the index-4 rule to EVERY dropdown that uses it
/// (QUIRK-1: "Español" as display language, "한국어" as chat language become unselectable).
pub fn is_picker_option_visible(index: usize, label: &str, vertex_enabled: bool) -> bool {
    !label.is_empty() && (index != 4 || vertex_enabled)
}

/// `(provider id, label)` pairs shown in the provider pickers, in order.
pub fn provider_picker_entries(vertex_enabled: bool) -> Vec<(i64, &'static str)> {
    PROVIDER_NAMES
        .iter()
        .enumerate()
        .filter(|(i, label)| is_picker_option_visible(*i, label, vertex_enabled))
        .map(|(i, label)| (i as i64, *label))
        .collect()
}

/// [`provider_picker_entries`] with `Config_VertexEnabled` read from the settings.
pub fn provider_picker_entries_from_settings<S: KvStore + ?Sized>(
    settings: &AppSettings<'_, S>,
) -> Vec<(i64, &'static str)> {
    provider_picker_entries(keys::VERTEX_ENABLED.get(settings))
}

/// V1 `getDefaultModelForProvider(i)` — the model the STATUS LINE shows when none is
/// configured. (These differ from the backend's real defaults; kept verbatim.)
pub fn default_model_for_provider(provider: i64) -> &'static str {
    match provider {
        0 => "gpt-4o",
        1 => "gemini-1.5-pro",
        2 => "claude-3-5-sonnet-20241022",
        3 => "llama-3.3-70b-versatile",
        4 => "gemini-1.5-pro",
        5 => "llama3.2",
        6 => "openai/gpt-4o",
        _ => "default",
    }
}

/// V1 `getProviderDisplayName(i, isCompatible, baseUrl)` (status line / marquee):
/// 0 → `"OpenAI"` or `"OpenAI互換(<baseUrl>)"` (Japanese in every language), 1 `Gemini`,
/// 2 `Claude`, 3 `Groq`, 4 `Vertex AI`, 5 `Ollama`, 6 `OpenRouter`, 8 `FreeAPI`, else `Unknown`.
pub fn provider_display_name(provider: i64, is_compatible: bool, base_url: &str) -> String {
    match provider {
        0 if is_compatible => format!("OpenAI互換({base_url})"),
        0 => "OpenAI".to_owned(),
        1 => "Gemini".to_owned(),
        2 => "Claude".to_owned(),
        3 => "Groq".to_owned(),
        4 => "Vertex AI".to_owned(),
        5 => "Ollama".to_owned(),
        6 => "OpenRouter".to_owned(),
        8 => "FreeAPI".to_owned(),
        _ => "Unknown".to_owned(),
    }
}

/// V1 `getLlmStatusFromSettings(latencyMs = -1)`: provider `getInt("Config_ApiProvider", 8)`;
/// model `Config_ModelName_{p} || Config_ModelName || getDefaultModelForProvider(p)`; display
/// name with `Config_OpenAICompatible === 1` and `Config_OpenAIBaseUrl`.
pub fn llm_status_from_settings<S: KvStore + ?Sized>(
    settings: &AppSettings<'_, S>,
    latency_ms: i64,
) -> LlmStatus {
    let provider = settings.provider();
    let mut model = settings.model_name_for_provider(provider);
    if model.is_empty() {
        model = default_model_for_provider(provider).to_owned();
    }
    let is_compatible = keys::OPENAI_COMPATIBLE.get(settings);
    let base_url = keys::OPENAI_BASE_URL.get(settings);
    LlmStatus {
        provider: provider_display_name(provider, is_compatible, &base_url),
        model,
        latency_ms,
    }
}

/// True for the providers the V1 client streams (`{1, 3, 4, 5, 6}`).
pub fn uses_streaming(provider: i64) -> bool {
    STREAMING_PROVIDERS.contains(&provider)
}

/// V1 `submitMessage` dispatch: reads `Config_ApiProvider` with default **0** (not 8; a V1
/// quirk that only matters when the key is missing) and returns whether to stream.
pub fn should_stream<S: KvStore + ?Sized>(settings: &AppSettings<'_, S>) -> bool {
    uses_streaming(settings.get_int(keys::API_PROVIDER.key, 0))
}

/// `TTS_SPEAKER_NAMES.indexOf(name)`, with -1 → 0 (config panel load).
pub fn tts_speaker_index(name: &str) -> usize {
    TTS_SPEAKER_NAMES
        .iter()
        .position(|n| *n == name)
        .unwrap_or(0)
}

/// `TTS_SPEAKER_NAMES[index] ?? "christina-jp"` (config panel apply).
pub fn tts_speaker_name(index: usize) -> &'static str {
    TTS_SPEAKER_NAMES
        .get(index)
        .copied()
        .unwrap_or(TTS_SPEAKER_NAMES[0])
}

/// `STT_PROVIDER_NAMES.indexOf(name)`, with -1 → 0.
pub fn stt_provider_index(name: &str) -> usize {
    STT_PROVIDER_NAMES
        .iter()
        .position(|n| *n == name)
        .unwrap_or(0)
}

/// `STT_PROVIDER_NAMES[index] ?? "groq"`.
pub fn stt_provider_name(index: usize) -> &'static str {
    STT_PROVIDER_NAMES
        .get(index)
        .copied()
        .unwrap_or(STT_PROVIDER_NAMES[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStore;

    #[test]
    fn tables_are_verbatim() {
        assert_eq!(PROVIDER_NAMES[7], "");
        assert_eq!(provider_name(8), Some("FreeAPI"));
        assert_eq!(provider_name(9), None);
        assert_eq!(provider_name(-1), None);
        let models: Vec<&str> = (-1..=9).map(default_model_for_provider).collect();
        assert_eq!(
            models,
            [
                "default",
                "gpt-4o",
                "gemini-1.5-pro",
                "claude-3-5-sonnet-20241022",
                "llama-3.3-70b-versatile",
                "gemini-1.5-pro",
                "llama3.2",
                "openai/gpt-4o",
                "default",
                "default",
                "default"
            ]
        );
        let names: Vec<String> = (-1..=9)
            .map(|p| provider_display_name(p, false, ""))
            .collect();
        assert_eq!(
            names,
            [
                "Unknown",
                "OpenAI",
                "Gemini",
                "Claude",
                "Groq",
                "Vertex AI",
                "Ollama",
                "OpenRouter",
                "Unknown",
                "FreeAPI",
                "Unknown"
            ]
        );
        assert_eq!(
            provider_display_name(0, true, "http://x/v1"),
            "OpenAI互換(http://x/v1)"
        );
        assert_eq!(provider_display_name(1, true, "http://x/v1"), "Gemini");
    }

    #[test]
    fn picker_filter() {
        assert_eq!(
            provider_picker_entries(false),
            [
                (0, "OpenAI"),
                (1, "Google Gemini"),
                (2, "Anthropic Claude"),
                (3, "Groq"),
                (5, "Ollama"),
                (6, "OpenRouter"),
                (8, "FreeAPI")
            ]
        );
        assert_eq!(provider_picker_entries(true).len(), 8);
        let store = MemoryStore::new();
        let s = AppSettings::new(&store);
        assert_eq!(provider_picker_entries_from_settings(&s).len(), 7);
        store.set("Config_VertexEnabled", "1");
        assert_eq!(provider_picker_entries_from_settings(&s).len(), 8);
    }

    /// Golden: V1 `getLlmStatusFromSettings()` for several storage states.
    #[test]
    fn llm_status_golden() {
        let store = MemoryStore::new();
        let s = AppSettings::new(&store);
        let status = |latency| {
            let st = llm_status_from_settings(&s, latency);
            (st.provider, st.model, st.latency_ms)
        };
        assert_eq!(status(-1), ("FreeAPI".into(), "default".into(), -1));
        s.initialize_defaults();
        assert_eq!(status(1234), ("FreeAPI".into(), "default".into(), 1234));
        store.set("Config_ApiProvider", "0");
        store.set("Config_OpenAICompatible", "1");
        store.set("Config_OpenAIBaseUrl", "http://x/v1");
        assert_eq!(
            status(-1),
            ("OpenAI互換(http://x/v1)".into(), "gpt-4o".into(), -1)
        );
        store.set("Config_ModelName", "legacy");
        assert_eq!(
            status(-1),
            ("OpenAI互換(http://x/v1)".into(), "legacy".into(), -1)
        );
        store.set("Config_ModelName_0", "m0");
        assert_eq!(
            status(-1),
            ("OpenAI互換(http://x/v1)".into(), "m0".into(), -1)
        );
        store.set("Config_ApiProvider", "7");
        store.set("Config_ModelName", "");
        assert_eq!(status(-1), ("Unknown".into(), "default".into(), -1));
        store.set("Config_ApiProvider", "abc");
        assert_eq!(status(-1), ("FreeAPI".into(), "default".into(), -1));
    }

    #[test]
    fn streaming_decision() {
        let streaming: Vec<i64> = (-1..=9).filter(|p| uses_streaming(*p)).collect();
        assert_eq!(streaming, [1, 3, 4, 5, 6]);
        let store = MemoryStore::new();
        let s = AppSettings::new(&store);
        assert!(
            !should_stream(&s),
            "missing key → default 0 (OpenAI) → non-streaming"
        );
        store.set("Config_ApiProvider", "1");
        assert!(should_stream(&s));
        store.set("Config_ApiProvider", "8");
        assert!(!should_stream(&s));
    }

    #[test]
    fn tts_stt_name_tables() {
        assert_eq!(tts_speaker_index("christina"), 1);
        assert_eq!(tts_speaker_index("nobody"), 0);
        assert_eq!(tts_speaker_name(1), "christina");
        assert_eq!(tts_speaker_name(5), "christina-jp");
        assert_eq!(stt_provider_index("local"), 2);
        assert_eq!(stt_provider_index("whisper"), 0);
        assert_eq!(stt_provider_name(1), "openai");
        assert_eq!(stt_provider_name(3), "groq");
    }
}

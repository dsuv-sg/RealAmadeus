//! Pure form logic of the two V1 settings UIs, so the exact read order, legacy fallbacks,
//! normalisations and write order (every write emits `settingsChanged`) are unit-tested here
//! and the Leptos components only hold the state:
//!
//! * [`MobileSettingsState`] — `MobileSettingsPanel` (`RealAmadeusApp.tsx` 582–648).
//! * [`ConfigFormState`] + [`ProviderBuffers`] — desktop `ConfigPanel` (3069–3322).

use std::collections::BTreeMap;

use super::jsnum::{js_parse_float, js_parse_int, js_to_fixed};
use super::keys::{self, api_key_for, model_name_for};
use super::{normalize_ollama_host, AppSettings};
use crate::providers::{
    stt_provider_index, stt_provider_name, tts_speaker_index, tts_speaker_name,
};
use crate::storage::KvStore;

/// State of the mobile settings sheet (V1 `readMobileSettingsState()` shape).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MobileSettingsState {
    /// `Config_Language`.
    pub language: i64,
    /// `Config_ApiProvider`.
    pub provider: i64,
    /// API key (or Vertex service-account JSON) of `provider`.
    pub api_credential: String,
    /// Model name of `provider`.
    pub model_name: String,
    /// `Config_WebSearch === 1`.
    pub web_search: bool,
    /// `Config_AutoMode === 1`.
    pub auto_mode: bool,
    /// `Config_VertexProject`.
    pub vertex_project: String,
    /// `Config_VertexLocation`.
    pub vertex_location: String,
    /// `Config_OllamaHost`.
    pub ollama_host: String,
    /// `Config_OpenAICompatible === 1`.
    pub openai_compatible: bool,
    /// `Config_OpenAIBaseUrl`.
    pub openai_base_url: String,
    /// `Config_SkipLogin === 1`.
    pub skip_login: bool,
}

impl MobileSettingsState {
    /// V1 `readMobileSettingsState()` (run once when the sheet mounts): per-provider key and
    /// model with the legacy `Config_ApiKey` / `Config_ModelName` fallback.
    pub fn read<S: KvStore + ?Sized>(settings: &AppSettings<'_, S>) -> Self {
        let provider = settings.provider();
        Self {
            language: settings.language(),
            provider,
            api_credential: settings.api_key_for_provider(provider),
            model_name: settings.model_name_for_provider(provider),
            web_search: keys::WEB_SEARCH.get(settings),
            auto_mode: keys::AUTO_MODE.get(settings),
            vertex_project: keys::VERTEX_PROJECT.get(settings),
            vertex_location: keys::VERTEX_LOCATION.get(settings),
            ollama_host: keys::OLLAMA_HOST.get(settings),
            openai_compatible: keys::OPENAI_COMPATIBLE.get(settings),
            openai_base_url: keys::OPENAI_BASE_URL.get(settings),
            skip_login: keys::SKIP_LOGIN.get(settings),
        }
    }

    /// V1 `handleProviderChange(p)`: loads the stored per-provider key/model of `p` (NO legacy
    /// fallback here, unlike [`read`](Self::read)); unsaved edits of the previous provider are
    /// discarded (V1 quirk).
    pub fn change_provider<S: KvStore + ?Sized>(
        &mut self,
        settings: &AppSettings<'_, S>,
        provider: i64,
    ) {
        let stored_key = settings.get_string(&api_key_for(provider), "");
        let api_credential = if !stored_key.is_empty() {
            stored_key
        } else if provider == self.provider {
            self.api_credential.clone()
        } else {
            String::new()
        };
        let stored_model = settings.get_string(&model_name_for(provider), "");
        let model_name = if !stored_model.is_empty() {
            stored_model
        } else if provider == self.provider {
            self.model_name.clone()
        } else {
            String::new()
        };
        self.provider = provider;
        self.api_credential = api_credential;
        self.model_name = model_name;
    }

    /// V1 `apply()` steps 1–14 in exact order (13 writes, each emitting, then `save()`).
    /// Afterwards the caller does what V1's props do: `onLanguageChange(language)` (the root
    /// writes `Config_Language` once more), `onSettingsSaved()` (refresh the LLM status) and
    /// `onClose()`.
    pub fn apply<S: KvStore + ?Sized>(&self, settings: &AppSettings<'_, S>) {
        keys::LANGUAGE.set(settings, self.language);
        keys::API_PROVIDER.set(settings, self.provider);
        keys::WEB_SEARCH.set(settings, self.web_search);
        keys::AUTO_MODE.set(settings, self.auto_mode);
        settings.set_string(&api_key_for(self.provider), &self.api_credential);
        settings.set_string(&model_name_for(self.provider), &self.model_name);
        keys::API_KEY.set(settings, &self.api_credential);
        keys::MODEL_NAME.set(settings, &self.model_name);
        keys::VERTEX_PROJECT.set(settings, &self.vertex_project);
        let location = if self.vertex_location.is_empty() {
            keys::DEFAULT_VERTEX_LOCATION
        } else {
            self.vertex_location.as_str()
        };
        keys::VERTEX_LOCATION.set(settings, location);
        keys::OLLAMA_HOST.set(settings, &normalize_ollama_host(&self.ollama_host));
        keys::OPENAI_COMPATIBLE.set(settings, self.openai_compatible);
        keys::OPENAI_BASE_URL.set(settings, &self.openai_base_url);
        keys::SKIP_LOGIN.set(settings, self.skip_login);
        settings.save();
    }
}

/// Per-provider unsaved edits of the desktop config panel (V1 `apiKeyBuffer` /
/// `modelNameBuffer`, `Record<number, string>`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderBuffers {
    /// Provider index → API key.
    pub api_keys: BTreeMap<i64, String>,
    /// Provider index → model name.
    pub model_names: BTreeMap<i64, String>,
}

/// Number of providers whose keys the desktop config panel loads and writes (0..=6; 7 and 8
/// are never touched there).
pub const CONFIG_PANEL_PROVIDER_SLOTS: i64 = 7;

/// State of the desktop config panel (V1 `ConfigPanel` `state`, same fields in snake_case).
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigFormState {
    /// `Config_Language`.
    pub language: i64,
    /// `Config_ChatLanguage`.
    pub chat_language: i64,
    /// `Config_SkipLoading`.
    pub skip_loading: bool,
    /// `Config_SkipLogin`.
    pub skip_login: bool,
    /// `Config_RightClickMenu`.
    pub right_click_menu: bool,
    /// `Config_EyeTracking`.
    pub eye_tracking: bool,
    /// `Config_DesktopNotifications`.
    pub show_notifications: bool,
    /// `Config_LightweightMode`.
    pub lightweight_mode: bool,
    /// `Config_TextSpeed`.
    pub text_speed: f64,
    /// `Config_AutoMode`.
    pub auto_mode: bool,
    /// `Config_AutoSpeed`.
    pub auto_speed: f64,
    /// `Config_MasterVol`.
    pub master_vol: f64,
    /// `Config_BGMVol`.
    pub bgm_vol: f64,
    /// `Config_SEVol`.
    pub se_vol: f64,
    /// `Config_VoiceVol`.
    pub voice_vol: f64,
    /// `Config_ScreenMode` (0 fullscreen, 1 windowed).
    pub screen_mode: i64,
    /// `Config_Resolution`.
    pub resolution: i64,
    /// `Config_ApiProvider`.
    pub provider: i64,
    /// `Config_OpenAICompatible`.
    pub openai_compatible: bool,
    /// `Config_OpenAIBaseUrl`.
    pub openai_base_url: String,
    /// API key input of `provider`.
    pub api_key: String,
    /// Model input of `provider`.
    pub model_name: String,
    /// `Config_WebSearch`.
    pub web_search: bool,
    /// `Config_VertexProject`.
    pub vertex_project: String,
    /// `Config_VertexLocation`.
    pub vertex_location: String,
    /// `Config_OllamaHost`.
    pub ollama_host: String,
    /// `Config_Experimental_TTS_Enabled`.
    pub tts_enabled: bool,
    /// Index into `TTS_SPEAKER_NAMES`.
    pub tts_speaker: usize,
    /// `Config_Experimental_TTS_ServerUrl`.
    pub tts_server_url: String,
    /// `Config_Experimental_STT_Enabled`.
    pub stt_enabled: bool,
    /// Index into `STT_PROVIDER_NAMES`.
    pub stt_provider: usize,
    /// `Config_Experimental_STT_ServerUrl`.
    pub stt_server_url: String,
    /// `Config_Experimental_STT_Language`.
    pub stt_language: String,
    /// `Config_Experimental_RAG_Enabled`.
    pub rag_enabled: bool,
    /// Text of the RAG top-k input (`String(getInt(..))`).
    pub rag_top_k: String,
    /// Text of the RAG threshold input (`getFloat(..).toFixed(2)`).
    pub rag_threshold: String,
    /// `Config_Experimental_RAG_ServerUrl`.
    pub rag_server_url: String,
}

impl Default for ConfigFormState {
    fn default() -> Self {
        Self::initial()
    }
}

/// What the caller must do after [`ConfigFormState::apply`] wrote the settings (V1 performs
/// these right after its writes, in this order: fullscreen request/exit, then the TTS / STT /
/// vector-database setters, then `AppSettings.save()` and `onLanguageChange(language)`).
/// The setters have no storage effect left: their only write in V1, the RAG clamp rewrite, is
/// already done by `apply`.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigApplyOutcome {
    /// Requested `Config_ScreenMode`: 0 → request fullscreen if not fullscreen, else exit it.
    pub screen_mode: i64,
    /// `onLanguageChange(language)`: the root writes `Config_Language` again (one more emit).
    pub language: i64,
    /// `TTSService.enabled = …` (a no-op in V1: the service already synced).
    pub tts_enabled: bool,
    /// `TTSService.speaker = …`.
    pub tts_speaker: &'static str,
    /// `TTSService.serverUrl = …`.
    pub tts_server_url: String,
    /// `STTService.enabled = …`.
    pub stt_enabled: bool,
    /// `STTService.provider = …`.
    pub stt_provider: &'static str,
    /// `STTService.serverUrl = …`.
    pub stt_server_url: String,
    /// `STTService.language = …`.
    pub stt_language: String,
    /// `VectorDatabase.enabled = …`.
    pub rag_enabled: bool,
    /// `VectorDatabase.topK = parseInt(ragTopK, 10) || 5` (the setter clamps to 1..=20).
    pub rag_top_k: f64,
    /// `VectorDatabase.threshold = parseFloat(ragThreshold) || 0.7` (the setter clamps to 0..=1).
    pub rag_threshold: f64,
    /// `VectorDatabase.serverUrl = …`.
    pub rag_server_url: String,
}

impl ConfigApplyOutcome {
    /// Net storage effect of V1's `VectorDatabase.topK = …; VectorDatabase.threshold = …`
    /// setters after Apply: they clamp (top-k to `max(1, min(20, trunc(v)))`, threshold to
    /// `max(0, min(1, v))`) and rewrite the key only when the clamped value differs from the
    /// value just synced from storage (`|Δ| >= f64::EPSILON` for the threshold).
    /// [`ConfigFormState::apply`] already calls this as its last step; calling it again is a
    /// no-op (idempotent).
    pub fn apply_vector_db_clamps<S: KvStore + ?Sized>(&self, settings: &AppSettings<'_, S>) {
        let top_k = js_max(1.0, js_min(20.0, self.rag_top_k.trunc()));
        let synced_top_k = keys::RAG_TOP_K.get(settings) as f64;
        if top_k != synced_top_k {
            settings.set_int_f64(keys::RAG_TOP_K.key, top_k);
        }
        let threshold = js_max(0.0, js_min(1.0, self.rag_threshold));
        let synced_threshold = keys::RAG_THRESHOLD.get(settings);
        // V1: `if (Math.abs(this._threshold - bounded) < Number.EPSILON) return;` (a NaN
        // difference is not "unchanged", so it writes).
        let unchanged = (synced_threshold - threshold).abs() < f64::EPSILON;
        if !unchanged {
            settings.set_float(keys::RAG_THRESHOLD.key, threshold);
        }
    }
}

/// `Math.max(a, b)` (NaN-propagating).
fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// `Math.min(a, b)` (NaN-propagating).
fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// `x || fallback` for a JS number (`NaN`, `0` and `-0` are falsy).
fn js_or_number(x: Option<f64>, fallback: f64) -> f64 {
    match x {
        Some(v) if v != 0.0 && !v.is_nan() => v,
        _ => fallback,
    }
}

impl ConfigFormState {
    /// The panel state before its first load (V1 `useState` initial value; never visible).
    pub fn initial() -> Self {
        Self {
            language: 0,
            chat_language: 0,
            skip_loading: false,
            skip_login: false,
            right_click_menu: true,
            eye_tracking: false,
            show_notifications: false,
            lightweight_mode: false,
            text_speed: 1.0,
            auto_mode: false,
            auto_speed: 3.0,
            master_vol: 1.0,
            bgm_vol: 0.8,
            se_vol: 1.0,
            voice_vol: 1.0,
            screen_mode: 0,
            resolution: 0,
            provider: 0,
            openai_compatible: false,
            openai_base_url: String::new(),
            api_key: String::new(),
            model_name: String::new(),
            web_search: false,
            vertex_project: String::new(),
            vertex_location: "us-central1".to_owned(),
            ollama_host: keys::DEFAULT_OLLAMA_HOST.to_owned(),
            tts_enabled: false,
            tts_speaker: 0,
            tts_server_url: "http://localhost:5100".to_owned(),
            stt_enabled: false,
            stt_provider: 0,
            stt_server_url: "http://localhost:5101".to_owned(),
            stt_language: "ja".to_owned(),
            rag_enabled: false,
            rag_top_k: "5".to_owned(),
            rag_threshold: "0.70".to_owned(),
            rag_server_url: "http://localhost:5102".to_owned(),
        }
    }

    /// V1 load-on-open: per-provider buffers for providers 0..=6 (legacy fallback only for the
    /// current provider), unknown TTS speaker / STT provider names → index 0, RAG threshold
    /// shown with `toFixed(2)`. (V1 also fires a TTS health check here; that is the caller's.)
    pub fn load<S: KvStore + ?Sized>(settings: &AppSettings<'_, S>) -> (Self, ProviderBuffers) {
        let provider = settings.provider();
        let mut buffers = ProviderBuffers::default();
        for i in 0..CONFIG_PANEL_PROVIDER_SLOTS {
            let mut api_key = settings.get_string(&api_key_for(i), "");
            if api_key.is_empty() && i == provider {
                api_key = keys::API_KEY.get(settings);
            }
            buffers.api_keys.insert(i, api_key);
            let mut model_name = settings.get_string(&model_name_for(i), "");
            if model_name.is_empty() && i == provider {
                model_name = keys::MODEL_NAME.get(settings);
            }
            buffers.model_names.insert(i, model_name);
        }
        let state = Self {
            language: keys::LANGUAGE.get(settings),
            chat_language: keys::CHAT_LANGUAGE.get(settings),
            skip_loading: keys::SKIP_LOADING.get(settings),
            skip_login: keys::SKIP_LOGIN.get(settings),
            right_click_menu: keys::RIGHT_CLICK_MENU.get(settings),
            eye_tracking: keys::EYE_TRACKING.get(settings),
            show_notifications: keys::DESKTOP_NOTIFICATIONS.get(settings),
            lightweight_mode: keys::LIGHTWEIGHT_MODE.get(settings),
            text_speed: keys::TEXT_SPEED.get(settings),
            auto_mode: keys::AUTO_MODE.get(settings),
            auto_speed: keys::AUTO_SPEED.get(settings),
            master_vol: keys::MASTER_VOL.get(settings),
            bgm_vol: keys::BGM_VOL.get(settings),
            se_vol: keys::SE_VOL.get(settings),
            voice_vol: keys::VOICE_VOL.get(settings),
            screen_mode: keys::SCREEN_MODE.get(settings),
            resolution: keys::RESOLUTION.get(settings),
            provider,
            api_key: buffers.api_keys.get(&provider).cloned().unwrap_or_default(),
            model_name: buffers
                .model_names
                .get(&provider)
                .cloned()
                .unwrap_or_default(),
            web_search: keys::WEB_SEARCH.get(settings),
            vertex_project: keys::VERTEX_PROJECT.get(settings),
            vertex_location: keys::VERTEX_LOCATION.get(settings),
            openai_compatible: keys::OPENAI_COMPATIBLE.get(settings),
            openai_base_url: keys::OPENAI_BASE_URL.get(settings),
            ollama_host: keys::OLLAMA_HOST.get(settings),
            tts_enabled: keys::TTS_ENABLED.get(settings),
            tts_speaker: tts_speaker_index(&keys::TTS_SPEAKER.get(settings)),
            tts_server_url: keys::TTS_SERVER_URL.get(settings),
            stt_enabled: keys::STT_ENABLED.get(settings),
            stt_provider: stt_provider_index(&keys::STT_PROVIDER.get(settings)),
            stt_server_url: keys::STT_SERVER_URL.get(settings),
            stt_language: keys::STT_LANGUAGE.get(settings),
            rag_enabled: keys::RAG_ENABLED.get(settings),
            rag_top_k: keys::RAG_TOP_K.get(settings).to_string(),
            rag_threshold: js_to_fixed(keys::RAG_THRESHOLD.get(settings), 2),
            rag_server_url: keys::RAG_SERVER_URL.get(settings),
        };
        (state, buffers)
    }

    /// V1 `handleProviderChange(newProvider)` with its stale-closure semantics: the buffered
    /// values of `new_provider` are read BEFORE the current edits are stored, so re-selecting
    /// the current provider shows (and later applies) the old buffered value (V1 QUIRK-19).
    pub fn change_provider(&mut self, buffers: &mut ProviderBuffers, new_provider: i64) {
        let buffered_key = buffers
            .api_keys
            .get(&new_provider)
            .cloned()
            .unwrap_or_default();
        let buffered_model = buffers
            .model_names
            .get(&new_provider)
            .cloned()
            .unwrap_or_default();
        buffers.api_keys.insert(self.provider, self.api_key.clone());
        buffers
            .model_names
            .insert(self.provider, self.model_name.clone());
        self.provider = new_provider;
        self.api_key = buffered_key;
        self.model_name = buffered_model;
    }

    /// V1 `handleApply()` storage writes in exact order (each emits `settingsChanged`):
    /// system/text/sound keys, screen mode, resolution, provider, languages, API options,
    /// `Config_ApiKey_i`/`Config_ModelName_i` interleaved for i = 0..=6, the legacy keys (the
    /// current provider's buffer entry, `""` for 7/8), TTS, STT and RAG (`parseInt(..) || 5`,
    /// `parseFloat(..) || 0.7`, unclamped), then the clamp rewrites of V1's `VectorDatabase`
    /// setters ([`ConfigApplyOutcome::apply_vector_db_clamps`], 0–2 more writes). `buffers`
    /// gets the current edits like V1's `setApiKeyBuffer(next)`. The returned outcome lists
    /// the remaining side effects.
    pub fn apply<S: KvStore + ?Sized>(
        &self,
        buffers: &mut ProviderBuffers,
        settings: &AppSettings<'_, S>,
    ) -> ConfigApplyOutcome {
        buffers.api_keys.insert(self.provider, self.api_key.clone());
        buffers
            .model_names
            .insert(self.provider, self.model_name.clone());

        keys::SKIP_LOADING.set(settings, self.skip_loading);
        keys::SKIP_LOGIN.set(settings, self.skip_login);
        keys::RIGHT_CLICK_MENU.set(settings, self.right_click_menu);
        keys::EYE_TRACKING.set(settings, self.eye_tracking);
        keys::DESKTOP_NOTIFICATIONS.set(settings, self.show_notifications);
        keys::LIGHTWEIGHT_MODE.set(settings, self.lightweight_mode);
        keys::TEXT_SPEED.set(settings, self.text_speed);
        keys::AUTO_MODE.set(settings, self.auto_mode);
        keys::AUTO_SPEED.set(settings, self.auto_speed);
        keys::MASTER_VOL.set(settings, self.master_vol);
        keys::BGM_VOL.set(settings, self.bgm_vol);
        keys::SE_VOL.set(settings, self.se_vol);
        keys::VOICE_VOL.set(settings, self.voice_vol);
        keys::SCREEN_MODE.set(settings, self.screen_mode);
        keys::RESOLUTION.set(settings, self.resolution);
        keys::API_PROVIDER.set(settings, self.provider);
        keys::LANGUAGE.set(settings, self.language);
        keys::CHAT_LANGUAGE.set(settings, self.chat_language);
        keys::WEB_SEARCH.set(settings, self.web_search);
        keys::OPENAI_COMPATIBLE.set(settings, self.openai_compatible);
        keys::OPENAI_BASE_URL.set(settings, &self.openai_base_url);
        keys::VERTEX_PROJECT.set(settings, &self.vertex_project);
        keys::VERTEX_LOCATION.set(settings, &self.vertex_location);
        keys::OLLAMA_HOST.set(settings, &normalize_ollama_host(&self.ollama_host));

        for i in 0..CONFIG_PANEL_PROVIDER_SLOTS {
            settings.set_string(
                &api_key_for(i),
                buffers.api_keys.get(&i).map(String::as_str).unwrap_or(""),
            );
            settings.set_string(
                &model_name_for(i),
                buffers
                    .model_names
                    .get(&i)
                    .map(String::as_str)
                    .unwrap_or(""),
            );
        }
        keys::API_KEY.set(
            settings,
            buffers
                .api_keys
                .get(&self.provider)
                .map(String::as_str)
                .unwrap_or(""),
        );
        keys::MODEL_NAME.set(
            settings,
            buffers
                .model_names
                .get(&self.provider)
                .map(String::as_str)
                .unwrap_or(""),
        );

        let tts_speaker = tts_speaker_name(self.tts_speaker);
        keys::TTS_ENABLED.set(settings, self.tts_enabled);
        keys::TTS_SPEAKER.set(settings, tts_speaker);
        keys::TTS_SERVER_URL.set(settings, &self.tts_server_url);

        let stt_provider = stt_provider_name(self.stt_provider);
        keys::STT_ENABLED.set(settings, self.stt_enabled);
        keys::STT_PROVIDER.set(settings, stt_provider);
        keys::STT_SERVER_URL.set(settings, &self.stt_server_url);
        keys::STT_LANGUAGE.set(settings, &self.stt_language);

        let rag_top_k = js_or_number(js_parse_int(&self.rag_top_k), 5.0);
        let rag_threshold = js_or_number(js_parse_float(&self.rag_threshold), 0.7);
        keys::RAG_ENABLED.set(settings, self.rag_enabled);
        settings.set_int_f64(keys::RAG_TOP_K.key, rag_top_k);
        keys::RAG_THRESHOLD.set(settings, rag_threshold);
        keys::RAG_SERVER_URL.set(settings, &self.rag_server_url);

        let outcome = ConfigApplyOutcome {
            screen_mode: self.screen_mode,
            language: self.language,
            tts_enabled: self.tts_enabled,
            tts_speaker,
            tts_server_url: self.tts_server_url.clone(),
            stt_enabled: self.stt_enabled,
            stt_provider,
            stt_server_url: self.stt_server_url.clone(),
            stt_language: self.stt_language.clone(),
            rag_enabled: self.rag_enabled,
            rag_top_k,
            rag_threshold,
            rag_server_url: self.rag_server_url.clone(),
        };
        // V1: the TTS / STT setters that follow are no-ops; `VectorDatabase.topK` and
        // `.threshold` rewrite their keys when clamping changes the value.
        outcome.apply_vector_db_clamps(settings);
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStore;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn recorder<S: KvStore + ?Sized>(settings: &AppSettings<'_, S>) -> Rc<RefCell<Vec<String>>> {
        let log = Rc::new(RefCell::new(Vec::new()));
        let sink = log.clone();
        settings
            .settings_changed()
            .connect(move |k| sink.borrow_mut().push(k.to_owned()));
        log
    }

    #[test]
    fn mobile_read_change_apply() {
        let store = MemoryStore::new();
        let s = AppSettings::new(&store);
        s.initialize_defaults();
        store.set("Config_ApiProvider", "1");
        store.set("Config_ApiKey", "LEGACY");
        store.set("Config_ModelName_1", "gem");
        store.set("Config_ApiKey_3", "GROQKEY");
        store.set("Config_OllamaHost", "");
        let mut state = MobileSettingsState::read(&s);
        assert_eq!(state.provider, 1);
        assert_eq!(state.api_credential, "LEGACY");
        assert_eq!(state.model_name, "gem");
        assert_eq!(state.ollama_host, "");
        state.change_provider(&s, 3);
        assert_eq!(
            (state.api_credential.as_str(), state.model_name.as_str()),
            ("GROQKEY", "")
        );
        state.change_provider(&s, 2);
        assert_eq!(
            (state.api_credential.as_str(), state.model_name.as_str()),
            ("", ""),
            "no legacy fallback"
        );
        state.api_credential = "typed".into();
        state.change_provider(&s, 2);
        assert_eq!(
            state.api_credential, "typed",
            "same provider keeps the edit"
        );
        state.vertex_location = String::new();
        state.ollama_host = "  ".into();
        let log = recorder(&s);
        state.apply(&s);
        assert_eq!(
            *log.borrow(),
            [
                "Config_Language",
                "Config_ApiProvider",
                "Config_WebSearch",
                "Config_AutoMode",
                "Config_ApiKey_2",
                "Config_ModelName_2",
                "Config_ApiKey",
                "Config_ModelName",
                "Config_VertexProject",
                "Config_VertexLocation",
                "Config_OllamaHost",
                "Config_OpenAICompatible",
                "Config_OpenAIBaseUrl",
                "Config_SkipLogin",
            ]
        );
        assert_eq!(store.get("Config_ApiKey").as_deref(), Some("typed"));
        assert_eq!(
            store.get("Config_VertexLocation").as_deref(),
            Some("us-central1")
        );
        assert_eq!(
            store.get("Config_OllamaHost").as_deref(),
            Some("http://localhost:11434")
        );
    }

    #[test]
    fn config_load_change_apply() {
        let store = MemoryStore::new();
        let s = AppSettings::new(&store);
        s.initialize_defaults();
        store.set("Config_ApiProvider", "2");
        store.set("Config_ApiKey", "LEGACY");
        store.set("Config_ApiKey_0", "K0");
        store.set("Config_Experimental_TTS_Speaker", "unknown");
        store.set("Config_Experimental_STT_Provider", "local");
        store.set("Config_Experimental_RAG_Threshold", "0.125");
        let (mut state, mut buffers) = ConfigFormState::load(&s);
        assert_eq!(
            state.api_key, "LEGACY",
            "legacy fallback for the current provider"
        );
        assert_eq!(buffers.api_keys.get(&0).map(String::as_str), Some("K0"));
        assert_eq!(
            buffers.api_keys.get(&1).map(String::as_str),
            Some(""),
            "no fallback for others"
        );
        assert_eq!(buffers.api_keys.len(), 7);
        assert_eq!((state.tts_speaker, state.stt_provider), (0, 2));
        assert_eq!(state.rag_threshold, "0.13");
        assert_eq!(state.rag_top_k, "5");
        assert!(state.show_notifications && state.right_click_menu);
        assert_eq!(state.screen_mode, 1);

        // QUIRK-19: re-selecting the current provider restores the old buffered value.
        state.api_key = "edited".into();
        state.change_provider(&mut buffers, 2);
        assert_eq!(state.api_key, "LEGACY");
        state.change_provider(&mut buffers, 0);
        assert_eq!(state.api_key, "K0");
        state.change_provider(&mut buffers, 8);
        assert_eq!(state.api_key, "");

        state.rag_top_k = "0".into();
        state.rag_threshold = "1.5".into();
        state.ollama_host = " http://o:1 ".into();
        state.tts_speaker = 1;
        state.stt_provider = 9;
        let log = recorder(&s);
        let outcome = state.apply(&mut buffers, &s);
        let log = log.borrow();
        assert_eq!(
            log.len(),
            52,
            "Apply: 51 set* calls + the threshold clamp rewrite (top-k 5 is in range)"
        );
        assert_eq!(log[51], "Config_Experimental_RAG_Threshold");
        assert_eq!(log[0], "Config_SkipLoading");
        assert_eq!(log[1], "Config_SkipLogin");
        assert_eq!(log[13], "Config_ScreenMode");
        assert_eq!(log[24], "Config_ApiKey_0");
        assert_eq!(log[25], "Config_ModelName_0");
        assert_eq!(log[37], "Config_ModelName_6");
        assert_eq!(log[38], "Config_ApiKey");
        assert_eq!(log[39], "Config_ModelName");
        assert_eq!(log[50], "Config_Experimental_RAG_ServerUrl");
        assert_eq!(
            store.get("Config_ApiKey").as_deref(),
            Some(""),
            "provider 8 clears the legacy key"
        );
        assert_eq!(store.get("Config_ApiKey_0").as_deref(), Some("K0"));
        assert_eq!(store.get("Config_ApiKey_2").as_deref(), Some("LEGACY"));
        assert_eq!(
            store.get("Config_Experimental_TTS_Speaker").as_deref(),
            Some("christina")
        );
        assert_eq!(
            store.get("Config_Experimental_STT_Provider").as_deref(),
            Some("groq")
        );
        assert_eq!(
            store.get("Config_OllamaHost").as_deref(),
            Some("http://o:1")
        );
        assert_eq!((outcome.rag_top_k, outcome.rag_threshold), (5.0, 1.5));
        assert_eq!(
            store.get("Config_Experimental_RAG_TopK").as_deref(),
            Some("5")
        );
        assert_eq!(
            store.get("Config_Experimental_RAG_Threshold").as_deref(),
            Some("1"),
            "1.5 is clamped by apply()"
        );
        drop(log);
        let rev = s.revision();
        outcome.apply_vector_db_clamps(&s);
        assert_eq!(s.revision(), rev, "idempotent");

        // Top-k above the range: written unclamped first, then rewritten (two writes, V1).
        state.rag_top_k = "25".into();
        state.rag_threshold = "-0.2".into();
        let log = recorder(&s);
        state.apply(&mut buffers, &s);
        let log = log.borrow();
        assert_eq!(log.len(), 53);
        assert_eq!(log[48], "Config_Experimental_RAG_TopK");
        assert_eq!(log[49], "Config_Experimental_RAG_Threshold");
        assert_eq!(log[51], "Config_Experimental_RAG_TopK");
        assert_eq!(log[52], "Config_Experimental_RAG_Threshold");
        assert_eq!(
            store.get("Config_Experimental_RAG_TopK").as_deref(),
            Some("20")
        );
        assert_eq!(
            store.get("Config_Experimental_RAG_Threshold").as_deref(),
            Some("0")
        );
    }

    #[test]
    fn config_initial_state_matches_v1() {
        let initial = ConfigFormState::initial();
        assert_eq!(initial.provider, 0);
        assert_eq!(initial.screen_mode, 0);
        assert!(!initial.show_notifications);
        assert_eq!(initial.rag_threshold, "0.70");
        assert_eq!(ConfigFormState::default(), initial);
    }
}

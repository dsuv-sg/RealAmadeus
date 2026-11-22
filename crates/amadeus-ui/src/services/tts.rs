//! Port of V1 `lib/amadeus/services/tts-service.ts`: synthesis through the backend proxy with an
//! in-memory blob-URL cache, and the health / runtime-info probe.
//!
//! V1 facts kept: the cache key is the 32-bit `simpleHash(text|speaker|serverUrl)` and is never
//! evicted (Q-T1); the checks and a cache hit emit **synchronously inside** `synthesize` (the
//! chat panel's queue relies on that re-entrancy); the setters return early on an unchanged
//! value, so after a CONFIG "Apply" (which writes the keys first) they are no-ops (Q-T2).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use amadeus_core::jscompat::js_trim;
use amadeus_core::protocol::TtsSynthesizeBody;
use amadeus_core::settings::keys;
use amadeus_core::types::Emitter;

use super::logic::{tts_cache_key, TtsRuntimeInfo};
use super::settings;
use crate::transport::{js_error_message, transport, TtsReply};

/// V1 `"TTS is not enabled"`.
pub const TTS_NOT_ENABLED: &str = "TTS is not enabled";
/// V1 `"Empty text provided"`.
pub const EMPTY_TEXT_PROVIDED: &str = "Empty text provided";
/// V1 `"Received empty audio data"`.
pub const RECEIVED_EMPTY_AUDIO: &str = "Received empty audio data";
/// V1 `"Synthesis failed"` (non-`Error` exception).
pub const SYNTHESIS_FAILED: &str = "Synthesis failed";

/// V1 `TTSService`.
pub struct TtsService {
    /// `synthesisComplete(objectUrl)`.
    pub synthesis_complete: Emitter<str>,
    /// `synthesisError(message)`.
    pub synthesis_error: Emitter<str>,
    /// `enabledChanged` (only from [`set_enabled`](Self::set_enabled)).
    pub enabled_changed: Emitter<()>,
    /// `speakerChanged`.
    pub speaker_changed: Emitter<()>,
    /// `serverUrlChanged`.
    pub server_url_changed: Emitter<()>,
    /// `serverAvailableChanged`.
    pub server_available_changed: Emitter<()>,
    /// `runtimeInfoChanged`.
    pub runtime_info_changed: Emitter<()>,
    cache: RefCell<HashMap<String, String>>,
    server_available: Cell<bool>,
    runtime: RefCell<TtsRuntimeInfo>,
    weak_self: Weak<TtsService>,
}

impl TtsService {
    /// V1 constructor: health check right away when TTS is enabled.
    pub(crate) fn new() -> Rc<Self> {
        let service = Rc::new_cyclic(|weak| Self {
            synthesis_complete: Emitter::new(),
            synthesis_error: Emitter::new(),
            enabled_changed: Emitter::new(),
            speaker_changed: Emitter::new(),
            server_url_changed: Emitter::new(),
            server_available_changed: Emitter::new(),
            runtime_info_changed: Emitter::new(),
            cache: RefCell::new(HashMap::new()),
            server_available: Cell::new(false),
            runtime: RefCell::new(TtsRuntimeInfo::initial()),
            weak_self: weak.clone(),
        });
        if service.enabled() {
            service.check_server_health();
        }
        service
    }

    /// V1 `enabled` (`Config_Experimental_TTS_Enabled === 1`).
    pub fn enabled(&self) -> bool {
        keys::TTS_ENABLED.get(settings())
    }

    /// V1 `set enabled(v)`: no-op when unchanged; persists, emits `enabled_changed`, and checks
    /// the server health when enabling.
    pub fn set_enabled(&self, value: bool) {
        if self.enabled() == value {
            return;
        }
        keys::TTS_ENABLED.set(settings(), value);
        self.enabled_changed.emit(&());
        if value {
            self.check_server_health();
        }
    }

    /// V1 `speaker` (`Config_Experimental_TTS_Speaker`, default `christina-jp`).
    pub fn speaker(&self) -> String {
        keys::TTS_SPEAKER.get(settings())
    }

    /// V1 `set speaker(v)` (no health check).
    pub fn set_speaker(&self, value: &str) {
        if self.speaker() == value {
            return;
        }
        keys::TTS_SPEAKER.set(settings(), value);
        self.speaker_changed.emit(&());
    }

    /// V1 `serverUrl` (`Config_Experimental_TTS_ServerUrl`, default `http://localhost:5100`).
    pub fn server_url(&self) -> String {
        keys::TTS_SERVER_URL.get(settings())
    }

    /// V1 `set serverUrl(v)`: persists, emits, then checks health (even when disabled).
    pub fn set_server_url(&self, value: &str) {
        if self.server_url() == value {
            return;
        }
        keys::TTS_SERVER_URL.set(settings(), value);
        self.server_url_changed.emit(&());
        self.check_server_health();
    }

    /// V1 `serverAvailable`.
    pub fn server_available(&self) -> bool {
        self.server_available.get()
    }

    /// V1 `usingGpu`.
    pub fn using_gpu(&self) -> bool {
        self.runtime.borrow().using_gpu
    }

    /// V1 `runtimeDevice`.
    pub fn runtime_device(&self) -> String {
        self.runtime.borrow().device.clone()
    }

    /// V1 `runtimeDetails`.
    pub fn runtime_details(&self) -> String {
        self.runtime.borrow().details.clone()
    }

    /// All three runtime fields.
    pub fn runtime_info(&self) -> TtsRuntimeInfo {
        self.runtime.borrow().clone()
    }

    /// V1 `clearLocalCache()` (unused by V1): revokes every cached object URL.
    pub fn clear_local_cache(&self) {
        let urls: Vec<String> = self
            .cache
            .borrow_mut()
            .drain()
            .map(|(_, url)| url)
            .collect();
        for url in urls {
            let _ = web_sys::Url::revoke_object_url(&url);
        }
    }

    /// V1 `synthesize(text)`. Disabled / empty text / cache hit are reported synchronously,
    /// before this returns; otherwise the request runs in the background and ends with exactly
    /// one `synthesis_complete(objectUrl)` or `synthesis_error(message)`.
    pub fn synthesize(&self, text: &str) {
        if !self.enabled() {
            self.synthesis_error.emit(TTS_NOT_ENABLED);
            return;
        }
        let normalized = js_trim(text);
        if normalized.is_empty() {
            self.synthesis_error.emit(EMPTY_TEXT_PROVIDED);
            return;
        }
        let speaker = self.speaker();
        let server_url = self.server_url();
        let cache_key = tts_cache_key(normalized, &speaker, &server_url);
        let cached = self.cache.borrow().get(&cache_key).cloned();
        if let Some(url) = cached.filter(|url| !url.is_empty()) {
            self.synthesis_complete.emit(&url);
            return;
        }

        let body = TtsSynthesizeBody {
            server_url,
            speaker,
            text: normalized.to_owned(),
            use_cache: true,
        };
        let weak = self.weak_self.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let result = transport().tts_synthesize(&body).await;
            let Some(this) = weak.upgrade() else { return };
            match result {
                Err(error) => this
                    .synthesis_error
                    .emit(&error.message_or(SYNTHESIS_FAILED)),
                Ok(TtsReply::Failed(reply)) => {
                    let message = reply.error_or(|| format!("Synthesis failed ({})", reply.status));
                    this.synthesis_error.emit(&message);
                }
                Ok(TtsReply::Audio(blob)) => {
                    if blob.size() == 0.0 {
                        this.synthesis_error.emit(RECEIVED_EMPTY_AUDIO);
                        return;
                    }
                    match web_sys::Url::create_object_url_with_blob(&blob) {
                        Ok(url) => {
                            this.cache.borrow_mut().insert(cache_key, url.clone());
                            this.synthesis_complete.emit(&url);
                        }
                        Err(error) => {
                            let message = js_error_message(&error)
                                .unwrap_or_else(|| SYNTHESIS_FAILED.to_owned());
                            this.synthesis_error.emit(&message);
                        }
                    }
                }
            }
        });
    }

    /// V1 `checkServerHealth()` (background): updates `server_available` (emitting on change)
    /// and the runtime info (emitting when any field changed).
    pub fn check_server_health(&self) {
        let server_url = self.server_url();
        let weak = self.weak_self.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let result = transport().tts_health(&server_url).await;
            let Some(this) = weak.upgrade() else { return };
            let reply = match result {
                Ok(reply) => reply,
                Err(_) => {
                    this.set_server_available(false);
                    this.update_runtime_info(TtsRuntimeInfo::unavailable());
                    return;
                }
            };
            let available = reply.available();
            this.set_server_available(available);
            match reply.body.as_ref() {
                Some(health) if available => {
                    this.update_runtime_info(TtsRuntimeInfo::from_health(health))
                }
                _ => this.update_runtime_info(TtsRuntimeInfo::unavailable()),
            }
        });
    }

    fn set_server_available(&self, available: bool) {
        if self.server_available.get() != available {
            self.server_available.set(available);
            self.server_available_changed.emit(&());
        }
    }

    /// V1 `updateRuntimeInfo`.
    fn update_runtime_info(&self, info: TtsRuntimeInfo) {
        let changed = *self.runtime.borrow() != info;
        *self.runtime.borrow_mut() = info;
        if changed {
            self.runtime_info_changed.emit(&());
        }
    }
}

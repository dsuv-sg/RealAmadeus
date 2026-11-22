//! Browser services (audio, TTS, STT, AI, RAG client, FreeAPI quota) as thread-local
//! singletons. OWNED BY: ui-services.
//!
//! Ports of V1 `lib/amadeus/services/*.ts` with the same signals ([`Emitter`]s with V1
//! `Signal` semantics: synchronous, snapshot, re-entrant), the same localStorage keys
//! (through the shared [`crate::hooks::app_settings`]) and the same error strings. Backend
//! calls go through [`crate::transport`] (HTTP on the web, Tauri IPC on the desktop).
//!
//! | V1 singleton | V2 accessor | notes |
//! |---|---|---|
//! | `AudioService` | [`audio()`] | `HTMLAudioElement` playback + (unused) synthetic lip sync |
//! | `TTSService` | [`tts()`] | blob-URL cache, health/runtime info |
//! | `STTService` | [`stt()`] | browser SpeechRecognition first, else MediaRecorder upload |
//! | `AIService` | [`ai()`] | `send_chat` (not abortable) / `send_chat_streaming` |
//! | `VectorDatabase` (HTTP part) | [`rag()`] | embedding client, health, store persistence |
//!
//! Singletons are created on first access. V1 built them when the app module was evaluated, so
//! the `/app` root creates them up front through [`crate::app::managers::init`], in V1's order.
//! That keeps the constructor effects at page load: `AudioService` reads `Config_VoiceVol` once
//! (a later change applies only after a reload, QUIRK-14), and STT, TTS and RAG health-check
//! when enabled. Other settings are read fresh from storage on every access, which is what
//! V1's `settingsChanged` re-sync achieves.
//!
//! Leptos `Callback`s need `Send + Sync` closures: never capture these `Rc`s in a `Callback`;
//! call the accessor inside it instead. Subscribe from components with [`connect_scoped`].

pub mod ai;
pub mod audio;
pub mod logic;
pub mod quota;
pub mod rag;
pub mod stt;
pub mod tts;

use std::cell::Cell;
use std::rc::Rc;

use amadeus_core::types::{ConnectionId, Emitter};

pub use ai::AiService;
pub use audio::AudioService;
pub use logic::{FreeApiQuota, TtsRuntimeInfo};
pub use rag::RagClient;
pub use stt::SttService;
pub use tts::TtsService;

thread_local! {
    static INITIALIZED: Cell<bool> = const { Cell::new(false) };
    static AUDIO: Rc<AudioService> = AudioService::new();
    static TTS: Rc<TtsService> = TtsService::new();
    static STT: Rc<SttService> = SttService::new();
    static AI: Rc<AiService> = AiService::new();
    static RAG: Rc<RagClient> = RagClient::new();
}

/// Initialize services once (idempotent). Called by [`crate::App`] on every route: it only
/// selects the transport. The service singletons are created by their accessors, on the `/app`
/// route up front by [`crate::app::managers::init`].
pub fn init() {
    if INITIALIZED.with(|done| done.replace(true)) {
        return;
    }
    crate::transport::init();
}

/// V1 `AudioService`.
pub fn audio() -> Rc<AudioService> {
    AUDIO.with(Rc::clone)
}

/// V1 `TTSService`.
pub fn tts() -> Rc<TtsService> {
    TTS.with(Rc::clone)
}

/// V1 `STTService`.
pub fn stt() -> Rc<SttService> {
    STT.with(Rc::clone)
}

/// V1 `AIService`.
pub fn ai() -> Rc<AiService> {
    AI.with(Rc::clone)
}

/// HTTP half of V1 `VectorDatabase` (health, embeddings, persistence).
pub fn rag() -> Rc<RagClient> {
    RAG.with(Rc::clone)
}

/// The shared V1 `AppSettings` (ui-foundation's page-wide instance).
pub(crate) fn settings() -> &'static crate::hooks::Settings {
    crate::hooks::app_settings()
}

/// Connects `handler` to an emitter of a service singleton for the lifetime of the current
/// reactive owner (disconnected in `on_cleanup`, like V1's effect cleanups). The accessor and
/// selector are `Copy + Send + Sync` (fn items / non-capturing closures), so the cleanup is a
/// valid Leptos cleanup; `handler` itself may capture anything (`Rc`, signals, …).
///
/// ```ignore
/// services::connect_scoped(services::tts, |s| &s.synthesis_complete, move |url: &str| {
///     audio_queue.update(|q| q.push(url.to_owned()));
/// });
/// ```
pub fn connect_scoped<S, T, F, E, H>(service: F, emitter: E, handler: H) -> ConnectionId
where
    S: 'static,
    T: ?Sized + 'static,
    F: Fn() -> Rc<S> + Copy + Send + Sync + 'static,
    E: for<'a> Fn(&'a S) -> &'a Emitter<T> + Copy + Send + Sync + 'static,
    H: Fn(&T) + 'static,
{
    let id = emitter(&service()).connect(handler);
    leptos::prelude::on_cleanup(move || {
        emitter(&service()).disconnect(id);
    });
    id
}

/// Compile-time check that [`connect_scoped`] infers its types from the intended call shape.
#[allow(dead_code)]
fn connect_scoped_type_check() {
    connect_scoped(tts, |s| &s.synthesis_complete, |_url: &str| {});
    connect_scoped(audio, |s| &s.playback_finished, |_: &()| {});
    connect_scoped(stt, |s| &s.recording_progress, |_level: &f64| {});
}

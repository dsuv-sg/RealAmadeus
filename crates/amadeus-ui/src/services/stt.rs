//! Port of V1 `lib/amadeus/services/stt-service.ts`.
//!
//! Decision tree of `start_recording` (V1 §4.5):
//! 1. disabled → `transcription_error("STT is not enabled")` (synchronously);
//! 2. already recording → nothing;
//! 3. the browser has `SpeechRecognition` / `webkitSpeechRecognition` → browser recognition
//!    (provider and server URL ignored, nothing uploaded; level meter fixed at 0.5; the browser
//!    ends the session on silence);
//! 4. no `navigator.mediaDevices.getUserMedia` → `"Microphone is not supported in this browser"`;
//! 5. otherwise `getUserMedia({audio:true})` + `MediaRecorder.start(250)` with a level meter
//!    (mean byte-frequency / 255 per animation frame); on stop the recording is uploaded to
//!    `stt/transcribe` unless it is smaller than 1024 bytes.
//!
//! Both paths stop after 30 s at the latest. There is no VAD.
//!
//! Deliberate deviations (spec-lib-media Q-S6 and Open question 4): the microphone is released
//! when `MediaRecorder` cannot be created or started (V1 left it open); a second start while the
//! permission prompt is pending is ignored (V1 opened two streams); every recording keeps its
//! own recorder/stream/meter, so a late `onstop`/`onend` of an older session cannot tear down a
//! newer one; on the desktop (Tauri) step 3 is skipped — WebView2's recognizer has no service
//! key and WKWebView needs a usage description, so desktop STT always uses the backend.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use amadeus_core::jscompat::js_trim;
use amadeus_core::protocol::SttTranscribeFields;
use amadeus_core::settings::keys;
use amadeus_core::types::Emitter;
use gloo_timers::callback::Timeout;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AnalyserNode, AudioContext, Blob, BlobEvent, BlobPropertyBag, MediaDevices, MediaRecorder,
    MediaStream, MediaStreamConstraints, RecordingState,
};

use super::logic::{
    recording_level, resolve_speech_language, ANALYSER_FFT_SIZE, BROWSER_RECOGNITION_LEVEL,
    FALLBACK_RECORDING_TYPE, MAX_RECORDING_MS, MIN_RECORDING_BYTES, RECORDER_TIMESLICE_MS,
};
use super::settings;
use crate::transport::{js_error_message, transport};

/// V1 `"STT is not enabled"`.
pub const STT_NOT_ENABLED: &str = "STT is not enabled";
/// V1 `"Microphone is not supported in this browser"`.
pub const MICROPHONE_NOT_SUPPORTED: &str = "Microphone is not supported in this browser";
/// V1 `"Recording too short"`.
pub const RECORDING_TOO_SHORT: &str = "Recording too short";
/// V1 `"Failed to start recording"`.
pub const FAILED_TO_START_RECORDING: &str = "Failed to start recording";
/// V1 `"Failed to stop speech recognition"`.
pub const FAILED_TO_STOP_RECOGNITION: &str = "Failed to stop speech recognition";
/// V1 `"No speech detected"`.
pub const NO_SPEECH_DETECTED: &str = "No speech detected";
/// V1 `"Transcription failed"`.
pub const TRANSCRIPTION_FAILED: &str = "Transcription failed";

/// How many finished-but-not-ended recognizer sessions are kept alive at most.
const MAX_PENDING_RECOGNITIONS: usize = 8;

/// V1 `STTService`.
pub struct SttService {
    /// `transcriptionComplete(text)`.
    pub transcription_complete: Emitter<str>,
    /// `transcriptionError(message)`.
    pub transcription_error: Emitter<str>,
    /// `enabledChanged`.
    pub enabled_changed: Emitter<()>,
    /// `providerChanged`.
    pub provider_changed: Emitter<()>,
    /// `serverUrlChanged`.
    pub server_url_changed: Emitter<()>,
    /// `languageChanged`.
    pub language_changed: Emitter<()>,
    /// `recordingStateChanged` (read [`is_recording`](Self::is_recording)).
    pub recording_state_changed: Emitter<()>,
    /// `serverAvailableChanged`.
    pub server_available_changed: Emitter<()>,
    /// `recordingProgress(level)`, 0..1.
    pub recording_progress: Emitter<f64>,
    is_recording: Cell<bool>,
    server_available: Cell<bool>,
    starting: Cell<bool>,
    max_duration_timer: RefCell<Option<Timeout>>,
    /// V1 `mediaRecorder` / `mediaStream` / `audioContext` / `analyser` of the current recording.
    recorder: RefCell<Option<Rc<RecorderSession>>>,
    /// Stopped recordings replaced by a newer one before their `onstop` fired (kept alive so
    /// they still finalize and release their microphone).
    finishing: RefCell<Vec<Rc<RecorderSession>>>,
    /// V1 `speechRecognition` (the running browser recognizer).
    recognition: RefCell<Option<Rc<RecognitionSession>>>,
    /// Recognizers kept alive until their `onend` (results arrive after `stop()`).
    recognitions: RefCell<Vec<Rc<RecognitionSession>>>,
    weak_self: Weak<SttService>,
}

impl SttService {
    /// V1 constructor: health check right away when STT is enabled.
    pub(crate) fn new() -> Rc<Self> {
        let service = Rc::new_cyclic(|weak| Self {
            transcription_complete: Emitter::new(),
            transcription_error: Emitter::new(),
            enabled_changed: Emitter::new(),
            provider_changed: Emitter::new(),
            server_url_changed: Emitter::new(),
            language_changed: Emitter::new(),
            recording_state_changed: Emitter::new(),
            server_available_changed: Emitter::new(),
            recording_progress: Emitter::new(),
            is_recording: Cell::new(false),
            server_available: Cell::new(false),
            starting: Cell::new(false),
            max_duration_timer: RefCell::new(None),
            recorder: RefCell::new(None),
            finishing: RefCell::new(Vec::new()),
            recognition: RefCell::new(None),
            recognitions: RefCell::new(Vec::new()),
            weak_self: weak.clone(),
        });
        if service.enabled() {
            service.check_server_health();
        }
        service
    }

    /// V1 `enabled` (`Config_Experimental_STT_Enabled === 1`).
    pub fn enabled(&self) -> bool {
        keys::STT_ENABLED.get(settings())
    }

    /// V1 `set enabled(v)`: no-op when unchanged; persists and emits; then a health check when
    /// enabling, or `stop_recording()` when disabling. (The chat panel's mic toggle enables STT
    /// this way, persistently.)
    pub fn set_enabled(&self, value: bool) {
        if self.enabled() == value {
            return;
        }
        keys::STT_ENABLED.set(settings(), value);
        self.enabled_changed.emit(&());
        if value {
            self.check_server_health();
        } else {
            self.stop_recording();
        }
    }

    /// V1 `provider` (`Config_Experimental_STT_Provider`, default `groq`).
    pub fn provider(&self) -> String {
        keys::STT_PROVIDER.get(settings())
    }

    /// V1 `set provider(v)`.
    pub fn set_provider(&self, value: &str) {
        if self.provider() == value {
            return;
        }
        keys::STT_PROVIDER.set(settings(), value);
        self.provider_changed.emit(&());
    }

    /// V1 `serverUrl` (`Config_Experimental_STT_ServerUrl`, default `http://localhost:5101`).
    pub fn server_url(&self) -> String {
        keys::STT_SERVER_URL.get(settings())
    }

    /// V1 `set serverUrl(v)`: persists, emits, then checks health.
    pub fn set_server_url(&self, value: &str) {
        if self.server_url() == value {
            return;
        }
        keys::STT_SERVER_URL.set(settings(), value);
        self.server_url_changed.emit(&());
        self.check_server_health();
    }

    /// V1 `language` (`Config_Experimental_STT_Language`, default `ja`; free text).
    pub fn language(&self) -> String {
        keys::STT_LANGUAGE.get(settings())
    }

    /// V1 `set language(v)`.
    pub fn set_language(&self, value: &str) {
        if self.language() == value {
            return;
        }
        keys::STT_LANGUAGE.set(settings(), value);
        self.language_changed.emit(&());
    }

    /// V1 `isRecording`.
    pub fn is_recording(&self) -> bool {
        self.is_recording.get()
    }

    /// V1 `serverAvailable`.
    pub fn server_available(&self) -> bool {
        self.server_available.get()
    }

    /// V1 `startRecording()` (see the module docs for the decision tree).
    pub fn start_recording(&self) {
        if !self.enabled() {
            self.transcription_error.emit(STT_NOT_ENABLED);
            return;
        }
        if self.is_recording.get() || self.starting.get() {
            return;
        }
        if !crate::env::is_tauri() && self.try_start_browser_recognition() {
            return;
        }
        let Some(devices) = media_devices() else {
            self.transcription_error.emit(MICROPHONE_NOT_SUPPORTED);
            return;
        };
        self.starting.set(true);
        let weak = self.weak_self.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let stream = open_microphone(&devices).await;
            let Some(this) = weak.upgrade() else { return };
            this.starting.set(false);
            let result = stream.and_then(|stream| this.begin_recording(stream));
            if let Err(message) = result {
                this.transcription_error.emit(&message);
            }
        });
    }

    /// V1 `stopRecording()`.
    pub fn stop_recording(&self) {
        if !self.is_recording.get() {
            return;
        }
        self.is_recording.set(false);
        self.recording_state_changed.emit(&());
        self.clear_max_duration_timer();
        let recognition = self.recognition.borrow_mut().take();
        if let Some(session) = recognition {
            // The browser then delivers `onresult` (if any) and `onend`.
            if let Err(error) = session.call("stop") {
                let message = js_error_message(&error)
                    .unwrap_or_else(|| FAILED_TO_STOP_RECOGNITION.to_owned());
                self.transcription_error.emit(&message);
            }
            return;
        }
        let recorder = self.recorder.borrow().clone();
        match recorder {
            // Final `dataavailable`, then `onstop` (asynchronously).
            Some(session) if session.recorder.state() != RecordingState::Inactive => {
                let _ = session.recorder.stop();
            }
            _ => self.cleanup_recorder(),
        }
    }

    /// V1 `transcribeData(blob)`: uploads the recording (`audio` as `recording.webm`,
    /// `provider`, raw `language`, `serverUrl`) and emits the transcript (`""` when the reply
    /// has no text) or an error.
    pub fn transcribe_data(&self, audio: Blob) {
        if !self.enabled() {
            self.transcription_error.emit(STT_NOT_ENABLED);
            return;
        }
        let fields = SttTranscribeFields {
            provider: self.provider(),
            language: self.language(),
            server_url: self.server_url(),
        };
        let weak = self.weak_self.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let result = transport().stt_transcribe(&audio, &fields).await;
            let Some(this) = weak.upgrade() else { return };
            match result {
                Err(error) => this
                    .transcription_error
                    .emit(&error.message_or(TRANSCRIPTION_FAILED)),
                Ok(reply) if !reply.ok => {
                    let message =
                        reply.error_or(|| format!("Transcription failed ({})", reply.status));
                    this.transcription_error.emit(&message);
                }
                Ok(reply) => this
                    .transcription_complete
                    .emit(&reply.field_or("text", String::new)),
            }
        });
    }

    /// V1 `checkServerHealth()` (background): `ok && Boolean(data?.available)`, emitting
    /// `server_available_changed` on change.
    pub fn check_server_health(&self) {
        let server_url = self.server_url();
        let weak = self.weak_self.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let available = transport()
                .stt_health(&server_url)
                .await
                .map(|r| r.available())
                .unwrap_or(false);
            let Some(this) = weak.upgrade() else { return };
            if this.server_available.get() != available {
                this.server_available.set(available);
                this.server_available_changed.emit(&());
            }
        });
    }

    fn arm_max_duration_timer(&self) {
        let weak = self.weak_self.clone();
        let timer = Timeout::new(MAX_RECORDING_MS, move || {
            if let Some(this) = weak.upgrade() {
                this.stop_recording();
            }
        });
        let previous = self.max_duration_timer.borrow_mut().replace(timer);
        drop(previous);
    }

    fn clear_max_duration_timer(&self) {
        let timer = self.max_duration_timer.borrow_mut().take();
        drop(timer); // dropping a gloo `Timeout` clears it
    }

    // -----------------------------------------------------------------------------------------
    // MediaRecorder path
    // -----------------------------------------------------------------------------------------

    /// The synchronous rest of V1's `try` block after `getUserMedia` resolved.
    fn begin_recording(&self, stream: MediaStream) -> Result<(), String> {
        let recorder = match MediaRecorder::new_with_media_stream(&stream) {
            Ok(recorder) => recorder,
            Err(error) => {
                stop_tracks(&stream); // Q-S6: V1 left the microphone open here.
                return Err(start_error(&error));
            }
        };
        let session = Rc::new(RecorderSession::new(stream, recorder));

        let weak_session = Rc::downgrade(&session);
        let on_data = Closure::<dyn FnMut(BlobEvent)>::new(move |event: BlobEvent| {
            let (Some(session), Some(data)) = (weak_session.upgrade(), event.data()) else {
                return;
            };
            if data.size() > 0.0 {
                session.chunks.borrow_mut().push(data);
            }
        });
        let weak_service = self.weak_self.clone();
        let weak_session = Rc::downgrade(&session);
        let on_stop = Closure::<dyn FnMut()>::new(move || {
            if let (Some(this), Some(session)) = (weak_service.upgrade(), weak_session.upgrade()) {
                this.on_recorder_stop(&session);
            }
        });
        session
            .recorder
            .set_ondataavailable(Some(on_data.as_ref().unchecked_ref()));
        session
            .recorder
            .set_onstop(Some(on_stop.as_ref().unchecked_ref()));
        *session.handlers.borrow_mut() = Some((on_data, on_stop));

        let previous = self.recorder.borrow_mut().replace(Rc::clone(&session));
        if let Some(previous) = previous {
            self.finishing.borrow_mut().push(previous);
        }
        if let Err(error) = session
            .recorder
            .start_with_time_slice(RECORDER_TIMESLICE_MS)
        {
            self.recorder.borrow_mut().take();
            session.cleanup(); // Q-S6: release the microphone.
            return Err(start_error(&error));
        }
        self.is_recording.set(true);
        self.recording_state_changed.emit(&());
        // V1: a meter failure is reported like a start failure and skips the 30 s timer.
        self.setup_recording_monitor(&session)
            .map_err(|e| start_error(&e))?;
        self.arm_max_duration_timer();
        Ok(())
    }

    /// V1 `setupRecordingMonitor(stream)`: a level meter, not VAD.
    fn setup_recording_monitor(&self, session: &Rc<RecorderSession>) -> Result<(), JsValue> {
        if !has_global("AudioContext") {
            return Ok(());
        }
        let context = AudioContext::new()?;
        let source = context.create_media_stream_source(&session.stream)?;
        let analyser = context.create_analyser()?;
        analyser.set_fft_size(ANALYSER_FFT_SIZE);
        source.connect_with_audio_node(&analyser)?; // not connected to the destination
        let mut data = vec![0_u8; analyser.frequency_bin_count() as usize];
        *session.audio_context.borrow_mut() = Some(context);
        *session.analyser.borrow_mut() = Some(analyser);

        let weak_service = self.weak_self.clone();
        let weak_session = Rc::downgrade(session);
        let tick = Closure::<dyn FnMut()>::new(move || {
            let (Some(this), Some(session)) = (weak_service.upgrade(), weak_session.upgrade())
            else {
                return;
            };
            let analyser = session.analyser.borrow().clone();
            let Some(analyser) = analyser else { return };
            if !this.is_recording.get() {
                return;
            }
            analyser.get_byte_frequency_data(&mut data);
            this.recording_progress.emit(&recording_level(&data));
            session.request_frame();
        });
        *session.monitor_tick.borrow_mut() = Some(tick);
        session.request_frame();
        Ok(())
    }

    /// V1 `mediaRecorder.onstop`.
    fn on_recorder_stop(&self, session: &Rc<RecorderSession>) {
        let mime = session.recorder.mime_type();
        let blob_type = if mime.is_empty() {
            FALLBACK_RECORDING_TYPE.to_owned()
        } else {
            mime
        };
        let parts = js_sys::Array::new();
        for chunk in session.chunks.borrow().iter() {
            parts.push(chunk);
        }
        let options = BlobPropertyBag::new();
        options.set_type(&blob_type);
        let blob = Blob::new_with_blob_sequence_and_options(&parts, &options);
        session.cleanup();
        let is_current = self
            .recorder
            .borrow()
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, session));
        if is_current {
            self.recorder.borrow_mut().take();
        } else {
            self.finishing
                .borrow_mut()
                .retain(|other| !Rc::ptr_eq(other, session));
        }
        match blob {
            Ok(blob) if blob.size() < MIN_RECORDING_BYTES => {
                self.transcription_error.emit(RECORDING_TOO_SHORT)
            }
            Ok(blob) => self.transcribe_data(blob),
            Err(error) => {
                let message =
                    js_error_message(&error).unwrap_or_else(|| TRANSCRIPTION_FAILED.to_owned());
                self.transcription_error.emit(&message);
            }
        }
    }

    /// V1 `cleanupRecordingGraph()` for the current recording.
    fn cleanup_recorder(&self) {
        let session = self.recorder.borrow_mut().take();
        if let Some(session) = session {
            session.cleanup();
        }
    }

    // -----------------------------------------------------------------------------------------
    // Browser SpeechRecognition path
    // -----------------------------------------------------------------------------------------

    /// V1 `tryStartBrowserRecognition()`.
    fn try_start_browser_recognition(&self) -> bool {
        let Some(window) = web_sys::window() else {
            return false;
        };
        let Some(ctor) = speech_recognition_ctor(&window) else {
            return false;
        };
        let created = ctor
            .dyn_ref::<js_sys::Function>()
            .ok_or_else(|| JsValue::from_str("SpeechRecognition is not a constructor"))
            .and_then(|ctor| js_sys::Reflect::construct(ctor, &js_sys::Array::new()));
        let recognition = match created {
            Ok(recognition) => recognition,
            Err(error) => {
                // V1: `new ctor()` throws out of `startRecording` (an unhandled rejection).
                log::warn!(
                    "SpeechRecognition could not be created: {}",
                    crate::env::js_to_string(&error)
                );
                return true;
            }
        };
        let session = Rc::new(RecognitionSession {
            recognition: recognition.clone(),
            handlers: RefCell::new(Vec::new()),
        });
        let set = |name: &str, value: &JsValue| {
            let _ = js_sys::Reflect::set(&recognition, &JsValue::from_str(name), value);
        };
        set(
            "lang",
            &JsValue::from_str(&resolve_speech_language(&self.language())),
        );
        set("interimResults", &JsValue::FALSE);
        set("continuous", &JsValue::FALSE);

        let weak = self.weak_self.clone();
        let on_result = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
            if let Some(this) = weak.upgrade() {
                this.on_recognition_result(&event);
            }
        });
        let weak = self.weak_self.clone();
        let on_error = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
            if let Some(this) = weak.upgrade() {
                this.on_recognition_error(&event);
            }
        });
        let weak = self.weak_self.clone();
        let weak_session = Rc::downgrade(&session);
        let on_end = Closure::<dyn FnMut(JsValue)>::new(move |_event: JsValue| {
            if let (Some(this), Some(session)) = (weak.upgrade(), weak_session.upgrade()) {
                this.on_recognition_end(&session);
            }
        });
        set("onresult", on_result.as_ref());
        set("onerror", on_error.as_ref());
        set("onend", on_end.as_ref());
        session
            .handlers
            .borrow_mut()
            .extend([on_result, on_error, on_end]);

        {
            let mut sessions = self.recognitions.borrow_mut();
            sessions.push(Rc::clone(&session));
            if sessions.len() > MAX_PENDING_RECOGNITIONS {
                sessions.remove(0);
            }
        }
        *self.recognition.borrow_mut() = Some(Rc::clone(&session));
        self.is_recording.set(true);
        self.recording_state_changed.emit(&());
        self.recording_progress.emit(&BROWSER_RECOGNITION_LEVEL);
        self.arm_max_duration_timer();
        if let Err(error) = session.call("start") {
            // V1 does not catch this (Q-S7, kept): "recording" until the 30 s timer.
            log::warn!(
                "SpeechRecognition.start() failed: {}",
                crate::env::js_to_string(&error)
            );
        }
        true
    }

    /// V1 `recognition.onresult`: first alternative of every result, concatenated, trimmed.
    fn on_recognition_result(&self, event: &JsValue) {
        let get = |target: &JsValue, key: &str| {
            js_sys::Reflect::get(target, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED)
        };
        let results = get(event, "results");
        let count = get(&results, "length").as_f64().unwrap_or(0.0).max(0.0) as u32;
        let mut transcript = String::new();
        for index in 0..count {
            let result = js_sys::Reflect::get_u32(&results, index).unwrap_or(JsValue::UNDEFINED);
            if !result.is_truthy() || get(&result, "length").as_f64().unwrap_or(0.0) <= 0.0 {
                continue;
            }
            let first = js_sys::Reflect::get_u32(&result, 0).unwrap_or(JsValue::UNDEFINED);
            if first.is_undefined() || first.is_null() {
                continue;
            }
            let text = get(&first, "transcript");
            if !text.is_undefined() && !text.is_null() {
                transcript.push_str(&crate::env::js_to_string(&text));
            }
        }
        let transcript = js_trim(&transcript);
        if transcript.is_empty() {
            self.transcription_error.emit(NO_SPEECH_DETECTED);
        } else {
            self.transcription_complete.emit(transcript);
        }
    }

    /// V1 `recognition.onerror`: `Speech recognition failed: <code>` (`unknown_error` default).
    fn on_recognition_error(&self, event: &JsValue) {
        let code =
            js_sys::Reflect::get(event, &JsValue::from_str("error")).unwrap_or(JsValue::UNDEFINED);
        let code = if code.is_undefined() || code.is_null() {
            "unknown_error".to_owned()
        } else {
            crate::env::js_to_string(&code)
        };
        self.transcription_error
            .emit(&format!("Speech recognition failed: {code}"));
    }

    /// V1 `recognition.onend`. A late `onend` of an older recognizer leaves a newer running one
    /// alone (V1 would mark it stopped).
    fn on_recognition_end(&self, session: &Rc<RecognitionSession>) {
        let stale = self
            .recognition
            .borrow()
            .as_ref()
            .is_some_and(|current| !Rc::ptr_eq(current, session));
        if !stale {
            self.recognition.borrow_mut().take();
            if self.is_recording.get() {
                self.is_recording.set(false);
                self.recording_state_changed.emit(&());
            }
            self.clear_max_duration_timer();
            self.recording_progress.emit(&0.0);
        }
        self.recognitions
            .borrow_mut()
            .retain(|s| !Rc::ptr_eq(s, session));
    }
}

/// `ondataavailable` and `onstop` of one recorder.
type RecorderHandlers = (Closure<dyn FnMut(BlobEvent)>, Closure<dyn FnMut()>);

/// `onresult`, `onerror` and `onend` of one recognizer.
type RecognitionHandlers = Vec<Closure<dyn FnMut(JsValue)>>;

/// One `MediaRecorder` recording (V1 kept these as service fields).
struct RecorderSession {
    stream: MediaStream,
    recorder: MediaRecorder,
    chunks: RefCell<Vec<Blob>>,
    audio_context: RefCell<Option<AudioContext>>,
    analyser: RefCell<Option<AnalyserNode>>,
    monitor_frame: Cell<Option<i32>>,
    monitor_tick: RefCell<Option<Closure<dyn FnMut()>>>,
    handlers: RefCell<Option<RecorderHandlers>>,
}

impl RecorderSession {
    fn new(stream: MediaStream, recorder: MediaRecorder) -> Self {
        Self {
            stream,
            recorder,
            chunks: RefCell::new(Vec::new()),
            audio_context: RefCell::new(None),
            analyser: RefCell::new(None),
            monitor_frame: Cell::new(None),
            monitor_tick: RefCell::new(None),
            handlers: RefCell::new(None),
        }
    }

    fn request_frame(&self) {
        let tick = self.monitor_tick.borrow();
        let (Some(tick), Some(window)) = (tick.as_ref(), web_sys::window()) else {
            return;
        };
        if let Ok(id) = window.request_animation_frame(tick.as_ref().unchecked_ref()) {
            self.monitor_frame.set(Some(id));
        }
    }

    /// V1 `cleanupRecordingGraph()`: cancel the meter, close the context (not awaited), stop
    /// every track.
    fn cleanup(&self) {
        if let (Some(id), Some(window)) = (self.monitor_frame.take(), web_sys::window()) {
            let _ = window.cancel_animation_frame(id);
        }
        let tick = self.monitor_tick.borrow_mut().take();
        drop(tick);
        let context = self.audio_context.borrow_mut().take();
        if let Some(context) = context {
            let _ = context.close();
        }
        self.analyser.borrow_mut().take();
        stop_tracks(&self.stream);
    }
}

impl Drop for RecorderSession {
    fn drop(&mut self) {
        if self.handlers.get_mut().take().is_some() {
            self.recorder.set_ondataavailable(None);
            self.recorder.set_onstop(None);
        }
    }
}

/// One browser recognizer and its handlers.
struct RecognitionSession {
    recognition: JsValue,
    handlers: RefCell<RecognitionHandlers>,
}

impl RecognitionSession {
    /// `recognition[method]()`.
    fn call(&self, method: &str) -> Result<(), JsValue> {
        let function = js_sys::Reflect::get(&self.recognition, &JsValue::from_str(method))?
            .dyn_into::<js_sys::Function>()
            .map_err(|_| JsValue::from_str("not a function"))?;
        function.call0(&self.recognition).map(|_| ())
    }
}

impl Drop for RecognitionSession {
    fn drop(&mut self) {
        if !self.handlers.get_mut().is_empty() {
            for name in ["onresult", "onerror", "onend"] {
                let _ = js_sys::Reflect::set(
                    &self.recognition,
                    &JsValue::from_str(name),
                    &JsValue::NULL,
                );
            }
        }
    }
}

/// V1 `window.SpeechRecognition ?? window.webkitSpeechRecognition`, then `if (!ctor)`.
fn speech_recognition_ctor(window: &web_sys::Window) -> Option<JsValue> {
    let get = |name: &str| {
        js_sys::Reflect::get(window, &JsValue::from_str(name)).unwrap_or(JsValue::UNDEFINED)
    };
    let standard = get("SpeechRecognition");
    let ctor = if standard.is_undefined() || standard.is_null() {
        get("webkitSpeechRecognition")
    } else {
        standard
    };
    ctor.is_truthy().then_some(ctor)
}

/// V1 `navigator.mediaDevices?.getUserMedia` (absent in insecure contexts).
fn media_devices() -> Option<MediaDevices> {
    let navigator = web_sys::window()?.navigator();
    let devices = js_sys::Reflect::get(&navigator, &JsValue::from_str("mediaDevices")).ok()?;
    if devices.is_undefined() || devices.is_null() {
        return None;
    }
    let get_user_media = js_sys::Reflect::get(&devices, &JsValue::from_str("getUserMedia")).ok()?;
    get_user_media.is_truthy().then(|| devices.unchecked_into())
}

/// `await navigator.mediaDevices.getUserMedia({ audio: true })`.
async fn open_microphone(devices: &MediaDevices) -> Result<MediaStream, String> {
    let constraints = MediaStreamConstraints::new();
    constraints.set_audio(&JsValue::TRUE);
    let promise = devices
        .get_user_media_with_constraints(&constraints)
        .map_err(|e| start_error(&e))?;
    let stream = JsFuture::from(promise).await.map_err(|e| start_error(&e))?;
    stream
        .dyn_into::<MediaStream>()
        .map_err(|_| FAILED_TO_START_RECORDING.to_owned())
}

/// V1 `error instanceof Error ? error.message : "Failed to start recording"`.
fn start_error(error: &JsValue) -> String {
    js_error_message(error).unwrap_or_else(|| FAILED_TO_START_RECORDING.to_owned())
}

fn stop_tracks(stream: &MediaStream) {
    for track in stream.get_tracks().iter() {
        track.unchecked_into::<web_sys::MediaStreamTrack>().stop();
    }
}

/// `typeof globalThis[name] !== "undefined"`.
fn has_global(name: &str) -> bool {
    js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str(name))
        .is_ok_and(|v| !v.is_undefined())
}

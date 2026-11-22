//! Port of V1 `lib/amadeus/services/audio-service.ts`: one `HTMLAudioElement` at a time plus a
//! synthetic lip-sync value (a position-keyed oscillation, EMA-smoothed, 20 Hz).
//!
//! V1 facts kept: the volume is read from `Config_VoiceVol` once, at construction (page load:
//! [`crate::app::managers::init`] creates the service when `/app` mounts, so a change made in
//! CONFIG, even on the login screen, applies only after a reload, QUIRK-14), and is not
//! clamped there (an out-of-range value makes `playFromFile` report the browser's
//! `IndexSizeError` message); `stop()` never emits `playback_finished`; a `play()` that is
//! interrupted by `stop()` rejects and emits a (harmless) `error_occurred`; one failure can emit
//! `error_occurred` twice (element `error` + rejected `play()`). Nothing in V1 consumes
//! `lip_sync_value_changed` (the Live2D mouth follows the typewriter instead).

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use amadeus_core::settings::keys;
use amadeus_core::types::Emitter;
use gloo_timers::callback::Interval;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::HtmlAudioElement;

use super::logic::{clamp_volume, lip_sync_smooth, lip_sync_target, LIP_SYNC_UPDATE_MS};
use super::settings;
use crate::transport::js_error_message;

/// V1 `"Audio playback failed"`.
pub const AUDIO_PLAYBACK_FAILED: &str = "Audio playback failed";
/// V1 `"Failed to resume audio"`.
pub const FAILED_TO_RESUME_AUDIO: &str = "Failed to resume audio";

/// `onended` and `onerror` of one element.
type ElementHandlers = (Closure<dyn FnMut()>, Closure<dyn FnMut()>);

/// The current element and the handlers attached to it.
struct Track {
    element: HtmlAudioElement,
    handlers: Option<ElementHandlers>,
}

impl Track {
    /// Removes the handlers before they are dropped (a detached element must never call a
    /// dropped closure).
    fn detach(&mut self) {
        if self.handlers.take().is_some() {
            self.element.set_onended(None);
            self.element.set_onerror(None);
        }
    }
}

/// V1 `AudioService`.
pub struct AudioService {
    /// `playbackStarted` — `play()` resolved (in `play_from_file`).
    pub playback_started: Emitter<()>,
    /// `playbackFinished` — the element ended.
    pub playback_finished: Emitter<()>,
    /// `playbackStateChanged`.
    pub playback_state_changed: Emitter<()>,
    /// `lipSyncValueChanged(value)` (no V1 subscriber).
    pub lip_sync_value_changed: Emitter<f64>,
    /// `volumeChanged(volume)`.
    pub volume_changed: Emitter<f64>,
    /// `errorOccurred(message)`.
    pub error_occurred: Emitter<str>,
    track: RefCell<Option<Track>>,
    lip_sync_timer: RefCell<Option<Interval>>,
    is_playing: Cell<bool>,
    current_lip_sync_value: Cell<f64>,
    volume: Cell<f64>,
    weak_self: Weak<AudioService>,
}

impl AudioService {
    /// V1 constructor: reads `Config_VoiceVol` (default 1.0) once. Called by
    /// [`super::audio`], first from [`crate::app::managers::init`].
    pub(crate) fn new() -> Rc<Self> {
        let volume = keys::VOICE_VOL.get(settings());
        Rc::new_cyclic(|weak| Self {
            playback_started: Emitter::new(),
            playback_finished: Emitter::new(),
            playback_state_changed: Emitter::new(),
            lip_sync_value_changed: Emitter::new(),
            volume_changed: Emitter::new(),
            error_occurred: Emitter::new(),
            track: RefCell::new(None),
            lip_sync_timer: RefCell::new(None),
            is_playing: Cell::new(false),
            current_lip_sync_value: Cell::new(0.0),
            volume: Cell::new(volume),
            weak_self: weak.clone(),
        })
    }

    /// V1 `isPlaying`.
    pub fn is_playing(&self) -> bool {
        self.is_playing.get()
    }

    /// V1 `currentLipSyncValue`.
    pub fn current_lip_sync_value(&self) -> f64 {
        self.current_lip_sync_value.get()
    }

    /// V1 `volume`.
    pub fn volume(&self) -> f64 {
        self.volume.get()
    }

    /// V1 `setVolume(value)` (unused by V1): clamps to `[0, 1]`, applies it to the current
    /// element, persists `Config_VoiceVol` (JS number formatting) and emits `volume_changed`.
    pub fn set_volume(&self, value: f64) {
        let volume = clamp_volume(value);
        self.volume.set(volume);
        let element = self.track.borrow().as_ref().map(|t| t.element.clone());
        if let Some(element) = element {
            if let Err(error) = js_sys::Reflect::set(
                &element,
                &JsValue::from_str("volume"),
                &JsValue::from_f64(volume),
            ) {
                // V1 throws an uncaught TypeError for NaN here.
                log::warn!(
                    "AudioService.setVolume: {}",
                    js_error_message(&error).unwrap_or_default()
                );
            }
        }
        keys::VOICE_VOL.set(settings(), volume);
        self.volume_changed.emit(&volume);
    }

    /// V1 `playFromFile(url)`: stops the current element, then plays `url` in a new one.
    pub fn play_from_file(&self, url: &str) {
        self.stop();
        if let Err(error) = self.start_track(url) {
            let message =
                js_error_message(&error).unwrap_or_else(|| AUDIO_PLAYBACK_FAILED.to_owned());
            self.error_occurred.emit(&message);
        }
    }

    /// V1 `playFromData(blob)` (unused by V1): plays an object URL of `blob` (never revoked).
    pub fn play_from_blob(&self, blob: &web_sys::Blob) {
        match web_sys::Url::create_object_url_with_blob(blob) {
            Ok(url) => self.play_from_file(&url),
            Err(error) => {
                let message =
                    js_error_message(&error).unwrap_or_else(|| AUDIO_PLAYBACK_FAILED.to_owned());
                self.error_occurred.emit(&message);
            }
        }
    }

    /// The `try` block of V1 `playFromFile`.
    fn start_track(&self, url: &str) -> Result<(), JsValue> {
        let element = HtmlAudioElement::new_with_src(url)?;
        // V1 assigns `this.audio` before setting the volume (which may throw).
        *self.track.borrow_mut() = Some(Track {
            element: element.clone(),
            handlers: None,
        });
        js_sys::Reflect::set(
            &element,
            &JsValue::from_str("volume"),
            &JsValue::from_f64(self.volume.get()),
        )?;

        let weak = self.weak_self.clone();
        let on_ended = Closure::<dyn FnMut()>::new(move || {
            if let Some(this) = weak.upgrade() {
                this.on_ended();
            }
        });
        let weak = self.weak_self.clone();
        let on_error = Closure::<dyn FnMut()>::new(move || {
            if let Some(this) = weak.upgrade() {
                this.error_occurred.emit(AUDIO_PLAYBACK_FAILED);
            }
        });
        element.set_onended(Some(on_ended.as_ref().unchecked_ref()));
        element.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        if let Some(track) = self.track.borrow_mut().as_mut() {
            track.handlers = Some((on_ended, on_error));
        }

        let promise = element.play()?;
        let weak = self.weak_self.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let result = JsFuture::from(promise).await;
            let Some(this) = weak.upgrade() else { return };
            match result {
                Ok(_) => {
                    this.is_playing.set(true);
                    this.playback_state_changed.emit(&());
                    this.playback_started.emit(&());
                    this.start_lip_sync_timer();
                }
                Err(error) => {
                    let message = js_error_message(&error)
                        .unwrap_or_else(|| AUDIO_PLAYBACK_FAILED.to_owned());
                    this.error_occurred.emit(&message);
                }
            }
        });
        Ok(())
    }

    /// V1 `audio.onended`, in V1 order.
    fn on_ended(&self) {
        self.stop_lip_sync_timer();
        self.is_playing.set(false);
        self.current_lip_sync_value.set(0.0);
        self.lip_sync_value_changed.emit(&0.0);
        self.playback_state_changed.emit(&());
        self.playback_finished.emit(&());
    }

    /// V1 `stop()`: always emits `lip_sync_value_changed(0)`; pauses, rewinds and drops the
    /// element; emits `playback_state_changed` if it was playing. Never `playback_finished`.
    pub fn stop(&self) {
        self.stop_lip_sync_timer();
        self.current_lip_sync_value.set(0.0);
        self.lip_sync_value_changed.emit(&0.0);
        let track = self.track.borrow_mut().take();
        if let Some(mut track) = track {
            let _ = track.element.pause();
            let _ = js_sys::Reflect::set(
                &track.element,
                &JsValue::from_str("currentTime"),
                &JsValue::from_f64(0.0),
            );
            track.detach();
        }
        if self.is_playing.get() {
            self.is_playing.set(false);
            self.playback_state_changed.emit(&());
        }
    }

    /// V1 `pause()` (unused by V1). Keeps the last lip-sync value.
    pub fn pause(&self) {
        let element = self.track.borrow().as_ref().map(|t| t.element.clone());
        let Some(element) = element else { return };
        let _ = element.pause();
        self.stop_lip_sync_timer();
        self.is_playing.set(false);
        self.playback_state_changed.emit(&());
    }

    /// V1 `resume()` (unused by V1). Does not emit `playback_started`.
    pub fn resume(&self) {
        let element = self.track.borrow().as_ref().map(|t| t.element.clone());
        let Some(element) = element else { return };
        let promise = match element.play() {
            Ok(promise) => promise,
            Err(error) => {
                let message =
                    js_error_message(&error).unwrap_or_else(|| FAILED_TO_RESUME_AUDIO.to_owned());
                self.error_occurred.emit(&message);
                return;
            }
        };
        let weak = self.weak_self.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let result = JsFuture::from(promise).await;
            let Some(this) = weak.upgrade() else { return };
            match result {
                Ok(_) => {
                    this.is_playing.set(true);
                    this.start_lip_sync_timer();
                    this.playback_state_changed.emit(&());
                }
                Err(error) => {
                    let message = js_error_message(&error)
                        .unwrap_or_else(|| FAILED_TO_RESUME_AUDIO.to_owned());
                    this.error_occurred.emit(&message);
                }
            }
        });
    }

    fn start_lip_sync_timer(&self) {
        self.stop_lip_sync_timer();
        let weak = self.weak_self.clone();
        let timer = Interval::new(LIP_SYNC_UPDATE_MS, move || {
            if let Some(this) = weak.upgrade() {
                this.update_lip_sync();
            }
        });
        *self.lip_sync_timer.borrow_mut() = Some(timer);
    }

    fn stop_lip_sync_timer(&self) {
        let timer = self.lip_sync_timer.borrow_mut().take();
        drop(timer); // dropping a gloo `Interval` clears it
    }

    /// V1 `updateLipSync()`.
    fn update_lip_sync(&self) {
        let position = if self.is_playing.get() {
            self.track
                .borrow()
                .as_ref()
                .map(|t| t.element.current_time())
        } else {
            None
        };
        let value = match position {
            None => 0.0,
            Some(position) => {
                let target = lip_sync_target(position, js_sys::Math::random());
                lip_sync_smooth(self.current_lip_sync_value.get(), target)
            }
        };
        self.current_lip_sync_value.set(value);
        self.lip_sync_value_changed.emit(&value);
    }
}

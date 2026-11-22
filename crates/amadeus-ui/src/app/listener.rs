//! `window` event listeners that own their closure. OWNED BY: app-root engineer.
//!
//! `leptos::prelude::window_event_listener` hands its closure to the JS garbage collector
//! (`Closure::into_js_value`), which leaks it unless wasm-bindgen weak references are enabled
//! (the Trunk build does not enable them). The root keyboard handler is re-registered on every
//! menu move, exactly like V1's effect, so the root uses this owned variant: dropping it removes
//! the listener and frees the closure.
//!
//! Like any DOM listener, one added while an event is being dispatched does not see that event,
//! and one removed before its turn is skipped.

use leptos::prelude::{LocalStorage, StoredValue, UpdateValue};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// `window.addEventListener(event, handler)` (bubble phase, no options) until dropped.
pub struct OwnedWindowListener {
    event: &'static str,
    closure: Closure<dyn FnMut(web_sys::Event)>,
}

impl OwnedWindowListener {
    /// Adds the listener; `None` without a window.
    pub fn new(event: &'static str, handler: impl FnMut(web_sys::Event) + 'static) -> Option<Self> {
        let window = web_sys::window()?;
        let closure = Closure::<dyn FnMut(web_sys::Event)>::new(handler);
        window
            .add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())
            .ok()?;
        Some(Self { event, closure })
    }

    /// A `keydown` listener receiving the [`web_sys::KeyboardEvent`].
    pub fn keydown(handler: impl Fn(&web_sys::KeyboardEvent) + 'static) -> Option<Self> {
        Self::new("keydown", move |event: web_sys::Event| {
            if let Some(event) = event.dyn_ref::<web_sys::KeyboardEvent>() {
                handler(event);
            }
        })
    }
}

impl Drop for OwnedWindowListener {
    fn drop(&mut self) {
        if let Some(window) = web_sys::window() {
            let _ = window.remove_event_listener_with_callback(
                self.event,
                self.closure.as_ref().unchecked_ref(),
            );
        }
    }
}

/// A reactive-owner-scoped slot for one listener (`Copy + Send + Sync`, so effects and cleanups
/// can capture it).
pub type ListenerSlot = StoredValue<Option<OwnedWindowListener>, LocalStorage>;

/// Replaces the slot's listener: the old one is removed first (V1 effect cleanup order).
pub fn replace_listener(slot: ListenerSlot, make: impl FnOnce() -> Option<OwnedWindowListener>) {
    slot.try_update_value(|current| {
        *current = None;
        *current = make();
    });
}

/// Removes the slot's listener (no-op once the slot was disposed).
pub fn clear_listener(slot: ListenerSlot) {
    slot.try_update_value(|current| *current = None);
}

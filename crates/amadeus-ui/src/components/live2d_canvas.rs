//! Live2D host component (V1 `Live2DCanvas.tsx`). OWNED BY: ui-foundation.
//!
//! Lifecycle (V1 order): after mount, load the Cubism Core script, create an
//! [`amadeus_live2d::Live2DPlayer`] in `<div class="live2d-host">`, push the latest emotion
//! (upper-cased), lip-sync value and spoken character, and force the canvas inline style to
//! `pointer-events:auto; width:100%; height:100%` (one 100 ms retry). Like V1's `useEffect`
//! dependencies, a prop is pushed only when its value changed (`Object.is`), however often
//! the signal notifies. With eye tracking on, `window` `mousemove` drives the gaze (viewport
//! mapping, clamped to ±0.7, y inverted); turning it off removes the listener and recentres
//! the gaze. On unmount: grab the WebGL context, `destroy()` the player, lose the context
//! (browsers cap live contexts and the layout switch remounts this component), empty the host.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use amadeus_live2d::{Live2DPlayer, PlayerOptions};
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

use crate::env::asset_url;

/// Base-relative URL of the proprietary Cubism Core (V1 `/resources/live2d/live2dcubismcore.min.js`).
pub const CORE_SCRIPT_PATH: &str = "resources/live2d/live2dcubismcore.min.js";

/// Eye-tracking clamp of both axes (V1 `Math.max(-0.7, Math.min(0.7, n))`).
pub const EYE_TRACKING_LIMIT: f64 = 0.7;

/// V1 mousemove mapping: `n = (client / inner - 0.5) * 2`, clamped to ±0.7, y inverted (screen
/// y grows down, model y up). Uses the **viewport**, not the canvas rect. A NaN axis (0-sized
/// window) becomes 0, which is what V1's player made of it (`eyeTrackingX || 0`).
pub fn eye_tracking_target(
    client_x: f64,
    client_y: f64,
    inner_width: f64,
    inner_height: f64,
) -> (f64, f64) {
    let nx = ((client_x / inner_width) - 0.5) * 2.0;
    let ny = ((client_y / inner_height) - 0.5) * 2.0;
    (clamp_axis(nx), -clamp_axis(ny))
}

fn clamp_axis(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(-EYE_TRACKING_LIMIT, EYE_TRACKING_LIMIT)
    }
}

/// V1 `emotion.toUpperCase()`.
fn normalize_emotion(emotion: &str) -> String {
    emotion.to_uppercase()
}

/// JS `Object.is` for numbers, React's dependency comparison: `NaN` equals `NaN`, `0` and `-0`
/// differ.
fn same_number(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

/// The eye-tracking `window` `mousemove` listener. Dropping it removes the listener and frees
/// the closure (leptos' `window_event_listener` hands its closure to JS for good, leaking one
/// per registration).
struct MouseMoveListener(Closure<dyn FnMut(web_sys::MouseEvent)>);

impl MouseMoveListener {
    fn add(handler: impl FnMut(web_sys::MouseEvent) + 'static) -> Option<Self> {
        let window = web_sys::window()?;
        let closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(handler);
        window
            .add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())
            .ok()?;
        Some(Self(closure))
    }
}

impl Drop for MouseMoveListener {
    fn drop(&mut self) {
        if let Some(window) = web_sys::window() {
            let _ = window
                .remove_event_listener_with_callback("mousemove", self.0.as_ref().unchecked_ref());
        }
    }
}

/// V1 `applyCanvasStyles`: true when a canvas was found and styled.
fn apply_canvas_styles(host: &web_sys::HtmlElement) -> bool {
    let Some(canvas) = host
        .query_selector("canvas")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
    else {
        return false;
    };
    let style = canvas.style();
    let _ = style.set_property("pointer-events", "auto");
    let _ = style.set_property("width", "100%");
    let _ = style.set_property("height", "100%");
    true
}

enum GlContext {
    WebGl(web_sys::WebGlRenderingContext),
    WebGl2(web_sys::WebGl2RenderingContext),
}

impl GlContext {
    /// V1 `c.getContext("webgl") || c.getContext("webgl2")` (returns the player's context; on a
    /// canvas without one this creates a WebGL1 context that is lost right away, like V1).
    fn of(canvas: &web_sys::HtmlCanvasElement) -> Option<Self> {
        if let Some(gl) = canvas
            .get_context("webgl")
            .ok()
            .flatten()
            .and_then(|ctx| ctx.dyn_into::<web_sys::WebGlRenderingContext>().ok())
        {
            return Some(Self::WebGl(gl));
        }
        canvas
            .get_context("webgl2")
            .ok()
            .flatten()
            .and_then(|ctx| ctx.dyn_into::<web_sys::WebGl2RenderingContext>().ok())
            .map(Self::WebGl2)
    }

    /// `gl.getExtension("WEBGL_lose_context")?.loseContext()` (no-op if already lost).
    fn lose(&self) {
        let extension = match self {
            Self::WebGl(gl) => gl.get_extension("WEBGL_lose_context"),
            Self::WebGl2(gl) => gl.get_extension("WEBGL_lose_context"),
        };
        if let Ok(Some(extension)) = extension {
            extension
                .unchecked_into::<web_sys::WebglLoseContext>()
                .lose_context();
        }
    }
}

fn with_player(
    player: StoredValue<Option<Live2DPlayer>, LocalStorage>,
    f: impl FnOnce(&Live2DPlayer),
) {
    player.try_with_value(|p| {
        if let Some(p) = p {
            f(p);
        }
    });
}

/// Hosts an `amadeus_live2d::Live2DPlayer` in `<div class="live2d-host">`.
#[component]
pub fn Live2DCanvas(
    #[prop(into)] emotion: Signal<String>,
    #[prop(into)] lip_sync_value: Signal<f64>,
    #[prop(optional, into)] spoken_char: Option<Signal<String>>,
    #[prop(optional, into)] eye_tracking_enabled: Option<Signal<bool>>,
) -> impl IntoView {
    let host: NodeRef<html::Div> = NodeRef::new();
    let player: StoredValue<Option<Live2DPlayer>, LocalStorage> = StoredValue::new_local(None);
    let mouse_listener: StoredValue<Option<MouseMoveListener>, LocalStorage> =
        StoredValue::new_local(None);
    let disposed = Arc::new(AtomicBool::new(false));

    // Mount: create the player once the host element exists (V1 mount effect).
    {
        let disposed = disposed.clone();
        Effect::new(move |started: Option<bool>| {
            if started == Some(true) {
                return true;
            }
            let Some(host_el) = host.get() else {
                return false;
            };
            let disposed = disposed.clone();
            leptos::task::spawn_local(async move {
                if let Err(error) =
                    amadeus_live2d::ensure_core_loaded(&asset_url(CORE_SCRIPT_PATH)).await
                {
                    web_sys::console::error_2(&JsValue::from_str("Cubism init failed"), &error);
                    return;
                }
                if disposed.load(Ordering::SeqCst) {
                    return;
                }
                let host_el: web_sys::HtmlElement = host_el.into();
                let created = Live2DPlayer::create(host_el.clone(), PlayerOptions::default());
                // The latest prop values (V1 kept them in refs while the player was null).
                created.set_emotion(&normalize_emotion(
                    &emotion.try_get_untracked().unwrap_or_default(),
                ));
                created.set_lip_sync(lip_sync_value.try_get_untracked().unwrap_or(0.0));
                created.set_spoken_char(
                    &spoken_char
                        .and_then(|s| s.try_get_untracked())
                        .unwrap_or_default(),
                );
                if player.try_set_value(Some(created.clone())).is_some() {
                    // Disposed between the check and now: nothing owns the player any more.
                    created.destroy();
                    return;
                }
                if !apply_canvas_styles(&host_el) {
                    set_timeout(
                        move || {
                            apply_canvas_styles(&host_el);
                        },
                        Duration::from_millis(100),
                    );
                }
            });
            true
        });
    }

    // V1 `useEffect(…, [emotion])`, `[lipSyncValue]`, `[spokenChar]`: each effect returns the
    // value it saw and pushes only a changed one (before the player exists the push is a no-op
    // and the create step pushes the latest values instead).
    Effect::new(move |prev: Option<String>| {
        let raw = emotion.get();
        if prev.as_ref() != Some(&raw) {
            let value = normalize_emotion(&raw);
            with_player(player, |p| p.set_emotion(&value));
        }
        raw
    });

    Effect::new(move |prev: Option<f64>| {
        let value = lip_sync_value.get();
        if !prev.is_some_and(|prev| same_number(prev, value)) {
            with_player(player, |p| p.set_lip_sync(value));
        }
        value
    });

    Effect::new(move |prev: Option<String>| {
        let value = spoken_char.map(|s| s.get()).unwrap_or_default();
        if prev.as_ref() != Some(&value) {
            with_player(player, |p| p.set_spoken_char(&value));
        }
        value
    });

    // V1 `useEffect(…, [eyeTrackingEnabled])`.
    Effect::new(move |prev: Option<bool>| {
        let enabled = eye_tracking_enabled.map(|s| s.get()).unwrap_or(false);
        if prev == Some(enabled) {
            return enabled;
        }
        // Dropping the previous listener removes it.
        drop(mouse_listener.try_update_value(Option::take));
        if enabled {
            let listener = MouseMoveListener::add(move |event: web_sys::MouseEvent| {
                let (width, height) = crate::env::inner_size();
                let (x, y) = eye_tracking_target(
                    f64::from(event.client_x()),
                    f64::from(event.client_y()),
                    width,
                    height,
                );
                with_player(player, |p| p.set_eye_tracking(x, y));
            });
            // Already disposed: the listener comes back and is dropped (removed) right here.
            drop(mouse_listener.try_set_value(listener));
        } else {
            with_player(player, |p| p.set_eye_tracking(0.0, 0.0));
        }
        enabled
    });

    on_cleanup(move || {
        disposed.store(true, Ordering::SeqCst);
        drop(mouse_listener.try_update_value(Option::take)); // removes the window listener
        let host_el = host.get_untracked();
        let gl = host_el
            .as_ref()
            .and_then(|h| h.query_selector("canvas").ok().flatten())
            .and_then(|c| c.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .and_then(|c| GlContext::of(&c));
        if let Some(p) = player.try_update_value(Option::take).flatten() {
            p.destroy();
        }
        if let Some(gl) = gl {
            gl.lose();
        }
        if let Some(h) = host_el {
            h.set_inner_html("");
        }
    });

    view! { <div class="live2d-host" node_ref=host></div> }
}

/// Fire-and-forget preload of the Cubism Core script and the default model assets (V1
/// `preloadLive2D`, called from the root mount effect while the login/boot screens show).
/// Never fails: errors are logged as `"Live2D preload warning:"`.
pub fn preload_live2d() {
    wasm_bindgen_futures::spawn_local(async {
        let result = async {
            amadeus_live2d::ensure_core_loaded(&asset_url(CORE_SCRIPT_PATH)).await?;
            amadeus_live2d::preload_default_assets(&PlayerOptions::default().resources_base).await
        }
        .await;
        if let Err(error) = result {
            web_sys::console::warn_2(&JsValue::from_str("Live2D preload warning:"), &error);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eye_mapping_matches_v1_vectors() {
        // spec-lib-media §6.5, 1920×1080 viewport.
        for ((cx, cy), (x, y)) in [
            ((0.0, 0.0), (-0.7, 0.7)),
            ((960.0, 540.0), (0.0, 0.0)),
            ((1920.0, 1080.0), (0.7, -0.7)),
            ((1440.0, 270.0), (0.5, 0.5)),
            ((480.0, 810.0), (-0.5, -0.5)),
        ] {
            assert_eq!(
                eye_tracking_target(cx, cy, 1920.0, 1080.0),
                (x, y),
                "({cx}, {cy})"
            );
        }
        // Outside the viewport (dragging over the scrollbar etc.) stays clamped.
        assert_eq!(
            eye_tracking_target(-500.0, 5000.0, 1920.0, 1080.0),
            (-0.7, -0.7)
        );
        // Zero-sized window: 0/0 is NaN → centred; c/0 is ±∞ → clamped.
        assert_eq!(eye_tracking_target(0.0, 10.0, 0.0, 0.0), (0.0, -0.7));
    }

    #[test]
    fn emotion_is_upper_cased() {
        assert_eq!(normalize_emotion("smile"), "SMILE");
        assert_eq!(normalize_emotion("Sleeping"), "SLEEPING");
        assert_eq!(normalize_emotion(""), "");
    }

    #[test]
    fn lip_sync_dedupe_uses_object_is() {
        assert!(same_number(0.25, 0.25));
        assert!(!same_number(0.25, 0.5));
        assert!(same_number(f64::NAN, f64::NAN), "Object.is(NaN, NaN)");
        assert!(!same_number(0.0, -0.0), "Object.is(0, -0) is false");
        assert!(same_number(f64::INFINITY, f64::INFINITY));
    }
}

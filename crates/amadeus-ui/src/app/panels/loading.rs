//! V1 `LoadingPanel` (desktop boot sequence, `RealAmadeusApp.tsx` 2063–2187). OWNED BY: panels
//! engineer.
//!
//! * `Config_SkipLoading == 1` (read once at mount, `getInt`): one empty frame, then
//!   `on_finished` from a 0 ms timeout.
//! * Otherwise, nominal timeline: 500 ms → BOOT_LOG revealed line by line (50 ms, or 400 ms after
//!   a line containing `...` or a blank line; the memory line animates 0 → 32767 in 51 frames of
//!   30 ms) → 2000 ms → `.boot-logo` replaces the `<pre>` → 4000 ms → `on_finished`
//!   (≈ 21.9 s, chained timeouts like V1, so the same small drift).
//! * `<pos=30%>` padding uses `measureText("M")` of `18px "DepartureMono", monospace`, measured
//!   once at mount like V1 (on the first boot of a page the web font is usually not loaded yet,
//!   so the platform's fallback monospace is measured; V1 quirk, SCR-011).
//! * The sequence runs once per mount (V1 restarted it on every parent re-render; spec §3.5 says
//!   not to replicate that). Unmounting cancels it and clears the pending timer.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::future::{AbortHandle, Abortable};
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::{JsCast, JsValue};

use amadeus_core::changelog::{
    boot_log_with_memory, boot_sequence, boot_visible_text, normalize_boot_char_width,
    BOOT_FINISH_DELAY_MS, BOOT_FONT, BOOT_INITIAL_DELAY_MS, BOOT_LOGO_DELAY_MS,
};
use amadeus_core::settings::keys;

use super::common::{fire, image_url, LOGO_V3};
use crate::hooks::app_settings;

#[component]
pub fn LoadingPanel(#[prop(into)] on_finished: Callback<()>) -> impl IntoView {
    // `useState(BOOT_LOG.replace("[MEM]", "0"))` (still unprocessed; nothing is visible yet).
    let display_text = RwSignal::new(boot_log_with_memory(0));
    let visible_chars = RwSignal::new(0_usize);
    let show_logo = RwSignal::new(false);

    // V1 ran the whole sequence from a mount `useEffect` (after the commit): the skip check, the
    // char-width measurement and the first timer all start once the panel is in the DOM. The body
    // is untracked, so the effect has no sources and runs exactly once per mount: a re-run would
    // cancel the sequence in its cleanup.
    Effect::new(move |_| {
        untrack(|| start_boot(display_text, visible_chars, show_logo, on_finished));
    });

    view! {
        <div class="loading-panel">
            {move || {
                if show_logo.get() {
                    view! { <img src=image_url(LOGO_V3) alt="logo" class="boot-logo"/> }.into_any()
                } else {
                    view! {
                        <pre class="boot-text">
                            {move || display_text.with(|text| boot_visible_text(text, visible_chars.get()).to_owned())}
                        </pre>
                    }
                        .into_any()
                }
            }}
        </div>
    }
}

/// The body of V1's mount effect. Registers its cleanup with the current owner (the effect):
/// unmounting clears the pending timer and cancels the sequence.
fn start_boot(
    display_text: RwSignal<String>,
    visible_chars: RwSignal<usize>,
    show_logo: RwSignal<bool>,
    on_finished: Callback<()>,
) {
    if app_settings().get_int(keys::SKIP_LOADING.key, 0) == 1 {
        // `const timer = window.setTimeout(onFinished, 0); return () => clearTimeout(timer);`
        let timer = set_timeout_with_handle(move || fire(on_finished, ()), Duration::ZERO).ok();
        on_cleanup(move || {
            if let Some(timer) = timer {
                timer.clear();
            }
        });
        return;
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    let (abort, registration) = AbortHandle::new_pair();
    on_cleanup({
        let cancelled = Arc::clone(&cancelled);
        move || {
            cancelled.store(true, Ordering::Relaxed);
            // Drops the pending `TimeoutFuture`, which clears its JS timeout (V1 cleared every
            // timeout id).
            abort.abort();
        }
    });
    let char_width = measure_boot_char_width();
    let run = run_boot_sequence(
        char_width,
        cancelled,
        display_text,
        visible_chars,
        show_logo,
        on_finished,
    );
    spawn_local(async move {
        let _ = Abortable::new(run, registration).await;
    });
}

/// V1 `runBootSequence()`. Checks `cancelled` after every await, like V1.
async fn run_boot_sequence(
    char_width: f64,
    cancelled: Arc<AtomicBool>,
    display_text: RwSignal<String>,
    visible_chars: RwSignal<usize>,
    show_logo: RwSignal<bool>,
    on_finished: Callback<()>,
) {
    let is_cancelled = || cancelled.load(Ordering::Relaxed);
    // The JS timeout starts here; V1 built each frame's text lazily inside the loop, so the steps
    // are precomputed while the first wait runs (the first line still appears 500 ms after mount).
    let initial_wait = TimeoutFuture::new(BOOT_INITIAL_DELAY_MS);
    let steps = boot_sequence(char_width);
    initial_wait.await;
    if is_cancelled() {
        return;
    }
    for step in steps {
        if is_cancelled() {
            return;
        }
        if let Some(text) = step.text {
            if display_text.try_set(text).is_some() {
                return;
            }
        }
        if visible_chars.try_set(step.visible_chars).is_some() {
            return;
        }
        if step.wait_after_ms > 0 {
            TimeoutFuture::new(step.wait_after_ms).await;
        }
    }
    TimeoutFuture::new(BOOT_LOGO_DELAY_MS).await;
    if is_cancelled() {
        return;
    }
    if show_logo.try_set(true).is_some() {
        return;
    }
    TimeoutFuture::new(BOOT_FINISH_DELAY_MS).await;
    if !is_cancelled() {
        fire(on_finished, ());
    }
}

/// V1 `estimateCharWidth()`: `measureText("M").width || 10` with
/// `font = '18px "DepartureMono", monospace'`, 10 without a 2D context.
fn measure_boot_char_width() -> f64 {
    normalize_boot_char_width(measure_m_width())
}

fn measure_m_width() -> Option<f64> {
    let canvas = document()
        .create_element("canvas")
        .ok()?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .ok()?;
    let context = canvas.get_context("2d").ok()??;
    js_sys::Reflect::set(
        &context,
        &JsValue::from_str("font"),
        &JsValue::from_str(BOOT_FONT),
    )
    .ok()?;
    let measure_text = js_sys::Reflect::get(&context, &JsValue::from_str("measureText"))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    let metrics = measure_text.call1(&context, &JsValue::from_str("M")).ok()?;
    js_sys::Reflect::get(&metrics, &JsValue::from_str("width"))
        .ok()?
        .as_f64()
}

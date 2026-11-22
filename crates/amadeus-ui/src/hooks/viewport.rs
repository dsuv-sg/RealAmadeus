//! Mobile layout detection: V1 `isMobileViewportSnapshot`, `subscribeViewportState` and
//! `useIsMobileViewport` (`RealAmadeusApp.tsx` L445–475).
//!
//! Rule (exact): `visualWidth = visualViewport?.width ?? innerWidth`,
//! `coarse = matchMedia("(pointer: coarse)").matches`,
//! `mobile = visualWidth <= 900 || (coarse && visualWidth <= 1200)`.
//! Re-evaluated on `window` `resize`, `visualViewport` `resize` and the `(pointer: coarse)`
//! media query `change`, nothing else. V2 is CSR, so the first value is the real snapshot
//! (V1's SSR snapshot `false` caused a desktop flash on phones).
//!
//! One page-wide watcher is installed on first use and lives for the whole page; consumers get
//! signals that only change when the boolean flips.

use std::cell::OnceCell;

use amadeus_core::settings::display::is_mobile_viewport;
use amadeus_core::types::{ConnectionId, Emitter};
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// Media query V1 uses for touch-first devices.
pub const COARSE_POINTER_QUERY: &str = "(pointer: coarse)";

/// V1 `visualViewport?.width ?? innerWidth` followed by the core rule
/// ([`amadeus_core::settings::display::is_mobile_viewport`]). `??` falls back only when the
/// visual viewport is missing: a visual width of `0` counts as mobile.
pub fn mobile_layout_for(
    visual_viewport_width: Option<f64>,
    inner_width: f64,
    coarse_pointer: bool,
) -> bool {
    is_mobile_viewport(visual_viewport_width.unwrap_or(inner_width), coarse_pointer)
}

/// V1 `isMobileViewportSnapshot()`: reads the browser right now (`false` without a window).
pub fn is_mobile_viewport_snapshot() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let coarse = window
        .match_media(COARSE_POINTER_QUERY)
        .ok()
        .flatten()
        .map(|query| query.matches())
        .unwrap_or(false);
    snapshot_with(&window, coarse)
}

fn snapshot_with(window: &web_sys::Window, coarse_pointer: bool) -> bool {
    let visual = window.visual_viewport().map(|vv| vv.width());
    // `innerWidth` always exists in a browser; if it cannot be read, assume a desktop window
    // (V1's server snapshot was the desktop layout too).
    let inner = window
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(f64::INFINITY);
    mobile_layout_for(visual, inner, coarse_pointer)
}

struct ViewportWatcher {
    mobile: ArcRwSignal<bool>,
    changed: Emitter<()>,
    coarse_query: Option<web_sys::MediaQueryList>,
    _listener: Option<Closure<dyn FnMut()>>,
}

thread_local! {
    static WATCHER: OnceCell<ViewportWatcher> = const { OnceCell::new() };
}

impl ViewportWatcher {
    fn install() -> Self {
        let window = web_sys::window();
        let coarse_query = window
            .as_ref()
            .and_then(|w| w.match_media(COARSE_POINTER_QUERY).ok().flatten());
        let initial = match &window {
            Some(w) => snapshot_with(w, coarse_query.as_ref().is_some_and(|q| q.matches())),
            None => false,
        };
        let mut watcher = Self {
            mobile: ArcRwSignal::new(initial),
            changed: Emitter::new(),
            coarse_query,
            _listener: None,
        };
        if let Some(window) = window {
            let listener = Closure::<dyn FnMut()>::new(on_viewport_event);
            let callback: &js_sys::Function = listener.as_ref().unchecked_ref();
            let _ = window.add_event_listener_with_callback("resize", callback);
            if let Some(visual) = window.visual_viewport() {
                let _ = visual.add_event_listener_with_callback("resize", callback);
            }
            if let Some(query) = &watcher.coarse_query {
                if query
                    .add_event_listener_with_callback("change", callback)
                    .is_err()
                {
                    // Safari < 14 only has the deprecated `addListener`.
                    let _ = query.add_listener_with_opt_callback(Some(callback));
                }
            }
            watcher._listener = Some(listener);
        }
        watcher
    }

    fn refresh(&self) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let coarse = self.coarse_query.as_ref().is_some_and(|q| q.matches());
        let mobile = snapshot_with(&window, coarse);
        if self.mobile.get_untracked() != mobile {
            self.mobile.set(mobile);
        }
    }
}

fn on_viewport_event() {
    WATCHER.with(|cell| {
        if let Some(watcher) = cell.get() {
            watcher.refresh();
            watcher.changed.emit(&());
        }
    });
}

fn with_watcher<R>(f: impl FnOnce(&ViewportWatcher) -> R) -> R {
    WATCHER.with(|cell| f(cell.get_or_init(ViewportWatcher::install)))
}

/// V1 `useIsMobileViewport()`: true while the mobile layout applies. The signal only changes
/// when the boolean flips (crossing the breakpoint remounts the layout tree in V1, D17).
pub fn use_is_mobile_viewport() -> Signal<bool> {
    with_watcher(|w| Signal::from(w.mobile.clone()))
}

/// Untracked read of the watched value (installs the watcher on first use).
pub fn is_mobile_viewport_now() -> bool {
    with_watcher(|w| w.mobile.get_untracked())
}

/// Handle returned by [`subscribe_viewport_state`]; call [`ViewportSubscription::remove`] to
/// stop listening.
#[derive(Debug)]
#[must_use = "call .remove() to stop listening"]
pub struct ViewportSubscription(ConnectionId);

impl ViewportSubscription {
    /// Disconnects the handler.
    pub fn remove(self) {
        WATCHER.with(|cell| {
            if let Some(watcher) = cell.get() {
                watcher.changed.disconnect(self.0);
            }
        });
    }
}

/// V1 `subscribeViewportState(onStoreChange)`: `handler` runs on every `window` resize,
/// `visualViewport` resize and `(pointer: coarse)` change (after the mobile signal was
/// refreshed), whether or not the layout flipped.
pub fn subscribe_viewport_state(handler: impl Fn() + 'static) -> ViewportSubscription {
    with_watcher(|w| ViewportSubscription(w.changed.connect(move |_: &()| handler())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_viewport_takes_precedence_over_inner_width() {
        // `??` semantics: only a missing visual viewport falls back to innerWidth.
        assert!(
            mobile_layout_for(Some(0.0), 1920.0, false),
            "visual width 0 counts as mobile"
        );
        assert!(
            mobile_layout_for(Some(800.0), 1920.0, false),
            "pinch-zoom quirk"
        );
        assert!(!mobile_layout_for(Some(1366.0), 800.0, false));
        assert!(mobile_layout_for(None, 900.0, false));
        assert!(!mobile_layout_for(None, 900.5, false));
    }

    #[test]
    fn breakpoints_match_v1() {
        for (width, coarse, expected) in [
            (390.0, true, true),    // phone portrait
            (932.0, true, true),    // phone landscape
            (1024.0, true, true),   // iPad portrait
            (1194.0, true, true),   // iPad landscape
            (1200.0, true, true),   // inclusive bound
            (1200.5, true, false),  //
            (1366.0, true, false),  // iPad Pro 12.9" landscape → desktop
            (900.0, false, true),   // desktop window at the breakpoint
            (901.0, false, false),  //
            (1920.0, false, false), //
        ] {
            assert_eq!(
                mobile_layout_for(None, width, coarse),
                expected,
                "{width} coarse={coarse}"
            );
            assert_eq!(
                mobile_layout_for(Some(width), 0.0, coarse),
                expected,
                "{width} coarse={coarse}"
            );
        }
    }
}

//! Runtime environment: base URL resolution, Tauri detection, clocks and small DOM helpers that
//! several UI modules share. OWNED BY: ui-foundation.
//!
//! Every function degrades gracefully outside a browser window (no panic): URLs fall back to
//! the root base, clocks to `Date.now()`, DOM predicates to `false`.

use std::cell::OnceCell;

use wasm_bindgen::{JsCast, JsValue};

thread_local! {
    static ROUTER_BASE: OnceCell<String> = const { OnceCell::new() };
}

/// Router base path derived from `document.baseURI` without trailing slash
/// (`"/amadeus"` on the web build, `""` on desktop). Computed once per page load (Trunk's
/// `<base href>` never changes at runtime).
pub fn router_base() -> String {
    ROUTER_BASE.with(|cell| cell.get_or_init(compute_router_base).clone())
}

fn compute_router_base() -> String {
    let base = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.base_uri().ok().flatten())
        .and_then(|u| web_sys::Url::new(&u).ok())
        .map(|u| u.pathname())
        .unwrap_or_else(|| "/".to_owned());
    base.trim_end_matches('/').to_owned()
}

/// Resolve a base-relative path (e.g. `"resources/fonts/x.otf"`) to an absolute URL string.
pub fn asset_url(rel: &str) -> String {
    format!("{}/{}", router_base(), rel.trim_start_matches('/'))
}

/// Resolve an API path from [`amadeus_core::protocol::paths`] (e.g. `"app/api/chat"`).
pub fn api_url(rel: &str) -> String {
    asset_url(rel)
}

/// True when running inside the Tauri desktop webview (`window.__TAURI__` exists).
pub fn is_tauri() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| js_sys::Reflect::get(&w, &"__TAURI__".into()).ok())
            .map(|v| !v.is_undefined() && !v.is_null())
            .unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

/// `Date.now()`: wall-clock milliseconds since the Unix epoch.
pub fn now_ms() -> f64 {
    js_sys::Date::now()
}

/// `performance.now()`: monotonic milliseconds since page load (falls back to `Date.now()`).
pub fn perf_now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or_else(now_ms)
}

/// `new Date().toISOString()` (UTC, e.g. `"2026-11-16T12:34:56.789Z"`), the timestamp V1 writes
/// into conversation exports (see `amadeus_core::types::conversation_export_file_name`).
pub fn now_iso() -> String {
    js_sys::Date::new_0().to_iso_string().into()
}

/// `Math.random()` in `[0, 1)`.
pub fn random() -> f64 {
    js_sys::Math::random()
}

/// `(window.innerWidth, window.innerHeight)` in CSS px; `(0, 0)` without a window.
pub fn inner_size() -> (f64, f64) {
    let Some(window) = web_sys::window() else {
        return (0.0, 0.0);
    };
    let read = |v: Result<JsValue, JsValue>| v.ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
    (read(window.inner_width()), read(window.inner_height()))
}

/// V1 `isTextInputTarget(event.target)`: true for an `HTMLElement` whose tag is `INPUT` (any
/// type, checkboxes included), `TEXTAREA` or `SELECT`, or which is `contentEditable`. Every
/// other target (SVG elements, `window`, `document`, `None`) is false.
pub fn is_text_input_target(target: Option<web_sys::EventTarget>) -> bool {
    let Some(element) = target.and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) else {
        return false;
    };
    is_text_input_tag(&element.tag_name()) || element.is_content_editable()
}

/// The tag-name half of [`is_text_input_target`] (`tagName` is upper-case for HTML elements).
pub fn is_text_input_tag(tag_name: &str) -> bool {
    matches!(tag_name, "INPUT" | "TEXTAREA" | "SELECT")
}

/// V1 `n.toLocaleString()`: the number formatted for the browser's default locale (NOT the app
/// language), e.g. `65000` → `"65,000"` in `en-US`. Used by the FreeAPI quota display.
pub fn to_locale_string(n: f64) -> String {
    let format = js_sys::Intl::NumberFormat::new(&js_sys::Array::new(), &js_sys::Object::new());
    format
        .format()
        .call1(&JsValue::UNDEFINED, &JsValue::from_f64(n))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| n.to_string())
}

/// JavaScript `String(value)`: strings as-is, errors as `"Name: message"`. V1 builds several
/// user-visible status texts with `String(error)` (export/import failures).
pub fn js_to_string(value: &JsValue) -> String {
    if let Some(s) = value.as_string() {
        return s;
    }
    js_sys::Reflect::get(&js_sys::global(), &"String".into())
        .ok()
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
        .and_then(|f| f.call1(&JsValue::UNDEFINED, value).ok())
        .and_then(|s| s.as_string())
        .unwrap_or_else(|| format!("{value:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_input_tags_match_v1() {
        for tag in ["INPUT", "TEXTAREA", "SELECT"] {
            assert!(is_text_input_tag(tag), "{tag}");
        }
        for tag in ["BUTTON", "DIV", "input", "OPTION", "svg", ""] {
            assert!(!is_text_input_tag(tag), "{tag}");
        }
    }
}

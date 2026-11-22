//! Loads `live2dcubismcore.min.js` once and waits until the Core answers (D7, pipeline spec §3.2–3.3).
//!
//! The Core script instantiates its embedded WASM asynchronously, so `Live2DCubismCore` exists
//! before it is callable: after the script's `load` event the Core is polled
//! (`Version.csmGetVersion()` through a catching binding) every few milliseconds, for at most
//! 10 s. Concurrent callers share one load (one JS promise); a failed load clears it, so a later
//! call retries (a failed `<script>` tag is removed first — V1's tag that already failed never
//! settled). The tag carries V1's marker `data-core="true"` and `data-loaded="true"` once loaded,
//! so a page never loads the Core twice.

use std::cell::RefCell;

use js_sys::Promise;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{future_to_promise, JsFuture};
use web_sys::{AddEventListenerOptions, Element, HtmlScriptElement};

use super::define::{CORE_READY_POLL_MS, CORE_READY_TIMEOUT_MS, CORE_SCRIPT_MARKER_ATTR};
use crate::core::ffi;

thread_local! {
    static CORE_LOAD: RefCell<Option<Promise>> = const { RefCell::new(None) };
}

fn js_error(message: &str) -> JsValue {
    js_sys::Error::new(message).into()
}

/// Ensure the Cubism Core script is loaded and its WASM is ready. `core_script_url` is resolved
/// against `document.baseURI` (e.g. `"resources/live2d/live2dcubismcore.min.js"` or an absolute
/// path). Concurrent callers share one load; a failed load can be retried.
pub async fn ensure_core_loaded(core_script_url: &str) -> Result<(), JsValue> {
    if ffi::core_available() {
        ffi::register_core_log_handler();
        return Ok(());
    }
    let promise = CORE_LOAD.with(|slot| {
        let mut slot = slot.borrow_mut();
        if let Some(existing) = slot.as_ref() {
            return existing.clone();
        }
        let url = core_script_url.to_owned();
        let promise =
            future_to_promise(async move { load_and_wait(&url).await.map(|_| JsValue::UNDEFINED) });
        *slot = Some(promise.clone());
        promise
    });
    let result = JsFuture::from(promise.clone()).await;
    if result.is_err() {
        CORE_LOAD.with(|slot| {
            let mut slot = slot.borrow_mut();
            if slot
                .as_ref()
                .is_some_and(|p| js_sys::Object::is(p.as_ref(), promise.as_ref()))
            {
                *slot = None;
            }
        });
    } else {
        ffi::register_core_log_handler();
    }
    result.map(|_| ())
}

/// Whether the global `Live2DCubismCore` namespace exists (its script has run).
fn core_namespace_present() -> bool {
    web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &JsValue::from_str("Live2DCubismCore")).ok())
        .is_some_and(|v| !v.is_undefined() && !v.is_null())
}

async fn load_and_wait(url: &str) -> Result<(), JsValue> {
    if !core_namespace_present() {
        load_script(url).await?;
    }
    wait_until_ready().await
}

/// `loadScript(src, "core")` of V1's Live2DCanvas, with the never-settling failed tag fixed.
async fn load_script(url: &str) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| js_error("no window"))?;
    let document = window.document().ok_or_else(|| js_error("no document"))?;
    let selector = format!("script[{CORE_SCRIPT_MARKER_ATTR}=\"true\"]");
    let existing = document.query_selector(&selector).ok().flatten();
    let script: Element = match existing {
        Some(tag) if tag.get_attribute("data-loaded").as_deref() == Some("true") => return Ok(()),
        Some(tag) if tag.get_attribute("data-failed").as_deref() == Some("true") => {
            tag.remove();
            create_script(&document, url)?
        }
        Some(tag) => tag,
        None => create_script(&document, url)?,
    };
    let source = script
        .get_attribute("src")
        .unwrap_or_else(|| url.to_owned());
    let settled = Promise::new(&mut |resolve, reject| {
        let options = AddEventListenerOptions::new();
        options.set_once(true);
        let loaded_tag = script.clone();
        let on_load = Closure::once_into_js(move |_event: JsValue| {
            let _ = loaded_tag.set_attribute("data-loaded", "true");
            let _ = resolve.call0(&JsValue::NULL);
        });
        let failed_tag = script.clone();
        let failed_source = source.clone();
        let on_error = Closure::once_into_js(move |_event: JsValue| {
            let _ = failed_tag.set_attribute("data-failed", "true");
            let _ = reject.call1(
                &JsValue::NULL,
                &js_error(&format!("Failed to load {failed_source}")),
            );
        });
        let _ = script.add_event_listener_with_callback_and_add_event_listener_options(
            "load",
            on_load.unchecked_ref(),
            &options,
        );
        let _ = script.add_event_listener_with_callback_and_add_event_listener_options(
            "error",
            on_error.unchecked_ref(),
            &options,
        );
    });
    if script.parent_node().is_none() {
        let parent: web_sys::Node = match document.head() {
            Some(head) => head.into(),
            None => document
                .document_element()
                .ok_or_else(|| js_error("no document element"))?
                .into(),
        };
        parent.append_child(&script)?;
    }
    JsFuture::from(settled).await.map(|_| ())
}

/// `<script src async data-core="true">` (not yet attached).
fn create_script(document: &web_sys::Document, url: &str) -> Result<Element, JsValue> {
    let element = document.create_element("script")?;
    if let Some(script) = element.dyn_ref::<HtmlScriptElement>() {
        script.set_src(url);
        script.set_async(true);
    } else {
        element.set_attribute("src", url)?;
        element.set_attribute("async", "")?;
    }
    element.set_attribute(CORE_SCRIPT_MARKER_ATTR, "true")?;
    Ok(element)
}

async fn sleep_ms(ms: i32) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| js_error("no window"))?;
    let promise = Promise::new(&mut |resolve, _reject| {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
    });
    JsFuture::from(promise).await.map(|_| ())
}

/// Polls the Core until `csmGetVersion()` answers (its WASM initializes asynchronously).
async fn wait_until_ready() -> Result<(), JsValue> {
    let start = js_sys::Date::now();
    loop {
        if ffi::core_available() {
            return Ok(());
        }
        if js_sys::Date::now() - start > CORE_READY_TIMEOUT_MS {
            return Err(js_error(
                "Live2DCubismCore did not become ready within 10 s (csmGetVersion unavailable)",
            ));
        }
        sleep_ms(CORE_READY_POLL_MS).await?;
    }
}

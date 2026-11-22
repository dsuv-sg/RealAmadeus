//! Page-global asset caches — V1 `cachedFetchArrayBuffer` / `cachedLoadImage` (pipeline spec §6.1).
//!
//! * One map per kind keyed by the **resolved** URL (so `resources/x` and `/amadeus/resources/x`
//!   share an entry); the cached value is the JS **promise**, so concurrent callers share one
//!   request and every consumer receives the same `ArrayBuffer` / `HTMLImageElement`.
//! * The request is issued when the function is called (not when the returned future is first
//!   polled), so prefetching is just calling and dropping.
//! * Non-2xx rejects with `Error("Failed to fetch <url>: <status>")`; a network error rejects with
//!   the fetch error. A rejected entry evicts itself inside the promise chain (the next call
//!   retries); successful entries live for the page lifetime (a remount needs no network).
//! * Images: `new Image()`, `decoding = "async"`, no `crossOrigin`, resolve after `load` +
//!   `decode()` (the decode result is ignored; V2 stops waiting for it after
//!   [`IMAGE_DECODE_WAIT_MS`]), reject with the `error` event.
//!
//! The shaders V1 also fetched through `cachedFetchText` are embedded in V2 (D4).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;

use js_sys::{Array, Function, Promise, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{future_to_promise, JsFuture};
use web_sys::{AddEventListenerOptions, HtmlImageElement, Response};

use super::define::IMAGE_DECODE_WAIT_MS;
use super::url::resolve;

struct Entry {
    generation: u64,
    promise: Promise,
}

#[derive(Clone, Copy)]
enum Kind {
    Bytes,
    Image,
}

thread_local! {
    static BYTES: RefCell<HashMap<String, Entry>> = RefCell::new(HashMap::new());
    static IMAGES: RefCell<HashMap<String, Entry>> = RefCell::new(HashMap::new());
    static NEXT_GENERATION: Cell<u64> = const { Cell::new(1) };
}

fn with_map<R>(kind: Kind, f: impl FnOnce(&mut HashMap<String, Entry>) -> R) -> R {
    match kind {
        Kind::Bytes => BYTES.with(|m| f(&mut m.borrow_mut())),
        Kind::Image => IMAGES.with(|m| f(&mut m.borrow_mut())),
    }
}

/// The cached promise for `key`, or a new one from `make(key, generation)`.
fn cached(kind: Kind, key: String, make: impl FnOnce(String, u64) -> Promise) -> Promise {
    if let Some(promise) = with_map(kind, |m| m.get(&key).map(|e| e.promise.clone())) {
        return promise;
    }
    let generation = NEXT_GENERATION.with(|g| {
        let v = g.get();
        g.set(v.wrapping_add(1));
        v
    });
    let promise = make(key.clone(), generation);
    with_map(kind, |m| {
        m.insert(
            key,
            Entry {
                generation,
                promise: promise.clone(),
            },
        )
    });
    promise
}

/// Removes the entry if it is still the one created with `generation`.
fn evict(kind: Kind, key: &str, generation: u64) {
    with_map(kind, |m| {
        if m.get(key).is_some_and(|e| e.generation == generation) {
            m.remove(key);
        }
    });
}

fn js_error(message: &str) -> JsValue {
    js_sys::Error::new(message).into()
}

fn window() -> Result<web_sys::Window, JsValue> {
    web_sys::window().ok_or_else(|| js_error("no window"))
}

async fn fetch_response(url: &str) -> Result<Response, JsValue> {
    let response = JsFuture::from(window()?.fetch_with_str(url)).await?;
    response.dyn_into::<Response>()
}

async fn fetch_array_buffer(url: &str) -> Result<JsValue, JsValue> {
    let response = fetch_response(url).await?;
    if !response.ok() {
        return Err(js_error(&format!(
            "Failed to fetch {url}: {}",
            response.status()
        )));
    }
    JsFuture::from(response.array_buffer()?).await
}

fn request_bytes(url: &str) -> Promise {
    cached(Kind::Bytes, resolve(url), |key, generation| {
        future_to_promise(async move {
            let result = fetch_array_buffer(&key).await;
            if result.is_err() {
                evict(Kind::Bytes, &key, generation);
            }
            result
        })
    })
}

fn array_buffer_to_vec(value: &JsValue) -> Vec<u8> {
    Uint8Array::new(value).to_vec()
}

/// `cachedFetchArrayBuffer(url)`: the shared (cached) request, copied into WASM memory.
pub fn fetch_bytes(url: &str) -> impl Future<Output = Result<Vec<u8>, JsValue>> + 'static {
    let promise = request_bytes(url);
    async move {
        let value = JsFuture::from(promise).await?;
        Ok(array_buffer_to_vec(&value))
    }
}

/// Issues (or joins) the cached request and returns at once (V1's `.catch(() => {})` prefetch).
pub fn prefetch_bytes(url: &str) {
    let _ = request_bytes(url);
}

/// Starts (or joins) the cached request without copying the bytes (preloading).
pub fn warm_bytes(url: &str) -> impl Future<Output = Result<(), JsValue>> + 'static {
    let promise = request_bytes(url);
    async move { JsFuture::from(promise).await.map(|_| ()) }
}

/// Uncached `fetch` with V1's motion/pose/user-data handling: `ok` → bytes; `status >= 400` →
/// empty (V1 logged a suppressed `CubismLogError` and continued with an empty buffer). Any other
/// outcome (network error, odd status) also yields an empty buffer here — V1 crashed on
/// `undefined.byteLength` and stalled the load chain (fixed, D18); the error is returned for logging.
pub async fn fetch_plain(url: &str) -> (Vec<u8>, Option<JsValue>) {
    let url = resolve(url);
    match fetch_response(&url).await {
        Ok(response) if response.ok() => match response.array_buffer() {
            Ok(promise) => match JsFuture::from(promise).await {
                Ok(buffer) => (array_buffer_to_vec(&buffer), None),
                Err(e) => (Vec::new(), Some(e)),
            },
            Err(e) => (Vec::new(), Some(e)),
        },
        Ok(response) if response.status() >= 400 => (Vec::new(), None),
        Ok(response) => (
            Vec::new(),
            Some(js_error(&format!(
                "Failed to fetch {url}: {}",
                response.status()
            ))),
        ),
        Err(e) => (Vec::new(), Some(e)),
    }
}

fn image_promise(url: String, generation: u64) -> Promise {
    future_to_promise(async move {
        let result = load_image_uncached(&url).await;
        if result.is_err() {
            evict(Kind::Image, &url, generation);
        }
        result.map(JsValue::from)
    })
}

async fn load_image_uncached(url: &str) -> Result<HtmlImageElement, JsValue> {
    let image = HtmlImageElement::new()?;
    image.set_decoding("async");
    let target = image.clone();
    let loaded = Promise::new(&mut |resolve, reject| {
        let options = AddEventListenerOptions::new();
        options.set_once(true);
        options.set_passive(true);
        let on_load = Closure::once_into_js(move |_event: JsValue| {
            let _ = resolve.call0(&JsValue::NULL);
        });
        let on_error = Closure::once_into_js(move |event: JsValue| {
            let _ = reject.call1(&JsValue::NULL, &event);
        });
        let _ = target.add_event_listener_with_callback_and_add_event_listener_options(
            "load",
            on_load.unchecked_ref(),
            &options,
        );
        let _ = target.add_event_listener_with_callback_and_add_event_listener_options(
            "error",
            on_error.unchecked_ref(),
            &options,
        );
    });
    image.set_src(url);
    JsFuture::from(loaded).await?;
    // `if (typeof img.decode === "function") img.decode().then(resolve, resolve)`, raced against a
    // generous timer: a decode that never settles (headless Chrome's virtual time with a
    // presenting WebGL canvas, pathologically slow devices) falls back to the synchronous decode
    // inside `texImage2D` — the uploaded pixels are identical.
    if let Some(decode) = Reflect::get(&image, &JsValue::from_str("decode"))
        .ok()
        .and_then(|f| f.dyn_into::<Function>().ok())
    {
        if let Ok(promise) = decode.call0(&image).and_then(|p| p.dyn_into::<Promise>()) {
            let race = Promise::race(&Array::of2(&promise, &timer(IMAGE_DECODE_WAIT_MS)));
            let _ = JsFuture::from(race).await;
        }
    }
    Ok(image)
}

/// A promise resolved after `ms` milliseconds (`setTimeout`).
fn timer(ms: i32) -> Promise {
    Promise::new(&mut |resolve, _reject| {
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
        }
    })
}

fn request_image(url: &str) -> Promise {
    cached(Kind::Image, resolve(url), image_promise)
}

/// `cachedLoadImage(url)`: the shared decoded image.
pub fn load_image(url: &str) -> impl Future<Output = Result<HtmlImageElement, JsValue>> + 'static {
    let promise = request_image(url);
    async move {
        let value = JsFuture::from(promise).await?;
        value.dyn_into::<HtmlImageElement>()
    }
}

/// Issues (or joins) the cached image load and returns at once.
pub fn prefetch_image(url: &str) {
    let _ = request_image(url);
}

/// Starts (or joins) the cached image load (preloading).
pub fn warm_image(url: &str) -> impl Future<Output = Result<(), JsValue>> + 'static {
    let promise = request_image(url);
    async move { JsFuture::from(promise).await.map(|_| ()) }
}

/// Whether a successful or in-flight entry exists for `url` (diagnostics/tests).
pub fn is_cached(url: &str) -> bool {
    let key = resolve(url);
    with_map(Kind::Bytes, |m| m.contains_key(&key))
        || with_map(Kind::Image, |m| m.contains_key(&key))
}

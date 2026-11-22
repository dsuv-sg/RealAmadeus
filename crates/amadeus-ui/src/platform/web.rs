//! Web implementations of the platform shell (DOM APIs, exactly as V1 `RealAmadeusApp.tsx`).

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use futures::channel::oneshot;
use futures::future::{self, FutureExt, LocalBoxFuture};
use js_sys::{Array, Function, Reflect};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, HtmlInputElement, Url};

use crate::env::js_to_string;

/// Where V1's SHUT DOWN ends up when the tab cannot be closed (`RealAmadeusApp.tsx` L4698).
pub const SHUTDOWN_REDIRECT_URL: &str = "https://dsuv.elvelt.com/amadeus";

fn document() -> Option<web_sys::Document> {
    web_sys::window().and_then(|w| w.document())
}

/// `!!document.fullscreenElement` (unprefixed API only, like V1).
pub fn is_fullscreen() -> bool {
    document().and_then(|d| d.fullscreen_element()).is_some()
}

/// Calls `target[name]()` if it is a function; a returned Promise is awaited and its rejection
/// swallowed (V1 `.catch(() => {})`). Missing APIs (iPhone Safari has no `requestFullscreen`)
/// are a no-op instead of V1's synchronous TypeError.
fn call_and_swallow(target: &JsValue, name: &str) {
    let Some(function) = Reflect::get(target, &name.into())
        .ok()
        .and_then(|f| f.dyn_into::<Function>().ok())
    else {
        return;
    };
    if let Ok(result) = function.call0(target) {
        if let Ok(promise) = result.dyn_into::<js_sys::Promise>() {
            spawn_local(async move {
                let _ = JsFuture::from(promise).await;
            });
        }
    }
}

/// V1: `if (!document.fullscreenElement) document.documentElement.requestFullscreen().catch(() => {})`.
pub fn request_fullscreen() {
    if is_fullscreen() {
        return;
    }
    if let Some(root) = document().and_then(|d| d.document_element()) {
        call_and_swallow(&root, "requestFullscreen");
    }
}

/// V1: `if (document.fullscreenElement) document.exitFullscreen().catch(() => {})`.
pub fn exit_fullscreen() {
    if !is_fullscreen() {
        return;
    }
    if let Some(document) = document() {
        call_and_swallow(&document, "exitFullscreen");
    }
}

/// `document.addEventListener("fullscreenchange", listener)`.
pub fn add_fullscreen_listener(listener: &Function) {
    if let Some(document) = document() {
        let _ = document.add_event_listener_with_callback("fullscreenchange", listener);
    }
}

/// V1 conversation export download: `Blob([contents], { type: mime })` → object URL →
/// temporary `<a download=file_name>` appended to `<body>`, clicked, removed → URL revoked.
pub fn save_text_file(file_name: &str, mime: &str, contents: &str) -> Result<(), String> {
    let document = document().ok_or("document is not available")?;
    let body = document.body().ok_or("document.body is not available")?;
    let options = BlobPropertyBag::new();
    options.set_type(mime);
    let parts = Array::of1(&JsValue::from_str(contents));
    let blob =
        Blob::new_with_str_sequence_and_options(&parts, &options).map_err(|e| js_to_string(&e))?;
    let url = Url::create_object_url_with_blob(&blob).map_err(|e| js_to_string(&e))?;
    let result = document
        .create_element("a")
        .map_err(|e| js_to_string(&e))
        .and_then(|el| {
            el.dyn_into::<HtmlAnchorElement>()
                .map_err(|_| "cannot create <a>".to_owned())
        })
        .and_then(|anchor| {
            anchor.set_href(&url);
            anchor.set_download(file_name);
            body.append_child(&anchor).map_err(|e| js_to_string(&e))?;
            anchor.click();
            let _ = body.remove_child(&anchor);
            Ok(())
        });
    let _ = Url::revoke_object_url(&url);
    result
}

/// V1 `file.text()`: the file decoded as UTF-8 (a BOM is stripped, invalid bytes → U+FFFD).
pub async fn read_file_text(file: &web_sys::File) -> Result<String, String> {
    let text = JsFuture::from(file.text())
        .await
        .map_err(|e| js_to_string(&e))?;
    text.as_string()
        .ok_or_else(|| "file.text() did not return a string".to_owned())
}

type PickResult = Result<Option<String>, String>;

/// One open file dialog: a hidden `<input type="file">` and its `change`/`cancel` listener.
struct Picker {
    input: HtmlInputElement,
    sender: Option<oneshot::Sender<PickResult>>,
    listener: Option<Closure<dyn FnMut(web_sys::Event)>>,
}

thread_local! {
    static PENDING_PICK: RefCell<Option<Rc<RefCell<Picker>>>> = const { RefCell::new(None) };
}

/// Detaches the picker's input and listener; returns the sender if nobody was answered yet.
fn finish_pick(picker: &Rc<RefCell<Picker>>) -> Option<oneshot::Sender<PickResult>> {
    let (input, sender, listener) = {
        let mut state = picker.borrow_mut();
        (
            state.input.clone(),
            state.sender.take(),
            state.listener.take(),
        )
    };
    if let Some(listener) = listener {
        let callback: &Function = listener.as_ref().unchecked_ref();
        let _ = input.remove_event_listener_with_callback("change", callback);
        let _ = input.remove_event_listener_with_callback("cancel", callback);
        // The listener may be running right now: release it on a later tick.
        spawn_local(async move { drop(listener) });
    }
    input.remove();
    PENDING_PICK.with(|pending| {
        let mut pending = pending.borrow_mut();
        if pending
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, picker))
        {
            *pending = None;
        }
    });
    sender
}

/// Opens the browser file dialog through a hidden `<input type="file" accept=…>` (V1's
/// `config-import-input`) and resolves to the chosen file's text, `Ok(None)` when the dialog
/// was cancelled (browsers with the `cancel` event) or superseded by a newer dialog.
///
/// The dialog opens synchronously, inside this call: call it directly from the click handler
/// (user activation), then await the returned future.
pub fn pick_text_file(accept: &str) -> LocalBoxFuture<'static, PickResult> {
    if let Some(previous) = PENDING_PICK.with(|pending| pending.borrow_mut().take()) {
        if let Some(sender) = finish_pick(&previous) {
            let _ = sender.send(Ok(None));
        }
    }
    let Some(document) = document() else {
        return future::ready(Err("document is not available".to_owned())).boxed_local();
    };
    let input = match document
        .create_element("input")
        .map(|el| el.dyn_into::<HtmlInputElement>())
    {
        Ok(Ok(input)) => input,
        _ => {
            return future::ready(Err("cannot create <input type=\"file\">".to_owned()))
                .boxed_local()
        }
    };
    input.set_type("file");
    input.set_accept(accept);
    let _ = input.style().set_property("display", "none");

    let (sender, receiver) = oneshot::channel::<PickResult>();
    let picker = Rc::new(RefCell::new(Picker {
        input: input.clone(),
        sender: Some(sender),
        listener: None,
    }));
    // The closure owns the picker (a cycle) until `finish_pick` takes the closure out.
    let handle = picker.clone();
    let listener = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
        let file = if event.type_() == "change" {
            handle.borrow().input.files().and_then(|files| files.get(0))
        } else {
            None
        };
        let Some(sender) = finish_pick(&handle) else {
            return;
        };
        match file {
            None => {
                let _ = sender.send(Ok(None));
            }
            Some(file) => spawn_local(async move {
                let _ = sender.send(read_file_text(&file).await.map(Some));
            }),
        }
    });
    {
        let callback: &Function = listener.as_ref().unchecked_ref();
        let _ = input.add_event_listener_with_callback("change", callback);
        let _ = input.add_event_listener_with_callback("cancel", callback);
    }
    picker.borrow_mut().listener = Some(listener);
    PENDING_PICK.with(|pending| *pending.borrow_mut() = Some(picker));
    if let Some(body) = document.body() {
        let _ = body.append_child(&input);
    }
    input.click();
    async move { receiver.await.unwrap_or(Ok(None)) }.boxed_local()
}

/// V1 SHUT DOWN (after the app closed its overlays): `window.close()`; 100 ms later
/// `history.back()` when there is history; 300 ms after that a hard navigation to
/// [`SHUTDOWN_REDIRECT_URL`].
pub fn shutdown() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let _ = window.close();
    leptos::prelude::set_timeout(
        move || {
            if let Some(window) = web_sys::window() {
                if let Ok(history) = window.history() {
                    if history.length().unwrap_or(0) > 1 {
                        let _ = history.back();
                    }
                }
            }
            leptos::prelude::set_timeout(
                || {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href(SHUTDOWN_REDIRECT_URL);
                    }
                },
                Duration::from_millis(300),
            );
        },
        Duration::from_millis(100),
    );
}

/// `window.open(url, "_blank", "noopener,noreferrer")` (the pages' `<a target="_blank"
/// rel="noopener noreferrer">` semantics).
pub fn open_external(url: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.open_with_url_and_target_and_features(url, "_blank", "noopener,noreferrer");
    }
}

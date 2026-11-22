//! Desktop shell access through `window.__TAURI__` (Tauri 2, `app.withGlobalTauri = true`).
//!
//! Everything is looked up at call time with `js_sys::Reflect`, never through static
//! `#[wasm_bindgen]` imports, so the web build never references a missing global: on the web
//! these functions return `Err` instead of throwing. Command names, argument shapes and the
//! fullscreen event are those of `docs/DESKTOP_IPC.md` §3–§4. The backend commands (§2) are
//! the transport's business; [`invoke`] is public so it can share this lookup.

use js_sys::{Array, Function, Object, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

/// Platform command names (DESKTOP_IPC §3).
pub mod commands {
    /// `{ fullscreen: boolean }` → `null`.
    pub const WINDOW_SET_FULLSCREEN: &str = "window_set_fullscreen";
    /// `{}` → `boolean`.
    pub const WINDOW_IS_FULLSCREEN: &str = "window_is_fullscreen";
    /// `{}` → `null` (if it settles at all).
    pub const APP_QUIT: &str = "app_quit";
    /// `{ url }` → `null`; http(s) only.
    pub const OPEN_EXTERNAL: &str = "open_external";
    /// `{ suggestedName, contents, mime? }` → `{ saved: true, path }` | `{ saved: false }`.
    pub const SAVE_TEXT_FILE: &str = "save_text_file";
    /// `{ extensions? }` → `string` | `null`.
    pub const OPEN_TEXT_FILE: &str = "open_text_file";
}

/// Event emitted by the shell when native fullscreen changes; payload `{ fullscreen: boolean }`.
pub const FULLSCREEN_CHANGED_EVENT: &str = "amadeus://fullscreen-changed";

fn tauri_namespace(name: &str) -> Result<JsValue, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let tauri = Reflect::get(&window, &"__TAURI__".into())?;
    if tauri.is_undefined() || tauri.is_null() {
        return Err(JsValue::from_str("window.__TAURI__ is not available"));
    }
    let namespace = Reflect::get(&tauri, &name.into())?;
    if namespace.is_undefined() || namespace.is_null() {
        return Err(JsValue::from_str(&format!(
            "window.__TAURI__.{name} is not available"
        )));
    }
    Ok(namespace)
}

fn method(target: &JsValue, name: &str) -> Result<Function, JsValue> {
    Reflect::get(target, &name.into())?
        .dyn_into::<Function>()
        .map_err(|_| JsValue::from_str(&format!("{name} is not a function")))
}

/// `window.__TAURI__.core.invoke(cmd, args)`: the returned Promise. The call is made
/// synchronously; await it with [`JsFuture`] (or use [`invoke_async`]).
pub fn invoke(cmd: &str, args: &JsValue) -> Result<Promise, JsValue> {
    let core = tauri_namespace("core")?;
    method(&core, "invoke")?
        .call2(&core, &JsValue::from_str(cmd), args)?
        .dyn_into::<Promise>()
}

/// [`invoke`] and await the result. A rejection (platform commands reject with a message
/// string) becomes `Err`.
pub async fn invoke_async(cmd: &str, args: &JsValue) -> Result<JsValue, JsValue> {
    JsFuture::from(invoke(cmd, args)?).await
}

/// `window.__TAURI__.event.listen(event, handler)`: resolves to the unlisten function.
pub fn listen(event: &str, handler: &Function) -> Result<Promise, JsValue> {
    let events = tauri_namespace("event")?;
    method(&events, "listen")?
        .call2(&events, &JsValue::from_str(event), handler)?
        .dyn_into::<Promise>()
}

/// A plain JS object from `(key, value)` pairs (camelCase keys, DESKTOP_IPC §1).
pub fn args(pairs: &[(&str, JsValue)]) -> JsValue {
    let object = Object::new();
    for (key, value) in pairs {
        let _ = Reflect::set(&object, &JsValue::from_str(key), value);
    }
    object.into()
}

/// A JS array of strings.
pub fn string_array<'a>(items: impl IntoIterator<Item = &'a str>) -> JsValue {
    items
        .into_iter()
        .map(JsValue::from_str)
        .collect::<Array>()
        .into()
}

/// `payload.fullscreen` of an `amadeus://fullscreen-changed` event object.
pub fn fullscreen_from_event(event: &JsValue) -> Option<bool> {
    let payload = Reflect::get(event, &"payload".into()).ok()?;
    Reflect::get(&payload, &"fullscreen".into()).ok()?.as_bool()
}

/// Extensions for `open_text_file` from an HTML `accept` list (`".json"`, `"*.txt"`, `"md"`);
/// MIME entries (`"application/json"`, `"text/*"`) are skipped.
pub fn accept_to_extensions(accept: &str) -> Vec<String> {
    accept
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty() && !entry.contains('/'))
        .map(|entry| {
            entry
                .trim_start_matches('*')
                .trim_start_matches('.')
                .to_owned()
        })
        .filter(|ext| !ext.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accept_lists() {
        assert_eq!(accept_to_extensions(".json"), vec!["json".to_owned()]);
        assert_eq!(
            accept_to_extensions(" .json, *.txt ,md,application/json,,"),
            vec!["json", "txt", "md"]
        );
        assert!(accept_to_extensions("").is_empty());
        assert!(accept_to_extensions("text/*").is_empty());
    }
}

# Desktop IPC reference (Tauri 2)

How the shared WASM frontend talks to the desktop shell (`apps/desktop`, crate
`amadeus-desktop`). The backend command names, argument names and return types are fixed by
`amadeus_core::protocol::ipc`; the platform commands and the event are defined by the desktop shell
(`apps/desktop/src/lib.rs` carries the same tables). The web build uses HTTP for the backend calls
and DOM APIs for the platform features, so everything here is desktop-only.

## 1. Basics

* **Detection**: `window.__TAURI__` exists (`app.withGlobalTauri = true`), see
  `amadeus_ui::env`.
* **Calling**: `window.__TAURI__.core.invoke(command, args)` returns a Promise. Argument keys are
  **camelCase** on the JS side (`streamId`, `audioBase64`, `suggestedName`); Rust receives them as
  snake_case parameters.
* **Errors**:
  * Backend commands (§2) never reject. Every outcome, including internal failures, is an
    `ApiReply` (500 replies use `{"error":"Internal error (500): …"}`, so V1's error classifier
    shows its server-error text).
  * Platform commands (§3) reject with a message string when something fails. A cancelled
    dialog is not a failure.
  * A rejection with `Command … not allowed by ACL` means the call came from a non-app origin or
    used a plugin API that the capability does not grant (§6).
* **Origin**: the app runs at `tauri://localhost/app` (macOS/Linux) or `http://tauri.localhost/app`
  (Windows), with Trunk `public_url` `/`. Every page of the SPA is served from `index.html`.
* **Storage**: the desktop webview has its own localStorage origin. Keys and formats are the same
  as on the web, but web settings do not carry over.

## 2. Backend commands

`ApiReply` (serde `tag = "kind"`) is the transport-neutral HTTP response:

```jsonc
{ "kind": "json",   "status": 200, "body": { /* the V1 JSON body, verbatim */ } }
{ "kind": "binary", "status": 200, "contentType": "audio/wav", "data": "<standard base64>" }
{ "kind": "empty",  "status": 500 }   // V1 cases with no body
```

Map it to the UI's `JsonReply { status, ok: 200..=299, body }`: `json` → `Some(body)`, `empty` →
`None`, `binary` → audio (decode `data` with `amadeus_core::protocol::b64::decode`, wrap the bytes
in a `Blob` of type `contentType`, then `URL.createObjectURL`; do not `fetch()` a `data:` URL, since
the CSP blocks it, §5).

| command | JS args | resolves to | web equivalent |
|---|---|---|---|
| `chat` | `{ payload }` | `ApiReply` | `POST app/api/chat` |
| `chat_stream` | `{ payload, streamId: number, onEvent: Channel<StreamEvent> }` | `null` when the stream ended or was cancelled, `ApiReply` for a pre-stream error | `POST app/api/chat/stream` |
| `chat_stream_cancel` | `{ streamId: number }` | `null` | `AbortController.abort()` |
| `chat_quota` | `{}` | `ApiReply` (`{count,max,remaining}` from the hosted server) | `GET app/api/chat/quota` |
| `tts_synthesize` | `{ body: { serverUrl, speaker, text, useCache } }` | `ApiReply` (`binary` audio on success) | `POST app/api/tts/synthesize` |
| `tts_health` | `{ body: { serverUrl } }` | `ApiReply` | `POST app/api/tts/health` |
| `stt_transcribe` | `{ fields: { provider, language, serverUrl }, audioBase64: string, mime: string }` | `ApiReply` (400 `{"error":"Invalid form data"}` if `audioBase64` is not base64) | `POST app/api/stt/transcribe` (multipart) |
| `stt_health` | `{ body: { serverUrl } }` | `ApiReply` | `POST app/api/stt/health` |
| `rag_embed` | `{ body: { serverUrl, text } }` | `ApiReply` | `POST app/api/rag/embed` |
| `rag_health` | `{ body: { serverUrl } }` | `ApiReply` | `POST app/api/rag/health` |
| `app_info` | `{}` | `AppInfo` | none (desktop only) |

* `payload` is the JSON of `ChatRequestPayload`, exactly the body the web build POSTs. The desktop
  backend runs the same `amadeus-backend` code path as the web server, so statuses and bodies
  match, except that desktop has no rate limit, chat log or request counter.
* `stt_transcribe`: `audioBase64` is the recording (standard or URL-safe base64); `mime` is its
  content type, e.g. `audio/webm;codecs=opus`. An empty `mime` means unknown
  (`application/octet-stream`).
* `AppInfo` (camelCase):
  `{"version":"2.0.0","displayVersion":"V2.0.0R","platform":"windows|macos|linux","arch":"x86_64|aarch64","remoteApiBase":"https://dsuv.elvelt.com/amadeus"}`.

### Desktop semantics

* **Local servers are the user's.** TTS/STT/RAG `serverUrl`s such as `http://localhost:5100`
  are requested from the user's machine (on the web they hit the web server's localhost).
* **FreeAPI is remote.** Provider 8 (FreeAPI) needs server-side credentials, so the desktop
  backend forwards it, and `chat_quota`, to the hosted server `remoteApiBase` (default
  `https://dsuv.elvelt.com/amadeus`, overridable with the environment variable
  `AMADEUS_REMOTE_API_BASE` when the app starts).

### Streaming (`chat_stream`)

```js
const { invoke, Channel } = window.__TAURI__.core;
const streamId = ++streamCounter;            // new id for every stream, per page load
const onEvent = new Channel();
onEvent.onmessage = (ev) => {
  if (aborted) return;                       // ignore events queued before the cancel
  switch (ev.type) {
    case "token":    /* ev.token */ break;
    case "complete": /* ev.fullResponse; the stream is finished */ break;
    case "error":    /* ev.error; the stream is finished */ break;
  }
};
const reply = await invoke("chat_stream", { payload, streamId, onEvent });
if (reply !== null) { /* like a non-ok HTTP response: reply.status, reply.body?.error */ }

// cancel (Ctrl+C / cancel button):
aborted = true;
await invoke("chat_stream_cancel", { streamId });
```

* Events are the V1 SSE objects (`{"type":"token","token":…}`,
  `{"type":"complete","fullResponse":…}`, `{"type":"error","error":…}`). They arrive in order,
  one Channel message per event. The Channel already delivers whole events, so no SSE line parsing
  is needed. If the UI shares code with the web transport, feed it
  `"data: " + JSON.stringify(ev) + "\n\n"`.
* **Finish on the terminal event, not on the promise.** Messages of 8 KiB or more (typically a
  long `complete`) take an extra round trip, so they can arrive *after* the `chat_stream` promise
  resolves. Use the resolved value only for the pre-stream error reply and for cleanup.
* **Cancellation** aborts the backend task immediately (the provider connection is dropped). After
  a cancel no terminal event is sent and the promise resolves to `null`. Set your `aborted` flag
  before invoking the cancel (V1 parity: no events after cancel).
* `chat_stream_cancel` may be sent right after `chat_stream`: if it overtakes the
  `chat_stream` invoke, that stream is cancelled as soon as it starts. Cancels of finished streams
  are ignored.
* Use a new `streamId` for every stream (an incrementing counter). Starting a stream with the id
  of a still-running one cancels the older one. A page reload cancels every stream and resets ids
  (see below).

### Page reloads and navigations

A full page load (reload, `location.href = …`, F5 in WebView2) ends every backend call the
previous page left running, like the web server drops a request whose browser went away:

* When the new document commits (Tauri's `PageLoadEvent::Started`, before its scripts run), the
  shell aborts all backend calls still in flight, streams included: the provider connection is
  dropped, the old page's stream ids are forgotten (the new page counts from 1 again), and the
  replies go nowhere. Same-document navigations (the SPA's own routing) are not page loads.
* Tauri re-sends every invoke whose IPC `fetch` failed through `window.ipc.postMessage`
  (`ipc-protocol.js`), and an unloading page fails all of its pending ones. The shell recognises
  such a re-send (delivered by `postMessage`, same command and arguments as a custom-protocol call
  that is running or ended less than 30 s ago) and does not run it again: the re-sent invoke
  resolves to the original call's outcome (a re-sent `chat_stream_cancel` is ignored, so it cannot
  cancel the new page's stream with the same id). A `postMessage` invoke without such a twin runs
  normally, so a webview whose IPC custom protocol does not work still works. Platform commands
  (§3) and `app_info` are not covered; a reload while a platform command is in flight is very
  unlikely (dialogs are modal).
* Remaining gap: if the page goes away before an invoke's request has reached the shell at all,
  the `postMessage` copy is the only one. It is normally aborted with the rest, because it arrives
  before the new document commits (the order seen with WebKitGTK on Linux); if a webview delivers
  it after the commit, or the shell starts it a moment after the commit, it runs once with nobody
  listening (one provider request, never two).

## 3. Platform commands (desktop `PlatformShell`)

| command | JS args | resolves to | notes |
|---|---|---|---|
| `window_set_fullscreen` | `{ fullscreen: boolean }` | `null` | native window fullscreen (not the DOM Fullscreen API); the change is reported by the event in §4 |
| `window_is_fullscreen` | `{}` | `boolean` | current native state (on Linux the state GTK reports, so changes made by the window manager count) |
| `app_quit` | `{}` | `null`, if it settles at all | leaves fullscreen (no event, §4), saves the window geometry, exits; replaces V1's SHUT DOWN redirect |
| `open_external` | `{ url: string }` | `null` | opens `http:`/`https:` URLs with a host in the default browser; anything else rejects |
| `save_text_file` | `{ suggestedName: string, contents: string, mime?: string }` | `{ saved: true, path: string }` or `{ saved: false }` (cancelled) | native save dialog, then writes `contents` as UTF-8 |
| `open_text_file` | `{ extensions?: string[] }` | `string` or `null` (cancelled) | native open dialog, then reads the file as UTF-8 |

* `save_text_file`: `suggestedName` is sanitized (path separators and characters Windows forbids
  become `_`). If it has no extension, one is derived from `mime` (`application/json` → `.json`,
  `text/plain` → `.txt`, `text/markdown` → `.md`, `text/csv`, `text/html`, `xml`), and that extension
  becomes the dialog's file-type filter. The dialog opens in Downloads (else Documents). The UI
  shows V1's success text (`"エクスポート完了: " + fileName`, the generated name) only after
  `{ saved: true }`, nothing for `{ saved: false }`, and V1's `"エクスポート失敗: " + message`
  when the promise rejects (e.g. `cannot write <path>: No space left on device (os error 28)`).
  The export achievement also waits for `{ saved: true }`.
* `open_text_file`: `extensions` like `["json"]` (`"*.json"` and `".json"` are accepted too)
  filter the dialog. The file is decoded like `FileReader.readAsText`: a UTF-8 BOM is removed and
  invalid bytes become U+FFFD. Files over 64 MiB are rejected.
* External links: the shell never loads other pages in the app window. Navigations away from the
  exact app origin (`tauri://localhost`, or `http://tauri.localhost` without a port on Windows;
  plus `about:blank` and the `devUrl` in development builds), `window.open(...)` and
  `<a target="_blank">` open http(s) URLs in the default browser automatically; other schemes are
  blocked. The check is exact because WebKitGTK and WebView2 resolve every `*.localhost` name to
  the loopback address (`http://tauri.localhost:<port>` would show a local web server inside the
  window) and Tauri's own local-origin test ignores the port. `open_external` is the explicit form.

## 4. Event: `amadeus://fullscreen-changed`

Payload `{ "fullscreen": boolean }`. Emitted (to all webviews) whenever the main window enters or
leaves native fullscreen, whatever the cause: `window_set_fullscreen`, the macOS green button or
View menu, a window-manager shortcut on Linux. On Linux the shell follows GTK's window-state
events (tao, Tauri's windowing layer, only remembers the state the app last requested there), so
the event comes when the window manager has actually changed the window, and not at all for a
request it refuses.

```js
const unlisten = await window.__TAURI__.event.listen("amadeus://fullscreen-changed",
  (e) => settings.setInt("Config_ScreenMode", e.payload.fullscreen ? 0 : 1));
```

* Desktop replacement for V1's `fullscreenchange` listener (`Config_ScreenMode`: 0 = fullscreen,
  1 = windowed). Read `window_is_fullscreen` once at startup, because the event only reports
  changes.
* Escape: like a browser in DOM fullscreen, the UI leaves fullscreen when Escape is pressed while
  the window is fullscreen, whatever entered it (F11, menu, CONFIG, macOS green button,
  window-manager shortcut). A capture-phase `keydown` listener of the platform shell sends
  `window_set_fullscreen { fullscreen: false }` and consumes the key press (not while an IME
  composition takes it), so no other listener of the page sees it; this event then reports the
  change like any other (DECISIONS D19).
* Not emitted while the app is exiting (`app_quit`, closing the window, the restart after an
  update). The shell leaves fullscreen before exiting so the saved window geometry is the windowed
  one; the user's `Config_ScreenMode` stays as it was.
* The window opens windowed. The shell does not restore fullscreen by itself: window-state saves
  size, position and maximized only. The UI does not apply a persisted `Config_ScreenMode = 0` at
  startup either, although `window_set_fullscreen` needs no user gesture: it writes the setting
  from the `window_is_fullscreen` answer instead (DECISIONS D19, which settles spec-app-part4
  §Open questions 4).
* Known limitation: quitting with macOS Cmd+Q / Dock "Quit" while fullscreen bypasses the shell,
  so the next launch is windowed at screen size.

## 5. Content Security Policy

`app.security.csp` in `apps/desktop/tauri.conf.json`:

```
default-src 'self'; script-src 'self' 'wasm-unsafe-eval';
style-src 'self' 'unsafe-inline' https://fonts.googleapis.com;
img-src 'self' data: blob:; media-src 'self' data: blob:; font-src 'self' data: https://fonts.gstatic.com;
connect-src 'self' ipc: http://ipc.localhost;
object-src 'none'; base-uri 'self'
```

Verified at runtime in the desktop webview: Trunk's bootstrap runs and the WASM app mounts, while
`eval`, `new Function`, injected inline scripts and `fetch` to other origins are blocked.

* **Scripts**: The two inline scripts of `apps/web/index.html`, the Google Fonts loader (see
  Fonts) and Trunk's `<script type="module">` bootstrap, are allowed by their SHA-256 hashes. At
  build time Tauri hashes every non-empty inline script of every HTML asset and adds the hashes to
  `script-src` of the served CSP header, so no `'unsafe-inline'` is needed (further inline
  scripts in `index.html` are hashed the same way). Scripts injected at runtime, `on*=`
  attributes, `javascript:` URLs, `eval` and `new Function` are blocked; event handlers attached
  from Rust/wasm are fine. WebAssembly is allowed by `'wasm-unsafe-eval'`.
* **Styles**: `style="…"` attributes, `<style>` elements and CSSOM are allowed (`'unsafe-inline'`).
  `dangerousDisableAssetCspModification: ["style-src"]` stops Tauri from adding a nonce for inline
  `<style>` elements in `index.html` (e.g. Trunk `data-inline`). A nonce would make the browser
  ignore `'unsafe-inline'` and block every `style` attribute.
* **Fonts**: `apps/web/index.html` adds V1's Noto Serif JP stylesheet from Google Fonts (an
  `@import` on line 1 of V1's `app/amadeus/app/globals.css`) with that inline loader, placed
  between the Tailwind bundle and `amadeus.css`, on the `/app` route only (the window's start page;
  elsewhere the link is inserted disabled and not fetched — one still loading or failed when `/app`
  is left stays enabled at `media="print"`, which applies nowhere — and `RouteStyles` toggles it on
  navigation); the
  sheet has `media="print"` until it has loaded, then `all`, so a slow or unreachable Google never
  holds back the app (DECISIONS D28). The stylesheet origin `https://fonts.googleapis.com`
  (`style-src`) and the font origin `https://fonts.gstatic.com` (`font-src`) are therefore
  allowed. Offline, the CSS falls back to `"MS Mincho", serif`. If the font is ever self-hosted
  under `static/`, remove the loader and both origins.
* **Network**: `fetch` is limited to the app itself and the IPC endpoints. Every other network
  request goes through a backend command. (V1's MS Mincho download from
  `raw.githubusercontent.com` is gone — a dead link, DECISIONS D24 — so that origin is no longer
  allowed.) `fetch("data:…")` / `fetch("blob:…")` are blocked; decode
  base64 in wasm instead.
* **Media and images**: `data:` and `blob:` URLs work for `<img>`, `<audio>` and textures.

## 6. Capabilities (JS plugin APIs)

`apps/desktop/capabilities/default.json` grants the main window, for the app's own origin only:

| permission | used for |
|---|---|
| `core:default` | Tauri's baseline: `window.__TAURI__.event.listen`/`unlisten`/`emit` (§4 needs `listen`), read-only window getters (`getCurrentWindow().isFullscreen()`, `innerSize()`, `scaleFactor()`, …), `app.getVersion()`, `path.*` resolution, and menu/tray/image/resource handles. Window *changes* (fullscreen, close, …) are not included; use §3. Prefer `window_is_fullscreen` over `isFullscreen()`, which on Linux returns the last requested state |

Custom commands (§2, §3) and Channels need no entries: Tauri skips the ACL for them when the call
comes from the app's own origin and no app ACL manifest is defined (calls from any other origin are
rejected). The dialog, log, opener, process, updater and window-state plugin APIs are deliberately
not exposed to JS; use the Rust commands above. To put frontend errors or WASM panics into the app
log, add `log:allow-log` together with the frontend code that calls
`window.__TAURI__.log.error(message)` (target `webview`); today no frontend code does, so the
permission is not granted. With `withGlobalTauri` some plugin JS wrappers exist
under `window.__TAURI__.*`, but calls to plugin commands that are not granted reject with an ACL
error. The dialog plugin also replaces `window.alert`/`window.confirm` with native dialogs, which
are not granted either, so do not use them (V1 does not).

Log files: Linux `~/.local/share/com.dsuv.realamadeus/logs/RealAmadeus.log`, macOS
`~/Library/Logs/com.dsuv.realamadeus/RealAmadeus.log`, Windows
`%LOCALAPPDATA%\com.dsuv.realamadeus\logs\RealAmadeus.log` (the app's own records, from the
`amadeus_desktop`, `amadeus_desktop_lib`, `amadeus_backend` and `amadeus_core` targets, from info
up; other libraries' records from warn up, so e.g. zbus's D-Bus call on every second launch is not
recorded; 2 MiB rotation, 3 files kept; Rust-side records only, Rust panics included). Records
forwarded from the frontend (target `webview`) would be kept from warn up like the other non-app
targets; add `.level_for("webview", LevelFilter::Info)` in `apps/desktop/src/logging.rs` if info
lines should be forwarded too.

## 7. Calling from Rust/wasm (sketch)

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// `window.__TAURI__.core.invoke`
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

fn to_js<T: serde::Serialize>(v: &T) -> Result<JsValue, JsValue> {
    // JSON semantics: plain JS objects (not Map), numbers, the web body's key order.
    let text = serde_json::to_string(v).map_err(|e| JsValue::from_str(&e.to_string()))?;
    js_sys::JSON::parse(&text)
}

// Channel: construct `window.__TAURI__.core.Channel` with js_sys::Reflect::construct, set its
// `onmessage` to a Closure<dyn FnMut(JsValue)>, and put the Channel object itself into the args
// object under `onEvent` (Tauri serializes it through its toJSON). Keep the closure alive until
// a terminal event or the cancel.
```

Replies deserialize with `serde_json::from_str::<ApiReply>(&text)`, where `text` is
`js_sys::JSON::stringify(&reply)` (or `Option<ApiReply>` for `chat_stream`, `AppInfo` for
`app_info`). `amadeus-ui` does exactly this (`transport::js::to_js_object`,
`transport::tauri::decode_api_reply`).

# V2 vs V1: intentional differences

This is the authoritative list of every intentional difference between RealAmadeus V2 (this Rust
workspace) and V1 (RealAmadeusWeb, the Next.js app served at `https://dsuv.elvelt.com/amadeus`)
that a user, an API client or stored data can observe. Anything not listed here behaves as in V1,
as the porting specs (`docs/specs/`) describe it, and was checked item by item against
`docs/specs/PARITY_CHECKLIST.md`.

Sources: `docs/DECISIONS.md` (binding), the deviation sections of the per-area port and review
reports, and the checklist. Claims were re-checked against the code on 2026-11-22.

**How to read an entry**

- **V1 → V2**: what V1 did and what V2 does instead.
- **Reason**: a decision of `docs/DECISIONS.md` (D1–D29), or a D18 category: *crash*, *security*,
  *dead link*, *platform constraint*, *data safety*. Two more labels are used: *V2 addition* (new
  behaviour with no V1 counterpart) and *spec* (the porting spec prescribes or explicitly allows
  the change; the section or quirk number is given).
- **Checklist**: `PARITY_CHECKLIST.md` item IDs, when the difference has one.
- V1 had no desktop build of the web app (the V1 desktop editions were the separate Unity and Qt
  apps). Desktop entries therefore compare the Tauri app with V1 *web* behaviour; the V2 web
  build keeps the V1 behaviour in those cases.

Contents: [1 Server & API](#1-server--api) · [2 FreeAPI, Vertex AI, secrets](#2-freeapi-vertex-ai-and-secrets) ·
[3 Desktop app](#3-desktop-app) · [4 App screen](#4-app-screen-root-menu-keyboard-mobile) ·
[5 Chat](#5-chat) · [6 Config](#6-config) · [7 Panels](#7-panels) · [8 Site pages](#8-site-pages) ·
[9 Live2D](#9-live2d) · [10 Core data & storage](#10-core-data-and-storage) ·
[11 Services](#11-services) · [12 Styling & i18n](#12-styling-and-i18n) ·
[13 Release & packaging](#13-release-and-packaging) ·
[Tool-only differences](#differences-visible-only-to-tools) · [V1 quirks kept](#v1-quirks-deliberately-kept) ·
[Known limitations](#known-limitations)

---

## 1. Server & API

The HTTP API contract (paths under `{base}/app/api/…`, JSON bodies, status codes, the SSE event
format, the cache header sets) is V1's. The differences are around it.

| # | V1 | V2 | Reason | Checklist |
|---|---|---|---|---|
| 1.1 | The app lived at `/amadeus/app` inside the Next.js site; static files were served from the origin root (`/resources/…`, `/amadeus_logo.png`, `/*_STEP*.png`). | One mount `{base}` (default `/amadeus`, `AMADEUS_BASE_PATH`): every page is a SPA route under it (`/amadeus/app` still opens the app), static files are base-relative (`{base}/resources/…`; V1's root files under `{base}/static/…`), the API stays at `{base}/app/api/…`. `amadeus-server` answers origin-root `/resources/**` with 404. | D1 | API-037, PAGE-006, L2D-009 |
| 1.2 | `/amadeus/` redirected (308) to `/amadeus`. | `{base}/` serves the app. Every other trailing or repeated slash still gets V1's 308 with a `Refresh` header and the target URL as body. | D1 (Trunk `public_url` is `/amadeus/`) | HDR-006 |
| 1.3 | `/` was the DSUV portfolio (another app on the same Next.js server). | `GET /` redirects (307) to `{base}`; other paths outside `{base}` get an empty 404. Behind the documented proxy rule only `/amadeus` and `/amadeus/…` reach `amadeus-server` (`/amadeus_logo.png` and `/sw.js` stay with the portfolio). | D1 | PLAT-012 |
| 1.4 | Unknown paths, API paths included, got Next.js's HTML 404 page. | Unknown `{base}/app/api/**` → JSON 404 `{"error":"Not found"}` (any method); unknown page under `{base}` → the SPA with status 404 (its NotFound view reproduces V1's 404 page); outside `{base}` → empty 404. | spec (any 404 allowed) | PAGE-009, PAGE-043 |
| 1.5 | No health endpoint; the manifest link `/amadeus/app/manifest.webmanifest` answered 404. | New `GET {base}/healthz` → 200 `ok` (text/plain, no-store); `{base}/manifest.webmanifest` is served. V1's kill-switch service worker is embedded byte-identical and served at origin-root `/sw.js`. | V2 addition | API-039 |
| 1.6 | No request body limit. | 16 MiB on the JSON API routes, 32 MiB on `stt/transcribe`; larger bodies get 413. | security | — |
| 1.7 | Method tokens Node's HTTP parser rejects (lowercase `get`, `FOO`) got a bare 400; non-GET/HEAD requests to static files got a 500 text/plain from Next's static handler. | The 400 carries the no-store header set; static files answer other methods with 405 + `Allow: GET, HEAD`. Everything else matches live V1: 405 without `Allow` for unimplemented standard methods, 204 for `OPTIONS` on API routes, empty 400 for non-standard methods, 500 for `TRACE`, page routes rendered for any method except `OPTIONS` (400). | platform constraint (400); crash (V1's 500) | API-003 |
| 1.8 | Next.js compressed responses, SSE included. | gzip/br only for compressible responses of at least 1 KiB, never images, audio, video or SSE; precompressed `.br`/`.gz` files are served when present. | spec (SSE is never compressed) | HDR-001, HDR-007 |
| 1.9 | App bundles were sent `no-store`. | Same by default; `AMADEUS_IMMUTABLE_ASSETS` opts in to `immutable` caching of Trunk's content-hashed js/wasm/css (`index.html`, the manifest, `sw.js` and the API stay `no-store`). | V2 addition (opt-in) | HDR-005 |
| 1.10 | Responses carried `X-Powered-By: Next.js`, `Vary: rsc, next-router-…` and RSC payloads. | Not sent. | platform constraint | HDR-008 |
| 1.11 | Every TTS/STT/RAG `serverUrl` was proxied. | Same by default; `AMADEUS_PROXY_ALLOWLIST` (comma-separated URL prefixes or `host[:port\|:*]`) rejects other targets with 403 `{"error":"serverUrl not allowed"}` (the health shape on health routes) and re-checks every redirect hop. Chat provider URLs are not covered (see [Known limitations](#known-limitations)). | security (opt-in) | — |
| 1.12 | When the browser went away or cancelled a stream, V1 kept reading the provider response until its next write to the closed stream failed. | Dropping the SSE response (or a desktop cancel) aborts the provider request at once. | spec (server-chat Q21) | API-044 |
| 1.13 | The FreeAPI rate limiter used wall-clock time and kept every IP forever; `.amadeus_request_count` was rewritten in place. | Monotonic clock, stale IPs pruned, the counter file written atomically in the same format. | data safety (server-chat Q16–Q18) | API-012 |
| 1.14 | Health replies' `statusText` was the upstream reason phrase (undici, HTTP/1.1). | The standard reason phrase when an HTTP/2 upstream sends none; a non-ASCII phrase (`503 サービス停止`) arrives empty (`503 `). The UI never displays it. | platform constraint | API-019 |
| 1.15 | Bodies were parsed with `JSON.parse`; upstream stream values were forwarded as JS values. | Request bodies with lone-surrogate escapes or numbers beyond the f64 range (`1e400`) take the malformed-JSON path; non-string `token`/`error` values in upstream stream data are sent as strings; lone surrogates in upstream text cannot be carried. | platform constraint | — |
| 1.16 | The provider-7 local proxy (`scripts/amadeus-local-proxy/server.mjs`) could corrupt UTF-8 characters split across chunks, never drained gcloud's stderr, quoted `cmd.exe` arguments badly on Windows and crashed on a bad or busy port. | The `amadeus-local-proxy` binary decodes the body once as UTF-8 (reads capped at 6 MB before V1's 2,000,000-unit check), discards gcloud's stderr, refreshes the token single-flight, quotes correctly on Windows without a console window, tries `/opt/homebrew/bin`, `/snap/bin` and `~/google-cloud-sdk/bin` after V1's gcloud paths, and exits 1 with a message on a bad or busy port. `HEAD /health` is still 404. `-h`/`--help` prints the usage (endpoints and environment variables) and `-V`/`--version` the version; both exit 0 without listening. V1 ignored every argument, so `--help` started the proxy on 0.0.0.0:5103. Other arguments are still ignored. | spec (fixes allowed); D27 (command line) | LPX-007, PLAT-009 |
| 1.17 | Next.js read the server settings from its `.env*` files and wrote `.amadeus_request_count` and `.amadeus_chat_log.jsonl` into its working directory (the repository root). | `amadeus-server` reads one dotenv file (`--env-file`, default `.env`; real environment variables win) and writes the same two files, in the same formats, into `--data-dir` (default `./data`). Switching from V1 means copying them over (`deploy/README.md` §4). A counter copied from V1 at 65000 keeps FreeAPI off until `AMADEUS_FREEAPI_MAX` is raised (2.6). | platform constraint | API-013, API-014, API-017 |
| 1.18 | Next.js served its own build; there was no separate frontend directory to check. | `amadeus-server` refuses to start (error log, exit 1) when the dist's `index.html` has a `<base href>` other than `{base}/`, because every page would load its assets from the wrong path. An `index.html` without `<base>`, or none, only logs a warning. | D1, D27 | API-037 |
| 1.19 | Next.js listened on port 9501 (`next start -p 9501`; the development server on 9601). The porting spec proposed `AMADEUS_HOST` / `AMADEUS_PORT` (alias `PORT`) for V2. | `amadeus-server` listens on `--bind` / `AMADEUS_BIND` only, default `127.0.0.1:9502`: it runs behind the reverse proxy, and V1's 9501 stays free so both can run side by side during the switch (`deploy/README.md` §4). `PORT`, `AMADEUS_HOST` and `AMADEUS_PORT` are not read. In development `trunk serve` listens on `127.0.0.1:8080` and proxies the API to `:9502`. | D27 | API-038, PLAT-009 |
| 1.20 | Upstream requests (LLM providers, the Vertex token exchange, the TTS/STT/RAG proxies) carried Node's `fetch` defaults: `User-Agent: node`, undici's implicit `accept-language: *`, `sec-fetch-mode: cors` and `accept-encoding: br, gzip, deflate` (`gzip, deflate` for plain-`http://` targets such as Ollama and local TTS/STT/RAG servers), and `cache-control: no-cache` plus `pragma: no-cache` on non-streaming provider calls (`cache: "no-store"`). | `User-Agent: RealAmadeus/<version>`; no `accept-language`, `sec-fetch-mode`, `cache-control` or `pragma`; `accept-encoding: gzip,deflate,br` for every target (plain-`http://` targets are now offered `br` too). No upstream depends on them; bodies, paths, methods and the documented headers are identical. The connection settings are V1's: 10 s connect timeout, 300 s read timeout, no total timeout, at most 20 redirects, no `Referer` on redirect hops (`crates/amadeus-backend/src/http.rs`, `client_builder`). | D25 | API-018, AI-010, AI-012, AI-015, TTS-016 |

## 2. FreeAPI, Vertex AI and secrets

| # | V1 | V2 | Reason | Checklist |
|---|---|---|---|---|
| 2.1 | A Google AI Studio key was hard-coded in the server source. | No built-in key: the server reads `AI_STUDIO_API_KEY` (fallback `GEMINI_API_KEY`; blank or `undefined` counts as none). Without a key, FreeAPI goes straight to its Vertex fallback. No secret is compiled into the desktop app, the WASM bundle or the Docker image. | D12 · security | AI-009 |
| 2.2 | The server's service account was `realamadeusmain-7a9af55234f3.json` in the Next.js working directory. | Only the file named by `AMADEUS_VERTEX_CREDENTIALS` (`--vertex-credentials`); none by default. | D12 · security | AI-024 |
| 2.3 | Service-account JWT `exp = iat + 3630`, beyond Google's one-hour maximum. | `exp = iat + 3600` (`iat = now − 30` kept). | D16 | AI-025 |
| 2.4 | Every Vertex request exchanged a fresh service-account token; three separate gcloud token caches. | Tokens are cached for `expires_in − 60 s` (at most 55 min; not cached when `expires_in` is missing or ≤ 60 s); a 401/403 on a cached token is retried once with a fresh exchange, so replies match V1; one shared, single-flight gcloud cache. | spec (server-chat Q14, Q15) | AI-025, AI-026 |
| 2.5 | The Vertex console line printed the first 15 characters of the credential. | The first 5 characters followed by `***`. | security | API-018 |
| 2.6 | The FreeAPI cap was the constant `MAX_GLOBAL_REQUESTS = 65000` (for requests with provider 8 or without a provider); the app's quota widget wrote 65,000 into its exhausted notice. | `AMADEUS_FREEAPI_MAX` / `--free-api-max` (default 65000; a whole number from 1; invalid values stop the server at startup) sets it, for the same requests. `/chat/quota`'s `max`, the 429 message `FreeAPIの全体リクエスト上限（N回）に達しました。…` and the app's FreeAPI quota widget (usage line, and the exhausted notice `全体リクエスト上限（N回）に達しました。…` / `Global request limit (N) reached. …`, N in the browser's number format) follow it. At the default 65000 every text is V1's. The desktop relays FreeAPI to the hosted server (3.3) and has no cap of its own. | D27 | API-013 |

## 3. Desktop app

Windows, macOS and Linux (Tauri 2). The full contract is `docs/DESKTOP_IPC.md`.

| # | V1 (web) | V2 desktop | Reason | Checklist |
|---|---|---|---|---|
| 3.1 | Settings, memory and achievements live in the site's localStorage. | The webview has its own storage and starts empty; nothing is migrated (CONFIG export/import moves a conversation). | D15 | PLAT-002, STO-010 |
| 3.2 | Backend calls went over HTTP to the web server; TTS/STT/RAG URLs such as `http://localhost:5100` meant the server's machine. | Backend calls go over Tauri IPC to the backend inside the app, so those URLs mean the user's machine. No rate limit, chat log or request counter. | platform constraint | PLAT-003, PLAT-007 |
| 3.3 | FreeAPI ran on the server with the server's secrets. | FreeAPI chat, streaming and quota are relayed to the hosted server `https://dsuv.elvelt.com/amadeus` (env `AMADEUS_REMOTE_API_BASE` at app start overrides it); if it cannot be reached, the user gets V1's FreeAPI communication-error reply. | D11 | PLAT-008 |
| 3.4 | Vertex AI (provider 4) without a user credential fell back to the server's service account. | The app holds no service account; provider 4 needs the user's own API key or service-account JSON. Because the backend runs on the user's machine, the no-credential fallback of V1's non-streaming route (`useGCloudCLI: false` and an empty key, server-chat route B2) would use the user's own `gcloud` CLI login; the app never takes that route for provider 4, which it always streams, and the stream path never uses gcloud (as in V1). | D11, D12 | PLAT-008, AI-024 |
| 3.5 | A browser window. | Window "Amadeus" that opens straight on the app screen (`/app`, the login), 1280×720 by default, minimum 960×540 (above the 900 px mobile breakpoint, so resizing never remounts the chat); size, position and maximized state are restored on the next start (fullscreen never is); single instance (a second launch focuses the window). | D13, D19 | PLAT-015 |
| 3.6 | F11 / SCREEN MODE used the DOM Fullscreen API; a persisted `Config_ScreenMode = 0` showed as fullscreen until the first toggle (QUIRK-8). | Native window fullscreen. The window always starts windowed and `Config_ScreenMode` follows the real window state from startup on (initial query plus `amadeus://fullscreen-changed`, which also reports the macOS green button and window-manager shortcuts), so the first F11 always enters fullscreen. The web build keeps QUIRK-8. Escape while fullscreen leaves fullscreen exactly like the browser's Escape in V1 (that key press never reaches the page, so an open overlay such as the achievement panel stays open; `Config_ScreenMode` becomes 1). | D19 | CSS-037, PLAT-001, KEY-011 |
| 3.7 | SHUT DOWN: `window.close()`, `history.back()` after 100 ms, then a redirect to `https://dsuv.elvelt.com/amadeus`. | Quits the app (it leaves fullscreen first and saves the windowed geometry). | platform constraint | SCR-013, PLAT-001 |
| 3.8 | CONFIG export downloaded `chat_export_<ts>.json` through the browser. | Native save dialog (opens in Downloads, the name is sanitized). The green success text (`エクスポート完了: chat_export_<ts>.json`, the generated name) and the export achievement come once the file is written; a cancelled dialog leaves the status line as it was; a failed write shows V1's red `エクスポート失敗: ` + the error (e.g. `cannot write …: No space left on device (os error 28)`). The web shows the success text at once, as in V1. | platform constraint | PLAT-001, CHAT-049 |
| 3.9 | CONFIG import used a hidden file input. | Native open dialog: UTF-8, BOM removed, invalid bytes become U+FFFD, files over 64 MiB are rejected. | platform constraint | UI-041 |
| 3.10 | Links opened in the tab or a new tab. | Only the exact app origin (`tauri://localhost`, or `http://tauri.localhost` without a port on Windows) is shown in the window; other http(s) navigations, `window.open` and `target="_blank"` open in the default browser, other schemes are blocked. The landing «Return» link goes to `https://dsuv.elvelt.com/` (web: `/`). | security | PAGE-050 |
| 3.11 | — | IPC commands are accepted only from the app origin; the window holds only Tauri's `core:default` permissions, so no plugin API is reachable from JS and `window.alert`/`confirm` are unavailable (V1 never used them). | security | — |
| 3.12 | A reload dropped the page's requests (the server kept the upstream running, see 1.12). | Any full page load (F5/Ctrl+R in WebView2, `location.href = …`) aborts every backend call of the old page, streams included; Tauri's re-send of an abandoned call returns the original outcome instead of running it again, so a provider is never called twice. | platform constraint (Tauri re-sends abandoned calls) | — |
| 3.13 | No Content Security Policy. | CSP: scripts only from the app (plus the hashes of the two inline scripts of `index.html`, Trunk's bootstrap and the Google Fonts loader, and `'wasm-unsafe-eval'`), inline styles and Google Fonts allowed, `fetch` only to the app and IPC, `object-src 'none'`; `raw.githubusercontent.com` is not allowed. | security, D13, D24 | CSS-037, PLAT-011 |
| 3.14 | — | Builds with the `updater` feature (release builds made with a signing key) check for an update 3 s after start, ask in a native Japanese + English dialog, install and restart (Windows: passive installer). | V2 addition | — |
| 3.15 | — | A Rust-side log file `RealAmadeus.log` in the OS log directory (the app's own records from info up, other libraries' from warn up, so for example zbus's D-Bus call on every second launch is not recorded; 2 MiB × 3 files); frontend errors are not in it. | V2 addition | — |
| 3.16 | Speech input: browser SpeechRecognition first, otherwise recording + upload. | Always records and uploads through the backend (the WebView2 and WKWebView recognizers are unusable). Not visible while the mic button stays hidden (D14). | D21 | PLAT-004 |
| 3.17 | — | A stream that ends without a terminal event finishes after a grace of up to 1 s (an IPC message of 8 KiB or more can arrive after the call resolves), so V1's Vertex silent-close case completes about 1 s later. | platform constraint | STR-018 |
| 3.18 | — | macOS keeps Tauri's default menu bar (app menu with Quit, Edit, View, Window). | platform constraint | — |

## 4. App screen (root, menu, keyboard, mobile)

| # | V1 | V2 | Reason | Checklist |
|---|---|---|---|---|
| 4.1 | The boot screens restarted whenever the root re-rendered (QUIRK-6): a Tab during the desktop boot or any re-render during the mobile loading started them over, so they could take much longer than nominal. | Each boot runs once (desktop about 22 s, mobile 1.5 s). | spec (app-part4 §10 item 6, do not replicate) | SCR-012 |
| 4.2 | The server-rendered page started at 1920×1080 and was scaled to the window after hydration. | The stage is fitted to the window from the first frame. | platform constraint (client-side rendering) | UI-048 |
| 4.3 | Settings access threw when localStorage was blocked (e.g. disabled site data), and writes threw when it was full. Only the long-term memory caught its own write errors: it logged `[MemoryManager] Failed to persist memory to localStorage:` with the error each time. | Writes never throw: a failed write is kept in memory for the session (reads stay consistent) and one warning is logged per session. Long-term memory writes behave as in V1: every failed `RealAmadeus.Memory` write logs `[MemoryManager] Failed to persist memory to localStorage: <error>`, with the error printed as text (`String(error)`). When localStorage cannot be opened at all, the single startup warning covers the memory too. | crash | STO-011 |
| 4.4 | Where the Fullscreen API is missing (iPhone, older iPadOS) the call threw a TypeError, which also aborted CONFIG Apply midway. | Fullscreen calls are no-ops there, so Apply completes. | crash | CFG-028 |

Desktop screen-mode and SHUT DOWN behaviour: entries 3.6 and 3.7.

## 5. Chat

D23 adopts spec-app-part2 §6.4 Option B for the providers the client always streams (Google
Gemini, Groq, Vertex AI, Ollama, OpenRouter: indices 1, 3, 4, 5, 6); everything else of the chat
engine is V1's.

| # | V1 | V2 | Reason | Checklist |
|---|---|---|---|---|
| 5.1 | Streamed replies never left `waitingAPI`: the dots ran until Ctrl+C / キャンセル and the reply was never shown or logged (it still went into history, memory, TTS and achievements). | Streamed replies are typed out with pages, ▼ and backlog entries like non-streamed ones. Typing starts once the leading emotion tag is decided and there is text to show. Only text the completion will display is typed: memory commands, `<think>` blocks and emotion tags are removed as at completion. Anything still undecided (a half-received `[SMI`, an unclosed `<think>`, an unterminated `[SAVE_…`) waits for more tokens, so the text never jumps back. Pages paused during the stream reach the backlog when it completes; a stream that ends without completing (cancelled, failed, or cut off when the chat panel unmounts, e.g. when the layout crosses the 900 px breakpoint) leaves none, even for pages already shown. A page whose pause was typed while the reply was still arriving stays on screen until more text arrives, so the last ▼ shows the last page even when only held-back text (memory commands, a `<think>` block) follows or the reply arrives slowly. | D23 | CHAT-028 |
| 5.2 | Skipping (Enter/click) while a stream was still open stalled the typewriter or dropped the rest of the reply. | The skip reveals what has arrived and keeps typing until the stream completes. The skipped page is logged once it is final (at its pause or at the end), not at the skip. | D23 | CHAT-028 |
| 5.3 | STATUS latency was re-reported on every token until the emotion tag was decided. | Latency is the time to the first displayable token. | D23 | CHAT-029 |
| 5.4 | The TTS session guard was effectively constant, so audio synthesized before a reset could still play. | A real generation counter drops stale results. While a stale synthesis is still outstanding, the next reply's first chunk waits for it. | D23 | TTS-006 |
| 5.5 | The streamed TTS tail was never flushed: a reply ending without `。！？!?` (typical English) was not spoken at all. | The tail is always spoken at completion. | D23 | TTS-003 |
| 5.6 | Streamed text went to TTS before the leading emotion tag was decided, so a tag split across tokens was spoken (V1 said `[SURPRISえっ！`), and so were an unclosed `<think>` block and memory commands (`[SAVE_FACT: …]`). | Streamed text is captured for TTS only once the tag is decided, and only the typed text (5.1) is captured, so a split tag, a `<think>` block or a memory command is never spoken. | D23 | TTS-002, CHAT-028 |
| 5.7 | When a streamed-TTS offset landed inside a surrogate pair, the lone half was carried into the TTS text. | The half is dropped. | platform constraint | TTS-002 |
| 5.8 | `[SAVE_FACT: …]` / `[SAVE_EVENT: …]` payloads were parsed with `JSON.parse`. | Payloads that JS accepts but serde_json rejects (lone-surrogate escapes, numbers beyond f64 such as `1e400`, nesting deeper than 128) take V1's parse-error path: the raw text is used, without tags. | platform constraint | CHAT-005 |
| 5.9 | An unparsable import file reported V8's message, e.g. `SyntaxError: Unexpected token …`. | `SyntaxError: <serde_json message>`, e.g. `SyntaxError: expected value at line 1 column 1`. | platform constraint (spec app-part1 Open Q8 accepts it) | CHAT-054 |
| 5.10 | Ctrl+C / キャンセル could cancel a streamed reply at any time, because the chat never left `waitingAPI` (5.1). | While a streamed reply is still arriving, the PC hint `CTRL+Cでキャンセル`, Ctrl+C and the mobile `キャンセル` button stay available while it is typed (same element and position as in `waitingAPI`; not in `inputReady`). Cancelling also stops the typewriter and keeps nothing of the reply: no backlog rows, and history, memory commands and achievements only happen at completion. As in V1, a cancel does not stop TTS. Once the stream has completed they disappear and Ctrl+C does nothing. | D23 | STR-005 |

## 6. Config

| # | V1 | V2 | Reason | Checklist |
|---|---|---|---|---|
| 6.1 | A failed read of the chosen import file left an unhandled promise rejection. | The failure is logged as an error. | crash | UI-041 |
| 6.2 | — | Desktop only: import and export use native dialogs (entries 3.8, 3.9). The web build clicks the same hidden `.config-import-input` as V1. | platform constraint | PLAT-001 |

## 7. Panels

| # | V1 | V2 | Reason | Checklist |
|---|---|---|---|---|
| 7.1 | STATUS `SYSTEM_VERSION` and the last credits line read `Real Amadeus v1.4N(Build 20260801_001)`. | `Real Amadeus V2.0.0R` (from the workspace version). | D3 | UI-026, UI-028 |
| 7.2 | CHANGE LOG listed 6 entries, `Version 1.4N` first. | A `Version 2.0R` / `2026. 11. 22` entry on top. Its text is built in (no `changelog_v20` locale key): Japanese for language 0, English for the other 12 languages. | D3 | UI-033 |
| 7.3 | STATUS inserted OPERATOR (the user name, which SAVE_FACT can take from LLM output) and LLM_MODEL as raw HTML. | Both are HTML-escaped; normal values render identically. | security (D18) | UI-027 |
| 7.4 | While STATUS was open, its clock and 5 s network poll restarted on every parent re-render. | They run independently of parent re-renders. | spec (app-part3 §5.2, do not replicate) | UI-025 |

## 8. Site pages

Landing, docs, download, changelog and the 404 page.

| # | V1 | V2 | Reason | Checklist |
|---|---|---|---|---|
| 8.1 | Landing: `SYSTEM V1.4 RELEASED` / `REAL AMADEUS V1.4 HAS BEEN OFFICIALLY RELEASED.` (ja `Real Amadeus V1.4が正式にリリースされました。`). | `V1.4` is replaced by the build version: `SYSTEM V2.0.0R RELEASED` and so on. | D3 | PAGE-026 |
| 8.2 | `/changelog` and the `/docs/download` timeline started at V1.4. | A `V2.0R` entry on top (same text as 7.2). | D3 | PAGE-028, PAGE-045 |
| 8.3 | The landing GitHub button opened `github.com/dsuv-sg/RealAmadeus` (now moved to `github.com/dsuv-sg/RealAmadeusUnity`). | `github.com/dsuv-sg/RealAmadeus` (V2 Rust repository). | spec (pages Open Q6) | PAGE-021 |
| 8.4 | `/download` showed the V1.4 Unity/Qt zip cards. | V2 layout from a static release table: a 2×2 grid (Windows NSIS + MSI, macOS DMG + `.app.tar.gz`, Linux AppImage + deb + rpm, Web: the web app and the self-host package), a version line under the subtitle, the visitor's OS card tagged «推奨 / RECOMMENDED» (phones: the web app), SHA256SUMS and All Releases links, and a collapsible «旧バージョン (V1) / Legacy (V1)» section with V1's cards. The content gets extra top padding so the taller page clears the fixed header. | V2 addition | PAGE-046, PAGE-048 |
| 8.5 | Images came from origin-root paths (`/OpenAI_STEP1.png` …). | `{base}/static/…`. | D1 | PAGE-020 |
| 8.6 | No `aria-current` attributes. | The link to the current page (e.g. the docs sidebar entry) carries `aria-current="page"`, which screen readers announce. | platform constraint (leptos_router `<A>`) | — |
| 8.7 | On the live server the 404 tab title was `DSUV` (the root layout's title won). | `404: This page could not be found.`, restored to `Amadeus` when leaving the page. | spec | PAGE-043 |
| 8.8 | Site pages had their own head: description `Personal Portfolio of DSUV - Lead Architect @ ELVELT`, icon `/amadeus/icon.png`, plain viewport, no manifest. | All routes share the app's head: description `Real Amadeus Web`, icon `static/amadeus_logo.png` (the same image), `viewport-fit=cover`, theme colour, manifest and Apple tags. `/amadeus/icon.png` is not served. | D26 (one SPA shell) | PAGE-003 |
| 8.9 | The app stylesheet's global reset (`* { margin:0; padding:0; user-select:none }`) could stay loaded after navigating client-side from `/app` to a site page, breaking its spacing (latent V1 bug). | The app stylesheet is disabled on every route except `/app`. | D9 | PAGE-017 |
| 8.10 | A missing site-dictionary path threw a TypeError. | It renders the path string. Unreachable with the shipped dictionaries. | crash | — |
| 8.11 | — | Desktop: the landing «Return» link goes to `https://dsuv.elvelt.com/` (entry 3.10). | spec | PAGE-050 |
| 8.12 | `/docs/download` documented only the V1.4 Unity (standard) and Qt (lightweight) builds; its change-log timeline was the last section. | A «Rust版 RealAmadeus V2 / Rust Edition RealAmadeus V2» section comes first: one box per platform with the V2 packages under the `/download` card labels (Windows: installer `.exe` + MSI, macOS: `.dmg` + `.app.tar.gz`, Linux: AppImage + `.deb` + `.rpm`, Web: self-host server; the MSI line only when the version can have an MSI) and a «ダウンロードページへ / GO TO DOWNLOAD PAGE» link to `{base}/download`. V1's change-log section (`V2.0R` on top, 8.2) comes next, moved out of V1's section grid so that the V2 entry does not read as legacy content. V1's description and its Standard and Lightweight sections follow unchanged under a «旧バージョン (V1) / Legacy (V1)» heading drawn like `/download`'s column headings. | V2 addition (owner decision) | PAGE-035 |
| 8.13 | Image preload hints: `/amadeus/app` preloaded 16 login and menu images (8 `<link rel="preload">` tags, 8 in a `Link` header), and some site pages sent `Link` preloads too (the landing 1, `/amadeus_logo.png`; `/docs/login` 1; `/docs/api/openai` 6; `/docs/api/gemini` 5; `/docs/api/claude` 6; `/docs/api/groq` 6; `/docs/api/vertex` 11, plus 6 `<link rel="preload" as="image">` tags). | No image preload hints on any route, neither `<link rel="preload" as="image">` tags nor a `Link` header (only Trunk's preloads of the WASM module and its JS glue): the client-side app boots its WASM before it renders any image, so preloads would only compete with the WASM download and make Chrome warn about preloaded resources that were not used. The images load when the UI first renders them. | D26 | PAGE-001 |

## 9. Live2D

V2's renderer was measured pixel-identical to V1's deployed bundle (56 scenarios, maximum
difference 0; same draw calls, RNG draws and console lines). The differences are in loading,
error handling and recovery.

| # | V1 | V2 | Reason | Checklist |
|---|---|---|---|---|
| 9.1 | `reama-cubism-runtime.js` fetched the 13 shader files at preload and compiled 242 programs at startup; a failing shader was retried every frame. | Shaders are embedded; only the programs the model needs are compiled, each once. A compile failure is logged once; if a mask program fails, masked parts are drawn without clipping instead of disappearing. `resources/live2d/reama-cubism-runtime.js` and `resources/live2d/shaders/*` are no longer shipped, and the `?v=19` cache-buster is gone. | D4 | L2F-021, L2D-021, L2D-049 |
| 9.2 | An exception in the frame loop stopped `requestAnimationFrame` for good (frozen model). | Per-frame errors are caught and logged once per distinct message; the loop keeps running. | D5 | L2F-025 |
| 9.3 | After a WebGL context loss the model stayed invisible until the page remounted it. | `webglcontextlost` is handled (drawing pauses); on `webglcontextrestored` the renderer is rebuilt and the texture re-uploaded. | D6 | L2D-046 |
| 9.4 | The Core log function was registered as `undefined`, which produced the uncaught `Logging.logFunction is not a function`. | The Core script is loaded first, readiness is polled (10 s timeout), a failed Core `<script>` is replaced on retry, and a real log handler is registered exactly once per page. Its lines, like the framework's `CubismLog*` messages (for example `Failed to loadPhysics().`), go to the `cubism` log target, which the app keeps silent like V1's `LogLevel_Off` (L2D-015). | D7 | L2D-047, L2F-004, L2D-015 |
| 9.5 | `destroy()` threw, logging `Live2D destroy error:` on every remount (for example when crossing the mobile breakpoint). | `destroy()` releases everything (frame loop, observers, listeners, renderer, textures, Core model and moc, framework reference) without throwing; pending loads become no-ops. | D8 | L2D-045 |
| 9.6 | The texture upload waited for `img.decode()`. | `img.decode()` is raced against a 3 s timer, after which the upload decodes synchronously (same pixels). | platform constraint (decode can stall, e.g. headless Chrome) | — |
| 9.7 | The model's load chain fetched its files one after another and did not handle some failures (a failed pose, user-data or motion file, an empty motion group, a moc that fails to load). | Requests run in parallel and complete in model3.json order; a moc failure stops loading with a log line, a failed optional file continues with an empty buffer, an empty motion group no longer hangs. V1's never-started WAV lip-sync provider is not ported. None of this affects the shipped model. | crash | L2D-019 |
| 9.8 | Growth of the Core's wasm heap was not handled (the model's memory views go stale). | The model is reloaded from the asset cache. | crash | — |
| 9.9 | Asset caches were keyed by the request path. | Keys are the resolved absolute URL; model paths are percent-encoded per segment (existing `%XX` kept). | D1 (base-relative URLs) | L2D-009 |
| 9.10 | `setLipSync(NaN)` kept NaN, which counted as "not speaking" but also blocked the idle sneeze timer. | NaN is mapped to 0 (not speaking) and the sneeze timer keeps running. A NaN eye-tracking axis becomes 0, as in V1. | spec (live2d-pipeline §9.6) | — |
| 9.11 | Malformed model JSON made the framework throw or loop forever (unknown motion segment type, unknown physics input/output type, motion used before its effect IDs were set). | An unknown segment type is an error, unknown physics entries are skipped, the motion runs without the effect IDs, placeholder strings such as `NullValue` become empty; arrays are sized from the actual data. | crash | — |
| 9.12 | Cubism 5.3 blend modes and offscreen drawing were supported by the bundled framework. | Not supported: such parts are drawn with the older compatible shaders after one warning at load. The shipped model (moc 5.0) uses none. | platform constraint (scope) | L2F-024 |

## 10. Core data and storage

localStorage keys and value formats are V1's (STO-001…013), so web users keep their data when V2
replaces V1 on the same origin. These are the cases where V2 reads or writes differently.

| # | V1 | V2 | Reason | Checklist |
|---|---|---|---|---|
| 10.1 | When the 50-unit preview or the 300-unit summary cap of the conversation trim cut through an emoji, V1 stored the lone half (`\ud83d`) in `conversationSummaries` and `episodicMemory`. | V2's trim drops the half. Memories written by V1 with such a half load completely (every other field as in V1, the half removed); the next save writes V1's bytes minus each lone half. In hand-edited data only, a string that is just a lone half becomes empty and entries differing only by one merge. | data safety | MEM-008, STO-006 |
| 10.2 | V1 kept wrongly typed top-level fields of `RealAmadeus.Memory` (`userName`, `totalInteractions`, `lastSessionDate`, `recentEmotions`) as stored and later failed on them or concatenated strings. | They are coerced on load (`userName` a string or JS-formatted number, else `""`; `totalInteractions` a truncated number, else 0; the others `String(v)`). What made V1 reset the whole memory (bad JSON, `null`, a non-iterable `userFacts`, …) still resets it. Only hand-edited storage contains such values. | crash, data safety | STO-012, MEM-009 |
| 10.3 | `RealAmadeus.VectorStore` kept entry ids, content and category as stored. | They become strings (a numeric id and the equal string id merge); entries whose embedding is not an array are skipped. V1 never writes this store. | data safety | STO-008 |
| 10.4 | `amadeus_achievements` containing `1e400` or a lone-surrogate escape loaded its other fields. | Such a value resets to the fresh state (V1 itself only writes ASCII there). | platform constraint | STO-007 |
| 10.5 | Integers beyond ±2^63 in settings, fact strength or interaction counts stayed floats (e.g. `1e20`). | They saturate at the i64 limits. Only hand-edited storage can reach this. | platform constraint | STO-003 |
| 10.6 | Passing NaN to the RAG top-K or threshold setter stored `"NaN"`. | NaN means "no change". | data safety | RAG-001 |
| 10.7 | Stored dates were parsed by V8, including its legacy fallback formats (strings such as `Jan …`) and its ±275,760-year range. | V8's ISO rules are reproduced, its legacy formats and range limit are not, so such hand-edited dates can be judged differently (an invalid date becomes today, as in V1). | platform constraint | MEM-010 |
| 10.8 | JS prototype lookups leaked into data: an emotion string `constructor` could unlock an achievement, `unlock("__proto__")` and `key in dict` lookups behaved specially. | Not reproduced. | platform constraint | — |
| 10.9 | The boot-log padding threw for a non-positive or non-finite character width. | Such a width counts as 10 and the padding is capped at 65,536 spaces. | crash | SCR-003 |

Fact ordering still uses the browser's `Intl.Collator("ja")`, exactly V1's
`localeCompare(…, "ja")` (D22); a built-in approximation is only used where `Intl` is missing.

## 11. Services

| # | V1 | V2 | Reason | Checklist |
|---|---|---|---|---|
| 11.1 | The microphone stayed open when `MediaRecorder` could not be created or started; a second start during the permission prompt opened two streams; a late `onstop`/`onend` of an old recording could tear down a newer one. | The microphone is released on failure, a second start while the prompt is open is ignored, and every recording keeps its own recorder, stream and meter. | security (microphone left open) | STT-013 |
| 11.2 | — | Desktop speech input always uploads through the backend (entry 3.16). | D21 | PLAT-004 |
| 11.3 | Events already received in the current chunk could still be delivered after a stream cancel; a cancel during the request phase could still emit an error; a finished old request could clear a newer stream's slot. | No stream events or errors after a cancel, and only the owning request clears the active slot. A non-streaming send still cannot be cancelled (D20). | spec (no events after cancel) | STR-003, STR-005 |
| 11.4 | A stopped audio element could still fire its `ended`/`error` handlers. | Handlers are removed when playback is stopped (a dropped wasm closure must never be called). | platform constraint | — |
| 11.5 | Non-string JSON fields in replies were used as JS values. | They are converted with JS `String(x)`, so the same text is shown. | platform constraint | — |
| 11.6 | — | Desktop: a stream that ends without a terminal event completes after a grace of up to 1 s (entry 3.17). | platform constraint | STR-018 |

## 12. Styling and i18n

| # | V1 | V2 | Reason | Checklist |
|---|---|---|---|---|
| 12.1 | Every `/app` load fetched MS Mincho from `raw.githubusercontent.com/vslabs/winfonts/master/msmincho.ttc`, now a dead link (404 and a console error). | No download. The CSS maps "MS Mincho" to the locally installed font or the bundled Noto Serif CJK, which is what V1 showed whenever the download failed. | D24 · dead link | CSS-037 |
| 12.2 | `@font-face` URLs were origin-absolute (`/resources/fonts/…`). | Base-relative (`resources/fonts/…`), resolving under `{base}`. | D1 | — |
| 12.3 | — | The in-app change-log entry for V2 has no locale keys (entry 7.2). | D3 | — |
| 12.4 | The Google Fonts sheet (Noto Serif JP 300/400/700, `display=swap`) was an `@import` on the first line of the app CSS. While fonts.googleapis.com was slow or unreachable, the import kept that stylesheet pending: the page stayed blank and did not hydrate until Chrome gave up on the request (about 28 s through a proxy that never answers, minutes when packets are dropped). | `index.html` adds the same sheet with an inline script placed between the Tailwind bundle and `amadeus.css`, so Google's faces still come before the local `@font-face` rules (same cascade and face order). The sheet has `media="print"` until it has loaded, then `all`. A slow or unreachable Google holds nothing back: the app starts and paints at once with the local faces and switches to Google's faces when the sheet arrives. (Ported verbatim, the `@import` hung V2 the same way: Chromium did not start Trunk's module loader, so the page stayed empty with no console error, and WebKitGTK showed the app unstyled.) Like the app CSS (D9), the sheet is used on the app route only: off `/app` the link is inserted disabled and is not fetched, so the site pages make no request to fonts.googleapis.com, as in V1; `RouteStyles` enables it when `/app` is shown (fetched at media `print`, then `all`) and disables it when the app is left (a sheet still loading at that moment once it has loaded). | D28 · crash · spec (spec-css §5.5, "Alternative (equivalent cascade)") | CSS-005, CSS-009 |

## 13. Release and packaging

| # | V1 | V2 | Reason | Checklist |
|---|---|---|---|---|
| 13.1 | Tag `RealAmadeusV1.4`; assets `RealAmadeus_V1.4U_Win64.zip`, `…_V1.4U_MacUniversal.zip`, `…_V1.4U_Linux.zip`, `…_V1.4Q_Win64.zip` (U = Unity, Q = Qt). | Tag `v<semver>`; assets `RealAmadeus_V<semver>R_<Platform>_<arch>.<ext>` (R = Rust): `_Windows_x64-setup.exe` (NSIS), `_Windows_x64.msi`, `_macOS_universal.dmg`, `_macOS_universal.app.tar.gz`, `_Linux_amd64.AppImage`, `_Linux_amd64.deb`, `_Linux_x86_64.rpm`, `_Web_Linux-x64.tar.gz`, plus `SHA256SUMS.txt`, and the Docker image `ghcr.io/dsuv-sg/realamadeus-web`. Optional Linux arm64 builds (repository variable `RELEASE_LINUX_ARM64`) add `_Linux_aarch64.AppImage`, `_Linux_arm64.deb`, `_Linux_aarch64.rpm` and `_Web_Linux-arm64.tar.gz`. | D2 | PLAT-010 |
| 13.2 | Zip archives. | Installers: NSIS (per-user, Japanese/English language selector), MSI (not built for pre-releases with non-numeric identifiers such as `2.1.0-rc.1`), universal DMG, AppImage, deb, rpm. | V2 addition | — |
| 13.3 | — | The installers show no license page (NSIS, MSI, DMG): Tauri CLI 2.12's MSI bundler breaks on the bilingual `LICENSE` (WiX `LGHT0311`). `LICENSE` ships in the Web package. | platform constraint | — |
| 13.4 | — | Linux packages list the app under Games (`Categories=Game;Amusement;`, deb section `games`); Tauri's default would map the bundle category to `Network;`. macOS keeps the Entertainment category. | platform constraint | — |
| 13.5 | — | macOS builds are ad-hoc signed and Windows builds unsigned unless signing secrets are configured (Gatekeeper / SmartScreen warnings, see `docs/RELEASE.md` §5). Auto-update is optional and never offers pre-releases. | platform constraint | — |
| 13.6 | V1's `public/resources` was served as is. | V2 ships it without the 93 Unity `.anim` files under `resources/models/…/motions/` (12 MB, referenced by nothing) and without the Live2D runtime and shader files (9.1), so those URLs answer 404. Everything the app loads is byte-identical. | spec (checklist G2, PLAT-011) | PLAT-011 |

---

## Differences visible only to tools

Listed for completeness; nothing changes for users in normal use.

- DOM: overlay wrappers use longhand `top/right/bottom/left: 0px` where V1's serialized style shows
  `inset: 0px`; Leptos inserts empty `<!---->` placeholder comments (`.chat-panel-root` has one
  only while the mini game is open); Chrome's Tab navigation skips comments, and ChatPanel
  re-inserts its swapped dialogue / input block in place so that the Tab starting point ends where
  React's commit left it (KEY-009, `crates/amadeus-ui/src/app/chat/blocks.rs`); React copied input
  values into `value` attributes and the Vertex textarea's text node, V2 sets only the property; a
  login panel mounted again after a logout keeps `autofocus=""`; attribute order and text-node
  splitting differ.
- Settings read timing: `Config_VertexEnabled` (the hidden flag that shows Vertex AI in the
  provider pickers) is read when a dropdown opens, not on every render, so editing it in the
  developer tools while a list is open shows only after reopening.
- Timing: the `""` flicker of ▼ at the double-▼ step may not render; the stream-complete emotion
  achievement uses the live emotion (V1 the last rendered one, different only when a whole stream
  arrives in one chunk); a transient lone surrogate shows as U+FFFD while typing; the site
  background is re-randomized on the next 50 ms clock tick instead of also on the EN/JP toggle;
  the backlog auto-scroll happens on the next animation frame, before it is painted.
- Startup requests: the three health POSTs go out before the Live2D Core script request; the
  first Live2D draw takes more animation frames (median 14 against 4) but not more time.
- Math: `Math.sin/cos` come from libm and differ from V8 by at most 1 ulp on about 1 % of inputs;
  the rendered pixels are identical.
- CSS: theme colours stay in `oklch` (V1's build added hex/`lab()`), two media queries print as
  `(width<=900px)`, the landing's never-applied `.glow-on-hover` rule is left out; on `/app`,
  `document.styleSheets` lists the Google Fonts sheet as its own entry between the Tailwind bundle
  and `amadeus.css` (V1: an `@import` rule inside the app CSS); on the other routes `<head>` holds
  its `<link id="amadeus-google-fonts">` disabled, like `amadeus-app-css` (except a sheet that was
  still loading, or had failed to load, when `/app` was left: it stays enabled at `media="print"`,
  which applies nowhere).
- Desktop: Tauri serves `.moc3` and `manifest.webmanifest` as `text/html` and answers missing
  files with `index.html` (200); the Core loads the model regardless.
- Server console: the masked `Vertex Request` line is printed once per stream, also when the
  stream restarts in another region (the line would be identical).

## V1 quirks deliberately kept

Kept for parity as the reports name them (D18: user-visible quirks stay unless they are crashes,
security holes or dead links).

- **Decisions:** D8 (Live2D: hair physics before the player driver, two Core updates per frame,
  `Date.now()` / `performance.now()` clocks with the 0.1 s cap, projection constants), D14 (mic
  button hidden; no `/dev audio input` command, checklist Gaps G1-1), D17 (crossing the 900 px
  breakpoint remounts the chat), D20 (Q-A5 double `errorOccurred`; Q-C1 a non-streaming send is
  not cancellable, so a late reply after Ctrl+C is still typed: AI-029, CHAT-042).
- **App screen** (spec-app-part4 §10): QUIRK-2, 4, 5, 7, 8 (web only, see 3.6), 11, 12, 15, 17,
  18, 21, 22, 23.
- **Config:** QUIRK-1 (Español / 한국어 cannot be picked from the dropdowns), QUIRK-11, 12, 13,
  QUIRK-19 (re-selecting the current provider discards unsaved edits), QUIRK-20 (hidden values
  normalised on Apply), QUIRK-21 (an invisible focused button still reacts to Space), QUIRK-15 /
  CFG-022 (desktop-layout Apply does not refresh the LLM status).
- **Chat:** the double ▼ and the repeated backlog entry (CHAT-041); import drops the system prompt
  (QUIRK-3, CHAT-050); no IME `isComposing` or key-repeat check (KEY-006); no autofocus after an
  exchange (CHAT-044); `/dev emotion help` + Enter reveals everything at once; the §9 pause rule
  ("or while the stream is incomplete") lets a pause decided before a closing `」` arrives start
  the next page with it. (V1's spoken split tag is not kept: entry 5.6.)
- **Services:** Q-S7 (a throwing `SpeechRecognition.start()` leaves the UI "recording" until the
  30 s limit), TTS-017 (32-bit TTS cache key, cached object URLs never freed), the V1 client SSE
  parsing quirks (`[DONE]`, `null`, end of body).
- **Server:** server-chat Q5 (Vertex silent close, STR-018), Q6 (the streaming fallback re-enters
  `/chat` with IP `unknown`: second rate-limit check, double count and log, STR-016), Q7–Q12, Q20
  (upstream status pass-through), and 500 with an empty body where V1 threw outside its try blocks.
- **Live2D:** the portrait framing crops the top of the head (constants 3.3 / −2.1); a
  `devicePixelRatio` change without a resize is ignored.
- **Site pages:** the landing DOWNLOAD link never glows (PAGE-005); the GitHub icon keeps V1's path
  typo (PAGE-019); the docs advertise `Alt + Enter / F11` while only F11 works (Gaps G1-9); the
  site language is not persisted (PAGE-010).
- **i18n:** V1's missing keys and inconsistent fallbacks (I18N-009, I18N-011); `toLocaleString`
  follows the browser locale (QUIRK-24).

## Known limitations

- **Unknown URLs whose path ends in `/app`** (for example `/amadeus/docs/app`, which shows the 404
  page) count as the app route for the route-scoped styles: they load `amadeus-app-css` and the
  Google Fonts sheet (V1 loads neither there), so that 404 text is not selectable. Rendering is
  otherwise identical. The check is `ends_with("/app")` in `crates/amadeus-ui/src/lib.rs` and the
  loader in `apps/web/index.html`; matching the exact `<base>app` route would fix it.
- **Windows and macOS desktop builds were not run on real machines** during the port (everything
  was verified on Linux/WebKitGTK and in headless Chrome): WebView2 and WKWebView behaviour, the
  order of page loads and IPC re-sends, native dialogs, updater installs, notarization and
  fullscreen through the macOS green button are untested there.
- **WebView2 browser shortcuts stay enabled** (F5 / Ctrl+R reload, Ctrl+P, Ctrl+F, Alt+←, Ctrl+±):
  a reload resets the screen like in a browser. Backend calls of the old page are aborted and never
  run twice (3.12); turning the keys off needs Windows-only code that could not be tested.
  Remaining gap: a call abandoned before it reached the shell can run once with nobody listening.
- **The desktop backend ignores HTTP proxies** (`no_proxy()`): behind a proxy-only network the LLM
  providers and the FreeAPI relay are unreachable from the desktop app (the webview itself uses
  the system proxy).
- **Desktop window state:** quitting with Cmd+Q / Dock Quit while fullscreen on macOS bypasses the
  shell, so the next launch opens at screen size (delete `.window-state.json`, `docs/RELEASE.md`
  §5). Notarized macOS builds will need the `com.apple.security.device.audio-input` entitlement if
  the mic button is ever enabled. Frontend errors and WASM panics are not written to
  `RealAmadeus.log`; developer tools exist only in debug builds.
- **GitHub `ubuntu-22.04` runners are removed on 2027-04-17** (brownouts on 2027-03-23, 03-30,
  04-06 and 04-13). The Linux desktop and web release legs still use them for the glibc 2.35
  baseline and must be migrated before the first brownout (`docs/RELEASE.md` §5).
- **actionlint was not run** on the workflows (installing it was not permitted); the Docker image,
  the Windows and macOS bundles and real Google credentials (JWT acceptance, live Vertex / AI
  Studio calls) were not exercised locally.
- **PAGE-047 is not implemented:** `/download` never looks up the live GitHub release; its links
  come from the static table for the build's version and it shows no release date.
- **V1 security surface kept on the web server for parity:** `chat/vertex-proxy` is
  unauthenticated and not rate-limited; chat provider URLs (`ollamaHost`, `openaiBaseUrl`, the
  local-proxy host/port) still let clients make the server POST to arbitrary URLs
  (`AMADEUS_PROXY_ALLOWLIST` covers only TTS/STT/RAG); the rate limit trusts the first
  `X-Forwarded-For` entry, so the reverse proxy must set it.
- **No settings migration** from the web version to the desktop app (D15).

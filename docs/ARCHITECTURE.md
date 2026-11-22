# RealAmadeus V2 (Rust) — Architecture

RealAmadeus V2 is a complete Rust rewrite of **RealAmadeusWeb** (V1: Next.js 16 / React 19,
served at `https://dsuv.elvelt.com/amadeus`). One codebase produces:

| Target  | How it is built                                  | Release assets (D2: `RealAmadeus_V<semver>R_…`) |
|---------|--------------------------------------------------|-------------------------------------------------|
| Web     | Trunk → WASM frontend + `amadeus-server` (axum)   | `_Web_Linux-x64.tar.gz` (server, `amadeus-local-proxy`, `dist/`), Docker image `ghcr.io/dsuv-sg/realamadeus-web` |
| Windows | Tauri 2 (`cargo tauri build`)                     | `_Windows_x64-setup.exe` (NSIS), `_Windows_x64.msi` |
| macOS   | Tauri 2, universal (Apple Silicon + Intel)       | `_macOS_universal.dmg`, `_macOS_universal.app.tar.gz` |
| Linux   | Tauri 2                                          | `_Linux_amd64.AppImage`, `_Linux_amd64.deb`, `_Linux_x86_64.rpm` |

`cargo xtask dist …` and `.github/workflows/release.yml` produce the same names; the table lives in
`amadeus_core::release`, which the download page also reads. The binding design decisions are in
`docs/DECISIONS.md` (D1–D29); every intentional difference from V1 is listed in
`docs/V1_DIFFERENCES.md`; the V1 behaviour itself is specified in `docs/specs/` (604 checklist
items in `docs/specs/PARITY_CHECKLIST.md`).

## Repository layout

```
RealAmadeusRust/
├── crates/
│   ├── amadeus-core      pure logic, wasm32 + native, NO I/O
│   ├── amadeus-backend   native: everything V1's Next.js API routes did
│   ├── amadeus-server    native: the web server (bin amadeus-server) + bin amadeus-local-proxy
│   ├── amadeus-live2d    wasm: Cubism Web Framework port, WebGL renderer, ReamaCubism player
│   └── amadeus-ui        wasm: Leptos 0.8 CSR UI, browser services, Web/Tauri transport
├── apps/
│   ├── web               Trunk crate `amadeus-web` (index.html, Trunk.toml, style/, manifest)
│   └── desktop           Tauri 2 crate `amadeus-desktop` (binary `RealAmadeus`)
├── xtask/                `cargo xtask …` build and release automation (pure Rust)
├── tools/                Live2D parity tooling: live2d-viewer, l2d-compare, l2d-probe
├── assets/public/        resources/ + static/, copied into every frontend build (V1 `public/`)
├── deploy/               systemd unit, nginx / Caddy configs, docker-compose, deployment guide
├── Dockerfile            the web image
├── docs/                 this file, DECISIONS, V1_DIFFERENCES, DESKTOP_IPC, RELEASE, specs/
└── .github/workflows/    ci.yml, release.yml (tag-driven release of all four targets)
```

Dependency direction (`A ◄── B`: B depends on A; no cycles):

```
amadeus-core ◄──┬── amadeus-backend ◄──┬── amadeus-server
                │                      └── amadeus-desktop   (also embeds apps/web/dist-desktop)
                └── amadeus-ui ◄────────── amadeus-web       (apps/web, the WASM binary)
amadeus-live2d ◄─┬── amadeus-ui
                 └── live2d-viewer                           (tools/)
xtask: no runtime dependency (dev-dependency on amadeus-core for the asset-name test)
```

`amadeus-live2d` does not depend on `amadeus-core`. The desktop crate depends on the frontend
only through the Trunk output that Tauri embeds (`frontendDist = ../web/dist-desktop`).

## Crates and modules

### amadeus-core (wasm32 + native, no I/O)

Persistence goes through `storage::KvStore`; networking lives in `amadeus-backend` (native) or
`amadeus-ui` (browser).

| Module | Contents |
|---|---|
| `settings` (`keys`, `defaults`, `forms`, `display`, `jsnum`) | V1 `AppSettings`: the `Config_*` keys and defaults, JS `parseInt`/`parseFloat` semantics, CONFIG and mobile form load/apply, stage fitting and the 900 px breakpoint, the chat payload builder |
| `storage` | the `KvStore` trait (localStorage in the browser, maps in tests); `try_set` returns a failed write's error to the caller (used by the memory manager) |
| `i18n` (`dict`, `site`) | both V1 translation systems: app strings for 13 languages (`locales/app/*.json`) and the site dictionary (`locales/site/{ja,en}.json`), compiled in; V2-only site texts (the `/docs/download` V2 section, the `/download` asset labels and the «Legacy (V1)» heading) are Rust data in `i18n::site::site_v2`, because the site JSON must stay token-identical to V1's literal |
| `memory` (`collation`, `time`, `jis_kanji`, `symbol_ranks`) | `MemoryManager`: user name, facts, episodes, trimming, prompt context; V8 date rules; a built-in ja collator used only where `Intl` is missing |
| `achievements`, `random_events`, `minigame`, `vector` | `AchievementManager`, `RandomEventService`, the quiz, the RAG vector-store logic |
| `chat_utils` | emotion tags, message splitting, TTS chunking and stream capture, the streamed display prefix (D23), `/dev` commands, the API error table, export/import |
| `persona`, `providers` | the system prompts per language; the provider table, the streaming set, the LLM status |
| `protocol` | wire DTOs, API paths (`paths`), the desktop IPC contract (`ipc`) |
| `types`, `jscompat` | V1's shared types, `Emitter` (V1 `Signal`), JS semantics (trim, `String(x)`; number formatting delegated to `settings::jsnum` / ryu-js) |
| `version`, `changelog`, `release` | `V2.0.0R` (D3), the in-app change log (V2 entry + V1 entries), the release asset table (D2) |

### amadeus-backend (native)

| Module | Contents |
|---|---|
| `lib.rs`, `config` | `Backend` and `BackendConfig`. `BackendMode::Server` keeps the rate limits (with the FreeAPI cap `free_api_max`, D27), the request counter and the chat log in `data_dir`; `BackendMode::Desktop` has none of them and relays FreeAPI to the hosted server |
| `http` | the shared reqwest client settings (`client_builder`: rustls, no proxy, 10 s connect / 300 s read timeouts, no automatic `Referer`, `User-Agent: RealAmadeus/<version>`; D25); `build_client` follows up to 20 redirects, the allowlist client (`aux/upstream`) uses its own redirect policy |
| `chat/` | `mod` (`complete`, `stream`, `quota`, `vertex_proxy`, `client_ip`), `payload` (lenient, JS-coercing parse), `bodies` / `extract` (upstream request and response shapes), `providers` (`/chat`), `streaming` + `sse` + `sink` (`/chat/stream`), `google` (service-account JWT, token caches, gcloud), `vertex` (regions, failover), `free_api`, `remote` (desktop relay), `ratelimit`, `chatlog`, `net` (undici-compatible errors), `texts` (verbatim V1 strings), `tests` |
| `aux/` | `tts`, `stt`, `rag`, `health` (the proxies), `upstream` (fetch semantics, redirects), `allowlist` (`AMADEUS_PROXY_ALLOWLIST`), `form` (undici `formData()`), `js`, `js_json` (`JSON.stringify`-compatible output) |
| `local_proxy` | library part of `amadeus-local-proxy` (V1 `scripts/amadeus-local-proxy/server.mjs`) |

### amadeus-server (native)

| Module | Contents |
|---|---|
| `main.rs` | binary `amadeus-server`: configuration, optional dotenv file, startup checks (refuses to start, exit 1, when the built `index.html` has another `<base href>` than the base path; warns when it has none; D27), graceful shutdown |
| `bin/amadeus-local-proxy.rs` | binary `amadeus-local-proxy` (provider 7), configured only through `LOCAL_PROXY_HOST` / `LOCAL_PROXY_PORT` / `VERTEX_PROJECT` / `VERTEX_LOCATION`; `-h`/`--help` and `-V`/`--version` exit without listening, other arguments are ignored |
| `config` | the 9 clap flags with environment fallbacks (`--free-api-max` included; `amadeus-server --help`) |
| `app` | router assembly, body limits (16 MiB JSON, 32 MiB STT upload), compression |
| `api` | `{base}/app/api/...` handlers and Next.js method handling |
| `static_files` | the Trunk `dist/` with SPA fallback, `{base}/healthz`, the V1 service worker at `/sw.js` |
| `policy`, `reply`, `client_ip`, `shutdown`, `local_proxy_http` | V1's cache-header rules and Next-style 308 redirects; backend results → HTTP; Node header semantics and `getClientIp`; signals; the local proxy's HTTP adapter |
| `tests/integration/` | `routing`, `sse`, `aux_proxy`, `local_proxy`, with temp dist dirs and mock upstreams (`common`) |

### amadeus-live2d (wasm)

| Module | Contents |
|---|---|
| `core/` (`ffi`, `model`) | wasm-bindgen bindings to the global `Live2DCubismCore`; `CubismModel` / `CubismMoc` with Rust mirrors of parameters and parts |
| `framework/` | the Cubism Web Framework 5-r.5 model layer: `model_setting`, `physics`, `effect/` (breath, eye blink, pose, look), `motion/` (motions, queue, expressions), `update/` (scheduler and the seven updaters), `user_model`, `user_data`, `json`, `model_state` (pure test double), `golden_tests` |
| `math/` | `CubismMatrix44`, model and view matrices, `Vector2`, `CubismMath`, target point |
| `render/` | the WebGL1/WebGL2 renderer: `gl`, `renderer`, `clipping` (masks), `shader` / `shader_table` / `shader_sources` (sources embedded from `shaders/` with `include_str!`, D4), `texture`, `render_target` |
| `app/` | the sample-app layer and the public `Live2DPlayer`: `player` (canvas, frame loop, destroy), `driver` / `emotion` / `noise` / `clock` (the ReamaCubism player), `pipeline` (V1 frame order), `projection`, `subdelegate` (DPR, resize, context loss D6), `manager`, `app_model`, `texture_manager`, `assets` / `preload` (page-level asset caches), `core_loader` (D7), `golden_tests` |

### amadeus-ui (wasm, Leptos 0.8 CSR)

Foundation:

| Module | Contents |
|---|---|
| `lib.rs` | `App`: `services::init()`, the `<Router>` (site routes and `/app`) and `RouteStyles`, which enables the app stylesheet and the Google Fonts link only on `/app` (D9; a Google sheet still loading is left enabled until it has loaded) |
| `env` | base URL helpers (`router_base`, `asset_url`, `api_url`, all relative to `document.baseURI`), `is_tauri()`, clocks, DOM helpers |
| `storage` | `LocalStorageKv` (never throws; failed writes are kept in memory; `set` warns once per session, `try_set` returns the error instead) |
| `hooks/` | `settings` (the `AppSettings` singleton with per-key reactive memos), `viewport` (900 px breakpoint) |
| `i18n` | app `t()` and the site locale signal |
| `platform/` (`web`, `tauri`) | `PlatformShell`: fullscreen (desktop: the window state and the Escape exit, D19), file save/open, shutdown, external links |
| `transport/` (`http`, `tauri`, `sse`, `reply`, `js`) | one `Transport`, HTTP `fetch` or Tauri IPC, with V1's client semantics and V1's SSE parser |
| `services/` | `ai`, `audio`, `tts`, `stt`, `rag`, `quota`, `logic` (V1 `lib/amadeus/services`) |
| `components/live2d_canvas` | V1 `Live2DCanvas` around `amadeus_live2d::Live2DPlayer`; `preload_live2d()` |

App screen (`/app`, V1 `RealAmadeusApp.tsx`):

| Module | Contents |
|---|---|
| `app/mod.rs` | `RealAmadeusApp`: `managers::init()`, the init effect (defaults, LLM status, Live2D preload, health checks, stage scaling, fullscreen sync D19), the menu-mode effect, the root keydown listener, the desktop/mobile switch (D17) |
| `app/root.rs`, `app/desktop.rs`, `app/mobile_root.rs` | root state and handlers (`AppRoot`), the two layout trees |
| `app/menu.rs`, `app/keyboard.rs`, `app/marquee.rs` | the side menu, the root key decision table, the mobile provider ticker |
| `app/conversation_io.rs`, `app/listener.rs`, `app/state.rs` | export/import and the back log, `window` listeners that own their closure, shared state types |
| `app/managers.rs` | the core manager singletons (D22, below) |
| `app/chat/` | `ChatPanel` (`mod`), `controller` (state, submit, AI/TTS handlers, the D23 streaming path and its cancel), `typewriter`, `tts_queue`, `listener`, `blocks` (the dialogue / input / trigger blocks as one branch; after each swap the block, read from the DOM between `.live2d-shift` and `.mini-game-panel`, is re-inserted in place so native Tab focus matches V1, KEY-009) |
| `app/config/` | `ConfigPanel` (`mod`), `rows`, `quota` (FreeAPI quota), `util` |
| `app/panels/` | `login`, `loading` (boot), `backlog`, `status`, `achievements`, `minigame`, `changelog`, `help`, `confirm`, `common` |
| `app/mobile/` | mobile `login`, `loading`, `settings` |

Site pages: `pages/` (`landing`, `download`, `changelog`, `not_found`, `docs/` with getting
started, download, login, controls and the five API guides, `chrome` for the terminal chrome,
`site` for the shared helpers) and `icons`.

### apps/web

`src/main.rs` installs the console logger and mounts `amadeus_ui::App`. The logger
(`src/logger.rs`) prints records at Info, except those of the `cubism` target (the Cubism
framework's `CubismLog*` equivalents, the Core's own log lines and V2-only render notes for cases
where V1 printed nothing), which is off like V1's `LogLevel_Off`; building with
`AMADEUS_CUBISM_LOG=error|warn|info|debug|trace` shows it. `index.html` carries V1's head, the
Tailwind bundle, the Google Fonts sheet (added by an inline script with a non-matching media until
it has loaded, so a slow fonts.googleapis.com never holds back the boot; inserted disabled off
`/app`, and `RouteStyles` toggles it with the app CSS; D28) and the app CSS in V1's cascade order,
and the Trunk copies of `assets/public` and `manifest.webmanifest`. `Trunk.toml` sets
`public_url = "/amadeus/"`, Tailwind 4.1.18, minification on release, `trunk serve` on
`127.0.0.1:8080` and the dev proxy to `:9502`. `style/amadeus.css` is V1's app `globals.css`
without its first line (the Google Fonts `@import`, now in `index.html`). `src/boot_tests.rs`
pins the `index.html` boot invariants.

### apps/desktop

| Module / file | Contents |
|---|---|
| `lib.rs`, `main.rs` | builder, plugins (single-instance, log, window-state, opener, dialog, process; updater with the `updater` feature), setup, the page-load hook |
| `ipc.rs` | the 11 backend commands, which call the same `amadeus-backend` functions as the HTTP routes |
| `calls.rs` | `CallRegistry`: a page (re)load aborts the calls of the old page, and Tauri's re-sent invoke gets the original result instead of running twice |
| `platform.rs` | the 6 platform commands (fullscreen, quit, external links, save/open dialogs) |
| `shell.rs` | window creation, the exact-origin navigation guard, fullscreen tracking (GTK window state on Linux), graceful quit |
| `state.rs`, `config.rs` | managed state (backend, calls, stream registry, page counter); the hosted server base (`AMADEUS_REMOTE_API_BASE`) and the data directory |
| `logging.rs`, `updater.rs` | `RealAmadeus.log` (app records from info, libraries from warn; 2 MiB × 3 files); the optional auto-update check |
| `tauri.conf.json`, `tauri.updater.conf.json`, `capabilities/default.json`, `linux/RealAmadeus.desktop`, `Info.plist` | window, CSP and bundles; updater endpoint and key; `core:default` only; the Linux menu entry (Games); macOS plist |

The IPC contract is `docs/DESKTOP_IPC.md`.

### xtask

`main` (clap CLI: `version`, `bump`, `release-notes`, `dist web|desktop|all`, `check`,
`check-classes`, `clean-release`), `version`, `changelog` (bump and release notes), `dist`
(including the installer-license guard), `platform` (target triples and asset names), `archive`,
`check` (the CI gates), `classes` (the Tailwind arbitrary-value class guard, PAGE-051, D29; skips
test code: `#[cfg(test)]` code down to a single field, variant, match arm or parameter, and the
files of out-of-line `#[cfg(test)]` modules), `util`. `templates/README-web.txt` ships inside the
web tarball.

## App-wide singletons

V1 created several objects at module evaluation time. V2 keeps one instance per page:

| Instance | Where | Created |
|---|---|---|
| `MemoryManager`, `AchievementManager`, `RandomEventService`, `VectorDatabase` (D22) | `amadeus_ui::app::managers::{memory, achievements, random_events, vectors}` | thread-local, leaked `&'static`, built by `managers::init()`, the first call of `RealAmadeusApp`, i.e. when the `/app` route mounts (V1: module evaluation of the app bundle). `init()` is idempotent; it installs `Intl.Collator("ja")` into the memory manager, forwards `AppSettings.settingsChanged` to the achievement and vector managers, and mirrors the RAG server availability from `services::rag()`; the memory manager's `save_failed` reports are logged as V1's `[MemoryManager] Failed to persist memory to localStorage: <error>` warning |
| `AudioService`, `TTSService`, `STTService`, `AIService`, the RAG client | `amadeus_ui::services::{audio, tts, stt, ai, rag}` | thread-local `Rc`; on `/app` they are created by `managers::init()` together with the managers, in V1's module-evaluation order (memory, achievements, AI, audio, STT, TTS, vector DB + RAG client, random events; spec-lib-core §1.10, D22); `services::init()` in `App` only selects the transport |
| `AppSettings` | `amadeus_ui::hooks::settings::app_settings()` over `storage::local_store()` | first access; reactive reads are memoized per key |
| `Transport` | `amadeus_ui::transport::transport()` | once per page: HTTP, or Tauri IPC when `window.__TAURI__` exists |
| `PlatformShell` | `amadeus_ui::platform::platform()` | `Copy` value (Web or Desktop) |
| Live2D Core and framework | `amadeus_live2d` | the Core script is loaded once per page and its log handler registered once (D7); the framework is reference-counted (`framework::acquire` / `release`); asset caches live for the page; framework and Core log lines use the `cubism` log target, silent by default (L2D-015) |
| `Backend` | `amadeus-server` (one for all requests) and `amadeus-desktop` (Tauri managed state, with `CallRegistry` and the stream registry) | at startup |

Leptos `Callback`s need `Send + Sync` closures: never capture these `Rc` or `&'static` values in a
`Callback`; call the accessor inside the closure, and subscribe to emitters with `connect_scoped`
(it disconnects on cleanup).

## Runtime topology

### Web
```
Browser ──GET {base}/…──────────► amadeus-server ──► dist/ (index.html, wasm, css, resources/, static/)
        ──POST {base}/app/api/…─► amadeus-server ──► amadeus-backend ──► LLM / TTS / STT / RAG
```
`{base}` defaults to `/amadeus` (env `AMADEUS_BASE_PATH`) and must equal Trunk's
`public_url` (the server refuses to start otherwise, D27). All runtime URLs in the frontend are
**relative to `document.baseURI`** (Trunk injects `<base href="{public_url}">`), so the same code
works under any base.

### Desktop
```
Tauri webview (same WASM frontend, public_url "/") ──invoke()/Channel──► amadeus-desktop
                                                                         └► amadeus-backend
```
The frontend detects Tauri via `window.__TAURI__` (`app.withGlobalTauri = true`) and routes
every backend call through IPC instead of `fetch`. Provider calls are made natively from Rust
(no CORS). The **FreeAPI** provider needs server-side secrets, so on desktop it is forwarded to
the hosted web server: `https://dsuv.elvelt.com/amadeus`, overridden by the environment variable
`AMADEUS_REMOTE_API_BASE` at app start (D11).

## Compatibility rules (V1 → V2)

1. **HTTP API contract is frozen.** Paths, JSON payloads, status codes and the SSE event format
   (`data: {"type":"token"|"complete"|"error",…}\n\n`) are identical to V1, so V1 clients, the
   desktop app and V2 web are interchangeable.
2. **localStorage keys and value formats are identical** (`Config_*`, memory, achievements …) so
   users keep their data when V2 replaces V1 on the same origin.
3. **CSS is reused** (V1 `globals.css` → `apps/web/style/amadeus.css`, whose Google Fonts
   `@import` became a loader script in `index.html`, D28; Tailwind v4.1.18 via Trunk). Component
   DOM structure and class names follow V1.
4. **Static paths mirror V1 `public/`** but are base-relative:
   V1 `/resources/...` → V2 `resources/...`; V1 root files (`/amadeus_logo.png`,
   `/Gemini_STEP1.png`, …) → V2 `static/...`.
5. Secrets are never compiled in or committed: the FreeAPI key comes from the environment at
   runtime (`AI_STUDIO_API_KEY`, fallback `GEMINI_API_KEY`) and the Vertex service-account JSON
   is loaded from `AMADEUS_VERTEX_CREDENTIALS` (see `docs/DECISIONS.md` D12). V1's
   `CANOPYWAVEAPI` belonged to a non-Amadeus route and is not used by V2.
6. Everything else that differs from V1 is intentional and listed in `docs/V1_DIFFERENCES.md`
   (D18: user-visible V1 quirks stay unless they are crashes, security holes or dead links).

## Live2D

* `live2dcubismcore.min.js` (proprietary, redistributable under the Live2D Proprietary Software
  License) is loaded at runtime by `amadeus-live2d` and is the only JavaScript file the frontend
  loads besides Trunk's generated wasm-bindgen glue (`index.html` also has two inline scripts: Trunk's
  module bootstrap and the Google Fonts loader, D28; `amadeus-server` also serves V1's kill-switch
  service worker at `/sw.js`, which the app never registers).
* Everything V1 got from `reama-cubism-runtime.js` (Cubism Web Framework 5-r.5 + sample app
  layer + the custom `ReamaCubismPlayer`) is re-implemented in Rust. Shaders are embedded with
  `include_str!` from `crates/amadeus-live2d/shaders/`.

## How parity is verified

The V1 reference is the RealAmadeusWeb checkout this workspace sits in (`..`), as the specs read
it on 2026-11-16. What is in the repository:

| Harness | What it checks | How to run |
|---|---|---|
| Unit and golden tests of every crate | amadeus-core: expected values captured by running V1's TypeScript under Node 22 (the memory scenario of spec Appendix G, achievement toasts, random events, quiz seeds, prompts pinned by SHA-256, the i18n tables; tests that need the V1 checkout skip when it is absent), plus the streamed display prefix (every prefix of a stream against its completion display, D23). amadeus-backend: upstream bodies, SSE parsing, Vertex JWT and token caches, `JSON.stringify` output, undici `formData()`, the shared client settings (timeouts, no `Referer` on redirect hops), the chat routes end to end against local mock providers. amadeus-live2d: `framework/golden_tests.rs` (motion, expression, eye-blink and physics curves) and `app/golden_tests.rs` (bit-exact replay of V1's 600-frame pipeline traces at six canvas sizes), plus MVP, clipping and shader-table goldens, and `log_channel_tests.rs` (which `log` calls use the silent `cubism` channel, and the list of calls that may reach the console, found by a source scan that sees every macro spelling and rejects imported log macros). amadeus-ui: the pure parts (typewriter, TTS queue, key and menu tables, the stream cancel decision, the block swap against models of React and Leptos (KEY-009), stage fitting, export bytes and the desktop export status, the desktop Escape rule, the FreeAPI quota texts, SSE parser, `RouteStyles`' path test and Google Fonts link rule). xtask: asset names equal `amadeus_core::release`, the installer-license guard, the Tailwind class extractor/escaper (broken fixture, test-code skipping incl. out-of-line test modules and `#[cfg(test)]` fields, variants, arms and parameters). apps/desktop: call registry, navigation guard, dialog file handling, fullscreen tracking, command names, log levels. apps/web: the per-target console logger (`src/logger.rs`) and the `index.html` boot invariants (`src/boot_tests.rs`: no remote `@import`, the Google Fonts loader's position, media switch and route scoping) | `cargo test` per crate, or `cargo xtask check` (the CI gates: fmt, clippy native + wasm32, tests, Trunk builds, the Tailwind class guard) |
| `crates/amadeus-server/tests/integration/` | status codes, cache headers, method handling, 308s, SPA fallback, Range requests, SSE headers and per-event flushing, the TTS/STT/RAG proxy matrix, the allowlist, redirects, body limits, local-proxy region failover, the FreeAPI cap setting, the local proxy's command line, no `Referer` on redirect hops; expected values were checked against a running V1 | `cargo test -p amadeus-server` |
| `tools/l2d-compare/` | V1's deployed `reama-cubism-runtime.js` against V2 (`tools/live2d-viewer`) on the same Core, model and texture in headless Chrome + SwiftShader, with deterministic clocks and RNG: screenshots, canvas readbacks, draw-call and RNG traces. During the port 56 scenarios were pixel-identical (maximum difference 0); the 18-scenario required matrix was re-run after the last Live2D fix with the same result | `tools/l2d-compare/run.sh` and `extended.sh` (need V1's `public/` via `V1_PUBLIC`, trunk, node ≥ 22, google-chrome, python3 with PIL + numpy) |
| `tools/live2d-viewer/browser-tests/` | smoke (model visible), V1 parity, lifecycle (context loss and restore, resize, destroy, remount, destroy while loading) | `tools/live2d-viewer/browser-tests/run.sh` (parity is skipped without V1's `public/`) |
| `tools/l2d-probe/` | Node probes that ran V1's bundle with the real Core and produced the golden data the Live2D tests read (`l2d-model-probe/golden/`, `pipeline-probe/trace-A-*.json`, the render-spec probes) | `node …` by hand; the scripts read V1's files from absolute paths of this machine's V1 checkout |

**Not in the repository:** most V1-against-V2 browser harnesses lived only in a scratch directory
of the porting sessions: the side-by-side CDP drivers against a locally running V1 for the app
screen, CONFIG, chat, panels and site pages; the end-to-end runs of the real desktop app under
Xvfb (WebDriver) and the headless-Chrome runs with a fake `window.__TAURI__`; the Node differential
fuzzers of the core managers; and the mock LLM/TTS servers. Their results are recorded in the per-area review reports. Re-verifying those areas means
rebuilding such harnesses against `docs/specs/PARITY_CHECKLIST.md`.

## Coding conventions

* Edition 2021, stable toolchain (see `rust-toolchain.toml`), MSRV 1.90.
* WASM crates must not depend on `rand`/`getrandom`/`uuid`/`tokio`; use `js_sys::Math::random()`
  and `js_sys::Date::now()` / `web_sys::Performance::now()`.
* `amadeus-core` must stay I/O-free and compile on both `wasm32-unknown-unknown` and the host.
* No `unwrap()` on data that comes from the network or storage; degrade gracefully like V1.
* Every crate must pass `cargo clippy -- -D warnings` on its target(s) and `cargo fmt --check`
  (`cargo xtask check` runs both).
* Keep user-visible strings verbatim from V1 (Japanese included).
* When a conditional block can take the focused or clicked element with it and another block
  takes its place, do not rely on Leptos's swap order for native Tab focus. Leptos inserts the new
  nodes before the old ones, React removes first, and Chrome's Tab starting point is a live range
  that follows the difference. Render the alternatives as one branch and re-insert the new block
  in place (`app/chat/blocks.rs`). Read the re-inserted block from the DOM between fixed
  neighbours instead of keeping a list of its elements.

## Build commands

```bash
# Web (frontend + server)
cd apps/web && trunk build --release          # → apps/web/dist (public_url /amadeus/)
cargo build --release -p amadeus-server       # → target/release/amadeus-server, amadeus-local-proxy
AMADEUS_DIST_DIR=apps/web/dist ./target/release/amadeus-server

# Desktop (runs Trunk via beforeBuildCommand)
cargo tauri build                             # from the repository root

# Release-style artifacts into release/, and the CI checks
cargo xtask dist all                          # dist web + dist desktop (current OS)
cargo xtask check
```

See `docs/RELEASE.md` for the release process and `deploy/README.md` for self-hosting.

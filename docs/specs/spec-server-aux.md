# RealAmadeus V2 porting spec: subsystem `server-aux`

Scope: the TTS/STT/RAG proxy API routes, the standalone gcloud/Vertex "local proxy", the Next.js server
policies (`next.config.ts` cache headers, trailing-slash redirect, static serving), the web app manifest,
the kill-switch service worker, the three layouts and the page, and the web `index.html` head, all ported to
Rust (`amadeus-core`, `amadeus-backend`, `amadeus-server`, plus the desktop and UI touch points).

Everything marked **Observed** was checked against the running V1 production server
(`next start -p 9501`, BUILD_ID `1794409851184`) on 2026-11-16 with curl, against mock upstream servers, and
against `server.mjs` started on `127.0.0.1:58103` with `VERTEX_*` unset. No Google API was called.

---

## 0. Sources

Assigned files, all read completely (paths relative to `/home/elvelt_admin/Elvelt/dsuv`):

| File | Lines | Notes |
|---|---|---|
| `app/amadeus/app/api/tts/synthesize/route.ts` | 59 | POST proxy to `${serverUrl}/synthesize` (binary audio) |
| `app/amadeus/app/api/tts/health/route.ts` | 38 | POST proxy to `GET ${serverUrl}/health` (5 s) |
| `app/amadeus/app/api/stt/transcribe/route.ts` | 48 | multipart proxy to `${serverUrl}/transcribe` |
| `app/amadeus/app/api/stt/health/route.ts` | 41 | POST proxy to `GET ${serverUrl}/health` (5 s) |
| `app/amadeus/app/api/rag/embed/route.ts` | 42 | JSON proxy to `${serverUrl}/embed` |
| `app/amadeus/app/api/rag/health/route.ts` | 37 | POST proxy to `GET ${serverUrl}/health` (5 s) |
| `scripts/amadeus-local-proxy/server.mjs` | 295 | standalone Node HTTP server (gcloud token + Vertex Gemini with region failover) |
| `next.config.ts` | 86 | `generateBuildId`, 3 header rules |
| `app/amadeus/app/manifest.ts` | 22 | web app manifest (**V1 never serves it**, see §9) |
| `public/sw.js` | 11 | self-unregistering service worker |
| `app/layout.tsx` | 33 | root layout (html/body, http-equiv metas, LanguageProvider) |
| `app/amadeus/layout.tsx` | 14 | title "Amadeus" for all `/amadeus/**` |
| `app/amadeus/app/layout.tsx` | 44 | metadata/viewport for `/amadeus/app`, `.amadeus-root` wrapper |
| `app/amadeus/app/page.tsx` | 7 | renders `<RealAmadeusApp />` |
| `app/amadeus/app/page.module.css` | 142 | **dead code** (not imported anywhere) |
| `package.json` | 38 | scripts |

Context files read (owned by other specs; used here only for contracts):
`lib/amadeus/services/{tts-service,stt-service,vector-database,ai-service,audio-service}.ts`,
`lib/amadeus/app-settings.ts`, `lib/amadeus/types.ts`, `hooks/useTranslation.tsx` (LanguageProvider only),
`app/amadeus/app/api/chat/{route,shared,stream/route}.ts` (only the local-proxy caller),
`RealAmadeusQT_clone/src/{ttsservice,sttservice,vectordatabase}.cpp` (upstream protocol hints),
`.next/routes-manifest.json`, `.next/server/app-paths-manifest.json`, `app/globals.css`,
`app/amadeus/app/globals.css` (head only), `public/resources/live2d/reama-cubism-runtime.js` (resource paths only).

---

## 1. Inventory: localhost services, ports, env vars

### 1.1 Services and ports

| Service | Default address | Endpoints used | Defined at |
|---|---|---|---|
| TTS server (external, not in repo) | `http://localhost:5100` | `GET /health`, `POST /synthesize` | `lib/amadeus/app-settings.ts:36`, `tts-service.ts:39,247`, `RealAmadeusApp.tsx:3107,3181` |
| STT server (external) | `http://localhost:5101` | `GET /health`, `POST /transcribe` (multipart) | `app-settings.ts:39`, `stt-service.ts:49,374`, `RealAmadeusApp.tsx:3110,3184` |
| RAG / embedding server (external) | `http://localhost:5102` | `GET /health`, `POST /embed` | `app-settings.ts:44`, `vector-database.ts:57,301`, `RealAmadeusApp.tsx:3115,3189` |
| Local Vertex proxy (`server.mjs`) | binds `0.0.0.0:5103`; callers use `http://127.0.0.1:5103` | `GET /health`, `POST /chat`, `OPTIONS *` | `server.mjs:6-7`; caller `app/amadeus/app/api/chat/route.ts:498-516`; settings `app-settings.ts:29-30` |
| Ollama (chat spec) | `http://localhost:11434` | `POST /api/chat` | `chat/route.ts:342`, `chat/stream/route.ts:351`, `app-settings.ts:31` |
| Next dev server | port `9601` | | `package.json` `dev` |
| Next prod server | port `9501` | | `package.json` `start` |
| Internal fake request (chat spec) | `http://localhost/amadeus/app/api/chat` | | `chat/stream/route.ts:290` |
| `getInternalBaseUrl` fallback (chat spec) | `localhost:3000` | | `chat/shared.ts:193` |

Ports 9500/9600 on the same host belong to a different project (`/home/elvelt_admin/Elvelt/elvelt`). Don't
confuse them with this one. In production, traffic arrives through Cloudflare Tunnels (`cloudflared`). That's
why `next.config.ts` sets `CDN-Cache-Control` and `Surrogate-Control`: Cloudflare honours `CDN-Cache-Control`.

The "server URLs" are resolved **on the machine that runs the proxy**. In the web build, `localhost:5100`
therefore means the web server's own localhost, not the visitor's. In the desktop build, where the backend runs
in-process via IPC, it means the user's machine, which matches the original QT app. Keep this difference.

### 1.2 Environment variables

| Variable | Read by | Default | Semantics |
|---|---|---|---|
| `LOCAL_PROXY_HOST` | `server.mjs:226` | `"0.0.0.0"` | Bind host. Also echoed as `bindHost` in `/health`, and used as the base of `new URL(req.url, ...)`. |
| `LOCAL_PROXY_PORT` | `server.mjs:227` | `"5103"` | `Number.parseInt(v, 10)`. Echoed as `bindPort` (a JSON **number**). An invalid value (`NaN`) makes Node's `listen` throw at startup. |
| `VERTEX_PROJECT` | `server.mjs:247,262` | `""` | Fallback project for `/chat` (`payload.vertexProject \|\| env`). Echoed as `project` in `/health`. |
| `VERTEX_LOCATION` | `server.mjs:248,268` | `"us-central1"` | Fallback location. Echoed as `location` in `/health`. |
| `LOCALAPPDATA`, `ProgramFiles`, `USERPROFILE` | `server.mjs:75-83` (Windows only) | (unset: path skipped) | Build candidate paths for `gcloud.cmd`. |
| `PATH` | implicit (`spawn("gcloud")`) | | Lookup for the bare `gcloud` candidate. |

Env vars of other subsystems (names only): `AI_STUDIO_API_KEY`, `GEMINI_API_KEY`
(`chat/vertex-helpers.ts:460`), `HOME`, `PATH` (chat gcloud lookup). `.env` also defines `CANOPYWAVEAPI`,
which no code references. The Next servers take their ports from CLI flags, not env.

Proposed V2 server variables (new; see §8.3): `AMADEUS_HOST` (default `0.0.0.0`), `AMADEUS_PORT` (alias
`PORT`, default `9501`), `AMADEUS_BASE_PATH` (build-time and runtime, default `/amadeus/app`),
`AMADEUS_PUBLIC_DIR` (default `./public`), `AMADEUS_DIST_DIR` (default `./dist`), `AMADEUS_PROXY_ALLOWLIST`
(optional SSRF guard, default unset meaning allow all, which is V1 behaviour), and `RUST_LOG`.

---

## 2. Semantics shared by the six aux proxy routes

These rules apply to `tts/synthesize`, `tts/health`, `stt/transcribe`, `stt/health`, `rag/embed` and
`rag/health`, all under `/amadeus/app/api/`.

### 2.1 Methods and routing (Observed)
- Only `POST` is implemented.
- `OPTIONS <route>` returns `204 No Content` with header `allow: OPTIONS, POST`, an empty body and no CORS
  headers. A preflight with `Origin` gets the same answer and no `Access-Control-*` headers, so cross-origin
  browser calls are impossible. Keep it that way: same-origin only.
- `GET`, `PUT`, and every other method return `405 Method Not Allowed` with an empty body, no
  `Content-Type` and no `Allow` header.
- A trailing slash such as `POST /amadeus/app/api/tts/health/` returns `308` with
  `location: /amadeus/app/api/tts/health` and `Refresh: 0;url=/amadeus/app/api/tts/health` (§7.4).
- Routing is case-sensitive. `/AMADEUS/APP` returns 404.
- An unknown `/amadeus/app/api/*` path returns 404 with Next's HTML not-found page
  (`text/html; charset=utf-8`). V2 may return any 404, for example `{"error":"Not found"}`.
- The global cache headers in §7 are added to every one of these responses, including 204, 400, 405 and 500.

### 2.2 Lenient JSON body parsing
The JSON routes do `(await request.json().catch(() => null))`:
- The body is parsed **regardless of `Content-Type`**. Observed: `Content-Type: text/plain` with a JSON body
  works. In Rust, **do not use axum's `Json` extractor**, which returns 415 for non-JSON content types. Take
  `Bytes` and call `serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null)`.
- An empty body or invalid JSON gives `null`, which behaves like "every field missing".
- A non-object payload (array, string, number, `true`, `null`) gives every field `undefined`, because of
  optional chaining.

### 2.3 Field typing quirk: `payload?.x?.trim()`
- A missing field or `null` gives `undefined`, which counts as missing.
- A string is trimmed with JS `trim` (§2.4).
- **Any other JSON type** (number, boolean, object, array) throws a TypeError ("trim is not a function")
  **before the `try` block**. Next turns this into **HTTP 500 with an empty body and no `Content-Type`**
  (Observed with `{"serverUrl":5}` on `tts/health`).
- This applies to `serverUrl`, `text` and `speaker` on the JSON routes. Fields are evaluated in source order,
  and every field is read before any check:
  - `tts/synthesize` reads `serverUrl`, then `text`, then `speaker`. A non-string `speaker` gives 500 even when
    `serverUrl` is missing.
  - `rag/embed` reads `serverUrl`, then `text`.
  - The three health routes read only `serverUrl`.
- Recommended helper:

```rust
/// Mirrors `payload?.[key]?.trim()`; Err(()) == JS TypeError (=> HTTP 500, empty body)
fn opt_trimmed(payload: &Value, key: &str) -> Result<Option<String>, ()> {
    match payload.as_object().and_then(|o| o.get(key)) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(js_trim(s).to_owned())),
        Some(_) => Err(()),
    }
}
```

An empty string after trimming counts as "missing", because the code checks `if (!serverUrl)`.

### 2.4 `js_trim` (exact JS `String.prototype.trim`)
JS trims WhiteSpace and LineTerminator characters: TAB, VT, FF, SP, U+00A0, U+FEFF, every `Zs` character
(U+1680, U+2000–U+200A, U+202F, U+205F, U+3000), LF, CR, U+2028 and U+2029. Rust's `char::is_whitespace`
differs in two places: it **includes** U+0085 (NEL), which JS does not trim, and it **excludes** U+FEFF, which
JS trims.

```rust
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c == '\u{FEFF}' || (c.is_whitespace() && c != '\u{85}'))
}
```

Japanese input often carries U+3000 (ideographic space). Both JS and Rust trim it.

### 2.5 Upstream URL construction
URLs are built by naive concatenation: `` `${serverUrl}/health` ``, `/synthesize`, `/transcribe`, `/embed`.
Nothing is normalized. `"http://h:5100/"` becomes `http://h:5100//health`; whatever the upstream does with
`//health` is its own business. Rust: `format!("{server_url}/health")`, with no `Url::join`.

### 2.6 Outbound request characteristics (undici `fetch`, Observed at the mock upstream)
- Headers sent: `host`, `connection: keep-alive`, `accept: */*`, `accept-language: *`,
  `sec-fetch-mode: cors`, `user-agent: node`, `accept-encoding: gzip, deflate`. JSON calls also send
  `Content-Type: application/json` and `content-length`. The `/health` GETs send no body.
- Redirects are followed (up to 20; 301/302 turn POST into GET per the fetch spec).
- Responses are decompressed automatically. **reqwest must enable the `gzip`, `deflate` and `brotli`
  features**; otherwise a compressed upstream body would be forwarded without its `Content-Encoding` header and
  the TTS audio would arrive corrupted.
- undici's defaults are `headersTimeout` = 300 s and `bodyTimeout` = 300 s (inactivity). These are the only
  timeouts on `synthesize`, `transcribe` and `embed`. Rust: `ClientBuilder::read_timeout(Duration::from_secs(300))`.
  Don't set a total timeout.
- undici ignores `HTTP_PROXY`/`HTTPS_PROXY`. **reqwest honours them by default**, which can break
  `http://localhost:510x`. Call `ClientBuilder::no_proxy()` for parity.
- The `user-agent: node` header doesn't matter to the upstreams. V2 may send `RealAmadeus/2.x`.

### 2.7 Error message strings (Observed) and the Rust mapping
The catch blocks return `error.message`. The exact V1 strings are:

| Situation | V1 `error.message` |
|---|---|
| Connection refused, DNS failure, reset, TLS error, unsupported scheme, undici 300 s timeout | `fetch failed` |
| URL cannot be parsed, e.g. `serverUrl="abc"` | `Failed to parse URL from abc/health` (the full concatenated URL) |
| 5 s AbortController fired (health routes) | `This operation was aborted` |

Rust mapping: parse with `reqwest::Url::parse(&url)` first; on failure, use
`format!("Failed to parse URL from {url}")`. An expired deadline (`tokio::time::timeout`) maps to
`"This operation was aborted"`. Every other `reqwest::Error`, including "URL scheme is not allowed", maps to
`"fetch failed"`. The V1 UI never shows these strings (the signals' callbacks ignore the message, see §5.3), so
exact parity is cheap but not critical.

### 2.8 Response formatting
- `NextResponse.json(x, {status})` produces `content-type: application/json` (no charset). The body is compact
  `JSON.stringify` with raw UTF-8, not `\u` escapes, and keys keep their insertion order.
- Use `serde_json` with the **`preserve_order`** feature when rebuilding objects such as the health spread
  (§3.2), so key order matches.
- The default status is 200.
- A Cache-Control header set by a route is **overridden** by the global rules (§7.2). Observed: `synthesize`
  sets `Cache-Control: no-store`, but the response carries
  `Cache-Control: no-store, no-cache, must-revalidate, proxy-revalidate, max-age=0`.

### 2.9 Upstream body parsing
`await response.json().catch(() => null)`. A non-JSON body, an empty body, or the JSON literal `null` all give
`null`. Any other JSON value (object, array, number, string, bool) is kept as is.

### 2.10 Security note: SSRF
`serverUrl` is fully client-controlled. Any visitor of the public site can make the server GET
`<anything>/health` or POST to `<anything>/synthesize|/transcribe|/embed`, including internal hosts. V1 has no
guard. V2 keeps the behaviour by default but should offer `AMADEUS_PROXY_ALLOWLIST` (comma-separated
`host[:port]` patterns) for the public web deployment. See Open questions.

---

## 3. Route specifications

`{base}` = `/amadeus/app`. All six routes are `POST {base}/api/...`.

### 3.1 `POST {base}/api/tts/synthesize`

V1 source (verbatim, `app/amadeus/app/api/tts/synthesize/route.ts`):

```ts
import { NextResponse } from "next/server";

interface TtsSynthesizePayload {
  serverUrl: string;
  text: string;
  speaker: string;
  useCache?: boolean;
}

export async function POST(request: Request): Promise<NextResponse> {
  const payload = (await request.json().catch(() => null)) as TtsSynthesizePayload | null;
  const serverUrl = payload?.serverUrl?.trim();
  const text = payload?.text?.trim();
  const speaker = payload?.speaker?.trim() || "christina-jp";

  if (!serverUrl) {
    return NextResponse.json({ error: "serverUrl is required" }, { status: 400 });
  }
  if (!text) {
    return NextResponse.json({ error: "text is required" }, { status: 400 });
  }

  try {
    const response = await fetch(`${serverUrl}/synthesize`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        text,
        speaker,
        use_cache: payload?.useCache ?? true,
      }),
    });

    if (!response.ok) {
      const errorBody = await response.text();
      return NextResponse.json(
        { error: `Synthesis failed (${response.status}): ${errorBody}` },
        { status: response.status },
      );
    }

    const audioData = await response.arrayBuffer();
    return new NextResponse(audioData, {
      status: 200,
      headers: {
        "Content-Type": response.headers.get("Content-Type") ?? "audio/wav",
        "Cache-Control": "no-store",
      },
    });
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Synthesis failed" },
      { status: 500 },
    );
  }
}
```

**Request** (from the V1 client, `tts-service.ts:143-154`): `Content-Type: application/json`, body
`{"serverUrl": string, "speaker": string, "text": string, "useCache": true}`.

**Algorithm**
1. `payload = parse_json_lenient(body)` (§2.2).
2. `serverUrl = opt_trimmed(payload, "serverUrl")?`, `text = opt_trimmed(payload, "text")?`,
   `speaker = opt_trimmed(payload, "speaker")?`. On a TypeError, return 500 with an empty body (§2.3).
3. `speaker = if speaker is None or "" { "christina-jp" } else { speaker }`. JS `||`: an empty-after-trim
   string falls back too.
4. If `serverUrl` is missing or empty, return **400** `{"error":"serverUrl is required"}`.
5. If `text` is missing or empty, return **400** `{"error":"text is required"}`. This check runs after the
   serverUrl check.
6. `use_cache = payload.useCache` when present and not null; otherwise `true`. JS `?? true`: **the value
   passes through untouched**. Observed: `"useCache":"nope"` sends `"use_cache":"nope"`, `false` stays
   `false`, `null` becomes `true`. Keep it as a `serde_json::Value`.
7. `POST {serverUrl}/synthesize` with `Content-Type: application/json` and the compact body
   `{"text":<text>,"speaker":<speaker>,"use_cache":<use_cache>}`, keys in exactly that order.
   Observed upstream body: `{"text":"こんにちは","speaker":"christina-jp","use_cache":true}`. The text was
   trimmed from `"  こんにちは  "`.
8. Upstream answers with a non-2xx status (`!response.ok`):
   - `errorBody = response.text()`: the whole body, decoded as UTF-8 with replacement characters.
   - Return status = **the upstream status** with body
     `{"error":"Synthesis failed (<status>): <errorBody>"}`. Observed:
     `{"error":"Synthesis failed (500): 合成エラー detail"}` at status 500.
9. Upstream answers 2xx: buffer the entire body (`arrayBuffer`) and return **200** with:
   - `Content-Type` = the upstream `Content-Type` value **verbatim**. Observed: `audio/x-wav` passed through.
     If the header is absent, use `audio/wav`. JS `??` only falls back on null: an upstream header that is
     present but empty stays `""`.
   - `Cache-Control: no-store`, which the global rule replaces (§2.8).
   - Body = the upstream bytes. V1 streams it chunked with no Content-Length; Rust may set Content-Length.
   - An empty 2xx body is forwarded as an empty 200. The client then emits "Received empty audio data".
10. A transport error (connect, read, URL parse) returns **500** `{"error": <message per §2.7>}`. The fallback
    string `"Synthesis failed"` is used only when the thrown value is not an Error, which never happens in
    practice.

No explicit timeout applies (undici's 300 s inactivity default, §2.6).

**Observed matrix** (mock upstream at 127.0.0.1:58100 "good", 58101 "bad", port 1 refused):

| Request body | Status | Response |
|---|---|---|
| `{}` | 400 | `{"error":"serverUrl is required"}` |
| `{"serverUrl":"http://127.0.0.1:58100"}` | 400 | `{"error":"text is required"}` |
| `{"serverUrl":"…58100","text":"  こんにちは  "}` | 200 | `content-type: audio/x-wav`, body = upstream bytes (`RIFF$\0\0\0WAVEfmt `) |
| `…,"speaker":"  christina ","useCache":false` | 200 | upstream got `{"text":"x","speaker":"christina","use_cache":false}` |
| `…,"speaker":"   ","useCache":null` | 200 | upstream got `"speaker":"christina-jp","use_cache":true` |
| `…58101` (upstream 500, text body `合成エラー detail`) | 500 | `{"error":"Synthesis failed (500): 合成エラー detail"}` |
| `http://127.0.0.1:1` | 500 | `{"error":"fetch failed"}` |

**Client behaviour (V1, `tts-service.ts:124-174`).** For reference only; owned by the services spec.
- The client refuses before calling the server when TTS is disabled (`"TTS is not enabled"`) or the text is
  empty after trim (`"Empty text provided"`).
- It caches object URLs under `simpleHash(\`${normalized}|${speaker}|${serverUrl}\`)`.
- On a non-OK response it emits `data?.error ?? \`Synthesis failed (${status})\``.
- On OK it reads `response.blob()`; size 0 gives `"Received empty audio data"`; otherwise it calls
  `URL.createObjectURL(blob)`. The blob's MIME type comes from the response Content-Type.
- Playback happens through `new Audio(url)` (`audio-service.ts:45`).

### 3.2 `POST {base}/api/tts/health`

V1 source (verbatim):

```ts
import { NextResponse } from "next/server";

interface TtsHealthPayload {
  serverUrl: string;
}

export async function POST(request: Request): Promise<NextResponse> {
  const payload = (await request.json().catch(() => null)) as TtsHealthPayload | null;
  const serverUrl = payload?.serverUrl?.trim();
  if (!serverUrl) {
    return NextResponse.json({ available: false, error: "serverUrl is required" }, { status: 400 });
  }

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 5000);
  try {
    const response = await fetch(`${serverUrl}/health`, {
      method: "GET",
      signal: controller.signal,
    });
    const body = await response.json().catch(() => null);
    if (!response.ok) {
      return NextResponse.json({ available: false, error: `${response.status} ${response.statusText}` });
    }
    return NextResponse.json({
      available: true,
      ...(body && typeof body === "object" ? body : {}),
    });
  } catch (error) {
    return NextResponse.json({
      available: false,
      error: error instanceof Error ? error.message : "Health check failed",
    });
  } finally {
    clearTimeout(timeout);
  }
}
```

**Request** (client `tts-service.ts:176-186`): `{"serverUrl": string}`.

**Algorithm**
1. Parse leniently. Get `serverUrl = opt_trimmed(payload,"serverUrl")?`; a non-string gives 500 with an empty
   body.
2. If it is missing or empty, return **400** `{"available":false,"error":"serverUrl is required"}`. Note the
   key order: `available`, then `error`.
3. `deadline = now + 5000 ms`. The timer starts **before** the request.
4. `GET {serverUrl}/health` with no body. If the deadline fires before the response headers arrive, return
   **200** `{"available":false,"error":"This operation was aborted"}`. Observed: elapsed 5.02 s.
5. **TTS-specific ordering: read the body BEFORE checking the status.**
   `body = response.json().catch(() => null)`. The read is still under the 5 s deadline. If the deadline fires
   during the body read, the read rejects, the `.catch` swallows it, and `body = null` (no error).
6. If the status is not 2xx, return **200** `{"available":false,"error":"<status> <statusText>"}`.
   `statusText` is the upstream HTTP/1.1 reason phrase verbatim. Observed: `"503 Service Unavailable"`.
   (Over HTTP/2 the reason would be empty, giving `"503 "` with a trailing space.)
7. Status 2xx: start with the object `{"available": true}`. If `body` is a JSON **object or array** (JS
   `body && typeof body === "object"`), spread its own enumerable keys in order. **Spread overrides**: an
   upstream `available` key replaces the value but keeps the first position. Observed: upstream
   `{"status":"ok","available":"yes-from-upstream","runtime":{…},"torch":{…}}` produced
   `{"available":"yes-from-upstream","status":"ok","runtime":{…},"torch":{…}}`.
   - An array body spreads its indices as string keys `"0"`, `"1"`, … (JS puts integer-like keys first in
     ascending order). Rarely matters; implement it if cheap.
   - Strings, numbers, booleans and `null` add nothing.
   - Return **200**.
8. Any other transport error returns **200** `{"available":false,"error":"<message per §2.7>"}`.

**Only the missing-serverUrl case is non-200.** Every upstream failure is reported as 200 with
`available:false`.

**Recommended Rust skeleton (shared by the three health routes):**

```rust
pub enum HealthBodyMode { ReadBeforeStatus /*tts*/, ReadAfterOk /*stt*/, Ignore /*rag*/ }

pub async fn probe_health(http: &Client, server_url: &str, mode: HealthBodyMode) -> Value /* status 200 */ {
    let url = format!("{server_url}/health");
    let deadline = Instant::now() + Duration::from_millis(5000);
    let parsed = match Url::parse(&url) { Ok(u) => u, Err(_) => return fail(format!("Failed to parse URL from {url}")) };
    let resp = match timeout_at(deadline, http.get(parsed).send()).await {
        Err(_) => return fail("This operation was aborted"),
        Ok(Err(_)) => return fail("fetch failed"),
        Ok(Ok(r)) => r,
    };
    let status = resp.status();
    let read_body = |r| async move { timeout_at(deadline, r.bytes()).await.ok().and_then(|b| b.ok())
                                     .and_then(|b| serde_json::from_slice::<Value>(&b).ok()) };
    match mode {
      ReadBeforeStatus => { let body = read_body(resp).await; if !status.is_success() { return not_ok(status) } spread(body) }
      ReadAfterOk      => { if !status.is_success() { return not_ok(status) } spread(read_body(resp).await) }
      Ignore           => { if !status.is_success() { return not_ok(status) } json!({"available": true}) }
    }
}
// not_ok: {"available":false,"error": format!("{} {}", status.as_u16(), reason)}  reason = hyper ReasonPhrase ext if present else canonical_reason().unwrap_or("")
// fail(msg): {"available":false,"error":msg}
```

**Client behaviour (V1).** `nextAvailable = response.ok && Boolean(health?.available)`. That is JS
truthiness, so `"yes"` counts as available and `0`, `""` and `false` count as unavailable. When available, the
client derives runtime info (§5.4). It never displays the `error` string.

### 3.3 `POST {base}/api/stt/transcribe`

V1 source (verbatim):

```ts
import { NextResponse } from "next/server";

export async function POST(request: Request): Promise<NextResponse> {
  const form = await request.formData().catch(() => null);
  if (!form) {
    return NextResponse.json({ error: "Invalid form data" }, { status: 400 });
  }

  const audio = form.get("audio");
  const provider = String(form.get("provider") ?? "groq");
  const language = String(form.get("language") ?? "ja");
  const serverUrl = String(form.get("serverUrl") ?? "").trim();

  if (!serverUrl) {
    return NextResponse.json({ error: "serverUrl is required" }, { status: 400 });
  }
  if (!(audio instanceof Blob)) {
    return NextResponse.json({ error: "audio file is required" }, { status: 400 });
  }

  try {
    const upstreamForm = new FormData();
    upstreamForm.append("audio", audio, "recording.webm");
    upstreamForm.append("provider", provider);
    upstreamForm.append("language", language);

    const response = await fetch(`${serverUrl}/transcribe`, {
      method: "POST",
      body: upstreamForm,
    });
    const body = await response.json().catch(() => null);
    if (!response.ok) {
      return NextResponse.json(
        {
          error: (body as { error?: string } | null)?.error ?? `Transcription failed (${response.status})`,
        },
        { status: response.status },
      );
    }
    return NextResponse.json(body ?? {});
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Transcription failed" },
      { status: 500 },
    );
  }
}
```

**Request** (client `stt-service.ts:212-221`): `multipart/form-data` with parts in this order:
- `audio`: a Blob with filename `recording.webm`. Its Content-Type is the MediaRecorder `mimeType`, for example
  Chrome `audio/webm;codecs=opus`, Safari `audio/mp4`, Firefox `audio/ogg;codecs=opus`, or `audio/webm` as a
  fallback.
- `provider`: `"groq"`, `"openai"` or `"local"`.
- `language`: for example `"ja"`.
- `serverUrl`.

The client already enforces two rules: a recording shorter than 1024 bytes gives the client-side error
`"Recording too short"`, and recordings are capped at 30 000 ms.

**Form parsing semantics** (`request.formData()`):
- Accepts `multipart/form-data` **and** `application/x-www-form-urlencoded`.
  - Observed: a urlencoded body `serverUrl=http://…` returns 400 `"audio file is required"`.
  - Observed: a urlencoded body `a=b` returns 400 `"serverUrl is required"`.
- Any other or missing content type, malformed multipart, or a JSON body returns **400**
  `{"error":"Invalid form data"}` (Observed).
- `form.get(name)` returns the **first** entry with that name, or null.
- A multipart part is a **File/Blob** iff its `Content-Disposition` has a `filename` parameter. Otherwise it is
  a string, even when it carries a Content-Type.
- `String(x ?? default)`:
  - A missing field gets the default (`"groq"`, `"ja"`, `""`).
  - A present but empty field stays `""`. Observed: `provider=` forwards an empty provider.
  - A File in `provider` or `language` stringifies to `"[object File]"`. A File in `serverUrl` also becomes
    `"[object File]"`, which then fails URL parsing and returns 500
    `"Failed to parse URL from [object File]/transcribe"`. This edge never happens in practice.
- `serverUrl` is js-trimmed. `provider` and `language` are **not** trimmed.

**Algorithm**
1. Parse the form. On failure, return 400 `{"error":"Invalid form data"}`.
2. Read the fields as above.
3. If `serverUrl` is empty, return **400** `{"error":"serverUrl is required"}`. This is checked **before**
   the audio check.
4. If `audio` is not a file part, return **400** `{"error":"audio file is required"}`.
5. Build the upstream multipart (Observed body below). Part order: `audio`, `provider`, `language`.
   `serverUrl` is **not** forwarded.

```
------formdata-undici-008668331459
Content-Disposition: form-data; name="audio"; filename="recording.webm"
Content-Type: audio/webm;codecs=opus

<audio bytes>
------formdata-undici-008668331459
Content-Disposition: form-data; name="provider"

openai
------formdata-undici-008668331459
Content-Disposition: form-data; name="language"

en
------formdata-undici-008668331459--
```

   - The filename is **always** `recording.webm`, whatever the incoming name or real format.
   - The part's Content-Type is the incoming part's Content-Type verbatim. When that is missing or empty, use
     `application/octet-stream` (Observed: curl's default octet-stream was forwarded unchanged).
   - The boundary format doesn't matter; reqwest's own random boundary is fine.
6. `POST {serverUrl}/transcribe` with no explicit timeout. Then `body = json().catch(null)`.
7. Upstream non-2xx: return **upstream status** with
   `{"error": body?.error ?? "Transcription failed (<status>)"}`. `body?.error` is taken **as any JSON value**
   when `body` is an object whose `error` key exists and is not null. It can be a string, an object or a number;
   copy the `Value`. Observed: upstream 422 `{"error":"bad audio"}` returned 422 `{"error":"bad audio"}`.
   - A FastAPI-style `{"detail": …}` gives `"Transcription failed (422)"`.
   - `body` null or a non-object gives the fallback.
8. Upstream 2xx: return **200** with `body` when it is not null, else `{}`. Arrays and primitives pass through
   verbatim. Observed: `{"text":"こんにちは","provider_used":"mock"}` was returned unchanged.
9. A transport error returns **500** `{"error": <msg §2.7>}`. The fallback is `"Transcription failed"`.

**Body size:** V1 imposes no limit. Set axum's `DefaultBodyLimit` on this route to at least 32 MiB; the axum
default of 2 MiB would be a regression for long or uncompressed recordings.

**Rust parsing recommendation:** use `multer` directly. axum's `Multipart` rejects urlencoded bodies; map its
rejection to `"Invalid form data"` and handle `application/x-www-form-urlencoded` separately with
`form_urlencoded::parse`, where every value is a string. Keep only the first occurrence of each name. Decode
text parts as lossy UTF-8. Content-type matching is case-insensitive on the essence
(`multipart/form-data; boundary=…`).

**Client behaviour.** On non-OK the client emits `data?.error ?? \`Transcription failed (${status})\``. On OK it
emits `data?.text ?? ""`. The UI shows `"音声入力: OK"` when the trimmed text is non-empty and
`"音声入力に失敗しました"` on error (`RealAmadeusApp.tsx:996-1003`). It never shows the message itself.

### 3.4 `POST {base}/api/stt/health`

V1 source (verbatim):

```ts
import { NextResponse } from "next/server";

interface SttHealthPayload {
  serverUrl: string;
}

export async function POST(request: Request): Promise<NextResponse> {
  const payload = (await request.json().catch(() => null)) as SttHealthPayload | null;
  const serverUrl = payload?.serverUrl?.trim();
  if (!serverUrl) {
    return NextResponse.json({ available: false, error: "serverUrl is required" }, { status: 400 });
  }

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 5000);
  try {
    const response = await fetch(`${serverUrl}/health`, {
      method: "GET",
      signal: controller.signal,
    });
    if (!response.ok) {
      return NextResponse.json({
        available: false,
        error: `${response.status} ${response.statusText}`,
      });
    }
    const body = await response.json().catch(() => null);
    return NextResponse.json({
      available: true,
      ...(body && typeof body === "object" ? body : {}),
    });
  } catch (error) {
    return NextResponse.json({
      available: false,
      error: error instanceof Error ? error.message : "Health check failed",
    });
  } finally {
    clearTimeout(timeout);
  }
}
```

The route is identical to §3.2 with one exception: **the body is read only after the OK check**
(`HealthBodyMode::ReadAfterOk`). A non-OK body is never read. The body read is still under the same 5 s
deadline, and a timeout during the read gives `body = null`, so the reply is `{"available":true}`.

Observed results:
- good upstream: the same spread output as TTS;
- 503 upstream: `{"available":false,"error":"503 Service Unavailable"}`;
- no body: 400 `{"available":false,"error":"serverUrl is required"}`;
- slow upstream: `{"available":false,"error":"This operation was aborted"}` after 5.02 s.

Client: `available = response.ok && Boolean(data?.available)` (`stt-service.ts:244-245`).

### 3.5 `POST {base}/api/rag/embed`

V1 source (verbatim):

```ts
import { NextResponse } from "next/server";

interface RagEmbedPayload {
  serverUrl: string;
  text: string;
}

export async function POST(request: Request): Promise<NextResponse> {
  const payload = (await request.json().catch(() => null)) as RagEmbedPayload | null;
  const serverUrl = payload?.serverUrl?.trim();
  const text = payload?.text?.trim();
  if (!serverUrl) {
    return NextResponse.json({ error: "serverUrl is required" }, { status: 400 });
  }
  if (!text) {
    return NextResponse.json({ error: "text is required" }, { status: 400 });
  }

  try {
    const response = await fetch(`${serverUrl}/embed`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ text }),
    });
    const body = await response.json().catch(() => null);
    if (!response.ok) {
      return NextResponse.json(
        { error: (body as { error?: string })?.error ?? `Embedding failed (${response.status})` },
        { status: response.status },
      );
    }
    return NextResponse.json(body ?? {});
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Embedding failed" },
      { status: 500 },
    );
  }
}
```

**Request** (client `vector-database.ts:279-288`): `{"serverUrl": string, "text": string}`. The client sends
the raw, untrimmed content; the **server trims it**.

**Algorithm**
1. Parse leniently. Read `serverUrl` and `text` with `opt_trimmed`; a non-string gives 500 with an empty body.
2. Missing serverUrl: 400 `{"error":"serverUrl is required"}`. Missing text: 400 `{"error":"text is required"}`.
3. `POST {serverUrl}/embed` with `Content-Type: application/json` and body `{"text":<trimmed>}`. Observed: the
   upstream got `{"text":"hello 世界"}` from `"  hello 世界  "`.
4. `body = json().catch(null)`.
5. Upstream non-2xx: **upstream status** with `{"error": body?.error ?? "Embedding failed (<status>)"}`, under
   the same any-JSON-value rule as §3.3. Observed: upstream 500 with a text body gave
   `{"error":"Embedding failed (500)"}`.
6. Upstream 2xx: `body ?? {}` at status 200. Observed: `{"embedding":[0.25,-0.5,1],"dim":3}` passed through
   verbatim. Integers stay integers (`1`, not `1.0`), so keep the `Value` without converting to f64.
7. A transport error gives 500 `{"error": <msg>}`. Observed: `{"error":"fetch failed"}` and
   `{"error":"Failed to parse URL from http://127.0.0.1:58100x/embed"}`. The fallback is `"Embedding failed"`.

**Client.** `requestEmbedding` returns `[]` unless `response.ok && Array.isArray(data.embedding)`; otherwise it
maps each item with `Number(value)`. Every failure is silent. See §5.

### 3.6 `POST {base}/api/rag/health`

V1 source (verbatim):

```ts
import { NextResponse } from "next/server";

interface RagHealthPayload {
  serverUrl: string;
}

export async function POST(request: Request): Promise<NextResponse> {
  const payload = (await request.json().catch(() => null)) as RagHealthPayload | null;
  const serverUrl = payload?.serverUrl?.trim();
  if (!serverUrl) {
    return NextResponse.json({ available: false, error: "serverUrl is required" }, { status: 400 });
  }

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 5000);
  try {
    const response = await fetch(`${serverUrl}/health`, {
      method: "GET",
      signal: controller.signal,
    });
    if (!response.ok) {
      return NextResponse.json({
        available: false,
        error: `${response.status} ${response.statusText}`,
      });
    }
    return NextResponse.json({ available: true });
  } catch (error) {
    return NextResponse.json({
      available: false,
      error: error instanceof Error ? error.message : "Health check failed",
    });
  } finally {
    clearTimeout(timeout);
  }
}
```

Same as §3.4, but **the body is never read** and the success reply is exactly `{"available":true}`
(`HealthBodyMode::Ignore`). Observed: good gives `{"available":true}`; 503 gives
`{"available":false,"error":"503 Service Unavailable"}`; refused gives
`{"available":false,"error":"fetch failed"}`; slow gives `This operation was aborted` after 5.02 s.

---

## 4. Upstream service contracts (external servers, not in this repo)

These are inferred from the V1 routes and clients and confirmed by the QT clone
(`RealAmadeusQT_clone/src/ttsservice.cpp:116-142,229-254`, `sttservice.cpp:227-310`,
`vectordatabase.cpp:239-397`).

| Server | Endpoint | Request | Success response | Fields consumed |
|---|---|---|---|---|
| TTS :5100 | `GET /health` | none | 200 JSON object | `runtime.using_gpu` (bool), `runtime.device` (string; the special value `"not_loaded"`), `runtime.gpu_name`, `runtime.attn_implementation`, `torch.cuda_available` (bool), `torch.detected_gpu_name`. All other keys are passed through. |
| TTS :5100 | `POST /synthesize` | JSON `{"text","speaker","use_cache"}` | 200, audio bytes (WAV; Content-Type e.g. `audio/wav`) | bytes and Content-Type |
| STT :5101 | `GET /health` | none | 200 JSON | the spread is passed to the client, which only reads `available` |
| STT :5101 | `POST /transcribe` | multipart `audio` (file), `provider`, `language` | 200 JSON `{"text": string, …}` | `text`; errors: non-2xx `{"error": string}` |
| RAG :5102 | `GET /health` | none | any 2xx | status only |
| RAG :5102 | `POST /embed` | JSON `{"text"}` | 200 JSON `{"embedding": number[]}` | `embedding`; errors: non-2xx, optional `{"error"}` |

The QT app uploads `filename="recording.wav"` (WAV). The web app always declares `recording.webm`, even for
`audio/mp4` or `audio/ogg`. The STT server must sniff the real format. V2 keeps `recording.webm` for parity.
The QT clients use the same 5 s transfer timeout for `/health` (`setTransferTimeout(5000)`).

---

## 5. V1 client-side consumers (context for the Transport layer and regression tests)

### 5.1 localStorage keys touched by these features (must stay identical)
All values are stored as strings. `setInt` writes `String(Math.trunc(v))`; `setFloat` writes `String(v)` (JS
number formatting: `"0.7"`, `"1"`, `"0.75"`). `initializeDefaults()` writes any missing key from
`DEFAULT_SETTINGS` when the app mounts.

| Key | Type | Default | Notes |
|---|---|---|---|
| `Config_Experimental_TTS_Enabled` | int | `0` | `=== 1` means enabled |
| `Config_Experimental_TTS_Speaker` | string | `christina-jp` | UI options `TTS_SPEAKER_NAMES = ["christina-jp", "christina"]` (`RealAmadeusApp.tsx:326`) |
| `Config_Experimental_TTS_ServerUrl` | string | `http://localhost:5100` | |
| `Config_Experimental_STT_Enabled` | int | `0` | |
| `Config_Experimental_STT_Provider` | string | `groq` | UI options `STT_PROVIDER_NAMES = ["groq", "openai", "local"]` (`:327`) |
| `Config_Experimental_STT_ServerUrl` | string | `http://localhost:5101` | |
| `Config_Experimental_STT_Language` | string | `ja` | |
| `Config_Experimental_RAG_Enabled` | int | `0` | |
| `Config_Experimental_RAG_TopK` | int | `5` | setter bounds 1..20 (`Math.trunc`) |
| `Config_Experimental_RAG_Threshold` | float | `0.7` | setter bounds 0..1 |
| `Config_Experimental_RAG_ServerUrl` | string | `http://localhost:5102` | |
| `RealAmadeus.VectorStore` | JSON | (absent) | `{"version":1,"entries":[{"id":uuid,"content":str,"category":str,"embedding":number[],"metadata":{}}]}`. Entries without an `id` or with an empty `embedding` are skipped on load. |
| `Config_ApiProvider` | int | `8` | `7` = local proxy (`PROVIDER_LOCAL`) |
| `Config_LocalProxyHost` | string | `127.0.0.1` | |
| `Config_LocalProxyPort` | string | `""` | empty means the chat route uses `"5103"` |
| `Config_VertexProject` | string | `""` | forwarded as `vertexProject` |
| `Config_VertexLocation` | string | `us-central1` | forwarded as `vertexLocation` |
| `Config_WebSearch` | int | `0` | forwarded as `webSearch` (bool) |
| `Config_ModelName_7` / `Config_ModelName` | string | `""` | forwarded as `model`; the proxy defaults it to `gemini-2.5-flash` |

The full table is in `lib/amadeus/app-settings.ts:5-50`. That table is owned by the core/settings spec.

### 5.2 When the aux routes are called
- **On app mount**, `tts/health`, `stt/health` and `rag/health` are all called **unconditionally**, whether or
  not each feature is enabled (`RealAmadeusApp.tsx:3972-3977`). Every page load therefore makes the server GET
  `http://localhost:5100/health`, `:5101/health` and `:5102/health` (defaults). With nothing listening these
  fail fast with "fetch failed".
- Each service constructor calls its own health check again when the feature is enabled.
- Health checks also run when the feature is toggled on, when `serverUrl` changes, and (TTS only) every time
  the CONFIG panel opens (`RealAmadeusApp.tsx:3191`).
- There is no periodic polling.
- `synthesize` runs per TTS text chunk from a queue: one in flight at a time (`ttsSynthInFlightRef`).
- `transcribe` runs once per recording, and **only when the browser has no `SpeechRecognition` or
  `webkitSpeechRecognition`**. Chrome and Safari use the Web Speech API, so the STT server is reached mainly
  from Firefox.
- `rag/embed` runs on `VectorDatabase.addEntry` and `search`, only when RAG is enabled.

### 5.3 How the client reads results (the Rust UI must replicate this)
- `ok` = HTTP 200–299.
- `json` = `response.json().catch(() => null)`.
- Health: `ok && js_truthy(json.available)`. `js_truthy`: Null gives false, Bool gives itself, a Number gives
  `n != 0 && !NaN`, a String gives `!is_empty()`, and Array or Object give true.
- The error strings are passed to signals whose UI callbacks ignore them.

### 5.4 TTS runtime-details derivation (reference; owned by the services spec, `tts-service.ts:196-231`)
1. If not available or the body is missing, use `(usingGpu=false, device="unavailable", details="unavailable")`.
2. Otherwise:
   - `runtime = health.runtime ?? {}` and `torch = health.torch ?? {}`;
   - `usingGpu = Boolean(runtime.using_gpu)`;
   - `device = runtime.device ?? "unknown"`;
   - `gpuName = runtime.gpu_name ?? ""` and `attn = runtime.attn_implementation ?? ""`;
   - `cuda = Boolean(torch.cuda_available)` and `detected = torch.detected_gpu_name ?? ""`;
   - `base = gpuName ? \`${device} (${gpuName})\` : device`;
   - `details = attn ? \`${base}, ${attn}\` : base`.
3. If `device === "not_loaded"`, then `usingGpu = false` and:
   - `details = cuda ? (detected ? \`not_loaded (CUDA available: ${detected})\` : "not_loaded (CUDA available)") : "not_loaded (CUDA unavailable)"`.
4. A fetch exception gives `available=false` and `"unavailable"`.

---

## 6. Local proxy: `scripts/amadeus-local-proxy/server.mjs` → Rust binary `amadeus-local-proxy`

### 6.1 Purpose and caller
A tiny standalone HTTP server (`npm run start:amadeus-proxy`) that turns a chat request into a Vertex AI Gemini
`generateContent` call. It authenticates with **`gcloud auth print-access-token`** and fails over across
regions.

The V1 chat route reaches it for provider `PROVIDER_LOCAL = 7` (`chat/route.ts:498-516`, owned by the chat spec
but reproduced here because it defines the proxy's input):

```ts
      case PROVIDER_LOCAL: {
        let host = payload.localProxyHost?.trim() || "127.0.0.1";
        let port = payload.localProxyPort?.trim() || "5103";
        let url = `http://${host}:${port}/chat`;
        if (host.startsWith("http")) {
          url = `${host}:${port}/chat`;
        }
        const response = await fetch(url, {
          method: "POST", cache: "no-store",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify(payload),
        });
        const body = (await response.json().catch(() => null)) as { response?: string; error?: string } | null;
        if (!response.ok) {
          return jsonError(body?.error ?? `${response.status} ${response.statusText}`, response.status);
        }
        return sendResponse(body?.response ?? "");
      }
```

- The **entire** ChatRequestPayload is forwarded: `messages`, `provider`, `apiKey`, `model`, `webSearch`,
  `vertexProject`, `vertexLocation`, `localProxyHost`, `localProxyPort`, `useGCloudCLI`, `ollamaHost`,
  `openaiCompatible`, `openaiBaseUrl`, plus any extra fields. Note that `apiKey` leaks to the proxy.
- The proxy reads only `messages`, `vertexProject`, `vertexLocation`, `model` and `webSearch`.
- `chat/route.ts:36-51` defines `resolveLocalProxyBaseUrl()`, which is **dead code**: nothing calls it. With an
  empty port it would have returned `https://${host}`.
- `chat/stream/route.ts` reaches provider 7 through `nonStreamingFallback`, which calls `handleChatRoute`
  internally.

### 6.2 Constants (verbatim)

```js
const DEFAULT_PORT = 5103;
const DEFAULT_HOST = "0.0.0.0";
const DEFAULT_MODEL = "gemini-2.5-flash";
const TOKEN_CACHE_MS = 50 * 60 * 1000;            // 3_000_000 ms
const VERTEX_REGIONS = ["us-central1", "us-west1", "us-east4", "asia-northeast1", "northamerica-northeast1"];
```

Magic numbers elsewhere in the file:
- gcloud child timeout `10000` ms;
- body limit `2_000_000` (characters of the accumulated decoded string);
- `maxOutputTokens: 2048`;
- token prefix `"ya29."`;
- retryable statuses: 429 and anything ≥ 500.

Module state: `let cachedToken = ""; let tokenExpiresAt = 0;`.

### 6.3 Startup
- `host = process.env.LOCAL_PROXY_HOST || "0.0.0.0"`. An empty string falls back to the default.
- `port = Number.parseInt(process.env.LOCAL_PROXY_PORT || "5103", 10)`. `parseInt` is lenient: `"5103abc"`
  gives 5103, and `"abc"` gives NaN, which crashes `listen`.
- `server.listen(port, host)`, then log exactly `[local-proxy] listening on http://${host}:${port}` to stdout.
- If the port is already in use, Node crashes with an unhandled EADDRINUSE error. V2 should print the error and
  exit with code 1.

### 6.4 Request dispatch (in order)
`url = new URL(req.url || "/", \`http://${host}:${port}\`)`. Only `url.pathname` is used; the query string is
ignored.

| # | Condition | Response |
|---|---|---|
| 1 | `method === "OPTIONS"` (any path) | `204`, headers `Access-Control-Allow-Origin: *`, `Access-Control-Allow-Methods: GET,POST,OPTIONS`, `Access-Control-Allow-Headers: Content-Type, Authorization`. No Content-Type, empty body. |
| 2 | `GET` and pathname `/health` | `200` `writeJson` `{"available":true,"mode":"gcloud-vertex-proxy","bindHost":<host>,"bindPort":<port number>,"project":<VERTEX_PROJECT or "">,"location":<VERTEX_LOCATION or "us-central1">}` |
| 3 | `POST` and pathname `/chat` | the chat flow (§6.6) |
| 4 | anything else | `404` `{"error":"Not found"}`. This includes `GET /chat`, `POST /health`, `/health/`, and `HEAD /health` (Node sends no body for HEAD). |

`writeJson(res, status, body)` sets `statusCode`, `Content-Type: application/json; charset=utf-8`, the three
CORS headers above, and the body `JSON.stringify(body)`. Node also adds `Content-Length`, `Date`,
`Connection: keep-alive` and `Keep-Alive: timeout=5`.

**Observed** (`127.0.0.1:58103`, no VERTEX env):
- `GET /health` gives
  `{"available":true,"mode":"gcloud-vertex-proxy","bindHost":"127.0.0.1","bindPort":58103,"project":"","location":"us-central1"}`
  (Content-Length 125).
- `GET /health?x=1` gives the same.
- `GET /health/`, `POST /health`, `GET /chat` and `HEAD /health` give 404 `{"error":"Not found"}`.
- `OPTIONS /anything` gives 204 with the CORS headers.

**axum note:** axum answers `HEAD` automatically for GET routes. To match V1 exactly (`HEAD /health` returns
404), route by method by hand, or accept the divergence; it is harmless.

### 6.5 Functions (algorithms)

`getTargetRegions(preferredLocation)` (`server.mjs:15-26`):
1. `targets = []`.
2. If `preferredLocation` is truthy, push it.
3. Push each `VERTEX_REGIONS` entry that is not already present, in order.
4. Example: `"asia-northeast1"` gives `[asia-northeast1, us-central1, us-west1, us-east4, northamerica-northeast1]`.
   `"global"` gives `[global, us-central1, us-west1, us-east4, asia-northeast1, northamerica-northeast1]`.

`buildVertexUrl(projectId, location, model, method)` (`:28-33`):
- `location === "global"` gives
  `https://aiplatform.googleapis.com/v1/projects/${projectId}/locations/global/publishers/google/models/${model}:${method}`.
- Otherwise:
  `https://${location}-aiplatform.googleapis.com/v1/projects/${projectId}/locations/${location}/publishers/google/models/${model}:${method}`.
- The proxy only uses `method = "generateContent"`, with no `?alt=sse`. The shared.ts variant adds `?alt=sse`
  for streaming, but that belongs to the chat spec.
- Nothing is URL-encoded. Interpolate verbatim.

`buildGeminiBody(messages, useGrounding = false)` (`:35-64`):
1. Start with `contents = []` and `systemInstruction = ""`.
2. For each `message` in order:
   - If `message?.role === "system"`, set `systemInstruction = typeof content === "string" ? content : ""`.
     This **overwrites**: the last system message wins, even with an empty string. Then skip the message.
   - Otherwise push `{role: message?.role === "assistant" ? "model" : "user", parts: [{text: typeof message?.content === "string" ? message.content : ""}]}`.
     Any other role (`"user"`, `undefined`, `"tool"`, …) and null elements become `"user"`. Consecutive turns
     with the same role are **not** merged.
3. `body = {contents, generationConfig: {maxOutputTokens: 2048}}`.
4. If `systemInstruction` is non-empty, set `body.system_instruction = {parts: [{text: systemInstruction}]}`.
   The key is **snake_case**.
5. If `useGrounding`, set `body.tools = [{googleSearch: {}}]`.
6. Key order: `contents`, `generationConfig`, `system_instruction`, `tools`.

`extractGeminiResponse(payload)` (`:66-69`): return `payload?.candidates?.[0]?.content?.parts?.[0]?.text` if
it is a string, else `"[Parse Error]"`. Only the **first part** is read; with grounding, extra parts are
dropped. This is a parity quirk.

`getGcloudCandidates()` (`:71-88`):
- Windows (`process.platform === "win32"`), in order:
  1. `"gcloud"`
  2. `%LOCALAPPDATA%\Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd` (if `LOCALAPPDATA` is set)
  3. `%ProgramFiles%\Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd` (if set)
  4. `%USERPROFILE%\AppData\Local\Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd` (if set)
- Everywhere else (Linux and macOS): `"gcloud"`, `"/usr/local/bin/gcloud"`, `"/usr/bin/gcloud"`.

`runGcloudAccessToken(gcloudPath)` resolves to a token string or `""` (`:90-133`):
1. `isWindowsCommand` is true on win32 when the lowercased path is `"gcloud"` or ends with `.cmd` or `.bat`.
2. Command line:
   - Windows command: `cmd.exe` with args `["/d", "/s", "/c", "\"<path>\" auth print-access-token"]`.
   - Otherwise: `<path>` with args `["auth", "print-access-token"]`.
   - `stdio: ["ignore", "pipe", "pipe"]`, `shell: false`. stderr is piped but **never read**: a chatty gcloud
     could block on a full pipe. In Rust, discard stderr (`Stdio::null()`) or drain it.
3. Collect stdout chunks as text.
4. Settle once:
   - The 10 000 ms timer runs `child.kill()` and resolves `""`.
   - A child `error` event (ENOENT and similar) clears the timer and resolves `""`.
   - The child `exit` event clears the timer. `token = stdout.trim()`. If `code === 0` and the token starts
     with `"ya29."`, resolve the token; otherwise resolve `""`.
5. V1 resolves on `exit`, not `close`, so trailing stdout could in theory be missed. Rust's
   `wait_with_output()` avoids that.
6. **Windows quoting caveat:** Node quotes the composite `/c` argument libuv-style (backslash-escaped quotes)
   without `windowsVerbatimArguments`, and `cmd /s` may then see `\"…\"`. The V1 Windows path is probably
   broken.
7. V2 on Windows:
   - For `.cmd`/`.bat` candidates, use `std::process::Command::new(path).args(["auth","print-access-token"])`.
     Rust ≥ 1.77 routes batch files through cmd.exe with safe escaping.
   - For the bare `gcloud`, use `Command::new("cmd").raw_arg("/d /s /c \"gcloud auth print-access-token\"")`
     (`CommandExt::raw_arg`). Rust's PATH search would not find `gcloud.cmd` on its own.
   - Add `CREATE_NO_WINDOW` (0x08000000) so no console flashes.

`getVertexAccessToken()` (`:135-155`):
1. If `cachedToken` is set and `Date.now() < tokenExpiresAt`, return it.
2. Otherwise, for each candidate in order: skip it when it is not `"gcloud"` and `!existsSync(candidate)`;
   otherwise `token = await runGcloudAccessToken(candidate)`. On the first non-empty token, set
   `cachedToken = token` and `tokenExpiresAt = Date.now() + 3_000_000`, then return it.
3. If no candidate succeeds, throw
   `"gcloud auth print-access-token に失敗しました。gcloud auth login を確認してください。"` (verbatim).
4. There is no single-flight lock, so concurrent requests may spawn several gcloud processes, and there is no
   invalidation on 401. V2 may add a `tokio::sync::Mutex` around the refresh; that doesn't change observable
   behaviour.

`isRetryableStatus(statusCode)`: `statusCode === 429 || statusCode >= 500`.

`callVertexWithFallback({projectId, preferredLocation, model, messages, webSearch})` (`:161-195`):
1. `token = await getVertexAccessToken()`. A thrown error propagates.
2. `targets = getTargetRegions(preferredLocation)`, `errors = []`, and
   `body = buildGeminiBody(messages, webSearch)`, built once.
3. For each region:
   - `POST buildVertexUrl(projectId, region, model, "generateContent")` with headers
     `Authorization: Bearer <token>` and `Content-Type: application/json`, and body `JSON.stringify(body)`.
     There is no timeout, only undici's 300 s defaults.
   - `payload = json().catch(null)`.
   - `ok`: return `extractGeminiResponse(payload)`.
   - Retryable status: `errors.push(\`${region}:${status}\`)` and continue.
   - Any other status: throw
     `` `Vertex Error (${region}) ${response.status} ${response.statusText}: ${JSON.stringify(payload)}` ``.
     A null payload prints `null`. Example:
     `Vertex Error (us-central1) 404 Not Found: {"error":{"code":404,...}}`.
   - A **network error** from fetch (for example DNS failure for a region host) is **not** caught here. It
     propagates as `"fetch failed"`, with no failover to the next region.
4. After the loop, throw `` `Vertex retryable failures: ${errors.join(", ")}` ``. Example:
   `Vertex retryable failures: us-central1:429, us-west1:503, us-east4:503, asia-northeast1:429, northamerica-northeast1:503`.

`readJsonBody(req)` (`:206-224`):
- Concatenate `chunk.toString()` for each chunk. Each chunk is decoded as UTF-8 on its own, so a multibyte
  character split across chunks is corrupted into U+FFFD. Rust should concatenate the bytes first.
- When the accumulated `data.length > 2_000_000` (UTF-16 units), reject with `Error("Payload too large")`. The
  stream keeps being read and the later resolve does nothing.
- On `end`: `data ? JSON.parse(data) : {}`. A parse failure rejects with `Error("Invalid JSON")`.
- A request `error` event rejects with that error.

### 6.6 `POST /chat` flow (`:253-288`)
Everything is inside `try`. The catch sends **500** `{"error": message || "Unknown proxy error"}`.

1. `payload = await readJsonBody(req)`. "Payload too large" and "Invalid JSON" both return **500**. That
   status is a quirk; V1 never returns 400 for these.
2. `messages = Array.isArray(payload?.messages) ? payload.messages : []`. If empty, return **400**
   `{"error":"messages is required"}`. Observed for: an empty body, `null`, `[]`, and `"hi"` (not an array).
3. `projectId = String(payload?.vertexProject || process.env.VERTEX_PROJECT || "").trim()`. If empty, return
   **400** `{"error":"vertexProject is required (or set VERTEX_PROJECT env)."}`.
   - Quirk: a whitespace-only `vertexProject` is truthy, so it does **not** fall back to the env and gives 400.
     Observed.
   - A non-string truthy value is stringified, for example `123` becomes `"123"`.
4. `location = String(payload?.vertexLocation || process.env.VERTEX_LOCATION || "us-central1").trim() || "us-central1"`.
5. `model = String(payload?.model || "").trim()`. If `!model || model.toLowerCase().startsWith("gpt")`, use
   `"gemini-2.5-flash"`.
6. `webSearch = Boolean(payload?.webSearch)`, using JS truthiness (`js_truthy`).
7. `responseText = await callVertexWithFallback(...)`. Return **200** `{"response": responseText}`.

V1 `String(x)` semantics apply here to arbitrary JSON values:
- strings stay as they are;
- numbers use JS formatting;
- booleans become `"true"`/`"false"`;
- objects become `"[object Object]"`;
- arrays are joined with `,`.

Only strings occur in practice.

### 6.7 What V2 must preserve versus what it may fix

| Preserve (contract) | May fix (document in CHANGELOG) |
|---|---|
| Port 5103, host 0.0.0.0, the env names, the log line | Windows `cmd.exe` quoting (§6.5) |
| Routes, CORS headers, the 204/404/400/500 shapes and strings | stderr pipe never drained |
| Region order and failover rule (429/≥500 only), message formats | UTF-8 chunk split corruption |
| Token cache of 50 min, gcloud candidate order, `ya29.` check | Optional: extra macOS/Linux candidates (`/opt/homebrew/bin/gcloud`, `/usr/local/google-cloud-sdk/bin/gcloud`, `$HOME/google-cloud-sdk/bin/gcloud`, `/snap/bin/gcloud`). GUI-launched apps on macOS have a minimal PATH. |
| `[Parse Error]` fallback, first-part-only extraction | Optional: single-flight token refresh |
| 2 MB limit giving 500 `Payload too large` | |

---

## 7. `next.config.ts` → amadeus-server HTTP policy

### 7.1 Source (verbatim)

```ts
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  generateBuildId: async () => `${Date.now()}`,
  async headers() {
    return [
      {
        source: '/amadeus/:path*',
        headers: [
          {
            key: 'Cache-Control',
            value: 'no-store, no-cache, must-revalidate, proxy-revalidate, max-age=0',
          },
          {
            key: 'Pragma',
            value: 'no-cache',
          },
          {
            key: 'Expires',
            value: '0',
          },
          {
            key: 'Surrogate-Control',
            value: 'no-store',
          },
          {
            key: 'CDN-Cache-Control',
            value: 'no-store',
          },
        ],
      },
      {
        source: '/(.*)',
        headers: [
          {
            key: 'Cache-Control',
            value: 'no-store, no-cache, must-revalidate, proxy-revalidate, max-age=0',
          },
          {
            key: 'Pragma',
            value: 'no-cache',
          },
          {
            key: 'Expires',
            value: '0',
          },
          {
            key: 'Surrogate-Control',
            value: 'no-store',
          },
          {
            key: 'CDN-Cache-Control',
            value: 'no-store',
          },
        ],
      },
      {
        source: '/resources/:path*',
        headers: [
          {
            key: 'Cache-Control',
            value: 'public, max-age=86400, stale-while-revalidate=604800',
          },
          {
            key: 'Pragma',
            value: 'public',
          },
          {
            key: 'Expires',
            value: '86400',
          },
          {
            key: 'Surrogate-Control',
            value: 'public, max-age=86400',
          },
          {
            key: 'CDN-Cache-Control',
            value: 'public, max-age=86400',
          },
        ],
      },
    ];
  },
};

export default nextConfig;
```

### 7.2 Rule semantics
`.next/routes-manifest.json` has `basePath: ""` (no Next basePath; the app sits at `/amadeus/app` because of
the directory layout), no i18n, and no rewrites. The compiled header regexes are:

| # | source | regex |
|---|---|---|
| 1 | `/amadeus/:path*` | `^/amadeus(?:/((?:[^/]+?)(?:/(?:[^/]+?))*))?(?:/)?$` |
| 2 | `/(.*)` | `^(?:/(.*))(?:/)?$` |
| 3 | `/resources/:path*` | `^/resources(?:/((?:[^/]+?)(?:/(?:[^/]+?))*))?(?:/)?$` |

Every matching rule applies in order, and **the later rule wins for the same key**. Rules 1 and 2 are
identical, so the effect is:
- **Default for every path:** `NO_STORE_SET`.
- **`/resources` and `/resources/**`:** `PUBLIC_SET` replaces all five keys. `/resourcesX` does **not** match.

```rust
pub const NO_STORE_SET: [(&str, &str); 5] = [
    ("Cache-Control", "no-store, no-cache, must-revalidate, proxy-revalidate, max-age=0"),
    ("Pragma", "no-cache"),
    ("Expires", "0"),
    ("Surrogate-Control", "no-store"),
    ("CDN-Cache-Control", "no-store"),
];
pub const PUBLIC_SET: [(&str, &str); 5] = [
    ("Cache-Control", "public, max-age=86400, stale-while-revalidate=604800"),
    ("Pragma", "public"),
    ("Expires", "86400"),
    ("Surrogate-Control", "public, max-age=86400"),
    ("CDN-Cache-Control", "public, max-age=86400"),
];
pub const NOT_FOUND_CACHE_CONTROL: &str = "private, no-cache, no-store, max-age=0, must-revalidate";
```

Provenance: rule 3 (`/resources/:path*`) is an **uncommitted working-tree change**. It is absent at HEAD
`7e45174`; the file was modified on 2026-11-09. It **is** live in the running production build (`.next` built
2026-11-12; the observed headers confirm it). This spec follows the working tree, which is what production runs.

`Expires: 0` and `Expires: 86400` are invalid HTTP-dates that caches treat as "already expired".
`Cache-Control` takes precedence anyway. `Pragma: public` is non-standard. Copy all of them verbatim.

### 7.3 Effective headers (Observed matrix)

| Request | Status | Cache-Control | Other relevant headers |
|---|---|---|---|
| `GET /amadeus/app` | 200 | no-store set | `Content-Type: text/html; charset=utf-8`; `Link: </resources/images/Amadeus_BG.png>; rel=preload; as="image", …` (8 images, §11.5); `X-Powered-By: Next.js`; `Vary: rsc, next-router-state-tree, next-router-prefetch, next-router-segment-prefetch, Accept-Encoding` |
| `GET /amadeus/app?foo=bar` | 200 | same | |
| `HEAD /amadeus/app` | 200 | same | |
| `GET /sw.js` | 200 | no-store set | `application/javascript; charset=UTF-8`, `ETag: W/"f3-19f5b33c060"`, `Last-Modified`, `Accept-Ranges: bytes`, `Content-Length: 243` |
| `GET /amadeus_logo.png` (public root file) | 200 | no-store set | `image/png` |
| `GET /resources/images/Amadeus_BG.png` | 200 | public set | `image/png`, ETag, Last-Modified, `Accept-Ranges: bytes` |
| `GET /resources/live2d/reama-cubism-runtime.js?v=18` | 200 | public set | `application/javascript; charset=UTF-8`. **The query string is ignored by the server**; it only busts browser and CDN caches. |
| `GET /_next/static/chunks/*.css` (build assets) | 200 | **no-store set** (V1 never caches its bundles) | `text/css; charset=UTF-8` |
| `GET /amadeus/app/icon.png`, `/amadeus/icon.png`, `/icon.png` | 200 | no-store set | `image/png`. These are Next metadata image routes; `app/icon.png` is actually a JPEG served as `image/png`. |
| `POST` API (any status) | * | no-store set | `content-type: application/json` |
| `OPTIONS` API | 204 | no-store set | `allow: OPTIONS, POST` |
| `GET` API | 405 | no-store set | no body |
| `GET /amadeus/app/manifest.webmanifest` | **404** | `private, no-cache, no-store, max-age=0, must-revalidate` | other 4 keys from the no-store set, HTML 404 page |
| `GET /favicon.ico`, `/amadeus/app/favicon.ico` | 404 | private… | |
| `GET /resources/nonexistent.png` | 404 | `private, no-cache, no-store, max-age=0, must-revalidate` | **Pragma: public, Expires: 86400, Surrogate-Control: public, max-age=86400, CDN-Cache-Control: public, max-age=86400** (mixed quirk) |
| `GET /amadeus/app/` (trailing slash) | 308 | (none) | `location: /amadeus/app`, `Refresh: 0;url=/amadeus/app`. No rule headers on redirects. |

V2 cache middleware (applied to every response after the handler):
1. 3xx: add nothing.
2. Otherwise choose the set by path: `/resources` or `/resources/…` gets `PUBLIC_SET`; everything else gets
   `NO_STORE_SET`. **Override** any handler-set values for those keys.
3. If status == 404, replace `Cache-Control` with `NOT_FOUND_CACHE_CONTROL`. This is optional parity; it
   reproduces V1, including the mixed /resources 404.
4. Hashed Trunk assets (`*-<hash>.js|.wasm|.css`) **may** get `public, max-age=31536000, immutable` as an
   improvement over V1's no-store. This is a decision point (see Open questions). `index.html`, the manifest,
   `sw.js` and the API must stay `NO_STORE_SET`.

### 7.4 Trailing-slash redirect (Next internal rule)
The rule is `{"source":"/:path+/","destination":"/:path+","statusCode":308}` with regex
`^(?:/((?:[^/]+?)(?:/(?:[^/]+?))*))/$`. If the path is not `/` and ends with `/`, respond **308**:
- `location` = the path with **exactly one** trailing slash removed, plus `?query` if there is one;
- `Refresh: 0;url=<same location>`;
- an empty body and no cache headers.

Observed:
- `/amadeus/app/?x=1&y=2` redirects to `/amadeus/app?x=1&y=2`;
- `/amadeus/app//` redirects to `/amadeus/app/` (one slash at a time);
- `/resources/images/` redirects to `/resources/images`;
- POST requests redirect too, and 308 keeps the method.

### 7.5 Static serving details
- Next serves `public/` at the root with weak ETags `W/"<size hex>-<mtime-ms hex>"`, `Last-Modified`,
  `Accept-Ranges: bytes`, conditional GET and Range support.
- tower-http `ServeDir` gives Last-Modified, Range and conditional GET, but no ETag. That is acceptable;
  optionally add an ETag layer.
- Percent-decoding is required. Paths contain spaces (`/resources/fonts/Eurostile Condensed Bold.otf`,
  `motions/mtn_01.motion3 1.json`) and UTF-8 (`Live2D紅莉栖forSDK5.0.*`, requested as `%E7%B4%85%E8%8E%89%E6%A0%96`).
  ServeDir handles both. Reject `..` traversal; ServeDir does.
- Compression: Next gzips compressible responses (`Vary: Accept-Encoding` on HTML, JS, CSS and JSON). V2: use
  tower-http `CompressionLayer` (gzip + br) with the default predicate. Its predicate skips images and
  `text/event-stream`, which matters for the chat SSE. Also precompress the WASM (`ServeDir::precompressed_br()`
  and `precompressed_gzip()` on the dist dir).

MIME types (Observed V1 values; V2 must at least get `.wasm` right):

| Ext | V1 Content-Type |
|---|---|
| `.js` | `application/javascript; charset=UTF-8` (`text/javascript` is equally fine) |
| `.css` | `text/css; charset=UTF-8` |
| `.json` | `application/json; charset=UTF-8` |
| `.png` / `.jpg` / `.ico` | `image/png` / `image/jpeg` / `image/x-icon` |
| `.otf` / `.ttf` | `font/otf` / `font/ttf` |
| `.moc3`, `.frag`, `.vert` | `application/octet-stream` |
| `.html` | `text/html; charset=utf-8` |
| `.wasm` (new) | **`application/wasm`**; required by `WebAssembly.instantiateStreaming` |
| `.webmanifest` (new, served) | `application/manifest+json` |

### 7.6 Do not replicate
- `X-Powered-By: Next.js`.
- `Vary: rsc, next-router-…` (send `Vary: Accept-Encoding` only when compressing).
- The `x-nextjs-cache` header.
- The RSC payload.
- `generateBuildId: async () => \`${Date.now()}\``: the build ID is the build-time epoch in milliseconds
  (current: `1794409851184`). Its only purpose is a fresh asset namespace per build. In V2, Trunk's
  content-hashed filenames do the same job. If a build ID is wanted (for example to append `?v=<id>` to
  `/resources` URLs or show it in the UI), generate `AMADEUS_BUILD_ID` = epoch ms in `build.rs` and expose it
  with `env!`.

---

## 8. amadeus-server: routing table, base path, packaging

### 8.1 V1 URL surface relevant to Amadeus
- **Pages:** `/amadeus`, `/amadeus/app`, `/amadeus/changelog`, `/amadeus/download`, `/amadeus/docs`,
  `/amadeus/docs/{api,api/claude,api/gemini,api/groq,api/openai,api/vertex,controls,download,login}`.
- **API:**
  - `/amadeus/app/api/chat`, `/chat/stream`, `/chat/quota`, `/chat/vertex-proxy` (chat spec);
  - `/amadeus/app/api/{tts/synthesize,tts/health,stt/transcribe,stt/health,rag/embed,rag/health}` (this spec).
- **Metadata routes:** `/amadeus/app/icon.png`, `/amadeus/icon.png`, `/icon.png` (portfolio JPEG). The
  `/amadeus/app/manifest.webmanifest` route does **not** exist (404) even though the HTML links to it.
- **Static (`public/`):**
  - `/resources/**` (96 MB: fonts about 75 MB including three NotoSerifCJK OTFs of about 24.5 MB each; models
    about 16 MB, made of the motions dir at 14 MB (Unity `.anim` files about 12 MB of that, **unused** by the
    web runtime), a 1.8 MB 8192 texture and a 280 KB `.moc3`; images 6.8 MB;
    `live2d/` with `live2dcubismcore.min.js`, `reama-cubism-runtime.js` and 13 shader files; and
    `locales/*.json`, a stale copy that is not used at runtime because locales are imported from
    `lib/amadeus/locales`);
  - root files `/amadeus_logo.png`, `/amadeus_login.png`, `/{Claude,Gemini,Groq,OpenAI,Vertex}_STEP*.png`
    (docs screenshots), and `/sw.js`.
  - Root portfolio files not needed by Amadeus: `dsuv_icon.png`, `*.svg`, `openhighway/`.
- **Non-Amadeus routes on the same Next app** (out of scope): `/`, `/openhighway`, `/steinsclock`,
  `/atchannel`, `/api/atchannel/chat`. None of them uses `/resources/` or `/sw.js`. Checked.

### 8.2 V2 route table (evaluate in this order)

| # | Method + path | Handler | Cache set |
|---|---|---|---|
| 1 | any `^/.+/$` | trailing-slash 308 (§7.4) | none |
| 2 | `POST {base}/api/tts/synthesize` | §3.1 | no-store |
| 3 | `POST {base}/api/tts/health` | §3.2 | no-store |
| 4 | `POST {base}/api/stt/transcribe` | §3.3 (body limit ≥ 32 MiB) | no-store |
| 5 | `POST {base}/api/stt/health` | §3.4 | no-store |
| 6 | `POST {base}/api/rag/embed` | §3.5 | no-store |
| 7 | `POST {base}/api/rag/health` | §3.6 | no-store |
| 8 | `{base}/api/chat…` | chat spec | no-store |
| 9 | `OPTIONS {base}/api/<known>` | 204 + `allow: OPTIONS, POST` | no-store |
| 10 | other methods on known API paths | 405, empty body | no-store |
| 11 | `{base}/api/**` (unknown) | 404 | no-store / 404 rule |
| 12 | `GET {base}/manifest.webmanifest` | §9 (new; fixes the V1 404) | no-store |
| 13 | `GET /sw.js` | §10, embedded bytes | no-store |
| 14 | `GET /resources/**` | ServeDir(`{public_dir}/resources`) | public |
| 15 | `GET /amadeus_logo.png`, `/amadeus_login.png`, `/*_STEP*.png`, `/favicon.ico` (new; see §9) | ServeDir(`{public_dir}`) with an allowlist or a plain fallthrough | no-store |
| 16 | `GET /amadeus/icon.png`, `{base}/icon.png` | serve `amadeus_logo.png` bytes (identical md5 `25379e9f…`) | no-store |
| 17 | `GET {base}/<file in dist>` (hashed js/wasm/css, snippets) | ServeDir(`{dist_dir}`), excluding `index.html` | no-store (V1) or immutable (decision) |
| 18 | `GET {base}`, and `GET /amadeus` plus `/amadeus/**` SPA routes that the UI crate implements | `{dist_dir}/index.html` | no-store |
| 19 | anything else | 404 | 404 rule |

SPA fallback policy: V1 returns 404 for `/amadeus/app/<unknown>`. Recommendation: serve `index.html` with
**200** only for the known SPA routes listed in §8.1, and with **404** status otherwise, so the Leptos router can
still render a NotFound view. HEAD must work on every GET route (axum does this automatically).

### 8.3 Base path and asset prefix
- `AMADEUS_BASE_PATH` (default `/amadeus/app`) is a **build-time** constant. It is passed to
  `trunk build --public-url /amadeus/app/` and to the UI as `option_env!("AMADEUS_BASE_PATH")`. The server
  reads the same value at runtime and **must** match the build; refuse to start if
  `{dist_dir}/index.html` doesn't reference `{base}/`.
- The UI's HTTP transport builds URLs as `{base}/api/...`. V1 hardcodes `/amadeus/app/api/...` in
  `tts-service.ts:143,178`, `stt-service.ts:218,235`, `vector-database.ts:221,279`, `ai-service.ts:67,95` and
  `RealAmadeusApp.tsx:334`.
- Static resources stay **origin-absolute**: `/resources/...`, `/amadeus_logo.png`, `/sw.js`. The Live2D
  runtime hardcodes `ResourcesPath = "/resources/models/AmadeusKurisu5.0/"` and
  `ShaderPath = "/resources/live2d/shaders/"`, and the CSS uses `url("/resources/fonts/…")`.
- Desktop (Tauri): a separate Trunk build with `--public-url /`, plus `copy-dir public/resources` and
  `copy-file` for the root PNGs into dist, so the same absolute paths resolve against Tauri's asset origin. The
  UI must treat `/` (desktop) and `{base}` (web) as the app route.

### 8.4 Co-hosting
The production origin also serves the DSUV portfolio (Next.js) through a Cloudflare Tunnel. To run V2 next to
it, the reverse proxy or tunnel ingress must send these to amadeus-server: `/amadeus`, `/amadeus/*`,
`/resources/*`, `/sw.js`, `/amadeus_logo.png`, `/amadeus_login.png` and `/*_STEP*.png`. Everything else stays on
Next. Keep `CDN-Cache-Control` so Cloudflare keeps caching only `/resources` (1 day).

### 8.5 Recommended server config struct

```rust
pub struct ServerConfig {
    pub host: IpAddr,            // AMADEUS_HOST, default 0.0.0.0
    pub port: u16,               // AMADEUS_PORT | PORT, default 9501 (dev default 9601)
    pub base_path: String,       // AMADEUS_BASE_PATH, default "/amadeus/app" (no trailing slash)
    pub public_dir: PathBuf,     // AMADEUS_PUBLIC_DIR, default "./public" (contains resources/, root PNGs)
    pub dist_dir: PathBuf,       // AMADEUS_DIST_DIR, default "./dist" (Trunk output)
    pub proxy_allowlist: Option<Vec<String>>, // AMADEUS_PROXY_ALLOWLIST (unset = allow all, V1 parity)
    pub immutable_hashed_assets: bool,        // AMADEUS_IMMUTABLE_ASSETS, default false (V1 parity)
}
```

---

## 9. Web app manifest (`app/amadeus/app/manifest.ts`)

### 9.1 Source (verbatim)

```ts
import type { MetadataRoute } from "next";

export default function manifest(): MetadataRoute.Manifest {
  return {
    name: "Amadeus",
    short_name: "Amadeus",
    description: "Real Amadeus Web",
    start_url: "/amadeus/app",
    display: "standalone",
    orientation: "landscape",
    background_color: "#000000",
    theme_color: "#000000",
    lang: "ja",
    icons: [
      {
        src: "/favicon.ico",
        sizes: "any",
        type: "image/x-icon",
      },
    ],
  };
}
```

### 9.2 V1 reality (Observed)
Next only recognises `manifest.ts` at the **root** of `app/`. The nested file is ignored: no route is
generated (`app-paths-manifest.json` has none), and `GET /amadeus/app/manifest.webmanifest` returns **404**,
even though every `/amadeus/app` page emits `<link rel="manifest" href="/amadeus/app/manifest.webmanifest"/>`.

The icon it lists (`/favicon.ico`) is also 404: there is no `public/favicon.ico` and no `app/favicon.ico`.
`app/amadeus/app/favicon.ico` (a 16x16 + 32x32 ICO with 4 images) is likewise ignored, because nested
`favicon.ico` isn't supported. So in V1 the PWA install metadata is effectively broken.

### 9.3 V2 behaviour (recommended: serve it)
Serve `GET {base}/manifest.webmanifest` with `Content-Type: application/manifest+json` and the no-store set.
The body is compact JSON with keys in this exact order (what Next would have produced):

```json
{"name":"Amadeus","short_name":"Amadeus","description":"Real Amadeus Web","start_url":"/amadeus/app","display":"standalone","orientation":"landscape","background_color":"#000000","theme_color":"#000000","lang":"ja","icons":[{"src":"/favicon.ico","sizes":"any","type":"image/x-icon"}]}
```

`start_url` must follow `AMADEUS_BASE_PATH`. Two recommended fixes, flagged in Open questions:
- serve `app/amadeus/app/favicon.ico` at `/favicon.ico`;
- append `{"src":"/amadeus_logo.png","sizes":"1024x1024","type":"image/png"}` to `icons`. Chrome needs a PNG of
  at least 192 px to be installable.

---

## 10. `public/sw.js`: self-unregistering ("kill-switch") service worker

Source (verbatim, 243 bytes, served as `application/javascript; charset=UTF-8` with no-store headers):

```js
self.addEventListener('install', function(e) {
  self.skipWaiting();
});

self.addEventListener('activate', function(e) {
  self.registration.unregister();
});

self.addEventListener('fetch', function(event) {
  // Pass through everything
});
```

Semantics:
- **Nothing in the current code registers it.** There are no `serviceWorker` or `sw.js` references anywhere
  in the repo.
- The file exists so that browsers still holding an **old** registration for `/sw.js` (scope `/`) fetch this
  version during their periodic update check. It installs (`skipWaiting`), activates, and unregisters itself.
  Pages it already controls stay controlled until they reload.
- The empty `fetch` listener passes everything through to the network.

V2 requirements:
- Keep serving these exact bytes at the **origin root** `/sw.js`, not under the base path, because the scope
  comes from the script's location.
- Use a JavaScript MIME type and the no-store set.
- Embed it in the server binary with `include_str!` so it is always present.
- Do **not** register any service worker in V2.
- The desktop (Tauri) build doesn't need it; it is harmless there.

---

## 11. Layouts, page, and the web `index.html`

### 11.1 `app/layout.tsx` (root, applies to every page including `/amadeus/app`)

```tsx
import type { Metadata } from "next";
import "./globals.css";

export const dynamic = 'force-dynamic';
export const revalidate = 0;

export const metadata: Metadata = {
  title: "DSUV",
  description: "Personal Portfolio of DSUV - Lead Architect @ ELVELT",
};

import { LanguageProvider } from "@/hooks/useTranslation";

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className="dark">
      <head>
        <meta httpEquiv="Cache-Control" content="no-cache, no-store, must-revalidate" />
        <meta httpEquiv="Pragma" content="no-cache" />
        <meta httpEquiv="Expires" content="0" />
      </head>
      <body className="bg-black text-white min-h-screen selection:bg-purple-500/30 selection:text-white">
        <LanguageProvider>
          {children}
        </LanguageProvider>
      </body>
    </html>
  );
}
```

- `dynamic='force-dynamic'` and `revalidate=0` mean every request is rendered fresh with no caching. In V2,
  `index.html` is static but served with the no-store set, which is equivalent.
- `./globals.css` is `app/globals.css` (69 lines). It contains `@import "tailwindcss";`, an `@theme` block
  (`--color-amadeus: #d49e38`, `--font-amadeus`, the glitch and scanline keyframes), `:root`
  (`--background:#000000; --foreground:#ededed`), `body { background: var(--background); color: var(--foreground); font-family: Arial, Helvetica, sans-serif; overflow-x: hidden; }`,
  and the `.glitch-text` rules. It **must be compiled by Tailwind v4**, and it is the **first** stylesheet.
- `LanguageProvider` (`hooks/useTranslation.tsx:656-668`) holds in-memory React state only: `locale` defaults to
  `'ja'` and `toggleLanguage` flips ja/en. It has **no side effects** and no storage, and `RealAmadeusApp` does
  not use it (it has its own localization). The `/amadeus` landing and docs pages do use it
  (`useLanguage()`). Port it in the UI crate as a context only where those pages need it.

### 11.2 `app/amadeus/layout.tsx`

```tsx

import type { Metadata } from 'next';

export const metadata: Metadata = {
    title: "Amadeus",
};

export default function AmadeusLayout({
    children,
}: {
    children: React.ReactNode;
}) {
    return children;
}
```

It adds no DOM. Its only effect is `<title>Amadeus</title>` for every `/amadeus/**` page. Those pages keep the
root description `Personal Portfolio of DSUV - Lead Architect @ ELVELT` (Observed, §11.6).

### 11.3 `app/amadeus/app/layout.tsx`

```tsx
import type { Metadata, Viewport } from "next";
import "./globals.css";

export const dynamic = 'force-dynamic';
export const revalidate = 0;

export const metadata: Metadata = {
  title: "Amadeus",
  description: "Real Amadeus Web",
  applicationName: "Amadeus",
  manifest: "/amadeus/app/manifest.webmanifest",
  appleWebApp: {
    capable: true,
    statusBarStyle: "black-translucent",
    title: "Amadeus",
  },
  icons: {
    icon: "/amadeus_logo.png",
    apple: "/amadeus_logo.png",
    shortcut: "/amadeus_logo.png",
  },
};

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  viewportFit: "cover",
  themeColor: "#000000",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <>
      <meta httpEquiv="Cache-Control" content="no-cache, no-store, must-revalidate" />
      <meta httpEquiv="Pragma" content="no-cache" />
      <meta httpEquiv="Expires" content="0" />
      <div className="amadeus-root h-screen w-screen overflow-hidden">{children}</div>
    </>
  );
}
```

- `./globals.css` is `app/amadeus/app/globals.css` (2966 lines, owned by the UI/CSS spec). It is the
  **second** stylesheet. It starts with
  `@import url('https://fonts.googleapis.com/css2?family=Noto+Serif+JP:wght@300;400;700&display=swap');`
  (an external network dependency), then `@font-face` rules pointing at `/resources/fonts/...`, a universal
  reset (`* {box-sizing:border-box; margin:0; padding:0; user-select:none}`), and `.amadeus-root`
  (`width:100%; height:100%; min-height:100dvh; overflow:hidden; font-family:"Noto Serif JP","MS Mincho",serif; font-weight:300; background:#000; color:#fff`).
- React 19 hoists the three `<meta http-equiv>` into `<head>`, so they appear **twice** (once from the root
  layout and once from here).
- The file-based `icon.png` in this segment is **not** emitted, because the config `icons` win (Observed).

### 11.4 `app/amadeus/app/page.tsx`

```tsx
"use client";

import { RealAmadeusApp } from "@/components/amadeus/RealAmadeusApp";

export default function Home() {
  return <RealAmadeusApp />;
}
```

This is the Leptos root view mount point (`RealAmadeusApp` is owned by the UI specs).

### 11.5 Rendered V1 `<head>` for `/amadeus/app` (Observed, in order, Next internals removed)

```html
<!DOCTYPE html><html lang="en" class="dark"><head>
<meta charSet="utf-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover"/>
<link rel="preload" as="image" href="/resources/images/ConfigSelected.png"/>
<link rel="preload" as="image" href="/resources/images/Status.png"/>
<link rel="preload" as="image" href="/resources/images/StatusSelected.png"/>
<link rel="preload" as="image" href="/resources/images/ChangeLog.png"/>
<link rel="preload" as="image" href="/resources/images/ChangeLogSelected.png"/>
<link rel="preload" as="image" href="/resources/images/Help.png"/>
<link rel="preload" as="image" href="/resources/images/HelpSelected.png"/>
<link rel="preload" as="image" href="/resources/images/DownAllow.png"/>
<link rel="stylesheet" href="/_next/static/chunks/301a5e2f106dc67e.css" data-precedence="next"/>   <!-- app/globals.css (Tailwind) -->
<link rel="stylesheet" href="/_next/static/chunks/1b90ae59afeb13d0.css" data-precedence="next"/>   <!-- app/amadeus/app/globals.css -->
<!-- …Next/Turbopack <script async> tags… -->
<meta http-equiv="Cache-Control" content="no-cache, no-store, must-revalidate"/>
<meta http-equiv="Pragma" content="no-cache"/>
<meta http-equiv="Expires" content="0"/>
<meta http-equiv="Cache-Control" content="no-cache, no-store, must-revalidate"/>
<meta http-equiv="Pragma" content="no-cache"/>
<meta http-equiv="Expires" content="0"/>
<meta name="theme-color" content="#000000"/>
<title>Amadeus</title>
<meta name="description" content="Real Amadeus Web"/>
<meta name="application-name" content="Amadeus"/>
<link rel="manifest" href="/amadeus/app/manifest.webmanifest"/>
<meta name="mobile-web-app-capable" content="yes"/>
<meta name="apple-mobile-web-app-title" content="Amadeus"/>
<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent"/>
<link rel="shortcut icon" href="/amadeus_logo.png"/>
<link rel="icon" href="/amadeus_logo.png"/>
<link rel="apple-touch-icon" href="/amadeus_logo.png"/>
</head>
```

The other 8 preloads arrive as an HTTP `Link` header, in this order: `Amadeus_BG.png`,
`RealAmadeus_Menu_BG_v3.jpg`, `amadeus_logo_v3.png`, `amadeus_login_button_v2.png`, `Menu.png`,
`Backlog.png`, `BacklogSelected.png`, `Config.png`, all under `/resources/images/`. These are React 19 SSR
image preloads for the login and menu screens. Notably `amadeus_login_button_v2.png` is 3.4 MB and
`amadeus_logo_v3.png` is 2.2 MB.

Notes:
- Next 16 emits `mobile-web-app-capable`, **not** `apple-mobile-web-app-capable`, for `appleWebApp.capable`.
- `lang="en"` on `<html>` even though the manifest says `lang: "ja"`. Keep both.
- The body starts with `<div hidden=""><!--$--><!--/$--></div>` (Next metadata outlet), then
  `<div class="amadeus-root h-screen w-screen overflow-hidden">`, then the RealAmadeusApp tree
  (`<div class="real-amadeus-root"><div class="window-frame windowed" style="width:1920px;height:1080px">…`).
  The hidden div is irrelevant; no CSS targets it.

### 11.6 Rendered head for the other `/amadeus/**` pages (Observed on `/amadeus`, `/amadeus/docs`, `/amadeus/download`, `/amadeus/changelog`, `/amadeus/docs/api/vertex`)

```html
<meta charSet="utf-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1"/>
<meta http-equiv="Cache-Control" content="no-cache, no-store, must-revalidate"/>
<meta http-equiv="Pragma" content="no-cache"/>
<meta http-equiv="Expires" content="0"/>
<title>Amadeus</title>
<meta name="description" content="Personal Portfolio of DSUV - Lead Architect @ ELVELT"/>
<link rel="icon" href="/amadeus/icon.png?icon.56174f8b.png" sizes="1024x1024" type="image/png"/>
```

These pages have no theme-color, no `viewport-fit=cover`, no manifest and no apple tags. They load only the root
CSS (`301a5e2f…css`), not the amadeus app CSS. If the V2 SPA serves every `/amadeus/**` route from one
`index.html`, use `leptos_meta` to switch the description, icon and viewport per route, or accept the
`/amadeus/app` head everywhere (see Open questions). `/amadeus/icon.png` is byte-identical to
`/amadeus_logo.png`.

### 11.7 Recommended V2 `index.html` (Trunk, web build)

```html
<!DOCTYPE html>
<html lang="en" class="dark">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
    <!-- Same 16 image preloads as V1 (8 from the Link header + 8 head tags) -->
    <link rel="preload" as="image" href="/resources/images/Amadeus_BG.png" />
    <link rel="preload" as="image" href="/resources/images/RealAmadeus_Menu_BG_v3.jpg" />
    <link rel="preload" as="image" href="/resources/images/amadeus_logo_v3.png" />
    <link rel="preload" as="image" href="/resources/images/amadeus_login_button_v2.png" />
    <link rel="preload" as="image" href="/resources/images/Menu.png" />
    <link rel="preload" as="image" href="/resources/images/Backlog.png" />
    <link rel="preload" as="image" href="/resources/images/BacklogSelected.png" />
    <link rel="preload" as="image" href="/resources/images/Config.png" />
    <link rel="preload" as="image" href="/resources/images/ConfigSelected.png" />
    <link rel="preload" as="image" href="/resources/images/Status.png" />
    <link rel="preload" as="image" href="/resources/images/StatusSelected.png" />
    <link rel="preload" as="image" href="/resources/images/ChangeLog.png" />
    <link rel="preload" as="image" href="/resources/images/ChangeLogSelected.png" />
    <link rel="preload" as="image" href="/resources/images/Help.png" />
    <link rel="preload" as="image" href="/resources/images/HelpSelected.png" />
    <link rel="preload" as="image" href="/resources/images/DownAllow.png" />
    <!-- 1st stylesheet: app/globals.css compiled with Tailwind v4 (must @source the Rust UI sources + this file) -->
    <link data-trunk rel="tailwind-css" href="crates/amadeus-ui/style/root.css" />
    <!-- 2nd stylesheet: app/amadeus/app/globals.css copied verbatim (keeps the Google Fonts @import first) -->
    <link data-trunk rel="css" href="crates/amadeus-ui/style/amadeus.css" />
    <meta http-equiv="Cache-Control" content="no-cache, no-store, must-revalidate" />
    <meta http-equiv="Pragma" content="no-cache" />
    <meta http-equiv="Expires" content="0" />
    <meta name="theme-color" content="#000000" />
    <title>Amadeus</title>
    <meta name="description" content="Real Amadeus Web" />
    <meta name="application-name" content="Amadeus" />
    <link rel="manifest" href="/amadeus/app/manifest.webmanifest" />
    <meta name="mobile-web-app-capable" content="yes" />
    <meta name="apple-mobile-web-app-title" content="Amadeus" />
    <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent" />
    <link rel="shortcut icon" href="/amadeus_logo.png" />
    <link rel="icon" href="/amadeus_logo.png" />
    <link rel="apple-touch-icon" href="/amadeus_logo.png" />
    <link data-trunk rel="rust" href="crates/amadeus-ui/Cargo.toml" data-bin="amadeus-ui" data-wasm-opt="z" />
    <!-- desktop build only (or always; harmless for web): -->
    <link data-trunk rel="copy-dir" href="public/resources" />
    <link data-trunk rel="copy-file" href="public/amadeus_logo.png" />
    <link data-trunk rel="copy-file" href="public/amadeus_login.png" />
  </head>
  <body class="bg-black text-white min-h-screen selection:bg-purple-500/30 selection:text-white">
    <div id="amadeus-root" class="amadeus-root h-screen w-screen overflow-hidden"></div>
  </body>
</html>
```

Notes:
- **Tailwind classes in this file** must exist in the compiled root CSS: `bg-black`, `text-white`,
  `min-h-screen`, `selection:bg-purple-500/30`, `selection:text-white`, `h-screen`, `w-screen` and
  `overflow-hidden`. Tailwind v4 needs `@source "../../../index.html"; @source "../src/**/*.rs";`, or the
  equivalent, in `root.css`.
- Trunk's `rel="tailwind-css"` needs `[tools] tailwindcss = "4.x"`. If that Trunk version can't run v4, use a
  `[[hooks]] stage="pre_build"` that runs the Tailwind v4 standalone CLI and link the output with `rel="css"`.
  Keep the **two** stylesheets **separate and in this order**: merging them would bury the `@import`, which
  must come first in a stylesheet, and would change the cascade.
- Mount Leptos into `#amadeus-root` with `leptos::mount::mount_to(element, App)`, so the DOM is
  `body > div.amadeus-root > div.real-amadeus-root …` as in V1. `mount_to` appends, so keep the div empty.
- **Do not** add `live2dcubismcore.min.js` here unless the live2d spec wants a static tag. V1 injects it
  dynamically (`Live2DCanvas.tsx:31-46`, `CORE_SCRIPT_SRC = "/resources/live2d/live2dcubismcore.min.js"`).
  The V1 Live2D runtime JS is loaded with a cache-buster: `?v=19` in the working tree, `?v=18` at HEAD.
- V1 SSR painted the login screen before JS ran. V2 CSR shows a black page until the WASM loads. `body` is
  already black, so there is no white flash. A loader, if wanted, must sit outside `#amadeus-root` or be
  removed before mounting.
- The CSS order also matters for the font loading: `amadeus.css` references the fonts as
  `/resources/fonts/...`, absolute, so it works for both web and desktop.

---

## 12. `app/amadeus/app/page.module.css`: dead code

This is the create-next-app template CSS module (classes `.page`, `.main`, `.intro`, `.ctas`, `a.primary`,
`a.secondary`, `.logo`, a `(hover: hover) and (pointer: fine)` block, a `(max-width: 600px)` block and a
`prefers-color-scheme: dark` block; it references the undefined `--font-geist-sans`).

`page.tsx` doesn't import it, and nothing else in the repo references it (grep for `page.module.css` and
`module.css` returns nothing). **Do not port it.** If someone insists, it is `app/amadeus/app/page.module.css`
lines 1–142.

---

## 13. `package.json` scripts → V2 tasks and release

V1 scripts (verbatim):

```json
"scripts": {
  "dev": "next dev -p 9601",
  "build": "next build",
  "start": "next start -p 9501",
  "start:amadeus-proxy": "node scripts/amadeus-local-proxy/server.mjs",
  "lint": "eslint"
}
```

The rest of `package.json` (context only): `next 16.1.6`, `react`/`react-dom` `19.2.3`, `zustand`, `three`
and the `@react-three/*` packages (those belong to `/openhighway`, not Amadeus), `lucide-react` (icons in the
Amadeus landing and docs pages), and devDependencies `tailwindcss ^4`, `@tailwindcss/postcss ^4`, `esbuild`,
`ws`, `typescript ^5`.

Proposed V2 equivalents (a `justfile` or `cargo xtask`, in RealAmadeusRust/):

| V1 | V2 |
|---|---|
| `npm run dev` (:9601) | `just dev-web`: `trunk watch --public-url /amadeus/app/` (into `dist/`) and `cargo run -p amadeus-server -- --port 9601 --dist dist --public public`. Alternative: `trunk serve --port 9601` with `[[proxy]]` entries for `/amadeus/app/api/` and `/resources/`. |
| `npm run build` | `just build-web`: `trunk build --release --public-url /amadeus/app/` and `cargo build -p amadeus-server --release` |
| `npm run start` (:9501) | `amadeus-server --port 9501` (env `AMADEUS_PORT=9501`). Run it under systemd, replacing `npm run start`. |
| `npm run start:amadeus-proxy` | `cargo run -p amadeus-backend --bin amadeus-local-proxy --release`. Ship the prebuilt `amadeus-local-proxy(.exe)` in releases. |
| `npm run lint` | `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings`, plus `cargo clippy --target wasm32-unknown-unknown -p amadeus-ui -p amadeus-live2d` |
| (new) desktop | `cargo tauri dev` / `cargo tauri build`. The Trunk desktop build uses `--public-url /`. |

Release suggestion, following the current GitHub naming
`RealAmadeus_V1.4U_Win64.zip`, `…_MacUniversal.zip`, `…_Linux.zip` under tag `RealAmadeusV1.4`:
- Use tag `RealAmadeusV2.0` with assets `RealAmadeus_V2.0R_Win64.msi`/`.zip`, `RealAmadeus_V2.0R_MacUniversal.dmg`,
  `RealAmadeus_V2.0R_Linux.AppImage`/`.deb`, `RealAmadeus_V2.0R_WebServer_<target>.tar.gz` (server binary plus
  `dist/` plus `public/`), and `amadeus-local-proxy_<target>`.
- A GitHub Actions matrix (windows-latest, macos-latest with universal-apple-darwin, ubuntu-22.04) can use
  `tauri-apps/tauri-action`. The web job builds Trunk plus the server and can also produce a Docker image.
- Set the workspace `version = "2.0.0"`.

---

## 14. Regression test matrix (derived from the V1 probes; implement as Rust integration tests with an axum mock upstream)

Mock upstream behaviour:
- **"good"**:
  - `GET /health` returns 200 `{"status":"ok","available":"yes-from-upstream","runtime":{"using_gpu":true,"device":"cuda","gpu_name":"MockGPU","attn_implementation":"sdpa"},"torch":{"cuda_available":true,"detected_gpu_name":"MockGPU"}}`;
  - `POST /synthesize` returns 200 `audio/x-wav` bytes `RIFF$\0\0\0WAVEfmt `;
  - `POST /transcribe` returns 200 `{"text":"こんにちは","provider_used":"mock"}`;
  - `POST /embed` returns 200 `{"embedding":[0.25,-0.5,1],"dim":3}`.
- **"bad"**:
  - `GET` returns `503 Service Unavailable` `{"error":"down"}`;
  - `POST /transcribe` returns 422 `{"error":"bad audio"}`;
  - `POST /embed` returns 500 `text/plain` `embed exploded`;
  - `POST /synthesize` returns 500 `text/plain; charset=utf-8` `合成エラー detail`.
- **"slow"**: `GET /health` sleeps 7 s.
- **refused**: `http://127.0.0.1:1`.

| # | Route | Request | Expected status | Expected body |
|---|---|---|---|---|
| 1 | tts/health | no body | 400 | `{"available":false,"error":"serverUrl is required"}` |
| 2 | tts/health | `not json` | 400 | same as 1 |
| 3 | tts/health | `{"serverUrl":"   "}` | 400 | same as 1 |
| 4 | tts/health | good | 200 | `{"available":"yes-from-upstream","status":"ok","runtime":{"using_gpu":true,"device":"cuda","gpu_name":"MockGPU","attn_implementation":"sdpa"},"torch":{"cuda_available":true,"detected_gpu_name":"MockGPU"}}` |
| 5 | tts/health | bad | 200 | `{"available":false,"error":"503 Service Unavailable"}` |
| 6 | tts/health | refused | 200 | `{"available":false,"error":"fetch failed"}` |
| 7 | tts/health | `{"serverUrl":"abc"}` | 200 | `{"available":false,"error":"Failed to parse URL from abc/health"}` |
| 8 | tts/health | `{"serverUrl":5}` | 500 | empty body, no content-type |
| 9 | tts/health | slow | 200 (~5 s) | `{"available":false,"error":"This operation was aborted"}` |
| 10 | tts/health | GET | 405 | empty |
| 11 | tts/health | OPTIONS | 204 | empty, `allow: OPTIONS, POST` |
| 12 | stt/health | good / bad / none / slow | 200 / 200 / 400 / 200 | same shapes as 4 / 5 / 1 / 9 |
| 13 | rag/health | good | 200 | `{"available":true}` |
| 14 | rag/health | bad / refused / slow | 200 | `503 Service Unavailable` / `fetch failed` / `This operation was aborted` |
| 15 | rag/health | `Content-Type: text/plain`, body `{"serverUrl":"http://127.0.0.1:1"}` | 200 | `{"available":false,"error":"fetch failed"}` |
| 16 | rag/embed | good, text `"  hello 世界  "` | 200 | `{"embedding":[0.25,-0.5,1],"dim":3}`; upstream got `{"text":"hello 世界"}` |
| 17 | rag/embed | `serverUrl:"http://127.0.0.1:58100x"` | 500 | `{"error":"Failed to parse URL from http://127.0.0.1:58100x/embed"}` |
| 18 | rag/embed | bad | 500 | `{"error":"Embedding failed (500)"}` |
| 19 | rag/embed | text `"   "` | 400 | `{"error":"text is required"}` |
| 20 | rag/embed | no serverUrl | 400 | `{"error":"serverUrl is required"}` |
| 21 | rag/embed | refused | 500 | `{"error":"fetch failed"}` |
| 22 | tts/synthesize | good, text `"  こんにちは  "` | 200 | `content-type: audio/x-wav`, bytes equal; upstream got `{"text":"こんにちは","speaker":"christina-jp","use_cache":true}` |
| 23 | tts/synthesize | `speaker:"  christina ", useCache:false` | 200 | upstream got `{"text":"x","speaker":"christina","use_cache":false}` |
| 24 | tts/synthesize | `speaker:"   ", useCache:null` | 200 | upstream got `…"speaker":"christina-jp","use_cache":true}` |
| 25 | tts/synthesize | `useCache:"nope"` | 200 | upstream got `"use_cache":"nope"` |
| 26 | tts/synthesize | bad | 500 | `{"error":"Synthesis failed (500): 合成エラー detail"}` |
| 27 | tts/synthesize | refused | 500 | `{"error":"fetch failed"}` |
| 28 | tts/synthesize | no text | 400 | `{"error":"text is required"}` |
| 29 | tts/synthesize | `{}` | 400 | `{"error":"serverUrl is required"}` |
| 30 | stt/transcribe | good, audio `audio/webm;codecs=opus`, provider `openai`, language `en` | 200 | `{"text":"こんにちは","provider_used":"mock"}`; upstream parts `audio` (filename `recording.webm`, type `audio/webm;codecs=opus`), `provider=openai`, `language=en` |
| 31 | stt/transcribe | good, audio only + serverUrl | 200 | upstream got `provider=groq`, `language=ja` |
| 32 | stt/transcribe | good, `provider=` and `language=` empty | 200 | upstream got empty strings |
| 33 | stt/transcribe | bad | 422 | `{"error":"bad audio"}` |
| 34 | stt/transcribe | no serverUrl | 400 | `{"error":"serverUrl is required"}` |
| 35 | stt/transcribe | no audio, or `audio=hello` as a text field | 400 | `{"error":"audio file is required"}` |
| 36 | stt/transcribe | JSON body | 400 | `{"error":"Invalid form data"}` |
| 37 | stt/transcribe | urlencoded `serverUrl=http://127.0.0.1:1` | 400 | `{"error":"audio file is required"}` |
| 38 | stt/transcribe | urlencoded `a=b` | 400 | `{"error":"serverUrl is required"}` |
| 39 | stt/transcribe | `multipart/form-data; boundary=xyz` + garbage | 400 | `{"error":"Invalid form data"}` |
| 40 | stt/transcribe | refused | 500 | `{"error":"fetch failed"}` |
| 41 | server | `GET /amadeus/app/?x=1&y=2` | 308 | `location: /amadeus/app?x=1&y=2`, `Refresh: 0;url=/amadeus/app?x=1&y=2` |
| 42 | server | `GET /resources/images/Amadeus_BG.png` | 200 | public set headers |
| 43 | server | `GET /sw.js` | 200 | exact 243 bytes, JS MIME, no-store set |
| 44 | server | `GET /amadeus/app` | 200 | index.html, no-store set |
| 45 | local-proxy | the cases in §6.4 and §6.6 | | as listed |

---

## Rust mapping

### `amadeus-core` (wasm32 + native, no I/O)
- `amadeus_core::js`:
  - `pub fn js_trim(s: &str) -> &str` (§2.4)
  - `pub fn js_truthy(v: &serde_json::Value) -> bool` (§5.3)
  - `pub fn js_string(v: &Value) -> String` (`String(x)` semantics for §6.6)
- `amadeus_core::endpoints` (constants):
  - `DEFAULT_TTS_SERVER_URL = "http://localhost:5100"`, `DEFAULT_STT_SERVER_URL = "http://localhost:5101"`,
    `DEFAULT_RAG_SERVER_URL = "http://localhost:5102"`
  - `DEFAULT_TTS_SPEAKER = "christina-jp"`, `TTS_SPEAKER_NAMES = ["christina-jp", "christina"]`
  - `DEFAULT_STT_PROVIDER = "groq"`, `STT_PROVIDER_NAMES = ["groq", "openai", "local"]`,
    `DEFAULT_STT_LANGUAGE = "ja"`
  - `HEALTH_TIMEOUT_MS: u64 = 5000`, `STT_UPLOAD_FILENAME = "recording.webm"`
  - `LOCAL_PROXY_DEFAULT_PORT: u16 = 5103`, `LOCAL_PROXY_DEFAULT_BIND_HOST = "0.0.0.0"`,
    `LOCAL_PROXY_DEFAULT_CLIENT_HOST = "127.0.0.1"`
- `amadeus_core::protocol::paths`:
  - `DEFAULT_BASE_PATH = "/amadeus/app"`
  - `API_TTS_SYNTHESIZE = "/api/tts/synthesize"`, `API_TTS_HEALTH = "/api/tts/health"`,
    `API_STT_TRANSCRIBE = "/api/stt/transcribe"`, `API_STT_HEALTH = "/api/stt/health"`,
    `API_RAG_EMBED = "/api/rag/embed"`, `API_RAG_HEALTH = "/api/rag/health"` (all relative to the base)
  - `MANIFEST_PATH = "/manifest.webmanifest"`
- `amadeus_core::protocol::aux` (client→server DTOs; serde `rename_all = "camelCase"`):
  - `TtsSynthesizeBody { server_url: String, speaker: String, text: String, use_cache: bool }`. Field order
    matches the V1 client: `serverUrl`, `speaker`, `text`, `useCache`.
  - `ServerUrlBody { server_url: String }`
  - `RagEmbedBody { server_url: String, text: String }`
  - `SttTranscribeFields { provider: String, language: String, server_url: String }` (the audio travels
    separately)
  - Server→client replies stay `serde_json::Value`, because pass-through and spread semantics require it.
- `amadeus_core::protocol::reply::ApiReply`, the transport-neutral result returned by every backend function
  and used by the server, desktop and UI:

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ApiReply {
    Json   { status: u16, body: serde_json::Value },
    Binary { status: u16, content_type: String, #[serde(with = "serde_bytes_b64")] data: Vec<u8> },
    Empty  { status: u16 },   // the JS-TypeError 500 and 204/405 cases
}
```

- `amadeus_core::web`:
  - `WEB_MANIFEST` builder: `pub fn web_manifest_json(base_path: &str) -> String` (§9.3, key order preserved)
  - `pub const SERVICE_WORKER_JS: &str` (§10 verbatim)
  - head constants: `APP_TITLE = "Amadeus"`, `APP_DESCRIPTION = "Real Amadeus Web"`,
    `THEME_COLOR = "#000000"`, `PORTFOLIO_DESCRIPTION = "Personal Portfolio of DSUV - Lead Architect @ ELVELT"`

### `amadeus-backend` (native; tokio + reqwest)
- `amadeus_backend::http::shared_client() -> reqwest::Client`:
  - `.no_proxy()`
  - features `gzip`, `deflate`, `brotli`, `json`, `multipart`, `stream`
  - `.redirect(Policy::limited(20))`, `.read_timeout(300 s)`
  - user agent `RealAmadeus/<ver>`
- `amadeus_backend::aux::fetch_err`:
  - `pub enum FetchFailure { BadUrl(String), Aborted, Failed }`
  - `fn message(&self) -> String` returns `"Failed to parse URL from {url}"`, `"This operation was aborted"` or
    `"fetch failed"`.
- `amadeus_backend::aux::input`:
  - `parse_json_lenient(&[u8]) -> Value`
  - `opt_trimmed(&Value, &str) -> Result<Option<String>, TypeErr>`
  - `pub enum FormValue { Text(String), File(FormFile) }`, `pub struct FormFile { bytes: Bytes, content_type: Option<String>, filename: String }`
  - `pub fn parse_form(content_type: Option<&str>, body: Bytes) -> Result<IndexMap<String, FormValue>, InvalidForm>`
    (multer + form_urlencoded; first occurrence wins)
- `amadeus_backend::aux::tts`:
  - `pub async fn synthesize(http, raw_json: &[u8]) -> ApiReply` (§3.1)
  - `pub async fn health(http, raw_json: &[u8]) -> ApiReply` (§3.2, `HealthBodyMode::ReadBeforeStatus`)
- `amadeus_backend::aux::stt`:
  - `pub async fn transcribe(http, form: IndexMap<String, FormValue>) -> ApiReply` (§3.3)
  - `pub async fn health(http, raw_json) -> ApiReply` (`ReadAfterOk`)
- `amadeus_backend::aux::rag`:
  - `pub async fn embed(http, raw_json) -> ApiReply` (§3.5)
  - `pub async fn health(http, raw_json) -> ApiReply` (`Ignore`)
- `amadeus_backend::aux::health`:
  - `pub enum HealthBodyMode { ReadBeforeStatus, ReadAfterOk, Ignore }`
  - `pub async fn probe(http, server_url: &str, mode) -> Value` (always status 200)
  - `fn spread_into_available(body: Option<Value>) -> Value` (`preserve_order`; array index keys)
- Typed convenience wrappers for IPC (they build the `raw_json`/form and call the functions above):
  `tts_synthesize_typed(&TtsSynthesizeBody)`, `stt_transcribe_typed(fields, audio: Vec<u8>, mime: Option<String>)`,
  and so on.
- `amadeus_backend::aux::allowlist::check(server_url, &Option<Vec<String>>) -> Result<(), ApiReply>`.
  Optional; on rejection return 403 `{"error":"serverUrl not allowed"}`, a new string, not in V1.
- `amadeus_backend::local_proxy` (library part), with fn names mirroring V1:
  - `pub struct LocalProxyConfig { host: String, port: u16 }` with `from_env()` reading `LOCAL_PROXY_HOST` and
    `LOCAL_PROXY_PORT` (JS parseInt-lenient: leading digits)
  - `pub fn target_regions(preferred: &str) -> Vec<String>`
  - `pub fn build_vertex_url(project: &str, location: &str, model: &str, method: &str) -> String`
  - `pub fn build_gemini_body(messages: &[Value], use_grounding: bool) -> Value`
  - `pub fn extract_gemini_response(payload: Option<&Value>) -> String`
  - `pub fn gcloud_candidates() -> Vec<PathBuf or String>`
  - `pub async fn run_gcloud_access_token(candidate: &str) -> String` (10 s timeout, kill on timeout)
  - `pub struct VertexTokenCache { token: String, expires_at: Instant }` with `get_or_refresh()` (50 min)
  - `pub fn is_retryable_status(code: u16) -> bool`
  - `pub async fn call_vertex_with_fallback(...) -> Result<String, String>`
  - `pub async fn serve(cfg) -> std::io::Result<()>`, using hyper or axum with **manual method/path dispatch**
    so HEAD on `/health` returns 404 as in V1
  - `fn write_json(status, &Value) -> Response` adds the CORS headers and
    `Content-Type: application/json; charset=utf-8`
  - `async fn read_json_body(body) -> Result<Value, &'static str>` (limit 2 000 000; errors
    `"Payload too large"` and `"Invalid JSON"` map to status 500)
- Binary `amadeus-backend/src/bin/amadeus-local-proxy.rs`, or a tiny crate `amadeus-local-proxy` depending on
  amadeus-backend. It prints `[local-proxy] listening on http://{host}:{port}`.

### `amadeus-server` (axum)
- `config::ServerConfig` (§8.5) with `from_env_and_args()` (clap: `--host`, `--port`, `--base-path`,
  `--public`, `--dist`).
- `routes::aux`: handlers `tts_synthesize`, `tts_health`, `stt_transcribe`, `stt_health`, `rag_embed`,
  `rag_health`.
  - Extractors: `HeaderMap` + `Bytes` only. No `Json` or `Multipart` extractors, because of their content-type
    strictness.
  - Each handler calls the backend fn and returns `ApiReply::into_response()`.
- `routes::api_method_fallback`: OPTIONS gives 204 + `allow: OPTIONS, POST`; other methods give 405 with an
  empty body (use `MethodRouter::fallback`).
- `routes::meta`: `web_manifest` (`application/manifest+json`) and `service_worker` (`/sw.js`).
- `static_files`:
  - `resources_service(public_dir)`, a ServeDir on `/resources`
  - `root_public_service(public_dir)`
  - `dist_assets_service(dist_dir)`, precompressed br/gzip
  - `spa_index(dist_dir, known_routes)`
- `middleware`:
  - `cache_policy::CachePolicyLayer` (§7.3 algorithm, constants `NO_STORE_SET`, `PUBLIC_SET`,
    `NOT_FOUND_CACHE_CONTROL`, option `immutable_hashed_assets`)
  - `trailing_slash::redirect_308` (§7.4, adds the `Refresh` header)
  - `CompressionLayer` (gzip + br, default predicate)
  - `DefaultBodyLimit::max(32 MiB)` on `stt/transcribe` (V1 has no limit)
  - tracing
- `reply::impl IntoResponse for ApiReply`:
  - `Json` produces `content-type: application/json` with `serde_json::to_vec` (`preserve_order`);
  - `Binary` produces the content type plus `Cache-Control: no-store` (overridden by the layer);
  - `Empty` produces the status with no body and no content-type.

### `amadeus-desktop` (Tauri 2)
- Commands (all `async`, state = shared `reqwest::Client`, each returning `ApiReply`):
  - `tts_synthesize(body: TtsSynthesizeBody)`
  - `tts_health(body: ServerUrlBody)`
  - `stt_transcribe(fields: SttTranscribeFields, audio: Vec<u8> /* or tauri::ipc::Request raw body */, mime: Option<String>)`
  - `stt_health(body)`, `rag_embed(body: RagEmbedBody)`, `rag_health(body)`
- `ApiReply::Binary` carries base64 audio for simplicity (one sentence of WAV is 100–500 KB). An optimization
  is to return `tauri::ipc::Response` raw bytes plus the content type in a small header frame.
- CSP must allow:
  - `media-src blob:` for TTS playback (`new Audio(blobUrl)`)
  - `style-src`/`font-src` for `fonts.googleapis.com` and `fonts.gstatic.com`, if the Google Fonts `@import` is
    kept
  - `connect-src ipc: http://ipc.localhost`
- Desktop doesn't serve `sw.js` or the manifest, and has no cache headers.

### `amadeus-ui` (Leptos CSR) touch points
- `transport::Transport` trait: `async fn post_json(path, body: Value) -> TransportResponse` and
  `async fn post_form(path, fields, audio: Blob) -> TransportResponse`.
- `TransportResponse { status: u16, content_type: Option<String>, body: Vec<u8> }` with `ok()` (2xx) and
  `json() -> Option<Value>` (`None` on parse failure, i.e. `.catch(() => null)`).
- `HttpTransport` uses `{AMADEUS_BASE_PATH}{API_*}` via `web_sys::fetch` (`gloo-net`).
- `TauriTransport` maps `ApiReply` into `TransportResponse`.
- Selection: `window.__TAURI_INTERNALS__` present means Tauri.
- `config::base_path()` = `option_env!("AMADEUS_BASE_PATH").unwrap_or("/amadeus/app")`.

---

## Open questions / risks

1. **Manifest.** Fix V1's 404 (serve `{base}/manifest.webmanifest`) or keep it 404 for strict parity? The
   recommendation is to serve it. Also: fix the broken `/favicon.ico` icon by serving
   `app/amadeus/app/favicon.ico` there, and/or add the 1024 px PNG icon? Should the description say "V2"? No V1
   string carries a version; the user asked for the Rust build to be labelled V2, so decide where.
2. **SSRF.** The six aux routes let anyone on the internet make the server request arbitrary URLs (GET
   `<url>/health`, POST `<url>/synthesize|transcribe|embed`). V1 has no guard. Decide whether the public
   deployment enables `AMADEUS_PROXY_ALLOWLIST` (for example `localhost:5100-5102,127.0.0.1:*`). Every page load
   already triggers three server-side GETs to `localhost:510x/health` (§5.2).
3. **Localhost semantics.** In the web build, "server URL" means the server's network. In the desktop build it
   means the user's machine. The same settings key and default (`http://localhost:5100`) behave differently;
   make sure the UI text and docs say so.
4. **Caching of hashed assets.** V1 serves every JS/CSS bundle with no-store, so users re-download the bundles
   on every visit. Trunk's hashed files could be `immutable`. That is safe because `index.html` stays no-store,
   and the WASM is several MB. Decide whether to keep strict parity (default `false` in this spec) or opt in.
5. **`/resources` caching.** `public, max-age=86400, stale-while-revalidate=604800` at the browser and
   Cloudflare means updated models or textures, such as the currently modified `texture_00.png`, can stay stale
   for 1–8 days. V1 works around it with `?v=N` on the runtime JS only. V2 could append `?v={AMADEUS_BUILD_ID}`
   to model and texture fetches in the live2d crate. Needs a decision with the live2d spec.
6. **Base path and co-hosting.** The production origin also serves the DSUV portfolio (Next.js) via a
   Cloudflare Tunnel. Moving Amadeus to amadeus-server needs path-based ingress (§8.4). Root-level PNG names
   (`/amadeus_logo.png`, `*_STEP*.png`) must be routed explicitly.
7. **SPA head per route.** One `index.html` means `/amadeus` (landing) would inherit the `/amadeus/app` head
   (theme-color, `viewport-fit=cover`, the manifest, description "Real Amadeus Web") unless `leptos_meta`
   overrides it per route. Alternatively, the server can serve a second HTML shell. Coordinate with the
   pages/landing spec.
8. **Desktop build differences.**
   - The Trunk `--public-url` differs (`/` vs `/amadeus/app/`), so the router must accept both roots.
   - Bundling `/resources` adds about 96 MB (about 73.6 MB of CJK fonts). Exclude the Unity `.anim` files
     (about 12 MB, unused) and consider subsetting fonts.
   - Unicode (`紅莉栖`) and space-containing filenames must survive the installers (MSI, DMG, AppImage, deb) and
     Tauri's asset protocol percent-decoding.
9. **Google Fonts `@import`.** Desktop builds need network access and CSP allowances for it, and it fails
   silently offline. The local `@font-face` fallbacks exist (`/resources/fonts/NotoSerif*.otf`).
10. **Tailwind v4 in the Rust pipeline.** The class scan must include the Rust sources and `index.html`. Trunk's
    tailwind integration may need a pre-build hook. The stylesheet order (root, then amadeus) must be preserved.
11. **reqwest vs undici differences.** Enable automatic decompression and `no_proxy()`, and set a 300 s read
    timeout. Without that, audio could be corrupted, localhost could be routed through an env proxy, or requests
    could hang forever.
12. **The axum `Json`/`Multipart` extractors are too strict.** V1 accepts JSON with any content type and
    urlencoded forms. Using the stock extractors silently changes 400 messages and statuses (415 and 400
    instead of V1's).
13. **The JS TypeError → empty 500 quirk** (non-string `serverUrl`, `text` or `speaker`). It is replicated in
    the spec. Confirm that's wanted versus a cleaner 400. The client never sends non-strings, so either choice is
    safe.
14. **Health reason phrase.** V1 uses the upstream's literal HTTP/1.1 reason phrase; hyper only exposes
    non-canonical phrases through the `ReasonPhrase` extension. The fallback `canonical_reason()` can differ
    cosmetically. The text is never displayed.
15. **Local proxy on Windows.** The V1 `cmd.exe /d /s /c "\"path\" auth …"` invocation is likely broken by
    libuv quoting. V2 should use Rust's batch-file handling or `raw_arg`, plus `CREATE_NO_WINDOW`. On macOS,
    apps launched from a GUI have a minimal PATH, so add Homebrew and SDK candidate paths (an improvement, not
    parity).
16. **Local proxy errors return 500 instead of 400** for invalid JSON and oversize payloads, and region failover
    doesn't cover network errors. Preserve both unless told otherwise.
17. **`apiKey` forwarding.** The chat route forwards the whole payload, including `apiKey`, to the local proxy.
    That is harmless on localhost but worth noting if the proxy host is remote (`Config_LocalProxyHost` is
    user-editable).
18. **Body limits.** V1 has none on the Next routes. Set ≥ 32 MiB on `stt/transcribe` and the default on the
    others. The local proxy's 2 000 000 limit counts UTF-16 units in V1 (Japanese counts 1 per character); in
    Rust, count bytes or chars (counting chars is closer).
19. **CSR first paint.** V1 SSR showed the login screen immediately. V2 shows black until the WASM loads. Keep
    the 16 image preloads, but they compete with the WASM download; consider `fetchpriority="low"` on the 3.4 MB
    and 2.2 MB images or preloading the WASM. Measure.
20. **Node version context.** The V1 behaviour was observed on Node 22.22.0 and undici's fetch. The error
    strings (`fetch failed`, `This operation was aborted`, `Failed to parse URL from …`) are Node-specific and
    reproduced on purpose.

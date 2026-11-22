# RealAmadeus V2 porting spec — subsystem `server-chat`

Target crates: **amadeus-backend** (provider clients, streaming, Vertex/Google auth, FreeAPI, rate limiter, chat log, quota) and **amadeus-server** (axum route handlers). DTOs shared with the UI/desktop go to **amadeus-core**.

Source of truth: working tree of `/home/elvelt_admin/Elvelt/dsuv` on 2026-11-16 (branch `main`, HEAD `7e45174`). **Note:** `rate-limit.ts`, `route.ts`, `shared.ts`, `stream/route.ts` and `vertex-helpers.ts` have **uncommitted** modifications. This spec describes the working tree, which differs from HEAD as follows:
- `MAX_GLOBAL_REQUESTS` is 65000 (HEAD: 25000). The error text changed with it.
- `DEFAULT_AI_STUDIO_API_KEY` was added to `shared.ts`.
- FreeAPI (provider 8) now tries Google AI Studio first (`fetchFreeApiResponse` / `createFreeApiStream`). HEAD went straight to Vertex with `gemini-3.1-flash-lite` / `realamadeusfreecredit` / `global`.

All paths below are relative to `/home/elvelt_admin/Elvelt/dsuv` unless absolute. `CHAT = app/amadeus/app/api/chat`.

---

## 0. Files in scope

| File | Lines | Role |
|---|---|---|
| `CHAT/route.ts` | 543 | `POST /amadeus/app/api/chat`: non-streaming chat for all 9 providers, plus private Vertex service-account and gcloud helpers |
| `CHAT/stream/route.ts` | 540 | `POST /amadeus/app/api/chat/stream`: SSE streaming chat |
| `CHAT/shared.ts` | 198 | Payload type, provider ids, body builders, response extractors, Vertex URL builder |
| `CHAT/vertex-helpers.ts` | 672 | Vertex auth (service account, embedded credential, gcloud), multi-region failover (stream and non-stream), FreeAPI (AI Studio, then Vertex fallback) |
| `CHAT/rate-limit.ts` | 110 | Client-IP extraction, per-IP sliding window, global FreeAPI counter persisted to file, JSONL chat log |
| `CHAT/quota/route.ts` | 13 | `GET /amadeus/app/api/chat/quota` |
| `CHAT/vertex-proxy/route.ts` | 310 | `POST /amadeus/app/api/chat/vertex-proxy`: gcloud-only Vertex proxy, optional SSE. **No first-party caller.** |

Related files I read for context (they belong to other subsystems):
- `lib/amadeus/services/ai-service.ts`: the only client of `/chat` and `/chat/stream`. It builds the payload and parses SSE.
- `lib/amadeus/app-settings.ts`: localStorage defaults for every `Config_*` key the payload uses.
- `components/amadeus/RealAmadeusApp.tsx`: picks streaming vs non-streaming per provider (L1789-1796), consumes quota (L330-369, 605, 3120), maps server error strings to user messages (`handleApiError`, L1653-1690).
- `scripts/amadeus-local-proxy/server.mjs`: the target of provider 7.
- `next.config.ts`: global response headers that override route-handler headers (§1.3).
- `app/api/atchannel/chat/route.ts`: the only user of `CANOPYWAVEAPI`. Not part of RealAmadeus (Appendix A).

Not in effect, do not port: the repo-root `patch*.js` scripts are historical code-mod scripts. In particular, `patch.js` adds `finishReason` handling to `extractGeminiResponse`, but the current `shared.ts` does **not** contain it. The repo-root `test*.js|mjs` files are ad-hoc experiments.

---

## 1. Endpoint inventory and HTTP-level contract

URL derivation: Next App Router directory `app/amadeus/app/api/chat/...`. `next.config.ts` sets no `basePath`. Production runs `next start -p 9501` (dev: `next dev -p 9601`) behind a reverse proxy that sets `X-Forwarded-For`. All 65,381 log lines carry a real IP; none are `"unknown"`. In V2 the base is configurable and defaults to `/amadeus`.

| Method | Path | Handler | Rate-limited | Global counter | Writes chat log | Success response |
|---|---|---|---|---|---|---|
| POST | `/amadeus/app/api/chat` | `route.ts` `POST` (L203-542) | yes (per-IP 20/min) | provider 8 or missing | yes (every `sendResponse`) | `200 {"response": string}` |
| POST | `/amadeus/app/api/chat/stream` | `stream/route.ts` `POST` (L438-540) | yes | provider 8 or missing | yes (on `complete` event with non-empty text) | `200 text/event-stream` |
| GET | `/amadeus/app/api/chat/quota` | `quota/route.ts` `GET` | no | read only | no | `200 {"count","max","remaining"}` |
| POST | `/amadeus/app/api/chat/vertex-proxy` | `vertex-proxy/route.ts` `POST` | **no** | no | **no** | `200 {"response"}` or SSE if header `x-vertex-stream: true` |

### 1.1 Method handling (Next auto-implementation, `next/dist/server/route-modules/app-route/helpers/auto-implement-methods.js`)
- POST-only routes: `OPTIONS` returns `204`, empty body, header `Allow: OPTIONS, POST`. `GET`, `HEAD`, `PUT`, `DELETE`, `PATCH` return `405` with an empty body.
- Quota (GET): `HEAD` reuses `GET` with the body omitted. `OPTIONS` returns `204` with `Allow: GET, HEAD, OPTIONS`. Other methods return `405`, empty body.
- Trailing slash: Next's default `trailingSlash: false` makes `/amadeus/app/api/chat/` answer **308** with `Location: /amadeus/app/api/chat`.
- No CORS headers are emitted. Same-origin browser use only.

### 1.2 Request body parsing (all POST handlers)
- `await request.json().catch(() => null)`. **The `Content-Type` header is ignored**, so any content type works if the body is JSON. A leading UTF-8 BOM is stripped. There is no size limit.
- If parsing fails, the body is JSON `null`, the value is not an object, or `messages` is not an array, the handler returns **400** `{"error":"Invalid request payload"}` with `Content-Type: application/json`.
- Rust: do **not** use axum's `Json` extractor. It rejects requests without `application/json` (415) and uses a different error body. Read `Bytes`, strip an optional BOM, and call `serde_json::from_slice::<serde_json::Value>`. Raise axum's `DefaultBodyLimit` (default 2 MB) to at least 16 MB.

### 1.3 Effective response headers (important: Next header-merge rule)
`next.config.ts` `headers()` defines rules `source: '/amadeus/:path*'` and `source: '/(.*)'`. Both match every API path and both set identical values:
```
Cache-Control: no-store, no-cache, must-revalidate, proxy-revalidate, max-age=0
Pragma: no-cache
Expires: 0
Surrogate-Control: no-store
CDN-Cache-Control: no-store
```
Next 16.1.6 applies config headers to the Node response before the handler runs (`router-utils/resolve-routes.js` L510-530). `send-response.js` then copies handler headers **only if the header is not already present**; `set-cookie`, `www-authenticate`, `proxy-authenticate` and `vary` are exempt. **The config `Cache-Control` therefore wins.** Effective headers:

| Response kind | Headers as written by the handler | Effective V1 headers on the wire |
|---|---|---|
| JSON (`NextResponse.json`, `Response.json`, `new Response(JSON.stringify(..), {headers: {"Content-Type":"application/json"}})`) | `Content-Type: application/json` (quota also sets `Cache-Control: no-store`, which is dropped) | `Content-Type: application/json` plus the 5 headers above |
| SSE (`/stream`, `vertex-proxy` with stream header) | `Content-Type: text/event-stream; charset=utf-8`, `Cache-Control: no-cache, no-transform`, `Connection: keep-alive` | `Content-Type: text/event-stream; charset=utf-8`, `Connection: keep-alive`, plus the 5 headers above. **`no-transform` is lost.** |

Other Next defaults are transport-only and need not be replicated: `X-Powered-By: Next.js`, chunked transfer, and gzip compression through Next's `compression` middleware. Compression applies to SSE as well because `no-transform` is gone, and Next flushes after every chunk. V2 recommendation: emit exactly the effective headers above. Never compress `text/event-stream`; tower-http's default predicate already excludes it. Send `Connection: keep-alive` only on HTTP/1.1, because hyper forbids connection-specific headers on HTTP/2.

### 1.4 Process model and state sharing
V1 is one Node process. `rate-limit.ts` module state (counter, per-IP map) is shared by `/chat`, `/chat/stream` and `/chat/quota`. I verified this in `.next/server/chunks/*`: Turbopack module id `48727` is the same in all three route chunks, and they share `[turbopack]_runtime.js`. JS is single-threaded, so check-and-increment is atomic. V2 must use one shared `Arc<RateLimiter>` behind a `Mutex`.

---

## 2. Wire types (`shared.ts`, client `ai-service.ts`)

### 2.1 `ChatMessage` (`lib/amadeus/types.ts`)
```ts
type ChatRole = "system" | "user" | "assistant";
interface ChatMessage { role: ChatRole; content: string; }
```
The server never validates roles. OpenAI-like providers receive `messages` **verbatim**: every message object exactly as received, including unknown roles or extra keys. The Gemini and Claude builders map roles as described in §3.

### 2.2 `ChatRequestPayload` (`shared.ts` L3-17)
Client builder: `ai-service.ts` `buildPayload` (L21-46). Serialization key order is the order below. It matters only for the provider-7 pass-through bytes.

| Field | TS type | Client source (localStorage key, client default) | Notes |
|---|---|---|---|
| `messages` | `ChatMessage[]` | conversation built by UI (system prompt, trimmed history of 30) | must be an array, else 400 |
| `provider` | `number` | `Config_ApiProvider` via `getInt(.., 8)`. AppSettings default is `8` | handlers use strict `===` |
| `apiKey` | `string` | `Config_ApiKey_${provider}`, falling back to `Config_ApiKey` if empty | |
| `model` | `string` | `Config_ModelName_${provider}`, falling back to `Config_ModelName` if empty | |
| `webSearch` | `boolean` | `Config_WebSearch` `=== 1` (default 0) | |
| `vertexProject` | `string` | `Config_VertexProject` (default `""`) | |
| `vertexLocation` | `string` | `Config_VertexLocation` (default `"us-central1"`) | |
| `localProxyHost` | `string` | `Config_LocalProxyHost` (default `"127.0.0.1"`) | |
| `localProxyPort` | `string` | `Config_LocalProxyPort` (default `""`) | |
| `useGCloudCLI` | `boolean` | `Config_GCloudCLI` `=== 1` (default **1**) | exact casing `useGCloudCLI` |
| `ollamaHost` | `string` | `Config_OllamaHost` (default `"http://localhost:11434"`) | |
| `openaiCompatible` | `boolean?` | `Config_OpenAICompatible` `=== 1` (default 0) | optional in server type |
| `openaiBaseUrl` | `string?` | `Config_OpenAIBaseUrl` (default `""`) | optional in server type |

The localStorage values are raw strings (`localStorage.setItem(key, String(value))`). `getInt` is `parseInt(raw, 10)` with the default used when the result is not finite. These keys and formats must stay identical in V2 (the UI subsystem owns them).

### 2.3 Provider ids (`shared.ts` L19-27) and routing matrix

| id | const | UI name (`PROVIDER_NAMES`, RealAmadeusApp.tsx L325) | `/chat` branch | `/chat/stream` branch | Official client uses streaming? (RealAmadeusApp L1789-1796) |
|---|---|---|---|---|---|
| 0 | `PROVIDER_OPENAI` | "OpenAI" | §6.2.0 | `streamOpenAILike` | no |
| 1 | `PROVIDER_GEMINI` | "Google Gemini" | §6.2.1 | `streamGemini` | **yes** |
| 2 | `PROVIDER_CLAUDE` | "Anthropic Claude" | §6.2.2 | `nonStreamingFallback` | no |
| 3 | `PROVIDER_GROQ` | "Groq" | §6.2.3 | `streamGroq` | **yes** |
| 4 | `PROVIDER_VERTEX` | "Vertex AI" | §6.3 | `streamVertex` | **yes** |
| 5 | `PROVIDER_OLLAMA` | "Ollama" | §6.2.5 | `streamOpenAILike` (NDJSON) | **yes** |
| 6 | `PROVIDER_OPENROUTER` | "OpenRouter" | §6.2.6 | `streamOpenAILike` | **yes** |
| 7 | `PROVIDER_LOCAL` | "" (empty) | §6.2.7 | `nonStreamingFallback` | no |
| 8 | `PROVIDER_FREE` | "FreeAPI" | §6.2.8 | `streamFree` | no |
| other or missing | none | none | missing or `null`: treated as 0. Any other value: `400 "Unknown provider."` | `nonStreamingFallback` | n/a |

Client quirk: the streaming decision reads `AppSettings.getInt("Config_ApiProvider", 0)` (default **0**), while `buildPayload` uses default **8**. A user with no `Config_ApiProvider` key therefore sends provider 8 through non-streaming `/chat`. In practice `initializeDefaults()` writes 8 first.

### 2.4 JS coercion semantics the server relies on (port with helpers)
- `x?.trim() ?? ""`: missing or `null` becomes `""`. A string becomes the JS-trimmed string. **Any other type throws `TypeError`.** For `payload.model` in `route.ts` L234 this happens before the `try`, so the request ends as an uncaught error: HTTP 500 with an empty body. Elsewhere it is caught: `500 {"error":"payload.x?.trim is not a function"}`, or an `error` SSE event. The official client always sends strings.
- `x?.trim() || "default"`: missing, null, or an empty string after trim all yield the default.
- `!payload.apiKey`: missing, null, `""`, `0` and `false` all count as "no key". Whitespace-only `"  "` counts as a key.
- `Boolean(payload.webSearch)`, `Boolean(payload.useGCloudCLI)`, and `payload.openaiCompatible && payload.openaiBaseUrl` use JS truthiness. Recommended Rust helper: `js_truthy(&Value)`. `Null` is false. `Bool(b)` is `b`. A number is false only for 0 or NaN. A string is false only when empty. Arrays and objects are true.
- JS `String.prototype.trim()` removes U+0009–U+000D, U+0020, U+00A0, U+1680, U+2000–U+200A, U+2028, U+2029, U+202F, U+205F, U+3000 and **U+FEFF**. It does **not** remove U+0085. Rust's `str::trim` does the opposite for those two code points, so provide `js_trim()`.
- `payload.provider` comparisons are strict: `"8"` (string) is not `8`, while `8.0` is `8`. Recommended normalization: `enum ProviderVal { Missing, Null, Num(f64), Other }`.
  - `route.ts` `provider = payload.provider ?? 0`: Missing and Null map to 0. `Num(n)` with integral `n` in 0..=8 selects that provider. Anything else gives "Unknown provider.".
  - The global counter applies only when the value is `Num(8)` or `Missing` (`provider === 8 || provider === undefined`). Null does **not** count.
  - The FreeAPI fallbacks apply only when the value is `Num(8)`.

### 2.5 `DEFAULT_AI_STUDIO_API_KEY` (`shared.ts` L28)
This is a hard-coded Google AI Studio key string. It is part of the uncommitted change. The value is intentionally not reproduced here. It is used only by `getAiStudioApiKey()` (§8.11) as the last fallback. **V2 must not hard-code it.** Read it from env or config on the server, and never compile it into desktop or WASM artifacts. See Open questions.

---

## 3. Request body builders (`shared.ts`), exact JSON

All objects serialize in the key order shown, which equals JS insertion order. Use structs or `serde_json` with the `preserve_order` feature. Number formatting: `2048` integer, `0.85`, `0.9`. serde_json/ryu output matches JS for these values.

### 3.1 `isValidGroqModel(model, allowCompound)` (L30-42)
`lowered = model.toLowerCase()`. Return true if `lowered` contains any of `"llama"`, `"mixtral"`, `"gemma"`, `"qwen"` or `"deepseek"`. Otherwise return `allowCompound && lowered.includes("compound")`.
Golden results as `[allowCompound=true, allowCompound=false]`:
- `llama-3.1-8b-instant` → [true, true]
- `groq/compound` → [true, false]
- `compound-beta` → [true, false]
- `openai/gpt-oss-120b` → [false, false]
- `Qwen/Qwen3-32B` → [true, true]
- `moonshotai/kimi-k2-instruct` → [false, false]
- `DeepSeek-r1` → [true, true]

So valid modern Groq models such as gpt-oss or kimi get **replaced** by `qwen3-32b`. This is an inherited quirk.

### 3.2 `buildOpenAIBody(messages, model, stream = false, isGroq = false)` (L44-66)
```
{ "model": model, "messages": <messages verbatim>, "max_tokens": 2048,
  [isGroq]  "temperature": 0.85, "top_p": 0.9,
  [isGroq && model.toLowerCase().includes("qwen")]  "reasoning_format": "hidden",
  [stream]  "stream": true }
```
Golden values (exact bytes):
- `buildOpenAIBody([{role:"user",content:"U"}],"gpt-4o")` → `{"model":"gpt-4o","messages":[{"role":"user","content":"U"}],"max_tokens":2048}`
- `(.., "qwen3-32b", true, true)` → `{"model":"qwen3-32b","messages":[{"role":"user","content":"U"}],"max_tokens":2048,"temperature":0.85,"top_p":0.9,"reasoning_format":"hidden","stream":true}`
- `(.., "llama-3.3-70b-versatile", false, true)` → `{"model":"llama-3.3-70b-versatile","messages":[{"role":"user","content":"U"}],"max_tokens":2048,"temperature":0.85,"top_p":0.9}`

### 3.3 `buildGroqCompoundBody(messages, stream)` (L68-79)
`{ "model": "groq/compound", "messages": <verbatim>, "max_tokens": 2048, "temperature": 0.85, [stream] "stream": true }`. It has no `top_p` and no `reasoning_format`.
Golden: `{"model":"groq/compound","messages":[{"role":"user","content":"U"}],"max_tokens":2048,"temperature":0.85,"stream":true}`

### 3.4 `buildGeminiBody(messages, useGrounding = false)` (L81-110), used by Gemini, Vertex and FreeAPI
Algorithm:
1. `contents = []`, `systemInstruction = ""`.
2. For each message in order:
   - If `role === "system"`: `systemInstruction = content`. The **last** system message wins, wherever it sits in the list. System messages never appear in `contents`.
   - Otherwise push `{ "role": role === "assistant" ? "model" : "user", "parts": [ { "text": content } ] }`. Any non-assistant role (`user`, unknown) becomes `"user"`. Consecutive same-role turns are **not** merged.
3. `body = { "contents": contents, "generationConfig": { "maxOutputTokens": 2048 } }`.
4. If `systemInstruction` is a non-empty string, add `"system_instruction": { "parts": [ { "text": systemInstruction } ] }`. The key is snake_case and has no `role` field.
5. If `useGrounding`, add `"tools": [ { "googleSearch": {} } ]`.

Golden for messages `[sys S, user U, asst A, sys S2, user U2]` with grounding:
`{"contents":[{"role":"user","parts":[{"text":"U"}]},{"role":"model","parts":[{"text":"A"}]},{"role":"user","parts":[{"text":"U2"}]}],"generationConfig":{"maxOutputTokens":2048},"system_instruction":{"parts":[{"text":"S2"}]},"tools":[{"googleSearch":{}}]}`
Golden for messages `[user U, tool T]` without grounding: `{"contents":[{"role":"user","parts":[{"text":"U"}]},{"role":"user","parts":[{"text":"T"}]}],"generationConfig":{"maxOutputTokens":2048}}`

### 3.5 `buildClaudeBody(messages, model)` (L112-132)
The last system message's content becomes `system`. The other messages map to `{role: role === "assistant" ? "assistant" : "user", content}`.
`{ "model": model, "max_tokens": 2048, ["system": system]  (only if non-empty), "messages": [...] }`
Golden: `{"model":"m","max_tokens":2048,"system":"S2","messages":[{"role":"user","content":"U"},{"role":"assistant","content":"A"},{"role":"user","content":"U2"}]}`. Without a system message: `{"model":"m","max_tokens":2048,"messages":[...]}`.

### 3.6 Ollama bodies (inline in route files)
- Non-stream: `{ "model": model, "messages": <verbatim>, "stream": false }`
- Stream: `{ "model": model, "messages": <verbatim>, "stream": true }`

### 3.7 Local-proxy body (provider 7)
`JSON.stringify(payload)`: the **entire parsed request payload**, re-serialized. It includes `apiKey`, `messages`, every field, and any extra fields the caller sent. JS preserves key order, except that integer-like keys move first. Rust: keep the raw `serde_json::Map` with `preserve_order` and serialize it back.

---

## 4. Response extractors (`shared.ts`)

All of these navigate untyped JSON. Rust: use `Value::get` chains. An array index only works on arrays; if any step is missing or has the wrong type, return the fallback.

| Function | Path | Returns when value is a string | Otherwise |
|---|---|---|---|
| `extractOpenAIResponse` (L134-137) | `choices[0].message.content` | the string | `"[Parse Error]"`. This includes `content: null` (tool calls or refusal). |
| `extractGeminiResponse` (L139-143) | `candidates[0].content.parts[0].text` | the string (may be `""`) | `"[Parse Error]"` (e.g. `finishReason: "SAFETY"` with no content). **Only `parts[0]` is read**; later parts are ignored. |
| `extractClaudeResponse` (L145-148) | `content[0].text` | the string | `"[Parse Error]"` |
| `extractOpenAIStreamToken` (L150-153) | `choices[0].delta.content` | the string | `""` |

`"[Parse Error]"` is returned to the user **as normal response text**, not as an error, by every non-stream path except FreeAPI's AI Studio step. Streaming paths skip tokens equal to `"[Parse Error]"`.

### 4.1 `extractVertexStreamTokensLikeUnity(chunk)` (L155-173). Used only by `vertex-proxy` streaming.
- Regex, global: `/"text"\s*:\s*"((?:\\.|[^"\\])*)"/g`, applied to an arbitrary text chunk, not to lines.
- For each match, take group 1, then apply these replacements **in this order**, each global and sequential: `\n`→LF, `\r`→CR, `\t`→TAB, `\"`→`"`, `\\`→`\`. If the result is non-empty, push it. Empty results are skipped.
- Quirks to reproduce:
  - `\uXXXX` and `\/` are **not** decoded.
  - An escaped backslash followed by `n` (raw `\\n`) becomes backslash plus LF, because `\n` is replaced first.
  - Any `"text"` key anywhere in the JSON matches.
  - Tokens split across network chunks are lost or truncated.
- Rust regex: JS `.` excludes `\n \r \u2028 \u2029`, while Rust `.` excludes only `\n`. Use `"text"\s*:\s*"((?:\\[^\n\r\u{2028}\u{2029}]|[^"\\])*)"`.
- Golden: input `data: {"candidates":[{"content":{"parts":[{"text": "a\nb \"q\" \\n \u3042"}]}}]}\r\n{"text":""} {"text" : "c"}` (backslashes literal) gives `["a⏎b \"q\" \\⏎ \\u3042", "c"]`, i.e. as JSON `["a\nb \"q\" \\\n \\u3042","c"]`.

### 4.2 Inline extractors
- Ollama non-stream: `body?.message?.content ?? ""`. A non-string value such as a number would be passed through as-is; ignore that case.
- Ollama stream: per NDJSON line, `parsed.message?.content` if truthy.
- Local proxy: `body?.response ?? ""`.

---

## 5. URL builders and helpers

### 5.1 `buildVertexUrl(projectId, location, model, method)` (L175-186)
- `suffix = method === "streamGenerateContent" ? "?alt=sse" : ""`.
- If `location === "global"`: `https://aiplatform.googleapis.com/v1/projects/${projectId}/locations/global/publishers/google/models/${model}:${method}${suffix}`.
- Otherwise: `https://${location}-aiplatform.googleapis.com/v1/projects/${projectId}/locations/${location}/publishers/google/models/${model}:${method}${suffix}`.
- Nothing is percent-encoded; values are interpolated raw. WHATWG URL parsing in both undici and the Rust `url` crate normalizes the result identically.
- Golden: `("p","global","gemini-2.5-flash","streamGenerateContent")` → `https://aiplatform.googleapis.com/v1/projects/p/locations/global/publishers/google/models/gemini-2.5-flash:streamGenerateContent?alt=sse`. `("p","asia-northeast1","m","generateContent")` → `https://asia-northeast1-aiplatform.googleapis.com/v1/projects/p/locations/asia-northeast1/publishers/google/models/m:generateContent`.

### 5.2 Fixed upstream endpoints

| Provider | Non-stream URL | Stream URL |
|---|---|---|
| OpenAI | `https://api.openai.com/v1/chat/completions`, or `<openaiBaseUrl trimmed, one trailing "/" removed>/chat/completions` | same |
| Gemini (AI Studio) | `https://generativelanguage.googleapis.com/v1beta/models/${model}:generateContent` | `https://generativelanguage.googleapis.com/v1beta/models/${model}:streamGenerateContent?alt=sse` |
| Claude | `https://api.anthropic.com/v1/messages` | n/a (non-stream fallback) |
| Groq | `https://api.groq.com/openai/v1/chat/completions` | same |
| Ollama | `${host}/api/chat` | same |
| OpenRouter | `https://openrouter.ai/api/v1/chat/completions` | same |
| Vertex | `buildVertexUrl(.., "generateContent")` | `buildVertexUrl(.., "streamGenerateContent")` (has `?alt=sse`) |
| Local proxy | `http://${host}:${port}/chat` (see §6.2.7) | n/a |
| Google OAuth | `token_uri` from the credential (default `https://oauth2.googleapis.com/token`) | n/a |

### 5.3 `getInternalBaseUrl(request)` (L188-197): dead code
It is imported by `route.ts` and `stream/route.ts` but never called. It returns `${protocol}//${host}` from `request.url`. On parse failure it uses `${x-forwarded-proto || "http"}://${host header || "localhost:3000"}`. Do not port it, or port it as an unused helper.

### 5.4 `resolveLocalProxyBaseUrl(rawHost, rawPort)` (`route.ts` L36-51): dead code
It is never called.
- `host = rawHost.trim() || "127.0.0.1"`. The host must match `/^(localhost|(\d{1,3}\.){3}\d{1,3}|[a-zA-Z0-9.-]+)$/`, else `null`.
- An empty port returns `https://${host}`.
- Otherwise the port is parsed with `parseInt`. It must be finite and in 1..=65535, else `null`. The result is `http://${host}:${port}`.
- Not used by provider 7, which builds its URL inline (§6.2.7). Optional to port.

---

## 6. `POST {base}/app/api/chat` (non-streaming, `route.ts`)

### 6.1 Top-level algorithm (L203-542)
```
1. payload ← parse body (§1.2); invalid → 400 {"error":"Invalid request payload"}
2. rl ← checkRateLimit(request, payload.provider)          // raw value, §10.4
3. if !rl.allowed:
     if payload.provider === 8 → 200, body {"response": FREE_FALLBACK_RATE_LIMITED}, Content-Type: application/json   (NOT logged)
     else → 429 {"error": rl.errorMsg ?? "Too many requests. Please wait a minute."}   (errorMsg is always set, so the default never appears)
4. count ← rl.count
   question ← (last message with role === "user")?.content || ""
   ip ← getClientIp(request)
   console.log("\n[Req #" + count + "] Q: " + question)          // note the leading "\n"
5. provider ← payload.provider ?? 0
   webSearch ← Boolean(payload.webSearch)
   model ← payload.model?.trim() ?? ""                         // outside try (§2.4)
6. try { switch (provider) {...} }                              // §6.2, §6.3
   catch (error):
     if payload.provider === 8 → 200 {"response": FREE_FALLBACK_COMM_ERROR}   (NOT logged)
     else → 500 {"error": error.message (if Error) else "Unknown server error"}

sendResponse(text):
   console.log("[Req #" + count + "] A: " + text)
   logInteraction(ip, payload.provider ?? 0, question, text, count)     // §10.6
   return 200 {"response": text}
```
Error helper: `jsonError(message, status = 400)` returns `NextResponse.json({error: message}, {status})`.

Upstream error formatting, used by OpenAI, Gemini, Claude, Groq, OpenRouter and the Vertex SA, gcloud and API-key paths. Call it **E_NS**:
```
status  = upstream status
message = `${upstream.status} ${upstream.statusText}\n${JSON.stringify(parsedBodyOrNull)}`
```
- The upstream body is parsed with `.json().catch(() => null)`. A non-JSON body is printed as `null`.
- `statusText` is the HTTP/1.1 reason phrase. undici speaks HTTP/1.1, so you get e.g. `Too Many Requests`. Rust: use `status.canonical_reason().unwrap_or("")`, because HTTP/2 has no reason phrase.
- Golden: `400 Bad Request\nnull` (with a real LF).
- The parsed JSON is re-serialized. Re-serialization differences such as integer-like key order or `1.0` vs `1` are acceptable.

Network or transport errors from `fetch` throw `TypeError("fetch failed")` (verified on Node 22.22 / undici 6.23). They reach the outer catch and produce `500 {"error":"fetch failed"}`, or the FreeAPI fallback for provider 8. A malformed URL throws `TypeError("Failed to parse URL from <url>")`. **Rust must map `reqwest` send errors to exactly `"fetch failed"`** and URL errors to `"Failed to parse URL from <url>"`. The client's `handleApiError` substring-matches these strings (§15.4). The default reqwest text contains the URL, which often contains "model" and would change the user-visible message.

All `/chat` upstream fetches use `cache: "no-store"`, which is Next-specific and has no Rust equivalent. No explicit timeouts are set (§14).

### 6.2 Provider branches

#### 6.2.0 OpenAI (0), L238-266
1. If `!payload.apiKey`, return 400 `"API Key が設定されていません。CONFIGから設定してください。"`.
2. If `model === ""`, use `"gpt-4o"`. There is no other model rewriting.
3. URL: `https://api.openai.com/v1/chat/completions`. If `payload.openaiCompatible` and `payload.openaiBaseUrl` are both truthy (untrimmed value checked):
   - `base = openaiBaseUrl.trim()`.
   - If it ends with `/`, remove **one** character.
   - `url = base + "/chat/completions"`.
   - A whitespace-only base gives `"/chat/completions"`, which fails with 500 `Failed to parse URL from /chat/completions`.
4. Send POST with headers `Authorization: Bearer ${apiKey}` and `Content-Type: application/json`. Body: `buildOpenAIBody(messages, model)`.
5. If the response is not ok, use E_NS. Otherwise `sendResponse(extractOpenAIResponse(body))`.

#### 6.2.1 Gemini (1), L268-291
1. If there is no apiKey, return 400 (same message).
2. If `model === ""` or `model.startsWith("gpt")` (**case-sensitive** here), use `"gemini-2.5-flash"`.
3. POST to `.../v1beta/models/${model}:generateContent`. Headers: `x-goog-api-key: ${apiKey}`, `Content-Type: application/json`. Body: `buildGeminiBody(messages, Boolean(payload.webSearch))`.
4. If the response is not ok, use E_NS. Otherwise `sendResponse(extractGeminiResponse(body))`.

#### 6.2.2 Claude (2), L293-314
1. If there is no apiKey, return 400.
2. If `model === ""`, `model.startsWith("gpt")` or `model.startsWith("gemini")` (case-sensitive), use `"claude-3-7-sonnet-20250219"`.
3. POST to `https://api.anthropic.com/v1/messages`. Headers: `x-api-key: ${apiKey}`, `anthropic-version: 2023-06-01`, `Content-Type: application/json`. Body: `buildClaudeBody(messages, model)`.
4. If the response is not ok, use E_NS. Otherwise `sendResponse(extractClaudeResponse(body))`.

#### 6.2.3 Groq (3), L316-339
1. If there is no apiKey, return 400.
2. If `model === ""` or `!isValidGroqModel(model, true)`, use `"qwen3-32b"`.
3. Body: if `webSearch`, `buildGroqCompoundBody(messages, false)`. Otherwise `buildOpenAIBody(messages, model, false, true)`.
4. POST to `https://api.groq.com/openai/v1/chat/completions` with `Authorization: Bearer` and `Content-Type`.
5. If the response is not ok, use E_NS. Otherwise `sendResponse(extractOpenAIResponse(body))`.

#### 6.2.4 Vertex (4): see §6.3

#### 6.2.5 Ollama (5), L341-366
1. No API key is needed.
2. `host = payload.ollamaHost?.trim() || "http://localhost:11434"`. If `!host.startsWith("http")` (case-sensitive), prefix `"http://"`. A trailing slash is **not** stripped, so `http://h:11434/` gives `http://h:11434//api/chat`.
3. If `model === ""` or `model.toLowerCase().startsWith("gpt")`, use `"mistral"`.
4. POST to `${host}/api/chat` with only `Content-Type: application/json`. Body: `{model, messages, stream:false}`.
5. Parse the body, or use `null`.
6. If the response is not ok: status = upstream, `{"error": body?.error ?? "${status} ${statusText}"}`. **This is not E_NS.**
7. Otherwise `sendResponse(body?.message?.content ?? "")`.

#### 6.2.6 OpenRouter (6), L368-390
1. If there is no apiKey, return 400.
2. If `model === ""` or the lowercased model starts with `"gpt"`, use `"google/gemini-2.5-flash"`. The stream path uses a **different** default (§7.7).
3. POST to `https://openrouter.ai/api/v1/chat/completions`. Headers: `Authorization: Bearer ${apiKey}`, `Content-Type: application/json`, `HTTP-Referer: https://dsuv.vercel.app`, `X-Title: RealAmadeus`. Body: `buildOpenAIBody(messages, model, false, false)`.
4. If the response is not ok, use E_NS. Otherwise `sendResponse(extractOpenAIResponse(body))`.

#### 6.2.7 Local proxy (7), L498-517
1. `host = payload.localProxyHost?.trim() || "127.0.0.1"`.
2. `port = payload.localProxyPort?.trim() || "5103"`. The client default `""` therefore becomes 5103.
3. `url = "http://${host}:${port}/chat"`. If `host.startsWith("http")`, use `"${host}:${port}/chat"` instead (e.g. `https://x.y` gives `https://x.y:5103/chat`).
4. POST to `url` with only `Content-Type: application/json`. Body: the **full payload** (§3.7).
5. Parse the body, or use `null`.
6. If the response is not ok: status = upstream, `{"error": body?.error ?? "${status} ${statusText}"}`.
7. Otherwise `sendResponse(body?.response ?? "")`.
8. The request is made **from the server**. On the Web version `127.0.0.1` means the server host. On desktop it means the user's machine.

#### 6.2.8 FreeAPI (8), L519-526
1. Call `result ← fetchFreeApiResponse(payload)` (§8.12). It **mutates** `payload.model`, `payload.vertexProject` and `payload.vertexLocation` when it falls back to Vertex; nothing observable depends on that.
2. If `result` is an error, call `sendResponse(FREE_FALLBACK_COMM_ERROR)`. The fallback is **logged** as `replyText`.
3. Otherwise call `sendResponse(result.response)`. This may be `"[Parse Error]"` if the Vertex fallback returned 200 without text.

#### Default branch
Any other value returns 400 `"Unknown provider."`.

### 6.3 Vertex (4) decision tree, non-stream (L392-496)
```
useGCloudCLI ← Boolean(payload.useGCloudCLI)                  // client default TRUE

A) useGCloudCLI === true:
     result ← fetchVertexResponse(payload)                    // §8.10. Despite the flag name, this does NOT use gcloud for provider 4:
                                                               // auth = payload SA JSON → embedded SA → raw apiKey (x-goog-api-key); multi-region failover
     error → jsonError(result.error, result.status)           // 503 token problem | 400 no project | 502 all regions failed
     ok    → sendResponse(result.response)

B) useGCloudCLI === false:
   credentialRaw ← payload.apiKey?.trim() ?? ""
   looksLikeJson ← credentialRaw.startsWith("{")
   payloadSA ← parseServiceAccountCredential(credentialRaw)  // strict variant §9.1
   if looksLikeJson && !payloadSA → 400 "Vertex AI: サービスアカウントJSONキーの形式が不正です。"
   if model === "" || model.toLowerCase().startsWith("gpt") → model = "gemini-2.5-flash"
   location ← payload.vertexLocation?.trim() || "us-central1"
   embeddedSA ← loadEmbeddedServiceAccountCredential()      // §9.3, cached
   sa ← payloadSA ?? embeddedSA            // !!! the embedded SA beats a plain API key and an empty key

   B1) sa present:
        projectId ← payload.vertexProject?.trim() || sa.project_id
        if !projectId → 400 "Vertex AI: サービスアカウントJSON利用時は Project ID が必要です。"
        token ← exchangeServiceAccountToken(sa)               // throws → outer catch → 500 "Vertex service account token exchange failed: <detail>"
        POST buildVertexUrl(projectId, location, model, "generateContent")    // ONE region, no failover, no retry
             headers: Authorization: Bearer <token>, Content-Type: application/json
             body: buildGeminiBody(messages, webSearch)
        !ok → E_NS ; ok → sendResponse(extractGeminiResponse(body))

   B2) no sa AND credentialRaw === "":
        token ← getGcloudAccessToken()                       // route.ts' OWN cache (§9.4)
        token non-empty:
           projectId ← payload.vertexProject?.trim(); if empty → 400 "Vertex AI: gcloud認証利用時は Project ID が必要です。"
           POST buildVertexUrl(projectId, location, model, "generateContent") with Bearer token (one region)
           !ok → E_NS ; ok → sendResponse(extractGeminiResponse(body))
        token empty → 400 "Vertex AI: 認証情報が見つかりません。APIキーを設定するか、サーバーの gcloud auth login を実行してください。"

   B3) no sa AND credentialRaw is a non-JSON API key:
        candidates ← []
        if payload.vertexProject?.trim() non-empty:
            candidates.push(buildVertexUrl(project, location, model, "generateContent") + "?key=" + encodeURIComponent(apiKey))
        candidates.push("https://generativelanguage.googleapis.com/v1beta/models/" + model + ":generateContent")   // no ?key=
        lastStatus ← 500; lastStatusText ← "Upstream request failed"; lastBody ← null     // the defaults are unreachable
        for url in candidates (sequential, no sleep):
            POST url, headers x-goog-api-key: <apiKey>, Content-Type: application/json, body buildGeminiBody(messages, webSearch)
            body ← json or null
            ok → return sendResponse(extractGeminiResponse(body))
            else remember status/statusText/body
            (a network error throws → outer catch → 500 "fetch failed")
        → status lastStatus, error "<lastStatus> <lastStatusText>\n<JSON(lastBody)>"
```
`encodeURIComponent` leaves `A–Z a–z 0–9 - _ . ! ~ * ' ( )` unescaped and percent-encodes everything else as UTF-8. Golden: `AIza-_.!~*'() /+?=&:` → `AIza-_.!~*'()%20%2F%2B%3F%3D%26%3A`.

**Production reality:** the embedded credential file exists in the repo root, so B1 serves every Vertex request with `useGCloudCLI=false` that lacks SA JSON, including requests where the user typed an API key. B2 and B3 are reachable only when the file is absent or unparseable, which is the desktop situation in V2. Note also that the official client streams provider 4 (§7.8). That path ignores `useGCloudCLI` completely.

### 6.4 `/chat` status and body matrix

| Situation | Status | Body |
|---|---|---|
| Invalid JSON, non-object, or `messages` not an array | 400 | `{"error":"Invalid request payload"}` |
| Rate limited, provider 8 | 200 | `{"response":"[SAD] ごめんなさい、リクエストが多すぎるみたい……。少し休ませてからまた話しかけてね。"}` |
| Rate limited, global cap, provider missing | 429 | `{"error":"FreeAPIの全体リクエスト上限（65000回）に達しました。サービスを停止しています。"}` |
| Rate limited per IP, any other provider | 429 | `{"error":"リクエストが多すぎます。1分ほど待ってから再度お試しください。"}` |
| Missing apiKey (providers 0, 1, 2, 3, 6) | 400 | `{"error":"API Key が設定されていません。CONFIGから設定してください。"}` |
| Upstream non-OK | upstream status | E_NS string. Ollama and local use `body.error ?? "status statusText"`. |
| Vertex validation errors | 400 / 503 / 502 | §6.3, §8.10 |
| Unknown provider | 400 | `{"error":"Unknown provider."}` |
| Exception, provider 8 | 200 | `{"response":"[SAD] ごめんなさい、ちょっと通信の調子が悪いみたい……。少し待ってからもう一度試してみてくれる？"}` |
| Exception, other providers | 500 | `{"error": <message> or "Unknown server error"}` |
| Uncaught exception (non-string `model`, malformed `messages` entries such as `null` in `filter(m => m.role ...)`) | 500 | empty body (Next default) |
| Success | 200 | `{"response": "<text>"}` |

---

## 7. `POST {base}/app/api/chat/stream` (SSE, `stream/route.ts`)

### 7.1 Wire format (what the server emits)
- Each event is the UTF-8 bytes of `"data: " + JSON.stringify(obj) + "\n\n"`, enqueued as **one chunk**. In Rust, send one `Bytes` frame per event and flush after each.
- There are no `event:`, `id:` or `retry:` fields, no comments or heartbeats, no initial padding, and the server never emits `[DONE]`.
- Event objects, with exact key order:
  - `{"type":"token","token":"<delta>"}`
  - `{"type":"complete","fullResponse":"<entire text>"}`
  - `{"type":"error","error":"<message>"}`
- JSON escaping follows `JSON.stringify`, which serde_json matches. Non-ASCII, `/`, `<`, `>`, U+2028 and U+2029 stay raw. `"`, `\` and control characters are escaped (`\b \f \n \r \t`, others as `\u00XX`). Golden: complete event with text `a"\⏎あ/<>` → `data: {"type":"complete","fullResponse":"a\"\\\nあ/<>"}` followed by `\n\n`.
- After validation and rate limiting, the HTTP status is **always 200**, even when the upstream fails. Errors travel as `error` events.
- A well-formed stream ends with exactly one `complete` **or** one `error`. Some quirks end with neither (§7.11, §19); the client then emits `streamComplete(full)` with whatever tokens arrived.

### 7.2 Top-level algorithm (L438-540)
```
1. payload ← parse (§1.2); invalid → 400 {"error":"Invalid request payload"} (application/json)
2. rl ← checkRateLimit(request, payload.provider)
3. if !rl.allowed:
     provider === 8 → 200 SSE (headers §1.3), body =
         data: {"type":"token","token":FREE_FALLBACK_RATE_LIMITED}\n\n
         data: {"type":"complete","fullResponse":FREE_FALLBACK_RATE_LIMITED}\n\n
       then close.  NOT logged (the interceptor below is not installed).
     else → 429 application/json {"error": rl.errorMsg ?? "Too many requests. Please wait a minute."}
4. count, question, ip as in §6.1; console.log("\n[Req #<count>] Q: <question>")
5. Open SSE stream. Producer:
   a. Interceptor wraps every enqueue: decode the chunk; if it starts with "data: ", raw = rest.trim();
      if raw non-empty and raw != "[DONE]", try JSON.parse; if obj.type === "complete" && obj.fullResponse is truthy (non-empty):
         console.log("[Req #<count>] A: <fullResponse>"); logInteraction(ip, payload.provider ?? 0, question, fullResponse, count)
      THEN forward the chunk. Logging happens before the bytes are written. Empty completions are not logged.
   b. dispatch (strict ===): 3→streamGroq, 1→streamGemini, 4→streamVertex, 8→streamFree, 0|5|6→streamOpenAILike, else→nonStreamingFallback
   c. catch (error):
        payload.provider === 8 → token(FALLBACK_COMM) + complete(FALLBACK_COMM)
        else → error(error.message if Error else "Unknown streaming error")
   d. finally → close
6. return 200 + SSE headers (§1.3)
```
Exceptions reach (c) from un-guarded `fetch` calls in streamGemini, streamGroq and streamOpenAILike (`"fetch failed"`), and from body-read failures mid-stream (`"terminated"`, verified undici behavior). **Rust must map body-read errors to `"terminated"`** and send errors to `"fetch failed"`.

### 7.3 Generic SSE line reader used by all upstream parsers
JS pattern: `buffer += decoder.decode(chunk, {stream:true})`. While the buffer contains `"\n"`: `line = buffer.slice(0, i).trim()`, `buffer = buffer.slice(i + 1)`, then process the line. Data lines are lines where `line.startsWith("data: ")`; `raw = line.slice(6).trim()`. A line `data:{...}` without the space is ignored. CRLF is handled by `trim()`.

Rust: keep a `Vec<u8>` buffer and split on `b'\n'`, which is ASCII and never inside a multi-byte sequence. Decode each complete line with `String::from_utf8_lossy`, then apply `js_trim`. This is equivalent to TextDecoder streaming. One edge differs: V1 never flushes the decoder at EOF, so an incomplete multi-byte tail is dropped, where lossy decoding yields U+FFFD.

Leftover at EOF differs per function. Variants:
- **(L1) Gemini / Groq / AI-Studio-free**: `if buffer.trim().startsWith("data: ")`: `raw = buffer.trim().slice(6).trim()`; if `raw !== "[DONE]"`, try to parse and extract the token. Errors are ignored.
- **(L2) createVertexStream**: `if buffer.trim()`: `line = buffer.trim()`; if it starts with `"data: "`, `raw = ...trim()`; if `raw` is non-empty, try to parse. There is no `[DONE]` check; parsing `"[DONE]"` simply fails.
- **(L3) streamOpenAILike, streamVertex/streamFree relays, the client**: the leftover is **discarded**.

### 7.4 `streamGemini` (L34-120)
1. If `!payload.apiKey`, emit error `"API Key が設定されていません。CONFIGから設定してください。"` and return.
2. `model = payload.model?.trim() ?? ""`. If empty or the lowercased model starts with `"gpt"`, use **`"gemini-2.0-flash"`**. This differs from the non-stream default (`gemini-2.5-flash`).
3. POST to `https://generativelanguage.googleapis.com/v1beta/models/${model}:streamGenerateContent?alt=sse`. Headers: `x-goog-api-key`, `Content-Type: application/json`, with no `Accept`. Body: `buildGeminiBody(messages, Boolean(webSearch))`.
4. If the response is not ok or has no body: `json = readJson(response)` (null on failure). Emit error `` `${status} ${statusText}: ${JSON.stringify(json)}` `` with **every `\n` replaced by a space**. Call this format **E_S**. Return.
5. Loop over lines:
   - On a data line with `raw === "[DONE]"`, emit `complete(full)` and **return**. Gemini never sends `[DONE]`.
   - Otherwise parse it, ignoring errors. `token = extractGeminiResponse(parsed)`. If the token is truthy and not `"[Parse Error]"`: `full += token`; emit `token`.
6. At EOF apply leftover variant L1, then `break` and emit `complete(full)`.

### 7.5 `streamGroq` (L122-209)
Same structure as §7.4, with these differences:
- If `model` is empty or `!isValidGroqModel(model, false)`, use `"qwen3-32b"`. Compound model names are **not** allowed here.
- Body: if `payload.webSearch` is truthy, `buildGroqCompoundBody(messages, true)`. Otherwise `buildOpenAIBody(messages, model, true, true)`.
- POST to `https://api.groq.com/openai/v1/chat/completions`. Headers: `Authorization: Bearer`, `Content-Type: application/json`, `Accept: text/event-stream`.
- Token extraction: `extractOpenAIStreamToken`, accepted if truthy. `[DONE]` triggers complete and return.
- Leftover: variant L1 using `extractOpenAIStreamToken`.

### 7.6 Errors in streaming handlers
All upstream non-OK responses in §7.4, §7.5 and §7.7 use **E_S**: `"<status> <statusText>: <JSON or null>"` with LF replaced by space. Golden: `400 Bad Request: {"error":{"code":400,"message":"x\ny"}}`. The `\n` inside the JSON string stays an escaped backslash-n, because only raw LFs are replaced.

### 7.7 `streamOpenAILike` (L310-434): OpenAI (0), OpenRouter (6), Ollama (5)
Base headers: `Content-Type: application/json`, `Accept: text/event-stream`. Ollama gets them too.
- **OpenRouter**: apiKey required (else the API-key error event). If `model` is empty or the lowercased model starts with `"gpt"`, use **`"openai/gpt-4o"`**. This differs from the non-stream default. URL `https://openrouter.ai/api/v1/chat/completions`. Add `Authorization: Bearer`, `HTTP-Referer: https://dsuv.vercel.app`, `X-Title: RealAmadeus`.
- **OpenAI**: apiKey required. If `model` is empty, use `"gpt-4o"`. The URL follows the same OpenAI-compatible rule as §6.2.0. Add `Authorization: Bearer`.
- **Ollama**: host and model rules as in §6.2.5 (model default `"mistral"`). URL `${host}/api/chat`. No auth.
- Any other provider emits error `"Unsupported provider for streamOpenAILike"`. This is unreachable.
- Body: Ollama uses `{model, messages, stream: true}`. The others use `buildOpenAIBody(messages, model, true, false)`, which has no Groq extras.
- If the response is not ok or has no body, emit E_S.
- Loop over lines. Leftover variant **L3** applies: a final line without a trailing LF is dropped.
  - Ollama: for each **non-empty** trimmed line, try `JSON.parse(line)`. If `message.content` is truthy, `full += content` and emit `token`. The `done` flag is ignored; the stream ends when the body ends.
  - Others: data lines only. `raw === "[DONE]"` emits `complete(full)` and returns. Otherwise parse and `token = extractOpenAIStreamToken`; if truthy, emit it. OpenRouter keep-alive comments (`: OPENROUTER PROCESSING`) are ignored. Mid-stream `data: {"error":...}` objects yield no token and are ignored.
- At the end of the body, emit `complete(full)`.

### 7.8 `streamVertex` (L211-246), provider 4
- Consume `createVertexStream(payload)` (§8.9) and run the line reader over its output. For each data line with a non-empty trimmed `raw`, re-emit `data: ${raw}\n\n`. The bytes are identical because the inner stream already emits our event JSON.
- If an exception occurs, emit error `message` or `"Unknown streaming error"`.
- `useGCloudCLI` is **ignored** here. Auth is payload SA, then embedded SA, then raw key. gcloud is never used for provider 4 in streaming.
- Location defaults to `"global"` if empty. The client sends `"us-central1"`, which `getVertexTargetRegions` turns into a list that starts at `global` and **excludes us-central1** (§8.7).
- `vertexProject` is **required**. There is no fallback to the SA `project_id`, unlike §6.3 B1.

### 7.9 `streamFree` (L248-282), provider 8
- Same relay as §7.8, over `createFreeApiStream(payload)` (§8.13).
- On exception, emit `token(FALLBACK_COMM)` then `complete(FALLBACK_COMM)`.
- The official client never streams provider 8 (§2.3), so this path is only reachable by direct API callers. It must still be ported.

### 7.10 `nonStreamingFallback(payload, request, controller)` (L285-308): Claude (2), Local (7), unknown or missing provider
1. Build a **fake** request: `new Request("http://localhost/amadeus/app/api/chat", {method: "POST", headers: {"Content-Type": "application/json"}, body: JSON.stringify(payload)})`. It carries **no** `X-Forwarded-For`, so its IP is `"unknown"`.
2. `response ← await route.ts POST(fakeRequest)`, i.e. the complete §6.1 algorithm. Consequences:
   - **The rate limit runs a second time** under IP `"unknown"`. That bucket is shared by all such stream callers, so they get 20 per minute in total.
   - For a missing provider the **global counter increments twice**, once in the outer check and once in the inner one.
   - **`logInteraction` runs twice**: the inner `sendResponse` logs with IP `"unknown"`, and the outer interceptor logs with the real IP. `[Req #]` console lines appear twice.
3. If the response is not ok: `json ← readJson`; emit error `json?.error ?? "Request failed (${status})"`. This also covers the inner 429.
4. Otherwise `json ← readJson`; emit `token(json?.response ?? "")` then `complete(json?.response ?? "")`. One token carries the whole text. An empty text produces an empty token, which the client ignores, and an empty complete, which is not logged.
5. The `request` parameter is unused.

### 7.11 Event sequences cheat-sheet

| Case | Events |
|---|---|
| Normal stream (1, 3, 0, 5, 6) | `token`×N, then `complete(full)` |
| Upstream non-OK (1, 3, 0, 5, 6) | `error(E_S)` |
| fetch throws or body breaks (1, 3, 0, 5, 6) | `token`×N (maybe 0), then `error("fetch failed" or "terminated")` |
| Claude, local, unknown via fallback | `token(whole)` + `complete(whole)`, or `error(...)` |
| Vertex (4) | §8.9. Includes a **silent close with no complete and no error** quirk. |
| FreeAPI (8) | AI Studio `token`×N + `complete`; or the Vertex sequence with errors replaced by `token(FALLBACK)` + `complete(FALLBACK)`; or partial tokens then close with no complete (§8.13) |
| Rate-limited FreeAPI | `token(RATE_LIMITED)` + `complete(RATE_LIMITED)` |

---

## 8. `vertex-helpers.ts`

### 8.1 Module constants and state
- `VERTEX_REGIONS = ["global","us-west1","us-west4","us-east1","us-east4","us-east5","us-south1","asia-northeast1","northamerica-northeast1"]` (L14-24).
- `type VertexServiceAccount = { client_email; private_key; token_uri; project_id? }`.
- `embeddedCredentialPromise`: a per-process memoized promise. A `null` result is cached forever.
- `gcloudCachedToken = ""`, `gcloudTokenExpiry = 0` (ms epoch). This cache is **separate** from route.ts's and vertex-proxy's caches.
- `FREE_API_STUDIO_MODELS = ["gemini-3.5-flash-lite", "gemini-3.1-flash-lite"]` (L467). `FREE_API_GCP_MODEL = "gemini-3.1-flash-lite"` (L468).
- The FreeAPI Vertex project `"realamadeusfreecredit"` and location `"global"` are inline (L515-516, L637-638).
- `extractVertexStreamTokensLikeUnity` is imported but unused here.

### 8.2 `parseServiceAccount(raw)`, loose variant (L30-37)
1. If `raw.trim()` does not start with `{`, return null.
2. `JSON.parse(raw)` on the **untrimmed** string. JSON tolerates whitespace. A parse error returns null.
3. `client_email` and `private_key` must be strings, else null.
4. Return `{client_email (untrimmed), private_key, token_uri: p.token_uri || "https://oauth2.googleapis.com/token" (no trim; empty or missing uses the default), project_id: string ? p.project_id : ""}` (untrimmed).

### 8.3 JWT and token exchange (L39-60)
- `base64Url(b)`: standard base64, then `+`→`-` and `/`→`_`, with trailing `=` stripped (base64url without padding).
- `makeJwt(cred)`:
  - `now = floor(Date.now()/1000)`.
  - Header JSON `{"alg":"RS256","typ":"JWT"}`.
  - Claims JSON in this key order: `{"iss":client_email,"sub":client_email,"aud":token_uri,"scope":"https://www.googleapis.com/auth/cloud-platform","iat":now-30,"exp":now+3600}`.
  - `unsigned = b64u(header) + "." + b64u(claims)`.
  - Signature: RSA-SHA256 (PKCS#1 v1.5) over the UTF-8 `unsigned`, using the PEM key `private_key.replace(/\\n/g, "\n").trim()`. Literal backslash-n sequences become real newlines.
  - Result: `unsigned + "." + b64u(signature)`.
  - **Note: `exp - iat = 3630 s`** (see Open questions).
- `exchangeToken(cred)`:
  - POST `cred.token_uri` with `Content-Type: application/x-www-form-urlencoded`.
  - Body: `URLSearchParams({grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer", assertion})` → `grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion=<jwt>`. `reqwest .form()` produces identical bytes.
  - `data = await res.json()`. There is **no catch**: a non-JSON body throws SyntaxError.
  - If the response is not ok or has no `access_token`, throw `Error("Vertex token exchange: " + (data.error ?? "failed"))`.
  - Otherwise return `access_token`.
  - **Tokens are not cached.** Every request performs a fresh exchange.

### 8.4 `loadEmbeddedCredential()` (L62-67)
Memoized. Reads `join(process.cwd(), "realamadeusmain-7a9af55234f3.json")` as UTF-8 and runs `parseServiceAccount`. Any read error caches `null`.

### 8.5 gcloud CLI (L69-124)
- `findGcloudPath()`:
  1. Start the candidate list with `"/usr/local/bin/gcloud"`, `"/usr/bin/gcloud"`, `"/snap/bin/gcloud"`, `"/usr/lib/google-cloud-sdk/bin/gcloud"`.
  2. If `$HOME` is non-empty, append `"$HOME/google-cloud-sdk/bin/gcloud"`.
  3. For each `dir` in `$PATH.split(":")`, append `dir + "/gcloud"` unless it equals `"/gcloud"` (an empty dir) or is already listed.
  4. Return the first candidate for which `existsSync` is true. Any file type counts, including directories.
  5. Otherwise return `"gcloud"`.
- `runGcloudToken(path)`: `execFile(path, ["auth","print-access-token"], {timeout: 10000})`.
  - Any error (non-zero exit, ENOENT, timeout kill, maxBuffer of 1 MiB exceeded) resolves to `""`.
  - Otherwise `stdout.trim()`, kept only if it starts with `"ya29."`, else `""`.
- `getGcloudAccessToken()` (exported):
  - If the cached token is non-empty and `Date.now() < gcloudTokenExpiry`, return it.
  - Otherwise run `findGcloudPath` and `runGcloudToken`. If the token is non-empty, cache it with expiry `now + 50*60*1000` (50 min from **fetch time**). Return the token, possibly `""`. Failures are not cached.
- `clearGcloudTokenCache()` (exported): sets the token to `""` and expiry to 0.

### 8.6 `getVertexAccessToken(payloadApiKey?)` (L126-141), returns `{token} | {error}`
1. `credRaw = payloadApiKey?.trim() ?? ""`.
2. If `parseServiceAccount(credRaw)` succeeds: try `exchangeToken`, returning `{token}`. On failure return `{error: "Vertex token error: " + (e.message or "unknown")}`.
3. Else if `loadEmbeddedCredential()` succeeds: same as step 2.
4. Else if `credRaw` is non-empty, return `{token: credRaw}`. The raw API key is used as the "token".
5. Else return `{error: "Vertex AI: 認証情報がありません。CONFIGからサービスアカウントJSONキーを設定してください。"}`.

Malformed JSON keys that fail to parse are treated as **raw API keys** here. Contrast route.ts B, which returns 400.

### 8.7 `getVertexTargetRegions(preferredLocation)` (L143-157)
```
targets = []
preferred = preferredLocation.trim()
if preferred && preferred !== "us-central1": targets.push(preferred)
else if !preferred:                           targets.push("global")
// preferred === "us-central1" → nothing pushed
for r in VERTEX_REGIONS: if r !== "us-central1" && !targets.includes(r): targets.push(r)
```
**us-central1 is never attempted** by vertex-helpers. Golden results:
- `""` → `[global, us-west1, us-west4, us-east1, us-east4, us-east5, us-south1, asia-northeast1, northamerica-northeast1]`
- `"us-central1"` → same 9
- `"global"` → same 9
- `"asia-northeast1"` → `[asia-northeast1, global, us-west1, us-west4, us-east1, us-east4, us-east5, us-south1, northamerica-northeast1]`
- `"europe-west4"` → `[europe-west4, global, us-west1, us-west4, us-east1, us-east4, us-east5, us-south1, asia-northeast1, northamerica-northeast1]` (10 entries)
- `" us-west4 "` → `[us-west4, global, us-west1, us-east1, us-east4, us-east5, us-south1, asia-northeast1, northamerica-northeast1]`

### 8.8 Small helpers
- `isRetryableStatus(s) = s === 429 || s >= 500`.
- `resolveVertexModel(payload)`: `payload.model?.trim() ?? ""`. If empty or the lowercased model starts with `"gpt"`, use `"gemini-2.5-flash"`.

### 8.9 `createVertexStream(payload, encoder)` (L172-377): exact algorithm
```
webSearch ← Boolean(payload.webSearch); model ← resolveVertexModel(payload)
projectId ← payload.vertexProject?.trim() ?? ""; location ← payload.vertexLocation?.trim() || "global"
emit(ev)       = write "data: " + JSON(ev) + "\n\n"
emitError(msg) = payload.provider === 8 ? (emit token(FALLBACK_COMM); emit complete(FALLBACK_COMM)) : emit error(msg)

// 1. Credentials
if payload.provider === 8:
    token ← getGcloudAccessToken()
    token === "" → emitError("FreeAPI: gcloud 認証情報がありません。サーバーの設定を確認してください。"); close; return
else:
    r ← getVertexAccessToken(payload.apiKey); r.error → emitError(r.error); close; return
    token ← r.token
// 2. Project (checked AFTER credentials)
projectId === "" → emitError("Vertex AI: Project ID が未設定です。Config_VertexProject を設定してください。"); close; return
// 3. Headers
isApiKey ← !token.startsWith("ya29.")
headers ← {"Content-Type":"application/json"} + (isApiKey ? {"x-goog-api-key": token} : {"Authorization": "Bearer " + token})
console.log("Vertex Request: Using token starting with " + token.substring(0,15) + " (isApiKey: " + isApiKey + ")")
body ← JSON(buildGeminiBody(messages, webSearch))      // serialized once, reused for every attempt
regions ← getVertexTargetRegions(location); success ← false; errors ← []

for region in regions:
  if success: break
  attempt ← 0; response ← undefined                    // reset per region
  while attempt < 3 && !success:
    attempt++
    url ← buildVertexUrl(projectId, region, model, "streamGenerateContent")
    try: response ← POST url (headers, body)            // no cache option, no timeout
    catch e:
      errors.push(`${region} (attempt ${attempt}): ${e.message ?? "network error"}`)
      if attempt < 3: sleep(200 + Math.random()*500); continue   // float ms in [200, 700)
      else break
    if !response.ok:
      if status ∈ {401, 403}: clearGcloudTokenCache()   // the current token/headers are NOT refreshed
      if isRetryable(status):
        errors.push(`${region} (attempt ${attempt}): ${status}`)
        if attempt < 3: sleep(300 + floor(Math.random()*500)); continue   // int ms 300..=799
        else break
      break                                             // non-retryable: leave the retry loop
    success ← true; break                               // set BEFORE the body is read
  if response === undefined: continue                   // all attempts threw
  // NB: if attempt 1 got 503 and attempts 2-3 threw, `response` still holds the stale 503 Response
  if !response.ok:
    errText ← await response.text() (or "" on failure)
    errors.push(`Vertex AI Error (${region}): ${status}\n${errText}`)
    continue                                            // next region (any status, including 400/404)
  if !response.body: errors.push(`${region}: no response body`); continue   // success is already true, so the loop exits
  full ← ""; gotTokens ← false; buffer ← ""
  try:
    line loop (§7.3), on each data line with non-empty raw: parse (ignore errors); t ← extractGeminiResponse;
        if t && t !== "[Parse Error]": gotTokens ← true; full += t; emit token(t)
    on EOF: leftover variant L2, then break
  catch e:
    if gotTokens: emitError(`Vertex AI Stream Interrupted (${region}): ${e.message ?? "unknown"}`); close; return
    errors.push(`${region}: stream interrupted`); continue    // success is already true → loop exits → SILENT CLOSE
  emit complete(full); success ← true                    // a 200 with zero tokens gives complete("")
if !success: emitError(`Vertex AI Stream: All regions failed (${regions.join(", ")}). Errors: ${errors.join("; ")}`)
close
```
Quirks to preserve, or to decide on explicitly (§19):
- (a) A stream interruption before any token, and a missing body, both end with **no** `complete` and **no** `error`. The client reports an empty completion.
- (b) For provider 8 an interruption after tokens emits the fallback token and a `complete` whose `fullResponse` is **only** the fallback text. The client then shows `fullResponse`.
- (c) A 400 or 404 (bad model) is tried in all 9 regions, one attempt each. A 429 or 5xx gets 3 attempts per region, so up to 27 requests and about 14.4 s of sleep in the worst case.
- (d) The final retryable attempt adds two error entries: `... (attempt 3): 503` and `Vertex AI Error (...): 503\n<body>`.
- (e) The log line prints the first 15 characters of the token or API key.

### 8.10 `fetchVertexResponse(payload)` (L379-457), returns `{response} | {error, status}`
```
webSearch, model (resolveVertexModel), location ← vertexLocation?.trim() || "global", projectId ← vertexProject?.trim() ?? ""
token: provider === 8 → getGcloudAccessToken(); "" → {error: "FreeAPI: gcloud 認証情報がありません。サーバーの設定を確認してください。", status: 503}
       else → getVertexAccessToken(payload.apiKey); error → {error: r.error, status: 503}
projectId === "" → {error: "Vertex AI: Project ID が未設定です。Config_VertexProject を設定してください。", status: 400}
headers as in §8.9 (isApiKey rule), body serialized once, regions ← getVertexTargetRegions(location), errors ← []
for region in regions:                                   // ONE attempt per region
  url ← buildVertexUrl(projectId, region, model, "generateContent")
  try response ← POST (cache: no-store)
  catch e: errors.push(`${region}: ${e.message ?? "fetch failed"}`); continue          // no sleep
  if ok: data ← json or null; return {response: extractGeminiResponse(data)}           // may be "[Parse Error]"
  if retryable: errors.push(`${region}: ${status}`); sleep(100 + floor(random*300)); continue   // 100..=399 ms, even after the LAST region
  if status ∈ {401,403}: clearGcloudTokenCache()
  errBody ← text or ""; errors.push(`Vertex AI Error (${region}): ${status} ${statusText}\n${errBody}`); continue
return {error: `Vertex AI: All regions failed (${regions.join(", ")}). Errors: ${errors.join("; ")}`, status: 502}
```
There is no console log here, unlike the stream version.

### 8.11 `getAiStudioApiKey()` (L459-465)
```
key = process.env.AI_STUDIO_API_KEY || process.env.GEMINI_API_KEY
if key && key !== "undefined" && key.trim(): return key.trim()
return DEFAULT_AI_STUDIO_API_KEY
```
Subtleties:
- `AI_STUDIO_API_KEY` set to `"undefined"` or whitespace goes to the **default constant**. It does not fall back to `GEMINI_API_KEY`.
- Only an empty or unset `AI_STUDIO_API_KEY` falls through to `GEMINI_API_KEY`.
- Because the default is non-empty, the result is always truthy in V1.

### 8.12 `fetchFreeApiResponse(payload)` (L470-518), non-stream FreeAPI
```
aiStudioKey ← getAiStudioApiKey(); webSearch ← Boolean(payload.webSearch)
if aiStudioKey:
  for model in ["gemini-3.5-flash-lite", "gemini-3.1-flash-lite"]:
    try:
      POST https://generativelanguage.googleapis.com/v1beta/models/${model}:generateContent  (cache no-store)
           headers x-goog-api-key: aiStudioKey, Content-Type: application/json; body buildGeminiBody(messages, webSearch)
      if ok:
        data ← json or null; text ← extractGeminiResponse(data)
        if text && text !== "[Parse Error]": return {response: text}       // "" (empty text) counts as failure
        console.warn(`[FreeAPI] Google AI Studio (${model}) response parsing failed: ${JSON.stringify(data)}`)
      else:
        errorText ← text or ""
        console.warn(`[FreeAPI] Google AI Studio (${model}) returned ${status} (${statusText}): ${errorText}. Trying next fallback...`)
    catch err: console.warn(`[FreeAPI] Google AI Studio (${model}) fetch error: ${err.message ?? err}. Trying next fallback...`)
console.log("[FreeAPI] Falling back to GCP Vertex AI (gemini-3.1-flash-lite)...")
payload.model ← "gemini-3.1-flash-lite"; payload.vertexProject ← "realamadeusfreecredit"; payload.vertexLocation ← "global"
return fetchVertexResponse(payload)        // provider is 8, so gcloud token, regions [global, us-west1, ...]
```
There is no sleep between AI Studio models, and no retry.

### 8.13 `createFreeApiStream(payload, encoder)` (L520-673)
```
aiStudioKey ← getAiStudioApiKey(); webSearch ← Boolean(payload.webSearch)
emitFallbackMessage() = emit token(FALLBACK_COMM); emit complete(FALLBACK_COMM)
streamSuccess ← false; gotTokens ← false
if aiStudioKey:
  for model in FREE_API_STUDIO_MODELS:
    if streamSuccess || gotTokens: break
    try:
      POST https://generativelanguage.googleapis.com/v1beta/models/${model}:streamGenerateContent?alt=sse
           headers x-goog-api-key, Content-Type: application/json (no Accept); body buildGeminiBody(messages, webSearch)
      if response.ok && response.body:
        full ← ""; buffer ← ""
        line loop: data line raw === "[DONE]" → emit complete(full); streamSuccess ← true; close; RETURN
                   else parse → t ← extractGeminiResponse → if t && t !== "[Parse Error]": gotTokens ← true; full += t; emit token(t)
        EOF: leftover variant L1, then break
        if gotTokens: emit complete(full); streamSuccess ← true; close; RETURN
        // a 200 with zero tokens falls through to the next model, without logging
      else:
        errorText ← text or ""
        console.warn(`[FreeAPI] Google AI Studio (${model}) stream error (${status}): ${errorText}. Trying next fallback...`)
    catch err:
      console.warn(`[FreeAPI] Google AI Studio (${model}) stream exception: ${err.message ?? err}. Trying next fallback...`)
      // if tokens were already emitted, the next iteration breaks (gotTokens), Vertex is skipped,
      // and the stream closes with NO complete: partial answer, not logged
if !streamSuccess && !gotTokens:
  console.log("[FreeAPI] Using GCP Vertex AI stream fallback (gemini-3.1-flash-lite)...")
  payload.model ← "gemini-3.1-flash-lite"; payload.vertexProject ← "realamadeusfreecredit"; payload.vertexLocation ← "global"
  try: relay createVertexStream(payload) line by line (data lines with non-empty raw → "data: " + raw + "\n\n")
  catch err: console.error("[FreeAPI] GCP Vertex AI fallback stream error:", err); emitFallbackMessage()
close
```
If no AI Studio key is configured (possible in V2 if the default is removed), the code goes straight to the Vertex fallback.

---

## 9. `route.ts` private auth helpers (strict variant)

### 9.1 `parseServiceAccountCredential(raw)` (L63-86)
1. `trimmed = raw.trim()`. If it does not start with `{`, return null.
2. `JSON.parse(trimmed)`. An error returns null.
3. `client_email` and `private_key` must be strings.
4. Return `{client_email: trimmed, private_key (as is), token_uri: string && trim() non-empty ? trim() : "https://oauth2.googleapis.com/token", project_id: string ? trim() : ""}`.

### 9.2 Assertion and exchange (L88-140)
- `base64UrlEncode` and `buildServiceAccountAssertion` are byte-identical to §8.3: same header, claims, key order, `iat = now-30`, `exp = now+3600`, `\\n` normalization and trim.
- `exchangeServiceAccountToken(credential)`:
  - Same POST and form body, with `cache: "no-store"`.
  - The payload is parsed with `.json().catch(() => null)`.
  - If the response is not ok or has no `access_token`: `detail = error_description ?? error ?? "Token exchange failed"`; throw `Error("Vertex service account token exchange failed: " + detail)`.
  - Not cached.

### 9.3 `loadEmbeddedServiceAccountCredential()` (L142-149)
Constant `EMBEDDED_VERTEX_CREDENTIAL_FILE = "realamadeusmain-7a9af55234f3.json"`. The file is read from `process.cwd()` with the strict parser. Memoized separately from §8.4, so the file is read at most twice per process.

### 9.4 Duplicate gcloud helpers (L151-199)
`findGcloudPath`, `runGcloudToken` and `getGcloudAccessToken` are identical in logic to §8.5, with their own `gcloudCachedToken` and `gcloudTokenExpiry`. `clearGcloudTokenCache` does **not** clear this cache. A single shared cache in Rust is acceptable; the only effect is that 401/403 clears also apply to route.ts B2.

### 9.5 Differences between the three Vertex implementations

| Aspect | route.ts B (useGCloudCLI=false) | vertex-helpers (`fetchVertexResponse` / `createVertexStream`) | vertex-proxy |
|---|---|---|---|
| Auth order | payload SA → embedded SA → gcloud (only when the key is empty) → raw key with `?key=` and an AI Studio candidate | provider 8: gcloud only. Otherwise payload SA → embedded SA → raw key | gcloud only |
| Malformed JSON key | 400 | treated as raw key | n/a |
| Project ID | payload, or SA `project_id` (SA path) | payload only (required) | payload only (required) |
| Location default | `us-central1` | `global` | `us-central1` |
| Regions | 1 (no failover) | §8.7 list (no us-central1) | preferred, then `[us-central1, us-west1, us-east4, asia-northeast1, northamerica-northeast1]` |
| Retries per region | 0 | non-stream 0; stream up to 3 | 0 |
| Non-retryable error | return E_NS | record and try next region | return or emit immediately |
| Header choice | always Bearer (SA, gcloud) or `x-goog-api-key` (key path) | `ya29.` prefix gives Bearer, otherwise `x-goog-api-key` | Bearer |
| Stream token parsing | n/a | line-based `extractGeminiResponse` | chunk regex (§4.1) |

---

## 10. `rate-limit.ts`

### 10.1 `getClientIp(req)` (L4-17)
1. If the `x-forwarded-for` header is non-empty: split on `,` and trim each entry. If entry 0 is non-empty, return it. Later entries are never used. `", 1.2.3.4"` falls through.
2. Else if `x-real-ip` is non-empty, return its trimmed value.
3. Else return `"unknown"`.

The TCP peer address is **never** used. The value is client-spoofable if the proxy appends to XFF.

### 10.2 Constants and state
- `rateLimitMap: Map<string, number[]>` holds per-IP ms timestamps. It is never pruned.
- `RATE_LIMIT_WINDOW_MS = 60000`, `MAX_REQUESTS_PER_WINDOW = 20`.
- `globalRequestCount` (JS number). `MAX_GLOBAL_REQUESTS = 65000` (exported).
- `STATE_FILE = cwd/.amadeus_request_count`, `LOG_FILE = cwd/.amadeus_chat_log.jsonl`.

### 10.3 Load and save
- At module load: if the state file exists, read it as UTF-8 and `parseInt(data, 10)`. If the result is not NaN, it becomes the counter.
  - JS `parseInt` skips leading whitespace, accepts a sign, and reads digits until the first non-digit. So `"65000\n"` is 65000 and `"-5"` is -5. Use i64.
  - Errors print `console.error("Failed to load request count", e)`.
- `saveState()`: `fs.writeFileSync(STATE_FILE, String(count), "utf8")`. This is a synchronous, non-atomic truncate-and-write with **no trailing newline**. Errors print `console.error("Failed to save request count", e)`.
- The file is read **only at startup**. Edits made while the server runs are overwritten at the next increment.

### 10.4 `checkRateLimit(req, provider?)` (L49-95), returns `{allowed, remaining, errorMsg?, count}`
```
countsGlobal ← (provider === 8 || provider === undefined)
if countsGlobal && globalRequestCount >= 65000:
    return {allowed:false, remaining:0, errorMsg:"FreeAPIの全体リクエスト上限（65000回）に達しました。サービスを停止しています。", count: globalRequestCount}
ip ← getClientIp(req); now ← Date.now()
valid ← map[ip] (or []) filtered by (now - t < 60000)          // strict <
if valid.length >= 20:
    map[ip] ← valid                                            // denied attempts are NOT recorded
    return {allowed:false, remaining:0, errorMsg:"リクエストが多すぎます。1分ほど待ってから再度お試しください。", count: globalRequestCount}
valid.push(now); map[ip] ← valid
if countsGlobal: globalRequestCount++; saveState()
return {allowed:true, remaining: 20 - valid.length, count: globalRequestCount}
```
- The rate limit applies to **all providers**, including user-key providers. The global cap applies only to provider 8 or a missing provider.
- The counter increments at **admission**, before the upstream call. Failed upstream calls still count.
- A global-cap denial happens before the per-IP check and is not recorded in the window.
- `remaining` is returned but never used by callers.
- **Live state:** `.amadeus_request_count` currently contains `65000`, which equals the cap. **FreeAPI is effectively disabled on the live server**: every provider-8 request gets the rate-limited fallback.

### 10.5 `getGlobalRequestCount()`
Returns the in-memory counter. Used by quota.

### 10.6 `logInteraction(ip, provider, requestText, replyText, count)` (L101-110)
- `timestamp = new Date().toISOString()`. Format `YYYY-MM-DDTHH:MM:SS.mmmZ`, UTC, always 3-digit milliseconds. Golden: `2026-11-16T01:02:03.004Z`.
- The appended line is `JSON.stringify({timestamp, ip, provider, count, requestText, replyText}) + "\n"`, with **exactly that key order**, written synchronously with `appendFileSync`.
- After a successful append it prints `` `[Interaction Log] Count: ${count} | Provider: ${provider} | IP: ${ip} | Req: ${requestText.substring(0, 30)}... | Rep: ${replyText.substring(0, 30)}...` ``. `substring` counts UTF-16 code units, and the `...` is always appended.
- Any error prints `console.error("Failed to log interaction", e)`. A non-string `requestText` makes `substring` throw after the line was already written, and the error is caught.
- Callers:
  - `route.ts` `sendResponse`, with `provider = payload.provider ?? 0`.
  - The `stream/route.ts` interceptor, same provider rule.
  - Both log `question` = the last user message content.
  - `count` = the counter value at admission. For non-free providers that is the current, un-incremented global count.

---

## 11. `GET {base}/app/api/chat/quota` (`quota/route.ts`)
- `export const dynamic = "force-dynamic"`. Not cached.
- Body: `{"count": count, "max": 65000, "remaining": Math.max(0, 65000 - count)}`. All three are JSON numbers. Status 200, `Content-Type: application/json`.
- The handler sets `Cache-Control: no-store`, but the effective header is the config value (§1.3).
- There is no rate limit and no logging. The live value today is `{"count":65000,"max":65000,"remaining":0}`.
- Consumer: `RealAmadeusApp.tsx` `fetchFreeApiQuota()` calls `fetch(url, {cache:"no-store"})`. A non-OK response gives `null`. The JSON is accepted only if all three fields are numbers. It is fetched once whenever provider 8 is selected and the settings panel is visible. The UI formats it with `%1`, `%2` and `toLocaleString()`, and shows "exhausted" when `remaining === 0`.

---

## 12. `POST {base}/app/api/chat/vertex-proxy` (`vertex-proxy/route.ts`)
There is no first-party caller in the repo: no hit in `components/`, `lib/`, `hooks/`, `app/`, `scripts/`, or the Qt clone. It is still part of the V1 HTTP surface. The handler does **not** rate-limit, count or log.

Constants: `VERTEX_TOKEN_CACHE_MINUTES = 50`. `VERTEX_REGIONS = ["us-central1","us-west1","us-east4","asia-northeast1","northamerica-northeast1"]`. Its own token cache is `cachedVertexToken` / `vertexTokenExpiry`.

`getVertexTargetRegions(preferred)` (local version):
1. `p = preferred.trim()`. If non-empty, push it. us-central1 is allowed here.
2. Append each `VERTEX_REGIONS` entry not already present.

`getVertexAccessToken()`:
1. Cache check.
2. `findGcloudPath()` (same list as §8.5). There is a dead `if (!gcloudPath) return ""`.
3. `runGcloudAccessToken`: execFile with a 10 s timeout and `ya29.` filter.
4. Cache a successful token for 50 min.

`POST`:
```
payload ← parse; invalid → 400 {"error":"Invalid request payload"}
webSearch ← Boolean(payload.webSearch)
model ← payload.model?.trim() ?? ""; "" or lower startsWith "gpt" → "gemini-2.5-flash"
projectId ← payload.vertexProject?.trim() ?? ""; location ← payload.vertexLocation?.trim() || "us-central1"
isStream ← request.headers.get("x-vertex-stream") === "true"          // exact, case-sensitive value
token ← getVertexAccessToken()   // gcloud is run even if projectId is missing (the token check comes first)
token === "" → 503 {"error":"Vertex AI: アクセストークンの取得に失敗しました。\nサーバーに gcloud CLI がインストールされ、gcloud auth login 済みか確認してください。"}   (contains a real LF)
projectId === "" → 400 {"error":"Vertex AI: Project ID が未設定です。Config_VertexProject を設定してください。"}
body ← JSON(buildGeminiBody(messages, webSearch)); regions ← local getVertexTargetRegions(location)
isStream ? streamVertexResponse(...) : nonStreamVertexResponse(...)
```
`nonStreamVertexResponse` (L155-198):
```
for region: POST buildVertexUrl(.., "generateContent"), headers Authorization: Bearer <token>, Content-Type: application/json
   // NO try/catch: a network error gives an uncaught 500 with an empty body
   ok → 200 {"response": extractGeminiResponse(json or null)}
   retryable → errors.push(`${region}: ${status}`); sleep(100 + floor(random*300)); continue
   else → return status=upstream, {"error": `Vertex AI Error (${region}): ${status} ${statusText}\n${errorBody}`}     // fail immediately
→ 502 {"error": `Vertex AI: All regions failed (${regions.join(", ")}). Errors: ${errors.join("; ")}`}
```
`streamVertexResponse` (L200-310). The response uses the SSE headers from §1.3, and events have the §7.1 shapes.
```
for region (break if success):
  try POST buildVertexUrl(.., "streamGenerateContent") (Bearer)
  catch e → errors.push(`${region}: ${e.message ?? "fetch failed"}`); continue
  !ok: retryable → errors.push(`${region}: ${status}`); sleep(100..=399); continue
       else → errText ← text or ""; emit error(`Vertex AI Error (${region}): ${status}\n${errText}`); close; return   // no statusText
  no body → errors.push(`${region}: no response body`); continue
  full ← ""; gotTokens ← false
  try: for each network chunk: chunk ← decoder.decode(value, {stream:true});
         for tok in extractVertexStreamTokensLikeUnity(chunk): if tok: gotTokens ← true; full += tok; emit token(tok)
  catch e: gotTokens → emit error(`Vertex AI Stream Interrupted (${region}): ${e.message ?? "unknown"}`); close; return
           else errors.push(`${region}: stream interrupted`); continue      // success is still false here, so the next region is tried
  emit complete(full); success ← true
!success → emit error(`Vertex AI Stream: All regions failed (${regions.join(", ")}). Errors: ${errors.join("; ")}`)
close
```

---

## 13. Persistent files, credentials, environment

### 13.1 `.amadeus_request_count` (repo root = Next `process.cwd()`)
- Format: an ASCII decimal integer with no newline, e.g. `65000` (5 bytes).
- Writers: `rate-limit.ts` `saveState()` only. Readers: `rate-limit.ts` module load only.
- **Tracked in git**: `git ls-files` lists it, and `git status` shows it modified.

### 13.2 `.amadeus_chat_log.jsonl`
- One JSON object per line, written by `logInteraction` only. The application has no reader.
- Schema, with this exact key order (verified on all 65,381 lines; 0 malformed):
  ```
  {"timestamp": "<ISO-8601 ms Z, 24 chars>", "ip": "<string>", "provider": <int>, "count": <int>, "requestText": "<string>", "replyText": "<string>"}
  ```
- Observed aggregates (no PII):
  - Size 46.8 MB, 65,381 lines.
  - Providers: 8 → 65,078; 1 → 236; 3 → 26; 6 → 20; 0 → 13; 2 → 8. Providers 4, 5 and 7 never appear.
  - `count` ranges 1..65000; 176 lines have `count = 65000`.
  - IPs: 37,748 IPv6 and 27,633 IPv4. None are `"unknown"`.
- **Tracked in git.** It contains full user messages, replies and IPs, which is a privacy concern.

### 13.3 `.amadeus_achievements.json` (orphan)
- 904 bytes, tracked in git.
- **No code reads or writes it**, in the current tree or in git history (`git log -S` finds nothing).
- Its shape is exactly the client localStorage value under key `amadeus_achievements` (`lib/amadeus/services/achievement-manager.ts` `STORAGE_KEY`): `{"unlocked": string[], "dates": {<achievementId>: string}, "seenEmotions": string[], "lastLaunchDate": string, "launchStreak": number}`.
- Likely a manual export. **No server port is needed.** The UI/achievements subsystem owns the format.

### 13.4 `realamadeusmain-7a9af55234f3.json` (embedded Vertex service-account credential)
- Repo root, 2,363 bytes. Ignored by git through the `.gitignore` pattern `realamadeusmain-*.json`, so it is not tracked.
- Readers: `route.ts` (`EMBEDDED_VERTEX_CREDENTIAL_FILE`, strict parser) and `vertex-helpers.ts` (`loadEmbeddedCredential`, loose parser). Both read from `process.cwd()`. No writers.
- Structure. Values are **redacted** here and must never be printed or logged.

| key | type | notes |
|---|---|---|
| `type` | string | `"service_account"` |
| `project_id` | string | `"realamadeusmain"` |
| `private_key_id` | string | 40 hex chars, secret-adjacent |
| `private_key` | string | PKCS#8 PEM (`-----BEGIN PRIVATE KEY-----`), about 1,708 chars with `\n` escapes. **SECRET** |
| `client_email` | string | `<project-number>-compute@developer.gserviceaccount.com` (default compute SA) |
| `client_id` | string | 21 digits |
| `auth_uri` | string | `https://accounts.google.com/o/oauth2/auth` |
| `token_uri` | string | `https://oauth2.googleapis.com/token` |
| `auth_provider_x509_cert_url` | string | `https://www.googleapis.com/oauth2/v1/certs` |
| `client_x509_cert_url` | string | `https://www.googleapis.com/robot/v1/metadata/x509/<urlencoded client_email>` |
| `universe_domain` | string | `"googleapis.com"` |

Only `client_email`, `private_key`, `token_uri` and `project_id` are used.

### 13.5 Environment variables
- `.env` (root, untracked via `.env*`). Next loads it at startup; real environment variables take precedence. Keys (values never printed):
  - `CANOPYWAVEAPI`. Comment: "CanopyWave API Key for Amadeus Kurisu Chat". Used **only** by `app/api/atchannel/chat/route.ts` (Appendix A). Not used by any `/amadeus` route.
  - `AI_STUDIO_API_KEY`. Comment: "Google AI Studio API Key for FreeAPI". Used by `getAiStudioApiKey()`.
- `GEMINI_API_KEY`: a fallback in `getAiStudioApiKey()`. Not present in `.env`.
- `HOME` and `PATH`: gcloud discovery (§8.5).
- Proxy-script-only variables: `LOCAL_PROXY_HOST`, `LOCAL_PROXY_PORT`, `VERTEX_PROJECT`, `VERTEX_LOCATION` (§17).
- Rust: use `dotenvy::dotenv()`, which does not override existing variables. That matches Next precedence for a single `.env`.

### 13.6 Git-tracking summary

| File | Tracked? |
|---|---|
| `.amadeus_request_count` | yes |
| `.amadeus_chat_log.jsonl` | yes |
| `.amadeus_achievements.json` | yes |
| `realamadeusmain-*.json` | no (gitignored) |
| `.env` | no (gitignored) |

---

## 14. Timeouts, retries, failover (summary)

| Item | V1 value |
|---|---|
| Explicit HTTP timeouts | **none**. undici defaults apply: connect 10 s, headers 300 s, body idle 300 s. Timeouts surface as `"fetch failed"` (connect/headers) or `"terminated"` (body). |
| Redirects | followed automatically (fetch default). A 301, 302 or 303 on POST becomes GET, which matches reqwest. |
| gcloud CLI | `execFile` timeout 10,000 ms. The token is cached 50 min from fetch. Three independent caches: route.ts, vertex-helpers, vertex-proxy. |
| SA token | exchanged on every request (no cache) |
| createVertexStream | regions from §8.7. Up to 3 attempts per region for network errors (sleep 200–700 ms, float) and 429/5xx (sleep 300–799 ms). Non-retryable statuses move to the next region. |
| fetchVertexResponse | 1 attempt per region. Retryable: sleep 100–399 ms, then next region. Others: next region immediately. |
| route.ts Vertex B1/B2 | single request, no retry |
| route.ts Vertex B3 | up to 2 candidate URLs, sequential |
| FreeAPI | AI Studio `gemini-3.5-flash-lite`, then `gemini-3.1-flash-lite` (1 attempt each, no sleep), then Vertex `gemini-3.1-flash-lite` / `realamadeusfreecredit` / `global` with the failover above |
| vertex-proxy | 1 attempt per region. Retryable: sleep 100–399 ms. Non-retryable: fail immediately. |
| Rate limit | sliding window of 60 s, 20 requests per IP. Global cap 65,000 for provider 8 or missing. |
| Client-side | no timeout. Streaming can be cancelled with AbortController. |

Rust recommendation: `reqwest::Client` with `connect_timeout(10s)`, `read_timeout(300s)` (per-read idle, like undici's body timeout) and no total timeout. Use one client per process. Sleeps use `rand` with the exact ranges above.

---

## 15. Verbatim string catalogue

### 15.1 User-facing assistant texts. Exact characters: `……` is two U+2026, `？` is U+FF1F, and `、` / `。` are U+3001 / U+3002.
- `FREE_FALLBACK_RATE_LIMITED` = `[SAD] ごめんなさい、リクエストが多すぎるみたい……。少し休ませてからまた話しかけてね。`
- `FREE_FALLBACK_COMM_ERROR` = `[SAD] ごめんなさい、ちょっと通信の調子が悪いみたい……。少し待ってからもう一度試してみてくれる？`
- `[Parse Error]` (returned as response text)

### 15.2 Error messages

| String | Where | HTTP / channel |
|---|---|---|
| `Invalid request payload` | all POST handlers | 400 JSON |
| `Too many requests. Please wait a minute.` | `/chat`, `/stream` default when `errorMsg` is unset (unreachable) | 429 |
| `FreeAPIの全体リクエスト上限（65000回）に達しました。サービスを停止しています。` (full-width parentheses U+FF08/U+FF09) | rate limiter, global cap | 429 (non-free callers) |
| `リクエストが多すぎます。1分ほど待ってから再度お試しください。` | rate limiter, per IP | 429 / stream fallback error event |
| `API Key が設定されていません。CONFIGから設定してください。` | providers 0, 1, 2, 3, 6 (`/chat`); on `/stream`, 0, 1, 3, 6 directly and 2 through `nonStreamingFallback` | 400 / error event |
| `Vertex AI: サービスアカウントJSONキーの形式が不正です。` | route.ts B | 400 |
| `Vertex AI: サービスアカウントJSON利用時は Project ID が必要です。` | route.ts B1 | 400 |
| `Vertex AI: gcloud認証利用時は Project ID が必要です。` | route.ts B2 | 400 |
| `Vertex AI: 認証情報が見つかりません。APIキーを設定するか、サーバーの gcloud auth login を実行してください。` | route.ts B2 | 400 |
| `Vertex service account token exchange failed: <error_description \| error \| "Token exchange failed">` | route.ts B1 (thrown) | 500 |
| `Vertex token exchange: <error \| "failed">` wrapped as `Vertex token error: <msg \| "unknown">` | vertex-helpers | 503 (non-stream) / error event |
| `Vertex AI: 認証情報がありません。CONFIGからサービスアカウントJSONキーを設定してください。` | getVertexAccessToken | 503 / error event |
| `FreeAPI: gcloud 認証情報がありません。サーバーの設定を確認してください。` | provider 8 Vertex fallback | 503 internally, then FreeAPI fallback text |
| `Vertex AI: Project ID が未設定です。Config_VertexProject を設定してください。` | vertex-helpers, vertex-proxy | 400 / error event |
| `Vertex AI: All regions failed (<r1, r2, ...>). Errors: <e1; e2; ...>` | fetchVertexResponse, vertex-proxy | 502 |
| `Vertex AI Stream: All regions failed (<regions>). Errors: <errors>` | createVertexStream, vertex-proxy stream | error event |
| `Vertex AI Stream Interrupted (<region>): <msg \| "unknown">` | stream paths | error event |
| `Vertex AI Error (<region>): <status>\n<body>` | createVertexStream errors[]; vertex-proxy stream (immediate) | inside the aggregate / error event |
| `Vertex AI Error (<region>): <status> <statusText>\n<body>` | fetchVertexResponse errors[]; vertex-proxy non-stream (immediate, upstream status) | inside the aggregate / JSON |
| `<region> (attempt <n>): <msg \| "network error">`, `<region> (attempt <n>): <status>` | createVertexStream errors[] | aggregate |
| `<region>: <status>`, `<region>: <msg \| "fetch failed">`, `<region>: no response body`, `<region>: stream interrupted` | various errors[] | aggregate |
| `Vertex AI: アクセストークンの取得に失敗しました。` + LF + `サーバーに gcloud CLI がインストールされ、gcloud auth login 済みか確認してください。` | vertex-proxy | 503 |
| `Unknown provider.` | `/chat` default branch | 400 |
| `Unknown server error` | `/chat` catch for non-Error throws | 500 |
| `Unknown streaming error` | `/stream` catch, streamVertex catch | error event |
| `Unsupported provider for streamOpenAILike` | unreachable | error event |
| `Request failed (<status>)` | nonStreamingFallback (and the client) | error event |
| `fetch failed`, `terminated`, `Failed to parse URL from <url>` | undici error messages surfaced as-is | 500 / error event |
| E_NS `<status> <statusText>\n<json>` | `/chat` upstream errors | upstream status |
| E_S `<status> <statusText>: <json>` (LF replaced by space) | `/stream` upstream errors | error event |
| Ollama and local: `body.error` or `<status> <statusText>` | `/chat` | upstream status |

### 15.3 Console log lines (operators may grep these). stdout = `console.log`; stderr = `console.warn` / `console.error`.
- `\n[Req #<count>] Q: <question>` (leading blank line)
- `[Req #<count>] A: <text>`
- `[Interaction Log] Count: <count> | Provider: <provider> | IP: <ip> | Req: <first 30 UTF-16 units>... | Rep: <first 30>...`
- `Failed to log interaction` + error, `Failed to load request count` + error, `Failed to save request count` + error
- `Vertex Request: Using token starting with <first 15 chars> (isApiKey: <true|false>)`. **V2: mask this.** See Open questions.
- `[FreeAPI] Google AI Studio (<model>) response parsing failed: <JSON>`
- `[FreeAPI] Google AI Studio (<model>) returned <status> (<statusText>): <body>. Trying next fallback...`
- `[FreeAPI] Google AI Studio (<model>) fetch error: <msg>. Trying next fallback...`
- `[FreeAPI] Falling back to GCP Vertex AI (gemini-3.1-flash-lite)...`
- `[FreeAPI] Google AI Studio (<model>) stream error (<status>): <body>. Trying next fallback...`
- `[FreeAPI] Google AI Studio (<model>) stream exception: <msg>. Trying next fallback...`
- `[FreeAPI] Using GCP Vertex AI stream fallback (gemini-3.1-flash-lite)...`
- `[FreeAPI] GCP Vertex AI fallback stream error:` + error

### 15.4 Client mapping that depends on server strings (`RealAmadeusApp.tsx` `handleApiError`, L1653-1690). Owned by the UI spec, listed here because server text must stay compatible.
Every error string from `errorOccurred` is checked **in this order**. The first match wins. The emotion is then set to `ANGRY`.

| Condition (substring) | Message shown |
|---|---|
| `API Key` \| `API key` \| `設定されていません` | `……APIキーが設定されてないみたいよ。CONFIGから設定してちょうだい。` |
| `401` \| `Unauthorized` | `APIキーが無効みたい……もう一度確認して設定し直してくれる？` |
| `429` \| `rate limit` \| `RESOURCE_EXHAUSTED` | `リクエストが多すぎるみたい。少し待ってからもう一度試してくれない？` |
| `insufficient_quota` \| `quota_project` | `Geminiの利用枠か課金設定に問題があるみたい。APIキーとプロジェクト設定を確認して。` |
| `timeout` \| `Timeout` | `応答がタイムアウトしたわ……ネットワークの状態を確認してみて。` |
| `Cannot connect` \| `Network` \| `ネットワーク` | `ネットワークに接続できないわ。インターネット接続を確認してちょうだい。` |
| `403` \| `Forbidden` | `このAPIへのアクセスが拒否されたわ。権限を確認してみて。` |
| `500` \| `502` \| `503` | `サーバー側でエラーが起きてるみたい。しばらくしてからもう一度試して。` |
| `model` \| `Model` | `指定されたモデルが見つからないみたい。CONFIGからモデル名を確認して。` |
| `JSON` | `サービスアカウントJSONキーの形式が正しいか確認して。` |
| `Vertex` \| `アクセストークン` | `Vertexの認証情報を確認して。APIキーかサービスアカウントJSONキーが必要よ。` |
| (none of the above) | `何かエラーが起きたみたい……もう一度試してくれる？` |

So status numbers and reason phrases in E_NS and E_S, and the absence of extra words such as reqwest's URL-bearing messages, change what the user sees.

---

## 16. Consumer contract (`lib/amadeus/services/ai-service.ts`)

The UI port (amadeus-ui `Transport`) must reproduce this.
- `sendChat(messages)` sends `POST /amadeus/app/api/chat` with header `Content-Type: application/json` and body `buildPayload(messages)` (§2.2).
  - It reads `data = json or null`.
  - If the response is not ok: `errorOccurred(data?.error ?? "Request failed (" + status + ")")`.
  - Otherwise `responseReceived(data?.response ?? "")`.
  - On exception: `errorOccurred(error.message or "Unknown error")`.
- `sendChatStreaming(messages)`:
  - First `cancelStreaming()` (aborts the previous controller), then create a new AbortController.
  - `POST /amadeus/app/api/chat/stream` with the same headers and body plus `signal`.
  - If the response is not ok or has no body: parse JSON and `errorOccurred(data?.error ?? "Request failed (" + status + ")")`.
  - Otherwise read the body with the line reader of §7.3 (leftover discarded). For each data line with a non-empty raw:
    - `token` with a truthy token: `full += token`, `streamToken(token)`.
    - `complete`: `streamComplete(fullResponse ?? full)`, clear the controller, **return**.
    - `error`: `errorOccurred(error ?? "Streaming error")`, clear, return.
    - Unknown `type`: ignored.
    - **A JSON parse failure treats the raw text as a token** (`full += raw; streamToken(raw)`).
  - At EOF without a terminal event: `streamComplete(full)`.
  - On exception: unless aborted, `errorOccurred(message or "Unknown streaming error")`.
- Streaming is chosen for providers 1, 3, 4, 5 and 6 (`Config_ApiProvider` via `getInt(.., 0)`). Every other provider uses `sendChat`.
- The quota consumer is described in §11.

---

## 17. Local proxy contract (`scripts/amadeus-local-proxy/server.mjs`, target of provider 7)
This is a separate Node script (`npm run start:amadeus-proxy`). Listed here only so provider 7 can be tested.
- Bind: `LOCAL_PROXY_HOST` (default `0.0.0.0`), `LOCAL_PROXY_PORT` (default `5103`).
- CORS is open (`Access-Control-Allow-Origin: *`, Methods `GET,POST,OPTIONS`, Headers `Content-Type, Authorization`). `OPTIONS` returns 204.
- `GET /health` returns `{available:true, mode:"gcloud-vertex-proxy", bindHost, bindPort, project: VERTEX_PROJECT||"", location: VERTEX_LOCATION||"us-central1"}`.
- `POST /chat`:
  - Body limit 2,000,000 characters (`Payload too large`). `Invalid JSON` on a parse error. Both are reported as 500.
  - Empty `messages` returns 400 `{"error":"messages is required"}`.
  - Project: `vertexProject` or env. If missing, 400 `{"error":"vertexProject is required (or set VERTEX_PROJECT env)."}`.
  - Location: payload, env, or `us-central1`. Model: `gemini-2.5-flash` if empty or starts with `gpt`.
  - Calls Vertex `generateContent` through regions `[preferred, us-central1, us-west1, us-east4, asia-northeast1, northamerica-northeast1]`, with no sleep. Retryable statuses move on; others throw.
  - Success: `200 {"response": text}`. Errors: `500 {"error": msg}`.
- gcloud discovery includes Windows `gcloud.cmd` paths run through `cmd.exe /d /s /c`. Reuse these candidates for the V2 desktop gcloud discovery (§Rust mapping).
- Not found: 404 `{"error":"Not found"}`.

---

## 18. Appendix A: CanopyWave (`POST /api/atchannel/chat`). Not RealAmadeus.
- Used by the separate `app/atchannel` page (an @channel BBS toy). It persists `localStorage['atchannel_posts']`.
- Upstream: `https://inference.canopywave.io/v1/chat/completions`, model `moonshotai/kimi-k2.6`, `Authorization: Bearer ${process.env.CANOPYWAVEAPI}`.
- Body `{model, messages, temperature (default 0.85), max_tokens (default 4000)}`, taken from the request body.
- If the key is missing or equals `your-api-key-here`, it returns 500 `{"error":"APIキーが設定されていません。.envファイルのCANOPYWAVEAPI=に有効なキーを設定してください。"}`.
- Upstream errors become `{"error": "CanopyWave API error (<status>): <text>"}` with the upstream status. Success returns the upstream JSON verbatim. Exceptions return 500 `{"error": "サーバーエラー: <msg|不明なエラー>"}`.
- **Recommendation:** exclude from RealAmadeus V2. Port it only if the orchestrator decides the whole dsuv site moves to Rust.

---

## 19. Quirk and edge-case register (V1 behavior, with a V2 recommendation)

"Keep" means reproduce exactly, because it is observable through the API or UI. "Internal OK" means the change is not observable.

| # | V1 behavior | Recommendation |
|---|---|---|
| Q1 | Default models differ by path. Gemini: `/chat` 2.5-flash vs `/stream` 2.0-flash. OpenRouter: `/chat` `google/gemini-2.5-flash` vs `/stream` `openai/gpt-4o`. Gemini/Claude non-stream `gpt` check is case-sensitive. | Keep. Put the defaults in a table in amadeus-core `providers.rs`. |
| Q2 | `useGCloudCLI=true` never invokes gcloud for provider 4. The streaming path ignores the flag. | Keep, and document it in the UI help. |
| Q3 | vertex-helpers never uses `us-central1`, and `global` is tried first. | Keep. |
| Q4 | The embedded service account beats a user's plain API key (route.ts B and getVertexAccessToken). | Keep on the server. **Never ship the credential in desktop or web bundles.** |
| Q5 | createVertexStream closes silently when the stream breaks before the first token or the body is missing. | Keep (the client shows an empty reply). Mark with `// V1 quirk Q5`. |
| Q6 | nonStreamingFallback re-enters `/chat`: second rate limit under IP `unknown`, possible double global increment, double log. | Recommended: call the provider dispatcher directly, skip the second rate limit and log (the outer interceptor still logs once). No first-party client path triggers this. Requires orchestrator sign-off; see Open questions. |
| Q7 | streamOpenAILike drops a final line without a trailing LF. Gemini and Groq process it. | Keep. |
| Q8 | vertex-proxy streaming parses arbitrary chunks with a regex (no `\u` decoding, split tokens, sequential-unescape bug). | Keep (faithful) unless the endpoint is dropped. |
| Q9 | FreeAPI AI Studio stream error after tokens: partial text, no `complete`, not logged. FreeAPI Vertex interruption after tokens: fallback appended and `complete` = fallback only. | Keep. |
| Q10 | `"[Parse Error]"` is delivered as an assistant reply, and the FreeAPI Vertex fallback can return it. | Keep. |
| Q11 | Groq model name filter (§3.1) replaces valid models with `qwen3-32b`. | Keep. |
| Q12 | The `/chat` rate-limit fallback for provider 8 is not logged. Exception fallbacks are not logged. FreeAPI comm-error fallbacks from `fetchFreeApiResponse` **are** logged. | Keep. |
| Q13 | `count` in logs: for non-free providers it is the current global value, not a per-provider count. | Keep. |
| Q14 | SA tokens are never cached, so every Vertex SA request performs a token exchange. | Internal OK: cache for 55 min, keyed by `client_email`. |
| Q15 | Three separate gcloud caches. | Internal OK: one shared cache. |
| Q16 | The rate-limit map is never pruned. | Internal OK: prune empty or stale entries periodically. |
| Q17 | The counter file write is non-atomic. | Internal OK: write a temp file and rename. Keep the format. |
| Q18 | The per-IP window uses wall-clock `Date.now()`. | Internal OK: monotonic `Instant`. |
| Q19 | Malformed field types trigger uncaught TypeErrors, giving 500 with an empty body. | Accept either: replicate 500 with an empty body, or use lenient coercion (non-string treated as absent). The official client never sends these. |
| Q20 | Upstream statuses are passed through verbatim (e.g. 529). A 304 would crash the `Response` constructor (500). | Keep pass-through. Map 1xx, 204 and 304 to 500 with message `Response constructor: Invalid response status code <n>` if exactness matters. |
| Q21 | The client cancels a stream mid-way, but V1 keeps reading upstream until the next enqueue throws. | Internal OK: cancel upstream when the client disconnects, and do not log. |
| Q22 | JSON re-serialization of upstream error bodies (key order, number formatting). | Acceptable difference. |
| Q23 | The JWT has `exp - iat = 3630 s` (see Open questions). | Test against Google. If rejected, use `iat = now`, `exp = now + 3600`. |

---

## Rust mapping

### A. Crate placement

**amadeus-core** (wasm32 and native, no I/O). Module `protocol::chat`:
```rust
pub const PROVIDER_OPENAI: i64 = 0; pub const PROVIDER_GEMINI: i64 = 1; pub const PROVIDER_CLAUDE: i64 = 2;
pub const PROVIDER_GROQ: i64 = 3;   pub const PROVIDER_VERTEX: i64 = 4; pub const PROVIDER_OLLAMA: i64 = 5;
pub const PROVIDER_OPENROUTER: i64 = 6; pub const PROVIDER_LOCAL: i64 = 7; pub const PROVIDER_FREE: i64 = 8;
pub const PROVIDER_NAMES: [&str; 9] = ["OpenAI","Google Gemini","Anthropic Claude","Groq","Vertex AI","Ollama","OpenRouter","","FreeAPI"];
pub const STREAMING_PROVIDERS: [i64; 5] = [1, 3, 4, 5, 6];           // client decision table

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage { pub role: String, pub content: String }     // role kept as String for verbatim pass-through

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ChatRequestPayload {                                       // CLIENT-side builder; field order = V1 key order
    pub messages: Vec<ChatMessage>,
    pub provider: i64,
    #[serde(rename = "apiKey")] pub api_key: String,
    pub model: String,
    #[serde(rename = "webSearch")] pub web_search: bool,
    #[serde(rename = "vertexProject")] pub vertex_project: String,
    #[serde(rename = "vertexLocation")] pub vertex_location: String,
    #[serde(rename = "localProxyHost")] pub local_proxy_host: String,
    #[serde(rename = "localProxyPort")] pub local_proxy_port: String,
    #[serde(rename = "useGCloudCLI")] pub use_gcloud_cli: bool,       // NB: rename_all=camelCase would give "useGcloudCli": wrong
    #[serde(rename = "ollamaHost")] pub ollama_host: String,
    #[serde(rename = "openaiCompatible")] pub openai_compatible: bool,
    #[serde(rename = "openaiBaseUrl")] pub openai_base_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]                      // the tag is serialized first → {"type":"token","token":..}
pub enum StreamEvent {
    Token { token: String },
    Complete { #[serde(rename = "fullResponse")] full_response: String },
    Error { error: String },
}
#[derive(Serialize, Deserialize)] pub struct ChatResponseBody { pub response: String }
#[derive(Serialize, Deserialize)] pub struct ErrorBody { pub error: String }
#[derive(Serialize, Deserialize, Clone, Copy)] pub struct QuotaInfo { pub count: i64, pub max: i64, pub remaining: i64 }

pub mod texts {   // verbatim strings from §15.1/§15.2 as consts
    pub const FREE_FALLBACK_RATE_LIMITED: &str = "[SAD] ごめんなさい、リクエストが多すぎるみたい……。少し休ませてからまた話しかけてね。";
    pub const FREE_FALLBACK_COMM_ERROR: &str = "[SAD] ごめんなさい、ちょっと通信の調子が悪いみたい……。少し待ってからもう一度試してみてくれる？";
    pub const PARSE_ERROR: &str = "[Parse Error]";
    pub const ERR_INVALID_PAYLOAD: &str = "Invalid request payload";
    pub const ERR_NO_API_KEY: &str = "API Key が設定されていません。CONFIGから設定してください。";
    // ... every string in §15.2
}
```
Also in core: `providers::defaults`, a table of default and replacement rules per provider and path (Q1), so the UI can show the effective model.

**amadeus-backend** (native: tokio, reqwest with rustls, `default-features = false`):
```
amadeus_backend::
  config.rs        BackendConfig { data_dir, base_ai_studio_key: Option<String>, embedded_vertex_credential: Option<PathBuf>,
                                   free_api: FreeApiMode, rate_limit: Option<RateLimitConfig>, chat_log: Option<PathBuf>,
                                   enable_vertex_proxy: bool }
                   FreeApiMode::{ Direct { studio_models: Vec<String> = ["gemini-3.5-flash-lite","gemini-3.1-flash-lite"],
                                           gcp_model: "gemini-3.1-flash-lite", gcp_project: "realamadeusfreecredit", gcp_location: "global" },
                                  RemoteRelay { chat_url, stream_url, quota_url } }      // desktop: relay to the official server
  jsx.rs           js_trim, js_truthy(&Value), js_parse_int(&str)->Option<i64>, encode_uri_component, js_substring_utf16(s, n),
                   iso_timestamp_ms() -> "YYYY-MM-DDTHH:MM:SS.mmmZ"
  request.rs       ServerChatRequest { raw: serde_json::Map<String, Value> }   // lenient, preserve_order
                     fn messages(&self) -> Option<&Vec<Value>>; fn provider(&self) -> ProviderVal; fn str_opt(&self, k) -> Result<Option<&str>, TypeErr>
                     fn truthy(&self, k) -> bool; fn set_str(&mut self, k, v)   // FreeAPI mutation
  http.rs          fn build_client() -> reqwest::Client; fn reason(status) -> &'static str (canonical_reason or "");
                   fn send_err_msg(e: &reqwest::Error) -> String { if e.is_builder() { format!("Failed to parse URL from {url}") } else { "fetch failed".into() } }
                   const BODY_ERR_MSG: &str = "terminated";
  llm/bodies.rs    build_openai_body(messages:&[Value], model, stream, is_groq) -> Value; build_groq_compound_body; build_gemini_body(messages, grounding);
                   build_claude_body; build_ollama_body(model, messages, stream); is_valid_groq_model(model, allow_compound)
  llm/extract.rs   extract_openai_response, extract_gemini_response, extract_claude_response, extract_openai_stream_token,
                   extract_vertex_stream_tokens_like_unity (regex compiled once via std::sync::LazyLock)
  llm/sse.rs       LineBuffer { buf: Vec<u8> } -> push(&[u8]) -> Vec<String>; leftover() -> String; fn data_payload(line) -> Option<&str>
                   fn encode_event(&StreamEvent) -> Bytes   // b"data: " + serde_json::to_vec + b"\n\n"
  llm/errors.rs    ApiError { status: u16, message: String }; fn e_ns(status, body: Option<Value>) -> String; fn e_s(...) -> String
  google/sa.rs     ServiceAccount { client_email, private_key, token_uri, project_id }; parse_strict(raw) (route.ts), parse_loose(raw) (helpers);
                   build_assertion(&sa, now_secs) -> Result<String, String>   // manual RS256: rsa::pkcs1v15::SigningKey<Sha256>, pkcs8 PEM
                   async exchange_strict(&http, &sa) -> Result<String, String>  // "Vertex service account token exchange failed: ..."
                   async exchange_loose(&http, &sa) -> Result<String, String>   // "Vertex token exchange: ..."
                   EmbeddedCredential: tokio::sync::OnceCell<Option<ServiceAccount>> ×2 (strict / loose) or one raw OnceCell<Option<String>>
  google/gcloud.rs fn gcloud_candidates() -> Vec<PathBuf>   // per-OS, see D
                   async fn print_access_token(path) -> String  // tokio::process::Command, kill_on_drop, timeout 10 s, "ya29." filter
                   GcloudTokenCache { inner: tokio::sync::Mutex<(String, Option<Instant>)> }: get(), clear(); TTL 50 min
  vertex/regions.rs VERTEX_REGIONS (9), VERTEX_PROXY_REGIONS (5); target_regions(pref), proxy_target_regions(pref); build_vertex_url(p,l,m,VertexMethod)
  vertex/auth.rs   async get_vertex_access_token(api_key) -> Result<String, String>   // §8.6
  vertex/once.rs   async fetch_vertex_response(req) -> Result<String, ApiError>       // §8.10
  vertex/stream.rs async create_vertex_stream(req, sink: &mut dyn EventSink)         // §8.9 (write directly to the sink, no inner byte stream)
  free_api.rs      async fetch_free_api_response(req); async create_free_api_stream(req, sink)
  providers/{openai,gemini,claude,groq,ollama,openrouter,local_proxy,vertex_route}.rs   // one fn per branch, §6.2/§6.3
  dispatch.rs      async chat_once(&self, req: &mut ServerChatRequest) -> Result<String, ApiError>    // §6.1 switch
                   async chat_stream(&self, req, sink)                                              // §7.2 dispatch and catch
                   async vertex_proxy_once / vertex_proxy_stream                                    // §12
  ratelimit.rs     RateLimiter { state: std::sync::Mutex<RateState>, cfg } where RateState { per_ip: HashMap<String, VecDeque<Instant>>, global: i64 }
                   fn load(cfg) -> Self (reads the counter file with js_parse_int); fn check(&self, ip:&str, counts_global: bool) -> RateDecision; fn global(&self) -> i64
                   RateDecision { allowed: bool, remaining: u32, error_msg: Option<&'static str or String>, count: i64 }
                   The counter write happens inside the lock (5-byte file; std::fs ok, or spawn_blocking)
  chatlog.rs       ChatLog { path, lock: std::sync::Mutex<()> }; fn append(ip, provider: &Value, request_text, reply_text, count)
                   #[derive(Serialize)] struct Entry<'a> { timestamp: String, ip: &'a str, provider: &'a Value, count: i64,
                                                           #[serde(rename="requestText")] request_text: &'a str, #[serde(rename="replyText")] reply_text: &'a str }
  sink.rs          #[async_trait] trait EventSink { async fn emit(&mut self, ev: StreamEvent) -> Result<(), SinkClosed>; }
                   struct LoggingSink<S> { inner: S, ip, provider: Value, question, count, log: Arc<ChatLog> }   // §7.2 step a: log BEFORE forwarding
```
Implementation notes:
- Write streams as `async fn`s that push into an `EventSink`, rather than nested `ReadableStream`s. The V1 relay functions (`streamVertex`, `streamFree`) are byte-identical pass-through, so calling `create_vertex_stream(req, sink)` directly is equivalent.
- On `SinkClosed` (client gone), return immediately. Dropping the `reqwest::Response` cancels the upstream request.
- Upstream bodies: `response.bytes_stream()`. Map read errors to `"terminated"`.
- `serde_json` with feature `preserve_order`. Never use `Value` key-sorting.
- Recommended crates:
  - Networking: reqwest 0.12 (`json`, `stream`, `rustls-tls`).
  - Async: tokio 1 (`rt-multi-thread`, `process`, `time`, `sync`, `fs`), futures-util, bytes, async-trait.
  - JSON: serde, serde_json (`preserve_order`).
  - Signing: rsa 0.9, sha2 0.10, pkcs8, base64 0.22 (`URL_SAFE_NO_PAD`).
  - Utilities: regex 1, rand 0.8/0.9, chrono (or time), percent-encoding 2, dotenvy, tracing (keep the §15.3 text exactly; `println!`/`eprintln!` is acceptable).

**amadeus-server** (axum 0.8):
```rust
fn api_router(base: &str) -> Router<AppState> {
    Router::new()
      .route(&format!("{base}/app/api/chat"),              post(post_chat).options(opts_post))
      .route(&format!("{base}/app/api/chat/stream"),       post(post_chat_stream).options(opts_post))
      .route(&format!("{base}/app/api/chat/quota"),        get(get_quota).options(opts_get))    // axum gives HEAD for free
      .route(&format!("{base}/app/api/chat/vertex-proxy"), post(post_vertex_proxy).options(opts_post))
      .layer(DefaultBodyLimit::max(16 * 1024 * 1024))
}
// handlers take (State<AppState>, HeaderMap, Bytes): never Json<..>
// opts_post → 204 + "Allow: OPTIONS, POST"; opts_get → 204 + "Allow: GET, HEAD, OPTIONS"; other methods: axum's 405 (empty body) matches V1
// middleware no_cache_headers: for every response whose path is not under /resources/, OVERWRITE the 5 headers (§1.3),
//   including Cache-Control on SSE and quota responses (V1-effective values)
// middleware trailing_slash_redirect: "/x/" → 308 Location "/x" (Next default)
// client_ip(&HeaderMap) -> String: exactly §10.1 (no ConnectInfo)
```
SSE handler shape:
```rust
let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(32);
tokio::spawn(async move {
    let mut sink = LoggingSink::new(ChannelSink(tx), ip, provider_val, question, count, log);
    backend.chat_stream(req, &mut sink).await;      // always ends with close (tx dropped)
});
Response::builder().status(200)
  .header("content-type", "text/event-stream; charset=utf-8")
  .header("connection", "keep-alive")              // only add for HTTP/1.1 requests
  .body(Body::from_stream(ReceiverStream::new(rx).map(Ok::<_, Infallible>)))
```
Do **not** use `axum::response::Sse`. It writes its own framing, can inject keep-alive comments, and sets its own Cache-Control.

Server config (env, with defaults matching V1):

| Variable | Default |
|---|---|
| `AMADEUS_BASE_PATH` | `/amadeus` |
| `PORT` | 9501 |
| `AMADEUS_DATA_DIR` | CWD; holds `.amadeus_request_count`, `.amadeus_chat_log.jsonl`, `realamadeusmain-7a9af55234f3.json` |
| `AMADEUS_FREEAPI_MAX` | 65000; the cap error message must interpolate it |
| `AMADEUS_RATE_LIMIT_PER_MIN` | 20 |
| `AMADEUS_ENABLE_VERTEX_PROXY` | see Open questions |
| `AI_STUDIO_API_KEY` / `GEMINI_API_KEY` | semantics of §8.11 |
| `AMADEUS_DEFAULT_AI_STUDIO_KEY` | optional; replaces the hard-coded constant |

### B. amadeus-desktop (Tauri 2)
```rust
#[tauri::command] async fn chat_send(state: State<'_, Backend>, payload: serde_json::Value) -> Result<ChatResponseBody, ApiErrorDto>;   // ApiErrorDto { status: u16, error: String }
#[tauri::command] async fn chat_stream(state: State<'_, Backend>, payload: serde_json::Value, on_event: tauri::ipc::Channel<StreamEvent>, stream_id: u32) -> Result<(), String>;
#[tauri::command] async fn chat_stream_cancel(state: State<'_, Backend>, stream_id: u32);   // AbortHandle map
#[tauri::command] async fn chat_quota(state: State<'_, Backend>) -> Option<QuotaInfo>;      // remote server quota when FreeAPI relays, else None
```
- Desktop `BackendConfig`: `rate_limit = None` and `chat_log = None` (single user, no IP). `embedded_vertex_credential = None`. `free_api = RemoteRelay` (see Open questions).
- The same `StreamEvent` JSON goes through `Channel`, so the UI parsing is identical.
- `ChannelSink` maps `Channel::send` errors to `SinkClosed`.

### C. amadeus-ui Transport (for the UI owner)
```rust
#[async_trait(?Send)] pub trait ChatTransport {
    async fn send_chat(&self, p: &ChatRequestPayload) -> Result<String, String>;                       // §16 sendChat semantics
    async fn send_chat_streaming(&self, p: &ChatRequestPayload, on: &mut dyn FnMut(StreamEvent)) -> Result<(), String>;  // §16 streaming semantics
    async fn fetch_quota(&self) -> Option<QuotaInfo>;
    fn cancel_streaming(&self);
}
```
- `HttpTransport` (web): `web_sys` fetch with ReadableStream reader and AbortController. Use the same line reader as §7.3, with JSON-parse-failure meaning a raw token.
- `TauriTransport` (desktop): `invoke` plus `Channel`.

### D. gcloud discovery per OS (desktop; extends §8.5)
- **Linux**: the V1 list exactly.
- **macOS**: add `/opt/homebrew/bin/gcloud`, `/opt/homebrew/share/google-cloud-sdk/bin/gcloud` and `/usr/local/share/google-cloud-sdk/bin/gcloud`, and keep `$HOME/google-cloud-sdk/bin/gcloud`. A GUI app's `PATH` is minimal, so explicit paths matter.
- **Windows**, from the local-proxy script: `gcloud`, `%LOCALAPPDATA%\Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd`, `%ProgramFiles%\Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd`, `%USERPROFILE%\AppData\Local\Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd`. Run them as `cmd.exe /d /s /c "\"<path>\" auth print-access-token"` with `CREATE_NO_WINDOW` (0x08000000) to avoid a console flash.

### E. Test plan (golden vectors from this spec, generated by running the V1 functions)
- §3 body builders: byte-exact JSON strings.
- §3.1 Groq filter.
- §4.1 regex extractor.
- §5.1 URLs.
- §8.7 region lists.
- §6.1 / §7.6 error formats.
- §7.1 SSE escaping.
- §10.6 ISO timestamp.
- `encodeURIComponent`.
- SSE line reader with chunk splits inside multi-byte characters and inside `data: ` prefixes.
- Rate limiter:
  - 20 allowed, the 21st denied.
  - A denied attempt is not recorded.
  - Window strictness `< 60000`.
  - The global cap applies only to provider 8 or missing; `null` does not count.
- Interceptor logs only non-empty `complete`.
- Mock upstreams (wiremock) for each provider branch, including Vertex retry counts and sleeps (use `tokio::time::pause`), and the silent-close quirk Q5.

---

## Open questions / risks

1. **FreeAPI on desktop (top risk).** FreeAPI depends on server secrets: the AI Studio key, and gcloud credentials for project `realamadeusfreecredit`. It also depends on a server-wide quota counter. None of these can live in a desktop or WASM binary. Recommendation: desktop relays provider 8 to the official server's `/amadeus/app/api/chat[/stream]` and `/quota` (`FreeApiMode::RemoteRelay`). **The official public base URL is needed.** The OpenRouter Referer says `https://dsuv.vercel.app`, but production runs `next start` behind a proxy.
2. **Secrets in source.** `DEFAULT_AI_STUDIO_API_KEY` is hard-coded in `shared.ts:28` (uncommitted). Move it to env or config and consider rotating it. The embedded service-account JSON must stay server-side; exclude it from Tauri resources and the Trunk `dist`. The V1 log line prints the first 15 characters of tokens and keys; V2 should mask it, e.g. `ya29.***`.
3. **The FreeAPI quota is exhausted now.** `.amadeus_request_count` = 65000 = cap, so every FreeAPI user receives the rate-limited fallback. Decide whether V2 launch keeps, raises or resets the cap. Make it configurable, with the message interpolating the cap.
4. **Security of the V1 surface, preserved for parity.**
   - (a) `vertex-proxy` is unauthenticated and not rate-limited, and it uses the server's gcloud token for any `vertexProject`.
   - (b) The embedded SA (project `realamadeusmain`) can be used by anyone through provider 4.
   - (c) SSRF: `ollamaHost`, `localProxyHost`/`Port` and `openaiBaseUrl` make the server POST to arbitrary URLs. Error bodies are reflected back, and provider 7 forwards the whole payload.
   - (d) `X-Forwarded-For` first-hop trust allows rate-limit bypass and log poisoning.
   - Decide: keep parity by default and add opt-in hardening flags (allow-lists, trusted proxy count, disable vertex-proxy)? **`AMADEUS_ENABLE_VERTEX_PROXY` default:** parity says on. It has no first-party caller and is risky, so I recommend off with a documented flag; the orchestrator should confirm.
5. **Privacy.** The chat log (46.8 MB, full texts plus IPs), the counter and the achievements dump are tracked in git. V2 should keep data in `AMADEUS_DATA_DIR` outside the repo, gitignore it, and consider log rotation. A git-based deploy can also reset or overwrite the counter.
6. **Parity versus fixing bugs.** Q5 (silent close), Q6 (double rate limit and log), Q7 (last-line drop), Q8 (regex chunk parsing) and Q9 (partial FreeAPI streams) are observable. This spec says keep, except Q6. The orchestrator should confirm.
7. **JWT lifetime.** V1 signs `iat = now-30`, `exp = now+3600` (3,630 s). Google documents a maximum of 1 h after `iat`. The SA path may already fail in V1 with `invalid_grant` ("Token must be a short-lived token (60 minutes)..."), which nobody would notice because provider 4 never appears in the logs. Verify against Google before deciding whether to keep it.
8. **Error-string parity is load-bearing.** The UI substring-matches errors (§15.4). The Rust backend must emit `fetch failed` and `terminated` for transport errors and canonical reason phrases. HTTP/2 upstreams have no reason phrase, so use `canonical_reason()`. Arbitrary reqwest messages would change user-visible text.
9. **Upstream API drift, inherited.**
   - `claude-3-7-sonnet-20250219` and `gemini-2.0-flash` may be retired by 2026.
   - Groq `qwen3-32b` may need the `qwen/` prefix.
   - OpenAI rejects `max_tokens` for o-series and newer models (it requires `max_completion_tokens`).
   - Gemini 2.5+ thinking tokens count against `maxOutputTokens: 2048`, which can produce empty replies or `[Parse Error]`.
   - API keys on regional `aiplatform` endpoints are usually rejected (Vertex express mode supports only the global, project-less path).
   - Keep the V1 bodies for parity, but flag these in the UI docs.
10. **Single-process assumption.** The rate limiter, counter and log are process-local. V2 must run as one instance, or move the state to shared storage.
11. **Compression and transport.** V1 likely gzips SSE (Next compression, with `no-transform` dropped). V2 will not. This is transparent to clients, but reverse proxies must not buffer `text/event-stream`. Consider sending `X-Accel-Buffering: no`; it is harmless.
12. **Body size.** V1 has no limit and axum defaults to 2 MB. Set 16 MB or more.
13. **Out-of-scope confirmations needed.** CanopyWave/`atchannel` is not RealAmadeus (Appendix A). `.amadeus_achievements.json` has no server reader or writer and can be ignored. `vertex-proxy` has no client; confirm it should be kept.
14. **Lone surrogates.** V1 can pass lone UTF-16 surrogates through, for example from upstream `\ud83d` escapes or `substring` cuts in log previews. Rust strings cannot hold them, and serde_json rejects lone-surrogate escapes, so such upstream JSON would become `[Parse Error]`. This is ultra-rare and acceptable.

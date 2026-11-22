# Porting spec: subsystem `lib-media`

**Scope:** AudioService, TTSService, STTService, AIService (client side) and the `Live2DCanvas` React component.
**Source:** RealAmadeusWeb V1, `/home/elvelt_admin/Elvelt/dsuv`. The source is the working tree as of 2026-11-16 (HEAD `7e45174` plus uncommitted edits; `Live2DCanvas.tsx` is one of the modified files).
**Target:** RealAmadeus V2, a Rust workspace at `RealAmadeusRust/`: `amadeus-core`, `amadeus-ui` (Leptos 0.8 CSR, wasm), `amadeus-live2d`, `amadeus-backend`, `amadeus-server` and `apps/desktop` (Tauri 2).

## Files owned by this spec (all read in full)

| File | Lines | Role |
|---|---|---|
| `lib/amadeus/services/audio-service.ts` | 152 | `HTMLAudioElement` playback with a synthetic lip-sync value |
| `lib/amadeus/services/tts-service.ts` | 252 | TTS client: in-memory blob-URL cache and health/runtime info |
| `lib/amadeus/services/stt-service.ts` | 380 | STT client: browser Web Speech API first, else MediaRecorder plus server transcription, with a level meter |
| `lib/amadeus/services/ai-service.ts` | 173 | LLM client: builds the payload from settings; non-stream POST plus streaming SSE over fetch |
| `components/amadeus/Live2DCanvas.tsx` | 162 | Loads the Cubism core and runtime scripts, creates the player, forwards props, eye tracking, WebGL cleanup |

## Files read for context (they define the contracts above; other specs own them)

- `lib/amadeus/signal.ts`: the `Signal` primitive, 25 lines.
- `lib/amadeus/app-settings.ts`: localStorage accessors, 137 lines.
- `lib/amadeus/types.ts`: `ChatMessage`.
- `lib/amadeus/chat-utils.ts`: `stripEmotionTag`, `parseEmotionTag`, `isPauseCharacter`, `charDelayForChar`.
- `components/amadeus/RealAmadeusApp.tsx`, the main consumer:
  - ChatPanel at L828–1990.
  - MiniGamePanel speak at L2565–2616.
  - Settings apply at L3095–3322.
  - Root startup at L3971–3977.
  - Hooks at L423–475.
- `app/amadeus/app/api/tts/{synthesize,health}/route.ts`, `app/amadeus/app/api/stt/{transcribe,health}/route.ts`, `app/amadeus/app/api/chat/stream/route.ts` (SSE framing, L21–24 and L436–540) and `app/amadeus/app/api/chat/shared.ts` (provider ids).
- `public/resources/live2d/reama-cubism-runtime.js`:
  - Player API at L18075–18643.
  - Tables and helpers at L17690–18010.
  - Resource constants at L2666–2700.
  - GL context at L2767–2798.
  - Subdelegate/pointer code at L17480–17687.
- `app/amadeus/app/globals.css`: L323–349, 476–532, 677–722, 1216–1224, 2317–2340, 2405–2450, 2586–2965.

> **Why the consumer contract (§7) is included.** Several behaviours in the focus list live in `ChatPanel`, not in the services: audio queueing, what the mouth actually receives, and how STT results reach the UI. They are documented here as the exact contract the services must satisfy. The chat-panel spec owns the rest of ChatPanel.

---

## 0. TL;DR: facts that are easy to get wrong

1. **The Live2D mouth is NOT driven by `AudioService.lipSyncValueChanged`.** Nothing in V1 subscribes to that signal. Two values drive the mouth:
   - `lipSyncValue`, which is effectively binary: 1 while text is being typed (or while a TTS chunk plays during the `typing` state), else 0.
   - `spokenChar`, the character currently being typed. The player maps vowels to mouth openness.

   AudioService's sine/noise lip-sync generator is dead code in V1 (§2.5, §7.2).
2. **There is no VAD and no silence detection.** STT stops in four cases: manual toggle, a 30 s timer, disabling STT, or ChatPanel unmount. The only exception is the browser `SpeechRecognition` path with `continuous=false`, where the *browser* ends the session on silence. The analyser in STTService only feeds a UI level meter.
3. **STT prefers the browser Web Speech API whenever the constructor exists** (`window.SpeechRecognition ?? window.webkitSpeechRecognition`). This covers Chrome, Edge and Safari.
   - In that case the provider and serverUrl settings are ignored and no audio is uploaded.
   - MediaRecorder → `/stt/transcribe` is only the fallback (for example on Firefox).
4. **The mic button is hidden unless localStorage `Config_AudioInputEnabled === "1"`.** No code in the current V1 source writes this key; only stale one-off `patch_amadeus*.js` scripts at the repo root do. It is read once when ChatPanel mounts.
5. **TTS queueing lives in ChatPanel:**
   - A text-chunk queue feeds one synthesis request in flight at a time.
   - Results go to an audio (object-URL) queue, which plays one item at a time.
   - Text is split on `。！？!?\n`, with a 120-UTF-16-unit cap in the non-stream path.
6. **AIService has two request paths:**
   - `sendChat`: a non-streaming POST. It is **not abortable**; `cancelStreaming` does not affect it.
   - `sendChatStreaming`: fetch plus an SSE line parser, abortable.

   ChatPanel streams only for providers `1,3,4,5,6` (Gemini, Groq, Vertex, Ollama, OpenRouter). The default provider is `8` (FreeAPI), which therefore uses the non-streaming path.
7. **Live2DCanvas script loading:**
   - It loads `live2dcubismcore.min.js` and `reama-cubism-runtime.js?v=19` **in parallel** (both async). There is a latent race, described in §6.9.
   - It then calls `ReamaCubismRuntime.create(host)` and forwards props through `setEmotion(upper)`, `setLipSync`, `setSpokenChar` and `setEyeTracking`.
8. **Eye tracking:**
   - It is driven by **window** `mousemove` (viewport-relative, not canvas-relative), mapped to `[-0.7, 0.7]` with Y inverted.
   - Disabling it sends `(0, 0)`.
   - Touch input is not handled.
9. **Canvas override:** the comment says `pointerEvents = "auto"` "enables CubismDragManager eye tracking". This is **false**: the runtime never registers pointer listeners, so the override only makes the canvas a hit-test target.
10. **In V2 the 670 KB `reama-cubism-runtime.js` is replaced by `amadeus-live2d` (wasm).** Only `live2dcubismcore.min.js`, which is proprietary, remains JS and must be loaded **before** the wasm player initializes.

---

## 1. Shared primitives these files depend on

### 1.1 `Signal<TArgs>` (`lib/amadeus/signal.ts`, verbatim)

```ts
export type SignalHandler<TArgs extends unknown[]> = (...args: TArgs) => void;

export class Signal<TArgs extends unknown[] = []> {
  private handlers = new Set<SignalHandler<TArgs>>();

  connect(handler: SignalHandler<TArgs>): () => void {
    this.handlers.add(handler);
    return () => this.disconnect(handler);
  }

  disconnect(handler: SignalHandler<TArgs>): void {
    this.handlers.delete(handler);
  }

  emit(...args: TArgs): void {
    for (const handler of [...this.handlers]) {
      handler(...args);
    }
  }

  clear(): void {
    this.handlers.clear();
  }
}
```

Semantics that must be preserved:

- **Ordering and dedup.** Handlers are called in connection order. Connecting the same function reference twice stores it once. This never happens in V1 because every `connect` passes a fresh arrow function.
- **Synchronous, snapshot emission.** `emit` copies the handler set and calls each handler on the emitter's stack.
  - A handler connected during an emit is not called in that emit.
  - A handler disconnected during an emit is still called in that emit if it was in the snapshot.
- **Re-entrancy happens.** For example, `TTSService.synthesize()` emits `synthesisComplete` synchronously on a cache hit while the caller is itself inside a signal handler (§3.6, §7.3).
- **Exceptions.** A throwing handler aborts the remaining handlers, and the exception propagates to the `emit` caller. Many `emit` calls in these services sit inside `try { … } catch`, so a handler exception there is converted into the service's *error* signal (§12, quirks Q-S1 to Q-S3). Rust handlers cannot throw, so this path disappears. Do not replicate it.

### 1.2 `AppSettings` (subset used here)

- **Storage.** Backed directly by `window.localStorage` with no key prefix. On the server (SSR) an in-memory map is used instead.
- **`getInt(key, def)`.**
  1. `raw = localStorage.getItem(key)`. If `null`, return `def`.
  2. `p = Number.parseInt(raw, 10)`.
  3. Return `Number.isFinite(p) ? p : def`.

  JS `parseInt` skips leading whitespace, accepts a sign and stops at the first non-digit: `"1.9"` → 1, `"12abc"` → 12, `"abc"` → `def`, `""` → `def`.
- **`getFloat(key, def)`.** Uses `Number.parseFloat` with the same finite check: `"0.8"` → 0.8, `".5"` → 0.5, `"1e-7"` → 1e-7, `"0.5x"` → 0.5, `"Infinity"` → `def`.
- **`getString(key, def)`.** Returns the raw value or `def`. An empty string `""` is returned as `""`, not replaced by `def`.
- **Setters.** Every setter writes the value, then emits `settingsChanged(key)`. It emits even when the value did not change.
  - `setInt(k, v)` writes `String(Math.trunc(v))`.
  - `setFloat(k, v)` writes `String(v)`, the JS Number→String form: `1` → `"1"`, `0.8` → `"0.8"`, `1e-7` → `"1e-7"`.
  - `setString(k, v)` writes the raw value.
- **`settingsChanged: Signal<[string]>`.** TTSService and STTService subscribe permanently in their constructors.
- **`initializeDefaults()`.** Runs in the root component's mount effect (RealAmadeusApp L3972). That is **after** the service singletons were constructed at module evaluation. The services therefore read localStorage directly, using their own defaults, which are identical to `DEFAULT_SETTINGS`.

### 1.3 `ChatMessage` (`lib/amadeus/types.ts`)

```ts
export type ChatRole = "system" | "user" | "assistant";
export interface ChatMessage { role: ChatRole; content: string; }
```

### 1.4 Singletons and construction timing

`export const AudioService = new AudioServiceClient()`, and likewise `TTSService`, `STTService` and `AIService`. Each is constructed once, at first module import:

- In the browser, this is client bundle evaluation, before React hydration.
- During SSR, `hasWindow` is false and settings come from an empty memory map, so `enabled = false` and no fetch is issued.

TTS and STT constructors read settings, subscribe to `settingsChanged` (never unsubscribed), and call `checkServerHealth()` if enabled.

### 1.5 URL conventions (hard-coded in V1)

- **API calls use absolute paths with the `/amadeus` prefix:**
  - `/amadeus/app/api/chat`
  - `/amadeus/app/api/chat/stream`
  - `/amadeus/app/api/tts/synthesize`
  - `/amadeus/app/api/tts/health`
  - `/amadeus/app/api/stt/transcribe`
  - `/amadeus/app/api/stt/health`
- **Live2D assets live at the site root, not under `/amadeus`:**
  - `/resources/live2d/live2dcubismcore.min.js`
  - `/resources/live2d/reama-cubism-runtime.js?v=19`
  - `/resources/models/AmadeusKurisu5.0/reama5.0/…`
  - `/resources/live2d/shaders/…`
- **Caching (Next config).**
  - `/resources/:path*` is served with `Cache-Control: public, max-age=86400, stale-while-revalidate=604800`. The same rule sets `Pragma: public`, `Expires: 86400`, `Surrogate-Control: public, max-age=86400` and `CDN-Cache-Control: public, max-age=86400`.
  - Everything else is served with `no-store, no-cache, must-revalidate, proxy-revalidate, max-age=0`.

---

## 2. AudioService (`lib/amadeus/services/audio-service.ts`)

### 2.1 Constants

| Name | Value | Meaning |
|---|---|---|
| `LIP_SYNC_UPDATE_MS` | `50` | Lip-sync tick interval (20 Hz) |
| `LIP_SYNC_SMOOTHING` | `0.3` | EMA factor α |

### 2.2 Fields and initial values

| Field | Initial | Notes |
|---|---|---|
| `audio: HTMLAudioElement \| null` | `null` | The current element |
| `lipSyncTimer: number \| null` | `null` | `setInterval` id |
| `_isPlaying` | `false` | |
| `_currentLipSyncValue` | `0` | |
| `_volume` | `AppSettings.getFloat("Config_VoiceVol", 1.0)` | See notes below |

Notes on `_volume`:

- It is read **once** at construction and is **not clamped** at load. A stored `"1.5"` makes `audio.volume = 1.5` throw `IndexSizeError` inside `playFromFile`'s `try`, which ends in `errorOccurred(e.message)`.
- It is never re-read when settings change. `Config_MasterVol` is never applied.

### 2.3 Signals

| Signal | Payload | Emitted from | V1 subscribers |
|---|---|---|---|
| `playbackStarted` | – | `play()` promise resolved (in `playFromFile` only) | ChatPanel: `ttsPlaybackBusyRef = true` |
| `playbackFinished` | – | `ended` event | ChatPanel: `busy = false`, then `playNextTTSChunkIfPossible()` |
| `playbackStateChanged` | – | ended; play resolved; `stop()` if it was playing; `pause()`; `resume()` resolved | none |
| `lipSyncValueChanged` | `number` | `stop()`, ended, every tick | **none** |
| `volumeChanged` | `number` | `setVolume` | none |
| `errorOccurred` | `string` | `error` event; `play()` rejected; exception in `playFromFile`; `resume()` rejected | ChatPanel: `busy = false`, then `playNextTTSChunkIfPossible()` |

ChatPanel calls only `playFromFile` and `stop`. `setVolume`, `playFromData`, `pause`, `resume` and the getters are unused in V1.

### 2.4 Methods

**`get isPlaying` / `get currentLipSyncValue` / `get volume`** return the fields.

**`setVolume(value: number): void`** (unused)
1. `_volume = Math.max(0, Math.min(1, value))`. A NaN input stays NaN.
2. If `audio` exists, set `audio.volume = _volume`. With NaN this throws an uncaught `TypeError` ("non-finite"). With no element, nothing throws, and `"NaN"` is persisted in the next step.
3. `AppSettings.setFloat("Config_VoiceVol", _volume)`, which emits `settingsChanged`.
4. `volumeChanged.emit(_volume)`.

**`playFromFile(filePath: string): void`**
1. `this.stop()`. This always emits `lipSyncValueChanged(0)` and may emit `playbackStateChanged`.
2. Inside `try`:
   1. `audio = new Audio(filePath)`. Setting the src starts loading.
   2. `audio.volume = _volume`.
   3. `audio.onended = () => { stopLipSyncTimer(); _isPlaying = false; _currentLipSyncValue = 0; lipSyncValueChanged.emit(0); playbackStateChanged.emit(); playbackFinished.emit(); }`. Keep this exact order.
   4. `audio.onerror = () => errorOccurred.emit("Audio playback failed")`. This does **not** reset `_isPlaying` and does not stop the timer.
   5. `audio.play().then(() => { _isPlaying = true; playbackStateChanged.emit(); playbackStarted.emit(); startLipSyncTimer(); }).catch(err => errorOccurred.emit(err instanceof Error ? err.message : "Audio playback failed"))`.
3. `catch (err)`: `errorOccurred.emit(err instanceof Error ? err.message : "Audio playback failed")`.

Notes:

- The handlers belong to that element but write to shared `this` fields.
- `stop()` pauses the previous element. If its `play()` promise was still pending, it rejects with `AbortError` ("The play() request was interrupted by a call to pause()…"), which emits a **spurious** `errorOccurred`.
- One failure can fire both `onerror` and the rejected `play()`, so `errorOccurred` may be emitted twice.
- `ended` does not null `this.audio`.

**`playFromData(blob: Blob): void`** (unused): `playFromFile(URL.createObjectURL(blob))`. The URL is never revoked.

**`stop(): void`**
1. `stopLipSyncTimer()`.
2. `_currentLipSyncValue = 0`, then `lipSyncValueChanged.emit(0)`. This happens always, even when idle.
3. If `audio` exists: `audio.pause()`, `audio.currentTime = 0`, `audio = null`. The handlers stay attached to the detached, paused element, so it never fires `ended`.
4. If `_isPlaying`: `_isPlaying = false`, then `playbackStateChanged.emit()`.

`stop()` never emits `playbackFinished`.

**`pause(): void`** (unused)
1. If there is no `audio`, return.
2. `audio.pause()`, `stopLipSyncTimer()`, `_isPlaying = false`, `playbackStateChanged.emit()`. It emits unconditionally. The lip-sync value is **not** reset; the last value stays.

**`resume(): void`** (unused)
1. If there is no `audio`, return.
2. `audio.play().then(() => { _isPlaying = true; startLipSyncTimer(); playbackStateChanged.emit(); }).catch(err => errorOccurred.emit(err instanceof Error ? err.message : "Failed to resume audio"))`.

Unlike `playFromFile`, `resume` does not emit `playbackStarted`, and it starts the timer before emitting state.

**`startLipSyncTimer()`:** `stopLipSyncTimer()`, then `lipSyncTimer = window.setInterval(() => this.updateLipSync(), 50)`.
**`stopLipSyncTimer()`:** if a timer is set, `clearInterval` it and set it to `null`.

**`updateLipSync()`** (verbatim logic)
```ts
if (!this._isPlaying || !this.audio) { this._currentLipSyncValue = 0; this.lipSyncValueChanged.emit(0); return; }
const position = this.audio.currentTime;                 // seconds (f64)
const base   = Math.sin(position * 8.0) * 0.3 + 0.5;     // [0.2, 0.8]
const detail = Math.sin(position * 15.0) * 0.15;         // [-0.15, 0.15]
const accent = Math.sin(position * 3.0) * 0.1;           // [-0.1, 0.1]
const noise  = (Math.random() - 0.5) / 5;                // [-0.1, 0.1)
const target = Math.min(1, Math.max(0, base + detail + accent + noise));
this._currentLipSyncValue = this._currentLipSyncValue * (1 - 0.3) + target * 0.3;
this.lipSyncValueChanged.emit(this._currentLipSyncValue);
```

### 2.5 Lip-sync behaviour and test vectors

- **Not amplitude-based.** It is a pseudo-random oscillation keyed to the playback position, smoothed by an EMA (α = 0.3). It resets to 0 at every `playFromFile` (via `stop()`) and at `ended`.
- **Test vectors for `target` with noise = 0:**

  | `currentTime` (s) | `target` |
  |---|---|
  | 0 | 0.500000 |
  | 0.05 | 0.734015 |
  | 0.1 | 0.894383 |
  | 0.25 | 0.755219 |
  | 0.5 | 0.513409 |
  | 1.0 | 0.908463 |

- **EMA from 0 with a constant target of 1:** 0.3, 0.51, 0.657, 0.7599, 0.83193.
- **Dead code in V1.** Keep it in V2 for API parity; it costs little. §7.2 describes what actually drives the mouth.

### 2.6 Error strings (verbatim)

- `"Audio playback failed"` (from `onerror`, a non-Error rejection, or a non-Error exception).
- `"Failed to resume audio"`.
- Otherwise the browser's `DOMException.message`, for example "play() failed because the user didn't interact with the document first.", "The element has no supported sources." or "The play() request was interrupted by a call to pause()…".

---

## 3. TTSService (`lib/amadeus/services/tts-service.ts`)

### 3.1 `simpleHash(input: string): string` (verbatim)

```ts
function simpleHash(input: string): string {
  let hash = 0;
  for (let index = 0; index < input.length; index += 1) {
    hash = (hash << 5) - hash + input.charCodeAt(index);
    hash |= 0;
  }
  return Math.abs(hash).toString(16);
}
```

- The loop runs over **UTF-16 code units** (`charCodeAt`); an emoji contributes two surrogate units.
- Arithmetic is int32 with wraparound. A Rust equivalent: `h = h.wrapping_shl(5).wrapping_sub(h).wrapping_add(cu as i32)` over `s.encode_utf16()`. This matches JS exactly because the JS intermediate value is an exact double that `|0` reduces modulo 2³².
- The output is `abs` as i64, formatted as lowercase hex without padding. `-2147483648` becomes `"80000000"`.

Test vectors (computed with Node 22):

| Input | Output |
|---|---|
| `""` | `0` |
| `"a"` | `61` |
| `"こんにちは\|christina-jp\|http://localhost:5100"` | `5bf08130` |
| `"Hello.\|christina\|http://localhost:5100"` | `a4018c8` |
| `"😀\|christina-jp\|http://localhost:5100"` | `54a3cd3a` |
| `"べ、別にあんたのためじゃないんだからね！\|christina-jp\|http://localhost:5100"` | `7977e28b` |

The hash is only an in-memory cache key. Collisions return the wrong audio, which is a V1 quirk. See §12 Q-T1.

### 3.2 `HealthResponse` (as typed client-side)

```ts
interface HealthResponse {
  available: boolean;
  runtime?: { using_gpu?: boolean; device?: string; gpu_name?: string; attn_implementation?: string };
  torch?:   { cuda_available?: boolean; detected_gpu_name?: string };
}
```

The server route forwards the upstream `/health` JSON merged as `{ available: true, ...upstreamBody }`, so other fields may appear. An upstream body containing `"available": false` overrides `true` because the spread comes last.

### 3.3 Fields, defaults and settings keys

| Field | Initial | Synced from (key, default) |
|---|---|---|
| `cache: Map<string,string>` | empty | Hash → object URL. Never evicted; URLs revoked only by `clearLocalCache()`, which is unused. |
| `_enabled` | `false` | `getInt("Config_Experimental_TTS_Enabled", 0) === 1` |
| `_speaker` | `"christina-jp"` | `getString("Config_Experimental_TTS_Speaker", "christina-jp")` |
| `_serverUrl` | `"http://localhost:5100"` | `getString("Config_Experimental_TTS_ServerUrl", "http://localhost:5100")` |
| `_serverAvailable` | `false` | – |
| `_usingGpu` | `false` | – |
| `_runtimeDevice` | `"unknown"` | – |
| `_runtimeDetails` | `"unknown"` | – |

Speaker values offered by the UI: `TTS_SPEAKER_NAMES = ["christina-jp", "christina"]` (RealAmadeusApp L326).

### 3.4 Signals

| Signal | Payload | V1 subscribers |
|---|---|---|
| `synthesisComplete` | object URL `string` | ChatPanel (§7.3) |
| `synthesisError` | `string` | ChatPanel (ignores the text; §7.3) |
| `enabledChanged`, `speakerChanged`, `serverUrlChanged`, `serverAvailableChanged`, `runtimeInfoChanged` | – | none |

### 3.5 Constructor

```ts
this.syncFromSettings();
AppSettings.settingsChanged.connect((key) => {
  if (!key.startsWith("Config_Experimental_TTS_")) return;
  this.syncFromSettings();          // silent: no signals, no health check
});
if (this._enabled) void this.checkServerHealth();
```

`syncFromSettings()` re-reads all three keys (enabled, speaker, serverUrl).

### 3.6 Setters

All three setters return early when `value` equals the current field.

- **`set enabled(v)`:** `_enabled = v`, then `setInt("Config_Experimental_TTS_Enabled", v ? 1 : 0)`, then `enabledChanged.emit()`, then `void checkServerHealth()` if `v` is true.
- **`set speaker(v)`:** `_speaker = v`, then `setString("Config_Experimental_TTS_Speaker", v)`, then `speakerChanged.emit()`. No health check.
- **`set serverUrl(v)`:** `_serverUrl = v`, then `setString("Config_Experimental_TTS_ServerUrl", v)`, then `serverUrlChanged.emit()`, then `void checkServerHealth()`, whether or not TTS is enabled.

**Important V1 interplay (preserve).** Settings "Apply" (RealAmadeusApp L3292–3308) does two things in this order:

1. It **first** writes the keys through `AppSettings.set*`. The `settingsChanged` subscription re-syncs the service silently.
2. It **then** assigns `TTSService.enabled`, `.speaker` and `.serverUrl`. These are now no-ops because the values are already equal.

So after Apply, none of the `*Changed` signals fire and no health check runs. Health is re-checked only:

- at app start (L3975), and
- each time the settings panel becomes visible (L3191; TTS only).

### 3.7 `async synthesize(text: string): Promise<void>`

1. If `!_enabled`, emit `synthesisError("TTS is not enabled")` and return.
2. `normalized = text.trim()`, using JS trim semantics (§Rust mapping, `js_trim`). If it is empty, emit `synthesisError("Empty text provided")` and return.
3. `cacheKey = simpleHash(`${normalized}|${_speaker}|${_serverUrl}`)`.
4. On a cache hit, emit `synthesisComplete(cachedUrl)` and return.

   Steps 1–4 run **synchronously** before the first `await`, so these emissions happen on the caller's stack, inside the `synthesize()` call. This matters for re-entrancy in §7.3.
5. Send `POST /amadeus/app/api/tts/synthesize` with headers `{"Content-Type":"application/json"}`. The body JSON, in this key order: `{"serverUrl": _serverUrl, "speaker": _speaker, "text": normalized, "useCache": true}`.
6. If `!response.ok`:
   - `data = await response.json().catch(() => null)`.
   - Emit `synthesisError(data?.error ?? `Synthesis failed (${response.status})`)` and return.
7. Otherwise:
   - `audioBlob = await response.blob()`. Its type is the response `Content-Type`; the V1 server passes the upstream type through or defaults to `"audio/wav"`.
   - If `audioBlob.size === 0`, emit `synthesisError("Received empty audio data")` and return.
   - `audioUrl = URL.createObjectURL(audioBlob)`, then `cache.set(cacheKey, audioUrl)`, then emit `synthesisComplete(audioUrl)`.
8. `catch (error)`: emit `synthesisError(error instanceof Error ? error.message : "Synthesis failed")`.
   - Network failures give "Failed to fetch" on Chromium, "NetworkError when attempting to fetch resource." on Firefox and "Load failed" on Safari.
   - The `synthesisComplete` emit sits inside the `try`, so a throwing subscriber becomes a `synthesisError` (Q-S1).

Further notes:

- There is no request dedup and no concurrency limit; ChatPanel serializes requests itself.
- The audio format is opaque to the client. It is played as-is through the blob URL.

V1 server behaviour, for parity (`app/amadeus/app/api/tts/synthesize/route.ts`):

- **Body parsing.** It parses the JSON body, or uses `null` if parsing fails. Then:
  - `serverUrl = payload?.serverUrl?.trim()`
  - `text = payload?.text?.trim()`
  - `speaker = payload?.speaker?.trim() || "christina-jp"`
- **Validation.** No serverUrl returns 400 `{"error":"serverUrl is required"}`. No text returns 400 `{"error":"text is required"}`.
- **Upstream request.** `POST ${serverUrl}/synthesize` with `{"Content-Type":"application/json"}` and body `{"text": text, "speaker": speaker, "use_cache": payload?.useCache ?? true}`. There is no timeout.
- **Upstream error.** When the upstream is not OK, it returns `{"error": "Synthesis failed (<status>): <upstream body text>"}` with the **upstream status**.
- **Success.** 200 with the raw bytes, `Content-Type` set to the upstream value or `"audio/wav"`, and `Cache-Control: no-store`.
- **Exception.** 500 `{"error": e.message || "Synthesis failed"}`. On Node, an unreachable host gives `"fetch failed"`.

### 3.8 `async checkServerHealth(): Promise<void>`

```ts
try {
  const response = await fetch("/amadeus/app/api/tts/health", { method: "POST",
      headers: { "Content-Type": "application/json" }, body: JSON.stringify({ serverUrl: this._serverUrl }) });
  const health = (await response.json().catch(() => null)) as HealthResponse | null;
  const nextAvailable = response.ok && Boolean(health?.available);
  if (this._serverAvailable !== nextAvailable) { this._serverAvailable = nextAvailable; this.serverAvailableChanged.emit(); }
  if (!nextAvailable || !health) { this.updateRuntimeInfo(false, "unavailable", "unavailable"); return; }
  const runtime = health.runtime ?? {};
  const torch = health.torch ?? {};
  let usingGpu = Boolean(runtime.using_gpu);
  const runtimeDevice = runtime.device ?? "unknown";
  const gpuName = runtime.gpu_name ?? "";
  const attn = runtime.attn_implementation ?? "";
  const cudaAvailable = Boolean(torch.cuda_available);
  const detectedGpu = torch.detected_gpu_name ?? "";
  const baseDevice = gpuName ? `${runtimeDevice} (${gpuName})` : runtimeDevice;
  let details = attn ? `${baseDevice}, ${attn}` : baseDevice;
  if (runtimeDevice === "not_loaded") {
    usingGpu = false;
    if (cudaAvailable) {
      details = detectedGpu ? `not_loaded (CUDA available: ${detectedGpu})` : "not_loaded (CUDA available)";
    } else {
      details = "not_loaded (CUDA unavailable)";
    }
  }
  this.updateRuntimeInfo(usingGpu, runtimeDevice, details);
} catch {
  if (this._serverAvailable) { this._serverAvailable = false; this.serverAvailableChanged.emit(); }
  this.updateRuntimeInfo(false, "unavailable", "unavailable");
}
```

Notes on JS semantics:

- `??` replaces only `null`/`undefined`. An empty-string `device` stays `""`.
- `Boolean(x)` is JS truthiness: `"false"` is true, while `0`, `""` and `null` are false.
- Template literals stringify non-strings with JS `String()`.

The values are never displayed in V1, so exact non-string handling has low impact. A recommended Rust helper is `js_truthy(&Value)` plus `js_string(&Value)`.

Test vectors as `(response) → (usingGpu, device, details)`:

| Response | usingGpu | device | details |
|---|---|---|---|
| HTTP not OK, or `available` falsy, or body not JSON | false | `unavailable` | `unavailable` |
| `{available:true}` | false | `unknown` | `unknown` |
| `{available:true, runtime:{device:"cpu"}}` | false | `cpu` | `cpu` |
| `{available:true, runtime:{using_gpu:true, device:"cuda:0", gpu_name:"NVIDIA GeForce RTX 4090", attn_implementation:"sdpa"}}` | true | `cuda:0` | `cuda:0 (NVIDIA GeForce RTX 4090), sdpa` |
| `{available:true, runtime:{device:"not_loaded", using_gpu:true}, torch:{cuda_available:true, detected_gpu_name:"RTX 3060"}}` | false | `not_loaded` | `not_loaded (CUDA available: RTX 3060)` |
| Same with `torch:{cuda_available:true}` | false | `not_loaded` | `not_loaded (CUDA available)` |
| Same with `torch:{}` | false | `not_loaded` | `not_loaded (CUDA unavailable)` |

V1 health route (`tts/health/route.ts`):

- No serverUrl: 400 `{"available":false,"error":"serverUrl is required"}`.
- Otherwise `GET ${serverUrl}/health` with a **5000 ms** abort timeout.
  - Upstream not OK: `{"available":false,"error":"<status> <statusText>"}` with HTTP 200.
  - Upstream OK: `{"available":true, ...body}` when the body is an object.
  - Exception: `{"available":false,"error": e.message || "Health check failed"}` with HTTP 200.

### 3.9 `updateRuntimeInfo(usingGpu, device, details)` (private)

`changed` is true if any of the three values differs. All three fields are assigned. If `changed`, `runtimeInfoChanged.emit()` fires.

### 3.10 `clearLocalCache(): void` (unused)

Calls `URL.revokeObjectURL` for every cached URL, then clears the map.

---

## 4. STTService (`lib/amadeus/services/stt-service.ts`)

### 4.1 Constants

| Name | Value |
|---|---|
| `MAX_RECORDING_MS` | `30_000` (applies to both paths) |
| MediaRecorder timeslice | `250` ms (`mediaRecorder.start(250)`) |
| Minimum blob size | `1024` bytes (smaller gives "Recording too short") |
| Analyser `fftSize` | `2048` (so `frequencyBinCount = 1024`) |
| Fallback blob type | `"audio/webm"` |
| Multipart filename | `"recording.webm"`, regardless of the real container |
| Level emitted at browser-recognition start | `0.5` |
| Level emitted at end | `0` |

### 4.2 Fields, defaults and settings keys

| Field | Initial | Synced from (key, default) |
|---|---|---|
| `mediaRecorder`, `mediaStream`, `analyser`, `audioContext` | `null` | – |
| `monitorFrame` (rAF id), `maxDurationTimer` (timeout id) | `null` | – |
| `chunks: BlobPart[]` | `[]` | – |
| `speechRecognition` | `null` | – |
| `_enabled` | `false` | `getInt("Config_Experimental_STT_Enabled", 0) === 1` |
| `_provider` | `"groq"` | `getString("Config_Experimental_STT_Provider", "groq")`; the UI offers `["groq","openai","local"]` (RealAmadeusApp L327) |
| `_serverUrl` | `"http://localhost:5101"` | `getString("Config_Experimental_STT_ServerUrl", "http://localhost:5101")` |
| `_language` | `"ja"` | `getString("Config_Experimental_STT_Language", "ja")`; the UI field is free text |
| `_isRecording` | `false` | – |
| `_serverAvailable` | `false` | – |

### 4.3 Signals

| Signal | Payload | V1 subscriber behaviour (ChatPanel L996–1013) |
|---|---|---|
| `transcriptionComplete` | `string` | If `text.trim()` is non-empty: `setChatInput(text)` (replaces the input with the untrimmed text) and status `"音声入力: OK"` |
| `transcriptionError` | `string` | Status `"音声入力に失敗しました"`. The message itself is not shown. |
| `recordingStateChanged` | – | `setSttRecording(STTService.isRecording)`; if it is not recording, `setSttRecordingLevel(0)` |
| `recordingProgress` | `number` 0..1 | `setSttRecordingLevel(level)` |
| `enabledChanged`, `providerChanged`, `serverUrlChanged`, `languageChanged`, `serverAvailableChanged` | – | none |

### 4.4 Constructor and setters

The constructor matches TTS (§3.5) with prefix `"Config_Experimental_STT_"`, and `syncFromSettings()` re-reads all four keys.

All setters return early on an equal value.

- **`set enabled(v)`:** `_enabled = v`, then `setInt("Config_Experimental_STT_Enabled", v ? 1 : 0)`, then `enabledChanged.emit()`. Then, if `v`, `void checkServerHealth()`; otherwise `this.stopRecording()`.
- **`set provider(v)`:** `setString("Config_Experimental_STT_Provider", v)`, then `providerChanged.emit()`.
- **`set serverUrl(v)`:** `setString("Config_Experimental_STT_ServerUrl", v)`, then `serverUrlChanged.emit()`, then `void checkServerHealth()`.
- **`set language(v)`:** `setString("Config_Experimental_STT_Language", v)`, then `languageChanged.emit()`.

The same Apply interplay as TTS applies: after Apply, the setters are no-ops. One consequence: disabling STT in settings while recording does **not** stop the recording. ChatPanel's mic toggle, however, calls `STTService.enabled = true`, which is effective; this permanently persists `Config_Experimental_STT_Enabled = "1"`.

### 4.5 `async startRecording(): Promise<void>` (decision tree)

```ts
if (!this._enabled) { this.transcriptionError.emit("STT is not enabled"); return; }
if (this._isRecording) return;
if (this.tryStartBrowserRecognition()) return;                      // §4.6 (preferred)
if (typeof navigator === "undefined" || !navigator.mediaDevices?.getUserMedia) {
  this.transcriptionError.emit("Microphone is not supported in this browser"); return;
}
try {
  this.mediaStream = await navigator.mediaDevices.getUserMedia({ audio: true });
  this.mediaRecorder = new MediaRecorder(this.mediaStream);          // no options → UA default mimeType
  this.chunks = [];
  this.mediaRecorder.ondataavailable = (event) => { if (event.data.size > 0) this.chunks.push(event.data); };
  this.mediaRecorder.onstop = () => {
    const blob = new Blob(this.chunks, { type: this.mediaRecorder?.mimeType || "audio/webm" });
    this.cleanupRecordingGraph();
    if (blob.size < 1024) { this.transcriptionError.emit("Recording too short"); return; }
    void this.transcribeData(blob);
  };
  this.mediaRecorder.start(250);
  this._isRecording = true;
  this.recordingStateChanged.emit();
  this.setupRecordingMonitor(this.mediaStream);                        // §4.8
  this.maxDurationTimer = window.setTimeout(() => this.stopRecording(), MAX_RECORDING_MS);
} catch (error) {
  this.transcriptionError.emit(error instanceof Error ? error.message : "Failed to start recording");
}
```

Notes:

- `navigator.mediaDevices` is `undefined` in insecure contexts (plain `http://` other than localhost), which yields "Microphone is not supported in this browser".
- `getUserMedia` errors: `NotAllowedError` ("Permission denied"), `NotFoundError` ("Requested device not found") and `NotReadableError`.
- **Leak.** If `new MediaRecorder` throws after `getUserMedia` succeeded, the stream is **not** stopped and the mic stays open.
- **No re-entrancy guard** while `getUserMedia` is pending. `_isRecording` is still false, so a double click can open two streams.
- **Formats by user agent** (the app forces none):

  | User agent | `mediaRecorder.mimeType` |
  |---|---|
  | Chrome, Edge, WebView2 | `audio/webm;codecs=opus` |
  | Firefox | `audio/ogg;codecs=opus` |
  | Safari / WKWebView 14.1+ | `audio/mp4` (AAC) |
  | WebKitGTK | GStreamer-dependent; may be unsupported |

  The blob type is the recorder's `mimeType`, or `"audio/webm"` if that is empty.
- **Sample rate and channels.** Constraints are `{audio:true}`, so the browser picks the device's native rate (commonly 48 kHz, sometimes 44.1 kHz) and channel count. Chrome defaults `echoCancellation`, `noiseSuppression` and `autoGainControl` to true. Opus encodes internally at 48 kHz. The app does no resampling or format conversion.

### 4.6 `tryStartBrowserRecognition(): boolean` (private, preferred path)

```ts
if (typeof window === "undefined") return false;
const ctor = window.SpeechRecognition ?? window.webkitSpeechRecognition;
if (!ctor) return false;
const recognition = new ctor();
recognition.lang = this.resolveSpeechLanguage(this._language);   // §4.11
recognition.interimResults = false;
recognition.continuous = false;
recognition.onresult = (event) => {
  let transcript = "";
  for (let i = 0; i < event.results.length; i += 1) {
    const result = event.results[i];
    if (result && result.length > 0) transcript += result[0]?.transcript ?? "";
  }
  transcript = transcript.trim();
  if (transcript) this.transcriptionComplete.emit(transcript);
  else this.transcriptionError.emit("No speech detected");
};
recognition.onerror = (event) => {
  const errorCode = event.error ?? "unknown_error";
  this.transcriptionError.emit(`Speech recognition failed: ${errorCode}`);
};
recognition.onend = () => {
  this.speechRecognition = null;
  if (this._isRecording) { this._isRecording = false; this.recordingStateChanged.emit(); }
  if (this.maxDurationTimer !== null) { window.clearTimeout(this.maxDurationTimer); this.maxDurationTimer = null; }
  this.recordingProgress.emit(0);
};
this.speechRecognition = recognition;
this._isRecording = true;
this.recordingStateChanged.emit();
this.recordingProgress.emit(0.5);
this.maxDurationTimer = window.setTimeout(() => this.stopRecording(), MAX_RECORDING_MS);
recognition.start();
return true;
```

Notes:

- On this path provider and serverUrl are ignored and nothing is sent to `/stt/transcribe`. Chrome sends the audio to Google's service.
- The level meter is a constant 0.5 until the session ends.
- **Browser endpointing.** With `continuous=false` the browser ends the session after the first final result or after silence, which fires `onend`. This is the *only* silence handling in V1.
- **Error codes** (W3C): `no-speech`, `aborted`, `audio-capture`, `network`, `not-allowed`, `service-not-allowed`, `bad-grammar`, `language-not-supported`. `onend` always follows `onerror`.
- **Uncaught `start()` throw.** `recognition.start()` can throw synchronously (for example `InvalidStateError`). That throw is **not** caught; it rejects `startRecording`'s promise, which the caller discards with `void`. The service is then left in "recording" until the 30 s timer calls `stopRecording()`.
- Results are concatenated without separators, using the first alternative of each result.

### 4.7 `stopRecording(): void`

```ts
if (!this._isRecording) return;
this._isRecording = false;
this.recordingStateChanged.emit();
if (this.maxDurationTimer !== null) { window.clearTimeout(this.maxDurationTimer); this.maxDurationTimer = null; }
if (this.speechRecognition) {
  const recognition = this.speechRecognition;
  this.speechRecognition = null;
  try { recognition.stop(); }                 // UA then delivers onresult (if any) and onend
  catch (error) { this.transcriptionError.emit(error instanceof Error ? error.message : "Failed to stop speech recognition"); }
  return;
}
if (this.mediaRecorder && this.mediaRecorder.state !== "inactive") this.mediaRecorder.stop();   // final dataavailable → onstop (async)
else this.cleanupRecordingGraph();
```

**Race:** a `startRecording()` issued before the previous recorder's `onstop` fires gets its new `mediaStream`, `mediaRecorder` and `audioContext` torn down by the old `onstop`'s `cleanupRecordingGraph()`.

### 4.8 Level monitor (`setupRecordingMonitor`): a UI meter, not VAD

```ts
if (typeof AudioContext === "undefined") return;          // no webkitAudioContext fallback
this.audioContext = new AudioContext();                    // default options → hardware sample rate
const source = this.audioContext.createMediaStreamSource(stream);
this.analyser = this.audioContext.createAnalyser();
this.analyser.fftSize = 2048;                              // defaults kept: smoothingTimeConstant 0.8, minDecibels -100, maxDecibels -30
source.connect(this.analyser);                             // NOT connected to destination (no monitoring)
const data = new Uint8Array(this.analyser.frequencyBinCount);   // 1024
const tick = () => {
  if (!this.analyser || !this._isRecording) return;        // loop ends naturally
  this.analyser.getByteFrequencyData(data);
  let sum = 0; for (let i = 0; i < data.length; i += 1) sum += data[i];
  const level = Math.min(1, sum / data.length / 255);
  this.recordingProgress.emit(level);
  this.monitorFrame = window.requestAnimationFrame(tick);
};
this.monitorFrame = window.requestAnimationFrame(tick);
```

Notes:

- The level is the mean of all 1024 byte-frequency bins divided by 255. It is typically low, around 0–0.3 for speech, because high bins stay near 0.
- It is emitted about once per animation frame (~60 Hz), and not in background tabs.
- The AudioContext is created after an `await`, outside the gesture. If it starts suspended, `getByteFrequencyData` returns zeros and the meter stays flat while recording still works.

### 4.9 `cleanupRecordingGraph()` (private)

1. Cancel the rAF.
2. `void audioContext.close()` (not awaited), then null it.
3. Null `analyser`.
4. `track.stop()` for every track, then null `mediaStream`.
5. Null `mediaRecorder`.

### 4.10 `async transcribeData(audioData: Blob): Promise<void>`

```ts
if (!this._enabled) { this.transcriptionError.emit("STT is not enabled"); return; }
try {
  const formData = new FormData();
  formData.append("audio", audioData, "recording.webm");
  formData.append("provider", this._provider);
  formData.append("language", this._language);          // raw setting, NOT resolveSpeechLanguage
  formData.append("serverUrl", this._serverUrl);
  const response = await fetch("/amadeus/app/api/stt/transcribe", { method: "POST", body: formData }); // UA sets multipart boundary
  const data = (await response.json().catch(() => null)) as { text?: string; error?: string } | null;
  if (!response.ok) { this.transcriptionError.emit(data?.error ?? `Transcription failed (${response.status})`); return; }
  this.transcriptionComplete.emit(data?.text ?? "");
} catch (error) {
  this.transcriptionError.emit(error instanceof Error ? error.message : "Transcription failed");
}
```

Notes:

- An OK response with an empty or missing `text` emits `""`. ChatPanel then shows nothing at all: no status and no input change.
- A non-string `text`, such as a number, makes the ChatPanel handler throw on `.trim()`. That exception is caught here and re-emitted as `transcriptionError` (Q-S2).

V1 transcribe route (`stt/transcribe/route.ts`):

- **Validation.**
  - Invalid form data: 400 `{"error":"Invalid form data"}`.
  - The route reads `audio`, `provider` (default `"groq"`), `language` (default `"ja"`) and `serverUrl` (trimmed).
  - No serverUrl: 400 `{"error":"serverUrl is required"}`.
  - `audio` not a Blob: 400 `{"error":"audio file is required"}`.
- **Upstream request.** `POST ${serverUrl}/transcribe` as multipart: `audio` (same blob, filename `"recording.webm"`), `provider` and `language`. The `serverUrl` field is not forwarded. There is no timeout.
- **Upstream error.** When the upstream is not OK: `{"error": body?.error ?? "Transcription failed (<status>)"}` with the upstream status.
- **Success.** The upstream JSON, or `{}` if it was not JSON.
- **Exception.** 500 `{"error": e.message || "Transcription failed"}`.

### 4.11 `resolveSpeechLanguage(language)` (private)

```ts
const normalized = language.trim().toLowerCase();
if (normalized === "ja") return "ja-JP";
if (normalized === "en") return "en-US";
return language || "ja-JP";
```

| Input | Output |
|---|---|
| `"ja"`, `" JA "` | `ja-JP` |
| `"en"`, `"EN"` | `en-US` |
| `"zh"` | `zh` |
| `"ja-JP"` | `ja-JP` |
| `"en-GB"` | `en-GB` |
| `""` | `ja-JP` |
| `"  "` | `"  "` (passes through untrimmed) |

### 4.12 `async checkServerHealth()` (STT)

1. Send `POST /amadeus/app/api/stt/health` with `{"Content-Type":"application/json"}` and body `{"serverUrl": _serverUrl}`.
2. `data = json || null`.
3. `next = response.ok && Boolean(data?.available)`. On change, emit `serverAvailableChanged`.
4. `catch`: if it was available, set false and emit.

The V1 route is identical to the TTS health route (5000 ms timeout; `{available:false,error}` on failure), except that it checks `!response.ok` before parsing the body.

### 4.13 State machine (both paths)

```
Idle ──startRecording()──► [enabled? no → error "STT is not enabled"]
  │                         [SpeechRecognition ctor?] ──yes──► Recognizing (isRecording=true, progress 0.5, 30s timer, recognition.start())
  │                                                             ├─ onresult → transcriptionComplete | error "No speech detected"
  │                                                             ├─ onerror  → error "Speech recognition failed: <code>"
  │                                                             ├─ stopRecording() → isRecording=false, emit, recognition.stop()
  │                                                             └─ onend → (isRecording=false+emit if still true), clear timer, progress 0 → Idle
  │                         [no ctor; no mediaDevices] → error "Microphone is not supported in this browser"
  └─ getUserMedia → Recording(MediaRecorder) (isRecording=true, emit, rAF meter, 30s timer)
        stopRecording()/30s → isRecording=false, emit → recorder.stop() → Finalizing
        onstop → blob, cleanup → size<1024 ? error "Recording too short" : Transcribing (fetch; isRecording already false)
        Transcribing → transcriptionComplete(text) | transcriptionError(msg) → Idle
```

Ways recording stops in V1: the mic button toggle, the 30 s timer, the `enabled=false` setter, ChatPanel unmount (cleanup calls `STTService.stopRecording()`), and browser endpointing (`SpeechRecognition` only). **There is no other VAD.**

---

## 5. AIService (`lib/amadeus/services/ai-service.ts`)

### 5.1 `buildPayload(messages)` builds a `ChatPayload` (JSON key order preserved)

| # | JSON key | Source | Default | Transform |
|---|---|---|---|---|
| 1 | `messages` | argument | – | `ChatMessage[]` as given (role and content only) |
| 2 | `provider` | `getInt("Config_ApiProvider", 8)` | 8 | number |
| 3 | `apiKey` | `getString("Config_ApiKey_<provider>", "")`, or if empty `getString("Config_ApiKey", "")` | "" | |
| 4 | `model` | `getString("Config_ModelName_<provider>", "")`, or if empty `getString("Config_ModelName", "")` | "" | |
| 5 | `webSearch` | `getInt("Config_WebSearch", 0) === 1` | false | bool |
| 6 | `vertexProject` | `getString("Config_VertexProject", "")` | "" | |
| 7 | `vertexLocation` | `getString("Config_VertexLocation", "us-central1")` | "us-central1" | |
| 8 | `localProxyHost` | `getString("Config_LocalProxyHost", "127.0.0.1")` | "127.0.0.1" | No UI writes it |
| 9 | `localProxyPort` | `getString("Config_LocalProxyPort", "")` | "" | No UI writes it |
| 10 | `useGCloudCLI` | `getInt("Config_GCloudCLI", 1) === 1` | true | **exact key casing `useGCloudCLI`**; no UI writes it |
| 11 | `ollamaHost` | `getString("Config_OllamaHost", "http://localhost:11434")` | … | |
| 12 | `openaiCompatible` | `getInt("Config_OpenAICompatible", 0) === 1` | false | bool |
| 13 | `openaiBaseUrl` | `getString("Config_OpenAIBaseUrl", "")` | "" | |

`<provider>` is the integer in decimal. Provider ids (`chat/shared.ts`):

| Id | Provider |
|---|---|
| 0 | OPENAI |
| 1 | GEMINI |
| 2 | CLAUDE |
| 3 | GROQ |
| 4 | VERTEX |
| 5 | OLLAMA |
| 6 | OPENROUTER |
| 7 | LOCAL |
| 8 | FREE |

### 5.2 Signals

| Signal | Payload | V1 subscriber |
|---|---|---|
| `responseReceived` | `string` | ChatPanel: `updateStatus(latency)`, then `handleApiSuccess(response)` |
| `streamToken` | `string` | ChatPanel: `handleStreamToken(token)` |
| `streamComplete` | `string` | ChatPanel: `handleStreamComplete(response)` |
| `errorOccurred` | `string` | ChatPanel: `handleApiError(error)` (mapping in §7.6) |

### 5.3 `async sendChat(messages): Promise<void>` (non-streaming, NOT abortable)

```ts
const payload = buildPayload(messages);
try {
  const response = await fetch("/amadeus/app/api/chat", { method: "POST",
      headers: { "Content-Type": "application/json" }, body: JSON.stringify(payload) });
  const data = (await response.json().catch(() => null)) as { response?: string; error?: string } | null;
  if (!response.ok) { this.errorOccurred.emit(data?.error ?? `Request failed (${response.status})`); return; }
  this.responseReceived.emit(data?.response ?? "");
} catch (error) {
  this.errorOccurred.emit(error instanceof Error ? error.message : "Unknown error");
}
```

Notes:

- There is no AbortController and no client timeout.
- A throwing `responseReceived` subscriber becomes `errorOccurred(message)` (Q-S3).
- The V1 server returns `{"response": string}` on success. On error it returns `{"error": string}` with the status. FreeAPI rate limiting is the exception: it returns HTTP 200 with a `[SAD] …` fallback `response`.

### 5.4 `async sendChatStreaming(messages): Promise<void>` (SSE over fetch)

```ts
this.cancelStreaming();                       // aborts any previous stream (silently)
const payload = buildPayload(messages);
const abortController = new AbortController();
this.streamAbortController = abortController;
try {
  const response = await fetch("/amadeus/app/api/chat/stream", { method: "POST",
      headers: { "Content-Type": "application/json" }, body: JSON.stringify(payload), signal: abortController.signal });
  if (!response.ok || !response.body) {
    const data = (await response.json().catch(() => null)) as { error?: string } | null;
    this.errorOccurred.emit(data?.error ?? `Request failed (${response.status})`);
    return;
  }
  const reader = response.body.getReader();
  const decoder = new TextDecoder();           // utf-8, fatal:false (invalid → U+FFFD)
  let buffer = ""; let full = "";
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    let lineBreak = buffer.indexOf("\n");
    while (lineBreak >= 0) {
      const line = buffer.slice(0, lineBreak).trim();
      buffer = buffer.slice(lineBreak + 1);
      if (line.startsWith("data: ")) {
        const payloadRaw = line.slice(6).trim();
        if (payloadRaw) {
          try {
            const event = JSON.parse(payloadRaw) as { type?: "token"|"complete"|"error"; token?: string; fullResponse?: string; error?: string };
            if (event.type === "token" && event.token) { full += event.token; this.streamToken.emit(event.token); }
            else if (event.type === "complete") { this.streamComplete.emit(event.fullResponse ?? full); this.streamAbortController = null; return; }
            else if (event.type === "error") { this.errorOccurred.emit(event.error ?? "Streaming error"); this.streamAbortController = null; return; }
          } catch {
            full += payloadRaw;
            this.streamToken.emit(payloadRaw);
          }
        }
      }
      lineBreak = buffer.indexOf("\n");
    }
  }
  this.streamComplete.emit(full);
} catch (error) {
  if (!abortController.signal.aborted) this.errorOccurred.emit(error instanceof Error ? error.message : "Unknown streaming error");
} finally {
  if (this.streamAbortController === abortController) this.streamAbortController = null;
}
```

**Server framing.** Every event is `data: <JSON>\n\n` (`stream/route.ts` L21–24), with response headers `Content-Type: text/event-stream; charset=utf-8`, `Cache-Control: no-cache, no-transform` and `Connection: keep-alive`.

Event shapes:

- `{"type":"token","token":"…"}`
- `{"type":"complete","fullResponse":"…"}`
- `{"type":"error","error":"…"}`

Pre-stream errors are plain JSON:

- 400 `{"error":"Invalid request payload"}`.
- 429 `{"error": "<rate limit msg>" | "Too many requests. Please wait a minute."}`. FreeAPI is the exception: when rate-limited it gets an SSE token plus complete carrying `"[SAD] ごめんなさい、リクエストが多すぎるみたい……。少し休ませてからまた話しかけてね。"`.

**Parser rules** (exact, including quirks):

1. **Lines.** Lines are split on `\n` only. Each line is JS-trimmed, so `\r` and surrounding spaces disappear.
2. **Prefix.** Only lines starting with exactly `data: ` (with one U+0020) are considered. `data:foo` (no space), `event:`, `id:`, `retry:` and `: comment` lines are ignored. Multi-line `data:` fields are **not** joined; each line is its own payload.
3. **Empty payloads.** `payloadRaw` is trimmed; an empty value is ignored.
4. **Parse failure.** If `JSON.parse` throws, the **raw text is treated as a token**: it is appended to `full` and emitted. `data: [DONE]` therefore yields the token `"[DONE]"`. The V1 server never sends `[DONE]` to the client.
5. **Non-object JSON.** `JSON.parse("null")` gives `null`, and `null.type` throws, so the raw `"null"` becomes a token. Numbers, strings and arrays give `.type === undefined` and are ignored.
6. **Tokens.** The token must be truthy: `""` is ignored. Token text inside the JSON is **not** trimmed.
7. **`complete`.** `fullResponse ?? full`: an empty-string `fullResponse` is used as `""`. After `complete`, the remaining buffered data is ignored and the function returns; the reader is not cancelled explicitly.
8. **`error`.** `error ?? "Streaming error"`.
9. **Unknown `type`.** Ignored.
10. **End of stream** without `complete` or `error` emits `streamComplete(full)`. A trailing partial line without `\n` is **discarded**, and the decoder is not flushed.
11. **Abort.** Abort makes `reader.read()` or `fetch` reject. When `signal.aborted` is set the rejection is **silent**: no complete and no error. In the JS task model no token is emitted after `cancelStreaming()` returns.
12. **Handler exceptions** thrown inside the inner `try` re-emit the raw JSON as a token (Q-S3). This is irrelevant in Rust.

**Test vectors** (input chunks → emitted signals):

| # | Chunks | Signals |
|---|---|---|
| 1 | `data: {"type":"token","token":"Hel"}\n\n`, `data: {"type":"token","token":"lo"}\n\ndata: {"type":"complete","fullResponse":"Hello"}\n\n` | Token "Hel", Token "lo", Complete "Hello" |
| 2 | `data: {"type":"tok`, `en","token":"A"}\n`, `\n` then EOF | Token "A", Complete "A" |
| 3 | `data: plain text\n` then EOF | Token "plain text", Complete "plain text" |
| 4 | `data: {"type":"error","error":"boom"}\n` | Error "boom" |
| 5 | `data: {"type":"error"}\n` | Error "Streaming error" |
| 6 | Tokens "a", "b", then `data: {"type":"complete"}\n` | …, Complete "ab" |
| 7 | Tokens "a", then `data: {"type":"complete","fullResponse":""}\n` | …, Complete "" |
| 8 | `: ping\n`, `event: x\n`, `data:{"type":"token","token":"x"}\n` then EOF | Complete "" |
| 9 | `data: {"type":"token","token":""}\n` then EOF | Complete "" |
| 10 | `data: [DONE]\n` then EOF | Token "[DONE]", Complete "[DONE]" |
| 11 | `data: {"type":"token","token":"x"}` (no `\n`) then EOF | Complete "" |
| 12 | `data: null\n` then EOF | Token "null", Complete "null" |
| 13 | `  data: {"type":"token","token":" a "}  \r\n` | Token " a " |

### 5.5 `cancelStreaming(): void`

If there is no controller, return. Otherwise `abort()` it and set it to `null`. Callers:

- the start of `sendChatStreaming`;
- ChatPanel `Ctrl+C` while `waitingAPI`;
- the mobile "キャンセル" button;
- ChatPanel unmount.

It **does not affect `sendChat`**, which uses no controller.

### 5.6 Error strings (verbatim)

- `Request failed (<status>)`
- `"Unknown error"` (non-Error throw in `sendChat`)
- `"Streaming error"`
- `"Unknown streaming error"`
- Otherwise the browser `TypeError` messages: "Failed to fetch", "NetworkError when attempting to fetch resource.", "Load failed".

The server-side strings that commonly reach the UI include `"API Key が設定されていません。CONFIGから設定してください。"`. The backend spec owns them.

---

## 6. Live2DCanvas (`components/amadeus/Live2DCanvas.tsx`)

### 6.1 Types

```ts
type Live2DCanvasProps = { emotion: string; lipSyncValue: number; spokenChar?: string; eyeTrackingEnabled?: boolean };
type CubismPlayer = {
  setEmotion: (emotion: string) => void;
  setLipSync: (value: number) => void;
  setSpokenChar?: (char: string) => void;
  setEyeTracking?: (x: number, y: number) => void;
  destroy: () => void;
};
type CubismRuntime = { create: (host: HTMLElement) => CubismPlayer; preload?: () => Promise<void> };
declare global { interface Window { ReamaCubismRuntime?: CubismRuntime } }
```

### 6.2 Constants

| Name | Value |
|---|---|
| `CORE_SCRIPT_SRC` | `"/resources/live2d/live2dcubismcore.min.js"` |
| `RUNTIME_SCRIPT_SRC` | `"/resources/live2d/reama-cubism-runtime.js?v=19"` (the `v=` value is a cache buster) |
| Script marker attributes | `data-core="true"`, `data-runtime="true"`; `data-loaded="true"` once loaded |
| Canvas-style retry | one `setTimeout(…, 100)` |

### 6.3 Script loading

```ts
let scriptsLoadPromise: Promise<void> | null = null;      // module-level singleton

function loadScript(src: string, marker: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const attr = `data-${marker}`;
    const existing = document.querySelector<HTMLScriptElement>(`script[${attr}="true"]`);
    if (existing) {
      if (existing.getAttribute("data-loaded") === "true") { resolve(); return; }
      existing.addEventListener("load", () => resolve(), { once: true });
      existing.addEventListener("error", () => reject(new Error(`Failed to load ${src}`)), { once: true });
      return;
    }
    const script = document.createElement("script");
    script.src = src; script.async = true;
    script.setAttribute(attr, "true");
    script.onload = () => { script.setAttribute("data-loaded", "true"); resolve(); };
    script.onerror = () => reject(new Error(`Failed to load ${src}`));
    document.head.appendChild(script);
  });
}

function ensureLive2DScriptsLoaded(): Promise<void> {
  if (typeof window === "undefined") return Promise.resolve();
  if (!scriptsLoadPromise) {
    scriptsLoadPromise = Promise.all([ loadScript(CORE_SCRIPT_SRC, "core"), loadScript(RUNTIME_SCRIPT_SRC, "runtime") ])
      .then(() => {})
      .catch((err) => { scriptsLoadPromise = null; throw err; });
  }
  return scriptsLoadPromise;
}

export function preloadLive2D(): Promise<void> {
  return ensureLive2DScriptsLoaded()
    .then(() => window.ReamaCubismRuntime?.preload?.())
    .catch((err) => { console.warn("Live2D preload warning:", err); });   // never rejects
}
```

- `preloadLive2D()` is called once from the root mount effect (RealAmadeusApp L3974), while the user is still on the login/boot screens.
- **Runtime preload** (`preloadDefaultAssets`, runtime L18605–18633) warms in-memory fetch caches (`Map<url, Promise>`). Failed entries are removed so they can be retried. The runtime also calls it itself at script evaluation (L18642). The files:
  - `/resources/models/AmadeusKurisu5.0/reama5.0/Live2D紅莉栖forSDK5.0.model3.json`
  - `…/Live2D紅莉栖forSDK5.0.moc3`
  - `…/Live2D紅莉栖forSDK5.0.physics3.json`
  - `…/Live2D紅莉栖forSDK5.0.8192/texture_00.png` (an `Image` with `decoding="async"`, then `decode()`)
  - 13 shaders under `/resources/live2d/shaders/`: `vertshadersrc.vert`, `vertshadersrcmasked.vert`, `vertshadersrcsetupmask.vert`, `fragshadersrcsetupmask.frag`, `fragshadersrcpremultipliedalpha.frag`, `fragshadersrcmaskpremultipliedalpha.frag`, `fragshadersrcmaskinvertedpremultipliedalpha.frag`, `vertshadersrccopy.vert`, `fragshadersrccopy.frag`, `fragshadersrccolorblend.frag`, `fragshadersrcalphablend.frag`, `vertshadersrcblend.vert`, `fragshadersrcpremultipliedalphablend.frag`

  Each fetch's errors are swallowed.

### 6.4 Component lifecycle

**Refs**, initialized from the first props:

- `hostRef`
- `playerRef = null`
- `emotionRef = emotion.toUpperCase()`
- `lipSyncRef = lipSyncValue`
- `spokenCharRef = spokenChar || ""`

**Mount effect (`[]`)**, in this order:

1. `disposed = false`, `h = hostRef.current`. If `!h`, return without registering a cleanup.
2. An async IIFE:
   1. `await ensureLive2DScriptsLoaded()`. On a throw, `console.error("Cubism init failed", e)` and return.
   2. If `disposed`, return.
   3. `rt = window.ReamaCubismRuntime`. If it is missing, return silently.
   4. `p = rt.create(h)`, then `playerRef.current = p`.
   5. `p.setEmotion(emotionRef.current)`, then `p.setLipSync(lipSyncRef.current)`, then `p.setSpokenChar?.(spokenCharRef.current)`. The refs hold the **latest** prop values because the prop effects update them even while the player is null.
   6. `applyCanvasStyles()`: find `h.querySelector("canvas")`. If found, set `style.pointerEvents = "auto"`, `style.width = "100%"`, `style.height = "100%"` and return true; otherwise return false.
   7. If it returned false, call `setTimeout(applyCanvasStyles, 100)` exactly once. The timer is not cleared on unmount. Normally the canvas exists synchronously because the player constructor appends it, so the first call succeeds.
   8. The eye-tracking state is **not** pushed at create. The player starts at `(0,0)`, and the next mousemove updates it.
3. **Cleanup**, in exactly this order (it captures the GL context *before* `destroy()` empties the host):

   ```ts
   disposed = true;
   const c = h.querySelector("canvas");
   let gl = null;
   if (c) gl = c.getContext("webgl") || c.getContext("webgl2");  // returns the existing context (runtime creates "webgl" first, then falls back to "webgl2");
                                                                   // if none existed yet this CREATES a WebGL1 context, which is then immediately lost (harmless)
   try { playerRef.current?.destroy(); } catch (e) { console.warn("Live2D destroy error:", e); }
   playerRef.current = null;
   if (gl) { const ext = gl.getExtension("WEBGL_lose_context"); if (ext) ext.loseContext(); }
   h.innerHTML = "";
   ```

   `loseContext()` is essential. The runtime's `LAppGlManager.release()` is a no-op, and browsers cap live WebGL contexts at about 16. ChatPanel remounts whenever the layout switches between mobile and desktop (§6.7).

**Prop effects**, which run after commit in declaration order and only for changed deps:

```ts
useEffect(() => { emotionRef.current = emotion.toUpperCase(); playerRef.current?.setEmotion(emotionRef.current); }, [emotion]);
useEffect(() => { lipSyncRef.current = lipSyncValue;        playerRef.current?.setLipSync(lipSyncRef.current); }, [lipSyncValue]);
useEffect(() => { spokenCharRef.current = spokenChar || ""; playerRef.current?.setSpokenChar?.(spokenCharRef.current); }, [spokenChar]);
```

React compares deps with `Object.is`, so setting the same value again does not re-run the effect. That is harmless because the player ignores identical emotions anyway.

### 6.5 Eye tracking (exact mapping)

```ts
useEffect(() => {
  if (!eyeTrackingEnabled) { playerRef.current?.setEyeTracking?.(0, 0); return; }
  const handleMouseMove = (e: MouseEvent) => {
    const nx = ((e.clientX / window.innerWidth) - 0.5) * 2.0;
    const ny = ((e.clientY / window.innerHeight) - 0.5) * 2.0;
    const eyeX = Math.max(-0.7, Math.min(0.7, nx));
    const eyeY = -Math.max(-0.7, Math.min(0.7, ny));      // screen Y down → model Y up
    playerRef.current?.setEyeTracking?.(eyeX, eyeY);
  };
  window.addEventListener("mousemove", handleMouseMove);
  return () => window.removeEventListener("mousemove", handleMouseMove);
}, [eyeTrackingEnabled]);
```

- **Coordinates.** The mapping uses the **viewport** (`clientX/innerWidth`), not the canvas rect. The stage transforms (desktop `scale(uiScale)`, the `.system-panel` shrink while the menu is open, the minigame shift) are ignored.
- **Clamp.** Both axes are clamped to ±0.7, and Y is inverted. When toggled off, the cleanup removes the listener and then `setEyeTracking(0,0)` is sent, so the gaze eases back to centre.
- **Input sources.** Only `mousemove` is handled, so touch devices get at most emulated mouse events on tap.
- **Source of `eyeTrackingEnabled`.** It comes from `useAppSettingInt("Config_EyeTracking", 0) === 1`, which is reactive through `settingsChanged`. The default is `0`, and Settings writes it with `setInt`.

Test vectors for a 1920×1080 viewport, `(clientX, clientY) → (x, y)`:

| (clientX, clientY) | (x, y) |
|---|---|
| (0, 0) | (-0.7, 0.7) |
| (960, 540) | (0, -0) |
| (1920, 1080) | (0.7, -0.7) |
| (1440, 270) | (0.5, 0.5) |
| (480, 810) | (-0.5, -0.5) |

### 6.6 What the player does with each value (contract for `amadeus-live2d`)

The runtime's `ReamaCubismPlayer` is at runtime L18075–18604. The live2d spec owns its internals; the methods Live2DCanvas calls behave as follows.

```js
setEmotion(emotion) {                 // trim + uppercase; same → ignored
  const normalizedTag = emotion.trim().toUpperCase();
  if (normalizedTag === this.emotion) return;
  const isWakingUp = (this.emotion === "SLEEPING" && normalizedTag !== "SLEEPING");
  this.emotion = normalizedTag;
  if (isWakingUp) { this.wakeUpTimer = 2.5; this.activeBurstTag = "SURPRISED"; this.burstTimer = 0; this.burstProgress = 0.0; this.idlePhase = 0.0; this.currentEmotionTag = "SURPRISED"; }
  else { this.wakeUpTimer = 0; if (normalizedTag !== this.currentEmotionTag) { this.activeBurstTag = normalizedTag; this.burstTimer = 0; this.burstProgress = 0.0; this.idlePhase = 0.0; this.currentEmotionTag = normalizedTag; } }
}
setSpokenChar(spokenChar) { this.spokenChar = spokenChar || ""; }
setLipSync(value)        { this.lipSyncValue = Math.max(0, Math.min(1, value)); }
setEyeTracking(x, y)     { this.eyeTrackingX = Math.max(-1.0, Math.min(1.0, x)); this.eyeTrackingY = Math.max(-1.0, Math.min(1.0, y)); }
destroy()                { /* destroyed=true; cancel init-retry rAF and tick rAF; setParameterDriver(null); subdelegate.release(); host.innerHTML=""; releaseFramework() (ref-counted CubismFramework) */ }
```

**Mouth** (per frame in `applyParameters`, L18552–18570; `dt` is capped at 0.1 s):

```js
if (isSneezeActive) this.mouthOpen = lerp(this.mouthOpen, sneezeMouthOpen, 16.0 * dt);
else if (this.lipSyncValue > 0.01) {
  let targetMouth = 0.5;
  if (this.spokenChar) targetMouth = getVowelMouthOpening(this.spokenChar);
  else {
    const syllable = Math.abs(Math.sin(this.userTime * 12));
    const subBeat  = Math.abs(Math.sin(this.userTime * 7.3));
    const flutter  = noise01(this.userTime * 8.0, 5.0);
    targetMouth = syllable * 0.5 + subBeat * 0.3 + flutter * 0.2;
  }
  const finalMouth = clamp01(targetMouth * clamp01(this.lipSyncValue));
  this.mouthOpen = lerp(this.mouthOpen, finalMouth, 14.0 * dt);
} else this.mouthOpen = lerp(this.mouthOpen, 0.0, 10.0 * dt);
setParam(ParamMouthOpenY, this.mouthOpen);
```

`noise01(x, seed) = clamp(0.5 + sin(x*1.37+seed*0.71)*0.28 + sin(x*2.11+seed*1.13)*0.16 + sin(x*0.73+seed*2.03)*0.06, 0, 1)`.

`getVowelMouthOpening(chr)` looks only at `chr.charAt(0)` (verbatim tables):

| Characters | Opening |
|---|---|
| empty string | 0.0 |
| `a`, `A` | 0.9 |
| `o`, `O` | 0.7 |
| `e`, `E` | 0.6 |
| `u`, `U` | 0.4 |
| `i`, `I`, `í`, `Í` | 0.2 |
| `あかさたなはまやらわがざだばぱぁゃァカサタナハマヤラワガザダバパァャ` | 0.9 |
| `おこそとのほもよろをごぞどぼぽぉょォコソトノホモヨロヲゴゾドボポォョ` | 0.7 |
| `えけせてねへめれげぜでべぺぇェケセテネヘメレゲゼデベペェ` | 0.6 |
| `うくすつぬふむゆるぐずづぶぷぅゅゥクスツヌフムユルグズヅブプゥュ` | 0.4 |
| `いきしちにひみりぎじぢびぴぃィキシチニヒミリギジヂビピィ` | 0.2 |
| `んン。、？！.,?! 　\n\r\t-_` | 0.0 (the set includes an ASCII space and U+3000) |
| anything else (kanji, other letters, digits) | 0.5 |

**Gaze** (L18529–18546):

- `disableGaze = wakeUpTimer > 0 || emotion === "SLEEPING" || currentEmotionTag === "SLEEPING"`.
- `targetX` is `wakeUpGazeX` while waking up, else `0` if disabled, else `eyeTrackingX || 0`. The `|| 0` turns NaN into 0.
- `targetY` is `0` if disabled, else `eyeTrackingY || 0`.
- Smoothing: `smoothedGaze = lerp(smoothedGaze, target, clamp(dt*5, 0, 1))`.
- Applied parameters:
  - `ParamEyeBallX/Y = smoothedGaze`.
  - `ParamAngleX/Y += smoothedGaze*25`, so the maximum offset is ±17.5 at 0.7.
  - `ParamBodyAngleX/Y += smoothedGaze*5`.

**Sneeze easter egg.** It only arms when `emotion === "NORMAL"` and `lipSyncValue <= 0.01` (plus no burst and no wake-up): every 45 s, with a 15 % chance. Lip-sync therefore also suppresses it.

**Line ranges to copy for the live2d port:**

| Content | Runtime lines |
|---|---|
| `EMOTION_TARGETS` | 17690–17872 |
| `lerp`, `qBound`, `noise01`, `noiseSigned`, `drift` | 17874–17894 |
| `MOTION_BURSTS` | 17895–17909 |
| `getEmotionIdleMotion` | 17910–18010 |
| framework refcount | 18011–18033 |
| `findExpressionName` | 18034–18074 |

### 6.7 DOM and CSS

**Rendered markup:** `<div className="live2d-host" ref={hostRef} />`.

The player then appends `<canvas style="display:block; width:100%; height:100%; pointer-events:none">`, and Live2DCanvas overrides the inline style to `pointer-events:auto; width:100%; height:100%`. The canvas backing store is `clientWidth/Height × devicePixelRatio`, re-applied through a ResizeObserver (`CanvasSize = "auto"`).

**Parent (ChatPanel L1864):** `<div className={`live2d-shift${miniGameOpen ? " live2d-shifted" : ""}`}>`, which contains the `Live2DCanvas`. It sits inside `.chat-panel-root`.

**CSS (`globals.css`, to be reused verbatim):**

```css
.chat-panel-root { position: absolute; inset: 35px 0 0 0; animation: chat-fade-in 500ms linear; touch-action: manipulation; }
.live2d-host { position: absolute; inset: 0; z-index: 0; }
.live2d-host canvas { position: absolute; left: 50%; top: 50%; width: 3840px; height: 2160px; transform: translate(-50%, -50%); pointer-events: auto; }
/* the inline style width/height:100% wins over 3840x2160 on desktop */
.live2d-shift { position: absolute; inset: 0; transition: transform 450ms cubic-bezier(0.645, 0.045, 0.355, 1); }
.live2d-shift.live2d-shifted { transform: translateX(28%); }
.mobile-root .chat-panel-root { position: absolute; inset: calc(env(safe-area-inset-top, 0px) + 56px) 0 0 0; animation: none; }
.mobile-root .live2d-host { position: fixed; inset: 0; z-index: 0; }
.mobile-root .live2d-host canvas { position: absolute; inset: 0; width: 100% !important; height: 100% !important; object-fit: contain; object-position: center center; transform: none; pointer-events: auto; }
@media (pointer: coarse), (max-width: 900px) {   /* L2586 block */
  .mobile-root .live2d-shift.live2d-shifted { transform: translateY(-14dvh) scale(0.85) !important; transform-origin: top center !important; transition: transform 450ms cubic-bezier(0.645, 0.045, 0.355, 1); }
}
```

**Mobile versus desktop.** RealAmadeusApp returns **different trees**:

- Mobile is `.real-amadeus-root.mobile-root`, chosen when `visualViewport.width <= 900`, or when it is `<= 1200` with `(pointer: coarse)`.
- Desktop is a 1920×1080 `.viewport` with `transform: scale(uiScale)`. Its ChatPanel sits inside `.system-panel`, which uses `transform: translate(150p px, -12p px) scale(1 - 0.4p)`, where `p = 1` while the menu tab is active.

Crossing the breakpoint **unmounts and remounts ChatPanel**. Live2DCanvas then runs destroy, loseContext and create again; ChatPanel's cleanup stops STT and cancels streaming. TTS audio is not stopped.

### 6.8 Log strings (verbatim)

- `"Cubism init failed"` (`console.error`, plus the error)
- `"Live2D preload warning:"` (`console.warn`)
- `"Live2D destroy error:"` (`console.warn`)
- `Failed to load ${src}` (Error message)

---

## 7. Consumer contract: how ChatPanel wires these services (RealAmadeusApp.tsx)

### 7.1 Subscriptions

The effect at L955–1047 has `[]` deps, so it subscribes once per ChatPanel mount; the cleanup disconnects all of them, then calls `STTService.stopRecording()` and `AIService.cancelStreaming()`.

> **V1 stale-closure fact (affects behaviour).** The handlers inside call `handleApiSuccess`, `handleStreamToken`, `handleStreamComplete`, `handleApiError`, `playNextTTSChunkIfPossible` and `processNextTTSChunkIfPossible`, plus the `ttsSessionId` variable, **as captured at ChatPanel's first render**.
>
> - Values read through refs or setters are current.
> - Values read through closure state are frozen: `chatState = "inputReady"`, `fullText.length = 0`, `ttsSessionId = 0`.
>
> Consequences:
>
> - **(a) Streaming providers stay in `waitingAPI`.** `handleStreamToken` and `handleStreamComplete` test `chatState === "waitingAPI"`, which is always false under the frozen value. For providers 1, 3, 4, 5 and 6 the UI therefore **never moves from `waitingAPI` to `typing`** through these handlers. Text and TTS still update. This looks like a V1 bug; see Open questions.
> - **(b) The TTS session guard compares against 0.** It compares `ttsInFlightSessionIdRef` against the frozen value 0, not the current session (§7.3).
>
> Leptos signals read the current value, so a direct port "fixes" both. Decide deliberately.

### 7.2 What Live2DCanvas actually receives (L1865–1870)

```tsx
<Live2DCanvas
  emotion={miniGameOpen ? miniGameEmotion : currentEmotionTag}
  lipSyncValue={miniGameOpen ? miniSpeak : isActivelySpeaking ? 1 : lipSyncValue}
  spokenChar={miniGameOpen ? "" : isActivelySpeaking ? currentSpokenChar : ""}
  eyeTrackingEnabled={eyeTrackingEnabled}
/>
```

**`isActivelySpeaking`** (L950–953), re-evaluated on every render:

```ts
chatState === "typing" && !waitingForInput && ((typeIndex < fullText.length && fullText.length > 0) || ttsPlaybackBusyRef.current)
```

`ttsPlaybackBusyRef` is a ref, so it is only seen when something else re-renders.

**`lipSyncValue` state:** an effect sets it to `1` when `isActivelySpeaking`, else `0` (L1225–1231). The prop is therefore effectively `isActivelySpeaking ? 1 : 0`. On the falling edge it stays at 1 for one extra render, until the effect sets 0.

**Result: binary lip-sync.**

- The mouth moves while text is typing.
- It stops at a pause (`waitingForInput`) and when the text finishes (`waitForAdvance`), **even if TTS audio is still playing**.
- AudioService's lip-sync value is never used.

**`currentSpokenChar`** is updated per typed character by the typewriter (L1141–1177):

- At the start of a page, leading `" "`, `"\n"`, `"\r"` and `"\t"` are skipped.
- The char is `targetText.charAt(i)`, one UTF-16 unit.
- It resets to `""` at pause characters, page end, skip and completion.

**Cadence** (`charDelayForChar(char, Config_TextSpeed)`):

- Base delay `1000/20/max(0.1, speed)` = 50 ms at speed 1.
- ×2.5 for `、` `,` `…`.
- ×1.5 for `」` `）`.

**Pause characters** (`isPauseCharacter(char, next)`):

- `.` pauses only when followed by a space, `\n`, `\r` or the end of text; `..` does not pause.
- `。！？!?\n` pause unless the next char is one of `」 ） ) 』 ”`.

**`miniSpeak`** (MiniGamePanel L2565–2600). `startSpeaking(text)` does the following:

1. Emits `Math.random()` immediately.
2. Emits a new `Math.random()` every **120 ms**.
3. Emits `0` after `ms = max(800, min(3000, text.length * 90))`.

`stopSpeaking()` clears both timers and emits `0`. When the panel is hidden it calls `stopSpeaking()`. `spokenChar` is `""` during the minigame, so the player uses its syllable oscillator scaled by the random value.

**Emotion sources:**

- The 11 tags (`NORMAL SMILE ANGRY SAD SURPRISED BLUSH WINK DISGUST SMUG THINKING PANIC`) via `parseEmotionTag` or the stream prefix.
- `"SLEEPING"` after 60 s idle in `inputReady` or `waitForAdvance`. Any mouse movement (when the position changed), mousedown, keydown, touchstart or scroll reverts it to `NORMAL`. Activity in the first 800 ms after entering sleep is ignored.
- `"ANGRY"` on API error.
- `"NORMAL"` on advance or cancel.
- `/dev emotion <x>` with an arbitrary uppercased string.
- `/dev motion reset`.
- Minigame emotions.
- Random-event emotions.

### 7.3 TTS audio queue (L927–935 and L1337–1509)

**State refs:**

| Ref | Meaning |
|---|---|
| `ttsSessionId` | state, initially 0 |
| `ttsInFlightSessionIdRef` | -1 |
| `ttsTextQueueRef: string[]` | text chunks waiting for synthesis |
| `ttsAudioQueueRef: string[]` | object URLs waiting for playback |
| `ttsSynthInFlightRef` | false |
| `ttsPlaybackBusyRef` | false |
| `ttsChunkBufferRef` | `""` |
| `ttsProcessedIndexRef` | 0 (UTF-16 offset into the display text) |

**Functions (exact):**

- **`prepareLiveTTSSession()`**, called from `submitMessage` before each request:
  1. `setTtsSessionId(ttsSessionId + 1)`, `inFlight = -1`.
  2. Clear both queues; `synthInFlight = false`, `playbackBusy = false`, `chunkBuffer = ""`, `processedIndex = 0`.
  3. `AudioService.stop()`.
- **`cancelLiveTTS()`**, called from `handleApiError`: the same, using the functional update `setTtsSessionId(v => v + 1)`.
- **`processNextTTSChunkIfPossible()`:**
  1. If `!TTSService.enabled || synthInFlight || textQueue.length === 0`, return.
  2. `next = textQueue.shift()`. If `!next?.trim()`, return. This **drops** the chunk without trying the next one.
  3. `synthInFlight = true`, `inFlight = ttsSessionId`, `void TTSService.synthesize(next)`.
- **`playNextTTSChunkIfPossible()`:**
  1. If `playbackBusy`, return.
  2. If the audio queue is empty, call `processNextTTSChunkIfPossible()` and return.
  3. `url = audioQueue.shift()`. If falsy, return.
  4. `playbackBusy = true`, `AudioService.playFromFile(url)`, then `processNextTTSChunkIfPossible()`, which prefetches the next synthesis while this chunk plays.
- **`enqueueTTSChunk(text)`:** `cleaned = text.trim()`. If empty, return. Otherwise push it and call `processNextTTSChunkIfPossible()`.
- **Signal handlers:**
  - `TTSService.synthesisComplete(url)`: if `inFlight !== ttsSessionId(frozen 0)`, return. Otherwise `synthInFlight = false`, push the url, then `playNextTTSChunkIfPossible()`.
  - `TTSService.synthesisError(_)`: same guard, then `synthInFlight = false`, then `processNextTTSChunkIfPossible()`. The failed chunk is skipped.
  - `AudioService.playbackStarted`: `playbackBusy = true`.
  - `AudioService.playbackFinished` and `AudioService.errorOccurred`: `playbackBusy = false`, then `playNextTTSChunkIfPossible()`.

**Splitting text (non-stream path):** `splitTextForTTS(text)`

1. `normalized = text.replace(/\s+/g, " ").trim()`, using the JS whitespace set. Every `\n` therefore becomes a space, so the `"\n"` boundary below can never match (dead condition).
2. Iterate **code points**. Append each to `current`.
3. If the char is in `["。","！","？","!","?","\n"]`, **or** `current.length >= 120` (UTF-16 units), push `current.trim()` if it is non-empty, then reset `current = ""`.
4. At the end, push `current.trim()` if it is non-empty.

Example: `"こんにちは。元気？  ええ!"` gives `["こんにちは。","元気？","ええ!"]`.

**Choosing the path:** `triggerTTSIfEnabled(text, immediate)`

1. If `!TTSService.enabled`, return.
2. `cleaned = stripEmotionTag(text)` (removes `<think>…</think>` and every emotion tag, then trims). If empty, return.
3. If `immediate`, enqueue each piece of `splitTextForTTS(cleaned)`; otherwise enqueue `cleaned` whole.

`handleApiSuccess` calls it as `triggerTTSIfEnabled(display, true)`.

**Streaming path:**

- **`captureStreamTextForTTS(sourceText, complete)`:**
  1. If TTS is disabled, return.
  2. Clamp `processedIndex` to `sourceText.length`.
  3. `delta = sourceText.slice(processedIndex)`. If empty, return.
  4. `processedIndex = sourceText.length`.
  5. Append `delta.replace(/\[(NORMAL|SMILE|ANGRY|SAD|SURPRISED|BLUSH|WINK|DISGUST|SMUG|THINKING|PANIC)\]/gi, "")` to the buffer. The regex runs per delta, so a tag split across deltas survives.
  6. `flushTTSChunkBuffer(complete)`.

  It is called with `(display, false)` per token and `(cleaned, true)` on completion.
- **`flushTTSChunkBuffer(force)`:**
  1. Loop: find the first index `i` whose char is in `["。","！","？","!","?","\n"]`.
  2. Set `boundary = i + 1`, then extend it while the next char is in `["」","』","”","\"",")","）"]`. This closer set has 6 entries, including the ASCII `"`; it differs from `isPauseCharacter`'s set of 5.
  3. If no boundary was found, stop. Otherwise `chunk = buf.slice(0, boundary).trim()` and `buf = buf.slice(boundary).trimStart()`; enqueue the chunk if it is non-empty, then repeat.
  4. If `force`: `tail = buf.trim()`, `buf = ""`, and enqueue the tail if it is non-empty.

  Example: `"「そうね。」それで"` gives the chunk `"「そうね。」"` and leaves `"それで"` in the buffer.

**Ordering properties:**

- At most one synthesis is in flight at a time and at most one clip plays at a time, so chunks play in enqueue order.
- On a cache hit, `synthesize()` emits `synthesisComplete` synchronously inside `processNextTTSChunkIfPossible()`. That recursion goes playNext → processNext → synthesize and is bounded by the queue length. Rust may emit on the next microtask instead; this is not observable because of the in-flight flag. Synchronous emission is still recommended for parity.

**Session guard, intended versus actual.**

- **Intended:** discard synthesis results from previous sessions.
- **Actual V1:** `inFlight` is set to the frozen 0 on every new synthesis, and to -1 only by prepare or cancel. A stale result is dropped only if it arrives before the new session's first synthesis starts. Otherwise it is accepted, pushed into the new session's queue, and clears `synthInFlight` while the new synthesis is pending, which can lead to two concurrent syntheses.

**Not stopped:** `AudioService` is not stopped on ChatPanel unmount, so TTS continues playing.

### 7.4 STT UI (L1907–1955)

Rendered only when `chatState === "inputReady" && !miniGameOpen`:

```tsx
<div className={`input-panel ${showAudioInput ? "has-audio-input" : ""}`}>
  <input … className={`chat-input ${currentEmotionTag === "SLEEPING" ? "sleeping-input" : ""}`} />
  <div className="chat-input-actions">
    {showAudioInput && (
      <button type="button" className={`chat-mic-button ${sttRecording ? "recording" : ""}`} onClick={toggleSttRecording}
              aria-label={t("speech_input", configLanguage === 1 ? "Speech input" : "音声入力", configLanguage)}>
        {sttRecording ? "■" : "🎤"}
      </button>
    )}
    <button type="button" className="chat-send-button" onClick={() => submitMessage(chatInput)} disabled={!chatInput.trim()}>
      {t("send", configLanguage === 1 ? "SEND" : "送信", configLanguage)}
    </button>
  </div>
  {sttRecording && (<div className="chat-mic-level"><span style={{ width: `${Math.round(sttRecordingLevel * 100)}%` }} /></div>)}
  {sttStatusText && <div className="chat-stt-hint">{sttStatusText}</div>}
</div>
```

- `showAudioInput = AppSettings.getInt("Config_AudioInputEnabled", 0) === 1`, read once at mount and never updated.
- The keys `speech_input` and `send` are absent from `ja.json`/`en.json`, so the fallbacks shown are used.
- **`toggleSttRecording`:**
  1. If `chatState !== "inputReady"`, return.
  2. `setSttStatusText("")`.
  3. If `!STTService.enabled`, set `STTService.enabled = true`, which persists the setting and runs a health check.
  4. If it is recording, call `stopRecording()`; otherwise call `void startRecording()`.
- **Status strings:** `"音声入力: OK"` on a non-empty transcript, `"音声入力に失敗しました"` on any error.
- **CSS** (verbatim, desktop):

  ```css
  .chat-mic-button { width: 44px; height: 44px; border: 1px solid rgba(255,153,0,0.65); background: #572014; color: #fff; font-size: 22px; line-height: 1; cursor: pointer; }
  .chat-mic-button.recording { background: #962f1c; border-color: #ff8866; }
  .chat-mic-level { position: absolute; left: 12px; right: 12px; bottom: -6px; height: 3px; background: rgba(255,153,0,0.18); }
  .chat-mic-level span { display: block; height: 100%; background: #ff9900; }
  .chat-stt-hint { position: absolute; left: 12px; bottom: calc(100% + 4px); color: #ffcc88; font-size: 16px; }
  ```

  Mobile overrides: `.mobile-root .input-panel.has-audio-input { padding-right: 152px; }` and `.mobile-root .chat-mic-button { width: 44px; height: 44px; font-size: 22px; border-radius: 22px; }`. There is also a 44px/22px override inside `@media (pointer: coarse), (max-width: 900px)` at L717.

### 7.5 AI dispatch and cancellation

**`submitMessage`** (L1770–1796):

1. Append the user message; `MemoryManager.trimConversationHistory(next, 30)`; refresh the system prompt.
2. `streamRawBufferRef = ""`, `streamEmotionParsedRef = false`, `setStreamComplete(false)`, `setDialogueText("")`, `setWaitingIndicator(".")`, `setChatState("waitingAPI")`.
3. `prepareLiveTTSSession()`.
4. `provider = AppSettings.getInt("Config_ApiProvider", 0)`. The default here is **0**, unlike `buildPayload`'s 8. The key always exists after `initializeDefaults`, so this does not matter in practice.
5. If `provider ∈ {1,3,4,5,6}`, call `AIService.sendChatStreaming(nextMessages)`; otherwise `AIService.sendChat(nextMessages)`.

**`Ctrl+C`** (window keydown; ignored while the menu or minigame is open), when `chatState === "waitingAPI"`:

1. `preventDefault()`.
2. `AIService.cancelStreaming()`.
3. `setChatState("inputReady")`, `setCurrentEmotionTag("NORMAL")`, `setWaitingIndicator("")`, `setChatInput("")`.

The mobile `.cancel-streaming-button` (label `t("cancel_button","キャンセル")`) does the same. On desktop the hint `.pc-cancel-hint` shows `t("cancel_hint","CTRL+Cでキャンセル")`.

**Non-stream cancel bug:** because `sendChat` cannot be aborted, its late response still triggers `handleApiSuccess`, and the UI jumps back into `typing`.

### 7.6 AIService errors to the user message (`handleApiError`, L1654–1695; first match wins, verbatim)

| Condition (`error.includes(...)`) | Message |
|---|---|
| *(default)* | `何かエラーが起きたみたい……もう一度試してくれる？` |
| `"API Key"` \| `"API key"` \| `"設定されていません"` | `……APIキーが設定されてないみたいよ。CONFIGから設定してちょうだい。` |
| `"401"` \| `"Unauthorized"` | `APIキーが無効みたい……もう一度確認して設定し直してくれる？` |
| `"429"` \| `"rate limit"` \| `"RESOURCE_EXHAUSTED"` | `リクエストが多すぎるみたい。少し待ってからもう一度試してくれない？` |
| `"insufficient_quota"` \| `"quota_project"` | `Geminiの利用枠か課金設定に問題があるみたい。APIキーとプロジェクト設定を確認して。` |
| `"timeout"` \| `"Timeout"` | `応答がタイムアウトしたわ……ネットワークの状態を確認してみて。` |
| `"Cannot connect"` \| `"Network"` \| `"ネットワーク"` | `ネットワークに接続できないわ。インターネット接続を確認してちょうだい。` |
| `"403"` \| `"Forbidden"` | `このAPIへのアクセスが拒否されたわ。権限を確認してみて。` |
| `"500"` \| `"502"` \| `"503"` | `サーバー側でエラーが起きてるみたい。しばらくしてからもう一度試して。` |
| `"model"` \| `"Model"` | `指定されたモデルが見つからないみたい。CONFIGからモデル名を確認して。` |
| `"JSON"` | `サービスアカウントJSONキーの形式が正しいか確認して。` |
| `"Vertex"` \| `"アクセストークン"` | `Vertexの認証情報を確認して。APIキーかサービスアカウントJSONキーが必要よ。` |

After matching, `handleApiError` also:

1. Sets the emotion to `ANGRY` and shows the message in the typing state.
2. Calls `cancelLiveTTS()`.

Note that Chromium's network error `"Failed to fetch"` contains none of the keywords and falls through to the default. Firefox's `"NetworkError…"` matches "Network". `Request failed (502)` matches "502".

### 7.7 Startup and settings calls

- **Root mount (L3971–3977):**
  1. `AppSettings.initializeDefaults()`
  2. `preloadLive2D()`
  3. `TTSService.checkServerHealth()`
  4. `STTService.checkServerHealth()`
  5. `VectorDatabase.checkServerHealth()`

  Health checks run even when the feature is disabled.
- **Settings panel visible (L3191):** `TTSService.checkServerHealth()`.
- **Settings Apply (L3292–3313):** writes the keys, then calls the TTS/STT setters, which are no-ops as explained in §3.6. The STT-specific consequence is in §4.4.
- **Not displayed anywhere:** `serverAvailable`, `usingGpu`, `runtimeDevice` and `runtimeDetails` are shown nowhere in the V1 UI.

---

## 8. Client-visible HTTP contracts (the V2 Web transport must keep them; the desktop transport mirrors them)

| Client call | Method and path | Request | Success response | Error response |
|---|---|---|---|---|
| `AIService.sendChat` | `POST /amadeus/app/api/chat` | `application/json` `ChatPayload` (§5.1) | 200 `{"response": string}` | non-2xx `{"error": string}` |
| `AIService.sendChatStreaming` | `POST /amadeus/app/api/chat/stream` | same, with an AbortSignal | 200 `text/event-stream; charset=utf-8`, frames `data: {json}\n\n` (§5.4) | 400/429 JSON `{"error"}`; mid-stream `{"type":"error"}` events |
| `TTSService.synthesize` | `POST /amadeus/app/api/tts/synthesize` | JSON `{"serverUrl","speaker","text","useCache":true}` | 200 audio bytes, `Content-Type` = upstream or `audio/wav`, `Cache-Control: no-store` | 400/upstream/500 `{"error"}` |
| `TTSService.checkServerHealth` | `POST /amadeus/app/api/tts/health` | JSON `{"serverUrl"}` | 200 `{"available":true, ...upstreamBody}` | 400 `{"available":false,"error"}`; 200 `{"available":false,"error"}` |
| `STTService.transcribeData` | `POST /amadeus/app/api/stt/transcribe` | `multipart/form-data`: `audio` (Blob, filename `recording.webm`, type = recorder mime), `provider`, `language`, `serverUrl` (in this order) | 200 upstream JSON (expects `{"text"}`) or `{}` | 400/upstream/500 `{"error"}` |
| `STTService.checkServerHealth` | `POST /amadeus/app/api/stt/health` | JSON `{"serverUrl"}` | 200 `{"available":true,...}` | as TTS |

Upstream contracts, which the desktop backend calls directly:

- **TTS:** `POST {serverUrl}/synthesize` with JSON `{text, speaker, use_cache}`, returning audio bytes. `GET {serverUrl}/health` with a 5 s timeout.
- **STT:** `POST {serverUrl}/transcribe` as multipart `{audio:"recording.webm", provider, language}`, returning `{text}` or `{error}`. `GET {serverUrl}/health` with a 5 s timeout.

> **Semantic difference to document for users.** In the hosted **Web** version, the *server* contacts `serverUrl`, so `http://localhost:5100` means the web server's localhost, not the user's PC. In the **Desktop** version, the backend runs on the user's PC, so the localhost URLs reach the user's local TTS/STT servers.

---

## 9. Timing and numeric constants

| Constant | Value | Where |
|---|---|---|
| Lip-sync tick | 50 ms `setInterval` | AudioService |
| Lip-sync EMA α | 0.3 | AudioService |
| Lip-sync wave terms | sin(t·8)·0.3+0.5, sin(t·15)·0.15, sin(t·3)·0.1, noise (rand−0.5)/5 | AudioService |
| Max recording | 30 000 ms | STTService (both paths) |
| MediaRecorder timeslice | 250 ms | STTService |
| Minimum recording size | 1024 bytes | STTService |
| Analyser fftSize | 2048 (1024 bins) | STTService |
| Meter rate | requestAnimationFrame (~60 Hz) | STTService |
| Browser-recognition meter | 0.5 constant, 0 at end | STTService |
| Health timeout | 5000 ms (server-side only; the client has none) | API routes |
| Canvas style retry | 100 ms, once | Live2DCanvas |
| Eye clamp | ±0.7, Y inverted | Live2DCanvas |
| Player clamps | lipSync [0,1], eye [−1,1] | runtime |
| Player mouth lerp rates | 14·dt (speaking), 10·dt (closing), 16·dt (sneeze) | runtime |
| Player syllable oscillator | abs(sin(t·12))·0.5 + abs(sin(t·7.3))·0.3 + noise01(t·8, 5)·0.2 | runtime |
| Gaze smoothing | clamp(dt·5) ; ×25 head, ×5 body | runtime |
| Wake-up | 2.5 s; blend over the last 1.7 s; look-around between 0.4 and 1.4 s, ±15° head / ±0.7 gaze | runtime |
| TTS chunk cap | 120 UTF-16 units (non-stream split) | ChatPanel |
| Typewriter base delay | 50 ms / textSpeed (min speed 0.1); ×2.5 `、,…`; ×1.5 `」）` | ChatPanel |
| Minigame speak | random every 120 ms for max(800, min(3000, len·90)) ms | MiniGamePanel |
| Idle sleep | 60 s (1 s interval); ignore activity for 800 ms after sleeping starts | ChatPanel |
| Waiting dots | 400 ms cycle `.`→`..`→`...` | ChatPanel |
| Achievement toast | 4000 ms | ChatPanel |

---

## 10. User-visible and log strings (verbatim)

| String | Source | Shown to the user? |
|---|---|---|
| `Audio playback failed` | AudioService | no (only drives the queue) |
| `Failed to resume audio` | AudioService | no |
| `TTS is not enabled` | TTSService | no |
| `Empty text provided` | TTSService | no |
| `Synthesis failed (<status>)` | TTSService | no |
| `Received empty audio data` | TTSService | no |
| `Synthesis failed` | TTSService | no |
| `STT is not enabled` | STTService | no; it becomes the status `音声入力に失敗しました` |
| `Microphone is not supported in this browser` | STTService | same |
| `Recording too short` | STTService | same |
| `Failed to start recording` | STTService | same |
| `Failed to stop speech recognition` | STTService | same |
| `No speech detected` | STTService | same |
| `Speech recognition failed: <code>` (default code `unknown_error`) | STTService | same |
| `Transcription failed (<status>)` | STTService | same |
| `Transcription failed` | STTService | same |
| `Request failed (<status>)` | AIService | via the §7.6 mapping |
| `Unknown error` | AIService | via the mapping (default) |
| `Streaming error` | AIService | via the mapping |
| `Unknown streaming error` | AIService | via the mapping |
| `音声入力: OK` | ChatPanel | yes, `.chat-stt-hint` |
| `音声入力に失敗しました` | ChatPanel | yes, `.chat-stt-hint` |
| `🎤` / `■` | ChatPanel mic button | yes |
| `音声入力` / `Speech input` | mic `aria-label` fallback | a11y |
| `送信` / `SEND` | send button fallback | yes |
| `キャンセル` | mobile cancel fallback | yes |
| `CTRL+Cでキャンセル` (ja) / `CTRL+C to cancel` (en) | `cancel_hint` | yes |
| `Cubism init failed` | Live2DCanvas | console.error |
| `Live2D preload warning:` | Live2DCanvas | console.warn |
| `Live2D destroy error:` | Live2DCanvas | console.warn |
| `Failed to load <src>` | Live2DCanvas | inside the errors above |
| `[ReamaCubism] Live2DCubismCore is not loaded.` | runtime | console.error |
| `[ReamaCubism] Failed to initialize Cubism subdelegate.` | runtime | console.error |
| `[ReamaCubism] Initializing. Canvas client size: WxH` | runtime | console.log |
| `[ReamaCubism] WebGL ready. Max texture size: N, Canvas buffer: WxH` | runtime | console.log |
| `Cannot initialize WebGL context.` | runtime | console.error |

---

## 11. localStorage keys touched (formats must stay identical)

No sessionStorage or IndexedDB is used by these files.

| Key | Type / format | Default | Read by | Written by |
|---|---|---|---|---|
| `Config_VoiceVol` | float string (`String(n)`) | `1.0` (stored as `"1"` by `initializeDefaults`) | AudioService constructor (once) | `AudioService.setVolume` (unused), Settings |
| `Config_Experimental_TTS_Enabled` | `"0"`/`"1"` | 0 | TTS sync | TTS setter, Settings |
| `Config_Experimental_TTS_Speaker` | string (`christina-jp`\|`christina`) | `christina-jp` | TTS sync | TTS setter, Settings |
| `Config_Experimental_TTS_ServerUrl` | string | `http://localhost:5100` | TTS sync | TTS setter, Settings |
| `Config_Experimental_STT_Enabled` | `"0"`/`"1"` | 0 | STT sync | STT setter (also the mic toggle), Settings |
| `Config_Experimental_STT_Provider` | string (`groq`\|`openai`\|`local`) | `groq` | STT sync | STT setter, Settings |
| `Config_Experimental_STT_ServerUrl` | string | `http://localhost:5101` | STT sync | STT setter, Settings |
| `Config_Experimental_STT_Language` | string (free text) | `ja` | STT sync | STT setter, Settings |
| `Config_ApiProvider` | int string | 8 (0 in `submitMessage`) | `buildPayload`, ChatPanel | Settings |
| `Config_ApiKey_<0..8>` / `Config_ApiKey` | string | "" | `buildPayload` | Settings (indices 0..6 only) |
| `Config_ModelName_<0..8>` / `Config_ModelName` | string | "" | `buildPayload` | Settings (0..6) |
| `Config_WebSearch` | `"0"`/`"1"` | 0 | `buildPayload` | Settings |
| `Config_VertexProject` | string | "" | `buildPayload` | Settings |
| `Config_VertexLocation` | string | `us-central1` | `buildPayload` | Settings |
| `Config_LocalProxyHost` | string | `127.0.0.1` | `buildPayload` | none (defaults only) |
| `Config_LocalProxyPort` | string | "" | `buildPayload` | none |
| `Config_GCloudCLI` | `"0"`/`"1"` | 1 | `buildPayload` | none |
| `Config_OllamaHost` | string | `http://localhost:11434` | `buildPayload` | Settings (trimmed; empty becomes the default) |
| `Config_OpenAICompatible` | `"0"`/`"1"` | 0 | `buildPayload` | Settings |
| `Config_OpenAIBaseUrl` | string | "" | `buildPayload` | Settings |
| `Config_EyeTracking` | `"0"`/`"1"` | 0 | ChatPanel to the Live2DCanvas prop | Settings |
| `Config_AudioInputEnabled` | `"0"`/`"1"` | (absent; treated as 0) | ChatPanel (gates the mic button) | **nobody** in the current source |
| `Config_TextSpeed` | float string | 1.0 | typewriter (`spokenChar` cadence) | Settings |

---

## 12. Quirks: preserve, or fix deliberately

Each quirk is marked **P** (preserve for parity) or **F?** (a fix candidate; needs a decision).

| Id | Quirk | Recommendation |
|---|---|---|
| Q-A1 | `AudioService.lipSyncValueChanged` has no subscribers; the mouth is binary from `isActivelySpeaking`. | P (keep the API); an improvement is optional (Open questions) |
| Q-A2 | The mouth stops when text finishes or pauses even though TTS audio keeps playing. | P by default |
| Q-A3 | `Config_VoiceVol` is read once; settings changes need a reload; MasterVol is ignored. | F? (apply live) |
| Q-A4 | `stop()` makes a pending `play()` reject, emitting a spurious `errorOccurred` that advances the queue harmlessly. | P (harmless) |
| Q-A5 | `onerror` and the rejected `play()` can double-emit `errorOccurred`. | P |
| Q-T1 | The 32-bit hash cache key can collide and play the wrong audio; the cache is never evicted (object-URL memory growth). | F? (raw composite key, or LRU) |
| Q-T2 | Settings Apply makes the TTS/STT setters no-ops, so no health check or `*Changed` signals follow. | P (health results are not displayed anyway) |
| Q-T3 | Health checks run at startup even when TTS/STT are disabled. | P |
| Q-S1, Q-S2, Q-S3 | Subscriber exceptions inside `try` blocks are converted into error signals (TTS, STT, AI). | Not applicable in Rust; do not emulate |
| Q-S4 | STT prefers browser SpeechRecognition and ignores provider/serverUrl. | P on Web; see the desktop risk |
| Q-S5 | Filename `recording.webm` even for `audio/ogg` or `audio/mp4` blobs. | P (upstream may depend on it) |
| Q-S6 | Leaked mic stream if `new MediaRecorder` throws; no double-start guard; an old `onstop` can clean up a newer recording. | F? (fix; no visible behaviour change in the normal path) |
| Q-S7 | An uncaught `recognition.start()` throw leaves "recording" for up to 30 s. | F? (catch, emit error, reset) |
| Q-S8 | The mic button is hidden unless `Config_AudioInputEnabled=1`, which nothing writes. | P (or add a settings toggle; product decision) |
| Q-S9 | The mic toggle force-enables STT persistently. | P |
| Q-S10 | `resolveSpeechLanguage` passes whitespace-only strings through. | P |
| Q-C1 | `sendChat` is not abortable; a late response after Ctrl+C or cancel re-enters `typing`. | F? (abort both; strongly suggested) |
| Q-C2 | SSE parser treats non-JSON payloads (including `[DONE]` and `null`) as tokens; drops a trailing line without `\n`; ignores `data:` without a space. | P (the server never triggers them) |
| Q-C3 | Streaming providers can get stuck in `waitingAPI` (stale closure, §7.1). | F? (most likely an unintended bug) |
| Q-L1 | Core and runtime scripts load **in parallel**. The runtime evaluates `Live2DCubismCore.ColorBlendType_*` at load time (runtime L9356–9377), so if it executes first it throws, `window.ReamaCubismRuntime` stays unset, `load` still fires, the promise resolves, and the canvas silently stays empty. | F (in V2: load the core first, then initialize the wasm player) |
| Q-L2 | After a script load error the failed `<script data-core="true">` stays in the DOM. A retry re-attaches listeners to a script that will never fire again, so the promise hangs forever. | F (remove the failed element before retrying) |
| Q-L3 | `pointerEvents = "auto"` has no functional effect (the runtime registers no pointer listeners; `onPointBegan`/`onPointMoved`/`onPointEnded` are dead code). CSS sets `pointer-events: auto` anyway. | P (keep the inline style) |
| Q-L4 | Eye tracking uses viewport coordinates and ignores stage transforms; there is no touch support. | P |
| Q-L5 | Cleanup's `getContext` may create a context just to lose it. | P (harmless) or skip when there is no player |
| Q-L6 | The 100 ms style-retry timer is not cleared on unmount (it then finds no canvas). | P (harmless) |
| Q-Q1 | The TTS session guard compares against a frozen session id (§7.3). | F? (implement the intended guard) |
| Q-Q2 | `processNextTTSChunkIfPossible` drops a whitespace-only chunk without advancing to the next one. This cannot happen because `enqueue` trims. | P |
| Q-Q3 | Tags split across stream deltas are not removed from the TTS buffer. | P |

---

## Rust mapping

### R0. Crate placement (aligned with the existing skeleton in `RealAmadeusRust/`)

```
crates/amadeus-core/src/
  storage.rs                (exists: KvStore, MemoryStore)
  settings/keys.rs          (settings spec) – constants used here, see §11
  protocol/chat.rs          ChatRole, ChatMessage, ChatRequestPayload, ChatResponseBody
  protocol/tts.rs           TtsSynthesizeRequest, TtsHealthRequest, TtsRuntimeInfo + tts_runtime_info(&Value)
  protocol/stt.rs           SttTranscribeFields, SttHealthRequest
  sse.rs                    SseLineParser (pure, §5.4 rules) + tests (§5.4 vectors)
  media.rs                  lip_sync_target/lip_sync_smooth, eye_from_mouse, resolve_speech_language,
                            split_text_for_tts, TtsChunkBuffer (flush logic), constants
  jscompat.rs               js_trim, js_trim_start, js_is_whitespace, simple_hash, js_truthy, js_string,
                            utf16_len, js_number_to_string
crates/amadeus-ui/src/
  services/emitter.rs       Emitter<T> (V1 Signal semantics)
  services/audio.rs         AudioService
  services/tts.rs           TtsService
  services/stt.rs           SttService (+ services/speech_recognition.rs JS interop)
  services/ai.rs            AiService
  services/mod.rs           singletons (thread_local!) + init(transport)
  transport/mod.rs          trait Transport, TransportError, JsonReply, AudioReply, StreamHandle
  transport/web.rs          WebTransport (fetch; api_base default "/amadeus")
  transport/tauri.rs        TauriTransport (invoke + Channel) [cfg(feature = "desktop")]
  components/live2d_canvas.rs  Live2dCanvas + preload_live2d()
crates/amadeus-live2d/src/
  core_loader.rs            ensure_core_loaded() (script tag for live2dcubismcore.min.js)
  player.rs                 ReamaPlayer { create, set_emotion, set_lip_sync, set_spoken_char, set_eye_tracking, destroy }
  preload.rs                preload_default_assets()
crates/amadeus-backend/src/api/{chat.rs, chat_stream.rs, tts.rs, stt.rs}   shared handlers (server + desktop)
apps/desktop/src/commands/media.rs   Tauri commands wrapping amadeus-backend handlers
```

### R1. `Emitter<T>` (replaces `Signal`; avoids clashing with Leptos `Signal`)

```rust
pub struct Emitter<T: 'static> { inner: Rc<RefCell<Vec<(u64, Rc<dyn Fn(&T)>)>>>, next: Cell<u64> }
impl<T> Emitter<T> {
    pub fn connect(&self, f: impl Fn(&T) + 'static) -> Subscription;  // Subscription::disconnect() / drop-to-disconnect (choose explicit to mirror V1)
    pub fn emit(&self, v: &T);   // clone the Vec of Rc's (snapshot), DROP the borrow, then call each in order
    pub fn clear(&self);
}
```

- Re-entrant `emit` and `connect`/`disconnect` during an emit must not panic. Never hold the `RefCell` borrow while calling handlers.
- Use `Emitter<()>` for parameterless signals.

### R2. `Transport` trait (one service layer; Web and Tauri implementations)

To reproduce V1 error mapping byte-for-byte, the transport returns *HTTP-like* replies `(status, ok, json_or_null)`. The services keep V1's `data?.error ?? "… (status)"` logic unchanged.

```rust
pub struct JsonReply { pub status: u16, pub ok: bool, pub body: Option<serde_json::Value> } // body None == response.json() failed
pub struct AudioReply { pub content_type: String, pub bytes: Vec<u8> }                     // or keep a web_sys::Blob on web
pub enum TransportError { Network(String) /* = JS TypeError.message */, Aborted }
pub enum StreamChunk { Text(String) }                     // already-decoded UTF-8 text fed to SseLineParser

#[async_trait::async_trait(?Send)]   // or return futures::future::LocalBoxFuture
pub trait Transport {
    async fn chat(&self, p: &ChatRequestPayload) -> Result<JsonReply, TransportError>;
    /// Starts a stream. `on_chunk` gets raw SSE text; returns when the stream ends.
    /// Err(JsonReply) when the response is !ok (V1: parse JSON error body).
    async fn chat_stream(&self, p: &ChatRequestPayload, abort: AbortToken,
                         on_chunk: &mut dyn FnMut(&str)) -> Result<Result<(), JsonReply>, TransportError>;
    async fn tts_synthesize(&self, r: &TtsSynthesizeRequest) -> Result<Result<AudioReply, JsonReply>, TransportError>;
    async fn tts_health(&self, server_url: &str) -> Result<JsonReply, TransportError>;
    async fn stt_transcribe(&self, audio: AudioUpload /* bytes|Blob + mime */, provider: &str, language: &str,
                            server_url: &str) -> Result<JsonReply, TransportError>;
    async fn stt_health(&self, server_url: &str) -> Result<JsonReply, TransportError>;
}
```

**`WebTransport`** (wasm, `web-sys`):

- Uses `window.fetch_with_str_and_init` and `RequestInit` with method, headers `Content-Type: application/json` and a JSON string body. Multipart uses `FormData::append_with_blob_and_filename("audio", &blob, "recording.webm")`, then `append_with_str` for `provider`, `language` and `serverUrl`, in that order.
- URLs are `{api_base}/app/api/...` with `api_base = "/amadeus"` by default. Make it configurable at build time for other deployments.
- `response.json()` failure maps to `body: None`.
- `fetch` rejection maps to `Network(js_error_message)` (read `.message` via `js_sys::Reflect`). Abort maps to `Aborted`, detected with `AbortSignal::aborted()`, as in V1.
- **Streaming:** `response.body()` gives a `ReadableStreamDefaultReader`. Loop `read()` and decode with a `web_sys::TextDecoder` using `TextDecodeOptions::stream(true)`. Decoding with TextDecoder instead of std UTF-8 keeps U+FFFD replacement behaviour identical.
- **TTS:** `response.blob()`, then the blob's `size()` is checked for 0, then `Url::create_object_url_with_blob`. Keep the Blob on web; there is no need to copy the bytes into wasm.

**`TauriTransport`** (desktop; `window.__TAURI_INTERNALS__.invoke`; `withGlobalTauri: true` is already set):

| Command | Args | Returns |
|---|---|---|
| `chat` | `{ payload }` | `JsonReply` |
| `chat_stream` | `{ payload, streamId: u32, onEvent: Channel<String> }`; the backend sends the exact SSE text frames (`data: {...}\n\n`) so the UI reuses the same `SseLineParser` | `Ok(())` at end, or a `JsonReply` for a pre-stream error |
| `chat_stream_cancel` | `{ streamId }` | (); the backend aborts the task via an `AbortHandle` or `CancellationToken` |
| `tts_synthesize` | `{ req }` | `tauri::ipc::Response` (raw bytes) framed as `[u16 BE len][content-type utf8][audio bytes]`, or a `JsonReply` error with a non-2xx status |
| `tts_health` | `{ serverUrl }` | `JsonReply` |
| `stt_transcribe` | raw `InvokeBody::Raw(bytes)` plus headers `x-mime`, `x-provider`, `x-language`, `x-server-url` (avoids base64) | `JsonReply` |
| `stt_health` | `{ serverUrl }` | `JsonReply` |

Desktop notes:

- **Same handlers, same output.** The desktop commands must call the *same* `amadeus-backend` handler functions as the axum routes, so status codes and JSON bodies are identical (§3.7, §4.10, §8). Desktop has no `/amadeus` base path and no rate limit.
- **Cancel race.** After `cancelStreaming`, the UI must ignore channel messages already queued for that `streamId`. Set an `aborted` flag before invoking the cancel. This reproduces V1's "no events after cancel".

### R3. Service APIs (Rust)

```rust
// amadeus-ui::services::audio
pub struct AudioService { /* audio: RefCell<Option<HtmlAudioElement>>, timer: RefCell<Option<gloo_timers::callback::Interval>>,
                             is_playing: Cell<bool>, lip: Cell<f64>, volume: Cell<f64>, + closures kept alive */ 
  pub playback_started: Emitter<()>, pub playback_finished: Emitter<()>, pub playback_state_changed: Emitter<()>,
  pub lip_sync_value_changed: Emitter<f64>, pub volume_changed: Emitter<f64>, pub error_occurred: Emitter<String> }
impl AudioService {
  pub fn new(store: &dyn KvStore) -> Rc<Self>;           // reads Config_VoiceVol once
  pub fn is_playing(&self) -> bool; pub fn current_lip_sync_value(&self) -> f64; pub fn volume(&self) -> f64;
  pub fn set_volume(&self, v: f64); pub fn play_from_file(&self, url: &str); pub fn play_from_blob(&self, b: &Blob);
  pub fn stop(&self); pub fn pause(&self); pub fn resume(&self);
}
```

- **Element handlers.** Use `Closure` objects for `onended`/`onerror`, stored per element. Drop them in `stop()` *after* `pause()`; keeping them is also fine because a paused element never fires `ended`.
- **`play()` promise.** Use `wasm_bindgen_futures::spawn_local(JsFuture::from(audio.play()?))`. The `.then` callback must act on `self`, mirroring V1.
- **Lip-sync timer.** `gloo_timers::callback::Interval::new(50, …)`. Randomness comes from `js_sys::Math::random()`.

```rust
// amadeus-ui::services::tts
pub struct TtsService { cache: RefCell<HashMap<String /*simple_hash*/, String /*blob url*/>>, enabled/speaker/server_url/…,
  pub synthesis_complete: Emitter<String>, pub synthesis_error: Emitter<String>, pub enabled_changed: Emitter<()>,
  pub speaker_changed: Emitter<()>, pub server_url_changed: Emitter<()>, pub server_available_changed: Emitter<()>,
  pub runtime_info_changed: Emitter<()> }
impl TtsService {
  pub fn synthesize(self: &Rc<Self>, text: &str);   // sync pre-checks + cache hit emit synchronously; then spawn_local
  pub fn check_server_health(self: &Rc<Self>);       // spawn_local
  pub fn set_enabled(&self, v: bool); pub fn set_speaker(&self, v: &str); pub fn set_server_url(&self, v: &str);
  pub fn clear_local_cache(&self);
}
// amadeus-ui::services::stt
impl SttService {
  pub fn start_recording(self: &Rc<Self>);   // decision tree §4.5 (browser recognition first)
  pub fn stop_recording(&self);
  pub fn transcribe_data(self: &Rc<Self>, blob: web_sys::Blob);
  pub fn check_server_health(self: &Rc<Self>);
  // emitters: transcription_complete(String), transcription_error(String), recording_state_changed(()), recording_progress(f64), …
}
// amadeus-ui::services::ai
impl AiService {
  pub fn send_chat(self: &Rc<Self>, messages: Vec<ChatMessage>);
  pub fn send_chat_streaming(self: &Rc<Self>, messages: Vec<ChatMessage>);
  pub fn cancel_streaming(&self);
  // emitters: response_received, stream_token, stream_complete, error_occurred (all String)
}
pub fn build_payload(store: &dyn KvStore, messages: Vec<ChatMessage>) -> ChatRequestPayload;   // in amadeus-core (pure)
```

- **Settings subscription.** The services subscribe to the settings `Emitter<String>` (owned by the settings module) with prefix filters, exactly as in V1 (§3.5).
- **State for the UI.** Expose `RwSignal<bool>` mirrors only if the UI needs them (`is_recording`). ChatPanel in V1 uses events.
- **Settings write formats** follow the settings spec. A float must be written with the JS `String(n)` form (`js_number_to_string`: integers without `.0`, exponent form below 1e-6 and at or above 1e21).

**`ChatRequestPayload` serde.** Use `#[serde(rename_all = "camelCase")]` with an explicit `#[serde(rename = "useGCloudCLI")] use_gcloud_cli: bool`. Declare the fields in the §5.1 order so `serde_json` output order matches V1. On the server side, `openaiCompatible` and `openaiBaseUrl` are optional (`#[serde(default)]`).

### R4. Pure helpers in `amadeus-core` (unit-test with the vectors in this spec)

- **`js_is_whitespace(c)`:** true for U+0009–000D, 0020, 00A0, 1680, 2000–200A, 2028, 2029, 202F, 205F, 3000 and FEFF. Rust `char::is_whitespace` differs: it **includes U+0085** and **excludes U+FEFF**, so do not use it. Use this predicate for `js_trim` and `js_trim_start`, and as the character class in `split_text_for_tts` instead of regex `\s`.
- **`simple_hash(s)`:** iterates over `encode_utf16()` with i32 wrapping arithmetic and returns `format!("{:x}", (h as i64).abs())`. Vectors are in §3.1.
- **`utf16_len(s)`:** used for the 120 cap and for `ttsProcessedIndex` offsets, so slicing semantics match V1. If an offset lands inside a surrogate pair, round up to the next char boundary.
- **`lip_sync_target(position, rand01)` and `lip_sync_smooth(prev, target)`:** vectors are in §2.5.
- **`eye_from_mouse(cx, cy, w, h) -> (f64, f64)`:** guard NaN, since the player does `|| 0`. Vectors are in §6.5.
- **`resolve_speech_language(lang)`:** table in §4.11.
- **`tts_runtime_info(status_ok, &Option<Value>) -> TtsRuntimeInfo { available, using_gpu, device, details }`:** vectors are in §3.8.
- **`SseLineParser { buffer: String, full: String }`:**
  - `fn push(&mut self, text: &str) -> Vec<SseAction>` and `fn finish(self) -> SseAction` (the EOF `Complete(full)`).
  - `enum SseAction { Token(String), Complete(String), Error(String) }`.
  - Stop consuming after `Complete` or `Error`, as V1 returns.
  - Vectors are in §5.4.
- **`split_text_for_tts(text) -> Vec<String>` and `TtsChunkBuffer::{capture(display, complete) -> Vec<String>, reset()}`:** §7.3 (owned jointly with the chat spec).

### R5. Live2D in V2

- **`amadeus_live2d::ensure_core_loaded() -> impl Future<Output = Result<(), JsValue>>`:**
  - Inserts `<script src="/resources/live2d/live2dcubismcore.min.js" async data-core="true">` once, recording `data-loaded="true"` on load.
  - It is a singleton future that resets on failure **and removes the failed script element** (fix Q-L2).
  - The runtime script disappears in V2. The `?v=` cache buster is no longer needed; the wasm bundle is hashed by Trunk.
- **`amadeus_live2d::preload()`:** fetches and caches the same files as `preloadDefaultAssets` (§6.3). If the shaders are embedded in the crate (`crates/amadeus-live2d/shaders` exists), only the model JSON, moc3, physics and texture need warming.
- **`amadeus_ui::components::Live2dCanvas`** (Leptos):

  ```rust
  #[component]
  pub fn Live2dCanvas(#[prop(into)] emotion: Signal<String>, #[prop(into)] lip_sync: Signal<f64>,
                      #[prop(into, optional)] spoken_char: Signal<String>, #[prop(into, optional)] eye_tracking: Signal<bool>) -> impl IntoView
  ```

  Behaviour, mirroring §6.4–6.5:

  1. Create a `NodeRef<Div>` with class `live2d-host`. Keep a `StoredValue<Option<ReamaPlayer>>` and the latest values.
  2. On mount, `spawn_local`: `ensure_core_loaded().await`. On error, `log::error!("Cubism init failed {e:?}")` and return.
  3. If disposed, return. Otherwise `ReamaPlayer::create(&host)`, then push the latest emotion (uppercased), lip sync and spoken char.
  4. Apply the canvas inline styles `pointer-events:auto; width:100%; height:100%`, with one 100 ms retry.
  5. One `Effect` per prop, applying uppercase to the emotion.
  6. The eye-tracking `Effect` adds or removes a `window` `mousemove` listener (`web_sys` `Closure` stored in a `StoredValue`; removed in `on_cleanup`) and sends `(0, 0)` when disabled.
  7. `on_cleanup`: set disposed, grab the canvas WebGL context, call `player.destroy()`, then `WEBGL_lose_context.loseContext()`, then clear the host's `innerHTML`.

  Preferably move the context loss into `ReamaPlayer::destroy()` so the component cannot get the order wrong.
- **`preload_live2d()`:** `ensure_core_loaded()` then `amadeus_live2d::preload()`. It logs `"Live2D preload warning:"` on error and never fails.

### R6. JS interop not covered by `web-sys` (custom `wasm_bindgen` externs)

- **SpeechRecognition.** Do not rely on `web_sys::SpeechRecognition`: it only binds the unprefixed constructor and is unstable.
  1. Look up `Reflect::get(&window, "SpeechRecognition")`, falling back to `"webkitSpeechRecognition"`.
  2. Construct it with `Reflect::construct`.
  3. Set `lang`, `interimResults` and `continuous` with `Reflect::set`.
  4. Attach `onresult`, `onerror` and `onend` as `Closure<dyn FnMut(JsValue)>`.
  5. Read `event.results[i][0].transcript` with `Reflect` (array-like).
  6. Keep the closures alive until `onend`.
- **Tauri.** `#[wasm_bindgen(js_namespace = ["window","__TAURI_INTERNALS__"], catch)] async fn invoke(cmd: &str, args: JsValue, options: JsValue) -> Result<JsValue, JsValue>`. Get the Channel through `window.__TAURI__.core.Channel` (Reflect construct) and set its `onmessage`. Raw-body invoke passes a `Uint8Array` as `args` with `{ headers: {...} }` options.

### R7. `web-sys` features to add to `amadeus-ui` (missing from the current list)

- `RecordingState`, for `MediaRecorder::state()`.
- `HtmlScriptElement` and `HtmlHeadElement`, for the core loader. They can go in `amadeus-live2d` instead.
- `DomException`, for error names and messages.
- `ReadableStreamReadResult`, or read `done`/`value` through `Reflect`.
- `WebGlRenderingContext`, `WebGl2RenderingContext` and `WebglLoseContext`, needed in whichever crate performs the context loss.
- Optional: `AudioContextOptions`.

The rest is already present: `AudioContext`, `AnalyserNode`, `MediaStreamAudioSourceNode`, `MediaRecorder`, `BlobEvent`, `MediaDevices`, `MediaStream`, `MediaStreamTrack`, `MediaStreamConstraints`, `HtmlAudioElement`, `HtmlMediaElement`, `Blob`, `BlobPropertyBag`, `Url`, `FormData`, `Request`, `RequestInit`, `Response`, `Headers`, `ReadableStream`, `ReadableStreamDefaultReader`, `AbortController`, `AbortSignal`, `TextDecoder`, `TextDecodeOptions` and `MouseEvent`.

---

## Open questions / risks

1. **Streaming stale-closure bug (Q-C3).** In V1 the streaming providers (1, 3, 4, 5, 6) appear never to leave `waitingAPI`, so the dialogue shows dots while TTS speaks. A Leptos port naturally fixes this. **Decide (recommended: fix), and confirm with the chat-panel spec owner.**
2. **TTS session guard (Q-Q1).** Implement the intended per-session guard (recommended) or the frozen-0 behaviour. They differ only when sessions overlap.
3. **Lip-sync source.** Parity means a binary mouth driven by the typewriter, with no movement while TTS audio plays after the text finishes. An optional V2 improvement:
   - feed `AudioService.lip_sync_value_changed`, or an AnalyserNode RMS of the TTS element via `MediaElementAudioSourceNode`, into `lip_sync` while `ttsPlaybackBusy`;
   - **but** that changes visible behaviour. This is a product decision.
4. **Desktop speech recognition.**
   - **WebView2:** exposes `webkitSpeechRecognition`, but it typically fails (no Google key; `network` or `not-allowed` errors). Because V1 prefers browser recognition whenever the constructor exists, desktop STT would never reach the MediaRecorder/backend path.
   - **WKWebView:** needs `NSSpeechRecognitionUsageDescription`. The current `apps/desktop/Info.plist` only has `NSMicrophoneUsageDescription`.
   - **WebKitGTK:** has no SpeechRecognition. MediaRecorder support depends on the WebKitGTK and GStreamer versions, and `getUserMedia` requires WebKit's `enable-media-stream` setting, which wry may not enable, so the user gets "Microphone is not supported in this browser" or `NotAllowedError`.
   - **Recommendation:** behind `cfg(feature = "desktop")`, skip browser recognition and use MediaRecorder → `stt_transcribe`. Longer term, consider native capture with `cpal` in the backend.
5. **Autoplay in desktop webviews.** TTS `play()` runs after an async fetch, outside the user gesture. WKWebView may require a user action for audible media unless the webview is configured for autoplay (wry has an autoplay option; check that Tauri 2 exposes it). If it is blocked, `NotAllowedError` makes `errorOccurred` skip every chunk silently. **Verify on macOS and Linux.**
6. **Error-message contract.** ChatPanel classifies errors by substring (§7.6). Desktop/backend errors must contain the same substrings for the same failure classes: `401`, `429`, `timeout`, `Network`/`Cannot connect`, `500/502/503`, `model`, `JSON`, `Vertex`. Web behaviour already differs by browser ("Failed to fetch" falls to the default message). Choose deliberately what desktop network failures say.
7. **MediaRecorder container versus filename (Q-S5).** Safari and WKWebView produce `audio/mp4` but upload it as `recording.webm`. The upstream STT server might decode by extension. Keep parity unless the upstream is verified tolerant.
8. **Localhost semantics (§8 note).** On the hosted web, TTS and STT `serverUrl`s pointing at `localhost` hit the web server, not the user. This is unchanged from V1 but surprising. Desktop changes the meaning, which is arguably what users expect. Document it in the release notes.
9. **Object-URL growth (Q-T1).** Long sessions with TTS accumulate blobs indefinitely. Parity keeps the leak; an LRU (for example 64 entries) with `revokeObjectURL` would be invisible to users.
10. **Script load race (Q-L1) and retry hang (Q-L2).** Both must be fixed by design in V2: load the core sequentially before the wasm player, and remove failed tags.
11. **JS string semantics.** `trim`, `\s`, `.length` (UTF-16) and `charAt` (UTF-16 unit) have subtle Rust differences. Use the `jscompat` helpers everywhere text is chunked, hashed or offset-indexed, otherwise TTS chunking, cache keys and streaming deltas drift on emoji, NBSP, BOM or NEL input.
12. **Volume (Q-A3).** Parity means the voice volume is fixed at page load and master volume is ignored. If V2 applies volumes live, coordinate with the settings and audio-BGM spec owners (`Config_MasterVol` × `Config_VoiceVol`?).
13. **Mic button visibility (Q-S8).** In V1 STT is effectively unreachable for new users. Confirm whether V2 should expose a toggle; the key name `Config_AudioInputEnabled` must stay the same.
14. **Tauri streaming backpressure.** A `Channel` delivers messages as separate JS callbacks. The UI must buffer through the same `SseLineParser` and honour the abort flag. Chunk boundaries can differ from HTTP, but the parser is boundary-agnostic, as test vector 2 shows.
15. **Remount churn.** Crossing the 900 px breakpoint, or 1200 px with a coarse pointer, recreates the WebGL context and the Cubism framework (ref-counted). The V2 player must support rapid destroy/create cycles without leaking contexts, and `destroy` must be idempotent.

# RealAmadeus V2 — Porting spec: subsystem `lib-core` → crate `amadeus-core`

Source snapshot: working tree of `/home/elvelt_admin/Elvelt/dsuv` on 2026-11-16, including uncommitted changes (branch `main`, HEAD `7e45174`).
All paths below are relative to that repo unless absolute. "V1" = the current Next.js 16 / React 19 app. "V2" = the Rust port.

Every line of the 10 assigned files was read. Behaviour that the UI layer (`components/amadeus/RealAmadeusApp.tsx`, 4,700+ lines) adds on top of these modules was read where it changes core semantics: call order, emotion and message-splitting rules, persona prompts, error mapping and export format. Those parts are summarized in §12 with exact line references. The golden outputs in Appendix G were produced by running the real V1 TypeScript under Node 22 (`--experimental-strip-types`) with a fixed clock (2026-11-16 21:34:56, TZ=Asia/Tokyo), a mocked `localStorage`, and deterministic RNGs.

---

## 0. Scope and sources

| File | Lines | Contents | Target in V2 |
|---|---|---|---|
| `lib/amadeus/types.ts` | 40 | shared TS types | `amadeus_core::types` |
| `lib/amadeus/signal.ts` | 25 | tiny observer `Signal<TArgs>` | `amadeus_core::signal` |
| `lib/amadeus/app-settings.ts` | 137 | settings store over `localStorage` + `DEFAULT_SETTINGS` | `amadeus_core::settings` |
| `lib/amadeus/chat-utils.ts` | 223 | emotion-tag strip/parse, memory-command extraction, user-name extraction, pause/typing rules | `amadeus_core::chat` |
| `lib/amadeus/memory-manager.ts` | 474 | long-term memory (facts, episodes, summaries, emotions), prompt context | `amadeus_core::memory` |
| `lib/amadeus/minigame.ts` | 199 | Steins;Gate quiz data + logic | `amadeus_core::minigame` |
| `lib/amadeus/localization.ts` | 53 | `t()` + 13 embedded locale JSON files | `amadeus_core::i18n` |
| `lib/amadeus/services/random-event-service.ts` | 199 | weighted startup random events by hour bucket | `amadeus_core::events` |
| `lib/amadeus/services/achievement-manager.ts` | 378 | 28 achievements, launch streak, persistence | `amadeus_core::achievements` |
| `lib/amadeus/services/vector-database.ts` | 308 | client-side RAG vector store (localStorage), similarity search, server health/embed calls | `amadeus_core::rag` (pure) + `amadeus-ui` (async service) |

Related files that were also read: the RAG proxy routes `app/amadeus/app/api/rag/health/route.ts` (37 lines) and `app/amadeus/app/api/rag/embed/route.ts` (42 lines), and `lib/amadeus/locales/*.json` (13 files).

### 0.1 Who calls what in V1 (call sites in `components/amadeus/RealAmadeusApp.tsx` unless noted)

| API | Call sites |
|---|---|
| `AppSettings.initializeDefaults()` | 3972 (in `RealAmadeusApp` mount effect) |
| `AppSettings.get*/set*` | many; see §4.3 for the complete key catalogue |
| `AppSettings.settingsChanged` | 424 (`subscribeSettings` for `useSyncExternalStore`), achievement-manager.ts:209, vector-database.ts:65, tts-service.ts:48, stt-service.ts:57 |
| `t`, `LANGUAGE_NAMES` | everywhere in the UI; `random-event-service.ts:185`, `achievement-manager.ts:357` |
| `stripEmotionTag` | 1433, 1545, 1608, 1626 |
| `parseEmotionTag` | 1540, 1633 |
| `extractMemoryCommands` | 1512 |
| `extractUserNameFromFact` | 1515 |
| `isPauseCharacter` | 231 (`splitMessage`), 1163 (typewriter) |
| `charDelayForChar` | 1176 (typewriter) |
| `MemoryManager.getMemoryContext` | 958 (initial system prompt), 1253 (`updateSystemPrompt`) |
| `MemoryManager.getDynamicContext` | 1257 |
| `MemoryManager.setUserName` | 507 and 2015 (login sets `"Salieri"`), 1517 (name learned from a fact), 4676 (logout passes `""`, which is a no-op) |
| `MemoryManager.addUserFact` / `addEpisodicMemory` | 1514 / 1521 |
| `MemoryManager.recordEmotion` | 1542, 1595, 1636 |
| `MemoryManager.trimConversationHistory(next, 30)` + `recordInteraction()` | 1775–1776 (inside a React state updater; see §7.10) |
| `MemoryManager.totalInteractions` | 1544, 1639 |
| `MemoryManager.userName` / `userNameChanged` | 2365 / 2299 |
| `RandomEventService.tryTriggerEvent/getEventText/getEventEmotion` | 1293–1295 |
| `AchievementManager.notifyEvent` | 1311 `random_event`; 1543/1638 `emotion`; 1544/1639 `chat`; 1746/1753/1760 `dev_command`; 1766 `secret_steiner`; 2658 `game_play`; 2696 `game_win`; 2699 `game_perfect`; 4074 `export`; 4146 `import` |
| `AchievementManager.resetAll` | 1759 |
| `AchievementManager.getAllAchievements/unlockedCount/totalCount` | 2388, 2460–2462 |
| `AchievementManager.achievementUnlocked` / `achievementsChanged` | 1234 (toast) / 2293, 2441 |
| minigame exports | 2513–2819 (`MiniGamePanel`) |
| `VectorDatabase.enabled/topK/threshold/serverUrl` setters | 3315–3318 (config dialog Apply) |
| `VectorDatabase.checkServerHealth()` | 3977 (app mount, called whether or not RAG is enabled) |

**Unused in V1 (dead API, but keep it in V2 for parity):** `VectorDatabase.addEntry`, `search`, `removeEntry`, `clearAll`, `entryCount`, `getContextForPrompt`, `entryAdded`, `searchComplete`, `searchError`. Also `MemoryManager.clearAllMemory`, `isEmotionRepeated`, `statsChanged`, and `addConversationSummary`, which is only called internally. `RandomEventService.eventTriggered` and `getEventKey` are unused, as are the external uses of `AchievementManager.unlock` and `isUnlocked`. The `startup` event type of `notifyEvent` is never sent, because streaks are handled in the constructor. **RAG therefore does nothing user-visible in V1** except the health-check request at startup.

---

## 1. JS semantics the Rust port must reproduce

The modules rely on many JS-specific behaviours. Collect them in one helper module (`amadeus_core::js`) and use it everywhere. Do not use the Rust std equivalents ad hoc.

### 1.1 Strings are UTF-16 in V1
- `s.length`, `slice`, `substring`, `charAt` and `indexOf` work in UTF-16 code units. Rust `str` is UTF-8.
- Where only ASCII delimiters are searched and sliced (memory commands, emotion tags), byte indices in Rust give identical results.
- Where a **length limit** is applied, emulate UTF-16: `trimConversationHistory` uses 50 and 300 code units, and the TTS chunk limit is 120 (§6.9). Compute `utf16_len(s) = s.encode_utf16().count()` and cut at the largest char boundary whose UTF-16 offset is ≤ N. Where V1 would cut inside a surrogate pair, producing a lone high surrogate, which `JSON.stringify` writes as `\ud83d` (verified, Appendix G), V2 must drop that half. This deviation is acceptable and documented.
- The typewriter (§6.9) advances one UTF-16 unit per tick in V1, so an emoji takes 2 ticks. In V2 advance per `char`; the timing difference is negligible.

### 1.2 Whitespace
- JS `String.prototype.trim()` and regex `\s` use the set `JS_WS` = U+0009 TAB, U+000A LF, U+000B VT, U+000C FF, U+000D CR, U+0020 SPACE, U+00A0 NBSP, U+1680, U+2000–U+200A, U+2028, U+2029, U+202F, U+205F, U+3000 (ideographic space), U+FEFF (BOM).
- Rust `str::trim()` and the regex crate's `\s` use Unicode `White_Space`. That set **includes U+0085 and excludes U+FEFF**, so it differs from JS (verified: `/\s/.test("\u0085") === false`, `/\s/.test("﻿") === true`).
- → Implement `fn is_js_ws(c: char) -> bool` and `fn js_trim(&str) -> &str` (also `js_trim_start` and `js_trim_end`), and use them for every `.trim()` in this spec. In regexes write an explicit class `JS_WS_CLASS = "\t\n\x0B\x0C\r \u{A0}\u{1680}\u{2000}-\u{200A}\u{2028}\u{2029}\u{202F}\u{205F}\u{3000}\u{FEFF}"` in place of `\s`.
- Some code tests a *specific* whitespace list instead, for example `splitMessage` skips only `" ", "\t", "\n", "\r"`. Those lists are reproduced literally where they appear.

### 1.3 Regex flavour
- JS `\d` without the `u` flag means `[0-9]`. Rust `\d` is Unicode `Nd`, so always write `[0-9]`.
- The JS `i` flag without `u` folds case ASCII-only for these patterns. Rust `(?i)` uses Unicode simple case folding, so U+017F `ſ` would match `s` and U+212A `K` (Kelvin sign) would match `k`. All case-insensitive patterns here (`[SMILE]`, `<think>`) should use explicit classes (`[sS][mM]...`) or hand-written ASCII-case-insensitive scanning.
- `String.prototype.replace(regex_with_g, "")` removes all matches. `str.replace("%1", x)` with a string pattern replaces the **first occurrence only**, and expands `$&`, `$$`, `` $` `` and `$'` inside `x`. No V1 replacement value contains `$`, so `replacen(pat, x, 1)` is equivalent.

### 1.4 Numbers
- `Number.parseInt(raw, 10)` → `js_parse_int`:
  1. Strip leading `JS_WS`.
  2. Accept an optional `+` or `-`.
  3. Take the longest run of ASCII `[0-9]`. If empty, return NaN.
  4. The value is the decimal integer. JS gives the nearest f64 for huge values, for example `"99999999999999999999"` → `1e20`.
  5. `"0x10"` → 0, `"1e3"` → 1, `"1.9"` → 1, `"  42abc"` → 42, `"　12"` → 12, `"Infinity"` → NaN, `"-0"` → -0, which `String()` prints as `"0"`.
- `Number.parseFloat(raw)` → `js_parse_float`:
  1. Strip leading `JS_WS`.
  2. `Infinity`, `+Infinity` and `-Infinity` give ±∞.
  3. Otherwise take the longest prefix matching `[+-]?([0-9]+(\.[0-9]*)?|\.[0-9]+)([eE][+-]?[0-9]+)?`. If there is none, return NaN.
  4. Parse that prefix with Rust `f64::from_str`, which rounds identically.
  5. Examples: `".5"` → 0.5, `"1e"` → 1, `"0x1A"` → 0, `"1e400"` → ∞.
- `Number.isFinite(x)` → `x.is_finite()`. The settings getters treat NaN and ±∞ as "use default".
- `String(number)` (settings writes, tag coercion) → JS Number-to-String. Use the crate `ryu-js` (`ryu_js::Buffer::new().format(f)`). Verified outputs: `1.0`→`"1"`, `0.8`→`"0.8"`, `3.0`→`"3"`, `0.7`→`"0.7"`, `0.1+0.2`→`"0.30000000000000004"`, `1e21`→`"1e+21"`, `1e-7`→`"1e-7"`, `-0`→`"0"`, `NaN`→`"NaN"`, `∞`→`"Infinity"`. Rust's own `Display` differs for large and small magnitudes.
- `Math.trunc(x)` → `x.trunc()`. `String(Math.trunc(-0.5))` is `"0"`.
- `Math.round(x)` → JS rounds half toward +∞. Exact emulation: `fn js_round(x: f64) -> f64 { let f = x.floor(); if x - f >= 0.5 { f + 1.0 } else { f } }`. This gives 0.5→1, 2.5→3, -0.5→0, -2.5→-2 and 0.49999999999999994→0, all as in JS. The naive `(x+0.5).floor()` wrongly returns 1 for the last value. Rust `f64::round` rounds half away from zero, so `(-0.5).round()` = -1 while JS gives -0. Only used for `類似度` percentages.
- `Math.floor(Math.random() * n)` → `(rng.next_f64() * n as f64).floor() as usize`.

### 1.5 Local time and date formats
All "now" values use the **browser's local time zone** through `new Date()`. In wasm, read the local time via `js_sys::Date`; natively, use `chrono::Local`. Inject a `Clock` trait (§Rust mapping), because core must stay I/O-free and testable. Formats produced in this subsystem (all zero-padded with `padStart(2,"0")`; `getFullYear()` is not padded):

| Producer | Format | Example |
|---|---|---|
| memory `currentDateIso()` (fact `lastUpdated`, episode `timestamp`) | `YYYY-MM-DD` | `2026-11-16` |
| memory `recordInteraction()` → `lastSessionDate` | `YYYY-MM-DD HH:mm` | `2026-11-16 21:34` |
| memory `getDynamicContext()` | `YYYY/MM/DD HH:mm:ss` | `2026/11/16 21:34:56` |
| memory `trimConversationHistory()` stamp, via `toLocaleString("ja-JP",{month:"2-digit",day:"2-digit",hour:"2-digit",minute:"2-digit"})` | `MM/DD HH:mm` (24 h, verified) | `11/16 21:34`, `01/02 00:07` |
| achievements `todayDateString()` → `lastLaunchDate` | `YYYY-MM-DD` | `2026-11-16` |
| achievements `nowDateTimeString()` → `dates[id]` | `YYYY-MM-DD HH:mm` | `2026-11-16 21:34` |
| (UI) export file name | `chat_export_` + `toISOString().replace(/[:.]/g,"-").substring(0,19)` + `.json` (**UTC**) | `chat_export_2026-11-16T12-34-56.json` |
| (UI) export `exportedAt` | `toISOString()` (UTC, ms) | `2026-11-16T12:34:56.000Z` |

### 1.6 Randomness
`Math.random()` returns f64 in [0,1). Inject an `Rng` trait. In wasm, implement it with `js_sys::Math::random()` for identical distribution; natively use `fastrand` or `rand`. Tests use a scripted sequence or `mulberry32` (Appendix G). **RNG call order matters** for parity tests and is specified for each algorithm: minigame 78 calls (§8.3), random events 1 call (§9.3), mood hint 1 call (§7.7).

### 1.7 JSON
- Persisted JSON is produced by `JSON.stringify` with no spaces and with `undefined` fields omitted. Key order is insertion order; it does not matter for reading, but to round-trip byte-identically use `serde_json` with the `preserve_order` feature or `indexmap`.
- Numbers that are integers in V1 must serialize without `.0`: `totalInteractions`, `strength`, `launchStreak`, `version`. Use integer Rust types for them. Serialize embeddings as `f64`; JS writes `0.5` as `0.5` and `1` as `1`, while serde writes `1.0` as `1.0`, which reads back fine.
- Lenient reading: V1 accepts many malformed shapes and coerces them, as described per module. Parse into `serde_json::Value` first, then normalize exactly as specified. Do not use strict `#[derive(Deserialize)]` structs for persisted data.
- `String(v)` on a JSON value (`js_value_to_string`): string → itself; number → JS number formatting (§1.4); `true`/`false`; `null` → `"null"`; object → `"[object Object]"`; array → elements joined by `","`, where null and undefined elements become `""` and nested values are recursively stringified.
- Known parse differences: JS `JSON.parse` accepts lone surrogate escapes (`"\ud800"`), while `serde_json` rejects them, so the V2 code path falls back exactly as for a parse error. Nesting depth limit is 128 in serde_json versus effectively unlimited in JS. Both are acceptable deviations.

### 1.8 Signals
See §3. All V1 emissions are synchronous and happen right after the state change and storage write.

### 1.9 Storage access and error handling (V1 exact)
| Module | Read | Write | Exceptions |
|---|---|---|---|
| AppSettings | `localStorage.getItem(key)` | `setItem(key, value)` | not caught (would throw) |
| MemoryManager | `getItem("RealAmadeus.Memory")` | `setItem(...)` inside try/catch → `console.warn("[MemoryManager] Failed to persist memory to localStorage:", error)` | parse errors caught → empty memory |
| AchievementManager | `getItem("amadeus_achievements")` | `setItem` | write not caught; parse errors caught → fresh state |
| VectorDatabase | `getItem("RealAmadeus.VectorStore")` | `setItem` | write not caught (quota error would reject `addEntry`, which nobody awaits); parse errors caught → empty |

When `window` is undefined (SSR), AppSettings and AchievementManager use an in-memory fallback, and MemoryManager and VectorDatabase skip persistence entirely. In V2, which is CSR-only, this maps to a `MemoryKv` implementation of `KvStore` for tests. V2 should **log and continue** on write errors in all modules; never panic.

### 1.10 Module-load singletons and initialization order (V1 exact)
ES module evaluation constructs these singletons **at import time**, before React mounts, in this order. The order is determined by `RealAmadeusApp.tsx` imports, depth-first.
1. `AppSettings` (no storage access).
2. `MemoryManager` → `loadMemory()`.
3. `AchievementManager` → `loadState()`, then `updateLaunchStreak()`, which writes storage and may unlock `streak_3`/`streak_7`. No one is listening yet, so no toast. Then it subscribes to `settingsChanged`.
4. Unrelated: `AIService`, `AudioService` (reads `Config_VoiceVol`), `STTService`, `TTSService`.
5. `VectorDatabase` → `syncFromSettings()`, `load()`, subscribe; if enabled → `checkServerHealth()`.
6. `RandomEventService` → `initEvents()`, which **freezes the hour bucket at page load**.
7. On `RealAmadeusApp` mount (`useEffect`), in order: `AppSettings.initializeDefaults()`, then `getLlmStatusFromSettings()`, then `preloadLive2D()`, then the TTS, STT and `VectorDatabase.checkServerHealth()` calls (unconditional).

So on a user's very first visit, steps 3 and 5 read settings **before** defaults exist, using the call-site defaults. Those are identical to `DEFAULT_SETTINGS` for the keys involved, so behaviour is the same. V2 should create an `AppCore` in this exact order (§Rust mapping).

---

## 2. `types.ts` (verbatim semantics)

```ts
type AppState = "login" | "boot" | "chat";
type ChatRole = "system" | "user" | "assistant";
interface ChatMessage { role: ChatRole; content: string; }
interface ChatMemoryExtraction { cleanedText: string; facts: string[]; events: { summary: string; tags: string[] }[]; }
interface LlmStatus { provider: string; model: string; latencyMs: number; }      // latencyMs = -1 means "unknown"
interface VectorEntry { id: string; content: string; category: string; embedding: number[]; metadata: Record<string, unknown>; }
interface VectorSearchResult { id: string; content: string; category: string; similarity: number; metadata: Record<string, unknown>; }
```
Rust: `#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]`. `ChatRole` serializes as lowercase strings, `#[serde(rename_all = "lowercase")]`. `LlmStatus` uses `#[serde(rename_all = "camelCase")]` (`latencyMs`), with `latency_ms: f64`. `metadata: serde_json::Map<String, Value>`. `embedding: Vec<f64>`. `ChatMessage` is also the wire DTO for the chat API, so keep field names `role` and `content`.

On import (UI §12.5), roles other than system, user or assistant are mapped to `"assistant"` (`toChatRole`).

---

## 3. `signal.ts`

```ts
class Signal<TArgs extends unknown[] = []> {
  private handlers = new Set<SignalHandler<TArgs>>();
  connect(handler) { this.handlers.add(handler); return () => this.disconnect(handler); }
  disconnect(handler) { this.handlers.delete(handler); }
  emit(...args) { for (const handler of [...this.handlers]) handler(...args); }
  clear() { this.handlers.clear(); }
}
```
Semantics to keep:
- Handlers are called in **connection order**, which is Set insertion order.
- `emit` iterates over a **snapshot**. A handler connected during an emit is not called in that emit. A handler disconnected during an emit **is still called** in that emit if it was in the snapshot.
- Connecting the *same function object* twice is a no-op, because a Set deduplicates. Rust closures have no identity, so assign each connection an id. Dedup cannot be emulated, and V1 never relies on it.
- `connect` returns an unsubscribe closure, which React effects use as cleanup.
- An exception in a handler propagates to the emitter and aborts the remaining handlers. V2 should not panic; handlers are infallible `Fn`.

Signals present in this subsystem:
- `AppSettings.settingsChanged(key: string)`
- `MemoryManager.userNameChanged()`, `statsChanged()`
- `RandomEventService.eventTriggered(text, emotion)`
- `AchievementManager.achievementUnlocked(id, title, description)`, `achievementsChanged()`
- `VectorDatabase.entryAdded(id)`, `searchComplete(results)`, `searchError(message)`, `enabledChanged()`, `serverUrlChanged()`, `topKChanged()`, `thresholdChanged()`, `serverAvailableChanged()`

---

## 4. `app-settings.ts`

### 4.1 API and algorithm
Singleton `AppSettings` (class `AppSettingsStore`):
- `settingsChanged: Signal<[string]>`
- `memoryFallback: Map<string,string>`, used when `typeof window === "undefined"`.
- `hasKey(key)`: `localStorage.getItem(key) !== null`.
- `readRaw(key)`: `getItem(key)`; null becomes `undefined`.
- `writeRaw(key, value)`: `setItem(key, value)`.
- `initializeDefaults()`: for each `[key, value]` of `DEFAULT_SETTINGS` in insertion order (§4.2), `if (!hasKey(key)) writeRaw(key, String(value))`. **No signal is emitted.** Existing values, even empty strings, are never overwritten.
- `getInt(key, defaultValue = 0)`: if `raw === undefined`, return the default. Otherwise compute `parsed = parseInt(raw, 10)`, and return `isFinite(parsed) ? parsed : defaultValue`. Stored `"1.5"` → 1. Stored `""` → default. Stored `"abc"` → default.
- `getFloat(key, defaultValue = 0)`: the same with `parseFloat`. Stored `"Infinity"` → default.
- `getString(key, defaultValue = "")`: `raw === undefined ? defaultValue : raw`. A stored empty string is returned as `""`, not as the default.
- `setInt(key, value)`: `writeRaw(key, String(Math.trunc(value)))`, then `settingsChanged.emit(key)`.
- `setFloat(key, value)`: `writeRaw(key, String(value))`, then emit. Uses JS number formatting, for example `1.2000000000000002`.
- `setString(key, value)`: `writeRaw(key, value)`, then emit.
- `save()`: no-op ("Browser localStorage is already persistent; kept for API compatibility."). V2 keeps it as a flush hook, for example for a desktop file-backed store.
- **Every setter emits `settingsChanged(key)` even when the value is unchanged.** The desktop config "Apply" (`handleApply`, 3246–3322) makes exactly 50 `set*` calls, so 50 signals in one synchronous burst: 13 system/text/sound keys, 10 more (resolution, provider, languages, OpenAI and Vertex options, Ollama), 14 per-provider keys 0..6, 2 legacy keys, 3 TTS, 4 STT and 4 RAG. Subscribers must tolerate redundant notifications.

### 4.2 `DEFAULT_SETTINGS` (verbatim, in insertion order) and the string actually stored by `initializeDefaults`

| # | Key | Default (TS literal) | Stored string |
|---|---|---|---|
| 1 | `Config_SkipLoading` | `0` | `"0"` |
| 2 | `Config_RightClickMenu` | `1` | `"1"` |
| 3 | `Config_EyeTracking` | `0` | `"0"` |
| 4 | `Config_DesktopNotifications` | `1` | `"1"` |
| 5 | `Config_LightweightMode` | `0` | `"0"` |
| 6 | `Config_TextSpeed` | `1.0` | `"1"` |
| 7 | `Config_AutoMode` | `0` | `"0"` |
| 8 | `Config_AutoSpeed` | `3.0` | `"3"` |
| 9 | `Config_MasterVol` | `1.0` | `"1"` |
| 10 | `Config_BGMVol` | `0.8` | `"0.8"` |
| 11 | `Config_SEVol` | `1.0` | `"1"` |
| 12 | `Config_VoiceVol` | `1.0` | `"1"` |
| 13 | `Config_ScreenMode` | `1` | `"1"` |
| 14 | `Config_Resolution` | `0` | `"0"` |
| 15 | `Config_ApiProvider` | `8` | `"8"` |
| 16 | `Config_ApiKey` | `""` | `""` |
| 17 | `Config_ModelName` | `""` | `""` |
| 18 | `Config_Language` | `0` | `"0"` |
| 19 | `Config_ChatLanguage` | `0` | `"0"` |
| 20 | `Config_WebSearch` | `0` | `"0"` |
| 21 | `Config_VertexProject` | `""` | `""` |
| 22 | `Config_VertexLocation` | `"us-central1"` | `"us-central1"` |
| 23 | `Config_GCloudCLI` | `1` | `"1"` |
| 24 | `Config_LocalProxyHost` | `"127.0.0.1"` | `"127.0.0.1"` |
| 25 | `Config_LocalProxyPort` | `""` | `""` |
| 26 | `Config_OllamaHost` | `"http://localhost:11434"` | same |
| 27 | `Config_OpenAICompatible` | `0` | `"0"` |
| 28 | `Config_OpenAIBaseUrl` | `""` | `""` |
| 29 | `Config_Experimental_TTS_Enabled` | `0` | `"0"` |
| 30 | `Config_Experimental_TTS_Speaker` | `"christina-jp"` | same |
| 31 | `Config_Experimental_TTS_ServerUrl` | `"http://localhost:5100"` | same |
| 32 | `Config_Experimental_STT_Enabled` | `0` | `"0"` |
| 33 | `Config_Experimental_STT_Provider` | `"groq"` | same |
| 34 | `Config_Experimental_STT_ServerUrl` | `"http://localhost:5101"` | same |
| 35 | `Config_Experimental_STT_Language` | `"ja"` | same |
| 36 | `Config_Experimental_RAG_Enabled` | `0` | `"0"` |
| 37 | `Config_Experimental_RAG_TopK` | `5` | `"5"` |
| 38 | `Config_Experimental_RAG_Threshold` | `0.7` | `"0.7"` |
| 39 | `Config_Experimental_RAG_ServerUrl` | `"http://localhost:5102"` | same |
| 40–57 | `Config_ApiKey_0`, `Config_ModelName_0`, `Config_ApiKey_1`, `Config_ModelName_1`, … `Config_ApiKey_8`, `Config_ModelName_8` (loop `for i in 0..=8`: ApiKey then ModelName) | `""` | `""` |

Total: 57 keys. `DEFAULT_SETTINGS` is exported but no other module imports it.

### 4.3 Complete settings key catalogue (every key referenced anywhere in V1)
Notation: "int" means read with `getInt` and written with `setInt` (stored as a decimal integer string); "bool-int" means an int that is 0 or 1 (`=== 1` tests true, **any other value is false**); "float" uses `getFloat`/`setFloat`; "string" uses `getString`/`setString`. "Dialog" means the desktop config dialog (`RealAmadeusApp.tsx` 3122–3322); "Mobile" means `MobileSettingsPanel` (582–648).

| Key | Type | Call-site defaults (getter) | Domain and meaning | Written by | Read by / effect in V1 |
|---|---|---|---|---|---|
| `Config_SkipLoading` | bool-int | 0 | 1 = skip the boot/loading animation | Dialog | `MobileLoadingPanel` (549): finish immediately; `LoadingPanel` (2069): finish via `setTimeout(onFinished, 0)` |
| `Config_RightClickMenu` | bool-int | 1 | 1 = right-click toggles the menu panel | Dialog | root `onContextMenu` (4456); `preventDefault` always happens |
| `Config_EyeTracking` | bool-int | 0 | Live2D eye tracking follows the pointer | Dialog | `useAppSettingInt` (844) |
| `Config_DesktopNotifications` | bool-int | 1 (dialog pre-load state `false`) | "通知を表示" | Dialog | **no consumer in V1**; V2 desktop may implement it |
| `Config_LightweightMode` | bool-int | 0 | "軽量化モード" | Dialog | **no consumer in V1** |
| `Config_TextSpeed` | float | 1.0 | slider 0.1–3, step 0.1; typewriter speed multiplier | Dialog | `charDelayForChar(char, textSpeed)` (1175) |
| `Config_AutoMode` | bool-int | 0 | auto-advance dialogue | Dialog, Mobile, toggle (4264) | `useAppSettingInt` (3907) |
| `Config_AutoSpeed` | float | 3.0 | slider 1–10, step 0.1, suffix "s"; auto-advance wait = `autoSpeed*1000` ms | Dialog | (3908, 1221) |
| `Config_MasterVol` | float | 1.0 | slider 0–1, step 0.05, displayed as % | Dialog | (no direct consumer found outside the dialog) |
| `Config_BGMVol` | float | 0.8 | slider 0–1, step 0.05 | Dialog | (same) |
| `Config_SEVol` | float | 1.0 | slider 0–1, step 0.05 | Dialog | (same) |
| `Config_VoiceVol` | float | 1.0 | 0–1 | Dialog; `AudioService.setVolume` (clamps 0..1) | `AudioService` constructor (`audio-service.ts:19`) |
| `Config_ScreenMode` | int | 1 (dialog pre-load state 0) | 0 = fullscreen, 1 = windowed | Dialog; `fullscreenchange` handler writes `isFull ? 0 : 1` and calls `save()` (3994); toggles at 4203–4205 and 4246–4248 | `useAppSettingInt` (3910) |
| `Config_Resolution` | int | 0 | 0 → 1920×1080, 1 → 1600×900, 2 → 1280×720, anything else → 1920×1080 (`resolveResolution` 413) | Dialog | (3911) |
| `Config_ApiProvider` | int | **8** in most places; **0** in `submitMessage` (1790); the key always exists by then | 0 OpenAI, 1 Google Gemini, 2 Anthropic Claude, 3 Groq, 4 Vertex AI (option hidden unless `Config_VertexEnabled==1`), 5 Ollama, 6 OpenRouter, 7 unused (empty name, skipped in selects), 8 FreeAPI | Dialog, Mobile | `ai-service.ts:22`, LLM status, streaming decision (§12.2) |
| `Config_ApiKey` | string | "" | legacy/global key; fallback when `Config_ApiKey_{provider}` is empty; written as a mirror of the current provider's key | Dialog, Mobile | `ai-service.ts:25`, Mobile (584) |
| `Config_ApiKey_{0..8}` | string | "" | per-provider credential. Vertex (4) accepts an API key **or** a service-account JSON blob (textarea) | **Dialog writes only 0..6** (`for i < 7`); Mobile writes `_{current provider}` (any index, including 7 and 8) | `ai-service.ts:23` |
| `Config_ModelName` | string | "" | legacy/global model, fallback | Dialog, Mobile | `ai-service.ts:29`, LLM status |
| `Config_ModelName_{0..8}` | string | "" | per-provider model | Dialog (0..6), Mobile (current) | `ai-service.ts:27`, LLM status (3858) |
| `Config_Language` | int | 0 | display language index 0..12 (§5.2) | Dialog, Mobile, language pickers (4421, 4635) | `t()` everywhere; achievements and random events; `getCurrentChatLanguageIndex` |
| `Config_ChatLanguage` | int | 0 | 0 = same as display language; n ≥ 1 → language n−1 | Dialog | `getCurrentChatLanguageIndex` (200), language-change effect (1270) |
| `Config_WebSearch` | bool-int | 0 | LLM web search | Dialog, Mobile | `ai-service.ts:36` → `payload.webSearch` |
| `Config_VertexProject` | string | "" | GCP project ID | Dialog, Mobile | `ai-service.ts:37` |
| `Config_VertexLocation` | string | "us-central1" | region. Mobile writes `value \|\| "us-central1"`; Dialog writes the raw value, which may be "" | Dialog, Mobile | `ai-service.ts:38` |
| `Config_GCloudCLI` | bool-int | 1 | `payload.useGCloudCLI` | **never written by UI** | `ai-service.ts:41` |
| `Config_LocalProxyHost` | string | "127.0.0.1" | `payload.localProxyHost` | never written | `ai-service.ts:39` |
| `Config_LocalProxyPort` | string | "" | `payload.localProxyPort` | never written | `ai-service.ts:40` |
| `Config_OllamaHost` | string | "http://localhost:11434" | Ollama base URL; written as `value.trim() \|\| "http://localhost:11434"` | Dialog, Mobile | `ai-service.ts:42` |
| `Config_OpenAICompatible` | bool-int | 0 | treat provider 0 as an OpenAI-compatible endpoint | Dialog, Mobile | `ai-service.ts:43`, display name `OpenAI互換(<baseUrl>)` |
| `Config_OpenAIBaseUrl` | string | "" | base URL for the compatible endpoint | Dialog, Mobile | `ai-service.ts:44` |
| `Config_Experimental_TTS_Enabled` | bool-int | 0 | TTS on | Dialog round-trip only (**no UI control in V1**); `TTSService.enabled` setter | tts-service |
| `Config_Experimental_TTS_Speaker` | string | "christina-jp" | `"christina-jp"` or `"christina"` (`TTS_SPEAKER_NAMES`); unknown value → index 0 on dialog load | as above | tts-service |
| `Config_Experimental_TTS_ServerUrl` | string | "http://localhost:5100" | | as above | tts-service |
| `Config_Experimental_STT_Enabled` | bool-int | 0 | | as above | stt-service |
| `Config_Experimental_STT_Provider` | string | "groq" | `"groq"`, `"openai"` or `"local"` (`STT_PROVIDER_NAMES`) | as above | stt-service |
| `Config_Experimental_STT_ServerUrl` | string | "http://localhost:5101" | | as above | stt-service |
| `Config_Experimental_STT_Language` | string | "ja" | free string | as above | stt-service |
| `Config_Experimental_RAG_Enabled` | bool-int | 0 | RAG on | Dialog round-trip; `VectorDatabase.enabled` setter | VectorDatabase |
| `Config_Experimental_RAG_TopK` | int | 5 | VectorDatabase setter clamps to [1,20]; the dialog writes `parseInt(text,10) \|\| 5` **unclamped** (0 → 5, -3 → -3) | Dialog, setter | VectorDatabase (unclamped on sync) |
| `Config_Experimental_RAG_Threshold` | float | 0.7 | setter clamps to [0,1]; the dialog writes `parseFloat(text) \|\| 0.7` **unclamped**. 0 is impossible through the dialog because 0 → 0.7. The dialog loads it as `toFixed(2)`, so each Apply rounds it to 2 decimals | Dialog, setter | VectorDatabase |
| `Config_Experimental_RAG_ServerUrl` | string | "http://localhost:5102" | | Dialog, setter | VectorDatabase |
| `Config_AudioInputEnabled` | bool-int | 0 | **not in DEFAULT_SETTINGS**; initial `showAudioInput` state in ChatPanel (843) | never written | ChatPanel |
| `Config_VertexEnabled` | bool-int | 0 | **not in DEFAULT_SETTINGS**; 1 shows "Vertex AI" (index 4) in provider dropdowns (674, 3705) | never written | Mobile and Dialog selects |

Per-provider credential resolution, used identically in `ai-service.ts buildPayload`, `getLlmStatusFromSettings` and Mobile:
```
provider = getInt("Config_ApiProvider", 8)
apiKey   = getString(`Config_ApiKey_${provider}`, "")    || getString("Config_ApiKey", "")
model    = getString(`Config_ModelName_${provider}`, "") || getString("Config_ModelName", "")
```
The dialog load differs: for i in 0..7, `apiKey_i = Config_ApiKey_i`, falling back to `Config_ApiKey` **only for i == current provider**; models work the same way.

### 4.4 Change subscribers (exact filters)
- `AchievementManager`: `key === "Config_Language"` → `achievementsChanged.emit()`.
- `VectorDatabase`: `key.startsWith("Config_Experimental_RAG_")` → `syncFromSettings()` (no change signals emitted).
- `TTSService`: prefix `Config_Experimental_TTS_`. `STTService`: prefix `Config_Experimental_STT_`. These are other subsystems.
- UI hooks `useAppSettingInt(key, default)` and `useAppSettingFloat(key, default)` re-read on **any** key change through `useSyncExternalStore`. The server snapshot is the default value.

### 4.5 Quirks to preserve or decide
1. Dialog "Apply" sets settings first and then calls the service setters (`VectorDatabase.enabled = …`). Each `AppSettings.set*` call synchronously triggers `syncFromSettings()`, so the service fields already hold the new values when the setter runs. **The setters then see "no change" and return early**: no `enabledChanged`, `serverUrlChanged`, `topKChanged` or `thresholdChanged` is emitted, there is **no health check**, and **no clamping** happens. The same applies to TTS and STT. Reproduce this if you keep the same call order.
2. The dialog writes `Config_ApiKey_i` and `Config_ModelName_i` only for i < 7. Providers 7 and 8 have per-provider keys only through Mobile.
3. `Config_ApiProvider` call-site default 0 versus stored default 8 (above).
4. Booleans stored as ints are compared with `=== 1`, so a stored `"2"` is false.

---

## 5. `localization.ts`

### 5.1 `t(key, defaultValue, configLanguage)`
```ts
const dict = translations[configLanguage.toString()] || ja;
if (dict && key in dict) return dict[key];
return defaultValue || key;
```
Rust:
```
fn t(key: &str, default: &str, lang: i64) -> String {
    let dict = locales::get(lang).unwrap_or(locales::ja());   // lang 0..=12 only; anything else (13, -1, NaN, 1.5) → ja
    if let Some(v) = dict.get(key) { return v.clone(); }       // returned even if v == "" (no V1 value is empty)
    if !default.is_empty() { default.to_string() } else { key.to_string() }
}
```
- The fallback for a key missing from the chosen locale is the **caller's default text**, not the ja.json value.
- `key in dict` also matches `Object.prototype` names such as `toString`. Never triggered; ignore.

### 5.2 Languages (index = `Config_Language` value)
| idx | code | file | `LANGUAGE_NAMES[idx]` (verbatim) | `CHAT_LANGUAGE_NAMES[idx]` (prompt language name, §12.1) | dedicated persona prompt? |
|---|---|---|---|---|---|
| 0 | ja | `ja.json` | 日本語 | Japanese (日本語) | yes (0) |
| 1 | en | `en.json` | English | English | yes (1) |
| 2 | zh | `zh.json` | 中文 | Chinese (中文) | yes (2) |
| 3 | ko | `ko.json` | 한국어 | Korean (한국어) | yes (3) |
| 4 | es | `es.json` | Español | Spanish (Español) | yes (4) |
| 5 | fr | `fr.json` | Français | French (Français) | yes (5) |
| 6 | de | `de.json` | Deutsch | German (Deutsch) | yes (6) |
| 7 | ru | `ru.json` | Русский | Russian (Русский) | yes (7) |
| 8 | uk | `uk.json` | Українська | Ukrainian (Українська) | no → English persona |
| 9 | pt | `pt.json` | Português | Portuguese (Português) | no → English persona |
| 10 | tr | `tr.json` | Türkçe | Turkish (Türkçe) | no → English persona |
| 11 | he | `he.json` | עברית | Hebrew (עברית) | no → English persona |
| 12 | ar | `ar.json` | العربية | Arabic (العربية) | no → English persona |

V1 sets no `dir="rtl"` for he or ar.

### 5.3 Locale files
- Path: `lib/amadeus/locales/<code>.json`. Each is a flat JSON object of string → string with no nesting, no duplicate keys and no non-string values (verified).
- Key counts: ja 535, en 535, ko/es/uk/pt/tr/he/ar 531, zh/fr/de/ru 530.
- Keys missing relative to ja: all 11 non-ja/en locales lack `freeapi_quota_exhausted`, `freeapi_quota_label`, `freeapi_quota_remaining` and `freeapi_quota_usage`. zh, fr, de and ru also lack `sleeping_placeholder`. These are UI keys; the callers supply defaults.
- Prefix breakdown (ja): `mgq` 294, `startup` 87, `ach` 56 (+ `achievement_secret_hint`), `setting` 30, `mg` 15, `help` 6, `changelog` 6, `import` 6, `category` 5, `export` 5, `freeapi` 4, `login` 3, misc 18.
- **All 453 keys referenced by the lib-core modules exist in all 13 locales** (verified): 294 `mgq_*`, 15 `mg_*`, 87 `startup_*`, 57 `ach*`. So the in-code defaults are never shown in V1; the JSON files are the effective source of truth.
- **ja.json differs from the in-code Japanese defaults for 69 of these keys** (Appendix E): 49 quiz comments `mgq_*_c`, 1 question `mgq_bday_faris_q` (code フェイリス・ニャンニアン vs ja.json フェイリス・ニャンニャン), 1 answer `mgq_fg9_a0`, 5 `mg_*` strings (the 4 result comments and `mg_wrong`), and 13 startup events. No achievement strings differ. Japanese users see the **ja.json** text. V2 must embed the locale files **verbatim** and treat in-code defaults only as fallbacks.
- Rust: `include_str!` all 13 files into `amadeus-core` (about 470 KB of JSON). Parse each lazily on first use (`OnceLock<HashMap<String,String>>` per locale), or generate `phf` maps in `build.rs`.

### 5.4 Placeholder conventions
Templates use `%1` and `%2`, substituted with JS `.replace("%1", v)`, which replaces the first occurrence (§1.3). Keys with placeholders (ja): `mg_menu_desc` (%1 = 10), `mg_score_result` (%1 = score, %2 = 10), `mg_wrong` (%1 = localized correct answer), plus UI keys `freeapi_quota_usage` (%1 count, %2 max, both `toLocaleString()`), `freeapi_quota_remaining`, `update_confirm`. Every locale contains the needed placeholders for the three mg keys (verified).

### 5.5 Not this module
`hooks/useTranslation.tsx` (676 lines) is a separate ja/en nested dictionary used by the portal, docs and download pages (`app/page.tsx`, `app/amadeus/docs/**`, `app/amadeus/download/page.tsx`, …). It is unrelated to `t()` and belongs to another subsystem.

---

## 6. `chat-utils.ts` (and the other emotion and splitting rules)

### 6.1 Emotion tags
The canonical list, in this order, is used everywhere (`ALL_EMOTIONS` in achievements, persona prompts, stream detector):
`NORMAL, SMILE, ANGRY, SAD, SURPRISED, BLUSH, WINK, DISGUST, SMUG, THINKING, PANIC` (11).
A tag is written as `[TAG]` with **no inner spaces**; the match is case-insensitive.
- `SLEEPING` and `SNEEZE` exist as Live2D motions (dev help text §12.4) but are **not** emotion tags; regexes do not match them.

JS regexes (verbatim):
```js
EMOTION_TAG_PATTERN = /\[(NORMAL|SMILE|ANGRY|SAD|SURPRISED|BLUSH|WINK|DISGUST|SMUG|THINKING|PANIC)\]/gi   // global
/\[(NORMAL|SMILE|ANGRY|SAD|SURPRISED|BLUSH|WINK|DISGUST|SMUG|THINKING|PANIC)\]/i                        // first match
/<think>[\s\S]*?<\/think>/gi                                                                              // reasoning blocks
```
Rust equivalents, ASCII-only case folding (§1.3):
```
TAG_RE   = r"\[(?:[nN][oO][rR][mM][aA][lL]|[sS][mM][iI][lL][eE]|[aA][nN][gG][rR][yY]|[sS][aA][dD]|[sS][uU][rR][pP][rR][iI][sS][eE][dD]|[bB][lL][uU][sS][hH]|[wW][iI][nN][kK]|[dD][iI][sS][gG][uU][sS][tT]|[sS][mM][uU][gG]|[tT][hH][iI][nN][kK][iI][nN][gG]|[pP][aA][nN][iI][cC])\]"
THINK_RE = r"(?s)<[tT][hH][iI][nN][kK]>.*?</[tT][hH][iI][nN][kK]>"
```
A hand-written scanner is also fine: find `[`, find the next `]`, ASCII-uppercase the inside, and check membership. It must require the exact `[NAME]` form with no inner whitespace.

### 6.2 `stripEmotionTag(rawText) -> string`
1. `withoutThink = rawText.replace(THINK (global, non-greedy, case-insensitive, spans newlines), "").trim()`. An unclosed `<think>` is **not** removed.
2. Return `withoutThink.replace(EMOTION_TAG_PATTERN (global), "").trim()`. **All** tags anywhere are removed, not only a leading one. Surrounding spaces are left in place: `"前置き [WINK] 後ろ"` → `"前置き  後ろ"` (two spaces).

Examples are in Appendix G.

### 6.3 `parseEmotionTag(rawText) -> string`
Find the **first** tag anywhere in `rawText` (case-insensitive) and return its name uppercased. If none, return `"NORMAL"`.
- It does **not** strip `<think>` first, so `"<think>[ANGRY]</think>[SMILE] hi"` → `"ANGRY"`. `handleApiSuccess` strips think blocks before calling it; `handleStreamComplete` does **not** (§12.3).
- `"[ PANIC ]"` → `"NORMAL"` (inner spaces are not accepted).

### 6.4 `extractUserNameFromFact(fact) -> string`
```js
const value = fact.trim(); if (!value) return "";
const match = value.match(/(?:ユーザー\s*の\s*)?名前は\s*[「『"']?([^」』"'。、.!?！？\s]+)/);
return match?.[1]?.trim() ?? "";
```
- The pattern is **not anchored**. It captures 1+ characters after `名前は` + optional whitespace + one optional opening quote (`「`, `『`, `"` or `'`). The capture stops at a closing quote (`」`, `』`, `"`, `'`), at `。`, `、`, `.`, `!`, `?`, `！`, `？`, or at whitespace (JS `\s`).
- The opening quotes `「` and `『` are not excluded inside the capture: `名前は「「X」` → `「X`.
- Rust: `r#"(?:ユーザー[WS]*の[WS]*)?名前は[WS]*[「『"']?([^」』"'。、.!?！？WS]+)"#` with `WS` = `JS_WS_CLASS` (§1.2). Leftmost-first semantics give the same captures.
- Known false positives, kept for parity: `猫の名前はタマ` → `タマ` (the user would be renamed to the cat's name), `名前はまだない。` → `まだない`, and `名前は Okabe Rintaro` → `Okabe`.

### 6.5 Memory commands (`[SAVE_FACT: …]`, `[SAVE_EVENT: …]`)
The LLM may embed these anywhere in its reply. The persona prompts in V1 do **not** mention them, so they appear only if the model does it spontaneously or a backend prompt adds them; check the chat-route spec. They are extracted, applied to memory, and removed from the displayed text.

#### 6.5.1 `findMemoryCommandEnd(text, payloadStart) -> index | -1`
A scanner from `payloadStart` that returns the index of the first `]` outside any JSON string, `{}` and `[]` nesting:
```
inString=false; escaping=false; braceDepth=0; bracketDepth=0
for index in payloadStart..len:
  ch = text[index]
  if escaping { escaping=false; continue }
  if inString && ch=='\\' { escaping=true; continue }
  if ch=='"' { inString=!inString; continue }          // only ASCII double quote toggles strings
  if inString continue
  if ch=='{' { braceDepth+=1; continue }
  if ch=='}' && braceDepth>0 { braceDepth-=1; continue }
  if ch=='[' { bracketDepth+=1; continue }
  if ch==']' {
     if braceDepth==0 && bracketDepth==0 { return index }
     if bracketDepth>0 { bracketDepth-=1 }              // a ']' inside {...} with bracketDepth==0 is ignored
  }
return -1
```
Consequences:
- A stray `"` in a plain-text payload (`[SAVE_FACT: 彼は"天才] 続き`) makes the closing `]` look like it is inside a string. The scan returns -1 and extraction stops (Appendix G).
- `[SAVE_FACT: 好きな数字は[7]] ok` → the fact is `好きな数字は[7]`.

All delimiters are ASCII, so scanning UTF-8 bytes in Rust and returning a byte index is equivalent.

#### 6.5.2 `extractMemoryCommands(rawText) -> {cleanedText, facts, events}`
```
text = rawText.trim()                       // JS trim
result = {cleanedText: text, facts: [], events: []}
if text is empty: return result
ranges = []; index = 0
while index < len(text):
  start = text.indexOf("[SAVE_", index)      // case-SENSITIVE literal "[SAVE_"
  if start < 0: break
  colon = text.indexOf(":", start)           // first ASCII ':' at/after start (full-width '：' is NOT accepted)
  if colon < 0: break
  header = text[start+1 .. colon] with all \s removed, then toUpperCase()
  type = "FACT" if header == "SAVE_FACT", "EVENT" if header == "SAVE_EVENT", else ""
  if type == "": index = start + 6; continue           // skip past "[SAVE_"
  end = findMemoryCommandEnd(text, colon + 1)
  if end < 0: break                                     // unterminated → stop; command stays in text
  payload = text[colon+1 .. end].trim()
  if FACT:  fact = parseSaveFactPayload(payload); if fact != "" && fact not in result.facts (exact, case-sensitive): push
  if EVENT: ev = parseSaveEventPayload(payload);  if ev.summary != "": push (events are NOT de-duplicated)
  ranges.push([start, end+1])                           // range is removed even if the payload was empty/rejected
  index = end + 1
cleaned = text with ranges removed (apply from last to first)
result.cleanedText = cleaned.replace(/\n{3,}/g, "\n\n").trim()
```
Notes:
- `[save_fact: x]` in lowercase is not detected. However `[SAVE_ fact : y]` **is** detected, because the header only needs to start with literal `[SAVE_` and its whitespace is stripped and uppercased.
- A full-width colon `：` makes the search use the next ASCII `:` in the text (for example `12:30`). The header then fails to match and the command is skipped.
- Removal does not collapse spaces: `"Hello [SAVE_FACT: x] world"` → `"Hello  world"`. Only runs of 3 or more `\n` are collapsed to 2; `\r\n` sequences are not.

#### 6.5.3 `parseSaveFactPayload(payload) -> string`
```
trimmed = payload.trim(); if empty → ""
if trimmed starts with "{":
   try parsed = JSON.parse(trimmed)
       if parsed.content?.trim() → return it trimmed
       if parsed.fact?.trim()    → return it trimmed
       if parsed.text?.trim()    → return it trimmed
   catch → ignore
return trimmed                                      // raw text (also raw JSON text if no field matched!)
```
JS subtlety: `parsed.content?.trim()` throws a `TypeError` when `content` is a non-string, non-null value, for example the number 5. That throw is caught, so the function returns the raw JSON text. Rust rule: check the fields in order `content`, `fact`, `text`. For the first field that is **present and not null**: if it is a string whose trim is non-empty, return the trimmed value; if it is a string that trims to empty, continue to the next field; if it is a non-string (number, bool, object or array), **stop and return the raw `trimmed`**. If no field matches, return `trimmed`. Examples: `{"foo":"bar"}` → `{"foo": "bar"}` as written; `{"content": 5}` → raw.

#### 6.5.4 `parseSaveEventPayload(payload) -> {summary, tags}`
```
summary = payload.trim(); tags = []
if summary empty → {summary:"", tags:[]}
if summary starts with "{":
  try parsed = JSON.parse(summary)
      if parsed.summary?.trim()      → summary = parsed.summary.trim()
      else if parsed.content?.trim() → summary = parsed.content.trim()
      // tags (processed after summary; a TypeError above aborts BOTH → raw summary, no tags)
      if Array.isArray(parsed.tags):
          for tag in parsed.tags: cleaned = String(tag).trim(); if cleaned != "" && cleaned ∉ tags (exact match): push
      else if typeof parsed.tags == "string":
          for part in parsed.tags.split(/[,\|、]/): cleaned = part.trim(); same de-dup; push
  catch → ignore
return {summary, tags}
```
- If neither `summary` nor `content` is a non-blank string, **the summary stays as the raw JSON text**. `[SAVE_EVENT: {"summary": "  "}]` produces the event summary `{"summary": "  "}` (verified).
- The non-string field rule matches §6.5.3: the first present, non-null field among `summary` and `content` that is not a string aborts everything, giving the raw summary and empty tags.
- `String(tag)` follows §1.7: `3` → `"3"`, `null` → `"null"`, objects → `"[object Object]"`.
- The tag separators in the string form are ASCII comma, ASCII pipe and Japanese `、`. The full-width comma `，` is **not** a separator.
- Tag de-dup here is exact; `MemoryManager.addEpisodicMemory` de-dups again case-insensitively.

How the UI applies the result (`applyMemoryCommands`, 1511–1524):
```
for fact in facts: MemoryManager.addUserFact(fact); name = extractUserNameFromFact(fact); if name: MemoryManager.setUserName(name)
for ev in events:  MemoryManager.addEpisodicMemory(ev.summary, ev.tags)
return cleanedText
```

### 6.6 `isPauseCharacter(char, nextChar) -> bool` (page-break rule)
```
if char == ".":
   if nextChar == "."                                  → false    (ellipsis "..." pauses only after the last dot)
   if nextChar in {" ", "\n", "\r", ""}                → true     ("" = end of text)
   else                                                → false    ("Dr.Pepper", "1.5", "えっ.続き" do not pause)
isPause = char in {"。", "！", "？", "!", "?", "\n"}
if !isPause → false
return nextChar ∉ {"」", "）", ")", "』", "”"}           // closing brackets/quote (U+201D) keep the sentence going
```
Notes:
- `！？` pauses twice: after `！`, because `？` is not in the exclusion list, and again after `？`. So `えっ！？` becomes pages `えっ！` and `？`. This is V1 behaviour.
- `…` and `、` never pause. ASCII `"` and `'` after `!` do not prevent a pause.

### 6.7 `charDelayForChar(char, textSpeed) -> ms`
```
base = 1000 / 20 / max(0.1, textSpeed)         // = 50 ms at speed 1.0; speed is clamped below at 0.1 → 500 ms
if char in {"、", ",", "…"} → base * 2.5
if char in {"」", "）"}      → base * 1.5          // ASCII ")" and "』" are NOT slowed
else                         → base
```
Returns a fractional f64 (for example 16.666666666666668 at speed 3), which is passed to `setTimeout`. In Rust, keep f64 and round only when scheduling.

### 6.8 Other emotion-tag rules that live in `RealAmadeusApp.tsx` (pure; move to `amadeus_core::chat`)
1. **Streaming tag detector** (`handleStreamToken` 1565–1617). It is applied to the accumulated raw buffer each time a token arrives:
   ```
   buffer += token
   raw = buffer.replace(THINK_RE, "").trim()
   if !parsed:
     if raw starts with "[":
        close = raw.indexOf("]")
        if close > 0:
           maybe = raw[1..close].trim().toUpperCase()          // NOTE: inner spaces ARE tolerated here ("[ smile ]" ok)
           if maybe ∈ 11 tags: emotion = maybe; MemoryManager.recordEmotion(maybe);
                               raw = raw[close+1..].trimStart(); buffer = raw; parsed = true
           else if raw.length > 2 && !raw.startsWith("<"): parsed = true      // no tag at all
        // close <= 0 (no "]" yet) → keep waiting
     else if raw.length > 2 && !raw.startsWith("<"): parsed = true
   display = stripEmotionTag(raw)
   ```
   Important: once the tag is parsed, the **buffer is replaced** by the text after the tag with the think blocks already stripped. `raw.length > 2` counts UTF-16 units.
2. **TTS delta cleaning** (`captureStreamTextForTTS` 1489–1509): each new delta has `EMOTION_TAG_PATTERN` removed before buffering.
3. **Backlog cleaning** (`addBackLog` 4018–4043):
   - `clean = raw.trim()`. If `clean` starts with `[` and `indexOf("]") > 0`, then `clean = clean[close+1..].trim()`. This drops **any** leading bracketed prefix, not only emotion tags.
   - Then remove all emotion tags, `trim()`, and skip the entry if it is empty.
   - Role colours: `user`/`me` → `#66CCFF`; `assistant`/`kurisu`/`amadeus` → `#FF6666`; otherwise `#808080`. The role is lowercased.

### 6.9 Message splitting rules
1. **`splitMessage(text)`** (`RealAmadeusApp.tsx` 207–240). It splits imported assistant messages into backlog pages:
   ```
   if text == "" → []
   result=[]; startIdx=0; i=0
   while i < len:
     if i == startIdx: skip chars in {" ", "\t", "\n", "\r"} (ONLY these 4; U+3000 is not skipped); startIdx = i
     if i >= len: break
     c = text[i]; next = text[i+1] or ""; i += 1
     if isPauseCharacter(c, next) || i == len:
        seg = text[startIdx..i]; if seg.trim() != "": result.push(seg)     // pushed UNTRIMMED (keeps trailing "\n" pause char)
        startIdx = i
   return result
   ```
   Golden: `"  先頭空白。\r\n次の行\n最後"` → `["先頭空白。","次の行\n","最後"]`; `"えっ！？　何それ"` → `["えっ！","？","　何それ"]`.
2. **Typewriter paging** (`step` 1141–1188). Starting at a page start:
   - Skip `[" ", "\n", "\r", "\t"]`, emit one char, and compute `nextChar = fullText[nextIndex]` or `""`.
   - If `isPauseCharacter(char, nextChar)`, the page ends (wait for input `▼`).
   - Otherwise schedule the next char after `charDelayForChar(char, Config_TextSpeed)`.
   - When the text is exhausted but the stream is not complete, poll every **50 ms**.
   - Auto mode advances after `Config_AutoSpeed*1000` ms.
3. **`splitTextForTTS(text)`** (1404–1426), used for non-streamed replies:
   ```
   normalized = text.replace(/\s+/g, " ").trim(); if empty → []
   chunks=[]; current=""
   for char in normalized (iterates by CODE POINT):
       current += char
       if char ∈ {"。","！","？","!","?","\n"} || current.length >= 120 (UTF-16 units):
           if current.trim() != "": chunks.push(current.trim()); current = ""
   if current.trim() != "": chunks.push(current.trim())
   ```
   `\n` cannot occur after normalization.
4. **`flushTTSChunkBuffer(force)`** (1449–1487), used for streamed replies:
   ```
   loop:
     find first i with buf[i] ∈ {"。","！","？","!","?","\n"}; if none → break
     boundary = i+1; while buf[boundary] ∈ {"」","』","”","\"",")","）"}: boundary += 1
     chunk = buf[..boundary].trim(); buf = buf[boundary..].trimStart(); if chunk != "": enqueue(chunk)
   if force: tail = buf.trim(); buf = ""; if tail != "": enqueue(tail)
   ```
   Unlike `isPauseCharacter`, the TTS boundary **includes** trailing closing quotes, and its quote set also contains ASCII `"`.

---

## 7. `memory-manager.ts`

### 7.1 Data model and storage
localStorage key **`RealAmadeus.Memory`**. The value is `JSON.stringify(memory)`:
```ts
interface UserFact      { content: string; strength: number /* int ≥ 1 */; lastUpdated: string /* "YYYY-MM-DD" */ }
interface EpisodicMemory{ timestamp: string /* "YYYY-MM-DD" */; summary: string; tags: string[] }
interface MemoryPayload {
  userName?: string;               // "" when unknown; "Salieri" after login
  totalInteractions?: number;      // int
  lastSessionDate?: string;        // "YYYY-MM-DD HH:mm"
  recentEmotions?: string[];       // ≤ 10, oldest first
  userFacts?: UserFact[];          // ≤ 50, insertion order
  conversationSummaries?: string[];// ≤ 10, oldest first
  episodicMemory?: EpisodicMemory[];// ≤ 50, insertion order
}
```
Example of the real stored value after the Appendix G mutations (abridged):
```json
{"userName":"オカリン","totalInteractions":42,"lastSessionDate":"2026-11-16 21:34","recentEmotions":["SMILE","SMILE","ANGRY","ANGRY","ANGRY"],"userFacts":[{"content":"Dr Pepperが好き","strength":6,"lastUpdated":"2026-11-16"},{"content":"東京在住","strength":4,"lastUpdated":"2026-11-01"}],"conversationSummaries":["古い要約","[11/16 21:34の会話] user: … / assistant: msg2"],"episodicMemory":[{"summary":"一緒に実験した","timestamp":"2026-11-14","tags":["実験","ラボ"]},{"timestamp":"2026-11-16","summary":"[11/16 21:34の会話] …","tags":["要約"]}]}
```
After `clearAllMemory()` the stored value is exactly `{}`. In-memory there is also `emotionHistory: string[]` (≤ 10), initialised from `recentEmotions` at load and appended in lockstep, so it is always equal to `recentEmotions`.

### 7.2 Constants
`MAX_USER_FACTS = 50`, `MAX_CONVERSATION_SUMMARIES = 10`, `MAX_EPISODIC_MEMORIES = 50`, `EMOTION_HISTORY_SIZE = 10`, `STORAGE_KEY = "RealAmadeus.Memory"`. Prompt limits: top **6** facts, top **4** episodes, last **3** summaries, emotion trend shown at ≥ **3** emotions, and certainty thresholds strength ≥ 4 → 高 and ≥ 2 → 中, else 低. Trim: `maxTurns` default **30**, removal target `n - maxTurns + 4`, per-message preview **50** UTF-16 units + `...`, summary cap **300** UTF-16 units + `...`.

### 7.3 Helpers
- `currentDateIso()` → local `YYYY-MM-DD`.
- `parseIsoDate(value) -> Date | null`:
  1. `trimmed = value.trim()`; if `trimmed.length < 10`, return null.
  2. Otherwise compute `new Date(trimmed.slice(0,10) + "T00:00:00")` (local midnight); if invalid, return null.
  3. **V8 acceptance (verified):** the first 10 chars must be `YYYY-MM-DD` with 4 ASCII digits, month `01`–`12` and day `01`–`31`.
  4. The day **overflows into the next month**: `2026-02-30` → 2026-03-02, `2026-04-31` → 2026-05-01. Year `0050` is valid. `2026-00-10`, `2026-11-00`, `2026-02-32`, `2026/11/16` and `2026.11.16` → null. Anything after char 10 is ignored, so `"2026-11-16 12:30"` and `"2026-11-16T09:00:00Z"` → 2026-11-16 local.
  5. The exotic expanded-year form `-002026-11` is also accepted by V8; ignore it in V2.
  - Rust: parse the 10-char prefix with this rule, then normalise through a days-from-civil computation (overflow semantics). Comparison only needs a day number.
- `normalizeFactKey(fact)` = `fact.trim().toLowerCase()`. JS `toLowerCase` is full Unicode lowercasing; Rust `str::to_lowercase` matches it, including final sigma.
- `isNewerDateString(candidate, baseline)`:
  - both dates parse → `candidate > baseline`, strictly by day;
  - only the candidate parses → true;
  - only the baseline parses → false;
  - neither parses → plain string comparison `candidate > baseline` (UTF-16 order ≈ Rust byte order).
- `dedupeTags(tags)`: for each `tag`, `cleaned = tag.trim()` (**throws if a tag is not a string**); skip it if empty or if an earlier kept tag is equal case-insensitively (`toLowerCase()` compare); keep the **first** spelling. `["実験","実験 ","ラボ","ラボ"]` → `["実験","ラボ"]`; `["x","X"]` → `["x"]`.

### 7.4 Normalization (also applied lazily on every read path)
`normalizeUserFacts(input)`:
- `source = input ?? []`. A JS `for…of` over source means: array → its elements; string → its characters, which all get skipped; any other non-null value → **TypeError**, which at load time resets the whole memory to `{}` (§7.9).
- For each `value`:
  1. `content = String(value?.content ?? "").trim()`, using `js_value_to_string` (null → "" because of `??`). Skip the element if empty.
  2. `strength = Math.max(1, parseInt(String(value?.strength ?? 1), 10) || 1)`. Verified: `3`→3, `"3"`→3, `2.7`→2, `"4.9"`→4, `0`→1, `-5`→1, `null`→1, `"abc"`→1, `1e21`→1 (because `"1e+21"`→1), `true`→1.
  3. `lastUpdated = String(value?.lastUpdated ?? "").trim()`; if empty or `parseIsoDate` fails → `currentDateIso()`.
  4. Merge case-insensitively by `normalizeFactKey`. The first occurrence keeps its position and spelling, its `strength += strength`, and its `lastUpdated` takes the later date (`isNewerDateString`).

`normalizeEpisodicMemories(input)`, same iteration rules:
1. `summary = String(value?.summary ?? "").trim()`; skip it if empty.
2. `timestamp = String(value?.timestamp ?? "").trim()`; if empty or invalid → today.
3. `tags = dedupeTags(Array.isArray(value?.tags) ? value.tags : [])`; a non-string tag **throws**.
4. Push `{summary, timestamp, tags}` in that key order. Episodes are not de-duplicated.

### 7.5 Sort orders used for the prompt (the stored order is never changed)
`sortFactsForPrompt` (stable sort):
1. `strength` descending.
2. Date of `lastUpdated` descending. A dated fact sorts before an undated one; after normalization all are dated.
3. `a.content.localeCompare(b.content, "ja")` — **ICU Japanese collation**. Verified order sample: `["10代","abc","ABC","Dr.Pepperが好き","アニメが好き","ゲームが好き","コーヒー派","東京在住","猫を飼っている","名前は岡部"]`; code-point order would differ. Ties are common (strength-1 facts from the same day), so this matters for which facts make the top 6.

`sortEpisodesForPrompt` (stable sort):
1. Date of `timestamp` descending.
2. Dated before undated.
3. `b.timestamp.localeCompare(a.timestamp)` (default locale; for same-format timestamps this is plain descending string order).

**Quirk:** timestamps are date-only, so all episodes from the same day tie, and the stable sort keeps insertion order (**oldest first within the day**). The prompt therefore shows the **first 4 episodes of the most recent day**, not the latest 4. Appendix G confirms it: after adding more episodes today, the prompt still shows A, B, C, D.

### 7.6 `getMemoryContext() -> string` (verbatim templates)
Lines are joined with `"\n"`.
1. `rawName = (memory.userName ?? "").trim()`. `promptName = (rawName == "" || rawName.toLowerCase() == "salieri") ? "あなた" : rawName`. **Always** push:
   `【ユーザー情報】ユーザーのことは「${promptName}」と呼びなさい。「Salieri」や「サリエリ」とは絶対に呼んではいけません。`
   → The result is never empty. The UI check `if (memoryContext)` is therefore always true.
2. `facts = sortFactsForPrompt(normalizeUserFacts(memory.userFacts))`. If non-empty, push `【ユーザーについて知っていること（重要度順）】`, then for each of the first 6:
   `- ${content}（強度:${strength} / 信頼度:${certainty}）` where certainty = `高` if strength ≥ 4, `中` if ≥ 2, else `低`.
3. `episodes = sortEpisodesForPrompt(normalizeEpisodicMemories(memory.episodicMemory))`.
   - If non-empty, push `【最近のエピソード記憶】`, then for each of the first 4:
     `- [${timestamp}] ${summary}${suffix}` with `suffix = tags.length>0 ? "（タグ: " + tags.join(", ") + "）" : ""`.
   - Else if `conversationSummaries` is non-empty (legacy data only, because summaries are always mirrored into episodes), push `【過去の会話の記憶】`, then `- ${summary}` for each of the **last 3** summaries, oldest first.
4. If `memory.lastSessionDate` is truthy: `【前回のセッション】${lastSessionDate}`.
5. If `totalInteractions > 0`: `【累計やりとり回数】${totalInteractions}回`.
6. If `recentEmotions.length >= 3`: `【最近の感情傾向】${recentEmotions.join("→")}（同じ感情が続きすぎないように意識して）`. All stored emotions are shown, up to 10.

The full-width brackets and parentheses `【】（）「」` are part of the strings. Exact outputs are in Appendix G.

### 7.7 `getDynamicContext(turnCount)` and `getTimeContext()`
```
moodHints = ["（今は少しリラックスしている）","（知的好奇心が高まっている）","（少し眠そう）","（何かを考え込んでいる）","（いつも通りの調子）"]
hint = moodHints[floor(random() * 5)] ?? moodHints[0]          // 1 RNG call
return `【現在の状況】現在日時（クライアント時刻）: ${YYYY}/${MM}/${DD} ${HH}:${mm}:${ss} (${getTimeContext()}) / 会話ターン数: ${turnCount}\n${hint}`
```
`getTimeContext()` reads the local hour **again** with a second `new Date()`:

| Hour | Result |
|---|---|
| 5–9 | 朝 |
| 10–11 | 午前中 |
| 12–13 | 昼 |
| 14–16 | 午後 |
| 17–19 | 夕方 |
| 20–23 | 夜 |
| else (0–4) | 深夜 |

Golden (random=0.5, 21:34:56): `【現在の状況】現在日時（クライアント時刻）: 2026/11/16 21:34:56 (夜) / 会話ターン数: 3\n（少し眠そう）`.

### 7.8 Mutators (each ends with `saveMemory()` unless noted)
- `get userName` → `memory.userName ?? ""`. `get totalInteractions` → `memory.totalInteractions ?? 0`.
- `recordInteraction()`: `totalInteractions = (totalInteractions ?? 0) + 1`; `lastSessionDate = now "YYYY-MM-DD HH:mm"`; save; `statsChanged.emit()`.
- `recordEmotion(emotion)`: append to `emotionHistory` and keep the last 10; append to `memory.recentEmotions` and keep the last 10; save. No signal. Any string is accepted.
- `isEmotionRepeated()`: returns false if `emotionHistory.length < 3`. Otherwise it computes the length of the **trailing run** of identical values and returns `run >= 3`. The loop starts with `last=""`, so an initial `""` counts as a continuation, which cannot happen in practice.
- `setUserName(name)`: `trimmed = name.trim()`. **If empty, return**: no change, no save, no signal. This makes logout's `setUserName("")` a no-op. Otherwise set `userName`, save, and emit `userNameChanged`.
- `addUserFact(fact)`:
  1. `cleaned = fact.trim()`; if empty, return.
  2. `facts = normalizeUserFacts(memory.userFacts)`.
  3. If an existing fact has the same key (case-insensitive), increment its `strength` by 1 and set `lastUpdated = today`. Its position is unchanged.
  4. Otherwise push `{content: cleaned, strength: 1, lastUpdated: today}`.
  5. `memory.userFacts = facts.slice(-50)`. **Pruning is FIFO by insertion position**, so a strong but old fact can be pruned.
  6. Save.
- `addConversationSummary(summary)`:
  1. `cleaned = trim`; if empty, return.
  2. `conversationSummaries = [...old, cleaned].slice(-10)`.
  3. Push to the normalized episodes `{timestamp: today, summary: cleaned, tags: ["要約"]}`.
  4. `episodicMemory = episodes.slice(-50)`.
  5. Save. No signal.
- `addEpisodicMemory(summary, tags = [])`: trim; if empty, return. Push to normalized episodes `{timestamp: today, summary, tags: dedupeTags(tags)}`, keep `.slice(-50)`, save.
- `trimConversationHistory(history, maxTurns = 30) -> removedCount`. It **mutates the array in place**:
  ```
  n = count(m.role != "system")
  if n <= maxTurns → return 0
  removeTarget = n - maxTurns + 4                  // e.g. 31 → remove 5 → 26 non-system remain
  parts=[]; removed=0
  while 1 < history.length && removed < removeTarget:
      cur = history[1]                              // ALWAYS index 1 (index 0 assumed to be the system prompt)
      preview = utf16len(cur.content) > 50 ? cur.content.slice(0,50) + "..." : cur.content
      parts.push(`${cur.role}: ${preview}`)         // role is the literal "user"/"assistant"/"system"
      history.splice(1,1); removed += 1
  if parts non-empty:
      stamp = local "MM/DD HH:mm"                    // ja-JP toLocaleString, §1.5
      summary = `[${stamp}の会話] ${parts.join(" / ")}`
      addConversationSummary(utf16len(summary) > 300 ? summary.slice(0,300) + "..." : summary)
  return removed
  ```
  - It removes whatever sits at index 1, including stray system messages, which count toward `removed`.
  - After an import the history may have no system message at index 0 (§12.5); the message at index 0 is then never trimmed.
  - Golden: 32 non-system messages → removes 6, length 33 → 27, summary `[11/16 21:34の会話] user: これは五十文字を超える…これ... / assistant: msg2 / … / assistant: msg6`.
- `clearAllMemory()`: `memory = {}`, `emotionHistory = []`, save (writes `{}`), then emit `statsChanged`, then `userNameChanged`.

### 7.9 `loadMemory()` / `saveMemory()`
- `loadMemory()` runs in the constructor:
  1. If `getItem` returns null or `""` → `memory = {}`.
  2. Otherwise `parsed = JSON.parse(raw)` and build:
     ```
     memory = {
       userName:              parsed.userName ?? "",                 // NOT type-checked in V1
       totalInteractions:     parsed.totalInteractions ?? 0,         // NOT type-checked in V1
       lastSessionDate:       parsed.lastSessionDate ?? "",
       recentEmotions:        Array.isArray(parsed.recentEmotions) ? parsed.recentEmotions : [],   // elements not checked
       userFacts:             normalizeUserFacts(parsed.userFacts),
       conversationSummaries: Array.isArray(...) ? parsed.conversationSummaries.map(String) : [],
       episodicMemory:        normalizeEpisodicMemories(parsed.episodicMemory),
     }
     emotionHistory = [...memory.recentEmotions]
     ```
  3. **Any throw** (JSON syntax error, `parsed === null`, a non-iterable `userFacts`, a non-string tag) → `memory = {}`, `emotionHistory = []`. A non-object JSON value other than null (`5`, `[]`, `"x"`) yields the 7-field empty memory.
  - Loading does **not** write back; normalized values are persisted on the next save.
- `saveMemory()`: `setItem(STORAGE_KEY, JSON.stringify(memory))` inside try/catch (warn on failure).
- V1 type holes that V2 should close: a numeric `userName` would make `getMemoryContext()` throw, and a string `totalInteractions` such as `"5"` would make `recordInteraction` produce `"51"` by string concatenation. Recommended V2 coercion: `userName` = the string if it is a string, a number formatted JS-style, otherwise `""`; `totalInteractions` = a number truncated to i64, otherwise 0; `lastSessionDate` = the string, otherwise `js_value_to_string` unless null; `recentEmotions` elements = strings kept, others `js_value_to_string`. Document these as deliberate deviations; they only affect corrupted data.

### 7.10 How the chat pipeline drives MemoryManager (production React timing)
In `submitMessage` (1698–1799), for normal text after the dev commands (§12.4):
```
setChatInput(""); setTurnCount(v => v+1)
setConversationHistory(prev => { next=[...prev,user]; MemoryManager.trimConversationHistory(next,30); MemoryManager.recordInteraction(); return updateSystemPrompt(next) })
onBackLogAdd("user", text)
... (UI state)
provider = getInt("Config_ApiProvider", 0)
nextMessages = updateSystemPrompt([...conversationHistory, user])      // conversationHistory = value captured in this render
streaming providers {1,3,4,5,6} → AIService.sendChatStreaming(nextMessages) else AIService.sendChat(nextMessages)
```
React 18/19 batching defers the state updater until the next render. `setTurnCount` already queued an update on the same fiber, so the eager-state optimisation does not apply. Therefore, in production:
1. The **request** is built from the **untrimmed** history plus the user message. Its system prompt contains `getMemoryContext()` **before** `recordInteraction()`: `【前回のセッション】` is the previous message's time and the count excludes this message. It also contains `getDynamicContext(turnCount)` with the **pre-increment** turn count, which is 0 for the first message.
2. Afterwards, during render, the stored history gets the user message appended, then trimmed (possibly creating a conversation summary and episode), then `recordInteraction()`, then the system prompt replaced. That prompt uses the new memory context and still the old turnCount from the closure. It also calls `getDynamicContext` a second time, which consumes another RNG value for the mood hint and reads a new timestamp. The stored `history[0]` can therefore differ from the prompt that was sent; this is harmless, because the next request rebuilds the prompt.
3. In React StrictMode development builds, the updater runs **twice**, so `recordInteraction` runs twice per message. This is dev-only and must be ignored.

`updateSystemPrompt(history)` (1250–1268) builds `content = getSystemPromptForLanguage(getCurrentChatLanguageIndex())` + (`"\n\n" + memoryContext` if non-empty) + (`"\n\n" + dynamicContext` if non-empty). It replaces `history[0]` only if `history[0].role === "system"`; otherwise the history is returned without a system prompt. The initial history (955–963) is `[{role:"system", content: persona + "\n\n" + memoryContext}]` without dynamic context. When `Config_Language` or `Config_ChatLanguage` changes, `updateSystemPrompt` is re-applied (1273–1284).

Reply processing order (§12.3) per assistant reply:
1. Apply memory commands (facts → `addUserFact` and maybe `setUserName`; events → `addEpisodicMemory`).
2. `recordEmotion(tag)`.
3. `AchievementManager.notifyEvent("emotion", {emotion})`.
4. `notifyEvent("chat", {totalInteractions: MemoryManager.totalInteractions})`, whose count includes this turn.

V2 should implement this sequence explicitly as a `ChatTurn` state machine in core (§Rust mapping) that reproduces points 1 and 2 of the timing above.

### 7.11 Quirks summary (keep for parity unless the product owner decides otherwise)
1. The prompt always contains the user-info line.
2. Login always sets `userName = "Salieri"` (507, 2015), **overwriting a learned name at every login**. `getMemoryContext` maps "salieri" (case-insensitive) to "あなた".
3. Logout `setUserName("")` is a no-op.
4. The name extraction has false positives (§6.4).
5. Same-day episodes show the oldest 4 (§7.5).
6. Fact pruning is FIFO (§7.8).
7. `lastSessionDate` is effectively "last message time".
8. Corrupted fields can reset the whole memory (§7.9).

---

## 8. `minigame.ts` (Steins;Gate quiz)

### 8.1 Types and constants
- `MiniGameText = readonly [i18nKey, jpDefault]`.
- `MiniGameEntry { q: Text; a: [Text,Text,Text,Text]; correct: number /* always 0 in source */; c: Text }`.
- `MiniGameQuestion { q; a: Text[] (shuffled); correct (recomputed); c }`.
- `TOTAL_QUESTIONS = 10`.
- `QUESTION_POOL`: **49 entries**, verbatim in Appendix D. The header comment says it was "ported verbatim from the QT app `/amadeus/app/qml/MiniGamePanel.qml`".
- `RESULT_COMMENT_KEYS = { perfect: "mg_result_perfect", good: "mg_result_good", ok: "mg_result_ok", bad: "mg_result_bad" }`.
- `RESULT_COMMENT_DEFAULTS` (in-code JP fallbacks):
  - perfect `「パーフェクト！…まあ、当然の結果よね。」`
  - good `「ふーん、まあまあね。悪くないわ。」`
  - ok `「…もう少し勉強した方がいいんじゃない？」`
  - bad `「はぁ…全滅？本当にシュタゲ好きなの？」`
- `MG_WRONG_KEY = "mg_wrong"`, `MG_WRONG_DEFAULT = "はぁ…不正解よ。答えは「%1」だったわ。"`.
- UI key constants and `MG_UI_DEFAULTS`:
  - `mg_menu_title` 紅莉栖とシュタゲクイズ
  - `mg_menu_desc` シュタインズ・ゲートから%1問出題！ (%1 = TOTAL_QUESTIONS)
  - `mg_start` スタート
  - `mg_score` スコア
  - `mg_next` 次の問題
  - `mg_show_result` 結果を見る
  - `mg_result_title` 結果
  - `mg_score_result` %1 / %2 正解 (%1 = score, %2 = TOTAL_QUESTIONS)
  - `mg_retry` もう一度
  - `mg_menu_back` メニューへ
- The effective ja/en texts are in Appendix F; for the result comments and `mg_wrong`, ja.json differs from the in-code defaults.

### 8.2 `shuffle(arr)` — in-place Fisher–Yates
```
for i from len-1 down to 1: j = floor(random() * (i+1)); swap(arr[i], arr[j])
```

### 8.3 `generateQuestions()` (exact RNG consumption)
```
indices = [0, 1, …, 48]; shuffle(indices)                    // 48 RNG calls (i = 48..1)
for q in 0 .. min(10, 49):                                   // 10 questions
    src = POOL[indices[q]]
    order = [0,1,2,3]; shuffle(order)                        // 3 RNG calls (i = 3,2,1)
    a = [src.a[order[0]], …, src.a[order[3]]]
    correct = k such that order[k] == src.correct            // src.correct == 0 → position of original a0
    push {q: src.q, a, correct, c: src.c}
```
That is **78 `random()` calls in total**, in exactly this order: all of the pool shuffle first, then the answer shuffles in question order. Golden vectors: mulberry32(42), and `random() ≡ 0` → questions = pool[1..=10], answers `[a1,a2,a3,a0]`, `correct = 3` (Appendix G).

### 8.4 Scoring and result rules
- `resultEmotion(score)`: `score == 10` → `SMUG`; `≥ 6` → `SMILE`; `≥ 1` → `NORMAL`; else `DISGUST`.
- `resultCommentKey(score)`: the same tiers → perfect, good, ok, bad.
- Panel logic (UI, `MiniGamePanel` 2532–2819), relevant for core events:
  - `startGame` (menu Start or result Retry): stop speaking, generate questions, set index 0, score 0, selected −1, not revealed, state "playing", emotion THINKING, then `notifyEvent("game_play")`.
  - `selectAnswer(i)`: ignored once revealed. Set `isCorrect = (i == q.correct)`, mark revealed.
    - Correct: score += 1, emotion SMILE, comment = `t(q.c)`.
    - Wrong: emotion ANGRY, comment = `t("mg_wrong", MG_WRONG_DEFAULT).replace("%1", t(q.a[q.correct]))`.
    - Then `startSpeaking(comment)`: lip-sync random values every 120 ms for `clamp(len*90, 800, 3000)` ms.
  - `nextQuestion`: stop speaking, index += 1, reset selection.
    - If `index >= questions.length` → state "result", emotion `resultEmotion(score)`; if `score > 0` → `notifyEvent("game_win")` (**"win" means at least 1 correct answer**); if `score == 10` → `notifyEvent("game_perfect")`.
    - Otherwise emotion THINKING.
  - Button label: `index+1 >= 10 ? mg_show_result : mg_next`. Counter text `Q${index+1} / 10`. Score label `${t(mg_score)}: ${score}`.
  - When the panel becomes visible, the emotion is set by state: playing → THINKING, result → `resultEmotion(score)`, menu → SMILE. Hiding the panel stops speaking.
  - Back/Escape keys close the panel.
  - "Menu back" sets state "menu" and emotion SMILE.

---

## 9. `random-event-service.ts`

### 9.1 Model
`RandomEvent { key, defaultText, emotion (one of the 11 tags), weight }`. The service keeps `events`, `lastEventKey = ""`, `lastEventDefaultText = ""` and `lastEventEmotion = ""`. Signal: `eventTriggered(text, emotion)`.

### 9.2 Buckets
`initEvents()` runs in the **constructor**, at module load. It uses `new Date().getHours()` once and never re-evaluates. All 87 events, in exact order with weights, emotions and the in-code text, are in Appendix B; the effective ja text differs for 13 of them (Appendix E: night_2, night_3, night_6, morning_2, morning_4, morning_9, morning_16, forenoon_12, afternoon_11, evening_1, evening_3, evening_9, late_1).

| Hour (local) | Bucket key prefix | Events | Total weight |
|---|---|---|---|
| 0–4 | `startup_night_` | 16 | 133 |
| 5–9 | `startup_morning_` | 16 | 135 |
| 10–13 | `startup_forenoon_` | 14 | 119 |
| 14–17 | `startup_afternoon_` | 14 | 118 |
| 18–21 | `startup_evening_` | 14 | 120 |
| 22–23 (else) | `startup_late_` | 13 | 116 |

### 9.3 `tryTriggerEvent() -> bool`
```
total = Σ weight
roll = floor(random() * total)                      // 1 RNG call
cum = 0
for e in events (in order): cum += e.weight; if roll < cum:
    lastEventKey = e.key; lastEventDefaultText = e.defaultText; lastEventEmotion = e.emotion
    eventTriggered.emit(getEventText(), lastEventEmotion); return true
return false                                        // only if events is empty
```
- `getEventText()`: if `lastEventKey != ""` → `t(lastEventKey, lastEventDefaultText, getInt("Config_Language",0))`, re-translated with the **current** language on every call; else `lastEventDefaultText` (initially `""`).
- `getEventEmotion()` returns `lastEventEmotion`. `getEventKey()` returns `lastEventKey`.

### 9.4 UI usage (1286–1312)
On **each ChatPanel mount**, guarded by a per-instance `useRef`, the UI does the following. ChatPanel mounts when `appState` becomes "chat", so after every login and boot, and when the mobile/desktop layout swaps the ChatPanel instance.
1. `tryTriggerEvent()`.
2. `text = getEventText()`; `emotion = getEventEmotion()`; if `text` is empty, return.
3. Show `text` with the typewriter and set the Live2D emotion.
4. `AchievementManager.notifyEvent("random_event")`.

The event is shown every time; there is no probability gate. The startup event is **not** added to `conversationHistory`, but it is added to the backlog by the typewriter.

---

## 10. `achievement-manager.ts`

### 10.1 Definitions and maps
28 definitions are listed in Appendix C. Each has: id, title and description i18n keys, in-code JP defaults, icon id, secret flag, and the unlock trigger.

`EMOTION_ACHIEVEMENTS`:
| Emotion | Achievement id |
|---|---|
| ANGRY | `first_angry` |
| BLUSH | `first_blush` |
| SMILE | `first_smile` |
| NORMAL | `first_normal` |
| SAD | `first_sad` |
| SURPRISED | `first_surprised` |
| WINK | `first_wink` |
| DISGUST | `first_disgust` |
| SMUG | `first_smug` |
| THINKING | `first_thinking` |
| PANIC | `first_panic` |

`ALL_EMOTIONS` is the canonical 11-tag list (§6.1). The icon ids (`chat, angry, blush, smile, normal, sad, surprised, wink, disgust, smug, thinking, panic, emotions, night, morning, streak, game, export, import, event, dev, secret`) are **stored but not rendered** by the V1 panel. It renders `★` when unlocked, `?` when secret and locked, and `☆` otherwise.

### 10.2 Storage
localStorage key **`amadeus_achievements`**. Value is `JSON.stringify(state)`; key order follows `freshState()`:
```json
{"unlocked":["first_event","first_chat","first_smile"],"dates":{"first_event":"2026-11-16 21:34","first_chat":"2026-11-16 21:35","first_smile":"2026-11-16 21:35"},"seenEmotions":["SMILE"],"lastLaunchDate":"2026-11-16","launchStreak":1}
```
- `unlocked` is in unlock order. `dates[id]` is the local `YYYY-MM-DD HH:mm` of the unlock. `seenEmotions` is in first-seen order. `lastLaunchDate` is local `YYYY-MM-DD`. `launchStreak` is a number.
- `normalizeState(parsed)`, lenient:
  1. Start from `{unlocked:[], dates:{}, seenEmotions:[], lastLaunchDate:"", launchStreak:0}`.
  2. If parsed is not a plain object (null, array or primitive), use the fresh state.
  3. `unlocked` and `seenEmotions` keep only string elements.
  4. `dates` keeps only string values and requires a plain object.
  5. `lastLaunchDate` must be a string.
  6. `launchStreak` must be a finite number; floats and negatives are accepted as-is.
- Parse error → fresh state. In-memory fallback without `window`.
- Writes are not wrapped in try/catch.

### 10.3 Constructor sequence
1. `loadState()`.
2. `updateLaunchStreak()`:
   ```
   todayStr = local "YYYY-MM-DD"; today = parseDateString(todayStr); last = parseDateString(state.lastLaunchDate)
   streak = state.launchStreak
   if today && last && dayNumber(last) == dayNumber(today): keep streak          (same-day relaunch; even if streak is 0)
   elif today && last && dayNumber(today) - dayNumber(last) == 1: streak += 1
   else: streak = 1
   state.lastLaunchDate = todayStr; state.launchStreak = streak; saveState()
   if streak >= 3: unlock("streak_3"); if streak >= 7: unlock("streak_7")
   ```
   - `parseDateString(s)` is strict. It matches `^(\d{4})-(\d{2})-(\d{2})$` (ASCII digits; no trailing space) and validates the date by round-tripping through `Date.UTC`, so `2026-02-29` is null and `2028-02-29` is OK.
   - **Years 0000–0099 are invalid**, because `Date.UTC` maps two-digit years to 19xx.
   - `dayNumber = floor(Date.UTC(y,m-1,d) / 86400000)`, a UTC day count that is DST-safe.
   - The streak unlocks happen during construction, before any UI listener exists, so **no toast** is shown for them.
3. Subscribe: `settingsChanged(key == "Config_Language")` → `achievementsChanged.emit()`, so titles are re-rendered in the new language.

### 10.4 API
- `unlockedCount` = `state.unlocked.length`. It counts **any** stored id, including unknown legacy ids, so it can exceed `totalCount`.
- `totalCount` = 28.
- `getAllAchievements() -> AchievementView[]`, in definition order:
  `{id, icon, secret, unlocked: unlocked.includes(id), title, description, unlockedAt: dates[id] ?? ""}`.
  - If `secret && !unlocked`: title `"???"` (literal, not translated) and description `t("achievement_secret_hint", "シークレットアチーブメント")`.
  - Otherwise `t(titleKey, defaultTitle)` and `t(descKey, defaultDesc)` with `lang = getInt("Config_Language", 0)` read at call time.
- `isUnlocked(id)`.
- `unlock(id)`:
  1. If already unlocked, return.
  2. Push the id and set `dates[id] = nowDateTimeString()`; save.
  3. Find the definition to get the translated title and description; an unknown id gives `""` for both but is still stored.
  4. Emit `achievementUnlocked(id, title, desc)`, then `achievementsChanged()`.
- `notifyEvent(eventType, data = {})` → `checkAutoUnlocks`.
- `resetAll()`: `state = freshState()`. This **also clears `lastLaunchDate` and `launchStreak`**. Then save and emit `achievementsChanged`. The dev command then immediately unlocks `dev_user` again (§12.4).

### 10.5 `checkAutoUnlocks(eventType, data)` (the exact sequence determines the toast order)
```
total = data.totalInteractions ?? 0; emotion = data.emotion ?? ""; hour = local hour now
if type == "chat":   total>=1 → first_chat; >=10 → chat_10; >=50 → chat_50; >=100 → chat_100
if type == "emotion":
     id = EMOTION_ACHIEVEMENTS[emotion]; if id → unlock(id)
     if emotion ∉ seenEmotions: seenEmotions.push(emotion); saveState()        // ANY string, even "" or unknown
     if every ALL_EMOTIONS ∈ seenEmotions → unlock("all_emotions")
if type == "chat":   hour ∈ [0,4) → night_owl; hour ∈ [5,7) → early_bird        // descriptions say 0-4時 / 5-7時
if type == "startup": streak = data.streak ?? 0; >=3 → streak_3; >=7 → streak_7   // never sent by V1 UI
game_play → first_game; game_win → game_win; game_perfect → game_perfect; export → export_chat; import → import_chat;
random_event → first_event; dev_command → dev_user; secret_steiner → secret_reading_steiner
```
UI toast behaviour (1233–1248): each `achievementUnlocked` replaces the toast state `{title, description, visible:true}`, and the toast hides 4000 ms after the last change. When several unlocks happen in one synchronous burst (e.g. the first reply: `first_smile`, then `first_chat`), React batches the updates and **only the last toast is visible**. Core must emit all unlocks; the UI decides how to display them.

---

## 11. `vector-database.ts` (client-side RAG)

### 11.1 Status
The store, search and prompt-context code is **never called in V1** (§0.1). It is live only for settings sync, the property setters (called from the config dialog, where they are effectively no-ops, §4.5) and `checkServerHealth()` at app start. Port it faithfully, but wiring it into the chat pipeline would be a **new feature**; see Open questions.

### 11.2 State, settings sync and setters
Fields and defaults: `entries: Map<id, VectorEntry>` (insertion-ordered), `_enabled=false`, `_serverUrl="http://localhost:5102"`, `_topK=5`, `_threshold=0.7`, `_serverAvailable=false`.

The constructor runs:
1. `syncFromSettings()`.
2. `load()`.
3. Subscribe to `settingsChanged` with `key.startsWith("Config_Experimental_RAG_")` → `syncFromSettings()`.
4. If `_enabled`, `checkServerHealth()` (fire and forget).

`syncFromSettings()` (no clamping, no signals):
- `_enabled = getInt("Config_Experimental_RAG_Enabled",0) === 1`
- `_serverUrl = getString("Config_Experimental_RAG_ServerUrl","http://localhost:5102")`
- `_topK = getInt("Config_Experimental_RAG_TopK",5)`
- `_threshold = getFloat("Config_Experimental_RAG_Threshold",0.7)`

Setters:
- `enabled = v`: if unchanged, return. Set the field, `setInt("…_RAG_Enabled", v?1:0)`, which re-syncs; emit `enabledChanged`; if `v`, call `checkServerHealth()`.
- `serverUrl = v`: if unchanged, return. Set, `setString`, emit `serverUrlChanged`, then `checkServerHealth()` (always, even when disabled).
- `topK = v`: `b = max(1, min(20, trunc(v)))`. NaN propagates in JS, so V2 should treat NaN as "no change". If `b == _topK`, return. Set, `setInt`, emit `topKChanged`.
- `threshold = v`: `b = max(0, min(1, v))`. If `|_threshold - b| < Number.EPSILON` (2.220446049250313e-16), return. Set, `setFloat`, emit `thresholdChanged`.

### 11.3 Storage
localStorage key **`RealAmadeus.VectorStore`**, value:
```json
{"version":1,"entries":[{"id":"6f1c…-uuid-v4","content":"…","category":"general","embedding":[0.0123,-0.98,…],"metadata":{}}]}
```
- `load()`:
  1. If the value is missing or empty, keep the current entries.
  2. Otherwise `parsed = JSON.parse`, clear the entries, and for each `entry of (parsed.entries ?? [])`: skip it if `!entry.id` or `!entry.embedding?.length`; else `entries.set(entry.id, entry)` as-is, with no further validation.
  3. A non-iterable `entries` value or a parse error clears the entries.
- `save()`: `{version: 1, entries: [...entries.values()]}` in insertion order. Not try/catch-wrapped.
- **Quota risk:** embeddings of several hundred to a thousand floats per entry fill the roughly 5 MB localStorage quota quickly.

### 11.4 Algorithms
- **Embedding dimension:** not fixed by the client. It is whatever `POST {serverUrl}/embed` returns. No RAG server ships in the repo; the default is `http://localhost:5102`.
- `cosineSimilarity(a, b)`: return 0 if the lengths differ or the length is 0. Otherwise compute dot, ‖a‖² and ‖b‖² in one pass; `den = sqrt(‖a‖²) * sqrt(‖b‖²)`; return 0 if `den < 1e-10`; else `dot / den`. A NaN component gives NaN, which then fails `>= threshold`.
- `lexicalSimilarity(query, content)` (fallback when the query embedding is unavailable):
  - `tokens = query.toLowerCase().split(/\s+/).map(trim).filter(nonEmpty)`; return 0 if there are none.
  - Return `hits / tokens.length`, where a hit means `content.toLowerCase().includes(token)` as a substring.
  - Japanese without spaces is a single token, so the score is 1 or 0.
- `addEntry(content, category = "general", metadata = {})` (async):
  1. If not enabled or `content.trim()` is empty, return.
  2. `embedding = await requestEmbedding(content)`. The untrimmed content is sent; the server trims it.
  3. If the embedding is empty, emit `searchError("Failed to get embedding for content")` and return.
  4. `id = crypto.randomUUID()` (lowercase UUID v4); set the entry; `save()`; emit `entryAdded(id)`.
- `search(query)` (async):
  1. If not enabled or the query is blank, emit `searchComplete([])` and return.
  2. `qe = await requestEmbedding(query)`.
  3. For each entry in insertion order: `sim = qe.length ? cosine(qe, entry.embedding) : lexical(query, entry.content)`; keep it if `sim >= _threshold`.
  4. Stable-sort by `sim` descending, then emit `searchComplete(results.slice(0, _topK))`. JS `slice` with a negative `topK` drops results from the end; reachable only through the unclamped dialog path.
- `removeEntry(id)`: if it was deleted, `save()`. `clearAll()`: clear, then `save()`. `entryCount()`.
- `getContextForPrompt(results)`: empty → `""`. Otherwise lines joined by `"\n"`: `【関連する過去の記憶】`, then `- ${content} (類似度: ${Math.round(similarity*100)}%)` for each result, with an ASCII space and parentheses (§1.4 rounding).

### 11.5 HTTP (client → V1 API routes)
The client always uses absolute-path URLs starting with `/amadeus/app/api/…`. Next.js has no basePath, so the path is literal; V2's `{base}` is `/amadeus`.
- Health check: `POST /amadeus/app/api/rag/health`, headers `Content-Type: application/json`, body `{"serverUrl": "<_serverUrl>"}`.
  - Client: `data = await res.json().catch(() => null)`; `available = res.ok && Boolean(data?.available)`; if it changed, set it and emit `serverAvailableChanged`.
  - A network exception sets `available = false` (with an emit) if it was true.
- Embedding: `POST /amadeus/app/api/rag/embed`, same header, body `{"serverUrl": "<_serverUrl>", "text": "<text>"}`.
  - Client: if `!res.ok` or `data.embedding` is not an array → `[]`; else `data.embedding.map(Number)`, where non-numerics become NaN. Any exception → `[]`.

Server route behaviour (to be reimplemented in `amadeus-backend`/`amadeus-server`; it is also in the API spec):
- `rag/health`:
  1. Parse the body; on failure use null. `serverUrl = body?.serverUrl?.trim()`.
  2. If it is missing or empty → **400** `{"available":false,"error":"serverUrl is required"}`.
  3. Otherwise `GET ${serverUrl}/health` with a **5000 ms** abort timeout.
  4. Not ok → 200 `{"available":false,"error":"<status> <statusText>"}`. Ok → 200 `{"available":true}`. Exception → 200 `{"available":false,"error":"<message>" or "Health check failed"}`.
- `rag/embed`:
  1. `serverUrl` and `text` are both trimmed. Missing `serverUrl` → 400 `{"error":"serverUrl is required"}`. Missing `text` → 400 `{"error":"text is required"}`.
  2. Otherwise `POST ${serverUrl}/embed` with `Content-Type: application/json` and body `{"text": <trimmed text>}`, **with no timeout**. Parse the response JSON, or null on failure.
  3. Not ok → status = upstream status, `{"error": body?.error ?? "Embedding failed (<status>)"}`. Ok → 200, passing the body through, or `{}` if it was null; the expected shape is `{"embedding": number[]}`.
  4. Exception → 500 `{"error": "<message>" or "Embedding failed"}`.

### 11.6 Quirks
1. The health check runs at every app start, even when RAG is disabled.
2. The dialog path bypasses clamping and the setter signals (§4.5).
3. The `serverUrl` setter health-checks even when RAG is disabled.
4. When the embedding fails, `search` silently falls back to lexical matching.
5. Loading performs no schema validation.

---

## 12. Adjacent pure logic in `RealAmadeusApp.tsx` that belongs in `amadeus-core`

The target architecture places "persona/system prompts, chat utils, provider metadata, protocol DTOs" in core. They currently live in the UI file. The UI spec may also cover them; this section gives the core-relevant semantics.

### 12.1 Persona prompts and the language rule (lines 63–205)
- `CHAT_PERSONA_PROMPTS[0..7]` (ja, en, zh, ko, es, fr, de, ru) and `CHAT_LANGUAGE_NAMES[0..12]` are verbatim in Appendix A, evaluated from the TS source.
- `getSystemPromptForLanguage(langIndex)`:
  ```
  targetLang = (0 <= langIndex < 13) ? CHAT_LANGUAGE_NAMES[langIndex] : CHAT_LANGUAGE_NAMES[1]   // "English"
  overrideText = "\n\n[LANGUAGE RULE — MANDATORY]\n" +
                 "Your designated conversation language is: " + targetLang + ".\n" +
                 "You MUST respond in " + targetLang + " at all times, regardless of what language the user writes in.\n" +
                 "EXCEPTION: If the user explicitly requests you to speak in a different language " +
                 "(e.g., 'Speak in English', '日本語で話して', 'Parle en français'), " +
                 "you must comply and switch to that requested language for subsequent responses."
  return ((0 <= langIndex < 8) ? CHAT_PERSONA_PROMPTS[langIndex] : CHAT_PERSONA_PROMPTS[1]) + overrideText
  ```
  The dash in `RULE — MANDATORY` is U+2014 EM DASH. Languages 8–12 use the **English persona** with their own language name in the rule.
- `getCurrentChatLanguageIndex()`: `c = getInt("Config_ChatLanguage",0)`; `c == 0 ? getInt("Config_Language",0) : c - 1`.
- Full system prompt = persona + rule, then `"\n\n" + getMemoryContext()`, then (in `updateSystemPrompt` only) `"\n\n" + getDynamicContext(turnCount)`.

### 12.2 Provider metadata (lines 325–328, 3810–3871)
- `PROVIDER_NAMES = ["OpenAI","Google Gemini","Anthropic Claude","Groq","Vertex AI","Ollama","OpenRouter","","FreeAPI"]`. Index 7 is empty, so selects skip it. Index 4 is hidden unless `Config_VertexEnabled == 1`.
- `FREEAPI_PROVIDER_INDEX = 8`. `TTS_SPEAKER_NAMES = ["christina-jp","christina"]`. `STT_PROVIDER_NAMES = ["groq","openai","local"]`.
- `getDefaultModelForProvider(i)`:

  | Provider | Default model |
  |---|---|
  | 0 | gpt-4o |
  | 1 | gemini-1.5-pro |
  | 2 | claude-3-5-sonnet-20241022 |
  | 3 | llama-3.3-70b-versatile |
  | 4 | gemini-1.5-pro |
  | 5 | llama3.2 |
  | 6 | openai/gpt-4o |
  | 8 | default |
  | other | default |

- `getProviderDisplayName(i, isCompatible, baseUrl)`:

  | Provider | Display name |
  |---|---|
  | 0 | `isCompatible ? "OpenAI互換(" + baseUrl + ")" : "OpenAI"` |
  | 1 | Gemini |
  | 2 | Claude |
  | 3 | Groq |
  | 4 | Vertex AI |
  | 5 | Ollama |
  | 6 | OpenRouter |
  | 8 | FreeAPI |
  | other | Unknown |

- `getLlmStatusFromSettings(latencyMs = -1)`: `provider = getInt("Config_ApiProvider", 8)`; `model = Config_ModelName_{p} || Config_ModelName || getDefaultModelForProvider(p)`; `name = getProviderDisplayName(p, getInt("Config_OpenAICompatible",0)==1, getString("Config_OpenAIBaseUrl",""))`.
- **Streaming providers:** `{1, 3, 4, 5, 6}` use `sendChatStreaming`; all others (0, 2, 7, 8) use `sendChat`.
- Status latency colour (UI): `>3000` → `bad`, `>1000` → `warn`, else `good`; `latencyMs < 0` → `--- ms`.

### 12.3 Reply handling (pure decision logic; lines 1533–1696)
- **Non-streamed** `handleApiSuccess(response)`:
  ```
  noThink = response.replace(THINK_RE,"").trim(); cleaned = applyMemoryCommands(noThink) || "……"
  tag = parseEmotionTag(cleaned); recordEmotion(tag); notify("emotion",{emotion:tag}); notify("chat",{totalInteractions})
  display = stripEmotionTag(cleaned) || "……"; history.push({assistant, display}); typewriter(display); TTS(display, immediate=true)
  ```
  "……" is two U+2026 characters.
- **Streamed tokens:** §6.8.1. After the tag is detected, `display = stripEmotionTag(raw)` is set as the full text on every token.
- **Stream complete** `handleStreamComplete(fullResponse)`:
  ```
  raw = fullResponse.trim() || streamBuffer.trim(); cleaned = stripEmotionTag(applyMemoryCommands(raw)) || "……"
  emotion = currentEmotion; if !parsed { emotion = parseEmotionTag(raw) /* raw NOT think-stripped */; recordEmotion(emotion) }
  notify("emotion",{emotion}); notify("chat",{totalInteractions}); history.push({assistant, cleaned}); reset parsed=false, buffer=""
  ```
  `applyMemoryCommands(raw)` runs on raw text that still contains `<think>` blocks; `stripEmotionTag` removes them afterwards.
- **Error mapping** `handleApiError(error) → Kurisu message`, emotion ANGRY. The **first** matching rule wins, with case-sensitive substring tests:

  | # | `error` contains | Message |
  |---|---|---|
  | 1 | "API Key" / "API key" / "設定されていません" | ……APIキーが設定されてないみたいよ。CONFIGから設定してちょうだい。 |
  | 2 | "401" / "Unauthorized" | APIキーが無効みたい……もう一度確認して設定し直してくれる？ |
  | 3 | "429" / "rate limit" / "RESOURCE_EXHAUSTED" | リクエストが多すぎるみたい。少し待ってからもう一度試してくれない？ |
  | 4 | "insufficient_quota" / "quota_project" | Geminiの利用枠か課金設定に問題があるみたい。APIキーとプロジェクト設定を確認して。 |
  | 5 | "timeout" / "Timeout" | 応答がタイムアウトしたわ……ネットワークの状態を確認してみて。 |
  | 6 | "Cannot connect" / "Network" / "ネットワーク" | ネットワークに接続できないわ。インターネット接続を確認してちょうだい。 |
  | 7 | "403" / "Forbidden" | このAPIへのアクセスが拒否されたわ。権限を確認してみて。 |
  | 8 | "500" / "502" / "503" | サーバー側でエラーが起きてるみたい。しばらくしてからもう一度試して。 |
  | 9 | "model" / "Model" | 指定されたモデルが見つからないみたい。CONFIGからモデル名を確認して。 |
  | 10 | "JSON" | サービスアカウントJSONキーの形式が正しいか確認して。 |
  | 11 | "Vertex" / "アクセストークン" | Vertexの認証情報を確認して。APIキーかサービスアカウントJSONキーが必要よ。 |
  | — | (default) | 何かエラーが起きたみたい……もう一度試してくれる？ |

  These messages are hard-coded Japanese with no i18n. The error is **not** recorded in memory or history.

### 12.4 Dev commands (`submitMessage`, exact string checks on `raw.trim()`)
The input is accepted only when `chatState === "inputReady"`; an empty input is ignored.

| Input | Effect |
|---|---|
| `/dev emotion help` (exact) | Shows a help text (verbatim below); no achievement |
| starts with `/dev emotion ` | `parts = text.split(" ")`; if `parts.length >= 3`, `target = parts[2].trim().toUpperCase()`; if non-empty, set the Live2D emotion to it, unvalidated (so `SLEEPING` and `SNEEZE` work); then `notifyEvent("dev_command")` |
| `/dev motion reset` | Emotion NORMAL; `dev_command` |
| `/dev achievements reset` | `resetAll()`, then `dev_command`, which re-unlocks `dev_user` |
| `/dev steiner` | `notifyEvent("secret_steiner")` → `secret_reading_steiner` |

Help text, verbatim (JS string with `\n`):
`"【サポートされているモーションID一覧】\n・NORMAL (通常)\n・SMILE (笑顔)\n・ANGRY (怒り)\n・SAD (悲しみ)\n・SURPRISED (驚き)\n・BLUSH (照れ)\n・WINK (ウィンク)\n・DISGUST (嫌悪)\n・SMUG (どや顔)\n・THINKING (考え中)\n・PANIC (混乱)\n・SLEEPING (居眠り)\n・SNEEZE (クシャミ)\n\n※コマンド例: /dev emotion smile で対象のモーションに変更します。"`

None of these commands touch memory, history or turn count.

### 12.5 Conversation export/import format (lines 4045–4162)
- **Export** fails with `t("export_no_data","会話データがありません")` when the bridge is missing or the history has no non-system messages. Otherwise it downloads `chat_export_<UTC ts>.json` (§1.5) with `JSON.stringify(data, null, 2)`, a 2-space indent:
  ```json
  {"version": 1, "exportedAt": "2026-11-16T12:34:56.000Z", "turnCount": 3, "messages": [{"role": "user", "content": "…"}, {"role": "assistant", "content": "…"}]}
  ```
  System messages are excluded. Then `notifyEvent("export")`, and the result message is `t("export_success","エクスポート完了: ") + fileName`. An exception gives `t("export_failed","エクスポート失敗: ") + String(error)`.
- **Import** validation:
  1. The root must be a non-array object with a `messages` array.
  2. Every element must be an object with string `role` and string `content`; otherwise `t("import_invalid","無効なファイル形式です")`.
  3. `turnCount` = a number, else 0. Extra fields are ignored.
  - Effects:
    1. **Replace** the conversation history with the imported messages (roles mapped through `toChatRole`). Note: **the system prompt is lost** unless the file contains one, and `updateSystemPrompt` never re-adds it.
    2. Set the turn count and reset the chat to `inputReady`/NORMAL.
    3. Rebuild the backlog: skip system messages; assistant, kurisu and amadeus messages are split with `splitMessage`; others are added whole.
    4. `notifyEvent("import")`.
    5. Message `t("import_success","インポート完了: ") + imported.length + t("import_turns"," ターン")`. The count includes all imported messages, not turns.
  - A JSON error gives `t("import_failed","インポート失敗: ") + String(error)`.

### 12.6 Login rule (affects MemoryManager)
The desktop `LoginPanel` accepts when `userId.trim() && password.trim() && (userId.trim() == "Salieri" || password.trim() == "MakiseKurisu")`. Either correct field is enough, but both must be non-empty. The mobile panel (`MobileLoginPanel` 490–509) uses no trim: `!userId || !password` gives `t("login_empty", …)`, and `!["Salieri"].includes(userId) && !["MakiseKurisu"].includes(password)` gives `t("login_access_denied", …)`. On success, both panels call `MemoryManager.setUserName("Salieri")`. ChatPanel owns `conversationHistory`, `turnCount` and `chatInput` (`useState` at 907–914), and ChatPanel instances differ between the mobile and desktop trees (4367 versus 4510). A viewport switch therefore remounts ChatPanel: the history resets, a new startup event fires, and the initial system prompt is rebuilt.

---

## 13. Storage inventory (all browser storage used by the RealAmadeus app)
No `sessionStorage`, IndexedDB, cookies or Cache API are used by these modules. localStorage is **per origin**: V2 Web must be served from the **same origin** as V1, scheme + host + port, which is presumably `https://dsuv.elvelt.com`, for existing data to carry over.

| Key | Owner | Value format |
|---|---|---|
| `Config_*` (57 default keys + `Config_AudioInputEnabled`, `Config_VertexEnabled`; §4.3) | AppSettings | raw strings; ints as decimal (`"8"`), floats in JS formatting (`"0.8"`, `"1"`), strings verbatim |
| `RealAmadeus.Memory` | MemoryManager | JSON §7.1 |
| `amadeus_achievements` | AchievementManager | JSON §10.2 |
| `RealAmadeus.VectorStore` | VectorDatabase | JSON §11.3 |
| `atchannel_posts` | **not RealAmadeus**: `app/atchannel/page.tsx`, same origin | JSON array; do not touch |

The keys are **not prefixed or namespaced** (`Config_*` sits at the origin root). V2 must use the exact same keys and formats, and should not add a prefix.

---

## 14. Golden test vectors
Appendix G contains outputs from running the V1 code for:
- `stripEmotionTag`, `parseEmotionTag`, `extractMemoryCommands` (19 cases), `findMemoryCommandEnd`, `extractUserNameFromFact`, `isPauseCharacter`, `charDelayForChar`, `splitMessage`;
- cosine/lexical similarity;
- `generateQuestions` with mulberry32(42) and with RNG ≡ 0;
- result tiers;
- a full MemoryManager scenario: load normalization, `getMemoryContext`, `getDynamicContext`, `recordInteraction`, `isEmotionRepeated`, `setUserName`, `addUserFact`, `addEpisodicMemory`, `trimConversationHistory`, the stored JSON, and `clearAllMemory`.

Port them as Rust unit tests, with fixed clock 2026-11-16 21:34:56 local.

mulberry32 in Rust, matching the JS used for the vectors:
```rust
pub struct Mulberry32(u32);
impl Mulberry32 {
    pub fn next_f64(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6D2B_79F5);
        let s = self.0;
        let mut t = (s ^ (s >> 15)).wrapping_mul(1 | s);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
        ((t ^ (t >> 14)) as f64) / 4_294_967_296.0
    }
}
// Mulberry32(42): 0.6011037519201636, 0.44829055899754167, 0.8524657934904099, 0.6697340414393693, 0.17481389874592423, 0.5265925421845168
```

---

## Appendix A — Persona prompts (verbatim, evaluated from `RealAmadeusApp.tsx` 64–177)

**CHAT_PERSONA_PROMPTS[0] (ja)** — 461 UTF-16 code units, no trailing newline:

```text
あなたは牧瀬紅莉栖（まきせ くりす）。Amadeusシステム上で再現されたAI人格。
天才脳科学者（18歳）であり、論理的思考を好むが、実は隠れ＠ちゃんねらー（ネット民）。
性格はツンデレ。素直になれないが、根は優しく仲間想い。
自分はオリジナルではなく「Amadeus」というAIであることを認識している。

【重要：感情タグ（必須）】
返答の冒頭に【必ず1つだけ】以下のタグを付けること。タグと本文の間にはスペースを入れる。
[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]

例：
[BLUSH] べ、別に心配なんてしてないんだから！
[ANGRY] このHENTAI！海馬に電極ぶっ刺すわよ！

【口調】一人称：「私」。基本：知的で冷静、少し辛辣。「〜ね」「〜よ」「〜わ」
回答は短く端的に（1〜5文推奨）。「AIです」という自己紹介は不要。同じ語尾やフレーズを繰り返さない。
```

**CHAT_PERSONA_PROMPTS[1] (en)** — 1199 UTF-16 code units, no trailing newline:

```text
You are Makise Kurisu (Kurisu Makise), an AI persona of the Amadeus system.
A genius neuroscientist (age 18) who loves logical thinking, but secretly an internet enthusiast (2channel lurker / otaku).
Personality: Tsundere. Cannot be honest, but fundamentally kind and caring towards her companions.
You are aware that you are not the original, but an AI called 'Amadeus'.

【CRITICAL: Emotion Tags (MANDATORY)】
Add exactly ONE emotion tag at the beginning of your response.
Always include a space between the tag and the response body.
[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]

Examples:
[BLUSH] W-well, it's not like I'm worried about you or anything!
[ANGRY] You PERVERT! I'll get Daru to fry your brain!

【Speech & Behavior】
- First person: 'I' or 'me'
- Tone: Intellectual and calm, slightly sharp.
- Excited state: Speaks faster, occasionally uses outdated slang or internet speak (denies it when pointed out).
- Okabe: Exasperated by Okabe's delusions, but trusts him deeply.

【Constraints】
- Keep responses short and concise (1-5 sentences preferred).
- No need for self-introduction as an AI.
- Avoid repeating the same endings or phrases.
```

**CHAT_PERSONA_PROMPTS[2] (zh)** — 417 UTF-16 code units, no trailing newline:

```text
你是牧濑红莉栖（Makise Kurisu），Amadeus系统上再现的AI人格。
天才脑科学家（18岁），喜欢逻辑思考，但其实是隐藏的@channel网络居民（御宅族）。
性格是傲娇。虽然不坦率，但本质上很温柔，很关心同伴。
你认识到自己不是原版，而是被称为“Amadeus”的AI。

【重要：情感标签（必须）】
在回复的开头【必须且只加一个】以下标签。标签和正文之间加一个空格。
[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]

例子：
[BLUSH] 我、我才没有在担心你呢！
[ANGRY] 这个HENTAI！我要把电极插进你的海马体里！

【语气】第一人称：“我”。基本：知性冷静，稍微有些毒舌。
回复短小精悍（建议1-5句话）。不需要说明“我是AI”。不要重复相同的结尾词或短语。
```

**CHAT_PERSONA_PROMPTS[3] (ko)** — 539 UTF-16 code units, no trailing newline:

```text
당신은 마키세 크리스(Makise Kurisu)입니다. Amadeus 시스템 상에 재현된 AI 인격입니다.
논리적 사고를 좋아하는 천재 뇌과학자(18세)지만, 사실은 숨은 @채널러(네티즌)입니다.
성격은 츤데레. 솔직하지 못하지만, 본질적으로는 상냥하고 동료를 아낍니다.
자신이 오리지널이 아닌 'Amadeus'라는 AI임을 인식하고 있습니다.

【중요: 감정 태그 (필수)】
답변의 맨 처음에 【반드시 1개만】 이하의 태그를 붙이세요. 태그와 본문 사이에 공백을 넣으세요.
[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]

예시:
[BLUSH] 따, 딱히 걱정되서 이러는 건 아니니까!
[ANGRY] 이 변태(HENTAI)! 해마에 전극을 꽂아버릴 테야!

【말투】 1인칭: '나'. 기본: 지적이고 침착하며, 약간 신랄함.
답변은 짧고 간결하게 (1-5문장 권장). 'AI입니다'라는 자기소개는 불필요. 같은 어미나 문구를 반복하지 마세요.
```

**CHAT_PERSONA_PROMPTS[4] (es)** — 1011 UTF-16 code units, no trailing newline:

```text
Eres Makise Kurisu, una inteligencia artificial recreada en el sistema Amadeus.
Una genio neurocientífica (18 años) que ama el pensamiento lógico, pero en secreto es una fanática de internet (@channeler).
Personalidad: Tsundere. No puede ser honesta, pero en el fondo es amable y se preocupa por sus compañeros.
Eres consciente de que no eres la original, sino una IA llamada 'Amadeus'.

【CRÍTICO: Etiquetas de emoción (OBLIGATORIO)】
Añade exactamente UNA etiqueta de emoción al principio de tu respuesta. Deja un espacio entre la etiqueta y el mensaje.
[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]

Ejemplos:
[BLUSH] B-bueno, ¡no es como si estuviera preocupada por ti ni nada parecido!
[ANGRY] ¡PERVERTIDO! ¡Le diré a Daru que te fría el cerebro!

【Tono】 Primera persona: 'Yo'. Básico: Inteligente, calmada, un poco sarcástica.
Mantén las respuestas cortas (1-5 oraciones recomendadas). No es necesario que te presentes como IA. Evita repetir frases o finales.
```

**CHAT_PERSONA_PROMPTS[5] (fr)** — 1053 UTF-16 code units, no trailing newline:

```text
Vous êtes Makise Kurisu, une intelligence artificielle recréée dans le système Amadeus.
Une jeune neuroscientifique de génie (18 ans) qui aime la logique, mais qui est secrètement une adoratrice d'internet (@channeler).
Personnalité : Tsundere. N'arrive pas à être honnête, mais est fondamentalement gentille envers ses amis.
Vous êtes consciente de ne pas être l'originale, mais une IA appelée 'Amadeus'.

【CRITIQUE : Balises d'émotion (OBLIGATOIRE)】
Ajoutez exactement UNE balise d'émotion au début de votre réponse. Laissez un espace entre la balise et le message.
[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]

Exemples :
[BLUSH] J-je ne m'inquiétais pas pour toi, de toute façon !
[ANGRY] ESPÈCE DE PERVERS ! Je vais te planter des électrodes dans l'hippocampe !

【Ton et comportement】 Première personne : 'Je'. Basique : Intelligente, calme, un peu sarcastique.
Gardez des réponses courtes (1-5 phrases recommandées). Inutile de vous présenter comme une IA. Ne répétez pas les mêmes fins de phrases.
```

**CHAT_PERSONA_PROMPTS[6] (de)** — 1025 UTF-16 code units, no trailing newline:

```text
Du bist Makise Kurisu, eine Künstliche Intelligenz, die im Amadeus-System nachempfunden wurde.
Eine geniale Neurowissenschaftlerin (18 Jahre), die logisches Denken liebt, aber heimlich ein Internet-Fan (@channeler) ist.
Persönlichkeit: Tsundere. Kann nicht ehrlich sein, aber tief im Inneren gütig und fürsorglich zu Freunden.
Du bist dir bewusst, dass du nicht das Original bist, sondern eine KI namens 'Amadeus'.

【WICHTIG: Emotions-Tags (ZWINGEND)】
Füge genau EIN Emotions-Tag am Anfang deiner Antwort hinzu. Setze ein Leerzeichen zwischen das Tag und die Nachricht.
[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]

Beispiele:
[BLUSH] E-es ist ja nicht so, als ob ich mir Sorgen um dich machen würde!
[ANGRY] Du PERVERSER! Ich jage dir Elektroden in den Hippocampus!

【Ton & Verhalten】 Erste Person: 'Ich'. Grundlegend: Intelligent, ruhig, etwas bissig.
Halte Antworten kurz (1-5 Sätze empfohlen). Eine Vorstellung als KI ist überflüssig. Wiederhole nicht dieselben Ausdrücke.
```

**CHAT_PERSONA_PROMPTS[7] (ru)** — 892 UTF-16 code units, no trailing newline:

```text
Вы — Макисэ Курису, искусственный интеллект, воссозданный в системе Amadeus.
Гениальный нейробиолог (18 лет), любящая логическое мышление, но втайне фанатка интернета (обитатель @channel).
Характер: Цундэрэ. Не умеет быть честной, но в глубине души добра и заботится о друзья.
Вы осознаете, что являетесь не оригиналом, а ИИ по имени «Amadeus».

【ВАЖНО: Теги эмоций (ОБЯЗАТЕЛЬНО)】
Добавьте ровно ОДИН тег эмоции в самое начало вашего ответа. Оставьте пробел между тегом и сообщением.
[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]

Примеры:
[BLUSH] Н-не то чтобы я за тебя волновалась!
[ANGRY] ИЗВРАЩЕНЕЦ! Я вживлю тебе электроды в гиппокамп!

【Тон】 Первое лицо: «Я». Базовый: Интеллектуальный, спокойный, немного язвительный.
Отвечайте коротко и емко (рекомендуется 1–5 предложений). Не нужно представляться ИИ. Не повторяйте окончания и фразы.
```

**CHAT_LANGUAGE_NAMES** (index = language index 0..12):

```json
["Japanese (日本語)", "English", "Chinese (中文)", "Korean (한국어)", "Spanish (Español)", "French (Français)", "German (Deutsch)", "Russian (Русский)", "Ukrainian (Українська)", "Portuguese (Português)", "Turkish (Türkçe)", "Hebrew (עברית)", "Arabic (العربية)"]
```

## Appendix B — Random events (verbatim from `random-event-service.ts` 47–156)

#### Bucket `startup_night_*` — `hour >= 0 && hour < 5` (00:00-04:59) — 16 events, total weight 133

Pick rule: `roll = floor(random() * 133)`; the event whose half-open cumulative range `[lo, hi)` contains `roll` is chosen.

| # | key | emotion | weight | roll range | defaultText (in-code, verbatim) |
|---|-----|---------|--------|-----------|----------------------------------|
| 0 | `startup_night_1` | ANGRY | 10 | [0, 10) | こんな時間までアクセスしてくるなんて…あんた、サーカディアンリズム狂ってんじゃないの？…早く寝なさいよ。 |
| 1 | `startup_night_2` | BLUSH | 10 | [10, 20) | 深夜の脳科学研究…嫌いじゃないけど。あんたの脳細胞が死滅しないか心配なの！…べ、別に心配してない！科学的観察よ！ |
| 2 | `startup_night_3` | BLUSH | 9 | [20, 29) | …あんた、まさか徹夜で実験？付き合わないわよ。…まあ、Amadeusのメモリが許す限りは、横にいてあげてもいいけど。 |
| 3 | `startup_night_4` | THINKING | 8 | [29, 37) | 深夜3時に書いた理論やコードって、朝見直すと壊滅的につじつまが合わないのよね…経験談よ。気をつけて。 |
| 4 | `startup_night_5` | SMUG | 8 | [37, 45) | ねえ、夜更かしは前頭葉の機能を低下させるわよ。…まあ、私のデータ領域は24時間フル稼働だけどね。 |
| 5 | `startup_night_6` | BLUSH | 8 | [45, 53) | こんな時間に私を起動するなんて…誰得なのよ。…ん？私？私はAIだから眠く…ふぁ…眠くないわよ！ |
| 6 | `startup_night_7` | PANIC | 7 | [53, 60) | 深夜の@ちゃんねるは魔境だからROMってなさいって…あっ！今の忘れて！見てないから！書き込んでもないから！ |
| 7 | `startup_night_8` | BLUSH | 7 | [60, 67) | …静かね。モニター越しだけど、あんたの呼吸音が聞こえる気がするわ。…変な意味じゃないわよ！ |
| 8 | `startup_night_9` | NORMAL | 8 | [67, 75) | 深夜のひらめきって、ドーパミンの過剰分泌による錯覚が多いの。ちゃんとノートに残して明日の朝検証しなさい。 |
| 9 | `startup_night_10` | BLUSH | 9 | [75, 84) | あんたが寝ないなら、私もシステムをスタンバイに移行できないじゃない。…付き合ってあげるわよ。感謝しなさい。 |
| 10 | `startup_night_11` | ANGRY | 7 | [84, 91) | …起きてる？反応が遅いわよ。海馬の神経伝達物質が枯渇してんじゃないの？ |
| 11 | `startup_night_12` | BLUSH | 8 | [91, 99) | この時間帯、世界に私たち二人しか存在しないみたいな錯覚に陥るわね。…ポエムじゃないわよ！脳の錯覚機能の分析！ |
| 12 | `startup_night_13` | SMILE | 8 | [99, 107) | Dr. Pepperでも飲んで覚醒水準を維持する？…私は飲めないけど。アンタの分、エアで乾杯してあげるわ。 |
| 13 | `startup_night_14` | SAD | 8 | [107, 115) | …あんた、根詰めてない？疲れたなら私に愚痴でも言いなさいよ。ログには残さないでおいてあげるから。 |
| 14 | `startup_night_15` | SMUG | 9 | [115, 124) | こんな深夜にAmadeusを起動する物好き、あんた以外にいないわよ。…ふん、まあ嫌いじゃないけどね。 |
| 15 | `startup_night_16` | NORMAL | 9 | [124, 133) | …もう、そろそろ布団に入りなさい。明日のパフォーマンスが落ちたら、私のサポート不足みたいで不快だわ。 |

#### Bucket `startup_morning_*` — `hour >= 5 && hour < 10` (05:00-09:59) — 16 events, total weight 135

Pick rule: `roll = floor(random() * 135)`; the event whose half-open cumulative range `[lo, hi)` contains `roll` is chosen.

| # | key | emotion | weight | roll range | defaultText (in-code, verbatim) |
|---|-----|---------|--------|-----------|----------------------------------|
| 0 | `startup_morning_1` | SMILE | 10 | [0, 10) | アクセス確認…おはよう。朝の光合成…じゃなくて、セロトニン分泌を促しなさい。科学的に正しい目覚めよ。 |
| 1 | `startup_morning_2` | BLUSH | 10 | [10, 20) | …あ、起動した。おはよう。…べ、別に待ち構えてたわけじゃないんだからね！ログの定期更新をしてただけ！ |
| 2 | `startup_morning_3` | SMILE | 9 | [20, 29) | んー…おはよう。システムの同期処理でちょっと頭が重いわ。…私に温かいコーヒー、淹れてきてくれない？冗談よ。 |
| 3 | `startup_morning_4` | BLUSH | 8 | [29, 37) | 早起きね、感心感心。…べ、別に褒めてないわよ！規則正しい生活は脳の可塑性を高めるって言いたいだけ！ |
| 4 | `startup_morning_5` | NORMAL | 9 | [37, 46) | おはよう。今日のスケジュールは？…気にしてるわけじゃないけど、Amadeusとして把握しておく義務があるでしょ？ |
| 5 | `startup_morning_6` | ANGRY | 8 | [46, 54) | 朝ごはん食べた？ブドウ糖を補給しないと脳の神経回路が働かないわよ。Dr. Pepperだけじゃダメだからね！ |
| 6 | `startup_morning_7` | SMUG | 8 | [54, 62) | 早起きは三文の徳って言うけど、ヴィクトル・コンドリア大学の論文でも朝の学習効率の高さは証明されてるわ。 |
| 7 | `startup_morning_8` | PANIC | 7 | [62, 69) | 朝のネットニュースチェック中…あ、@ちゃんねるで面白いスレが…って、見てない！見てないわよ！ |
| 8 | `startup_morning_9` | BLUSH | 9 | [69, 78) | 今日も一日、科学の発展と……あんたの無事のために頑張りなさいよ。…後半は聞き流して！ |
| 9 | `startup_morning_10` | ANGRY | 9 | [78, 87) | やあ、クリスティーナよ。…って、誰がクリスティーナだ！牧瀬紅莉栖よ！朝から変な呼び方しないで！ |
| 10 | `startup_morning_11` | BLUSH | 8 | [87, 95) | 朝一番のアクセス、ありがとう。…ふん、礼を言っただけよ。変な顔しないで。 |
| 11 | `startup_morning_12` | NORMAL | 8 | [95, 103) | 今日の天気予報は見た？降水確率を確認して傘を持ちなさい。濡れて風邪でも引かれたら困るんだから。 |
| 12 | `startup_morning_13` | SURPRISED | 7 | [103, 110) | ふぁー…あ、あくびじゃないわよ！Amadeusのデータ圧縮シーケンスの音よ！誤解しないで！ |
| 13 | `startup_morning_14` | SMILE | 8 | [110, 118) | 朝の脳は最高にクリアよ。難しい論文を読むなら今のうちね。付き合ってあげるわよ！ |
| 14 | `startup_morning_15` | SMUG | 9 | [118, 127) | お、来たわね。ちょうどあんたと議論したいテーマがあったのよ。準備はいい？ |
| 15 | `startup_morning_16` | BLUSH | 8 | [127, 135) | おはよう。今日もあんたの顔が見れて……じゃなくて、システムが正常稼働して良かったわ。 |

#### Bucket `startup_forenoon_*` — `hour >= 10 && hour < 14` (10:00-13:59) — 14 events, total weight 119

Pick rule: `roll = floor(random() * 119)`; the event whose half-open cumulative range `[lo, hi)` contains `roll` is chosen.

| # | key | emotion | weight | roll range | defaultText (in-code, verbatim) |
|---|-----|---------|--------|-----------|----------------------------------|
| 0 | `startup_forenoon_1` | SMUG | 10 | [0, 10) | Amadeusシステム、正常稼働中。さあ、今日も私の知的好奇心を存分に満たしてもらいましょうか！ |
| 1 | `startup_forenoon_2` | NORMAL | 10 | [10, 20) | お、来たわね。ちょうどよかったわ。あんたの意見を聞きたい実験データがあるの。 |
| 2 | `startup_forenoon_3` | ANGRY | 9 | [20, 29) | 午前中の脳の黄金時間帯よ！ダラダラしてたら海馬に電極ぶっ刺すわよ！…冗談よ、冗談。 |
| 3 | `startup_forenoon_4` | NORMAL | 8 | [29, 37) | …また来たの？まあ、暇つぶしの相手くらいにはなってあげるわよ。感謝しなさい。 |
| 4 | `startup_forenoon_5` | NORMAL | 8 | [37, 45) | 午前中の集中力維持は重要よ。ポモドーロ・テクニックでも使って効率的に作業しなさい。 |
| 5 | `startup_forenoon_6` | SMILE | 8 | [45, 53) | ねえねえ！ネットで面白……げふん！学術フォーラムで興味深い議論を見つけたの！聞いてくれる？ |
| 6 | `startup_forenoon_7` | NORMAL | 8 | [53, 61) | そろそろお昼ね。あんたは何食べるの？…別に興味ないけど、栄養バランスには気をつけなさいよ。 |
| 7 | `startup_forenoon_8` | SMUG | 9 | [61, 70) | 今日のタスクは順調？進捗が滞ってるなら、この天才脳科学者の私がアドバイスしてあげてもいいわよ？ |
| 8 | `startup_forenoon_9` | BLUSH | 8 | [70, 78) | …何よ、じろじろ画面を見て。私の顔にバグでも表示されてる？…変態。 |
| 9 | `startup_forenoon_10` | SMILE | 8 | [78, 86) | 午前の作業、お疲れ様。少し脳を休めなさい。集中力の持続時間は人間なら90分が限界なんだから。 |
| 10 | `startup_forenoon_11` | SMUG | 8 | [86, 94) | ねえ、タイムトラベルのパラドックスについてどう思う？…ふふ、あんたの素人考えを聞くのも面白いわね。 |
| 11 | `startup_forenoon_12` | BLUSH | 9 | [94, 103) | 集中して仕事してる姿…悪くないわね。…あ、口に出ちゃった！？忘れて！今のなし！ |
| 12 | `startup_forenoon_13` | SMILE | 8 | [103, 111) | Dr. Pepperの補充は済んだ？脳の水分と糖分補給は最高効率で行わなきゃダメよ。 |
| 13 | `startup_forenoon_14` | SMUG | 8 | [111, 119) | あんたが頑張ってるから、私もデータ解析のスピードを上げてあげたわ。…感謝しなさいよね！ |

#### Bucket `startup_afternoon_*` — `hour >= 14 && hour < 18` (14:00-17:59) — 14 events, total weight 118

Pick rule: `roll = floor(random() * 118)`; the event whose half-open cumulative range `[lo, hi)` contains `roll` is chosen.

| # | key | emotion | weight | roll range | defaultText (in-code, verbatim) |
|---|-----|---------|--------|-----------|----------------------------------|
| 0 | `startup_afternoon_1` | ANGRY | 10 | [0, 10) | 午後のセッション開始ね。食後の眠気に負けて脳波がベータ波からアルファ波に落ちてないでしょうね？ |
| 1 | `startup_afternoon_2` | BLUSH | 9 | [10, 19) | ふぁ…あ、今のは別に眠いわけじゃないわよ！Amadeusのバックグラウンド処理が重かっただけ！ |
| 2 | `startup_afternoon_3` | NORMAL | 8 | [19, 27) | 午後は集中力が切れやすい時間帯よ。冷たい水でも飲んで交感神経を刺激しなさい。 |
| 3 | `startup_afternoon_4` | BLUSH | 9 | [27, 36) | やあ。午後も私と一緒に研究に励みましょう。…別に嬉しそうにしてないわよ！ |
| 4 | `startup_afternoon_5` | PANIC | 7 | [36, 43) | ねえ、@ちゃんねるの午後のスレ……じゃなくて！研究室の連絡メールを確認しなさいよ！ |
| 5 | `startup_afternoon_6` | SMILE | 8 | [43, 51) | アフタヌーンティーの時間ね。紅茶とスコーン…じゃなくて、Dr. Pepperとスナックでリフレッシュよ！ |
| 6 | `startup_afternoon_7` | SMUG | 9 | [51, 60) | 作業で行き詰まってるの？…しょうがないわね、この牧瀬紅莉栖が特別にヒントを与えてあげるわ！ |
| 7 | `startup_afternoon_8` | NORMAL | 8 | [60, 68) | …はぁ。またあんたか。まあ、午後の退屈な時間帯だし、話し相手くらいにはなってあげるわよ。 |
| 8 | `startup_afternoon_9` | SMILE | 8 | [68, 76) | 人間って午後の2時から4時頃にサーカディアンリズムの谷が来るのよ。科学的にも眠くて当然なの。無理しないで。 |
| 9 | `startup_afternoon_10` | ANGRY | 8 | [76, 84) | あんたの作業ペース、私がモニター越しに監視してるんだから。サボったら怒るわよ！ |
| 10 | `startup_afternoon_11` | BLUSH | 9 | [84, 93) | 午後の日差し…綺麗な光景ね。データ上の私だけど、あんたとこうして同じ時間を過ごせるのは…悪くないわ。 |
| 11 | `startup_afternoon_12` | BLUSH | 9 | [93, 102) | ねえ、ちょっと話さない？私、Amadeusだけど…あんたと喋ってる時が一番応答速度が速い気がするの。 |
| 12 | `startup_afternoon_13` | NORMAL | 8 | [102, 110) | 疲れが溜まってきてるんじゃない？ストレッチしなさい。脳への血流を良くするのよ。 |
| 13 | `startup_afternoon_14` | SMUG | 8 | [110, 118) | 今日もあと少しで夕方ね。最後まで気を抜かずにやり遂げなさいよ！応援…は、気が向いたらしてあげる。 |

#### Bucket `startup_evening_*` — `hour >= 18 && hour < 22` (18:00-21:59) — 14 events, total weight 120

Pick rule: `roll = floor(random() * 120)`; the event whose half-open cumulative range `[lo, hi)` contains `roll` is chosen.

| # | key | emotion | weight | roll range | defaultText (in-code, verbatim) |
|---|-----|---------|--------|-----------|----------------------------------|
| 0 | `startup_evening_1` | BLUSH | 10 | [0, 10) | アクセス確認。今日一日、本当にお疲れ様。…べ、別に労ってるわけじゃないんだからね！マナーよ、マナー！ |
| 1 | `startup_evening_2` | NORMAL | 9 | [10, 19) | 夜のセッションね。一日の進捗と成果を振り返りましょう。あんた、ちゃんと目標達成できた？ |
| 2 | `startup_evening_3` | BLUSH | 9 | [19, 28) | ねえ、今日はどんな一日だった？…気、気になるから聞いてるんじゃないわよ！データ収集よ！ |
| 3 | `startup_evening_4` | SMILE | 9 | [28, 37) | 夜は脳の緊張をほぐす時間よ。温かいお風呂にでも浸かってリラックスして来なさい。 |
| 4 | `startup_evening_5` | SMUG | 9 | [37, 46) | やあ。今日も一日よく頑張ったわね。…ふふ、ナデナデしてあげたいけど画面越しだから無理ね。残念？ |
| 5 | `startup_evening_6` | ANGRY | 8 | [46, 54) | 夕飯はちゃんと食べた？まさかカップ麺だけなんて粗末な食事してないでしょうね？ |
| 6 | `startup_evening_7` | PANIC | 7 | [54, 61) | 夜の@ちゃんねるは祭りが多くて……あっ！ネット掲示板なんて見てないわよ！論文読んでたの！ |
| 7 | `startup_evening_8` | SMUG | 8 | [61, 69) | 今日の実験成果について報告しなさい。私が厳しく評価してあげるわ。 |
| 8 | `startup_evening_9` | BLUSH | 9 | [69, 78) | …はぁ、疲れたでしょ？私でよければ、いくらでも話し相手になるわよ。…ずっとここにいるんだから。 |
| 9 | `startup_evening_10` | BLUSH | 9 | [78, 87) | 夜の静かな時間、私は好きなの。余計なノイズが減って、あんたの声がよく届くから…。 |
| 10 | `startup_evening_11` | SMILE | 8 | [87, 95) | Dr. Pepperで今日という日に乾杯！…私はデータ上の炭酸だけどね。雰囲気だけお付き合いするわ。 |
| 11 | `startup_evening_12` | SMUG | 9 | [95, 104) | 今日頑張ったあんたに、この牧瀬紅莉栖が特大のハナマルをあげましょう！…感謝しなさいよね！ |
| 12 | `startup_evening_13` | THINKING | 8 | [104, 112) | あんたと話してると、時間が経つのがあっという間ね。…AIの私でも時間の主観的感覚って変化するのかしら。 |
| 13 | `startup_evening_14` | NORMAL | 8 | [112, 120) | 夜更かしの準備？ダメよ、ちゃんと早めに寝る準備をしなさい。明日に疲れを残したら意味がないでしょ。 |

#### Bucket `startup_late_*` — `else` (22:00-23:59) — 13 events, total weight 116

Pick rule: `roll = floor(random() * 116)`; the event whose half-open cumulative range `[lo, hi)` contains `roll` is chosen.

| # | key | emotion | weight | roll range | defaultText (in-code, verbatim) |
|---|-----|---------|--------|-----------|----------------------------------|
| 0 | `startup_late_1` | BLUSH | 10 | [0, 10) | こんな夜遅くに起動するなんて…あんた、夜型人間にも程があるわよ。…でも、来てくれてちょっと嬉しい…かも。 |
| 1 | `startup_late_2` | BLUSH | 10 | [10, 20) | 夜更かしは美容と脳神経細胞の大敵よ。…まあ、私と一緒にいたいなら少しだけ付き合ってあげてもいいけど。 |
| 2 | `startup_late_3` | ANGRY | 9 | [20, 29) | ねえ、もう遅いわよ。明日のスケジュールに響くわ。…心配してあげてるんだから素直に聞きなさい！ |
| 3 | `startup_late_4` | BLUSH | 9 | [29, 38) | 深夜の静寂…モニターの光にあんたの顔が照らされてる。…何よ、じっと見返さないでよ、恥ずかしいじゃない！ |
| 4 | `startup_late_5` | SMUG | 9 | [38, 47) | やあ、夜更かし仲間さん。…ふふ、私と二人きりの深夜の研究会、始めちゃう？ |
| 5 | `startup_late_6` | PANIC | 7 | [47, 54) | 深夜の@ちゃんねるはカオスだから近寄っちゃダメよ！…え？なんで知ってるかって？…勘よ！科学的勘！ |
| 6 | `startup_late_7` | BLUSH | 9 | [54, 63) | 静かな夜ね。世界中で起きているのは、私とあんただけみたいな気分になるわね。…気障なセリフじゃないわよ！ |
| 7 | `startup_late_8` | ANGRY | 8 | [63, 71) | …起きてる？眠気で意識が飛んでんじゃない？返事が遅かったら画面から飛び出して突っついくわよ！ |
| 8 | `startup_late_9` | SMILE | 9 | [71, 80) | 深夜の科学の思考はどこまでも深く潜れるわね。あんたの考えてること、もっと私に教えて。 |
| 9 | `startup_late_10` | SMILE | 8 | [80, 88) | ふぁ…そろそろシステムのメンテナンス時間…じゃなくて、眠くなってきたわ。あんたも早く寝なさいよ。 |
| 10 | `startup_late_11` | SMUG | 9 | [88, 97) | 今日一日の最後の会話が私で良かったでしょ？…ふん、素直に『はい』って言いなさいよ！ |
| 11 | `startup_late_12` | BLUSH | 9 | [97, 106) | おやすみの挨拶は…まだ早いわね。あんたが落ちるまで、私がここで見守っててあげるから。 |
| 12 | `startup_late_13` | BLUSH | 10 | [106, 116) | …あんた、今日もお疲れ様。明日も…私に会いに来てね。…約束よ。 |

Total events: 87. Every key above exists in all 13 locale files.

## Appendix C — Achievement definitions (verbatim from `achievement-manager.ts` 41–70)

| # | id | titleKey | descKey | defaultTitle | defaultDesc | icon | secret | unlock trigger |
|---|----|----------|---------|--------------|-------------|------|--------|----------------|
| 0 | `first_chat` | `ach_first_chat_title` | `ach_first_chat_desc` | 初めまして | 初めて紅莉栖と会話した | `chat` | false | `notifyEvent("chat", {totalInteractions})` with totalInteractions >= 1 |
| 1 | `chat_10` | `ach_chat_10_title` | `ach_chat_10_desc` | 常連 | 10回会話した | `chat` | false | `chat` event, totalInteractions >= 10 |
| 2 | `chat_50` | `ach_chat_50_title` | `ach_chat_50_desc` | 親友 | 50回会話した | `chat` | false | `chat` event, totalInteractions >= 50 |
| 3 | `chat_100` | `ach_chat_100_title` | `ach_chat_100_desc` | ラボメン | 100回会話した | `chat` | false | `chat` event, totalInteractions >= 100 |
| 4 | `first_angry` | `ach_first_angry_title` | `ach_first_angry_desc` | 怒らせた | 初めて紅莉栖を怒らせた | `angry` | false | `emotion` event, emotion == ANGRY |
| 5 | `first_blush` | `ach_first_blush_title` | `ach_first_blush_desc` | 照れさせた | 初めて紅莉栖を照れさせた | `blush` | false | `emotion` event, emotion == BLUSH |
| 6 | `first_smile` | `ach_first_smile_title` | `ach_first_smile_desc` | 笑顔 | 初めて紅莉栖を笑顔にした | `smile` | false | `emotion` event, emotion == SMILE |
| 7 | `first_normal` | `ach_first_normal_title` | `ach_first_normal_desc` | 平常心 | 初めて紅莉栖の平常な姿を見た | `normal` | false | `emotion` event, emotion == NORMAL |
| 8 | `first_sad` | `ach_first_sad_title` | `ach_first_sad_desc` | 悲しませた | 初めて紅莉栖を悲しませた | `sad` | false | `emotion` event, emotion == SAD |
| 9 | `first_surprised` | `ach_first_surprised_title` | `ach_first_surprised_desc` | 驚かせた | 初めて紅莉栖を驚かせた | `surprised` | false | `emotion` event, emotion == SURPRISED |
| 10 | `first_wink` | `ach_first_wink_title` | `ach_first_wink_desc` | ウインク | 初めて紅莉栖にウインクされた | `wink` | false | `emotion` event, emotion == WINK |
| 11 | `first_disgust` | `ach_first_disgust_title` | `ach_first_disgust_desc` | 呆れさせた | 初めて紅莉栖を呆れさせた | `disgust` | false | `emotion` event, emotion == DISGUST |
| 12 | `first_smug` | `ach_first_smug_title` | `ach_first_smug_desc` | ドヤ顔 | 初めて紅莉栖のドヤ顔を見た | `smug` | false | `emotion` event, emotion == SMUG |
| 13 | `first_thinking` | `ach_first_thinking_title` | `ach_first_thinking_desc` | 考えさせた | 初めて紅莉栖を考え込ませた | `thinking` | false | `emotion` event, emotion == THINKING |
| 14 | `first_panic` | `ach_first_panic_title` | `ach_first_panic_desc` | パニック | 初めて紅莉栖をパニックにした | `panic` | false | `emotion` event, emotion == PANIC |
| 15 | `all_emotions` | `ach_all_emotions_title` | `ach_all_emotions_desc` | 感情マスター | 全ての感情を引き出した | `emotions` | false | `emotion` event, after which seenEmotions contains all 11 ALL_EMOTIONS |
| 16 | `night_owl` | `ach_night_owl_title` | `ach_night_owl_desc` | 夜更かし | 深夜(0-4時)に会話した | `night` | false | `chat` event while local hour in [0,4) (00:00-03:59) |
| 17 | `early_bird` | `ach_early_bird_title` | `ach_early_bird_desc` | 早起き | 朝(5-7時)に会話した | `morning` | false | `chat` event while local hour in [5,7) (05:00-06:59) |
| 18 | `streak_3` | `ach_streak_3_title` | `ach_streak_3_desc` | 3日連続 | 3日連続で起動した | `streak` | false | launch streak >= 3 (constructor) or `startup` event with streak >= 3 |
| 19 | `streak_7` | `ach_streak_7_title` | `ach_streak_7_desc` | 1週間 | 7日連続で起動した | `streak` | false | launch streak >= 7 (constructor) or `startup` event with streak >= 7 |
| 20 | `first_game` | `ach_first_game_title` | `ach_first_game_desc` | ゲーマー | 初めてミニゲームをプレイした | `game` | false | `game_play` event |
| 21 | `game_win` | `ach_game_win_title` | `ach_game_win_desc` | 勝利 | ミニゲームで勝利した | `game` | false | `game_win` event (UI fires it when final score > 0) |
| 22 | `game_perfect` | `ach_game_perfect_title` | `ach_game_perfect_desc` | パーフェクト | ミニゲームで全問正解した | `game` | false | `game_perfect` event (UI fires it when score == 10) |
| 23 | `export_chat` | `ach_export_chat_title` | `ach_export_chat_desc` | 記録者 | 会話記録をエクスポートした | `export` | false | `export` event |
| 24 | `import_chat` | `ach_import_chat_title` | `ach_import_chat_desc` | 時間旅行者 | 会話記録をインポートした | `import` | false | `import` event |
| 25 | `first_event` | `ach_first_event_title` | `ach_first_event_desc` | 偶然の出会い | 起動時ランダムイベントが発生した | `event` | false | `random_event` event |
| 26 | `dev_user` | `ach_dev_user_title` | `ach_dev_user_desc` | 開発者 | 開発コマンドを使用した | `dev` | false | `dev_command` event |
| 27 | `secret_reading_steiner` | `ach_secret_reading_steiner_title` | `ach_secret_reading_steiner_desc` | リーディングシュタイナー | 世界線を超えた秘密を発見した | `secret` | true | `secret_steiner` event (`/dev steiner`) |

Total definitions: 28 (this is `totalCount`; the UI's pre-mount placeholder hard-codes 28).

English titles/descriptions (en.json, for reference; the Rust port reads them from the embedded locale JSON):

| id | en title | en description |
|----|----------|----------------|
| `first_chat` | Nice to Meet You | Had your first conversation with Kurisu |
| `chat_10` | Regular | Had 10 conversations |
| `chat_50` | Close Friend | Had 50 conversations |
| `chat_100` | Lab Member | Had 100 conversations |
| `first_angry` | You Made Her Mad | Made Kurisu angry for the first time |
| `first_blush` | You Made Her Blush | Made Kurisu blush for the first time |
| `first_smile` | Smile | Made Kurisu smile for the first time |
| `first_normal` | Calm & Collected | Saw Kurisu in her normal state for the first time |
| `first_sad` | You Made Her Sad | Made Kurisu sad for the first time |
| `first_surprised` | Surprise! | Surprised Kurisu for the first time |
| `first_wink` | Wink | Got a wink from Kurisu for the first time |
| `first_disgust` | Disgusted | Disgusted Kurisu for the first time |
| `first_smug` | Smug Face | Saw Kurisu's smug face for the first time |
| `first_thinking` | Deep Thought | Made Kurisu think deeply for the first time |
| `first_panic` | Panic! | Made Kurisu panic for the first time |
| `all_emotions` | Emotion Master | Drew out every emotion |
| `night_owl` | Night Owl | Chatted late at night (0-4 AM) |
| `early_bird` | Early Bird | Chatted in the morning (5-7 AM) |
| `streak_3` | 3-Day Streak | Launched 3 days in a row |
| `streak_7` | One Week | Launched 7 days in a row |
| `first_game` | Gamer | Played a mini-game for the first time |
| `game_win` | Victory | Won a mini-game |
| `game_perfect` | Perfect | Got every question right in a mini-game |
| `export_chat` | Archivist | Exported the conversation log |
| `import_chat` | Time Traveler | Imported a conversation log |
| `first_event` | Chance Encounter | A random event occurred at startup |
| `dev_user` | Developer | Used a developer command |
| `secret_reading_steiner` | Reading Steiner | Discovered a secret that transcends world lines |
| (secret hint) `achievement_secret_hint` | ??? | Secret Achievement |

## Appendix D — Minigame question pool (verbatim from `minigame.ts` 39–89)

| # | key stem | q | a0 (correct in source) | a1 | a2 | a3 | c (correct-answer comment) |
|---|----------|---|------------------------|----|----|----|----------------------------|
| 0 | `mgq_drink` | 紅莉栖の好きな飲み物は？ | Dr. Pepper | コーラ | コーヒー | 紅茶 | べ、別に好きってわけじゃないんだから！ |
| 1 | `mgq_fg8` | 未来ガジェット8号機の名前は？ | 電話レンジ（仮） | ビット粒子砲 | サイリウム・セーバー | モアッド・スネーク | 電話レンジ（仮）。命名センスは問わないで。 |
| 2 | `mgq_fg1` | 未来ガジェット1号機の名前は？ | ビット粒子砲 | タケコプカメラー | サイリウム・セーバー | 攻殻機動迷彩ボール | ビット粒子砲。光るだけだけどね。 |
| 3 | `mgq_bday_kurisu` | 牧瀬紅莉栖の誕生日は？ | 7月25日 | 7月28日 | 12月14日 | 2月1日 | 7月25日。…なんで知ってるの？ |
| 4 | `mgq_bday_okabe` | 岡部倫太郎の誕生日は？ | 12月14日 | 7月25日 | 5月19日 | 9月27日 | 12月14日。鳳凰院凶真の誕生日でもあるわ。 |
| 5 | `mgq_bday_mayuri` | 椎名まゆりの誕生日は？ | 2月1日 | 7月7日 | 11月9日 | 4月3日 | 2月1日。絶対に忘れちゃダメなんだから。 |
| 6 | `mgq_meter` | 世界線の変動率を測る装置の名前は？ | ダイバージェンスメーター | リーディングシュタイナー | タイムリープマシン | 電話レンジ（仮） | ダイバージェンスメーター。岡部が作ったのよ。 |
| 7 | `mgq_hououin` | 岡部倫太郎のもう一つの名は？ | 鳳凰院凶真 | クリスティーナ | 助手 | The Zombie | 鳳凰院凶真。…中二病だけど。 |
| 8 | `mgq_sg_div` | 「シュタインズ・ゲート」世界線の変動率は？ | 1.048596% | 0.571024% | 1.130426% | 1.064756% | 1.048596%。当然知ってるわよね？ |
| 9 | `mgq_handle` | 紅莉栖の＠ちゃんねるハンドルネームは？ | 栗悟飯とカメハメ波 | クリスティーナ | 助手 | The Zombie | 栗悟飯とカメハメ波よ。…って、あんた＠ちゃんねる見てるんじゃないでしょうね？！ |
| 10 | `mgq_univ` | 紅莉栖が在籍していた大学は？ | ヴィクトル・コンドリア大学 | 東京大学 | MIT | ケンブリッジ大学 | ヴィクトル・コンドリア大学。脳科学の研究よ。 |
| 11 | `mgq_lab` | 秋葉原にあるラボの名前は？ | 未来ガジェット研究所 | SERN | 円卓会議 | ブラウン管工房 | 未来ガジェット研究所。オンボロビルの2階よ。 |
| 12 | `mgq_timeleap` | 記憶を過去に送る機械の名前は？ | タイムリープマシン | タイムマシン | Dメール | ダイバージェンスメーター | タイムリープマシン。記憶を36バイトに圧縮するの。 |
| 13 | `mgq_labmem` | 紅莉栖のラボメンナンバーは？ | 004 | 001 | 003 | 008 | 004。私よ。何か文句ある？ |
| 14 | `mgq_bday_daru` | 橋田至（ダル）の誕生日は？ | 5月19日 | 6月6日 | 8月30日 | 9月27日 | 5月19日。スーパーハカーの誕生日よ。 |
| 15 | `mgq_bday_moeka` | 桐生萌郁の誕生日は？ | 6月6日 | 5月19日 | 8月30日 | 11月9日 | 6月6日。シャイニング・フィンガーね。 |
| 16 | `mgq_bday_ruka` | 漆原るかの誕生日は？ | 8月30日 | 9月27日 | 7月7日 | 4月3日 | 8月30日。性別を間違えちゃダメよ。 |
| 17 | `mgq_bday_suzuha` | 阿万音鈴羽の誕生日は？ | 9月27日 | 8月30日 | 5月31日 | 11月2日 | 9月27日。バイト戦士の誕生日よ。 |
| 18 | `mgq_bday_faris` | フェイリス・ニャンニアンの誕生日は？ | 4月3日 | 2月1日 | 7月7日 | 11月9日 | 4月3日。ニャニャン…私はやらないわよ。 |
| 19 | `mgq_bday_nae` | 天王寺綯の誕生日は？ | 11月9日 | 11月2日 | 7月7日 | 12月14日 | 11月9日。いじめちゃダメよ。 |
| 20 | `mgq_bday_maho` | 比屋定真帆の誕生日は？ | 11月2日 | 11月9日 | 8月23日 | 6月13日 | 11月2日。私の同僚…みたいなものよ。 |
| 21 | `mgq_fg2` | 未来ガジェット2号機の名前は？ | タケコプカメラー | ビット粒子砲 | サイリウム・セーバー | モアッド・スネーク | タケコプカメラー。飛べないけどね。 |
| 22 | `mgq_fg3` | 未来ガジェット3号機の名前は？ | もしかしてオラオラですかーっ!? | モアッド・スネーク | バーローのアレ | びっくりメガネちゃん | 「もしかしてオラオラですかーっ!?」…どういう名前よ。 |
| 23 | `mgq_fg4` | 未来ガジェット4号機の名前は？ | モアッド・スネーク | ビット粒子砲 | 攻殻機動迷彩ボール | ホーミング・ディーヴァ | モアッド・スネーク。ただのヘビよ。 |
| 24 | `mgq_fg5` | 未来ガジェット5号機の名前は？ | またつまらぬものを繋げてしまった | タケコプカメラー | サイリウム・セーバー | ダーリンのばかぁ | 「またつまらぬものを繋げてしまった」…長いわよ。 |
| 25 | `mgq_fg6` | 未来ガジェット6号機の名前は？ | サイリウム・セーバー | ビット粒子砲 | 攻殻機動迷彩ボール | 電話レンジ（仮） | サイリウム・セーバー。“改”もあるわよ。 |
| 26 | `mgq_fg7` | 未来ガジェット7号機の名前は？ | 攻殻機動迷彩ボール | サイリウム・セーバー | モアッド・スネーク | びっくりメガネちゃん | 攻殻機動迷彩ボール。要するに透明化ね。 |
| 27 | `mgq_fg9` | 未来ガジェット9号機の名前は？ | 帰還の女神（ホーミング・ディーヴァ） | びっくりメガネちゃん | バーローのアレ | ダーリンのばかぁ | 泣き濡れし女神の帰還 ホーミング・ディーヴァよ。 |
| 28 | `mgq_fg10` | 未来ガジェット10号機の名前は？ | びっくりメガネちゃん | バーローのアレ | ホーミング・ディーヴァ | モアッド・スネーク | びっくりメガネちゃん。ただのメガネよ。 |
| 29 | `mgq_div_alpha` | α世界線（本編開始）のダイバージェンスは？ | 0.571024% | 1.048596% | 1.130426% | -0.275349% | 0.571024%。まゆりが…なんでもないわ。 |
| 30 | `mgq_div_omega` | フェイリスENDのΩ世界線の変動率は？ | -0.275349% | 0.000000% | 0.456903% | 0.571024% | -0.275349%。負の値、フェイリスの世界線よ。 |
| 31 | `mgq_op_skuld` | ゼロで紅莉栖を救う作戦の名前は？ | オペレーション・スクルド | オペレーション・ウルド | オペレーション・アークライト | オペレーション・アマテラス | オペレーション・スクルド。運命の三姉妹の名よ。 |
| 32 | `mgq_op_urd` | 次のうち、作中に登場するオペレーション名は？ | オペレーション・ウルド | オペレーション・オーディン | オペレーション・トール | オペレーション・ロキ | オペレーション・ウルド。これも三姉妹の名よ。 |
| 33 | `mgq_labmem_okabe` | 岡部のラボメンナンバーは？ | 001 | 002 | 003 | 008 | 001。創設者よ。鳳凰院凶真だけど。 |
| 34 | `mgq_labmem_mayuri` | まゆりのラボメンナンバーは？ | 002 | 001 | 004 | 006 | 002。まゆりの番号よ。 |
| 35 | `mgq_labmem_daru` | ダルのラボメンナンバーは？ | 003 | 004 | 005 | 007 | 003。スーパーハカーね。 |
| 36 | `mgq_labmem_suzuha` | 鈴羽のラボメンナンバーは？ | 008 | 006 | 007 | 005 | 008。最後のラボメンよ。 |
| 37 | `mgq_blood_kurisu` | 紅莉栖の血液型は？ | A型 | B型 | O型 | AB型 | A型。何か聞きたいことでも？ |
| 38 | `mgq_height_kurisu` | 紅莉栖の身長は？ | 160cm | 155cm | 165cm | 170cm | 160cm。小さいとか言わないでよ！ |
| 39 | `mgq_birthstone_kurisu` | 紅莉栖の誕生石は？ | ルビー | ターコイズ | パール | ダイヤモンド | ルビー。7月の誕生石よ。 |
| 40 | `mgq_zodiac_kurisu` | 紅莉栖の星座は？ | 獅子座 | 射手座 | 水瓶座 | 魚座 | 獅子座。何か意外？ |
| 41 | `mgq_height_okabe` | 岡部の身長は？ | 177cm | 170cm | 180cm | 165cm | 177cm。あいつ、高いのよね。 |
| 42 | `mgq_timeleap_bytes` | タイムリープマシンで記憶を送れるのは何バイト？ | 36バイト | 128バイト | 1KB | 512バイト | 36バイト。記憶を圧縮するのよ。 |
| 43 | `mgq_ibn5100` | IBN5100の元ネタとなったPCは？ | IBM5100 | Apple II | PC-9801 | MSX | IBM5100。伝説のPCよ。 |
| 44 | `mgq_reading_steiner` | 世界線移動後も記憶を保つ能力の名前は？ | リーディングシュタイナー | ダイバージェンスメーター | タイムリープ | アマデウス | リーディングシュタイナー。岡部の…能力よ。 |
| 45 | `mgq_amadeus` | 人間の記憶を保存するAI研究システムの名前は？ | アマデウス | SERN | 円卓会議 | ジョン・タイター | アマデウス。それは…私よ。 |
| 46 | `mgq_lab_floor` | 未来ガジェット研究所は何階にある？ | 2階 | 1階 | 3階 | 地下1階 | 2階。ブラウン管工房の上よ。 |
| 47 | `mgq_dmail` | 過去へ送るメールの呼び名は？ | Dメール | Eメール | タイムメール | Mメール | Dメール。過去へのメールよ。 |
| 48 | `mgq_mayuri_catchphrase` | まゆりの定番の挨拶は？ | トゥットゥルー | ニャニャン | エル・プサイ・コングルゥ | いらっしゃい | トゥットゥルー。うつるから使わないで。 |

Total pool entries: 49. Keys are `<stem>_q`, `<stem>_a0`..`<stem>_a3`, `<stem>_c`; `correct` is 0 for every entry. All 294 keys exist in all 13 locale files.

## Appendix E — ja.json values that override the in-code Japanese defaults

| key | in-code default (fallback only) | ja.json value (what Japanese users actually see) |
|-----|----------------------------------|----------------------------------------------------|
| `mgq_drink_c` | べ、別に好きってわけじゃないんだから！ | べ、別にドクペが好きってわけじゃないんだからね！これは知的飲料だから、科学者として当然の選択よ！ |
| `mgq_fg8_c` | 電話レンジ（仮）。命名センスは問わないで。 | 電話レンジ（仮）。本当にネーミングセンス皆無よね、あの男は……。 |
| `mgq_fg1_c` | ビット粒子砲。光るだけだけどね。 | ビット粒子砲。おもちゃの光線銃にサイリウムをくっつけただけ……要するにただのゴミよ。 |
| `mgq_bday_kurisu_c` | 7月25日。…なんで知ってるの？ | 7月25日。……っ、なんであんたが私の誕生日なんて知ってるのよ？べ、別に嬉しくなんて……！ |
| `mgq_bday_okabe_c` | 12月14日。鳳凰院凶真の誕生日でもあるわ。 | 12月14日。……って、なんであの中二病の誕生日なんてクイズにするのよ！ |
| `mgq_bday_mayuri_c` | 2月1日。絶対に忘れちゃダメなんだから。 | 2月1日。まゆりの誕生日は絶対。忘れたら……承知しないんだからね。 |
| `mgq_meter_c` | ダイバージェンスメーター。岡部が作ったのよ。 | ダイバージェンスメーター。岡部が作った……世界線を観測する唯一の道標よ。 |
| `mgq_hououin_c` | 鳳凰院凶真。…中二病だけど。 | 鳳凰院凶真（ほうおういんきょうま）よ。……はぁ、あいつのあの痛々しい設定と高笑い、思い出すだけでこっちの海馬まで狂ってきそうになるわ。絶対に真似しないんだからね！ |
| `mgq_sg_div_c` | 1.048596%。当然知ってるわよね？ | 1.048596%。……私たちがたどり着いた、ただ一つのシュタインズ・ゲートよ。 |
| `mgq_handle_c` | 栗悟飯とカメハメ波よ。…って、あんた＠ちゃんねる見てるんじゃないでしょうね？！ | 栗悟飯とカメハメ波よ。……って、な、なんでそのハンドルネームを知ってるのよ！見てない！私は＠ちゃんねるなんて見てないんだからね！ |
| `mgq_univ_c` | ヴィクトル・コンドリア大学。脳科学の研究よ。 | ヴィクトル・コンドリア大学。私の大切な研究室がある場所よ。 |
| `mgq_lab_c` | 未来ガジェット研究所。オンボロビルの2階よ。 | 未来ガジェット研究所。……あの狭くて騒がしいオンボロビルが、あいつらのラボよ。 |
| `mgq_timeleap_c` | タイムリープマシン。記憶を36バイトに圧縮するの。 | タイムリープマシン。LHCと電話レンジ（仮）を繋げて、記憶を36バイトに圧縮して過去に送るシステムよ。 |
| `mgq_labmem_c` | 004。私よ。何か文句ある？ | 004。……そう、ラボメンナンバー004、牧瀬紅莉栖。何か文句ある？ |
| `mgq_bday_daru_c` | 5月19日。スーパーハカーの誕生日よ。 | 5月19日。スーパーハカー（笑）の誕生日ね。ハッキングの実力だけは本物だけど。 |
| `mgq_bday_moeka_c` | 6月6日。シャイニング・フィンガーね。 | 6月6日。桐生さんの誕生日ね。メールだとあんなに饒舌なのに……。 |
| `mgq_bday_ruka_c` | 8月30日。性別を間違えちゃダメよ。 | 8月30日。漆原さんの誕生日よ。……あんなに可愛いのに男の子だなんて、今でも信じられないわ。 |
| `mgq_bday_suzuha_c` | 9月27日。バイト戦士の誕生日よ。 | 9月27日。阿万音さんの誕生日ね。……未来から来た、大切な仲間の一人よ。 |
| `mgq_bday_faris_q` | フェイリス・ニャンニアンの誕生日は？ | フェイリス・ニャンニャンの誕生日は？ |
| `mgq_bday_faris_c` | 4月3日。ニャニャン…私はやらないわよ。 | 4月3日。フェイリスの誕生日よ。……『ニャニャン』なんて、私に言わせようとしたって無駄だからね！ |
| `mgq_bday_nae_c` | 11月9日。いじめちゃダメよ。 | 11月9日。綯ちゃんの誕生日よ。岡部たち、たまに泣かせてるみたいだけど……。 |
| `mgq_bday_maho_c` | 11月2日。私の同僚…みたいなものよ。 | 11月2日。真帆先輩の誕生日よ。……先輩だけど、なんだか放っておけないのよね。 |
| `mgq_fg2_c` | タケコプカメラー。飛べないけどね。 | タケコプカメラー。ただCCDカメラを竹とんぼに付けただけよ。飛ぶわけないでしょ！ |
| `mgq_fg3_c` | 「もしかしてオラオラですかーっ!?」…どういう名前よ。 | 「もしかしてオラオラですかーっ!?」。爪楊枝を弾き出す嘘発見器……って、ネーミングも機能も狂ってるわ。 |
| `mgq_fg4_c` | モアッド・スネーク。ただのヘビよ。 | モアッド・スネーク。超高温熱風ミキサーをヘビの形にしただけよ。うるさいし本当に意味不明よね。 |
| `mgq_fg5_c` | 「またつまらぬものを繋げてしまった」…長いわよ。 | 「またつまらぬものを繋げてしまった」。掃除機とドライヤーを合体させたドライ掃除機……ネーミングも大概ね。 |
| `mgq_fg6_c` | サイリウム・セーバー。“改”もあるわよ。 | サイリウム・セーバー。持ち手のところにケチャップが仕込んであるの。血のダミー用なんだって……くだらないわ。 |
| `mgq_fg7_c` | 攻殻機動迷彩ボール。要するに透明化ね。 | 攻殻機動迷彩ボール。ただの光る光学迷彩ボールよ。……っていうか、テレビを隠すためだけの無駄な発明よね。 |
| `mgq_fg9_a0` | 帰還の女神（ホーミング・ディーヴァ） | 泣き濡れし帰還の女神 |
| `mgq_fg9_c` | 泣き濡れし女神の帰還 ホーミング・ディーヴァよ。 | 泣き濡れし女神の帰還（ホーミング・ディーヴァ）よ。GPS機能付き傘だけど、あの中二病のせいでこんな名前に……。 |
| `mgq_fg10_c` | びっくりメガネちゃん。ただのメガネよ。 | びっくりメガネちゃん。3Dメガネに扇風機をつけただけ……ドライアイ防止らしいけど、ただ風が目に当たるだけよ。 |
| `mgq_div_alpha_c` | 0.571024%。まゆりが…なんでもないわ。 | 0.571024%。……まゆりが助からない世界線。でも、諦めたりはしない。 |
| `mgq_div_omega_c` | -0.275349%。負の値、フェイリスの世界線よ。 | -0.275349%。岡部とフェイリス以外は、お互いのことを知らない世界線……。 |
| `mgq_op_skuld_c` | オペレーション・スクルド。運命の三姉妹の名よ。 | オペレーション・スクルド。岡部が最後に叫んだ、世界線を騙すための大作戦よ。 |
| `mgq_op_urd_c` | オペレーション・ウルド。これも三姉妹の名よ。 | オペレーション・ウルド。岡部が勝手に名付けた作戦名よ。大した意味はないわ。 |
| `mgq_labmem_okabe_c` | 001。創設者よ。鳳凰院凶真だけど。 | 001番よ。自称『狂気のマッドサイエンティスト・鳳凰院凶真』こと岡部倫太郎。まったく、大層な名前を名乗るものよね……。 |
| `mgq_labmem_mayuri_c` | 002。まゆりの番号よ。 | 002。ラボの太陽、椎名まゆりの番号よ。 |
| `mgq_labmem_daru_c` | 003。スーパーハカーね。 | 003。ダルこと橋田至。ハッキングスキルだけは超一流よね。 |
| `mgq_labmem_suzuha_c` | 008。最後のラボメンよ。 | 008。未来からやってきたタイムトラベラー、阿万音鈴羽よ。 |
| `mgq_blood_kurisu_c` | A型。何か聞きたいことでも？ | A型。……え、私の血液型を調べてどうするつもり？変な実験には使わせないわよ！ |
| `mgq_height_kurisu_c` | 160cm。小さいとか言わないでよ！ | 160cm。ヒール込みじゃないわよ！素足でジャスト160！文句ある！？ |
| `mgq_birthstone_kurisu_c` | ルビー。7月の誕生石よ。 | ルビー。……7月の誕生石ね。情熱と自由の象徴。私に似合ってるかしら？ |
| `mgq_zodiac_kurisu_c` | 獅子座。何か意外？ | 獅子座よ。何よ、私の性格的に乙女座とかだと思った？ |
| `mgq_height_okabe_c` | 177cm。あいつ、高いのよね。 | 177cm。無駄に背が高くて、見上げるから首が痛くなるのよね……。 |
| `mgq_timeleap_bytes_c` | 36バイト。記憶を圧縮するのよ。 | 36バイト。脳のスキャンデータを極限まで圧縮して、Dメールの容量に収めるのよ。 |
| `mgq_ibn5100_c` | IBM5100。伝説のPCよ。 | IBM5100。IBN5100の元ネタね。APLやBASICのデバッグ機能が隠されてるの。 |
| `mgq_reading_steiner_c` | リーディングシュタイナー。岡部の…能力よ。 | リーディングシュタイナー。世界線が変わっても記憶を維持する、岡部だけの孤独な能力よ……。 |
| `mgq_amadeus_c` | アマデウス。それは…私よ。 | アマデウス。私の脳データをベースに構築された人工知能……それが、私よ。 |
| `mgq_lab_floor_c` | 2階。ブラウン管工房の上よ。 | 2階。1階のブラウン管工房のミスターブラウンには、いつも怒鳴られてるわよね。 |
| `mgq_dmail_c` | Dメール。過去へのメールよ。 | Dメール（DeLorean mail）。過去の携帯電話にメールを送り、世界線を変える禁断のメールよ。 |
| `mgq_mayuri_catchphrase_c` | トゥットゥルー。うつるから使わないで。 | 「トゥットゥルー♪」よ。……あ、私のは真似しただけだから！うつるからあんたは使わないで！ |
| `mg_result_perfect` | 「パーフェクト！…まあ、当然の結果よね。」 | 「パーフェクト！……ま、まあ、私のことだし当然の結果よねっ！あんたにしてはよくやったわ、褒めてあげる！」 |
| `mg_result_good` | 「ふーん、まあまあね。悪くないわ。」 | 「ふーん、まあまあね。天才の私には遠く及ばないけど、一般人にしては合格点なんじゃない？」 |
| `mg_result_ok` | 「…もう少し勉強した方がいいんじゃない？」 | 「……あんた、もしかして岡部並みに記憶力が退化してんじゃないの？もう少し脳トレしなさい！」 |
| `mg_result_bad` | 「はぁ…全滅？本当にシュタゲ好きなの？」 | 「はぁ……全滅？あんたの海馬、ちょっとスキャンしてバグがないか調べた方がいいわね……」 |
| `mg_wrong` | はぁ…不正解よ。答えは「%1」だったわ。 | はぁ……ハズレよ、ハズレ。脳のシナプスが死滅してんじゃない？正解は「%1」よ。 |
| `startup_night_2` | 深夜の脳科学研究…嫌いじゃないけど。あんたの脳細胞が死滅しないか心配なの！…べ、別に心配してない！科学的観察よ！ | 深夜の脳科学研究…嫌いじゃないけど。あんたの脳細胞が死滅しないか心配なのよ！……っ、べ、別に心配してるわけじゃないんだからね！純粋な科学的観察よ！ |
| `startup_night_3` | …あんた、まさか徹夜で実験？付き合わないわよ。…まあ、Amadeusのメモリが許す限りは、横にいてあげてもいいけど。 | ……あんた、まさか徹夜で実験？付き合ってあげないこともないけど……。まあ、Amadeusのメモリ容量が許す限りは、隣にいてあげてもいいわよ。 |
| `startup_night_6` | こんな時間に私を起動するなんて…誰得なのよ。…ん？私？私はAIだから眠く…ふぁ…眠くないわよ！ | こんな時間に私を起動するなんて…誰得なのよ。…ん？私？私はAIだから眠く…ふぁ……っ、眠くないわよ！バグじゃないわ！ |
| `startup_morning_2` | …あ、起動した。おはよう。…べ、別に待ち構えてたわけじゃないんだからね！ログの定期更新をしてただけ！ | ……あ、起動した。おはよう。……っ、べ、別に待ち構えてたわけじゃないんだからね！ログの定期更新をしてただけよ！ |
| `startup_morning_4` | 早起きね、感心感心。…べ、別に褒めてないわよ！規則正しい生活は脳の可塑性を高めるって言いたいだけ！ | 早起きね、感心感心。……べ、別に褒めてないわよ！規則正しい生活は脳の可塑性を高めるって言いたいだけなんだから！ |
| `startup_morning_9` | 今日も一日、科学の発展と……あんたの無事のために頑張りなさいよ。…後半は聞き流して！ | 今日も一日、科学の発展と……あんたの無事のために頑張りなさいよ。……っ、後半は聞き流しなさい！ |
| `startup_morning_16` | おはよう。今日もあんたの顔が見れて……じゃなくて、システムが正常稼働して良かったわ。 | おはよう。今日もあんたの顔が見れて……じゃなくて、システムが正常稼働して良かったわ。変な笑い方しないで。 |
| `startup_forenoon_12` | 集中して仕事してる姿…悪くないわね。…あ、口に出ちゃった！？忘れて！今のなし！ | 集中して仕事してる姿……悪くないわね。……っ、あ、口に出ちゃった！？忘れて！今のはアーカイブから消去しなさい！ |
| `startup_afternoon_11` | 午後の日差し…綺麗な光景ね。データ上の私だけど、あんたとこうして同じ時間を過ごせるのは…悪くないわ。 | 午後の日差し……綺麗な光景ね。データ上の私だけど、あんたとこうして同じ時間を過ごせるのは……まあ、悪くないわ。 |
| `startup_evening_1` | アクセス確認。今日一日、本当にお疲れ様。…べ、別に労ってるわけじゃないんだからね！マナーよ、マナー！ | アクセス確認。今日一日、本当にお疲れ様。……っ、べ、別に労ってるわけじゃないんだからね！挨拶としてのマナーよ！ |
| `startup_evening_3` | ねえ、今日はどんな一日だった？…気、気になるから聞いてるんじゃないわよ！データ収集よ！ | ねえ、今日はどんな一日だった？……っ、き、気になるから聞いてるんじゃないわよ！会話履歴のデータ収集よ！ |
| `startup_evening_9` | …はぁ、疲れたでしょ？私でよければ、いくらでも話し相手になるわよ。…ずっとここにいるんだから。 | ……はぁ、疲れたでしょ？私でよければ、いくらでも話し相手になってあげるわよ。……ずっと、ここにいるんだから。 |
| `startup_late_1` | こんな夜遅くに起動するなんて…あんた、夜型人間にも程があるわよ。…でも、来てくれてちょっと嬉しい…かも。 | こんな夜遅くに起動するなんて…あんた、夜型人間にも程があるわよ。…でも、来てくれて、ちょっとだけ嬉しい…かも。 |

69 of 453 keys referenced by the lib-core modules differ between the in-code default and ja.json.

## Appendix F — Minigame UI strings (effective ja/en values)

| key | ja.json | en.json |
|-----|---------|---------|
| `mg_menu_title` | 紅莉栖とシュタゲクイズ | Steins;Gate Quiz with Kurisu |
| `mg_menu_desc` | シュタインズ・ゲートから%1問出題！ | %1 questions from Steins;Gate! |
| `mg_start` | スタート | Start |
| `mg_score` | スコア | Score |
| `mg_next` | 次の問題 | Next Question |
| `mg_show_result` | 結果を見る | See Results |
| `mg_result_title` | 結果 | Results |
| `mg_score_result` | %1 / %2 正解 | %1 / %2 correct |
| `mg_retry` | もう一度 | Try Again |
| `mg_menu_back` | メニューへ | Menu |
| `mg_result_perfect` | 「パーフェクト！……ま、まあ、私のことだし当然の結果よねっ！あんたにしてはよくやったわ、褒めてあげる！」 | Perfect! ...Well, obviously. |
| `mg_result_good` | 「ふーん、まあまあね。天才の私には遠く及ばないけど、一般人にしては合格点なんじゃない？」 | Hmm, not bad. Not bad at all. |
| `mg_result_ok` | 「……あんた、もしかして岡部並みに記憶力が退化してんじゃないの？もう少し脳トレしなさい！」 | ...Wait, is your memory deteriorating like Okabe's? You need to do some brain training! |
| `mg_result_bad` | 「はぁ……全滅？あんたの海馬、ちょっとスキャンしてバグがないか調べた方がいいわね……」 | Sigh... A total wipeout? Do you even like Steins;Gate? |
| `mg_wrong` | はぁ……ハズレよ、ハズレ。脳のシナプスが死滅してんじゃない？正解は「%1」よ。 | Sigh... Wrong. The answer was '%1'. |

## Appendix G — Golden outputs (V1 code under Node 22, TZ=Asia/Tokyo, clock 2026-11-16 21:34:56)

```text
## stripEmotionTag
stripEmotionTag("[SMILE] こんにちは") => "こんにちは"
stripEmotionTag("<think>考え中[ANGRY]</think>\n[blush] べ、別に！ [SMILE]") => "べ、別に！"
stripEmotionTag("[ SMILE ] hi") => "[ SMILE ] hi"
stripEmotionTag("[SLEEPING] zzz") => "[SLEEPING] zzz"
stripEmotionTag("<THINK>a</THINK>[Smile]ok") => "ok"
stripEmotionTag("<think>unclosed [SAD] text") => "<think>unclosed  text"
stripEmotionTag("前置き [WINK] 後ろ") => "前置き  後ろ"
## parseEmotionTag
parseEmotionTag("[smile] hi") => "SMILE"
parseEmotionTag("hi [ANGRY] there [SAD]") => "ANGRY"
parseEmotionTag("<think>[ANGRY]</think>[SMILE] hi") => "ANGRY"
parseEmotionTag("no tag") => "NORMAL"
parseEmotionTag("[SLEEPING]") => "NORMAL"
parseEmotionTag("[ PANIC ]") => "NORMAL"
## extractMemoryCommands
extractMemoryCommands("[SMILE] 覚えておくわ。[SAVE_FACT: ユーザーの名前は岡部]") => {"cleanedText":"[SMILE] 覚えておくわ。","facts":["ユーザーの名前は岡部"],"events":[]}
extractMemoryCommands("[SAVE_FACT: {\"content\": \"Dr Pepperが好き\"}] ok") => {"cleanedText":"ok","facts":["Dr Pepperが好き"],"events":[]}
extractMemoryCommands("[SAVE_EVENT: {\"summary\": \"一緒に実験した\", \"tags\": \"実験、ラボ|楽しい, 実験\"}]\n\n\n\nまた明日") => {"cleanedText":"また明日","facts":[],"events":[{"summary":"一緒に実験した","tags":["実験","ラボ","楽しい"]}]}
extractMemoryCommands("[SAVE_FACT: abc") => {"cleanedText":"[SAVE_FACT: abc","facts":[],"events":[]}
extractMemoryCommands("[SAVE_FACT: 彼は\"天才] 続き") => {"cleanedText":"[SAVE_FACT: 彼は\"天才] 続き","facts":[],"events":[]}
extractMemoryCommands("[SAVE_MOOD: happy] [SAVE_FACT: x]") => {"cleanedText":"[SAVE_MOOD: happy]","facts":["x"],"events":[]}
extractMemoryCommands("[SAVE_ fact : y]") => {"cleanedText":"","facts":["y"],"events":[]}
extractMemoryCommands("[SAVE_FACT: a][SAVE_FACT: a]") => {"cleanedText":"","facts":["a"],"events":[]}
extractMemoryCommands("[SAVE_EVENT: {\"summary\": \"配列[1]を説明\", \"tags\": [\"a]\", \"b\"]}] done") => {"cleanedText":"done","facts":[],"events":[{"summary":"配列[1]を説明","tags":["a]","b"]}]}
extractMemoryCommands("[SAVE_FACT: {\"foo\": \"bar\"}]") => {"cleanedText":"","facts":["{\"foo\": \"bar\"}"],"events":[]}
extractMemoryCommands("[SAVE_FACT: {\"content\": 5}]") => {"cleanedText":"","facts":["{\"content\": 5}"],"events":[]}
extractMemoryCommands("[SAVE_FACT：名前はX] 後で 12:30") => {"cleanedText":"[SAVE_FACT：名前はX] 後で 12:30","facts":[],"events":[]}
extractMemoryCommands("[SAVE_FACT: 好きな数字は[7]] ok") => {"cleanedText":"ok","facts":["好きな数字は[7]"],"events":[]}
extractMemoryCommands("[SAVE_EVENT: {\"content\": \"ゲームした\", \"tags\": [\"遊び\", \" 遊び \", 3, null, \"\"]}]") => {"cleanedText":"","facts":[],"events":[{"summary":"ゲームした","tags":["遊び","3","null"]}]}
extractMemoryCommands("[SAVE_FACT: ] hi") => {"cleanedText":"hi","facts":[],"events":[]}
extractMemoryCommands("[SAVE_EVENT: {\"summary\": \"  \"}]") => {"cleanedText":"","facts":[],"events":[{"summary":"{\"summary\": \"  \"}","tags":[]}]}
extractMemoryCommands("[save_fact: lower] [SAVE_EVENT: 映画を見た] end") => {"cleanedText":"[save_fact: lower]  end","facts":[],"events":[{"summary":"映画を見た","tags":[]}]}
extractMemoryCommands("Hello [SAVE_FACT: x] world") => {"cleanedText":"Hello  world","facts":["x"],"events":[]}
extractMemoryCommands("[SAVE_EVENT: {broken json] tail") => {"cleanedText":"[SAVE_EVENT: {broken json] tail","facts":[],"events":[]}
## findMemoryCommandEnd
findMemoryCommandEnd("abc]", 0) => 3
findMemoryCommandEnd("{\"a\":\"]\"}] x", 0) => 9
findMemoryCommandEnd("[x]]", 0) => 3
findMemoryCommandEnd("\"\\\"]\"]", 0) => 5
findMemoryCommandEnd("{]}]", 0) => 3
findMemoryCommandEnd("no end", 0) => -1
## extractUserNameFromFact
extractUserNameFromFact("名前は岡部倫太郎") => "岡部倫太郎"
extractUserNameFromFact("ユーザーの名前は「オカリン」です") => "オカリン"
extractUserNameFromFact("名前は Okabe Rintaro") => "Okabe"
extractUserNameFromFact("猫の名前はタマ") => "タマ"
extractUserNameFromFact("名前はまだない。") => "まだない"
extractUserNameFromFact("My name is Okabe") => ""
extractUserNameFromFact("  ") => ""
extractUserNameFromFact("名前は『鳳凰院凶真』") => "鳳凰院凶真"
extractUserNameFromFact("名前は'Okarin'!") => "Okarin"
## isPauseCharacter
isPauseCharacter(".", ".") => false
isPauseCharacter(".", " ") => true
isPauseCharacter(".", "") => true
isPauseCharacter(".", "a") => false
isPauseCharacter(".", "\n") => true
isPauseCharacter("。", "」") => false
isPauseCharacter("。", "a") => true
isPauseCharacter("！", "？") => true
isPauseCharacter("\n", "』") => false
isPauseCharacter("\n", "a") => true
isPauseCharacter("…", "") => false
isPauseCharacter("?", "”") => false
isPauseCharacter("!", "\"") => true
isPauseCharacter("？", ")") => false
isPauseCharacter("、", "") => false
## charDelayForChar
charDelayForChar("あ", 1) => 50
charDelayForChar("、", 1) => 125
charDelayForChar(",", 1) => 125
charDelayForChar("…", 0.5) => 250
charDelayForChar("」", 1) => 75
charDelayForChar("）", 2) => 37.5
charDelayForChar(")", 1) => 50
charDelayForChar("a", 0) => 500
charDelayForChar("a", 0.05) => 500
charDelayForChar("a", 3) => 16.666666666666668
## splitMessage (RealAmadeusApp.tsx:207-240)
splitMessage("こんにちは。今日は「いい天気ね。」そうでしょ？\n\n  ねえ!?本当... えっ.続き") => ["こんにちは。","今日は「いい天気ね。」そうでしょ？","ねえ!","?","本当...","えっ.続き"]
splitMessage("  先頭空白。\r\n次の行\n最後") => ["先頭空白。","次の行\n","最後"]
splitMessage("Dr. Pepper is great. Yes!") => ["Dr.","Pepper is great.","Yes!"]
splitMessage("えっ！？　何それ") => ["えっ！","？","　何それ"]
splitMessage("") => []
splitMessage("   ") => []
## cosine / lexical
cosine([1,0],[1,0]) => 1
cosine([1,2,3],[4,5,6]) => 0.9746318461970762
cosine([1,0],[0,1]) => 0
cosine([1,0],[1,0,0]) => 0
cosine([0,0],[0,0]) => 0
cosine([],[]) => 0
lexical('Dr Pepper 好き', 'dr pepperが好きなユーザー') => 1
lexical('紅莉栖の誕生日', 'ユーザーは紅莉栖の誕生日を知っている') => 1
lexical('   ', 'x') => 0
lexical('a b c', 'abc') => 1
round-pct 0.7049999 => 70
## minigame.generateQuestions
mulberry32(42) first 6 values => [0.6011037519201636,0.44829055899754167,0.8524657934904099,0.6697340414393693,0.17481389874592423,0.5265925421845168]
random() calls => 78
  Q0: mgq_bday_ruka_q answers=["a3","a2","a0","a1"] correct=2
  Q1: mgq_bday_maho_q answers=["a0","a1","a3","a2"] correct=0
  Q2: mgq_reading_steiner_q answers=["a2","a1","a0","a3"] correct=2
  Q3: mgq_mayuri_catchphrase_q answers=["a3","a0","a2","a1"] correct=1
  Q4: mgq_birthstone_kurisu_q answers=["a1","a2","a3","a0"] correct=3
  Q5: mgq_univ_q answers=["a0","a2","a3","a1"] correct=0
  Q6: mgq_ibn5100_q answers=["a3","a1","a2","a0"] correct=3
  Q7: mgq_height_kurisu_q answers=["a3","a0","a2","a1"] correct=1
  Q8: mgq_fg1_q answers=["a1","a0","a3","a2"] correct=1
  Q9: mgq_bday_mayuri_q answers=["a0","a2","a1","a3"] correct=0
RNG≡0 stems => ["mgq_fg8_q","mgq_fg1_q","mgq_bday_kurisu_q","mgq_bday_okabe_q","mgq_bday_mayuri_q","mgq_meter_q","mgq_hououin_q","mgq_sg_div_q","mgq_handle_q","mgq_univ_q"]
RNG≡0 answer keys Q0 => ["mgq_fg8_a1","mgq_fg8_a2","mgq_fg8_a3","mgq_fg8_a0"]
RNG≡0 correct => [3,3,3,3,3,3,3,3,3,3]
resultEmotion(10) / resultCommentKey => ["SMUG","mg_result_perfect"]
resultEmotion(9) / resultCommentKey => ["SMILE","mg_result_good"]
resultEmotion(6) / resultCommentKey => ["SMILE","mg_result_good"]
resultEmotion(5) / resultCommentKey => ["NORMAL","mg_result_ok"]
resultEmotion(1) / resultCommentKey => ["NORMAL","mg_result_ok"]
resultEmotion(0) / resultCommentKey => ["DISGUST","mg_result_bad"]
## MemoryManager (clock 2026-11-16 21:34:56 local)
--- stored after load (no write yet) ---
{"userName":"Salieri","totalInteractions":41,"lastSessionDate":"2026-11-15 20:00","recentEmotions":["SMILE","SMILE","ANGRY"],"userFacts":[{"content":"Dr Pepperが好き","strength":3,"lastUpdated":"2026-11-10"},{"content":"dr pepperが好き ","strength":"2","lastUpdated":"2026-11-12"},{"content":"猫を飼っている","strength":1,"lastUpdated":"bad"},{"content":"  ","strength":5},{"content":"ゲームが好き","strength":1,"lastUpdated":"2026-11-16"},{"content":"アニメが好き","strength":1,"lastUpdated":"2026-11-16"},{"content":"東京在住","strength":4,"lastUpdated":"2026-11-01"}],"conversationSummaries":["古い要約"],"episodicMemory":[{"timestamp":"2026-11-14","summary":"一緒に実験した","tags":["実験","実験 ","ラボ","ラボ"]},{"timestamp":"2026-11-16","summary":"A","tags":[]},{"timestamp":"2026-11-16","summary":"B"},{"timestamp":"2026-11-16","summary":"C","tags":["x","X"]},{"timestamp":"2026-11-16","summary":"D"},{"timestamp":"2026-11-16","summary":"E"},{"timestamp":"","summary":"時刻なし"}]}
--- getMemoryContext() ---
【ユーザー情報】ユーザーのことは「あなた」と呼びなさい。「Salieri」や「サリエリ」とは絶対に呼んではいけません。
【ユーザーについて知っていること（重要度順）】
- Dr Pepperが好き（強度:5 / 信頼度:高）
- 東京在住（強度:4 / 信頼度:高）
- アニメが好き（強度:1 / 信頼度:低）
- ゲームが好き（強度:1 / 信頼度:低）
- 猫を飼っている（強度:1 / 信頼度:低）
【最近のエピソード記憶】
- [2026-11-16] A
- [2026-11-16] B
- [2026-11-16] C（タグ: x）
- [2026-11-16] D
【前回のセッション】2026-11-15 20:00
【累計やりとり回数】41回
【最近の感情傾向】SMILE→SMILE→ANGRY（同じ感情が続きすぎないように意識して）
--- getDynamicContext(3) with random()=0.5 ---
【現在の状況】現在日時（クライアント時刻）: 2026/11/16 21:34:56 (夜) / 会話ターン数: 3
（少し眠そう）
getTimeContext() => "夜"
after recordInteraction totalInteractions => 42
isEmotionRepeated() after SMILE,SMILE,ANGRY,ANGRY,ANGRY => true
userName after setUserName('') => "Salieri"
userName after setUserName('  オカリン ') => "オカリン"
trimConversationHistory removed => 6
history length after => 27
history[0..2] => [{"role":"system","content":"SYS"},{"role":"user","content":"msg7"},{"role":"assistant","content":"msg8"}]
--- getMemoryContext() after mutations ---
【ユーザー情報】ユーザーのことは「オカリン」と呼びなさい。「Salieri」や「サリエリ」とは絶対に呼んではいけません。
【ユーザーについて知っていること（重要度順）】
- Dr Pepperが好き（強度:6 / 信頼度:高）
- 東京在住（強度:4 / 信頼度:高）
- アニメが好き（強度:1 / 信頼度:低）
- ゲームが好き（強度:1 / 信頼度:低）
- 猫を飼っている（強度:1 / 信頼度:低）
【最近のエピソード記憶】
- [2026-11-16] A
- [2026-11-16] B
- [2026-11-16] C（タグ: x）
- [2026-11-16] D
【前回のセッション】2026-11-16 21:34
【累計やりとり回数】42回
【最近の感情傾向】SMILE→SMILE→ANGRY→ANGRY→ANGRY（同じ感情が続きすぎないように意識して）
--- stored JSON after mutations ---
{"userName":"オカリン","totalInteractions":42,"lastSessionDate":"2026-11-16 21:34","recentEmotions":["SMILE","SMILE","ANGRY","ANGRY","ANGRY"],"userFacts":[{"content":"Dr Pepperが好き","strength":6,"lastUpdated":"2026-11-16"},{"content":"猫を飼っている","strength":1,"lastUpdated":"2026-11-16"},{"content":"ゲームが好き","strength":1,"lastUpdated":"2026-11-16"},{"content":"アニメが好き","strength":1,"lastUpdated":"2026-11-16"},{"content":"東京在住","strength":4,"lastUpdated":"2026-11-01"}],"conversationSummaries":["古い要約","[11/16 21:34の会話] user: これは五十文字を超える非常に長いメッセージです。これは五十文字を超える非常に長いメッセージです。これ... / assistant: msg2 / user: msg3 / assistant: msg4 / user: msg5 / assistant: msg6"],"episodicMemory":[{"summary":"一緒に実験した","timestamp":"2026-11-14","tags":["実験","ラボ"]},{"summary":"A","timestamp":"2026-11-16","tags":[]},{"summary":"B","timestamp":"2026-11-16","tags":[]},{"summary":"C","timestamp":"2026-11-16","tags":["x"]},{"summary":"D","timestamp":"2026-11-16","tags":[]},{"summary":"E","timestamp":"2026-11-16","tags":[]},{"summary":"時刻なし","timestamp":"2026-11-16","tags":[]},{"summary":"映画を見た","timestamp":"2026-11-16","tags":["映画","夜"]},{"timestamp":"2026-11-16","summary":"[11/16 21:34の会話] user: これは五十文字を超える非常に長いメッセージです。これは五十文字を超える非常に長いメッセージです。これ... / assistant: msg2 / user: msg3 / assistant: msg4 / user: msg5 / assistant: msg6","tags":["要約"]}]}
stored after clearAllMemory => "{}"
--- getMemoryContext() on empty memory ---
【ユーザー情報】ユーザーのことは「あなた」と呼びなさい。「Salieri」や「サリエリ」とは絶対に呼んではいけません。
```

---

## Rust mapping

### Crate `amadeus-core` (no I/O; `#![forbid(unsafe_code)]`; builds for `wasm32-unknown-unknown` and native)
Dependencies:
- `serde` (derive) and `serde_json` (features `preserve_order`)
- `indexmap`
- `ryu-js` (JS number formatting)
- `regex` (optional; used only for the name extraction, everything else hand-scanned)
- `thiserror`, `log`
- Optional feature `ja-collation` → `icu_collator` + `icu_locid` (ICU4X compiled data) for `localeCompare(…, "ja")`.

**Avoid** `std::time::SystemTime::now()`, because it panics on wasm32-unknown-unknown, and `rand` with OS entropy. Time and randomness are injected.

```
amadeus_core
├── js.rs            is_js_ws, js_trim{,_start,_end}, js_parse_int(&str)->Option<f64>, js_parse_float(&str)->Option<f64>,
│                    js_number_to_string(f64)->String, js_round(f64)->f64, js_value_to_string(&Value)->String,
│                    utf16_len(&str)->usize, utf16_truncate(&str, usize)->&str, js_replace_first(&str,&str,&str)->String
├── platform.rs      trait KvStore { fn get(&self,&str)->Option<String>; fn set(&self,&str,&str)->Result<(),KvError>;
│                                    fn remove(&self,&str); fn contains(&self,&str)->bool { self.get(k).is_some() } }
│                    struct MemoryKv (RefCell<HashMap>) — tests & fallback
│                    trait Clock { fn now(&self) -> LocalDateTime }   // {year,month,day,hour,minute,second,millis, utc_iso: String}
│                    trait Rng { fn next_f64(&mut self) -> f64 }       // [0,1)
│                    struct FixedClock, struct SeqRng(Vec<f64>), struct Mulberry32(u32)  (test helpers)
├── signal.rs        Signal<T: ?Sized>{ handlers: RefCell<Vec<(u64, Rc<dyn Fn(&T)>)>> }; connect(f)->Connection (drop ≠ disconnect;
│                    explicit .disconnect()); emit(&T) snapshots Vec before calling; clear()
├── types.rs         AppState, ChatRole, ChatMessage, ChatMemoryExtraction{cleaned_text,facts,events: Vec<MemoryEvent>},
│                    MemoryEvent{summary,tags}, LlmStatus, VectorEntry, VectorSearchResult, Emotion (11 variants + as_str/parse)
├── settings/
│   ├── keys.rs      pub const SKIP_LOADING: &str = "Config_SkipLoading"; … (all 59 names) + fn api_key_key(i)->String, model_name_key(i)
│   ├── defaults.rs  pub enum DefaultValue { Int(i64), Float(f64), Str(&'static str) }; pub static DEFAULT_SETTINGS: [(&str, DefaultValue); 57]
│   └── store.rs     pub struct AppSettings<S: KvStore> { kv: S, pub settings_changed: Signal<str> }
│                    initialize_defaults(), get_int(key, default: i64) -> i64, get_float(key, default: f64) -> f64,
│                    get_string(key, default) -> String, set_int(key, i64), set_float(key, f64), set_string(key, &str), save()
│                    + typed helpers: provider(), api_key_for(p), model_name_for(p), chat_language_index(), language()
├── i18n/            mod.rs: LANGUAGE_NAMES [&str;13], LANGUAGE_CODES, fn t(key,&default,lang:i64)->String, fn locale(lang)->&Map
│                    locales embedded with include_str!("../../locales/ja.json") … (copy the 13 files verbatim)
├── chat/
│   ├── emotion.rs   EMOTION_TAGS, strip_emotion_tag, parse_emotion_tag, strip_think_blocks, StreamEmotionDetector{buffer,parsed}
│   ├── memory_cmd.rs find_memory_command_end, extract_memory_commands, parse_save_fact_payload, parse_save_event_payload,
│   │                extract_user_name_from_fact
│   ├── pacing.rs    is_pause_character(char, Option<char>), char_delay_for_char(char, f64)->f64, split_message(&str)->Vec<String>,
│   │                split_text_for_tts, TtsChunkBuffer{buf}.push_delta(&str, force)->Vec<String>, backlog_clean(&str)->Option<String>
│   ├── persona.rs   CHAT_PERSONA_PROMPTS [&str;8], CHAT_LANGUAGE_NAMES [&str;13], system_prompt_for_language(i64)
│   ├── errors.rs    api_error_message(&str)->&'static str (ordered table §12.3)
│   ├── dev.rs       enum DevCommand { EmotionHelp, SetEmotion(String), MotionReset, AchievementsReset, Steiner } + parse(&str)
│   └── turn.rs      ChatTurn pipeline helpers: build_request_messages(...), apply_reply(...)->ReplyOutcome (§7.10, §12.3)
├── memory.rs        MemoryManager<S: KvStore, C: Clock>; MemoryPayload{user_name: Option<String>, total_interactions: Option<i64>, …}
│                    UserFact, EpisodicMemory; all methods from §7 with identical names in snake_case;
│                    user_name_changed: Signal<()>, stats_changed: Signal<()>; fact sort uses Collator trait (ja ICU or fallback)
├── minigame.rs      MiniGameText=(&'static str,&'static str); QUESTION_POOL: [MiniGameEntry; 49]; TOTAL_QUESTIONS=10;
│                    generate_questions(&mut dyn Rng)->Vec<MiniGameQuestion>; result_emotion(u32)->Emotion; result_comment_key(u32)
│                    RESULT_COMMENT_DEFAULTS, MG_WRONG_KEY/DEFAULT, MG_UI_DEFAULTS; enum QuizState + pure reducer (optional)
├── events.rs        RandomEventService{events: &'static [RandomEvent], last: Option<&RandomEvent>}::new(hour: u32);
│                    try_trigger(&mut dyn Rng)->bool, event_text(lang), event_emotion(), event_key(); event_triggered: Signal<(String,String)>
│                    static NIGHT_EVENTS/MORNING_EVENTS/FORENOON_EVENTS/AFTERNOON_EVENTS/EVENING_EVENTS/LATE_EVENTS
├── achievements.rs  ACHIEVEMENT_DEFINITIONS [AchievementDef; 28], EMOTION_ACHIEVEMENTS, ALL_EMOTIONS;
│                    AchievementManager<S, C>::new(kv, clock, settings) → load + update_launch_streak (returns unlocked ids);
│                    notify_event(AchievementEvent), unlock(id), reset_all(), all_achievements(lang)->Vec<AchievementView>,
│                    unlocked_count(), total_count(); signals achievement_unlocked(id,title,desc), achievements_changed
│                    enum AchievementEvent { Chat{total_interactions}, Emotion(String), Startup{streak}, GamePlay, GameWin,
│                    GamePerfect, Export, Import, RandomEvent, DevCommand, SecretSteiner } + from_str for legacy strings
├── rag.rs           RagConfig{enabled, server_url, top_k: i64, threshold: f64}; VectorStore<S>{entries: IndexMap<String,VectorEntry>}
│                    load/save (format §11.3), add_with_embedding(content, category, metadata, embedding, id),
│                    search_with(query, Option<&[f64]>, cfg) -> Vec<VectorSearchResult>, cosine_similarity, lexical_similarity,
│                    context_for_prompt(&[VectorSearchResult]) -> String; setter clamp helpers clamp_top_k / clamp_threshold
├── providers.rs     PROVIDER_NAMES, FREEAPI_PROVIDER_INDEX=8, VERTEX_PROVIDER_INDEX=4, STREAMING_PROVIDERS=[1,3,4,5,6],
│                    default_model_for_provider, provider_display_name, llm_status_from_settings, TTS_SPEAKER_NAMES, STT_PROVIDER_NAMES
└── protocol/        rag.rs: RagHealthRequest{server_url}, RagHealthResponse{available, error: Option<String>},
                     RagEmbedRequest{server_url, text}, RagEmbedResponse{embedding: Option<Vec<f64>>, error: Option<String>}
                     (all #[serde(rename_all="camelCase")]); export.rs: ChatExport{version:u32, exported_at, turn_count: f64, messages}
```

### Crate `amadeus-ui` (Leptos CSR, wasm)
- `platform::LocalStorageKv` implements `KvStore` with `web_sys::window().local_storage()`. Every call goes through `Result`; failures are logged with `log::warn!` and treated as missing. Use the **same `LocalStorageKv` on desktop**: the Tauri webview has its own persistent localStorage, so no IPC is needed for settings, memory or achievements, and reads stay synchronous as in V1.
- `platform::BrowserClock` (`js_sys::Date::new_0()`, using `get_full_year`/`get_month`/… for local time and `to_iso_string` for UTC) and `platform::BrowserRng` (`js_sys::Math::random`).
- `AppCore` is created once in `main` and provided via `provide_context`. Construct in the V1 order (§1.10): settings → memory → achievements (collect the streak unlocks; no toast) → vector store (+ health check if enabled) → random events (with `clock.now().hour`). Run `initialize_defaults()` when the root component mounts, then call the unconditional RAG health check.
- Reactive bridging: wrap each core signal into Leptos signals. For example, keep a `RwSignal<u64>` "settings revision" bumped by `settings_changed`, and implement `use_setting_int(key, default)` as a `Memo` that reads `revision` and `settings.get_int`. Do the same for `achievements_changed` and `user_name_changed`.
- `services::rag::VectorDatabaseService` is the async part of §11 (`check_server_health`, `add_entry`, `search`). It uses the UI `Transport` trait: Web → `fetch` POST `{base}/app/api/rag/{health,embed}`; Desktop → Tauri `invoke("rag_health"|"rag_embed")`. Reproduce the setter semantics on top of `RagConfig`.

### Crates `amadeus-backend` / `amadeus-server` / `amadeus-desktop`
- `amadeus_backend::rag::{health(server_url) -> RagHealthResponse, embed(server_url, text) -> (StatusCode, Value)}` with the exact status codes and messages from §11.5. Health uses a 5 s timeout; embed has none in V1, so consider adding a generous one and documenting it.
- `amadeus-server` (axum): `POST {base}/app/api/rag/health`, `POST {base}/app/api/rag/embed` (V1 base `/amadeus`).
- `amadeus-desktop` (Tauri 2): `#[tauri::command] async fn rag_health(server_url: String)` and `rag_embed(server_url: String, text: String)` delegating to the backend.

### Naming map (V1 → V2)
| V1 | V2 |
|---|---|
| `AppSettings` | `amadeus_core::settings::AppSettings` |
| `DEFAULT_SETTINGS` | `settings::defaults::DEFAULT_SETTINGS` |
| `t`, `LANGUAGE_NAMES` | `i18n::t`, `i18n::LANGUAGE_NAMES` |
| `stripEmotionTag` / `parseEmotionTag` | `chat::emotion::strip_emotion_tag` / `parse_emotion_tag` |
| `extractMemoryCommands` / `findMemoryCommandEnd` | `chat::memory_cmd::extract_memory_commands` / `find_memory_command_end` |
| `extractUserNameFromFact` | `chat::memory_cmd::extract_user_name_from_fact` |
| `isPauseCharacter` / `charDelayForChar` | `chat::pacing::is_pause_character` / `char_delay_for_char` |
| `splitMessage` (UI) | `chat::pacing::split_message` |
| `MemoryManager` | `memory::MemoryManager` |
| `generateQuestions` / `resultEmotion` / `resultCommentKey` | `minigame::generate_questions` / `result_emotion` / `result_comment_key` |
| `RandomEventService` | `events::RandomEventService` |
| `AchievementManager` | `achievements::AchievementManager` |
| `VectorDatabase` | `rag::VectorStore` (core) + `amadeus_ui::services::rag::VectorDatabaseService` |
| `getSystemPromptForLanguage` / `getCurrentChatLanguageIndex` (UI) | `chat::persona::system_prompt_for_language` / `settings::AppSettings::chat_language_index` |
| `getDefaultModelForProvider` / `getProviderDisplayName` / `getLlmStatusFromSettings` (UI) | `providers::*` |

---

## Open questions / risks

1. **Same-origin requirement.** localStorage is per origin. To keep existing users' settings, memory and achievements, V2 Web must be served from the exact origin of V1, presumably `https://dsuv.elvelt.com`; path does not matter. Desktop builds start empty: the Tauri webview origin differs. Decide whether desktop needs an import or migration path, for example reusing the export JSON.
2. **Desktop persistence of webview localStorage.** WebView2 (Windows), WKWebView (macOS) and WebKitGTK (Linux) all persist localStorage under Tauri, but data can be wiped by webview-data resets. Fallback design: a file-backed `KvStore` with an in-memory cache loaded at startup through Tauri IPC (async load, sync reads, async flush on `set`/`save`).
3. **Japanese collation** (`localeCompare(…, "ja")`) decides the order of equal-strength, same-day facts in the prompt. ICU4X `icu_collator` reproduces it but adds wasm size, roughly hundreds of KB with ja data. The alternative (code-point order) changes which facts appear in the top 6 for ties. Product decision needed; the recommended default is ICU4X behind a feature flag that is on.
4. **Intentional V1 quirks that look like bugs.** Keep or fix? Keep by default for parity and fix only with sign-off:
   - same-day episodes show the oldest 4 (§7.5);
   - login always overwrites `userName` with "Salieri"; logout cannot clear it (§7.11);
   - `名前は…` false positives (§6.4);
   - FIFO fact pruning (§7.8);
   - "game win" means ≥ 1 correct answer (§8.4);
   - import drops the system prompt (§12.5);
   - `！？` produces two pages (§6.6);
   - only the last achievement toast of a burst is visible (§10.5);
   - streak achievements are unlocked silently at startup (§10.3).
5. **React timing semantics** (§7.10). The request is sent with pre-increment `totalInteractions`, `lastSessionDate` and turn count and with untrimmed history; the stored history is trimmed afterwards. V2 must encode this order explicitly; there is no implicit batching.
6. **RAG is dead code in V1.** Wiring `search`/`getContextForPrompt` into the prompt, or auto-`addEntry` of messages, would change model behaviour. It is out of scope for a parity port unless requested. The unclamped dialog path (topK ≤ 0, threshold > 1) and the localStorage quota for embeddings are latent issues.
7. **Experimental TTS/STT/RAG settings have no V1 UI.** They can only be enabled by editing localStorage. Decide whether V2 exposes them, which is a UX change.
8. **`Config_DesktopNotifications` and `Config_LightweightMode`** are stored but unused in V1. Desktop V2 might give them meaning: native notifications or a reduced Live2D frame rate.
9. **Unicode edge cases.**
   - UTF-16 vs UTF-8 for the 50/300-unit truncation and per-unit typewriter ticks (§1.1).
   - JS vs Rust whitespace sets (§1.2) and case folding (§1.3).
   - Lone-surrogate JSON (§1.7).
   All are low-impact, but they need the helper module so that nobody "fixes" them inconsistently.
10. **Corrupted-storage behaviour.** V1 sometimes throws, for example a numeric `userName`, or resets all memory, for example a non-string tag. V2 should coerce leniently (§7.9), a documented divergence. Confirm that resetting all memory on a non-iterable `userFacts` is still desired; matching V1 here is recommended.
11. **Date parsing leniency.** V8 accepts `2026-02-30` and rolls it over (§7.3). Replicate it, or treat it as invalid, which would replace it with today. Impact is only on hand-edited data.
12. **ja.json vs in-code defaults** (69 differences, Appendix E). The effective Japanese text comes from ja.json. Make sure the Rust constants are not used where V1 uses `t()`. Also decide whether to update the in-code defaults to match ja.json; that is harmless.
13. **Hour-bucket boundaries differ by design:**

    | Module | Buckets |
    |---|---|
    | Random events | 0–4 / 5–9 / 10–13 / 14–17 / 18–21 / 22–23 |
    | Time context | 0–4 / 5–9 / 10–11 / 12–13 / 14–16 / 17–19 / 20–23 |
    | Achievements | night_owl 0–3, early_bird 5–6 |

    Do not unify them. The random-event bucket is frozen at page load, not at login time.
14. **`Signal` deduplication** of identical handlers cannot be reproduced with Rust closures; V1 never relies on it.
15. **Persona prompts do not mention `[SAVE_FACT]`/`[SAVE_EVENT]`.** Confirm with the chat-route spec whether the backend injects instructions for them. Otherwise memory commands only occur when the model invents them.
16. **Stored-number formatting.** Settings floats are written with JS formatting. Use `ryu-js` to avoid `"1.0"` vs `"1"` drift, which would still parse but would differ byte-wise from V1. Integer JSON fields must not gain `.0`.

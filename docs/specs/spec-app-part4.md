# Porting spec — app-part4: ConfigPanel + root `RealAmadeusApp`

Source of truth: `/home/elvelt_admin/Elvelt/dsuv/components/amadeus/RealAmadeusApp.tsx`, **lines 3061–4779** (the end of the file; the file has 4779 lines; the working tree has uncommitted edits, so this spec describes the working tree as of 2026-11-16).
Target crates: `amadeus-core` (pure logic), `amadeus-ui` (Leptos 0.8 CSR), `amadeus-desktop` (Tauri 2), `amadeus-server` (axum).

Conventions in this document:
- "V1" = the current Next.js app. "Rust port" = V2.
- `p` = the selected LLM provider index (`state.provider`).
- `lang` = the UI language index (`Config_Language`, 0 = ja, 1 = en, 2–12 = other).
- Every string in `"..."` is verbatim; `\n` means a real newline character in the JS string.
- **QUIRK-n** marks behaviour that looks unintended. Section 10 catalogues them, each with a replicate-or-fix recommendation.

---

## 0. Symbol map

| Symbol | Lines | Exported | Purpose | Rust target |
|---|---|---|---|---|
| `ConfigPanelProps`, `ConfigPanel` | 3061–3663 | no | desktop CONFIG overlay (5 categories) | `amadeus-ui::components::config::ConfigPanel` + `amadeus-core::config_form` |
| `ConfigSelectRow` | 3665–3727 | no | custom dropdown row | `amadeus-ui::components::config::ConfigSelectRow` |
| `ConfigInputRow` | 3729–3748 | no | text/password input row | `ConfigInputRow` |
| `ConfigCheckRow` | 3750–3769 | no | checkbox row | `ConfigCheckRow` |
| `ConfigSliderRow` | 3771–3808 | no | range slider row | `ConfigSliderRow` |
| `getDefaultModelForProvider` | 3810–3831 | yes | model name shown in the status when none is configured | `amadeus-core::providers::default_model_for_provider` |
| `getProviderDisplayName` | 3833–3854 | yes | provider label for the status | `amadeus-core::providers::provider_display_name` |
| `getLlmStatusFromSettings` | 3856–3871 | yes | builds `LlmStatus` from settings | `amadeus-core::providers::llm_status_from_settings` |
| `MarqueeStatus` | 3873–3902 | no | mobile top-bar provider/model ticker | `amadeus-ui::components::MarqueeStatus` |
| `RealAmadeusApp` | 3904–4708 | yes (page entry) | root: screen state machine, menu, overlays, keyboard, fullscreen, scaling, import/export, backlog store | `amadeus-ui::app::RealAmadeusApp` |
| `MenuItem` | 4710–4752 | no | big menu entry (icon + label) | `amadeus-ui::components::menu::MenuItem` |
| `SmallMenuLabel` | 4754–4778 | no | small text-only menu entry | `amadeus-ui::components::menu::SmallMenuLabel` |

Mount context (outside this range, but it is the DOM the root lives in):
- `app/amadeus/app/page.tsx`: `"use client"; export default function Home() { return <RealAmadeusApp />; }` → URL `/amadeus/app`. There is no Next `basePath`, so static assets are at `/resources/...` and the API is at `/amadeus/app/api/...`.
- `app/amadeus/app/layout.tsx` wraps the root in `<div className="amadeus-root h-screen w-screen overflow-hidden">` (Tailwind `h-screen w-screen overflow-hidden`) and emits `<meta httpEquiv="Cache-Control" content="no-cache, no-store, must-revalidate" />`, `<meta httpEquiv="Pragma" content="no-cache" />`, `<meta httpEquiv="Expires" content="0" />`. Metadata: title `"Amadeus"`, description `"Real Amadeus Web"`, manifest `/amadeus/app/manifest.webmanifest`, icons `/amadeus_logo.png`, viewport `width=device-width, initial-scale=1, viewport-fit=cover`, theme color `#000000`, apple-web-app capable, status bar `black-translucent`.
- Site root layout: `<html lang="en" class="dark">`, `<body class="bg-black text-white min-h-screen selection:bg-purple-500/30 selection:text-white">`. Its `LanguageProvider` (`hooks/useTranslation.tsx`) is **not** used by the Amadeus app.
- For the Trunk `index.html`, reproduce: `html.dark > body.(those classes) > div.amadeus-root.h-screen.w-screen.overflow-hidden > [app root]`.

---

## 1. Dependencies defined outside the range (the port needs them)

### 1.1 Types (verbatim)

```ts
// RealAmadeusApp.tsx L45-62
type OverlayPanel =
  | "none" | "backlog" | "config" | "status" | "changeLog" | "help"
  | "confirmLogout" | "confirmShutdown" | "minigame" | "achievements";

type BackLogEntry = { role: string; nameColor: string; message: string; };

// L242-263
type ConfigIoResult = { success: boolean; message: string; };
type ImportedMessage = { role: string; content: string; };
type ConversationBridge = {
  getHistory: () => ChatMessage[];
  getTurnCount: () => number;
  applyImported: (messages: ChatMessage[], turnCount: number) => void;
};
function toChatRole(role: string): ChatRole {
  if (role === "system" || role === "user" || role === "assistant") { return role; }
  return "assistant";
}

// lib/amadeus/types.ts
export type AppState = "login" | "boot" | "chat";
export type ChatRole = "system" | "user" | "assistant";
export interface ChatMessage { role: ChatRole; content: string; }
export interface LlmStatus { provider: string; model: string; latencyMs: number; }
```

`"minigame"` is in the `OverlayPanel` union but nothing sets it. ChatPanel tracks its minigame with its own boolean.

### 1.2 Constants (verbatim, L325-328, and `lib/amadeus/localization.ts` L31-45)

```ts
const PROVIDER_NAMES = ["OpenAI", "Google Gemini", "Anthropic Claude", "Groq", "Vertex AI", "Ollama", "OpenRouter", "", "FreeAPI"];
const TTS_SPEAKER_NAMES = ["christina-jp", "christina"];
const STT_PROVIDER_NAMES = ["groq", "openai", "local"];
const FREEAPI_PROVIDER_INDEX = 8;

export const LANGUAGE_NAMES = ["日本語","English","中文","한국어","Español","Français","Deutsch","Русский","Українська","Português","Türkçe","עברית","العربية"];
```

Provider indices: 0 OpenAI, 1 Gemini, 2 Claude, 3 Groq, 4 Vertex AI, 5 Ollama, 6 OpenRouter, 7 *(unused, empty name)*, 8 FreeAPI.

### 1.3 `AppSettings` storage semantics (`lib/amadeus/app-settings.ts`, 137 lines)

The port must reproduce these **exactly**, because the localStorage keys and formats stay identical to V1.
- Storage: `window.localStorage` with **un-prefixed** keys (for example `localStorage["Config_Language"] = "0"`). Every value is a string. Without a window, an in-memory map is used (SSR only).
- `getInt(key, def=0)`: raw missing → `def`. Otherwise `Number.parseInt(raw, 10)`; a non-finite result (NaN) → `def`. JS parseInt semantics: skips leading whitespace (ECMAScript WhiteSpace + LineTerminator), accepts an optional sign, reads digits until the first non-digit, and needs at least one digit. Examples: `"1.9"`→1, `" 2x"`→2, `"1e3"`→1, `""`→def, `"0x10"`→0.
- `getFloat(key, def=0)`: `Number.parseFloat(raw)` (leading whitespace, sign, digits, fraction, exponent, and the literal `Infinity`). A non-finite result → `def`. Examples: `"0.8abc"`→0.8, `".5"`→0.5, `"Infinity"`→def.
- `getString(key, def="")`: raw, or `def` if missing. An empty string is a present value and is returned as `""`.
- `setInt(key, v)`: writes `String(Math.trunc(v))`, then emits `settingsChanged(key)`.
- `setFloat(key, v)`: writes `String(v)` (ECMAScript Number→String: `1` → `"1"`, `0.8` → `"0.8"`, `0.30000000000000004` stays as is, `-0` → `"0"`), then emits `settingsChanged(key)`.
- `setString(key, v)`: writes `v`, then emits.
- `save()`: no-op.
- `initializeDefaults()`: for every key in `DEFAULT_SETTINGS` that is not present, writes `String(value)` through `writeRaw`. **This does not emit `settingsChanged`.**
- `settingsChanged` is an in-process `Signal<[string]>`. Handlers run synchronously, in insertion order, over a copy of the handler set. Cross-tab `storage` events are **not** observed.
- `DEFAULT_SETTINGS` (verbatim): `Config_SkipLoading: 0, Config_RightClickMenu: 1, Config_EyeTracking: 0, Config_DesktopNotifications: 1, Config_LightweightMode: 0, Config_TextSpeed: 1.0, Config_AutoMode: 0, Config_AutoSpeed: 3.0, Config_MasterVol: 1.0, Config_BGMVol: 0.8, Config_SEVol: 1.0, Config_VoiceVol: 1.0, Config_ScreenMode: 1, Config_Resolution: 0, Config_ApiProvider: 8, Config_ApiKey: "", Config_ModelName: "", Config_Language: 0, Config_ChatLanguage: 0, Config_WebSearch: 0, Config_VertexProject: "", Config_VertexLocation: "us-central1", Config_GCloudCLI: 1, Config_LocalProxyHost: "127.0.0.1", Config_LocalProxyPort: "", Config_OllamaHost: "http://localhost:11434", Config_OpenAICompatible: 0, Config_OpenAIBaseUrl: "", Config_Experimental_TTS_Enabled: 0, Config_Experimental_TTS_Speaker: "christina-jp", Config_Experimental_TTS_ServerUrl: "http://localhost:5100", Config_Experimental_STT_Enabled: 0, Config_Experimental_STT_Provider: "groq", Config_Experimental_STT_ServerUrl: "http://localhost:5101", Config_Experimental_STT_Language: "ja", Config_Experimental_RAG_Enabled: 0, Config_Experimental_RAG_TopK: 5, Config_Experimental_RAG_Threshold: 0.7, Config_Experimental_RAG_ServerUrl: "http://localhost:5102"`, plus `Config_ApiKey_0..8` and `Config_ModelName_0..8` all `""`. Written strings for the float defaults are `"1"`, `"3"`, `"0.8"`, `"0.7"`.

### 1.4 `t()` (`lib/amadeus/localization.ts` L47-53, verbatim)

```ts
export function t(key: string, defaultValue: string, configLanguage: number): string {
  const dict = translations[configLanguage.toString()] || ja;   // index → dict; unknown index → ja dict
  if (dict && key in dict) { return dict[key]; }
  return defaultValue || key;                                    // empty default → the key itself
}
```

Dicts: `'0' ja, '1' en, '2' zh, '3' ko, '4' es, '5' fr, '6' de, '7' ru, '8' uk, '9' pt, '10' tr, '11' he, '12' ar` (files under `lib/amadeus/locales/*.json`, embedded verbatim in `amadeus-core::i18n`). A missing key falls back to the caller's default, and every caller in this range builds the default as `lang === 1 ? en : ja`, so **languages 2–12 fall back to Japanese**.

### 1.5 Hooks and helpers (verbatim semantics)

```ts
// L413-421 — note: the root computes this but never uses the result (QUIRK-9)
function resolveResolution(index) { if (index===1) return {width:1600,height:900}; if (index===2) return {width:1280,height:720}; return {width:1920,height:1080}; }

// L423-443 — reactive settings read; re-evaluated on every settingsChanged emit (any key)
function useAppSettingInt(key, def) { return useSyncExternalStore(subscribeSettings, () => AppSettings.getInt(key, def), () => def); }
function useAppSettingFloat(key, def) { /* same with getFloat */ }

// L445-475 — mobile layout detection
// subscribes to: window "resize", window.visualViewport "resize", matchMedia("(pointer: coarse)") "change"
function isMobileViewportSnapshot(): boolean {
  const visualWidth = window.visualViewport?.width ?? window.innerWidth;
  const coarsePointer = window.matchMedia("(pointer: coarse)").matches;
  return visualWidth <= 900 || (coarsePointer && visualWidth <= 1200);
}   // server snapshot: false

// L477-483
function isTextInputTarget(target): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || target.isContentEditable;
}   // NOTE: true for ANY <input>, including checkbox and range
```

`splitMessage(text)` (L207-240) is used by the import path:
```ts
function splitMessage(text: string): string[] {
  if (!text) return [];
  const result = []; let startIdx = 0; let i = 0;
  while (i < text.length) {
    if (i === startIdx) {                                      // skip leading " ", "\t", "\n", "\r" of each segment
      let ch = text.charAt(i);
      while (i < text.length && (ch === " " || ch === "\t" || ch === "\n" || ch === "\r")) { i += 1; if (i < text.length) ch = text.charAt(i); }
      startIdx = i;
    }
    if (i >= text.length) break;
    const c = text.charAt(i); const nextC = i + 1 < text.length ? text.charAt(i + 1) : "";
    i += 1;
    if (isPauseCharacter(c, nextC) || i === text.length) {
      const segment = text.substring(startIdx, i);
      if (segment.trim().length > 0) result.push(segment);
      startIdx = i;
    }
  }
  return result;
}
// lib/amadeus/chat-utils.ts L195-211
export function isPauseCharacter(char, nextChar) {
  if (char === ".") { if (nextChar === ".") return false; if (nextChar === " " || nextChar === "\n" || nextChar === "\r" || nextChar === "") return true; return false; }
  const isPause = char === "。" || char === "！" || char === "？" || char === "!" || char === "?" || char === "\n";
  if (!isPause) return false;
  return !["」", "）", ")", "』", "”"].includes(nextChar);
}
```
Iterating over chars in Rust gives the same result: splits only happen after BMP pause characters, so surrogate pairs are never split.

FreeAPI quota (L330-411):
```ts
type FreeApiQuota = { count: number; max: number; remaining: number };
async function fetchFreeApiQuota() {           // GET /amadeus/app/api/chat/quota, cache:"no-store"
  try { const r = await fetch("/amadeus/app/api/chat/quota", { cache: "no-store" });
        if (!r.ok) return null;
        const d = await r.json().catch(() => null);
        if (d && typeof d.count==="number" && typeof d.max==="number" && typeof d.remaining==="number") return d;
        return null; } catch { return null; }
}
function useFreeApiQuota(active: boolean) {    // fetch each time `active` goes false→true; result kept (NOT cleared) when inactive
  const [quota, setQuota] = useState(null);
  useEffect(() => { if (!active) return; let cancelled = false;
    void fetchFreeApiQuota().then((r) => { if (!cancelled) setQuota(r); });
    return () => { cancelled = true; }; }, [active]);
  return quota;
}
```
The `FreeApiQuotaDisplay` DOM is in §3.11. Server side (`app/amadeus/app/api/chat/quota/route.ts`) returns `{ count, max: 65000, remaining: max(0, max-count) }` with the header `Cache-Control: no-store`.

### 1.6 Services and other modules this range calls

| Call | Where | Behaviour that matters here |
|---|---|---|
| `TTSService.checkServerHealth()` | root mount; ConfigPanel open | `POST /amadeus/app/api/tts/health`, headers `{"Content-Type":"application/json"}`, body `{"serverUrl": <TTS url>}`. Response JSON: `{available:true, ...fields from <url>/health}`, or `{available:false, error}` (HTTP 400 when serverUrl is empty after trim). Updates `serverAvailable`, `usingGpu`, `runtimeDevice`, `runtimeDetails` (consumed by StatusPanel). |
| `STTService.checkServerHealth()` | root mount | `POST /amadeus/app/api/stt/health`, same headers, body `{"serverUrl": <STT url>}`; reads `available`. |
| `VectorDatabase.checkServerHealth()` | root mount | `POST /amadeus/app/api/rag/health`, same shape. |
| `TTSService.enabled/speaker/serverUrl =` | ConfigPanel Apply | Setters return early when the value is unchanged. Each service subscribes to `settingsChanged` and re-reads its `Config_Experimental_TTS_*` / `STT_*` / `RAG_*` keys **synchronously when those keys are written**. Apply writes the keys first, so these setter calls are **no-ops** (no health check, no events). |
| `STTService.enabled/provider/serverUrl/language =` | Apply | Also no-ops, for the same reason. In particular, disabling STT here does **not** call `stopRecording()`. |
| `VectorDatabase.enabled/serverUrl =` | Apply | No-ops. |
| `VectorDatabase.topK = n` | Apply | Setter clamps `max(1, min(20, trunc(n)))`. If the clamped value differs from the synced value it rewrites `Config_Experimental_RAG_TopK`, so the net stored value is clamped (see §3.7). |
| `VectorDatabase.threshold = x` | Apply | Setter clamps `max(0, min(1, x))` and rewrites the key if `abs(diff) >= Number.EPSILON`. |
| `AchievementManager.notifyEvent("export" \| "import")` | root export/import | Unlocks achievement `export_chat` ("記録者") / `import_chat` ("時間旅行者"). |
| `MemoryManager.setUserName("")` | logout confirm | **No-op**: `setUserName` trims and returns early on an empty name (QUIRK-2). |
| `preloadLive2D()` | root mount | Injects `<script src="/resources/live2d/live2dcubismcore.min.js" data-core="true">` and `<script src="/resources/live2d/reama-cubism-runtime.js?v=19" data-runtime="true">` (async), then calls `window.ReamaCubismRuntime.preload()`. Errors go to `console.warn("Live2D preload warning:", err)`. In V2 this becomes `amadeus_live2d::preload()`. |
| `AudioService` | — | Reads `Config_VoiceVol` **once at module load** and has no settings listener, so a changed voice volume only takes effect after a reload (QUIRK-14). |

---

## 2. Settings catalogue (every key this part reads or writes)

"Read default" is the default ConfigPanel passes when loading. After the root runs `initializeDefaults()` the keys exist, so the defaults only matter for corrupt values or values deleted at runtime.

| # | Key | Stored format | DEFAULT_SETTINGS | ConfigPanel read | UI (category → control) | Apply writes | V1 runtime consumers / effect |
|---|---|---|---|---|---|---|---|
| 1 | `Config_Language` | int | 0 | `getInt(…,0)` | System → select `LANGUAGE_NAMES` | `setInt(state.language)`, then again via `onLanguageChange` | root `configLanguage` (every `t()`); ChatPanel system prompt language when ChatLanguage = 0 |
| 2 | `Config_ChatLanguage` | int: 0 = same as UI, n = LANGUAGE index n-1 | 0 | `getInt(…,0)` | System → select `["表示言語と同じ"(tr), ...LANGUAGE_NAMES]` | `setInt` | ChatPanel `getCurrentChatLanguageIndex()` |
| 3 | `Config_SkipLoading` | "0"/"1" | 0 | `getInt(…,0)===1` | System → checkbox | `setInt(b?1:0)` | LoadingPanel skips the boot sequence via `setTimeout(onFinished,0)`; MobileLoadingPanel calls `onFinished()` synchronously in its effect (otherwise it finishes after 1500 ms) |
| 4 | `Config_RightClickMenu` | "0"/"1" | 1 | `getInt(…,1)===1` | System → checkbox | `setInt` | desktop root `onContextMenu` (read live at event time) |
| 5 | `Config_EyeTracking` | "0"/"1" | 0 | `getInt(…,0)===1` | System → checkbox | `setInt` | ChatPanel → Live2D eye tracking |
| 6 | `Config_DesktopNotifications` | "0"/"1" | 1 | `getInt(…,1)===1` | System → checkbox ("Show Notifications") | `setInt` | **none** (stored only) |
| 7 | `Config_LightweightMode` | "0"/"1" | 0 | `getInt(…,0)===1` | System → checkbox | `setInt` | **none** (stored only) |
| 8 | `Config_TextSpeed` | float | 1.0 ("1") | `getFloat(…,1.0)` | Text → slider 0.1–3, step 0.1, shown `toFixed(1)` | `setFloat` | ChatPanel typewriter speed |
| 9 | `Config_AutoMode` | "0"/"1" | 0 | `getInt(…,0)===1` | Text → checkbox | `setInt` | root `autoMode` → ChatPanel auto-advance + "Auto" label; F3 toggles it |
| 10 | `Config_AutoSpeed` | float, seconds | 3.0 ("3") | `getFloat(…,3.0)` | Text → slider 1–10, step 0.1, suffix "s" | `setFloat` | root `autoSpeed` → ChatPanel |
| 11 | `Config_MasterVol` | float 0–1 | 1.0 | `getFloat(…,1.0)` | Sound → slider 0–1, step 0.05, shown as % | `setFloat` | **none** |
| 12 | `Config_BGMVol` | float | 0.8 | `getFloat(…,0.8)` | Sound → slider | `setFloat` | **none** |
| 13 | `Config_SEVol` | float | 1.0 | `getFloat(…,1.0)` | Sound → slider | `setFloat` | **none** |
| 14 | `Config_VoiceVol` | float | 1.0 | `getFloat(…,1.0)` | Sound → slider | `setFloat` | AudioService initial volume (applies after reload only) |
| 15 | `Config_ScreenMode` | int: **0 = fullscreen, 1 = windowed** | 1 | `getInt(…,1)` | Graphics → select [Fullscreen, Windowed] | `setInt` + request/exit fullscreen | root `.window-frame` class, menu item 3 label; also written by F11, menu item 3, and `fullscreenchange` |
| 16 | `Config_Resolution` | int 0/1/2 | 0 | `getInt(…,0)` | Graphics → select `["1920x1080","1600x900","1280x720"]` | `setInt` | **none** (QUIRK-9) |
| 17 | `Config_ApiProvider` | int 0–8 | 8 | `getInt(…,8)` | API → select `PROVIDER_NAMES` | `setInt` | AIService / backend; status line |
| 18 | `Config_ApiKey` *(legacy)* | string | "" | fallback for the active provider only | (buffer) | `setString(buf[p] ?? "")` | AIService (other spec) |
| 19 | `Config_ApiKey_0`…`_6` | string | "" | `getString(…,"")` | API → key input | `setString` for **all** i = 0..6 | AIService |
| 20 | `Config_ModelName` *(legacy)* | string | "" | fallback for the active provider only | (buffer) | `setString(buf[p] ?? "")` | status line, AIService |
| 21 | `Config_ModelName_0`…`_6` | string | "" | `getString(…,"")` | API → model input | `setString` for all i = 0..6 | status line, AIService |
| — | `Config_ApiKey_7/_8`, `Config_ModelName_7/_8` | string | "" | never read by ConfigPanel | — | never written by ConfigPanel (MobileSettingsPanel does write `_8`) | `getLlmStatusFromSettings` reads `Config_ModelName_<p>` first |
| 22 | `Config_WebSearch` | "0"/"1" | 0 | `getInt(…,0)===1` | API → checkbox (p∉{5,8}) | `setInt` | AIService `webSearch` |
| 23 | `Config_OpenAICompatible` | "0"/"1" | 0 | `getInt(…,0)===1` | API → checkbox (p=0) | `setInt` | AIService; status display name |
| 24 | `Config_OpenAIBaseUrl` | string | "" | `getString(…,"")` | API → input (p=0 and compatible) | `setString` raw (no trim, no validation) | AIService; status display name |
| 25 | `Config_VertexProject` | string | "" | `getString(…,"")` | API → input (p=4) | `setString` raw | AIService |
| 26 | `Config_VertexLocation` | string | "us-central1" | `getString(…,"us-central1")` | API → input (p=4) | `setString` raw (empty stays empty; mobile writes the default instead) | AIService |
| 27 | `Config_OllamaHost` | string | "http://localhost:11434" | `getString(…,default)` | API → input (p=5) | `setString(js_trim(v) \|\| "http://localhost:11434")` | AIService |
| 28 | `Config_VertexEnabled` | int (hidden flag) | **not in defaults** | read **live on every render** of every `ConfigSelectRow` dropdown | none | never | hides option index 4 in **every** ConfigSelectRow unless it equals 1 (QUIRK-1) |
| 29 | `Config_Experimental_TTS_Enabled` | "0"/"1" | 0 | `getInt(…,0)===1` | **no UI** (round-trip) | `setInt` | TTSService |
| 30 | `Config_Experimental_TTS_Speaker` | `"christina-jp"` \| `"christina"` | "christina-jp" | `TTS_SPEAKER_NAMES.indexOf(getString(…,"christina-jp"))`, -1 → 0 | no UI | `setString(TTS_SPEAKER_NAMES[idx] ?? "christina-jp")` (normalises unknown values) | TTSService |
| 31 | `Config_Experimental_TTS_ServerUrl` | string | "http://localhost:5100" | `getString` | no UI | `setString` | TTSService |
| 32 | `Config_Experimental_STT_Enabled` | "0"/"1" | 0 | | no UI | `setInt` | STTService |
| 33 | `Config_Experimental_STT_Provider` | `"groq"` \| `"openai"` \| `"local"` | "groq" | `indexOf`, -1 → 0 | no UI | `setString(normalised)` | STTService |
| 34 | `Config_Experimental_STT_ServerUrl` | string | "http://localhost:5101" | | no UI | `setString` | STTService |
| 35 | `Config_Experimental_STT_Language` | string | "ja" | | no UI | `setString` | STTService |
| 36 | `Config_Experimental_RAG_Enabled` | "0"/"1" | 0 | | no UI | `setInt` | VectorDatabase |
| 37 | `Config_Experimental_RAG_TopK` | int | 5 | `String(getInt(…,5))` | no UI | `setInt(parseInt(s,10) \|\| 5)`, then the clamp rewrite | VectorDatabase |
| 38 | `Config_Experimental_RAG_Threshold` | float | 0.7 | `getFloat(…,0.7).toFixed(2)` (a string) | no UI | `setFloat(parseFloat(s) \|\| 0.7)`, then the clamp rewrite | VectorDatabase |
| 39 | `Config_Experimental_RAG_ServerUrl` | string | "http://localhost:5102" | | no UI | `setString` | VectorDatabase |

Keys this part never touches: `Config_GCloudCLI`, `Config_LocalProxyHost`, `Config_LocalProxyPort`, `Config_AudioInputEnabled` (ChatPanel).

---

## 3. ConfigPanel (L3061–3663)

### 3.1 Props

```ts
type ConfigPanelProps = {
  visible: boolean;                                   // root: activeOverlay === "config"
  onLanguageChange: (value: number) => void;          // root: AppSettings.setInt("Config_Language", value)
  onExportConversation: () => ConfigIoResult;         // root exportConversation (§4.1)
  onImportConversation: (raw: string) => ConfigIoResult; // root importConversation (§4.2)
  onClose: () => void;                                // root: setActiveOverlay("none")
};
```
The panel is **always mounted** in the desktop tree; visibility is only opacity and pointer-events. Consequences: `activeCategory`, `ioStatus` and each select's `isOpen` **persist across open/close**. Only the form `state` and the two buffers are reloaded on each open.

### 3.2 Internal state and initial values (before the first load)

- `activeCategory = 0` (0 System, 1 Text, 2 Sound, 3 Graphics, 4 API).
- `ioStatus = { success: false, message: "" }`.
- `importInputRef` points at the hidden `<input type=file>`.
- `state` initial (only visible before the first load, and the panel is hidden then):
  `{language:0, chatLanguage:0, skipLoading:false, rightClickMenu:true, eyeTracking:false, showNotifications:false, lightweightMode:false, textSpeed:1, autoMode:false, autoSpeed:3, masterVol:1, bgmVol:0.8, seVol:1, voiceVol:1, screenMode:0, resolution:0, provider:0, openaiCompatible:false, openaiBaseUrl:"", apiKey:"", modelName:"", webSearch:false, vertexProject:"", vertexLocation:"us-central1", ollamaHost:"http://localhost:11434", ttsEnabled:false, ttsSpeaker:0, ttsServerUrl:"http://localhost:5100", sttEnabled:false, sttProvider:0, sttServerUrl:"http://localhost:5101", sttLanguage:"ja", ragEnabled:false, ragTopK:"5", ragThreshold:"0.70", ragServerUrl:"http://localhost:5102"}`
- `apiKeyBuffer: Record<number,string> = {}`, `modelNameBuffer: Record<number,string> = {}`.
- `freeApiQuota = useFreeApiQuota(visible && state.provider === 8)`.

### 3.3 Load on open (effect on `[visible]`; runs when `visible` becomes true)

```
if !visible: return
provider = getInt("Config_ApiProvider", 8)
for i in 0..=6:                                   // NOTE: 7 entries only (0..6)
    apiKey = getString("Config_ApiKey_" + i, "")
    if apiKey == "" && i == provider: apiKey = getString("Config_ApiKey", "")       // legacy migration
    localApi[i] = apiKey
    modelName = getString("Config_ModelName_" + i, "")
    if modelName == "" && i == provider: modelName = getString("Config_ModelName", "")
    localModel[i] = modelName
apiKeyBuffer = localApi; modelNameBuffer = localModel          // replace whole maps
speakerIndex = TTS_SPEAKER_NAMES.indexOf(getString("Config_Experimental_TTS_Speaker", "christina-jp"))
sttProviderIndex = STT_PROVIDER_NAMES.indexOf(getString("Config_Experimental_STT_Provider", "groq"))
state = {
  language: getInt("Config_Language", 0),
  chatLanguage: getInt("Config_ChatLanguage", 0),
  skipLoading: getInt("Config_SkipLoading", 0) === 1,
  rightClickMenu: getInt("Config_RightClickMenu", 1) === 1,
  eyeTracking: getInt("Config_EyeTracking", 0) === 1,
  showNotifications: getInt("Config_DesktopNotifications", 1) === 1,
  lightweightMode: getInt("Config_LightweightMode", 0) === 1,
  textSpeed: getFloat("Config_TextSpeed", 1.0),
  autoMode: getInt("Config_AutoMode", 0) === 1,
  autoSpeed: getFloat("Config_AutoSpeed", 3.0),
  masterVol: getFloat("Config_MasterVol", 1.0),
  bgmVol: getFloat("Config_BGMVol", 0.8),
  seVol: getFloat("Config_SEVol", 1.0),
  voiceVol: getFloat("Config_VoiceVol", 1.0),
  screenMode: getInt("Config_ScreenMode", 1),
  resolution: getInt("Config_Resolution", 0),
  provider,
  apiKey: localApi[provider] ?? "",          // "" for provider 7/8 (not in the map)
  modelName: localModel[provider] ?? "",
  webSearch: getInt("Config_WebSearch", 0) === 1,
  vertexProject: getString("Config_VertexProject", ""),
  vertexLocation: getString("Config_VertexLocation", "us-central1"),
  openaiCompatible: getInt("Config_OpenAICompatible", 0) === 1,
  openaiBaseUrl: getString("Config_OpenAIBaseUrl", ""),
  ollamaHost: getString("Config_OllamaHost", "http://localhost:11434"),
  ttsEnabled: getInt("Config_Experimental_TTS_Enabled", 0) === 1,
  ttsSpeaker: speakerIndex >= 0 ? speakerIndex : 0,
  ttsServerUrl: getString("Config_Experimental_TTS_ServerUrl", "http://localhost:5100"),
  sttEnabled: getInt("Config_Experimental_STT_Enabled", 0) === 1,
  sttProvider: sttProviderIndex >= 0 ? sttProviderIndex : 0,
  sttServerUrl: getString("Config_Experimental_STT_ServerUrl", "http://localhost:5101"),
  sttLanguage: getString("Config_Experimental_STT_Language", "ja"),
  ragEnabled: getInt("Config_Experimental_RAG_Enabled", 0) === 1,
  ragTopK: String(getInt("Config_Experimental_RAG_TopK", 5)),
  ragThreshold: getFloat("Config_Experimental_RAG_Threshold", 0.7).toFixed(2),   // JS toFixed: ties round up
  ragServerUrl: getString("Config_Experimental_RAG_ServerUrl", "http://localhost:5102"),
}
fire-and-forget TTSService.checkServerHealth()          // a POST /amadeus/app/api/tts/health on every open
```
When the panel closes nothing is reset, and the next open reloads everything.

### 3.4 `tr(ja, en, key?)` (panel-local translator; uses the **unsaved** `state.language`)

```ts
tr = (ja, en, key?) => key ? t(key, state.language === 1 ? en : ja, state.language)
                           : (state.language === 0 ? ja : en);
```
- Picking a new Display Language re-renders **only this panel** in that language straight away (live preview). The rest of the app changes only after Apply.
- Keyless calls give Japanese for language 0 and **English for 1–12**. Keyed calls with a missing key give English for 1 and **Japanese for 0 and 2–12** (QUIRK-12). Keyless strings in the panel: the Vertex key label and the Ollama note.

### 3.5 Keyboard (effect on `[visible, state]`; `window` `keydown`; only while visible)

```
on keydown(e):
  if isTextInputTarget(e.target): return          // ANY <input> (incl. checkbox/range), textarea, select, contentEditable
  if e.key == "Backspace": e.preventDefault(); handleApply(); onClose()
  if e.key == "ArrowUp"   || e.key.toLowerCase() == "w": activeCategory = (activeCategory - 1 + 5) % 5
  if e.key == "ArrowDown" || e.key.toLowerCase() == "s": activeCategory = (activeCategory + 1) % 5
```
- The arrow, W and S keys are **not** `preventDefault`ed. Enter, Escape, Left and Right do nothing here (there is no Escape handling).
- After a checkbox or slider is clicked it usually holds focus (in Chrome), so Backspace/W/S are ignored until focus moves (QUIRK-13). A focused range input still moves with the arrow keys natively.
- No `stopPropagation`: the root handler sees the same event, but its menu branch is gated by `!isSubPanelOpen`, while F11, F3 and Tab still apply (see §7.6 for Tab while an overlay is open).

### 3.6 `handleProviderChange(newProvider)` — keep the stale-closure semantics

```
// apiKeyBuffer / modelNameBuffer below are the values from the CURRENT render (before this update)
oldBufKey   = apiKeyBuffer[newProvider] ?? ""
oldBufModel = modelNameBuffer[newProvider] ?? ""
apiKeyBuffer[state.provider]    = state.apiKey      // store the edits of the provider being left
modelNameBuffer[state.provider] = state.modelName
state.provider = newProvider; state.apiKey = oldBufKey; state.modelName = oldBufModel
```
QUIRK-19: if `newProvider === state.provider` (the user re-selects the current provider), the input shows the **old buffered value**. Apply then uses `state.apiKey` for the current provider, so the unsaved edit is lost. Rust: read the buffer *before* inserting, exactly as above.

### 3.7 `handleApply()` — exact write order and side effects

Each `set*` emits `settingsChanged(key)` synchronously, in this order:
```
nextApi   = {...apiKeyBuffer,    [state.provider]: state.apiKey}
nextModel = {...modelNameBuffer, [state.provider]: state.modelName}
apiKeyBuffer = nextApi; modelNameBuffer = nextModel
setInt  Config_SkipLoading            skipLoading?1:0
setInt  Config_RightClickMenu         rightClickMenu?1:0
setInt  Config_EyeTracking            eyeTracking?1:0
setInt  Config_DesktopNotifications   showNotifications?1:0
setInt  Config_LightweightMode        lightweightMode?1:0
setFloat Config_TextSpeed             textSpeed
setInt  Config_AutoMode               autoMode?1:0
setFloat Config_AutoSpeed             autoSpeed
setFloat Config_MasterVol             masterVol
setFloat Config_BGMVol                bgmVol
setFloat Config_SEVol                 seVol
setFloat Config_VoiceVol              voiceVol
setInt  Config_ScreenMode             screenMode
if screenMode === 0: if (!document.fullscreenElement) document.documentElement.requestFullscreen().catch(()=>{})
else:                if ( document.fullscreenElement) document.exitFullscreen().catch(()=>{})
setInt  Config_Resolution             resolution
setInt  Config_ApiProvider            provider
setInt  Config_Language               language
setInt  Config_ChatLanguage           chatLanguage
setInt  Config_WebSearch              webSearch?1:0
setInt  Config_OpenAICompatible       openaiCompatible?1:0
setString Config_OpenAIBaseUrl        openaiBaseUrl
setString Config_VertexProject        vertexProject
setString Config_VertexLocation       vertexLocation
setString Config_OllamaHost           js_trim(ollamaHost) || "http://localhost:11434"
for i in 0..=6: setString Config_ApiKey_i  (nextApi[i] ?? "");  setString Config_ModelName_i (nextModel[i] ?? "")   // interleaved per i
setString Config_ApiKey               nextApi[provider] ?? ""       // provider 8 → "" (clears the legacy key)
setString Config_ModelName            nextModel[provider] ?? ""
setInt  Config_Experimental_TTS_Enabled   ttsEnabled?1:0
setString Config_Experimental_TTS_Speaker TTS_SPEAKER_NAMES[ttsSpeaker] ?? "christina-jp"
setString Config_Experimental_TTS_ServerUrl ttsServerUrl
setInt  Config_Experimental_STT_Enabled   sttEnabled?1:0
setString Config_Experimental_STT_Provider STT_PROVIDER_NAMES[sttProvider] ?? "groq"
setString Config_Experimental_STT_ServerUrl sttServerUrl
setString Config_Experimental_STT_Language  sttLanguage
setInt  Config_Experimental_RAG_Enabled   ragEnabled?1:0
setInt  Config_Experimental_RAG_TopK      parseInt(ragTopK,10) || 5          // 0 → 5 (QUIRK-20)
setFloat Config_Experimental_RAG_Threshold parseFloat(ragThreshold) || 0.7  // 0 → 0.7
setString Config_Experimental_RAG_ServerUrl ragServerUrl
TTSService.enabled = ttsEnabled; TTSService.speaker = TTS_SPEAKER_NAMES[ttsSpeaker] ?? "christina-jp"; TTSService.serverUrl = ttsServerUrl   // no-ops
STTService.enabled = sttEnabled; .provider = STT_PROVIDER_NAMES[sttProvider] ?? "groq"; .serverUrl = sttServerUrl; .language = sttLanguage // no-ops
VectorDatabase.enabled = ragEnabled                                         // no-op
VectorDatabase.topK = parseInt(ragTopK,10) || 5        // clamp 1..20; if changed → setInt(TopK, clamped)
VectorDatabase.threshold = parseFloat(ragThreshold) || 0.7   // clamp 0..1; if changed → setFloat(Threshold, clamped)
VectorDatabase.serverUrl = ragServerUrl                // no-op
AppSettings.save()                                     // no-op
onLanguageChange(state.language)                       // root → setInt("Config_Language", value) again (redundant emit)
```
Net effect on the hidden RAG keys (implement directly):
- `TopK_final = clamp(1, 20, (js_parse_int(ragTopK) || 5))`, written through `setInt` (two writes when clamping changes the value).
- `Threshold_final = clamp(0, 1, (js_parse_float(ragThreshold) || 0.7))`, written as `String()`. Examples: 1.5 → "1"; −0.2 → "0"; 0 → "0.7".

Callers: the Apply button runs `handleApply(); onClose();`, and Backspace (when the target is not a text input) runs `preventDefault(); handleApply(); onClose();`.

Validation: none beyond the normalisations above. No URL validation, no trimming (except the Ollama host), no length limits.

Other side effects of Apply:
- Root re-renders (new `configLanguage`, `autoMode`, `autoSpeed`, `screenMode`).
- ChatPanel re-renders its system prompt when `lang` or ChatLanguage changed.
- The **desktop root does not refresh `llmStatus`** after Apply (QUIRK-15; mobile does).
- `requestFullscreen()` failure is swallowed and Config_ScreenMode stays 0 even though the page is not fullscreen.

### 3.8 Cancel

The Cancel button calls `onClose()` only. Nothing is written or reverted (nothing was applied live, except the in-panel language preview, which is reloaded on the next open).

### 3.9 DOM structure (CSS is reused verbatim — keep the classes, nesting and inline styles)

```
div  style="opacity:{visible?1:0}; pointer-events:{visible?'auto':'none'}; transition:opacity 0.3s ease-in-out;
            position:absolute; top:0; left:0; right:0; bottom:0; z-index:2000"
└─ div.overlay-panel.config-panel
   ├─ img.overlay-bg   src="/resources/images/Amadeus_BG.png" alt=""
   ├─ div.config-main
   │  ├─ div.config-title            "CONFIG"
   │  ├─ div.config-sidebar
   │  │   └─ ×5 button[type=button] class={activeCategory===i ? "active" : ""}  (React key = label)
   │  │        text = categoryLabels[i]; onClick → activeCategory = i
   │  └─ div.config-content
   │      ├─ div.config-section-title  {categoryLabels[activeCategory]}
   │      ├─ div.line.thin
   │      └─ category body (exactly one of the five, §3.10)
   └─ div.config-footer
       ├─ button.cancel[type=button]  {tr("キャンセル","Cancel","cancel")}   onClick → onClose()
       └─ button.apply[type=button]   {tr("適用","Apply","apply")}          onClick → handleApply(); onClose()
```
`categoryLabels = [tr("システム","System","category_system"), tr("テキスト","Text","category_text"), tr("サウンド","Sound","category_sound"), tr("グラフィック","Graphics","category_graphics"), tr("API","API","category_api")]`

### 3.10 Category bodies (rows in DOM order)

**0 — System** (`div.config-form`)
1. `ConfigSelectRow` label `tr("表示言語","Display Language","setting_display_language")`, value `state.language`, options `LANGUAGE_NAMES` → `language`.
2. `ConfigSelectRow` label `tr("会話言語","Conversation Language","setting_chat_language")`, value `state.chatLanguage`, options `[tr("表示言語と同じ","Same as Display Language","setting_chat_lang_same_as_ui"), ...LANGUAGE_NAMES]` → `chatLanguage`.
3. `ConfigCheckRow` `tr("起動画面スキップ","Skip Loading Screen","setting_skip_loading")` → `skipLoading`.
4. `ConfigCheckRow` `tr("右クリックメニュー","Right Click Menu","setting_right_click")` → `rightClickMenu`.
5. `ConfigCheckRow` `tr("視線トラッキング","Eye Tracking","setting_eye_tracking")` → `eyeTracking`.
6. `ConfigCheckRow` `tr("通知を表示","Show Notifications","setting_show_notifications")` → `showNotifications`.
7. `ConfigCheckRow` `tr("軽量化モード","Lightweight Mode","setting_lightweight")` → `lightweightMode`.

With `Config_VertexEnabled != 1`, the dropdown of row 1 hides **"Español"** (index 4) and row 2 hides **"한국어"** (index 4) — QUIRK-1.

**1 — Text** (`div.config-form`)
1. `ConfigSliderRow` `tr("文字表示速度","Text Speed","setting_text_speed")`, min 0.1, max 3, step 0.1, no suffix → shows e.g. `"1.0"`.
2. `ConfigCheckRow` `tr("オート表示","Auto Mode","setting_auto_mode")` → `autoMode`.
3. `ConfigSliderRow` `tr("オート待機時間","Auto Wait Time","setting_auto_speed")`, min 1, max 10, step 0.1, suffix `"s"` → e.g. `"3.0s"`.
4. `div.config-io-separator`
5. Export row:
   ```
   div.config-row
     span.config-label  {tr("会話記録エクスポート","Export Conversation","setting_export_chat")}
     button.config-io-button.export[type=button] {tr("書き出す","Export","export")}   onClick → ioStatus = onExportConversation()
   ```
6. Import row:
   ```
   div.config-row
     span.config-label  {tr("会話記録インポート","Import Conversation","setting_import_chat")}
     button.config-io-button.import[type=button] {tr("読み込む","Import","import")}   onClick → importInputRef.click()
     input.config-import-input[type=file][accept=".json"]      (CSS display:none)
        onChange: file = files[0]; input.value = "" (so the same file can be chosen again); if !file return;
                  file.text().then(raw => ioStatus = onImportConversation(raw))      // UTF-8, BOM stripped; no .catch
   ```
7. If `ioStatus.message !== ""`: `div.config-io-status style="color:{success ? '#00CC00' : '#FF4444'}"` → `{ioStatus.message}`.

**2 — Sound** (`div.config-form`): four `ConfigSliderRow` with min 0, max 1, step 0.05, `asPercent`:
`tr("マスター音量","Master Volume","setting_master_vol")` → masterVol; `tr("BGM音量","BGM Volume","setting_bgm_vol")` → bgmVol; `tr("SE音量","SE Volume","setting_se_vol")` → seVol; `tr("ボイス音量","Voice Volume","setting_voice_vol")` → voiceVol.

**3 — Graphics** (`div.config-form`)
1. `ConfigSelectRow` `tr("画面モード","Screen Mode","setting_screen_mode")`, options `[tr("フルスクリーン","Fullscreen","screen_mode_fullscreen"), tr("ウィンドウ","Windowed","screen_mode_windowed")]` → screenMode (0 = fullscreen).
2. `ConfigSelectRow` `tr("解像度","Resolution","setting_resolution")`, options `["1920x1080","1600x900","1280x720"]` → resolution.

**4 — API** (`div.config-form.dense`), conditional on `p`:
1. Always: `ConfigSelectRow` `tr("LLM APIプロバイダ","LLM API Provider","setting_api_provider")`, options `PROVIDER_NAMES`, `onChange = handleProviderChange`. The dropdown skips index 7 (empty name) and skips index 4 (Vertex AI) unless `Config_VertexEnabled === 1`. The trigger still shows `options[p] || ""`.
2. If `p === 8 && freeApiQuota`: `FreeApiQuotaDisplay` with className `"config-freeapi-quota"` (§3.11).
3. If `p !== 5 && p !== 8`: `ConfigInputRow`, label `p === 4 ? tr("Vertex APIキー / SA JSON","Vertex API Key / SA JSON") : tr("APIキー","API Key","setting_api_key")`, value `state.apiKey`, type `p === 4 ? "text" : "password"` → apiKey. (The desktop shows a single-line input for the SA JSON; mobile uses a textarea.)
4. If `p !== 8`: `ConfigInputRow` `tr("LLM モデル名","LLM Model Name","setting_model_name")` → modelName (type text).
5. If `p !== 5 && p !== 8`: `ConfigCheckRow` `tr("LLM Web検索","LLM Web Search","setting_web_search")` → webSearch.
6. If `p === 0`: `ConfigCheckRow` `tr("互換APIを使う","OpenAI Compatible","setting_openai_compatible")` → openaiCompatible. If `openaiCompatible` is also set: `ConfigInputRow` `tr("ベースURL","Base URL","setting_openai_base_url")` → openaiBaseUrl, followed by `div.config-note {tr("ベースURLの例: https://api.example.com/v1","Base URL Example: https://api.example.com/v1","setting_openai_base_url_example")}`.
7. If `p === 4`: `ConfigInputRow` label `"Vertex Project ID"` (literal, never translated) → vertexProject; `ConfigInputRow` label `"Vertex Location"` → vertexLocation; `div.config-note {tr("※ Vertexを使用するためには、gCloud CLIのインストールが必要です。","* gCloud CLI must be installed to use Vertex.","setting_vertex_note")}`.
8. If `p === 5`: `ConfigInputRow` label `"Ollama Host"` → ollamaHost; `div.config-note {tr("※ Ollamaはローカルで動作するLLMランナーです。デフォルトのホストは http://localhost:11434 です。","* Ollama is a local LLM runner. The default host is http://localhost:11434.")}` (keyless).

Resulting rows per provider: OpenAI = key, model, websearch, compatible (+ base URL and note); Gemini, Claude, Groq, OpenRouter = key, model, websearch; Vertex = key (text type, Vertex label), model, websearch, project, location, note; Ollama = model, host, note; FreeAPI = quota only; index 7 (corrupt value) = key, model, websearch, with an empty trigger.

### 3.11 FreeApiQuotaDisplay (L372-411) — DOM and strings

Fetch: `GET /amadeus/app/api/chat/quota` (fetch option `cache: "no-store"`), triggered when `visible && state.provider === 8` turns true. That happens once per open while provider 8 is selected, and again whenever the user switches to provider 8 inside the panel. An in-flight result is discarded when the condition turns false. The stale quota is **not** cleared, so the old value shows until the new fetch resolves. A failed fetch sets it to null, which hides the block.
```
exhausted   = quota.remaining === 0
labelColor  = exhausted ? "#FF4444" : "#FF9900"
numberColor = exhausted ? "#FF4444" : "#FFFFFF"
usageText   = tr("%1 / %2 回使用","%1 / %2 requests used","freeapi_quota_usage")
                .replace("%1", quota.count.toLocaleString()).replace("%2", quota.max.toLocaleString())   // replaces FIRST occurrence only
remainingText = tr("残り %1 回","%1 remaining","freeapi_quota_remaining").replace("%1", quota.remaining.toLocaleString())

div.{className}                       // "config-freeapi-quota" here, "mobile-settings-freeapi-quota" on mobile
  div
    span style="color:{labelColor}"                      {tr("FreeAPIクォータ","FreeAPI Quota","freeapi_quota_label")}
    span style="color:{numberColor}; margin-left:16px"   {usageText}
  div style="color:{numberColor}"                        {remainingText}
  (exhausted) div style="color:#FF4444"  {tr("全体リクエスト上限（65,000回）に達しました。サービスを停止しています。","Global request limit (65,000) reached. Service suspended.","freeapi_quota_exhausted")}
```
`toLocaleString()` uses the **browser default locale**, not the app language (e.g. en-US gives `65,000`, de-DE gives `65.000`). In Rust/WASM call `js_sys::Intl::NumberFormat` (or `Number.prototype.toLocaleString` through js_sys) with an empty locales array to match. The `freeapi_quota_*` keys exist only in ja and en, so languages 2–12 show the Japanese text.

### 3.12 Row components

**ConfigSelectRow** `{label, value:number, options:string[], onChange(index)}`; local state `isOpen=false`; ref on the shell span.
```
div.config-row  style="display:flex; align-items:center"         // inline style overrides the CSS grid
├─ span.config-label {label}
└─ span.config-select-shell (ref)
   ├─ div.custom-select-trigger{ isOpen ? " open" : " " }   {options[value] || ""}   onClick → isOpen = !isOpen
   ├─ (isOpen) div.custom-select-dropdown
   │     for (option, index): if option == "" → skip
   │                          if index == 4 && AppSettings.getInt("Config_VertexEnabled",0) != 1 → skip   // applies to EVERY select (QUIRK-1)
   │                          div.custom-select-option {option}  (key=index)  onClick → onChange(index); isOpen = false
   └─ span.config-select-arrow[aria-hidden="true"] style="pointer-events:none"
        img src="/resources/images/DownAllow.png" alt=""
```
- Outside click: while `isOpen`, register a `document` `mousedown` listener; if the event target is not inside the shell span, set `isOpen = false`. The listener is removed when the dropdown closes or the row unmounts.
- No keyboard support (the trigger is a non-focusable div). There is no `.open` CSS rule.
- Switching category unmounts the rows, which resets `isOpen`. Closing the panel with Backspace keeps `isOpen` alive, so the dropdown is still open on the next open (minor).
- The class string is `"custom-select-trigger "` with a trailing space when closed. That is harmless; exact class tokens are `custom-select-trigger` and `open`.

**ConfigInputRow** `{label, value, onChange(string), type="text", disabled=false}`:
```
label.config-row
  span.config-label {label}
  input.config-input-control type={type} value={value} disabled={disabled} onInput→onChange(target.value)
```
The `<label>` wrapper makes a click on the label text focus the input. `disabled` is never passed in this file. Fire on every input event (React `onChange` = DOM `input` event).

**ConfigCheckRow** `{label, checked, onChange(bool), disabled=false}`:
```
label.config-row.checkbox-row
  span.config-label {label}
  span.config-checkbox-shell
    input[type=checkbox] checked={checked} disabled={disabled} onChange→onChange(target.checked)
```
The whole row is clickable because the input is inside the `<label>`.

**ConfigSliderRow** `{label, value, min, max, step, onChange(number), suffix?, asPercent?}`:
```
display = asPercent ? `${Math.round((value / max) * 100)}%` : `${value.toFixed(1)}${suffix ?? ""}`
label.config-row.slider-row
  span.config-label {label}
  div.slider-wrap
    input[type=range] min={min} max={max} step={step} value={value}
          style="--fill: {((value - min) / (max - min)) * 100}%"      // JS number formatting, e.g. "31.03448275862069%"
          onInput → onChange(Number(target.value))                     // fires continuously while dragging
    div.slider-value {display}
```
Use JS-compatible `Math.round` (half rounds toward +∞), `toFixed` (an exact tie picks the larger value) and Number→String for `--fill`.

---

## 4. Conversation export / import and the backlog store (root callbacks)

### 4.1 `exportConversation(): ConfigIoResult` (L4045-4085; deps `[configLanguage]`)

```
bridge = conversationBridgeRef.current                 // set only while ChatPanel is mounted (appState "chat")
history = bridge ? bridge.getHistory() : []
exportMessages = history.filter(m => m.role !== "system")
if !bridge || exportMessages.length == 0:
    return {success:false, message: t("export_no_data", "会話データがありません", lang)}
try:
    timestamp = new Date().toISOString().replace(/[:.]/g, "-").substring(0, 19)    // UTC, e.g. "2026-11-16T12-34-56"
    fileName  = `chat_export_${timestamp}.json`
    exportData = { version: 1, exportedAt: new Date().toISOString() /* a second Date() call */,
                   turnCount: bridge.getTurnCount(), messages: exportMessages }
    json = JSON.stringify(exportData, null, 2)
    blob = new Blob([json], {type: "application/json"}); url = URL.createObjectURL(blob)
    a = document.createElement("a"); a.href = url; a.download = fileName
    document.body.appendChild(a); a.click(); document.body.removeChild(a)
    URL.revokeObjectURL(url)                               // revoked synchronously right after click()
    AchievementManager.notifyEvent("export")
    return {success:true, message: t("export_success", "エクスポート完了: ", lang) + fileName}
catch (error):
    return {success:false, message: t("export_failed", "エクスポート失敗: ", lang) + String(error)}
```

**Export file format** (byte-exact target: `JSON.stringify(x, null, 2)`, i.e. 2-space indent, `": "` separator, raw non-ASCII, escapes only `"` `\` and control characters < 0x20 as `\b \f \n \r \t` or `\u00XX`; `serde_json::to_string_pretty` produces the same bytes):
```json
{
  "version": 1,
  "exportedAt": "2026-11-16T12:34:56.789Z",
  "turnCount": 3,
  "messages": [
    {
      "role": "user",
      "content": "こんにちは"
    },
    {
      "role": "assistant",
      "content": "[SMILE] ……"
    }
  ]
}
```
- Key order: `version, exportedAt, turnCount, messages`. Each message: `role, content` (ChatMessage objects have exactly these two fields).
- No trailing newline.
- `turnCount` is a JS number. It is an integer in practice, but it can be a float after importing a file with a float (serialise integral values without `.0`; see the Open questions).
- Assistant `content` is whatever ChatPanel stored in history (see the ChatPanel spec).
- The `exportedAt` ISO string always has milliseconds and `Z` (`YYYY-MM-DDTHH:mm:ss.sssZ`).

### 4.2 `importConversation(raw): ConfigIoResult` (L4087-4162; deps `[addBackLog, configLanguage]`)

```
try:
  parsed = JSON.parse(raw)                                         // may throw → catch
  if typeof parsed != "object" || parsed == null || Array.isArray(parsed): return INVALID
  if !Array.isArray(parsed.messages): return INVALID
  imported = []
  for entry in parsed.messages:
     if typeof entry != "object" || entry == null: return INVALID   // arrays pass this test but then fail the next
     if typeof entry.role != "string" || typeof entry.content != "string": return INVALID
     imported.push({role: entry.role, content: entry.content})       // role kept raw (e.g. "kurisu", "System")
  turnCount = typeof parsed.turnCount == "number" ? parsed.turnCount : 0
  bridge = conversationBridgeRef.current
  if bridge: bridge.applyImported(imported.map(m => ({role: toChatRole(m.role), content: m.content})), turnCount)
       // ChatPanel then: history = messages (REPLACES everything, including its system prompt — QUIRK-3),
       //                 turnCount = turnCount, chatState = "inputReady", emotion = "NORMAL"
  backLogEntries = []                                               // clears the backlog even with no bridge (QUIRK-4)
  for m in imported:
     if m.role === "system": continue                               // case-sensitive
     if lower(m.role) in {"assistant","kurisu","amadeus"}:
         for seg in splitMessage(m.content): addBackLog(m.role, seg)
     else: addBackLog(m.role, m.content)
  AchievementManager.notifyEvent("import")
  return {success:true, message: t("import_success","インポート完了: ",lang) + String(imported.length) + t("import_turns"," ターン",lang)}
       // counts ALL imported messages (including system ones), not turns
catch (error):
  return {success:false, message: t("import_failed","インポート失敗: ",lang) + String(error)}
INVALID = {success:false, message: t("import_invalid","無効なファイル形式です",lang)}
```
`String(error)` for a JSON syntax error is V8's text (e.g. `SyntaxError: Unexpected token 'x', "xyz" is not valid JSON`). Rust cannot reproduce it byte-for-byte; use `format!("SyntaxError: {serde_err}")`.

Extra fields in the file (`version`, `exportedAt`, unknown keys) are ignored, and `version` is not checked.

### 4.3 `addBackLog(role, rawMessage)` (L4018-4043; stable callback)

```
clean = js_trim(rawMessage)
if clean.startsWith("["):
    close = clean.indexOf("]")
    if close > 0: clean = js_trim(clean.slice(close + 1))        // strips ANY leading [..] tag — user text too
clean = clean.replace(/\[(NORMAL|SMILE|ANGRY|SAD|SURPRISED|BLUSH|WINK|DISGUST|SMUG|THINKING|PANIC)\]/gi, "")
clean = js_trim(clean)
if clean == "": return
roleId = role.toLowerCase()
nameColor = (roleId == "user" || roleId == "me") ? "#66CCFF"
          : (roleId == "assistant" || roleId == "kurisu" || roleId == "amadeus") ? "#FF6666"
          : "#808080"
backLogEntries.push({role: roleId, nameColor, message: clean})
```
- The backlog lives only in root memory: it is unbounded, not persisted, and **not cleared on logout**. It survives a mobile↔desktop layout switch because it is root state.
- Callers: ChatPanel (`onBackLogAdd("user", text)`, `onBackLogAdd("assistant", pageText)`) and import.
- BackLogPanel renders it (other spec). Its name column shows `t("you", …)` for user/me, `t("amadeus_kurisu", …)` for assistant/kurisu/amadeus, and otherwise `role.toUpperCase()`.

---

## 5. Provider / status helpers (exported)

```ts
export function getDefaultModelForProvider(i: number): string {
  // 0 "gpt-4o" | 1 "gemini-1.5-pro" | 2 "claude-3-5-sonnet-20241022" | 3 "llama-3.3-70b-versatile"
  // 4 "gemini-1.5-pro" | 5 "llama3.2" | 6 "openai/gpt-4o" | 8 "default" | default → "default"
}
export function getProviderDisplayName(i: number, isCompatible: boolean, baseUrl: string): string {
  // 0 → isCompatible ? `OpenAI互換(${baseUrl})` : "OpenAI"   (hard-coded Japanese in every language; ASCII parentheses)
  // 1 "Gemini" | 2 "Claude" | 3 "Groq" | 4 "Vertex AI" | 5 "Ollama" | 6 "OpenRouter" | 8 "FreeAPI" | default "Unknown"
}
export function getLlmStatusFromSettings(latencyMs: number = -1): LlmStatus {
  const p = AppSettings.getInt("Config_ApiProvider", 8);
  let model = AppSettings.getString(`Config_ModelName_${p}`, "");
  if (!model) model = AppSettings.getString("Config_ModelName", "") || getDefaultModelForProvider(p);
  const isCompatible = AppSettings.getInt("Config_OpenAICompatible", 0) === 1;
  const baseUrl = AppSettings.getString("Config_OpenAIBaseUrl", "");
  return { provider: getProviderDisplayName(p, isCompatible, baseUrl), model, latencyMs };
}
```
Callers: the root on mount (`latencyMs` −1), the mobile settings save (−1), and ChatPanel after each response (measured latency). These display defaults **differ from the backend's real defaults**. For example, the backend uses `gemini-2.5-flash` for Gemini and Vertex, `claude-3-7-sonnet-20250219` for Claude and `google/gemini-2.5-flash` for OpenRouter (`app/amadeus/app/api/chat/route.ts` L243/273/298/373/410). Keep the display defaults verbatim (QUIRK-16).

---

## 6. MarqueeStatus (mobile top bar; L3873-3902)

```
text = (provider && model) ? `${provider} / ${model}` : "---"
div.mobile-topbar-status{ isOverflowing ? " marquee" : " " }   (ref container)
  span.mobile-topbar-status-inner {text}                          (ref text)
effect on [text]:
  check = () => isOverflowing = text.offsetWidth > container.clientWidth
  check(); timer = setTimeout(check, 100); window.addEventListener("resize", check)
  cleanup: clearTimeout(timer); removeEventListener
```
CSS (`globals.css` L2083-2114): `.marquee` adds `padding-left:100%` and a 12 s linear infinite translateX(0→−100%) animation to the inner span. Once `.marquee` is applied, the padding makes `offsetWidth > clientWidth` permanently true, so **the marquee never switches off** until remount (QUIRK-17). `suppressHydrationWarning` attributes are SSR-only and can be dropped.

---

## 7. Root component `RealAmadeusApp` (L3904-4708)

### 7.1 State, derived values and refs

| Name | Initial | Notes |
|---|---|---|
| `appState: AppState` | `"login"` | screen state machine (§7.5) |
| `isMobileViewport` | hook (§1.5) | chooses the mobile or desktop tree |
| `autoMode` | `useAppSettingInt("Config_AutoMode",0) === 1` | reactive |
| `autoSpeed` | `useAppSettingFloat("Config_AutoSpeed",3.0)` | reactive |
| `configLanguage` (`lang`) | `useAppSettingInt("Config_Language",0)` | reactive |
| `screenMode` | `useAppSettingInt("Config_ScreenMode",1)` | reactive |
| `resolutionIndex` | `useAppSettingInt("Config_Resolution",0)` | used only for the unused `screenResolution` |
| `isTabActive` | false | "menu mode": the system panel is zoomed out and ChatPanel paused (desktop) |
| `menuOpen` | false | menu overlay visible and interactive |
| `menuSelectedIndex` | 0 | **not** reset when the menu opens or closes |
| `activeOverlay: OverlayPanel` | `"none"` | |
| `achievementsOpen` | false | desktop: opened from StatusPanel |
| `mobileSettingsOpen` | false | |
| `dimensions` | `{width:1920, height:1080, scale:1}` | updated on mount and on resize |
| `llmStatus` | `{provider:"", model:"", latencyMs:-1}` | |
| `backLogEntries` | `[]` | |
| `confirmMessage` | `""` | text for both confirmation dialogs |
| `viewportRef` | — | assigned, never read |
| `conversationBridgeRef` | null | filled by ChatPanel on mount, nulled on unmount |

Derived: `isSubPanelOpen = activeOverlay !== "none"`; `screenResolution = resolveResolution(resolutionIndex)` (**unused**); `outerWidth = ${dimensions.width}px`; `outerHeight = ${dimensions.height}px`; `uiScale = dimensions.scale`; `systemAnimProgress = isTabActive ? 1 : 0`.

All hooks and effects run **before** the mobile/desktop branch, so every effect below is active in both layouts.

### 7.2 Mount effects (in declaration order)

**Effect A — MS Mincho font loader (once)**
```
cancelled = false; controller = new AbortController(); timeoutId = setTimeout(() => controller.abort(), 8000)
async:
  try:
    if document.fonts.check('16px "MS Mincho"'): return               // finally still clears the timeout
    resp = await fetch("https://raw.githubusercontent.com/vslabs/winfonts/master/msmincho.ttc", {signal})
    if !resp.ok: return
    buf = await resp.arrayBuffer(); if cancelled: return
    face = new FontFace("MS Mincho", buf); await face.load(); if cancelled: return
    document.fonts.add(face)
  catch: (silent — the CSS already falls back to Noto Serif JP)
  finally: clearTimeout(timeoutId)
cleanup: cancelled = true; controller.abort(); clearTimeout(timeoutId)
```
`globals.css` declares `@font-face "MS Mincho"` as `local("MS Mincho"), url(/resources/fonts/NotoSerifCJKsc-Regular.otf)`. `check()` usually returns false at mount, because that face has not loaded yet, so the download normally happens. It is a cross-origin GET of a proprietary Windows font (see Risks).

**Effect B — init (once)**
```
AppSettings.initializeDefaults()                 // no settingsChanged emission
llmStatus = getLlmStatusFromSettings()           // latencyMs -1
preloadLive2D()                                  // fire and forget
TTSService.checkServerHealth(); STTService.checkServerHealth(); VectorDatabase.checkServerHealth()   // always, even if disabled
updateDimensions(); window.addEventListener("resize", updateDimensions)
document.addEventListener("fullscreenchange", onFullscreenChange)
cleanup: remove both listeners

updateDimensions():
  scale = Math.min(window.innerWidth / 1920, window.innerHeight / 1080)
  dimensions = {width: 1920 * scale, height: 1080 * scale, scale}
onFullscreenChange():
  setInt("Config_ScreenMode", document.fullscreenElement ? 0 : 1); save()
```
Only the window `resize` event is observed here (not visualViewport). `.real-amadeus-root` safe-area padding is ignored by the computation; `.window-frame` has `max-width/max-height:100%` and `overflow:hidden`.

**Effect C — menu-open delay (on `[isTabActive]`)**
```
if !isTabActive: return
active = document.activeElement
if active is HTMLElement && (tag == "INPUT" || tag == "TEXTAREA"): active.blur()     // not SELECT / contentEditable
timer = setTimeout(() => menuOpen = true, 450)
cleanup: clearTimeout(timer)                       // toggling back within 450 ms means the menu never opens
```
`menuOpen` is set back to false by the closers (closeMenu, toggles, logout and shutdown confirms), never by this effect.

### 7.3 Menu control functions

```
closeMenu():          isTabActive = false; menuOpen = false
toggleMenuPanel():    if isSubPanelOpen: return
                      isTabActive = !isTabActive; if (new value false) menuOpen = false
executeMenuSelection(i):
  0 → activeOverlay = "backlog"
  1 → activeOverlay = "config"
  2 → activeOverlay = "status"
  3 → toggleScreenMode()                                   // see §7.7
  4 → activeOverlay = "changeLog"
  5 → confirmMessage = t("logout_confirm", lang===1 ? "Do you want to log out?" : "ログアウトしますか？", lang);
      activeOverlay = "confirmLogout"
  6 → activeOverlay = "help"
  7 → confirmMessage = t("exit_confirm", lang===1 ? "Do you want to exit Real Amadeus?" : "リアルアマデウスを終了しますか？", lang);
      activeOverlay = "confirmShutdown"
  8 → closeMenu()
  other → nothing
```
Opening an overlay does **not** close the menu. The overlay covers it (z-index 2000/3000 vs 99), and closing the overlay returns to the still-open menu with the same `menuSelectedIndex`.

### 7.4 Global keyboard handler (`window` `keydown`; re-registered on `[autoMode, executeMenuSelection, isSubPanelOpen, menuOpen, menuSelectedIndex]`)

```
on keydown(e):
  if e.key == "F11": e.preventDefault(); toggleScreenMode(); return            // any state, any target
  if e.key == "F3":  e.preventDefault(); setInt("Config_AutoMode", autoMode ? 0 : 1); save(); return   // any state
  if e.key == "Tab":
      if isSubPanelOpen: return                         // NO preventDefault → native focus traversal happens
      e.preventDefault(); isTabActive = !isTabActive; if (new value false) menuOpen = false; return   // works even in text inputs
  if menuOpen && !isSubPanelOpen:
      if isTextInputTarget(e.target): return
      up    = e.key == "ArrowUp"    || e.key.toLowerCase() == "w"
      down  = e.key == "ArrowDown"  || e.key.toLowerCase() == "s"
      left  = e.key == "ArrowLeft"  || e.key.toLowerCase() == "a"
      right = e.key == "ArrowRight" || e.key.toLowerCase() == "d"
      if e.key == "Backspace": e.preventDefault(); closeMenu(); return
      if e.key == "Enter":     e.preventDefault(); executeMenuSelection(menuSelectedIndex); return
      if up||down||left||right: e.preventDefault(); menuSelectedIndex = MENU_NAV[menuSelectedIndex][dir]
```
Modifiers and `e.repeat` are not checked, so Ctrl+S with the menu open counts as "down" and the browser's save dialog is suppressed. During the 450 ms opening delay `menuOpen` is still false, so menu keys are inactive (and ChatPanel ignores keys because `menuPanelOpen = isTabActive`).

**Menu navigation table** (`MENU_NAV`; "=" means stay):

| from \ key | Up | Down | Left | Right |
|---|---|---|---|---|
| 0 BACKLOG | 6 | 1 | = | 8 |
| 1 CONFIG | 0 | 2 | = | 8 |
| 2 STATUS | 1 | 4 | 3 | 8 |
| 3 FULLSCREEN/WINDOW | 7 | 5 | 8 | 2 |
| 4 CHANGELOG | 2 | 6 | 5 | 8 |
| 5 LOGOUT | 3 | 7 | 8 | 4 |
| 6 HELP | 4 | 0 | 7 | 8 |
| 7 SHUTDOWN | 5 | 3 | 8 | 6 |
| 8 CLOSE MENU | = | = | 6 | 7 |
| any other index | = | = | = | = |

Mouse: clicking any menu entry runs `menuSelectedIndex = i; executeMenuSelection(i)`. Hovering does **not** change the selection.

**Keyboard map of the whole app** (for parity tests; all listeners sit on `window` and none stops propagation):

| Key | Root | ConfigPanel (visible) | Other overlays (other specs) | ChatPanel (other spec) |
|---|---|---|---|---|
| F11 | toggle screen mode | — | — | — |
| F3 | toggle AutoMode | — | — | — |
| Tab | toggle menu unless an overlay is open | — | — | — |
| Backspace | menu open & no overlay → close menu | apply + close | BackLog/Status (unless achievements open)/ChangeLog/Help → close; Confirmation → cancel; Achievement → close | — |
| Escape | **nothing** (the browser exits native fullscreen, then `fullscreenchange` syncs the setting) | — | Achievement (and MiniGame) → close | — |
| Enter | menu open & no overlay → execute selection | — | Confirmation → confirm if "Yes" selected, else cancel | ignored while `menuPanelOpen`; otherwise submit / advance |
| Arrows/WASD | menu navigation | Up/W, Down/S → category | Confirmation: Left/A = No, Right/D = Yes | — |
| Ctrl+C | — | — | — | cancel streaming (unless menu open) |
| Right-click | toggle menu (desktop, if `Config_RightClickMenu==1` and no overlay); native menu always suppressed | — | — | — |

Help text shown to users (HelpPanel, for reference): "Tab / 右クリック → メニュー開閉", "Backspace → 保存して閉じる", "WASD / ↑←↓→ → 項目選択", "Enter → 決定 / 会話を進める", "Ctrl+C → 会話をキャンセル".

### 7.5 Screen state machine (`appState`)

```
          LoginPanel.onAccepted / MobileLoginPanel.onAccepted
 "login" ─────────────────────────────────────────────────────► "boot"
    ▲                                                              │ LoadingPanel.onFinished / MobileLoadingPanel.onFinished
    │ ConfirmLogout.onConfirm                                      ▼  (desktop: setTimeout 0, mobile: synchronous, if Config_SkipLoading==1;
                                                                       mobile otherwise finishes after 1500 ms)
    └──────────────────────────────────────────────────────────  "chat"
```
- `login` renders LoginPanel (desktop, inside `.system-panel`) or MobileLoginPanel (in `.mobile-stage`). Desktop credentials (LoginPanel, other spec): accepted when both fields are non-empty after trim **and** (`userId.trim()==="Salieri"` **or** `password.trim()==="MakiseKurisu"`); on success it calls `MemoryManager.setUserName("Salieri")`.
- `boot` renders LoadingPanel / MobileLoadingPanel. V1 passes an **inline arrow** as `onFinished`, and both panels' effects depend on it, so **any root re-render during boot (window resize, Tab, settings change) restarts the boot animation** (desktop) or the 1500 ms timer (mobile) (QUIRK-6).
- `chat` renders ChatPanel (+ "Auto" label on desktop, top bar on mobile).
- Logout confirm: `MemoryManager.setUserName("")` (no-op); `appState = "login"`; `activeOverlay = "none"`; `isTabActive = false`; `menuOpen = false`. The backlog, `llmStatus` and `menuSelectedIndex` are kept. ChatPanel unmounts and the bridge becomes null.
- Shutdown confirm (desktop only):
  ```
  activeOverlay = "none"; menuOpen = false            // isTabActive NOT reset (system panel stays zoomed out)
  window.close()                                      // only closes script-opened windows
  setTimeout(100):  if window.history.length > 1: window.history.back()
                    setTimeout(300): window.location.href = "https://dsuv.elvelt.com/amadeus"
  ```
- The menu (Tab / right-click) works in **every** state, including `login` and `boot`. The "MENU" button exists only in `chat`. In `login`/`boot`, Export reports "no data" and Import updates the backlog but not the conversation (bridge null).
- When the layout switches (mobile ↔ desktop across the breakpoint), React mounts a different tree: ChatPanel remounts (conversation reset to a fresh system prompt), while root state (appState, backlog, overlay) is kept. On mobile, `activeOverlay` values without a mobile renderer (`config`, `status`, `help`, …) leave nothing visible but still block Tab.

### 7.6 Overlay state machine (`activeOverlay`, `achievementsOpen`, `mobileSettingsOpen`)

| Overlay | Opened by | Closed by | Rendered in |
|---|---|---|---|
| `backlog` | menu 0 (desktop); top bar "LOG" (mobile) | BackLogPanel onClose (close button / Backspace) → "none" | both |
| `config` | menu 1 | Cancel / Apply / Backspace → "none" | desktop only |
| `status` | menu 2 | StatusPanel onClose → "none" | desktop only |
| `changeLog` | menu 4 | onClose → "none" | desktop only |
| `confirmLogout` | menu 5 | No / Backspace / Enter on "No" → "none"; Yes → logout sequence | desktop only |
| `help` | menu 6 | onClose → "none" | desktop only |
| `confirmShutdown` | menu 7 | No → "none"; Yes → shutdown sequence | desktop only |
| `achievements` | mobile ChatPanel "ACHIEVEMENTS" button (via `setActiveOverlay`; desktop ChatPanel gets no setter) | mobile AchievementPanel onClose → `achievementsOpen=false; activeOverlay="none"` | mobile |
| `minigame` | never set by the root | — | — |
| `achievementsOpen` (bool) | StatusPanel `onOpenAchievements` (desktop) | AchievementPanel onClose → false | desktop: visible=`achievementsOpen`; mobile: visible=`achievementsOpen \|\| activeOverlay==="achievements"` |
| `mobileSettingsOpen` (bool) | top bar "SETTINGS" button | MobileSettingsPanel onClose (Close or Apply) | mobile (conditionally **mounted**) |

While any `activeOverlay !== "none"`: Tab and the menu keys are ignored, right-click does not toggle (native menu still suppressed), and F11/F3 still work. On desktop, ChatPanel is paused because `isTabActive` stays true (overlays are reached through the menu).

### 7.7 Fullscreen handling (four writers plus one sync)

`toggleScreenMode()` (menu item 3 and F11 share this code):
```
current = getInt("Config_ScreenMode", 1); next = current === 0 ? 1 : 0
setInt("Config_ScreenMode", next); save()
if next === 0: if (!document.fullscreenElement) document.documentElement.requestFullscreen().catch(() => {})
else:          if ( document.fullscreenElement) document.exitFullscreen().catch(() => {})
```
- ConfigPanel Apply runs the same request/exit logic using the panel's `screenMode`.
- `fullscreenchange` writes 0 or 1 from `document.fullscreenElement`. That catches Escape, browser UI and F11 (if the browser handled it natively).
- Unprefixed API only. Where `requestFullscreen` is undefined (iPhone Safari, old iPadOS) the call throws synchronously, and in Apply that **aborts the rest of `handleApply`**. ConfigPanel is desktop-layout only, but large coarse-pointer tablets wider than 1200px do get the desktop layout.
- Startup: a persisted `Config_ScreenMode=0` gives the `.window-frame.fullscreen` class (no border) without actually being fullscreen. Fullscreen is not requested at load because a user gesture is required. The first F11 or menu toggle then only flips the setting to 1 (QUIRK-8).
- The UI: `.window-frame` class is `fullscreen` when `screenMode === 0`, else `windowed` (adds `border:1px solid #666`). Menu item 3's label shows the **current** mode: `screenMode === 0 ? "FULL\nSCREEN" : "WINDOW"`.

### 7.8 Window scaling / "resolution"

The stage is a fixed 1920×1080 design canvas (`.viewport`, `transform-origin: top left`), scaled by `scale = min(innerWidth/1920, innerHeight/1080)` inside `.window-frame` (size `1920*scale × 1080*scale` px). It is centred by `.real-amadeus-root` (`display:flex; align-items:center; justify-content:center`). **`Config_Resolution` has no effect** in V1. Inline style strings use JS number formatting, e.g. `width: 1365.3333333333333px`, `transform: scale(0.7111111111111111)`.

`.system-panel` inline transform: `translate(${150*p}px, ${-12*p}px) scale(${1 + (0.6 - 1)*p})` with `p ∈ {0,1}` → exactly `"translate(0px, 0px) scale(1)"` (JS prints `-0` as `0`; **Rust must not print `-0`**) and `"translate(150px, -12px) scale(0.6)"`. CSS animates it with `transition: transform 500ms linear`.

`.menu-panel` inline transform: `scaleX(${dimensions.width / (1920*uiScale)}) scaleY(${dimensions.height / (1080*uiScale)})`. That is always `scaleX(1) scaleY(1)`, or `NaN` (an invalid transform, ignored) when the window is 0×0. Compute it the same way or emit the constant.

### 7.9 Desktop DOM (exact)

```
div.real-amadeus-root   onContextMenu: e.preventDefault(); if getInt("Config_RightClickMenu",1)===1 → toggleMenuPanel()
├─ (appState==="chat") button.mobile-menu-toggle[type=button]  onClick=toggleMenuPanel
│     text: isTabActive ? t("close", lang===1 ? "CLOSE" : "閉じる", lang) : "MENU"
│     (CSS: display:none unless @media (pointer: coarse), (max-width: 900px))
└─ div.window-frame.{fullscreen|windowed}  style="width:{outerWidth}; height:{outerHeight}"
   └─ div.viewport  style="transform: scale({uiScale})"
      ├─ BackLogPanel  visible={activeOverlay==="backlog"} entries={backLogEntries} configLanguage onClose→activeOverlay="none"
      ├─ div.system-panel  style="transform: {see §7.8}"
      │   ├─ img.menu-wallpaper  src="/resources/images/RealAmadeus_Menu_BG_v3.jpg" alt=""
      │   ├─ div.title-bar > span "Amadeus System"
      │   ├─ img.system-bg  src="/resources/images/Amadeus_BG.png" alt=""
      │   ├─ (login) LoginPanel onAccepted→appState="boot"
      │   ├─ (boot)  LoadingPanel onFinished→appState="chat"
      │   └─ (chat)  ChatPanel autoMode autoSpeed configLanguage menuPanelOpen={isTabActive} onBackLogAdd={addBackLog}
      │              onLlmStatusUpdate={setLlmStatus} activeOverlay conversationRef   (NO setActiveOverlay on desktop)
      │              + (autoMode) div.auto-text "Auto"
      ├─ div.menu-panel  style="opacity:{menuOpen?1:0}; pointer-events:{menuOpen?'auto':'none'};
      │                         transition:opacity 0.3s ease-in-out; transform:{§7.8}; transform-origin:top left"
      │   ├─ img.menu-bg  src="/resources/images/Menu.png" alt=""
      │   └─ div.menu-content
      │       ├─ MenuItem Backlog   ├─ MenuItem Config ├─ MenuItem Status ├─ MenuItem ChangeLog ├─ MenuItem Help
      │       └─ SmallMenuLabel (3) ├─ (5) ├─ (7) ├─ (8)            (order exactly as listed)
      ├─ ConfigPanel visible={activeOverlay==="config"} onLanguageChange→setInt("Config_Language",v)
      │              onExportConversation onImportConversation onClose→"none"
      ├─ StatusPanel visible={activeOverlay==="status"} llmStatus configLanguage isLoggedIn={appState==="chat"}
      │              achievementsOpen onOpenAchievements→achievementsOpen=true onClose→"none"
      ├─ AchievementPanel visible={achievementsOpen} configLanguage onClose→achievementsOpen=false
      ├─ ChangeLogPanel visible={activeOverlay==="changeLog"} configLanguage onClose→"none"
      ├─ HelpPanel visible={activeOverlay==="help"} configLanguage onClose→"none"
      ├─ ConfirmationDialog visible={activeOverlay==="confirmLogout"} message={confirmMessage} configLanguage
      │              onCancel→"none"  onConfirm→logout sequence (§7.5)
      └─ ConfirmationDialog visible={activeOverlay==="confirmShutdown"} message={confirmMessage} configLanguage
                     onCancel→"none"  onConfirm→shutdown sequence (§7.5)
```
All overlay components are always mounted and hidden by opacity. Wrapper z-indexes (inline): BackLog 3000, Config 2000, Status 2000, Achievement 2500, ChangeLog 2000, Help 2000, Confirmation 3000; `.menu-panel` 99 (CSS). CSS mount animations (`.overlay-panel` 250 ms fade, `.menu-bg` 250 ms, `.menu-content` 300 ms) run once at app start because the components stay mounted.

**MenuItem** `{icon, label, selected, onClick, iconPos, labelPos}` renders two sibling buttons (fragment):
```
button.menu-icon{ selected ? " selected" : " " }[type=button] style="left:{iconPos.x-45}px; top:{iconPos.y-45}px" onClick
  img.menu-icon-normal   src="/resources/images/{icon}.png"          alt=""
  img.menu-icon-selected src="/resources/images/{icon}Selected.png"  alt=""
button.menu-label{ selected ? " selected" : " " }[type=button] style="left:{labelPos.x-100}px; top:{labelPos.y-25}px" onClick
  {label}     // contains "\n"; CSS white-space: pre-line renders the line break
```
**SmallMenuLabel** `{selected, label, pos, onClick}`:
```
button.small-menu-label{ selected ? " selected" : " " }[type=button] style="left:{pos.x-100}px; top:{pos.y-25}px" onClick  {label}
```
**Menu layout table** (sizes from CSS: icon 90×90, labels 200×50):

| idx | Component | icon | label (verbatim) | iconPos → left/top | labelPos/pos → left/top |
|---|---|---|---|---|---|
| 0 | MenuItem | `Backlog` | `"BACK\nLOG"` | (270,146) → 225/101 | (441,147) → 341/122 |
| 1 | MenuItem | `Config` | `"CONFIG"` | (270,320) → 225/275 | (441,321) → 341/296 |
| 2 | MenuItem | `Status` | `"STATUS"` | (270,494) → 225/449 | (441,495) → 341/470 |
| 4 | MenuItem | `ChangeLog` | `"CHANGE\nLOG"` | (270,674) → 225/629 | (441,669) → 341/644 |
| 6 | MenuItem | `Help` | `"HELP"` | (270,848) → 225/803 | (441,843) → 341/818 |
| 3 | SmallMenuLabel | — | `screenMode===0 ? "FULL\nSCREEN" : "WINDOW"` | — | (135,584) → 35/559 |
| 5 | SmallMenuLabel | — | `"LOGOUT"` | — | (135,758) → 35/733 |
| 7 | SmallMenuLabel | — | `"SHUT\nDOWN"` | — | (135,950) → 35/925 |
| 8 | SmallMenuLabel | — | `"CLOSE\nMENU"` | — | (405,950) → 305/925 |

### 7.10 Mobile DOM (exact)

```
div.real-amadeus-root.mobile-root   onContextMenu: e.preventDefault()   (no menu toggle; Config_RightClickMenu ignored)
├─ (appState==="chat") div.mobile-topbar
│   ├─ div.mobile-topbar-buttons
│   │   ├─ button.mobile-topbar-button[type=button] {t("settings", lang===1 ? "SETTINGS" : "設定", lang)}  onClick→mobileSettingsOpen=true
│   │   └─ button.mobile-topbar-button[type=button] {t("backlog",  lang===1 ? "LOG" : "ログ", lang)}       onClick→activeOverlay="backlog"
│   └─ MarqueeStatus provider={llmStatus.provider} model={llmStatus.model}
├─ div.mobile-stage
│   ├─ (login) MobileLoginPanel configLanguage onAccepted→appState="boot"
│   ├─ (boot)  MobileLoadingPanel configLanguage onFinished→appState="chat"
│   └─ (chat)  ChatPanel autoMode autoSpeed configLanguage menuPanelOpen={mobileSettingsOpen} onBackLogAdd={addBackLog}
│              onLlmStatusUpdate={setLlmStatus} activeOverlay setActiveOverlay conversationRef
├─ (mobileSettingsOpen) MobileSettingsPanel onLanguageChange→setInt("Config_Language",v)
│              onSettingsSaved→llmStatus=getLlmStatusFromSettings()  onClose→mobileSettingsOpen=false
├─ BackLogPanel visible={activeOverlay==="backlog"} entries configLanguage onClose→"none"
└─ AchievementPanel visible={achievementsOpen || activeOverlay==="achievements"} configLanguage
               onClose→ achievementsOpen=false; activeOverlay="none"
```
The `settings` and `backlog` keys exist in **no** locale, so the labels are `"SETTINGS"`/`"LOG"` for en and `"設定"`/`"ログ"` for every other language (QUIRK-12). The mobile layout has no menu panel, ConfigPanel, Status, Help, ChangeLog, confirmations or "Auto" label. The root keyboard handler still runs, though: with a hardware keyboard, Tab sets `isTabActive` and after 450 ms an **invisible** `menuOpen`, and then Enter or arrows can execute menu actions (e.g. backlog) — QUIRK-18.

### 7.11 Right-click

Desktop: the native context menu is suppressed **everywhere**, including text inputs (no paste-by-menu). The menu toggles only if `Config_RightClickMenu` (read live) is 1 and no overlay is open. Mobile: native menu suppressed (long-press), nothing else.

### 7.12 Every network request / resource the root and ConfigPanel cause

| # | When | Method | URL | Headers / options | Body | Response use |
|---|---|---|---|---|---|---|
| 1 | root mount, unless `document.fonts.check('16px "MS Mincho"')` | GET | `https://raw.githubusercontent.com/vslabs/winfonts/master/msmincho.ttc` | AbortSignal (8000 ms) | — | arrayBuffer → `FontFace("MS Mincho")` → `document.fonts.add` |
| 2 | root mount | script | `/resources/live2d/live2dcubismcore.min.js`, `/resources/live2d/reama-cubism-runtime.js?v=19` | async script tags | — | then `ReamaCubismRuntime.preload()` (model fetches: Live2D spec) |
| 3 | root mount | POST | `/amadeus/app/api/tts/health` | `Content-Type: application/json` | `{"serverUrl":"<TTS url>"}` | `{available, runtime?, torch?, error?}` |
| 4 | root mount | POST | `/amadeus/app/api/stt/health` | same | `{"serverUrl":"<STT url>"}` | `{available, error?}` |
| 5 | root mount | POST | `/amadeus/app/api/rag/health` | same | `{"serverUrl":"<RAG url>"}` | `{available, error?}` |
| 6 | each ConfigPanel open | POST | `/amadeus/app/api/tts/health` | same | same as #3 | same |
| 7 | ConfigPanel open with p=8, or switching to p=8 | GET | `/amadeus/app/api/chat/quota` | `cache: "no-store"` | — | `{count,max,remaining}` (numbers), else hidden |
| 8 | shutdown confirm | navigation | `https://dsuv.elvelt.com/amadeus` (after ~400 ms, if still on the page) | — | — | — |
| 9 | export | blob download | `blob:` URL, filename `chat_export_<ts>.json`, MIME `application/json` | — | — | — |
| 10 | import | local file | `<input type=file accept=".json">` | — | — | text → parse |

Service singletons may also fire their own health checks at module construction when enabled (other specs). Images referenced by this range: `/resources/images/Amadeus_BG.png`, `DownAllow.png`, `RealAmadeus_Menu_BG_v3.jpg`, `Menu.png`, and `Backlog`/`Config`/`Status`/`ChangeLog`/`Help` + `Selected` variants (`.png`). All 17 files in `public/resources/images/` must be served unchanged.

### 7.13 Timing summary

| Value | Where |
|---|---|
| 450 ms | `isTabActive` → `menuOpen` delay |
| 500 ms linear | `.system-panel` transform transition (CSS) |
| 0.3 s ease-in-out | opacity transition of the menu and every overlay wrapper (inline) |
| 100 ms | MarqueeStatus second overflow check |
| 8000 ms | font fetch abort |
| 100 ms, then +300 ms | shutdown: `history.back()`, then redirect |
| 0 ms | ConfirmationDialog resets its selection to "Yes" when shown; LoadingPanel skip (MobileLoadingPanel skip is synchronous; its normal duration is 1500 ms with a 300 ms dot interval) |
| immediate | `URL.revokeObjectURL` right after `a.click()` |

### 7.14 Listener interplay (for implementers)

V1 registers independent `window` `keydown` listeners: root (always), ConfigPanel and other overlays (while visible), ChatPanel (while mounted, ignores keys when `menuPanelOpen || miniGameOpen`), ChatPanel idle-reset (any key), and LoginPanel input `onKeyDown`. None stop propagation. Correctness depends on the guard conditions alone (`isSubPanelOpen`, `menuOpen`, `visible`), not on listener order. Reproduce the same guards; a single dispatcher is fine as long as every rule in §7.4 holds.

A known focus hazard to keep in mind: buttons of hidden overlays can keep keyboard focus (`pointer-events:none` does not stop keyboard activation), so Space can activate an invisible focused button such as Apply or Cancel (QUIRK-21).

---

## 8. Translation keys used by this range

Embed the locale JSON files verbatim. This table gives ja/en values and the locales that are **missing** the key (those fall back to the inline default: the en default for lang 1, otherwise ja).

| key | ja | en | missing in |
|---|---|---|---|
| `category_system` | "システム" | "System" | - |
| `category_text` | "テキスト" | "Text" | - |
| `category_sound` | "サウンド" | "Sound" | - |
| `category_graphics` | "グラフィック" | "Graphics" | - |
| `category_api` | "API" | "API" | - |
| `setting_display_language` | "表示言語" | "Display Language" | - |
| `setting_chat_language` | "会話言語" | "Conversation Language" | - |
| `setting_chat_lang_same_as_ui` | "表示言語と同じ" | "Same as Display Language" | - |
| `setting_skip_loading` | "起動画面スキップ" | "Skip Loading Screen" | - |
| `setting_right_click` | "右クリックメニュー" | "Right Click Menu" | - |
| `setting_eye_tracking` | "視線トラッキング" | "Eye Tracking" | - |
| `setting_show_notifications` | "通知を表示" | "Show Notifications" | - |
| `setting_lightweight` | "軽量化モード" | "Lightweight Mode" | - |
| `setting_text_speed` | "文字表示速度" | "Text Speed" | - |
| `setting_auto_mode` | "オート表示" | "Auto Mode" | - |
| `setting_auto_speed` | "オート待機時間" | "Auto Wait Time" | - |
| `setting_export_chat` | "会話記録エクスポート" | "Export Chat Log" | - |
| `export` | "書き出す" | "Export" | - |
| `setting_import_chat` | "会話記録インポート" | "Import Chat Log" | - |
| `import` | "読み込む" | "Import" | - |
| `setting_master_vol` | "マスター音量" | "Master Volume" | - |
| `setting_bgm_vol` | "BGM音量" | "BGM Volume" | - |
| `setting_se_vol` | "SE音量" | "SE Volume" | - |
| `setting_voice_vol` | "ボイス音量" | "Voice Volume" | - |
| `setting_screen_mode` | "画面モード" | "Screen Mode" | - |
| `screen_mode_fullscreen` | "フルスクリーン" | "Fullscreen" | - |
| `screen_mode_windowed` | "ウィンドウ" | "Windowed" | - |
| `setting_resolution` | "解像度" | "Resolution" | - |
| `setting_api_provider` | "LLM APIプロバイダ" | "LLM API Provider" | - |
| `setting_api_key` | "APIキー" | "API Key" | - |
| `setting_model_name` | "LLM モデル名" | "LLM Model Name" | - |
| `setting_web_search` | "LLM Web検索" | "LLM Web Search" | - |
| `setting_openai_compatible` | "互換APIを使う" | "Use Compatible API" | - |
| `setting_openai_base_url` | "ベースURL" | "Base URL" | - |
| `setting_openai_base_url_example` | "ベースURLの例: https://api.example.com/v1" | "Base URL Example: https://api.example.com/v1" | - |
| `setting_vertex_note` | "※ Vertexを使用するためには、gCloud CLIのインストールが必要です。" | "* gCloud CLI is required to use Vertex." | - |
| `cancel` | "キャンセル" | "Cancel" | - |
| `apply` | "適用" | "Apply" | - |
| `freeapi_quota_usage` | "%1 / %2 回使用" | "%1 / %2 requests used" | zh,ko,es,fr,de,ru,uk,pt,tr,he,ar |
| `freeapi_quota_remaining` | "残り %1 回" | "%1 remaining" | zh,ko,es,fr,de,ru,uk,pt,tr,he,ar |
| `freeapi_quota_label` | "FreeAPIクォータ" | "FreeAPI Quota" | zh,ko,es,fr,de,ru,uk,pt,tr,he,ar |
| `freeapi_quota_exhausted` | "全体リクエスト上限（65,000回）に達しました。サービスを停止しています。" | "Global request limit (65,000) reached. Service suspended." | zh,ko,es,fr,de,ru,uk,pt,tr,he,ar |
| `export_no_data` | "会話データがありません" | "No conversation data available" | - |
| `export_success` | "エクスポート完了: " | "Export complete: " | - |
| `export_failed` | "エクスポート失敗: " | "Export failed: " | - |
| `import_invalid` | "無効なファイル形式です" | "Invalid file format" | - |
| `import_success` | "インポート完了: " | "Import complete: " | - |
| `import_turns` | " ターン" | " turns" | - |
| `import_failed` | "インポート失敗: " | "Import failed: " | - |
| `logout_confirm` | "ログアウトしますか？" | "Do you want to log out?" | - |
| `exit_confirm` | "リアルアマデウスを終了しますか？" | "Do you want to exit Real Amadeus?" | - |
| `settings` | — | — | **all 13** (defaults "SETTINGS"/"設定") |
| `backlog` | — | — | **all 13** (defaults "LOG"/"ログ") |
| `close` | "閉じる" | "Close" | - (zh "关闭", ko "닫기", es "Cerrar", fr "Fermer", de "Schließen", ru "Закрыть", uk "Закрити", pt "Fechar", tr "Kapat", he "סגור", ar "إغلاق") |

Where the JSON value differs from the inline English default (the JSON wins): `setting_export_chat` ("Export Chat Log" vs "Export Conversation"), `setting_import_chat`, `setting_openai_compatible` ("Use Compatible API" vs "OpenAI Compatible"), `setting_vertex_note` ("is required" vs "must be installed"), `close` (desktop MENU toggle shows "Close", not "CLOSE").

Keyless or literal strings: `"CONFIG"`, `"MENU"`, `"Amadeus System"`, `"Auto"`, `"---"`, `"Vertex Project ID"`, `"Vertex Location"`, `"Ollama Host"`, the menu labels, `"1920x1080"`, `"1600x900"`, `"1280x720"`, `LANGUAGE_NAMES`, `PROVIDER_NAMES`, `"OpenAI互換(" + baseUrl + ")"`, the Vertex key label pair and the Ollama note pair (§3.10).

---

## 9. CSS reference (`app/amadeus/app/globals.css`, 2966 lines, reused verbatim)

| Lines | Selectors | Behaviour relevant to the port |
|---|---|---|
| 1–65 | Google Fonts import (Noto Serif JP 300/400/700), `@font-face` MateSC, Eurostile, DepartureMono, Liberation Sans, MS Mincho (`local()` + Noto CJK sc), Yu Mincho, Noto Serif JP (CJK range + light), Noto Serif, Noto Serif CJK KR | fonts under `/resources/fonts/` |
| 67–95 | `*` reset, `user-select:none` (inputs/textarea re-enabled), focus outlines removed, `.config-input-control:focus` border forced to #333 (`!important`, which overrides the #ff9900 rule at 1814) | |
| 97–158 | `.amadeus-root`, `.real-amadeus-root` (flex centre, safe-area padding), `.window-frame` (+`.windowed` border), `.viewport` (1920×1080, top-left origin) | |
| 160–202 | `.system-panel` (transition 500 ms), `.menu-wallpaper`, `.title-bar`, `.system-bg` | |
| 535–546 | `.auto-text` | |
| 548–669 | `.menu-panel` (z 99), `.menu-bg`, `.menu-content`, `.closing` variants (unused by V1), keyframes, `.menu-icon` (+`.selected` swaps icon), `.menu-label`, `.small-menu-label` | |
| 671–722 | `.mobile-menu-toggle` hidden, shown under `@media (pointer: coarse), (max-width: 900px)` | |
| 726–822 | `.overlay-panel` (z 110, fade 250 ms), `.overlay-bg`, `.line`, `.line.thin`, `.overlay-close-button` | |
| 1592–1657 | `.dialog-overlay`, `.confirmation-dialog` | |
| 1659–2049 | all `.config-*`, `.custom-select-*`, `.slider-*`, checkbox styling, `.config-note`, `.config-freeapi-quota`, `.config-footer` | the slider track fill uses `var(--fill)` |
| 2051–2120, 2445–2457 | `.mobile-root`, `.mobile-topbar*`, marquee keyframes (12 s), `.mobile-stage`, `.mobile-root .auto-text` (unused) | |

There are no mobile overrides for the config panel or menu.

---

## 10. Quirk catalogue (V1 behaviour → recommendation)

1. **Index-4 filter applies to every ConfigSelectRow.** With `Config_VertexEnabled != 1` (the default; the key is not even in the defaults), "Español" cannot be picked as Display Language and "한국어" cannot be picked as Conversation Language. **Recommend fixing**: give the select a `hidden_indices` prop and pass `[4]` (when Vertex is disabled) only to the provider select, keeping the empty-string skip. Ask the product owner (see Open questions).
2. **Logout calls `MemoryManager.setUserName("")`, which is a no-op.** Replicate (it is harmless), or decide explicitly.
3. **Import replaces the whole conversation history, including the system prompt.** Exported files never contain system messages, so after importing one the persona prompt is gone and later requests carry no system message (`updateSystemPrompt` only rewrites index 0 when it is already `system`; the backend does not add one). **Recommend fixing** in the ChatPanel bridge (re-insert the current system prompt at index 0 when missing), and coordinate with the ChatPanel spec owner.
4. Import with no bridge (login/boot) clears and refills the backlog, unlocks the achievement and reports success, but the conversation is unchanged. Replicate.
5. The import success message counts messages, not turns ("12 ターン" for 12 messages). Replicate.
6. **The boot sequence restarts on any root re-render** (inline `onFinished`). **Do not replicate**: Leptos callbacks are stable. Document this as a deliberate deviation.
7. In the desktop branch, the shutdown confirm leaves `isTabActive` true. Replicate (the page navigates away anyway).
8. A persisted ScreenMode 0 at startup is not actually fullscreen, so the first toggle only flips the flag. Replicate on Web. On desktop (Tauri) you could apply the persisted mode at startup without a user gesture (open question).
9. **`Config_Resolution` is stored but unused.** Web: replicate the no-op. Desktop: option to resize the window (open question).
10. Master, BGM and SE volume, Notifications and Lightweight mode are stored but have no consumer. Replicate (store only).
11. F3 or F11 pressed while ConfigPanel is open is overwritten by a later Apply with the panel's stale values. Replicate.
12. Inconsistent fallback languages: keyless `tr` gives English for languages 2–12, while a keyed call with a missing key gives Japanese. The mobile "SETTINGS/LOG" labels are Japanese for languages 2–12. Replicate.
13. ConfigPanel shortcuts are disabled while focus is on a checkbox or slider (`isTextInputTarget` matches any INPUT). Replicate.
14. Voice volume only takes effect after reload. Replicate, or improve in the audio spec.
15. Desktop Apply does not refresh `llmStatus` (StatusPanel shows the old provider until the next chat response; mobile refreshes). **Recommend fixing** (call `llm_status_from_settings(-1)` after Apply). That is low risk and the only visible effect is that the status is correct.
16. The status default model names differ from the backend defaults. Replicate the display table verbatim.
17. The marquee class never turns off once applied. Replicate (cheap), or fix by measuring the inner text without padding.
18. On the mobile layout with a hardware keyboard, Tab arms an invisible menu, and then Enter or arrows can trigger overlays. Replicate, or guard the menu branch with `!is_mobile` (open question).
19. Re-selecting the current provider discards unsaved key/model edits. Replicate.
20. Hidden RAG settings are normalised and clamped on every Apply (TopK 0 → 5, threshold 0 → 0.7, clamped to [1,20] and [0,1]). TTS speaker and STT provider are normalised to known names. Replicate exactly; this is persisted-data parity.
21. Hidden overlays keep keyboard focus, so Space can activate an invisible button. Recommend blurring `document.activeElement` when an overlay closes (deviation; open question).
22. The Tab key while an overlay is open is not `preventDefault`ed, so native focus traversal happens. Replicate.
23. Right-click suppresses the native context menu everywhere (no paste via menu). Replicate.
24. `toLocaleString` uses the browser locale, not the app language. Replicate via `Intl`.

---

## Rust mapping

### amadeus-core (no I/O, wasm32 + native)

```rust
// src/jsnum.rs — ECMAScript-compatible primitives, shared by every subsystem
pub fn js_parse_int(s: &str) -> Option<f64>;          // radix 10; None == NaN
pub fn js_parse_float(s: &str) -> Option<f64>;        // None == NaN; Some(±inf) possible for "Infinity"
pub fn js_number_to_string(v: f64) -> String;         // Number::toString (use the `ryu-js` crate); -0 → "0", NaN → "NaN"
pub fn js_to_fixed(v: f64, digits: u32) -> String;    // exact-value rounding, ties → larger
pub fn js_math_round(v: f64) -> f64;                  // JS Math.round: ties toward +inf; NOT naive (v+0.5).floor() (wrong for 0.49999999999999994); -0 handling
pub fn js_trim(s: &str) -> &str;                      // WhiteSpace (TAB VT FF SP NBSP ZWNBSP(U+FEFF) Zs) + LineTerminator (LF CR LS PS)

// src/settings/mod.rs
pub trait KvStore { fn get(&self, key: &str) -> Option<String>; fn set(&self, key: &str, value: &str); }
pub struct AppSettings<S: KvStore> { /* store + change listeners: Vec<Box<dyn Fn(&str)>> */ }
impl<S: KvStore> AppSettings<S> {
    pub fn initialize_defaults(&self);                // writes missing keys, NO change emission
    pub fn get_int(&self, key: &str, default: i64) -> i64;    // js_parse_int, saturating cast
    pub fn get_float(&self, key: &str, default: f64) -> f64;  // js_parse_float, non-finite → default
    pub fn get_string(&self, key: &str, default: &str) -> String;
    pub fn set_int(&self, key: &str, v: f64);         // String(trunc(v)) then emit(key)
    pub fn set_float(&self, key: &str, v: f64);       // js_number_to_string(v) then emit(key)
    pub fn set_string(&self, key: &str, v: &str);     // emit(key)
    pub fn subscribe(&self, f: impl Fn(&str) + 'static) -> ListenerId;
}
pub mod keys { pub const LANGUAGE: &str = "Config_Language"; /* … every key in §2 … */
               pub fn api_key(i: usize) -> String { format!("Config_ApiKey_{i}") }
               pub fn model_name(i: usize) -> String { format!("Config_ModelName_{i}") }
               pub const VERTEX_ENABLED: &str = "Config_VertexEnabled"; }
pub const DEFAULT_SETTINGS: &[(&str, DefaultValue)];  // §1.3, same insertion order

// src/providers.rs
pub const PROVIDER_NAMES: [&str; 9]; pub const FREEAPI_PROVIDER_INDEX: i64 = 8;
pub const TTS_SPEAKER_NAMES: [&str; 2]; pub const STT_PROVIDER_NAMES: [&str; 3];
pub fn default_model_for_provider(i: i64) -> &'static str;
pub fn provider_display_name(i: i64, is_compatible: bool, base_url: &str) -> String;
pub fn llm_status_from_settings<S: KvStore>(s: &AppSettings<S>, latency_ms: i64) -> LlmStatus;
#[derive(Clone, PartialEq, Serialize, Deserialize)] pub struct LlmStatus { pub provider: String, pub model: String, pub latency_ms: i64 }

// src/config_form.rs — all ConfigPanel logic, natively unit-testable
#[derive(Clone, PartialEq)] pub struct ConfigFormState { /* the fields of §3.2, same names in snake_case;
    rag_top_k: String, rag_threshold: String, tts_speaker: usize, stt_provider: usize */ }
#[derive(Clone, Default)] pub struct ProviderBuffers { pub api_keys: BTreeMap<i64, String>, pub model_names: BTreeMap<i64, String> }
impl ConfigFormState { pub fn initial() -> Self; pub fn load<S: KvStore>(s: &AppSettings<S>) -> (Self, ProviderBuffers); }
pub fn change_provider(state: &mut ConfigFormState, bufs: &mut ProviderBuffers, new_provider: i64); // §3.6 semantics
pub struct ApplyOutcome { pub screen_mode: i64, pub language: i64 }
pub fn apply<S: KvStore>(state: &ConfigFormState, bufs: &mut ProviderBuffers, s: &AppSettings<S>) -> ApplyOutcome;
      // performs EVERY write of §3.7 in order, including the RAG clamp rewrites; the UI then does fullscreen + language

// src/conversation_io.rs
pub fn export_file_name(now_iso: &str) -> String;     // "chat_export_" + first 19 chars of iso with ':' and '.' → '-' + ".json"
pub fn build_export_json(history: &[ChatMessage], turn_count: f64, exported_at_iso: &str) -> Option<String>; // None ⇒ "no data"
#[derive(Debug)] pub enum ImportError { Invalid, Parse(String) }
pub struct ImportedConversation { pub messages: Vec<ImportedMessage>, pub turn_count: f64 }
pub fn parse_import(raw: &str) -> Result<ImportedConversation, ImportError>;   // §4.2 validation order
pub fn to_chat_role(role: &str) -> ChatRole;
pub fn import_success_message(lang: i64, count: usize) -> String; // t(import_success)+count+t(import_turns)

// src/backlog.rs
#[derive(Clone, PartialEq)] pub struct BackLogEntry { pub role: String, pub name_color: String, pub message: String }
pub fn make_backlog_entry(role: &str, raw: &str) -> Option<BackLogEntry>;      // §4.3 (regex `(?i)\[(NORMAL|…|PANIC)\]`)
pub fn backlog_entries_for_import(msgs: &[ImportedMessage]) -> Vec<BackLogEntry>; // uses chat_utils::split_message

// src/menu.rs
#[repr(u8)] pub enum MenuItemId { Backlog=0, Config=1, Status=2, ScreenToggle=3, ChangeLog=4, Logout=5, Help=6, Shutdown=7, CloseMenu=8 }
pub enum Dir { Up, Down, Left, Right }
pub fn menu_navigate(index: u8, dir: Dir) -> u8;       // table §7.4
pub struct MenuLayoutEntry { pub id: MenuItemId, pub icon: Option<&'static str>, pub label: &'static str, pub icon_pos: Option<(i32,i32)>, pub label_pos: (i32,i32) }
pub const MENU_LAYOUT: [MenuLayoutEntry; 9];          // table §7.9 (index-3 label resolved at render)

// src/layout.rs
pub struct Dimensions { pub width: f64, pub height: f64, pub scale: f64 }
pub fn fit_dimensions(inner_w: f64, inner_h: f64) -> Dimensions;     // min(w/1920, h/1080)
pub fn system_panel_transform(tab_active: bool) -> &'static str;     // the two exact strings of §7.8
pub fn resolve_resolution(index: i64) -> (u32, u32);
pub fn is_mobile_viewport(visual_width: f64, coarse_pointer: bool) -> bool;   // <=900 || (coarse && <=1200)

// src/protocol.rs (shared DTOs)
#[derive(Serialize, Deserialize)] pub struct FreeApiQuota { pub count: f64, pub max: f64, pub remaining: f64 }
#[derive(Serialize, Deserialize)] pub struct HealthRequest { #[serde(rename = "serverUrl")] pub server_url: String }
```

### amadeus-ui (Leptos 0.8 CSR)

- `app.rs` — `#[component] pub fn RealAmadeusApp() -> impl IntoView`. Signals: `app_state: RwSignal<AppState>`, `is_tab_active`, `menu_open`, `menu_selected_index: RwSignal<u8>`, `active_overlay: RwSignal<OverlayPanel>`, `achievements_open`, `mobile_settings_open`, `dimensions: RwSignal<Dimensions>`, `llm_status: RwSignal<LlmStatus>`, `backlog: RwSignal<Vec<BackLogEntry>>`, `confirm_message: RwSignal<String>`, `bridge: StoredValue<Option<ConversationBridge>>` (`ConversationBridge { get_history: Rc<dyn Fn()->Vec<ChatMessage>>, get_turn_count: Rc<dyn Fn()->f64>, apply_imported: Rc<dyn Fn(Vec<ChatMessage>, f64)> }`). The 450 ms timer uses `set_timeout_with_handle`, cancelled on change (`Effect::new` + `on_cleanup`).
- `state.rs` — `enum AppState { Login, Boot, Chat }`, `enum OverlayPanel { None, Backlog, Config, Status, ChangeLog, Help, ConfirmLogout, ConfirmShutdown, Minigame, Achievements }`.
- `hooks/settings.rs` — a global `SettingsCtx { settings: Rc<AppSettings<LocalStorageKv>>, rev: RwSignal<u64> }`. Every `set_*` bumps `rev` (subscribe a listener that does `rev.update(|v| *v += 1)`). `use_app_setting_int(key, def) -> Memo<i64>` and `use_app_setting_float` track `rev` and read storage.
- `hooks/viewport.rs` — `use_is_mobile_viewport() -> Signal<bool>` (window resize + visualViewport resize + `matchMedia("(pointer: coarse)")` change).
- `components/config/mod.rs` `ConfigPanel` holds `RwSignal<ConfigFormState>`, `StoredValue<ProviderBuffers>`, `active_category`, `io_status`. Load on `visible` rising edge; the keyboard listener only while visible. Sub-files `select_row.rs` (`ConfigSelectRow { label: Signal<String>, value: Signal<i64>, options: Signal<Vec<String>>, on_change: Callback<usize> }`; keep the V1 index-4 rule behind a flag if QUIRK-1 is to be preserved), `input_row.rs`, `check_row.rs`, `slider_row.rs` (formats with `js_to_fixed` / `js_math_round` / `js_number_to_string`), `freeapi_quota.rs` (`FreeApiQuotaDisplay`, `use_free_api_quota(active: Signal<bool>)`).
- `components/menu.rs` — `MenuPanel`, `MenuItem`, `SmallMenuLabel`. `components/marquee_status.rs` — `MarqueeStatus`.
- `keyboard.rs` — `install_root_keydown(ctx)` implementing §7.4 with `web_sys::KeyboardEvent::key()`. `is_text_input_target(&EventTarget) -> bool`.
- `platform/mod.rs` — `trait PlatformShell { fn is_fullscreen(&self) -> bool; fn request_fullscreen(&self); fn exit_fullscreen(&self); fn on_fullscreen_change(&self, cb: Box<dyn Fn(bool)>); fn save_text_file(&self, file_name: &str, mime: &str, contents: &str) -> Result<(), String>; fn shutdown(&self); }`. `platform/web.rs` uses the DOM Fullscreen API, anchor + blob download, and the `window.close()` / `history.back()` / redirect sequence. `platform/tauri.rs` uses IPC: `window_set_fullscreen`, `window_is_fullscreen`, the `tauri://resize` event to sync ScreenMode, a save dialog plus write, and `app_quit`.
- `services/font_loader.rs` — `load_ms_mincho()` (web only; see Risks).
- `transport` — `Transport::get_free_api_quota() -> Option<FreeApiQuota>`, `tts_health(url)`, `stt_health(url)`, `rag_health(url)`. HTTP on Web (exact URLs of §7.12, `cache: no-store` for the quota), IPC on desktop.
- `i18n` — `t(key, default, lang)` from `amadeus-core::i18n`, plus a `tr` closure in ConfigPanel bound to `form.language`.

### amadeus-server (axum)

`GET /amadeus/app/api/chat/quota` → `{"count","max":65000,"remaining"}` + `Cache-Control: no-store`; `POST /amadeus/app/api/{tts,stt,rag}/health` with `{serverUrl}` → `{available,…}`, HTTP 400 `{available:false,error:"serverUrl is required"}`. Serve `/resources/**` (V1 cache headers: `public, max-age=86400, stale-while-revalidate=604800`) and the SPA at `/amadeus/app` with fallback. (Details in the backend specs.)

### amadeus-desktop (Tauri 2)

Commands: `quota_get`, `tts_health`, `stt_health`, `rag_health`, `export_save_file(file_name, contents)` (dialog plugin + fs), `window_set_fullscreen(bool)`, `window_is_fullscreen() -> bool`, `app_quit()`. Note that a desktop install has its own localStorage origin, so web settings do not carry over (the format is the same).

---

## Open questions / risks

1. **QUIRK-1 (Español/한국어 unselectable)**: replicate the V1 bug for parity, or scope the Vertex filter to the provider select? A fix is recommended; the product owner should confirm.
2. **QUIRK-3 (import drops the system prompt)**: a fix in the ChatPanel bridge is recommended. It needs agreement with the ChatPanel spec, because exact parity would carry a real functional bug into V2.
3. **QUIRK-6 (boot restart on re-render)**: the plan is not to replicate it; confirm that this deviation is acceptable.
4. **Desktop fullscreen, resolution and shutdown semantics**: Tauri should use window APIs instead of the DOM Fullscreen API (WKWebView element fullscreen is not reliably enabled). Decide whether `Config_Resolution` resizes the desktop window, whether a persisted ScreenMode=0 is applied at launch, and whether SHUT DOWN calls `app.exit(0)` (recommended) instead of redirecting to `https://dsuv.elvelt.com/amadeus`.
5. **Export on desktop**: anchor downloads of blob URLs are unsupported or inconsistent in WKWebView and WebKitGTK, so route `save_text_file` through Tauri dialog + fs. The success message format (`"エクスポート完了: " + fileName`) should stay; decide what to show if the user cancels the dialog (V1 has no cancel case).
6. **MS Mincho download**: a third-party GitHub mirror of a proprietary Microsoft font, which raises licensing, privacy (it leaks the client IP to GitHub), CSP and offline concerns. Tauri needs CSP `connect-src` for `raw.githubusercontent.com`. Recommend keeping it on Web for parity, flagged, and disabling it on desktop. The owner should decide.
7. **JS number and string formatting parity**: settings values (`String(number)`), `toFixed`, `Math.round`, `parseInt`/`parseFloat`, `trim` and `toLocaleString` must use the `jsnum` helpers, or persisted strings and UI text drift (e.g. `-0`, ties at .x5, U+FEFF trimming). Add unit tests against values captured from V1.
8. **Import error text**: V8 `SyntaxError` messages cannot be reproduced; accept `SyntaxError: <serde message>`.
9. **`turnCount` type**: V1 keeps any JSON number (floats allowed) and re-exports it unchanged. Choose `f64` with integral values serialised without `.0` (custom serializer), or document truncation.
10. **Hidden-setting normalisation on Apply (QUIRK-20)**: must be reproduced exactly, or users' TTS/STT/RAG settings stored by V1 differ after the first V2 Apply.
11. **`llmStatus` staleness after desktop Apply (QUIRK-15)**: the fix is low-risk; confirm.
12. **FreeAPI quota on desktop**: the counter lives on the hosted server (`.amadeus_request_count`, cap 65000). If desktop FreeAPI traffic goes to the public server, `quota_get` must query the remote URL, not a local counter. This needs an architecture decision with the backend spec.
13. **Mobile hardware-keyboard menu (QUIRK-18)** and **invisible focused buttons (QUIRK-21)**: replicate or guard? Both are cheap to fix.
14. **Startup health checks** fire three POSTs even when TTS/STT/RAG are disabled; the defaults point at localhost:5100–5102, so the server-side probes time out after 5 s. Keep for parity (StatusPanel shows availability), but consider a flag.
15. **iPhone/old iPadOS `requestFullscreen` undefined** throws inside `handleApply`. V2 should guard with feature detection (deviation: Apply completes instead of aborting halfway). Recommended.
16. **Layout switch remounts ChatPanel** (conversation reset). Decide whether V2 keeps the conversation state outside the layout tree. Parity says reset; UX says keep.
17. The status default-model table (§5) is cosmetic and differs from the backend; keep it verbatim unless the backend spec decides to share one table.
18. The case-insensitive regex in `addBackLog`: JS non-unicode `/i` does not fold `ſ` or `K` (Kelvin sign) to ASCII, while Rust `(?i)` does. The edge case is negligible, but note it in tests.

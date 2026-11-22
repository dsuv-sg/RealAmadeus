# Spec app-part2 — `ChatPanel` (RealAmadeusWeb V1 → RealAmadeus V2 `amadeus-ui`)

> **Primary source:** `/home/elvelt_admin/Elvelt/dsuv/components/amadeus/RealAmadeusApp.tsx`
> `type ChatPanelProps` (L816–826) and `function ChatPanel` (L828–1995).
> Working tree of 2026‑11‑16. It contains uncommitted edits on top of HEAD `7e45174`: the optional `setActiveOverlay` prop, the mobile `ACHIEVEMENTS` trigger button, and the mini‑game `onClose` that resets the overlay. **All line numbers below refer to the working‑tree files.**
>
> **Also read in full for this spec, because ChatPanel's behavior depends on them:**
> `lib/amadeus/chat-utils.ts`, `lib/amadeus/signal.ts`, `lib/amadeus/types.ts`, `lib/amadeus/localization.ts`, `lib/amadeus/app-settings.ts`, `lib/amadeus/memory-manager.ts`, `lib/amadeus/services/{ai-service,tts-service,audio-service,stt-service,achievement-manager,random-event-service}.ts`, `components/amadeus/Live2DCanvas.tsx`, RealAmadeusApp.tsx L1–490 (module helpers), L2189–2264 (BackLogPanel), L2513–2827 (MiniGamePanel), L3413–3486 (config import/export UI), L3810–3871 (LLM‑status helpers), L3904–4779 (parent `RealAmadeusApp`), `app/amadeus/app/globals.css` (chat rules), every locale JSON for the keys used, `app/amadeus/app/api/chat/{route.ts,stream/route.ts,rate-limit.ts,shared.ts}` (only for error strings / SSE shape), `public/resources/live2d/reama-cubism-runtime.js` L18131–18165 (player setters).
>
> **Cross-references:** other specs own the helpers at L1–490 (app‑part1), BackLogPanel, MiniGamePanel and AchievementPanel (their own part), the services (services specs) and the Live2D runtime (amadeus‑live2d). This spec repeats every part of them that ChatPanel's observable behavior depends on.

---

## 0. TL;DR (read first)

1. ChatPanel is the whole conversation screen. It holds the Live2D model, the dialogue box with its typewriter, the chat input bar (with an optional mic button and a SEND button), the MINI GAME and ACHIEVEMENTS triggers, the (always mounted) MiniGamePanel, and the achievement toast.
2. A 4‑state machine `chatState ∈ {inputReady, waitingAPI, typing, waitForAdvance}` drives everything. Two flags refine `typing`: `waitingForInput` (paused at a sentence end) and `skipTyping` (the user asked to reveal everything).
3. LLM providers `1 Gemini, 3 Groq, 4 Vertex, 5 Ollama, 6 OpenRouter` use **SSE streaming** (`AIService.sendChatStreaming`). Every other index (`0 OpenAI, 2 Claude, 7 Local, 8 FreeAPI = default`) uses **one JSON request** (`AIService.sendChat`).
4. **Confirmed V1 defect:** with a streaming provider the UI never leaves `waitingAPI`. The dots animate forever, the answer text is never shown and never logged, but TTS audio, emotion changes and memory still happen. The cause is that the stream handlers are **frozen first‑render closures** that always see `chatState === "inputReady"` (§6). Parity versus fix is a decision for the team: see Open question #1 and the exact recommended fix in §6.4.
5. **Typewriter:** the base speed is 20 chars/s × `Config_TextSpeed`. It stops at every "pause character" (end of a sentence), shows `▼` and waits for a click, Enter or the auto timer. When it resumes it clears the box and starts a new "page". A click while it is typing reveals all remaining text at once.
6. **Backlog:** the user line is logged on submit. After that the backlog receives each completed page and, after a skip, the revealed rest. **At the very end it also receives the full response text if that differs from the last logged page.** So V1 duplicates multi‑sentence answers in the backlog. Keep this for parity unless told otherwise.
7. **Lip sync is binary and NOT driven by audio.** The mouth value is 1 while characters are being typed (and not paused), otherwise 0. `spokenChar` is the character being typed; the runtime maps it to a vowel mouth shape.
8. **TTS** (experimental, off by default): text is cut into chunks at `。！？!?\n` and played one chunk after another, with at most one synthesis in flight. TTS is **not synchronised** with the typewriter.
9. **Idle:** after 60 s without activity in `inputReady` or `waitForAdvance`, the emotion becomes `SLEEPING` and the placeholder shows "Zzz...". Any activity at least 800 ms after she fell asleep wakes her to `NORMAL`.
10. **Auto mode** (`Config_AutoMode`, toggled with F3 in the parent): after every pause and at the end, wait `Config_AutoSpeed` seconds, then advance.
11. **Every mount** types out a random greeting chosen by time of day (`RandomEventService`). The greeting is not sent to the LLM, has no TTS, and unlocks the `first_event` achievement.
12. **Keyboard:** `Enter` submits in `inputReady` and advances in the other states. `Ctrl+C` cancels while `waitingAPI`. All keys are ignored while the menu (PC: Tab menu; mobile: the settings sheet) or the mini game is open.
13. **Dev commands** (typed into the input): `/dev emotion help`, `/dev emotion <TAG>`, `/dev motion reset`, `/dev achievements reset`, `/dev steiner`.
14. **Remounting:** the parent re‑mounts ChatPanel whenever it switches between the mobile and PC layouts (the viewport crosses the breakpoint) and on logout/login. Each re‑mount resets the conversation and shows a new greeting; the backlog lives in the parent and survives.
15. RAG (`VectorDatabase`) is **not used anywhere in the V1 chat flow**. The only "memory" is the MemoryManager text injected into the system prompt.

---

## 1. Responsibilities (in data‑flow order)

```
user text ─▶ submitMessage ─▶ dev command? ──yes──▶ local action (emotion / help text / achievements)
                │no
                ▼
     history += user msg (trim to 30, MemoryManager.recordInteraction, refresh system prompt)
     backlog += user line
     chatState = waitingAPI (animated "." ".." "..." indicator), TTS session reset
                ▼
     AIService.sendChat (JSON)      or      AIService.sendChatStreaming (SSE)
                ▼                                     ▼
     responseReceived(text)              streamToken(token)* … streamComplete(full)
     handleApiSuccess                    handleStreamToken / handleStreamComplete
       strip <think>, memory cmds          incremental emotion-tag detection, display text,
       emotion tag → Live2D,               incremental TTS chunking, memory cmds at end,
       achievements, history += reply      achievements, history += reply
       chatState = typing, TTS chunks      (V1: chatState stays waitingAPI — §6)
                ▼
     typewriter (pages, pauses ▼, skip) ──▶ backlog += pages ──▶ waitForAdvance ▼ ──advance──▶ inputReady (emotion NORMAL)
     error at any point ─▶ handleApiError: ANGRY + friendly Japanese message typed out
```

The panel also does the following, alongside the main flow:
- It reports LLM status and latency to the parent, through `onLlmStatusUpdate`.
- It runs the STT mic flow.
- It runs the idle/sleep detection.
- It runs auto mode.
- It shows the achievement toast.
- It opens the mini game and drives Live2D while the game is open.
- It exposes a `ConversationBridge` that the parent uses for export and import.
- It handles the global keyboard shortcuts.

---

## 2. Component contract

### 2.1 Props (L816–826)

```ts
type OverlayPanel = "none" | "backlog" | "config" | "status" | "changeLog" | "help"
                  | "confirmLogout" | "confirmShutdown" | "minigame" | "achievements";   // L45–55 ("minigame" is never used)

type ConversationBridge = {                       // L252–256
  getHistory: () => ChatMessage[];
  getTurnCount: () => number;
  applyImported: (messages: ChatMessage[], turnCount: number) => void;
};

type ChatPanelProps = {
  autoMode: boolean;                 // Config_AutoMode === 1 (reactive, parent)
  autoSpeed: number;                 // Config_AutoSpeed float, default 3.0 (reactive, parent), seconds
  configLanguage: number;            // Config_Language int, default 0 (reactive, parent); UI language index 0..12
  menuPanelOpen: boolean;            // PC: isTabActive ; mobile: mobileSettingsOpen
  onBackLogAdd: (role: string, message: string) => void;   // parent addBackLog (stable)
  onLlmStatusUpdate: (status: LlmStatus) => void;          // parent setLlmStatus (stable)
  activeOverlay: OverlayPanel;       // only used to reset the idle timer when it changes
  setActiveOverlay?: (overlay: OverlayPanel) => void;      // mobile only; undefined on PC
  conversationRef: { current: ConversationBridge | null }; // parent useRef
};
```

`ChatMessage = { role: "system" | "user" | "assistant"; content: string }` and `LlmStatus = { provider: string; model: string; latencyMs: number }` (`lib/amadeus/types.ts`).

### 2.2 How the parent mounts it

**PC branch** (RealAmadeusApp.tsx L4508–4522). The panel sits inside `.system-panel`, after the `menu-wallpaper`, `title-bar` and `system-bg` elements:
```tsx
{appState === "chat" && (
  <>
    <ChatPanel autoMode={autoMode} autoSpeed={autoSpeed} configLanguage={configLanguage}
      menuPanelOpen={isTabActive} onBackLogAdd={addBackLog} onLlmStatusUpdate={setLlmStatus}
      activeOverlay={activeOverlay} conversationRef={conversationBridgeRef} />   {/* no setActiveOverlay */}
    {autoMode && <div className="auto-text">Auto</div>}
  </>
)}
```
**Mobile branch** (L4403–4415). The panel sits inside `<div className="mobile-stage">` with `menuPanelOpen={mobileSettingsOpen}` and `setActiveOverlay={setActiveOverlay}`. Mobile renders **no** `auto-text` label.

The parent picks the branch with `useIsMobileViewport()` (§3.11).

### 2.3 Mount and unmount semantics (keep these)

- ChatPanel mounts when `appState` becomes `"chat"` and unmounts on logout. It also unmounts and re‑mounts whenever `isMobileViewport` flips, because the parent renders a different tree for each layout.
- Each mount gets a fresh conversation (system prompt only), a fresh startup greeting, a new Live2D player and WebGL context, and new subscriptions to the services.
- Unmount cleanup calls `STTService.stopRecording()` and `AIService.cancelStreaming()`, then disconnects every signal handler.
- The backlog entries, the LLM status and `conversationBridgeRef` live in the parent. The backlog survives a re‑mount.

---

## 3. External APIs used (exact surface)

All services are **module‑level singletons**. They expose `Signal`s from `lib/amadeus/signal.ts`:

```ts
class Signal<TArgs extends unknown[] = []> {
  private handlers = new Set<(...args: TArgs) => void>();
  connect(handler): () => void { this.handlers.add(handler); return () => this.disconnect(handler); }
  disconnect(handler): void { this.handlers.delete(handler); }
  emit(...args): void { for (const handler of [...this.handlers]) handler(...args); }   // snapshot iteration
  clear(): void { this.handlers.clear(); }
}
```
Emission is synchronous. It iterates over a snapshot of the handler set: a handler removed during an emit is still called once, and a handler added during an emit is not called. Several service methods emit **synchronously, inside the call** (re‑entrant). These are flagged below because they matter for Rust `RefCell` borrows.

### 3.1 AppSettings (`lib/amadeus/app-settings.ts`), the localStorage key/value store

- All values are stored as **strings** in `localStorage`.
  - `getInt(k, d)`: `parseInt(raw, 10)`. Returns the parsed value if it is finite, otherwise `d`. A missing key also returns `d`.
  - `getFloat(k, d)`: `parseFloat(raw)`, same fallback rules.
  - `getString(k, d)`: the raw string, or `d` when the key is missing.
  - `setInt` writes `String(Math.trunc(v))` and emits `settingsChanged(key)`. `setFloat` and `setString` work the same way.
- `initializeDefaults()` runs in the parent's mount effect, before ChatPanel can exist. It writes every missing default; notable defaults are `Config_ApiProvider = 8`, `Config_TextSpeed = 1.0`, `Config_AutoSpeed = 3.0`, `Config_EyeTracking = 0` and `Config_ChatLanguage = 0`. `Config_AudioInputEnabled` is **not** among the defaults.
- `useAppSettingInt(key, def)` / `useAppSettingFloat` (L429–443) are `useSyncExternalStore(subscribe = settingsChanged.connect, getSnapshot = getInt/getFloat, getServerSnapshot = def)`. They are reactive only to in‑app `set*` calls, not to `storage` events from other tabs.

### 3.2 MemoryManager (`lib/amadeus/memory-manager.ts`; localStorage `"RealAmadeus.Memory"`, JSON)

Members used by ChatPanel:

| Member | Behavior relevant to ChatPanel |
|---|---|
| `getMemoryContext(): string` | Multi‑line Japanese block: user name rule, top‑6 facts, top‑4 episodes (or the last 3 summaries), last session, interaction count, recent emotions (only when there are ≥ 3). It is **always non‑empty**: the first line `【ユーザー情報】ユーザーのことは「${name}」と呼びなさい。「Salieri」や「サリエリ」とは絶対に呼んではいけません。` is always pushed, because an empty name or `salieri` maps to `あなた`. ChatPanel's `if (memoryContext)` checks are therefore always true. Algorithm: L189–238. |
| `getDynamicContext(turnCount): string` | Returns exactly `` `【現在の状況】現在日時（クライアント時刻）: ${YYYY}/${MM}/${DD} ${HH}:${mm}:${ss} (${getTimeContext()}) / 会話ターン数: ${turnCount}\n${hint}` ``. The local time is zero‑padded. `hint` is chosen uniformly at random from `["（今は少しリラックスしている）","（知的好奇心が高まっている）","（少し眠そう）","（何かを考え込んでいる）","（いつも通りの調子）"]`. `getTimeContext()`: `5≤h<10 "朝"`, `10≤h<12 "午前中"`, `12≤h<14 "昼"`, `14≤h<17 "午後"`, `17≤h<20 "夕方"`, `20≤h<24 "夜"`, else `"深夜"`. |
| `recordEmotion(tag)` | Appends to recent emotions (keeps the last 10) and saves. |
| `totalInteractions` (getter) | Returns `memory.totalInteractions ?? 0`. |
| `recordInteraction()` | Increments `totalInteractions`, sets `lastSessionDate = "YYYY-MM-DD HH:mm"`, saves, and emits `statsChanged`. |
| `addUserFact(fact)` | Matches facts case‑insensitively after trimming. A duplicate gets `strength+1`; a new fact starts at strength 1. Keeps the last 50 and saves. |
| `setUserName(name)` | Trims the name. An empty name is ignored. Saves and emits `userNameChanged`. |
| `addEpisodicMemory(summary, tags)` | Keeps the last 50 and saves. |
| `trimConversationHistory(history, 30)` | **Mutates the array in place.** Let `n` be the number of non‑system messages. If `n > 30`, it removes `n − 30 + 4` messages **starting at index 1**, because it assumes index 0 is the system message. It builds `` `[${stamp}の会話] ${parts.join(" / ")}` ``, with no space before `の`. `stamp = new Date().toLocaleString("ja-JP", {month:"2-digit", day:"2-digit", hour:"2-digit", minute:"2-digit"})`, e.g. `11/16 19:04`, and each part is `` `${role}: ${content.length > 50 ? content.slice(0,50) + "..." : content}` ``. A summary longer than 300 chars is cut to 300 + `"..."`. It then calls `addConversationSummary` and returns the number removed. |

### 3.3 AIService (`lib/amadeus/services/ai-service.ts`)

- `sendChat(messages)` sends `POST /amadeus/app/api/chat` with `Content-Type: application/json` and a body of type `ChatPayload` (below).
  - The response JSON is `{response?: string; error?: string}`.
  - If the response is not ok, it emits `errorOccurred(data?.error ?? \`Request failed (${status})\`)`.
  - If it is ok, it emits `responseReceived(data?.response ?? "")`.
  - If an exception is thrown, it emits `errorOccurred(err.message || "Unknown error")`.
  - **It cannot be cancelled.**
- `sendChatStreaming(messages)` first calls `cancelStreaming()`. It then sends `POST /amadeus/app/api/chat/stream` with the same headers and payload plus an `AbortController` signal.
  - If the response is not ok or has no body, it emits `errorOccurred(json?.error ?? \`Request failed (${status})\`)`.
  - Otherwise it reads the body as UTF‑8, splits it on `\n`, `trim()`s each line, and only looks at lines that start with `"data: "`. The rest of the line after `"data: "` is trimmed and then:
    - `{"type":"token","token":t}`: if `t` is truthy, it adds `t` to the accumulator and emits `streamToken(t)`.
    - `{"type":"complete","fullResponse":f}`: it emits `streamComplete(f ?? accumulated)`, clears the controller and **returns**.
    - `{"type":"error","error":e}`: it emits `errorOccurred(e ?? "Streaming error")` and returns.
    - If the line is not valid JSON, the raw payload text is treated as a token: it is accumulated and `streamToken(raw)` is emitted.
  - At EOF without a `complete` event, it emits `streamComplete(accumulated)`.
  - An abort ends the stream silently. Any other exception emits `errorOccurred(msg || "Unknown streaming error")`.
- `cancelStreaming()` aborts the current streaming request, if there is one. It is a no‑op for `sendChat` and after a stream has completed.
- `ChatPayload` is built from settings at send time:
  - `provider = getInt("Config_ApiProvider", 8)`
  - `apiKey = getString("Config_ApiKey_"+provider) || getString("Config_ApiKey")`
  - `model = getString("Config_ModelName_"+provider) || getString("Config_ModelName")`
  - `webSearch = Config_WebSearch==1`
  - `vertexProject`
  - `vertexLocation` (default `"us-central1"`)
  - `localProxyHost` (default `"127.0.0.1"`)
  - `localProxyPort` (default `""`)
  - `useGCloudCLI = Config_GCloudCLI==1` (default 1)
  - `ollamaHost` (default `"http://localhost:11434"`)
  - `openaiCompatible = Config_OpenAICompatible==1`
  - `openaiBaseUrl`

  The payload also carries `messages: ChatMessage[]`. The amadeus‑backend/server specs own the details.

### 3.4 TTSService (`lib/amadeus/services/tts-service.ts`)

- `enabled` getter: mirrors `Config_Experimental_TTS_Enabled == 1`. It is re‑synced on every `Config_Experimental_TTS_*` settings change.
- `synthesize(text)` (async; **the checks before the first `await` run synchronously**):
  - If TTS is disabled, it emits `synthesisError("TTS is not enabled")` synchronously.
  - If the trimmed text is empty, it emits `synthesisError("Empty text provided")` synchronously.
  - On a **cache hit** (key = `simpleHash(text|speaker|serverUrl)`), it emits `synthesisComplete(cachedObjectUrl)` **synchronously, inside the call**.
  - Otherwise it sends `POST /amadeus/app/api/tts/synthesize` with JSON `{serverUrl, speaker, text, useCache: true}`. The response blob is checked (an empty blob is an error), turned into an object URL, cached, and emitted with `synthesisComplete(url)`. Failures emit `synthesisError(msg)`.
- Signals used: `synthesisComplete(url)` and `synthesisError(msg)`.

### 3.5 AudioService (`lib/amadeus/services/audio-service.ts`)

- `playFromFile(url)`:
  1. Calls `stop()`.
  2. Creates `new Audio(url)` with `volume = _volume`. `_volume` comes from `Config_VoiceVol` when the module loads and is updated by `setVolume`.
  3. Wires the callbacks: `onended` stops the lip timer, sets the value to 0, then emits `lipSyncValueChanged(0)`, `playbackStateChanged` and `playbackFinished`. `onerror` emits `errorOccurred("Audio playback failed")`. When `play()` resolves, it emits `playbackStateChanged` and `playbackStarted` and starts a 50 ms lip timer. When `play()` rejects, it emits `errorOccurred(msg)`.
  4. **Caveat:** a single failure can emit `errorOccurred` twice (from `onerror` and from the `play()` rejection).
- `stop()` (synchronous): clears the lip timer, emits `lipSyncValueChanged(0)`, pauses the audio, rewinds it and drops it; if it was playing, it also emits `playbackStateChanged`. Pausing while a `play()` promise is pending rejects that promise, which leads to `errorOccurred`.
- Signals ChatPanel uses: `playbackStarted`, `playbackFinished`, `errorOccurred`. **It does not use `lipSyncValueChanged`.**

### 3.6 STTService (`lib/amadeus/services/stt-service.ts`)

- `enabled` getter/setter.
  - Setting it to `true` writes `Config_Experimental_STT_Enabled = 1`, emits `enabledChanged` and starts a health check.
  - Setting it to `false` writes the key and stops recording.
- `isRecording` getter.
- `startRecording()` (async):
  - If STT is disabled, it emits `transcriptionError("STT is not enabled")` (synchronously).
  - If it is already recording, it returns.
  - **If the browser has `SpeechRecognition`/`webkitSpeechRecognition`, that is used instead of the server provider:**
    - Settings: `lang` is `ja-JP` for `ja`, `en-US` for `en`, otherwise the raw language string (default `ja-JP`). `interimResults = false`, `continuous = false`.
    - The service then synchronously emits `recordingStateChanged` and `recordingProgress(0.5)`, and starts a 30 s max timer.
    - `onresult` emits `transcriptionComplete(transcript)`, or `transcriptionError("No speech detected")` if the transcript is empty.
    - `onerror` emits `transcriptionError(\`Speech recognition failed: ${code}\`)`.
    - `onend` sets recording to false, emits `recordingStateChanged` and then `recordingProgress(0)`.
  - **Otherwise it uses `getUserMedia({audio:true})` + `MediaRecorder.start(250)`:**
    - On start it emits `recordingStateChanged`.
    - While recording it emits the level 0..1 via `requestAnimationFrame` (average of `getByteFrequencyData` / 255, `fftSize = 2048`) through `recordingProgress(level)`. A 30 s max timer applies.
    - On stop it builds the blob. If the blob is smaller than 1024 bytes, it emits `transcriptionError("Recording too short")`. Otherwise it sends `POST /amadeus/app/api/stt/transcribe` as multipart with `audio` (file `recording.webm`), `provider`, `language` and `serverUrl`, then emits `transcriptionComplete(data.text ?? "")` or `transcriptionError(...)`.
- `stopRecording()`.
- Signals used: `transcriptionComplete(text)`, `transcriptionError(msg)`, `recordingStateChanged()`, `recordingProgress(level)`.

### 3.7 AchievementManager (`lib/amadeus/services/achievement-manager.ts`; localStorage `"amadeus_achievements"`)

- `notifyEvent(type, data)`. ChatPanel sends these types:
  - `"emotion"` with `{emotion}`: unlocks `first_<emotion>` for the 11 tags, tracks `seenEmotions`, and unlocks `all_emotions` once all 11 have been seen.
  - `"chat"` with `{totalInteractions}`: unlocks `first_chat` at ≥1, `chat_10`, `chat_50`, `chat_100`, plus `night_owl` (local hour 0–3) and `early_bird` (5–6).
  - `"random_event"`: unlocks `first_event`.
  - `"dev_command"`: unlocks `dev_user`.
  - `"secret_steiner"`: unlocks `secret_reading_steiner`.
- `resetAll()`.
- `achievementUnlocked(id, title, description)` is emitted synchronously by `unlock`. The title and description are localized with `Config_Language` at the moment of unlock.
- **Note:** the launch‑streak unlocks (`streak_3`, `streak_7`) happen in the constructor, when the module is evaluated at page load. That is before ChatPanel subscribes, so **no toast** appears for them.

### 3.8 RandomEventService (`lib/amadeus/services/random-event-service.ts`)

- The time‑of‑day bucket is chosen once, **when the module is constructed** (page load), from the local hour: `[0,5)` night, `[5,10)` morning, `[10,14)` forenoon, `[14,18)` afternoon, `[18,22)` evening, else late.
- The data (key, Japanese default text, emotion, weight) is at **L47–156**. Copy it verbatim into amadeus‑core.
- `tryTriggerEvent()` rolls `Math.floor(Math.random() * totalWeight)` and picks the entry by cumulative weight. It always succeeds, because every pool is non‑empty and every weight is > 0. It stores key, text and emotion and emits `eventTriggered`, which ChatPanel does not use.
- `getEventText()` returns `t(lastEventKey, lastEventDefaultText, AppSettings.getInt("Config_Language", 0))`.
- `getEventEmotion()` returns the stored emotion, one of the 11 tags.

### 3.9 chat-utils (`lib/amadeus/chat-utils.ts`). Verbatim; port these exactly.

```ts
const EMOTION_TAG_PATTERN = /\[(NORMAL|SMILE|ANGRY|SAD|SURPRISED|BLUSH|WINK|DISGUST|SMUG|THINKING|PANIC)\]/gi;

export function stripEmotionTag(rawText: string): string {
  const withoutThink = rawText.replace(/<think>[\s\S]*?<\/think>/gi, "").trim();
  return withoutThink.replace(EMOTION_TAG_PATTERN, "").trim();
}

export function parseEmotionTag(rawText: string): string {
  const match = rawText.match(/\[(NORMAL|SMILE|ANGRY|SAD|SURPRISED|BLUSH|WINK|DISGUST|SMUG|THINKING|PANIC)\]/i);
  return match ? match[1].toUpperCase() : "NORMAL";          // first tag ANYWHERE in the text
}

export function extractUserNameFromFact(fact: string): string {
  const value = fact.trim();
  if (!value) return "";
  const match = value.match(/(?:ユーザー\s*の\s*)?名前は\s*[「『"']?([^」』"'。、.!?！？\s]+)/);
  return match?.[1]?.trim() ?? "";
}

export function isPauseCharacter(char: string, nextChar: string): boolean {
  if (char === ".") {
    if (nextChar === ".") return false;
    if (nextChar === " " || nextChar === "\n" || nextChar === "\r" || nextChar === "") return true;
    return false;
  }
  const isPause = char === "。" || char === "！" || char === "？" || char === "!" || char === "?" || char === "\n";
  if (!isPause) return false;
  return !["」", "）", ")", "』", "”"].includes(nextChar);
}

export function charDelayForChar(char: string, textSpeed: number): number {
  const base = 1000 / 20 / Math.max(0.1, textSpeed);
  if (char === "、" || char === "," || char === "…") return base * 2.5;
  if (char === "」" || char === "）") return base * 1.5;
  return base;
}
```

`extractMemoryCommands(rawText): { cleanedText, facts: string[], events: {summary, tags}[] }` lives at L95–193 (with `findMemoryCommandEnd`, `parseSaveFactPayload` and `parseSaveEventPayload` at L27–140). In outline:
- It scans for `[SAVE_` followed by a header up to `:`, with whitespace removed and uppercased; the header must be `SAVE_FACT` or `SAVE_EVENT`.
- The payload ends at the first `]` that is outside any string, brace or bracket nesting.
- Payloads may be JSON objects. For FACT it reads `content`/`fact`/`text`. For EVENT it reads `summary`/`content` and `tags`, where `tags` is an array or a string split on `,|、`.
- Matched ranges are removed from the text, `\n{3,}` is collapsed to `\n\n`, and the result is trimmed. **The collapse and trim happen even when no command is present.** The input is trimmed first. Facts are de‑duplicated.

The amadeus‑core spec owns this; reproduce it exactly.

### 3.10 Localization `t()` (`lib/amadeus/localization.ts`)

```ts
export function t(key: string, defaultValue: string, configLanguage: number): string {
  const dict = translations[configLanguage.toString()] || ja;   // unknown index → Japanese dict
  if (dict && key in dict) return dict[key];
  return defaultValue || key;
}
```
Indices 0..12 map to `ja, en, zh, ko, es, fr, de, ru, uk, pt, tr, he, ar`. ChatPanel always passes the `configLanguage` prop.

### 3.11 Module helpers in RealAmadeusApp.tsx used by ChatPanel

- **`CHAT_PERSONA_PROMPTS`** (L64–161): 8 persona prompts, index 0 ja, 1 en, 2 zh, 3 ko, 4 es, 5 fr, 6 de, 7 ru. Copy them **byte‑for‑byte**: they contain full‑width brackets `【】`, the full‑width `＠` in the ja prompt, and `\n` escapes. Each one tells the model to start its answer with exactly one of `[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]`.
- **`CHAT_LANGUAGE_NAMES`** (L163–177), verbatim:
  `["Japanese (日本語)", "English", "Chinese (中文)", "Korean (한국어)", "Spanish (Español)", "French (Français)", "German (Deutsch)", "Russian (Русский)", "Ukrainian (Українська)", "Portuguese (Português)", "Turkish (Türkçe)", "Hebrew (עברית)", "Arabic (العربية)"]`
- **`getSystemPromptForLanguage(langIndex)`** (L179–197), verbatim logic:
  ```ts
  const targetLang = langIndex >= 0 && langIndex < 13 ? CHAT_LANGUAGE_NAMES[langIndex] : CHAT_LANGUAGE_NAMES[1];
  const overrideText =
    "\n\n[LANGUAGE RULE — MANDATORY]\n" +                       // em dash U+2014
    "Your designated conversation language is: " + targetLang + ".\n" +
    "You MUST respond in " + targetLang + " at all times, regardless of what language the user writes in.\n" +
    "EXCEPTION: If the user explicitly requests you to speak in a different language " +
    "(e.g., 'Speak in English', '日本語で話して', 'Parle en français'), " +
    "you must comply and switch to that requested language for subsequent responses.";
  if (langIndex >= 0 && langIndex < 8) return CHAT_PERSONA_PROMPTS[langIndex] + overrideText;
  return CHAT_PERSONA_PROMPTS[1] + overrideText;     // uk/pt/tr/he/ar & out-of-range → English persona + target-language rule
  ```
- **`getCurrentChatLanguageIndex()`** (L199–205): `chatLang = getInt("Config_ChatLanguage", 0)`. If `chatLang === 0`, it returns `getInt("Config_Language", 0)` (the chat follows the UI language); otherwise it returns `chatLang - 1`.
- **`splitMessage(text)`** (L207–240). The parent's import path uses it to split assistant messages into backlog segments. Verbatim:
  ```ts
  function splitMessage(text: string): string[] {
    if (!text) return [];
    const result: string[] = [];
    let startIdx = 0; let i = 0;
    while (i < text.length) {
      if (i === startIdx) {
        let ch = text.charAt(i);
        while (i < text.length && (ch === " " || ch === "\t" || ch === "\n" || ch === "\r")) {
          i += 1; if (i < text.length) ch = text.charAt(i);
        }
        startIdx = i;
      }
      if (i >= text.length) break;
      const c = text.charAt(i);
      const nextC = i + 1 < text.length ? text.charAt(i + 1) : "";
      i += 1;
      if (isPauseCharacter(c, nextC) || i === text.length) {
        const segment = text.substring(startIdx, i);
        if (segment.trim().length > 0) result.push(segment);
        startIdx = i;
      }
    }
    return result;
  }
  ```
- **`toChatRole(role)`** (L258–263): returns `"system"`, `"user"` or `"assistant"` unchanged; any other role becomes `"assistant"`.
- **`useIsMobileViewport()`** (L445–475): `width = visualViewport?.width ?? innerWidth`, `coarse = matchMedia("(pointer: coarse)").matches`. The result is **`width <= 900 || (coarse && width <= 1200)`**. It re‑evaluates on window `resize`, visualViewport `resize` and the coarse media query `change`. The server snapshot is `false`.
- **`getLlmStatusFromSettings(latencyMs = -1)`** (L3856–3871), verbatim:
  ```ts
  const providerIndex = AppSettings.getInt("Config_ApiProvider", 8);
  let model = AppSettings.getString(`Config_ModelName_${providerIndex}`, "");
  if (!model) model = AppSettings.getString("Config_ModelName", "") || getDefaultModelForProvider(providerIndex);
  const isCompatible = AppSettings.getInt("Config_OpenAICompatible", 0) === 1;
  const baseUrl = AppSettings.getString("Config_OpenAIBaseUrl", "");
  return { provider: getProviderDisplayName(providerIndex, isCompatible, baseUrl), model, latencyMs };
  ```
  - `getDefaultModelForProvider`: 0 `"gpt-4o"`, 1 `"gemini-1.5-pro"`, 2 `"claude-3-5-sonnet-20241022"`, 3 `"llama-3.3-70b-versatile"`, 4 `"gemini-1.5-pro"`, 5 `"llama3.2"`, 6 `"openai/gpt-4o"`, 8 `"default"`, any other `"default"`.
  - `getProviderDisplayName`: 0 → `isCompatible ? \`OpenAI互換(${baseUrl})\` : "OpenAI"`, 1 `"Gemini"`, 2 `"Claude"`, 3 `"Groq"`, 4 `"Vertex AI"`, 5 `"Ollama"`, 6 `"OpenRouter"`, 8 `"FreeAPI"`, any other `"Unknown"`.

### 3.12 Child components

- **`Live2DCanvas`** (`components/amadeus/Live2DCanvas.tsx`) props: `{ emotion: string; lipSyncValue: number; spokenChar?: string; eyeTrackingEnabled?: boolean }`.
  - It renders `<div class="live2d-host">` and creates the Cubism player through `window.ReamaCubismRuntime.create(host)`, after loading `/resources/live2d/live2dcubismcore.min.js` and `/resources/live2d/reama-cubism-runtime.js?v=19`.
  - It forwards `setEmotion(emotion.toUpperCase())`, `setLipSync(v)` and `setSpokenChar(c)` whenever the matching prop changes.
  - When eye tracking is on, it listens to window `mousemove` and sets eyes to `clamp((x/innerWidth − .5)·2, ±0.7)` and `−clamp((y/innerHeight − .5)·2, ±0.7)`. When it is off, it calls `setEyeTracking(0, 0)`.
  - Runtime semantics: `setEmotion` uppercases and trims, and ignores repeats. A change **from `SLEEPING` to anything else** plays a startled `SURPRISED` burst for 2.5 s. `SLEEPING` makes her nod every 20–40 s. While `NORMAL`, idle and silent, there is a 15% chance of a sneeze every 45 s. `setLipSync` clamps to 0..1. `spokenChar` selects a vowel mouth opening while `lipSync > 0.01`.
- **`MiniGamePanel`** (L2524–2827) props: `{ visible; configLanguage; onClose; onEmotionChange(emotion); onSpeakValueChange(value) }`.
  - It is always mounted and hidden through opacity and pointer‑events.
  - While visible it pushes its Kurisu emotion (`SMILE` on the menu, `THINKING` while playing, `SMILE`/`ANGRY` per answer, a result emotion on the result screen) through `onEmotionChange`. While it is "speaking" a comment it pushes random mouth values every 120 ms through `onSpeakValueChange`, and 0 when it stops.
  - While visible, `Backspace`/`Escape` calls `onClose`.
  - It fires the achievements `game_play`, `game_win` and `game_perfect`. Details are in the mini‑game spec.

---

## 4. State inventory

### 4.1 React state (`useState`), in declaration order

| # | Name | Type | Initial | Written by | Read by |
|---|---|---|---|---|---|
| 1 | `chatState` | `"inputReady"\|"waitingAPI"\|"typing"\|"waitForAdvance"` | `"inputReady"` | submit, handlers, typewriter, onAdvance, auto, cancel, bridge, startup | everything |
| 2 | `currentEmotionTag` | string | `"NORMAL"` | resetIdleTimer (SLEEPING→NORMAL), idle tick (→SLEEPING), success/stream/complete (tag), error (ANGRY), dev cmds, onAdvance/auto/cancel (NORMAL), startup (event emotion), bridge (NORMAL) | Live2D prop, placeholder, `.sleeping-input`, idle tick; mirror ref used by stream complete |
| 3 | `currentSpokenChar` | string | `""` | typewriter (current char / `""`) | Live2D `spokenChar` (only while speaking) |
| 4 | `showAudioInput` | boolean | `AppSettings.getInt("Config_AudioInputEnabled", 0) === 1`, **read once at mount** | **never** (the setter is unused) | mic button visibility, `has-audio-input` class |
| 5 | `chatInput` | string | `""` | input onChange, submit (clear), STT result, cancel (clear) | input value, SEND disabled, Enter handler |
| 6 | `dialogueText` | string | `""` | typewriter, submit (`""`), success/error/startup/help (`""`) | `.dialogue-text` |
| 7 | `waitingIndicator` | string | `""` | dots interval, typewriter (`"▼"`/`""`), submit (`"."`), success/error/startup/help/onAdvance/auto/cancel (`""`) | `.waiting-indicator` |
| 8 | `waitingForInput` | boolean | `false` | typewriter pause (true), skip (false), onAdvance/auto (false), resets (false) | typewriter, auto, isActivelySpeaking, indicator |
| 9 | `skipTyping` | boolean | `false` | onAdvance (true), typewriter skip (false), resets (false) | typewriter (through its ref) |
| 10 | `conversationHistory` | `ChatMessage[]` | `[]`, then `[system]` at mount | mount, submit (updater), finishAssistantMessage, language effect, bridge import | submit (request), bridge export |
| 11 | `turnCount` | number | `0` | submit (+1), bridge import | updateSystemPrompt (dynamic context), bridge |
| 12 | `fullText` | string | `""` | success, stream token/complete, error, startup, help | typewriter, isActivelySpeaking |
| 13 | `typeIndex` | number (UTF‑16 index) | `0` | typewriter, resets | typewriter, isActivelySpeaking |
| 14 | `typewriterStartIndex` | number | `0` | typewriter (clear page), resets | typewriter |
| 15 | `typewriterClearPageOnResume` | boolean | `false` | typewriter pause, clear branch, resets | typewriter |
| 16 | `lastLoggedPageText` | string | `""` | typewriter, resets | typewriter (dedup) |
| 17 | `streamComplete` | boolean | `false` | submit (false), success/complete/error/startup/help (true) | typewriter |
| 18 | `ttsSessionId` | number | `0` | prepareLiveTTSSession (+1), cancelLiveTTS (+1) | processNext / handlers (see §6.3: effectively always 0) |
| 19 | `lipSyncValue` | number | `0` | lip effect (1/0) | Live2D prop fallback |
| 20 | `sttRecording` | boolean | `false` | STT state signal | mic button label/class, level bar |
| 21 | `sttRecordingLevel` | number 0..1 | `0` | STT progress signal, state signal (0) | level bar width |
| 22 | `sttStatusText` | string | `""` | toggle (`""`), STT ok/error | `.chat-stt-hint` |
| 23 | `miniGameOpen` | boolean | `false` | MINI GAME button (true), mini game onClose (false) | layout, Live2D props, keyboard guard |
| 24 | `miniGameEmotion` | string | `"NORMAL"` | MiniGamePanel `onEmotionChange` | Live2D emotion while the game is open |
| 25 | `miniSpeak` | number | `0` | MiniGamePanel `onSpeakValueChange` | Live2D lip sync while the game is open |
| 26 | `achievementToast` | `{title, description, visible}` | `{ "", "", false }` | unlock signal (visible true), 4 s timer (visible false) | toast DOM |

### 4.2 External‑store values

| Name | Source | Notes |
|---|---|---|
| `isMobileViewport` | `useIsMobileViewport()` | Only chooses between the mobile cancel button and the PC hint. |
| `eyeTrackingEnabled` | `useAppSettingInt("Config_EyeTracking", 0) === 1` | Reactive; passed to Live2DCanvas. |
| `chatLanguageSetting` | `useAppSettingInt("Config_ChatLanguage", 0)` | Reactive; triggers the system‑prompt refresh (E11). |

### 4.3 Refs (mutable, never trigger a render)

| Name | Initial | Purpose |
|---|---|---|
| `idleSecondsRef` | 0 | Seconds of idle, counted only in `inputReady`/`waitForAdvance`. |
| `sleepTimestampRef` | 0 | `Date.now()` when SLEEPING was entered, used for the 800 ms wake guard. **Not** updated by `/dev emotion sleeping`. |
| `lastMousePosRef` | `{x:-1,y:-1}` | Drops `mousemove` events whose coordinates did not change. |
| `streamRawBufferRef` | `""` | Accumulated raw stream tokens. Replaced by the post‑tag remainder once the tag has been parsed. |
| `streamEmotionParsedRef` | false | Whether the opening emotion tag of the stream has been decided. |
| `requestStartTimeRef` | 0 | `Date.now()` at submit, for the latency figure. |
| `ttsInFlightSessionIdRef` | -1 | Session id stamped when a synthesis starts; `-1` after a reset. |
| `ttsTextQueueRef` | `[]` | Text chunks waiting for synthesis. |
| `ttsAudioQueueRef` | `[]` | Object URLs waiting to be played. |
| `ttsSynthInFlightRef` | false | At most one synthesis runs at a time. |
| `ttsPlaybackBusyRef` | false | Audio is currently playing, or has been requested. |
| `ttsChunkBufferRef` | `""` | Streaming TTS text not yet cut into chunks. |
| `ttsProcessedIndexRef` | 0 | How much of the display text has already been fed to the TTS buffer. |
| `typingTimeoutRef` | null | Pending typewriter `setTimeout` id. |
| `lastLanguageKeyRef` | `""` | Last seen `"${configLanguage}:${chatLanguageSetting}"`. |
| `startupEventShownRef` | false | Once‑per‑mount guard for the startup greeting. |

### 4.4 Ref mirrors (L1061–1095), assigned on every render (`xRef.current = x`)

`typeIndexRef`, `fullTextRef`, `currentEmotionTagRef`, `conversationHistoryRef`, `turnCountRef`, `streamCompleteRef`, `waitingForInputRef`, `skipTypingRef`, `menuPanelOpenRef`, `typewriterStartIndexRef`, `typewriterClearPageOnResumeRef`, `lastLoggedPageTextRef`.

They give the timer‑driven typewriter `step()` and the bridge access to the **last rendered** values. A `setX()` inside `step()` is **not** visible through the ref until React re‑renders. V1 relies on the next timer tick (≥ 10 ms) coming after that re‑render; §9.6 has the Rust guidance.

### 4.5 Derived values (computed during render)

```ts
const isActivelySpeaking =
  chatState === "typing" && !waitingForInput &&
  ((typeIndex < fullText.length && fullText.length > 0) || ttsPlaybackBusyRef.current);
```
`ttsPlaybackBusyRef` is only sampled when something else causes a render.

---

## 5. Effects, in declaration order

React runs mount effects in declaration order, **after** the effects of the children (Live2DCanvas and MiniGamePanel effects run first). All of them run after the first commit.

| ID | Lines | Deps | Body | Cleanup |
|---|---|---|---|---|
| E1 | 860–888 | `[resetIdleTimer]` (stable, so mount‑once) | Adds **window** listeners for `mousemove`, `mousedown`, `keydown`, `touchstart` and `scroll`, all with the same handler. For `mousemove` the handler returns early if `clientX`/`clientY` equal `lastMousePosRef`; otherwise it stores them. It then calls `resetIdleTimer()`. No listener options are passed (bubble phase, default passive). | Removes all five. |
| E2 | 890–901 | `[chatState, currentEmotionTag]` | `setInterval(1000)`: if `chatState` is `inputReady` or `waitForAdvance`, `idleSecondsRef += 1`; then if `idle >= 60 && currentEmotionTag !== "SLEEPING"`, set `sleepTimestampRef = Date.now()` and `setCurrentEmotionTag("SLEEPING")`. The interval is **re‑created whenever the state or the emotion changes**, which drops the partial second. | `clearInterval`. |
| E3 | 903–905 | `[activeOverlay, menuPanelOpen, resetIdleTimer]` | `resetIdleTimer()`. On mount it is a no‑op. | none |
| E4 | 955–1047 | `[]` (eslint‑disabled) | **Mount init**, §5.1. | Disconnects all 13 handlers, then calls `STTService.stopRecording()` and `AIService.cancelStreaming()`. |
| E5 | 1049–1059 | `[chatState]` | Only while `waitingAPI`: `let dots = 1; setInterval(400): dots = (dots % 3) + 1; setWaitingIndicator(".".repeat(dots))`. Submit already showed `"."`, so the sequence is `"." → ".." (400 ms) → "..." (800) → "." (1200) → …`. | `clearInterval`. The indicator keeps its last value until something resets it. |
| E6 | 1099–1199 | `[chatState, menuPanelOpen, waitingForInput, onBackLogAdd]` | **Typewriter** (§9). Guard: if not `typing`, or the menu is open, or `waitingForInput`, it clears the pending timeout and returns. Otherwise it calls `step()` **synchronously**. | Clears the pending timeout. |
| E7 | 1201–1223 | `[autoMode, autoSpeed, chatState, menuPanelOpen, waitingForInput]` | **Auto mode** (§15). | `clearTimeout`. |
| E8 | 1225–1231 | `[isActivelySpeaking]` | `setLipSyncValue(isActivelySpeaking ? 1 : 0)`. | none |
| E9 | 1233–1238 | `[]` | `AchievementManager.achievementUnlocked.connect((_id, title, description) => setAchievementToast({ title, description, visible: true }))`. | Disconnects. |
| E10 | 1240–1248 | `[achievementToast]` (object identity) | If visible: after `setTimeout(4000)`, `setAchievementToast(prev => ({...prev, visible: false}))`. Each new unlock creates a new object, which **restarts** the 4 s. | `clearTimeout`. |
| E11 | 1273–1284 | `[chatLanguageSetting, configLanguage, updateSystemPrompt]` | `key = \`${configLanguage}:${chatLanguageSetting}\``. On the first run (`lastLanguageKeyRef === ""`) it stores the key and returns. If the key is unchanged it returns. Otherwise it stores the key and runs `setConversationHistory(prev => updateSystemPrompt(prev))`. `updateSystemPrompt` changes identity whenever `turnCount` changes, which re‑runs the effect; the key comparison makes that a no‑op. | none |
| E12 | 1288–1312 | `[]` | **Startup random event** (§17). | none |
| E13 | 1314–1328 | `[conversationRef]` (stable) | Installs the bridge (§23). | `conversationRef.current = null`. |
| E14 | 1832–1860 | `[chatInput, chatState, menuPanelOpen, miniGameOpen, onAdvance, submitMessage]` | **Global keyboard**, §19. It is re‑registered on almost every render, so in the window listener list it always sits **after** E1. | Removes the listener. |

### 5.1 E4 mount init (exact order)

1. Builds `history = [{ role: "system", content: getSystemPromptForLanguage(getCurrentChatLanguageIndex()) + (memoryContext ? "\n\n" + memoryContext : "") }]`, where `memoryContext = MemoryManager.getMemoryContext()`. **There is no dynamic context at mount.** Then calls `setConversationHistory(history)`.
2. Connects these handlers:
   - `AIService.responseReceived(response)`: `updateStatus(Date.now() - requestStartTimeRef.current)`, then `handleApiSuccess(response)`.
   - `AIService.streamToken(token)`: `handleStreamToken(token)`.
   - `AIService.streamComplete(response)`: `handleStreamComplete(response)`.
   - `AIService.errorOccurred(error)`: `handleApiError(error)`.
   - `TTSService.synthesisComplete(audioUrl)`: `if (ttsInFlightSessionIdRef.current !== ttsSessionId) return; ttsSynthInFlightRef = false; ttsAudioQueueRef.push(audioUrl); playNextTTSChunkIfPossible();`
   - `TTSService.synthesisError()`: same guard; then `ttsSynthInFlightRef = false; processNextTTSChunkIfPossible();`
   - `STTService.transcriptionComplete(text)`: `if (text.trim()) { setChatInput(text); setSttStatusText("音声入力: OK"); }`. The input receives the **untrimmed** text and replaces whatever was there.
   - `STTService.transcriptionError()`: `setSttStatusText("音声入力に失敗しました")`.
   - `STTService.recordingStateChanged()`: `setSttRecording(STTService.isRecording); if (!STTService.isRecording) setSttRecordingLevel(0);`
   - `STTService.recordingProgress(level)`: `setSttRecordingLevel(level)`.
   - `AudioService.playbackStarted()`: `ttsPlaybackBusyRef = true`.
   - `AudioService.playbackFinished()`: `ttsPlaybackBusyRef = false; playNextTTSChunkIfPossible();`
   - `AudioService.errorOccurred()`: `ttsPlaybackBusyRef = false; playNextTTSChunkIfPossible();`
3. Calls `updateStatus(0)`, so the parent receives `getLlmStatusFromSettings(0)` (latency 0).

**All handlers in step 2 are first‑render closures. §6 explains why that matters.**

---

## 6. CRITICAL: frozen (stale) closures and their effective V1 behavior

The E4 effect has `[]` deps, so every handler connected in §5.1 calls the **first render's** versions of `handleStreamToken`, `handleStreamComplete`, `handleApiSuccess`, `handleApiError` and `updateStatus`, together with everything those close over. In the first render `chatState === "inputReady"`, `fullText === ""` and `ttsSessionId === 0`. Leptos signals are always live, so **a straight port would silently change behavior**. Here is the exact effect of each frozen closure.

### 6.1 Streaming never enters `typing` (V1 defect)

`handleStreamToken` and `handleStreamComplete` both contain `if (chatState === "waitingAPI") setChatState("typing");`, but the frozen `chatState` is `"inputReady"`. So with providers 1/3/4/5/6:

- After submit the state is `waitingAPI` (dots, plus the PC hint or the mobile cancel button), and **it stays there**.
- Tokens are still processed:
  - `fullText` is updated, but it is invisible because the typewriter only runs in `typing`.
  - The emotion tag is applied as soon as it is parsed, so Live2D changes expression while the dots animate, and `MemoryManager.recordEmotion` runs.
  - TTS chunks are synthesized and **played** (if enabled), with the mouth closed, because `isActivelySpeaking` requires `typing`.
- On completion:
  - Memory commands are applied.
  - The fallback emotion parse runs if needed.
  - The `emotion` and `chat` achievements fire.
  - The assistant message is appended to the history.
  - `streamComplete = true`.
  - The UI **remains** in `waitingAPI`.
- The user's only way out is `Ctrl+C` (PC) or the `キャンセル` button (mobile). Either one leads to `inputReady` with emotion NORMAL. `Enter` and clicking the dialogue do nothing, because `onAdvance` ignores `waitingAPI`.
- The answer text is never displayed and **never logged to the backlog**. It is in the history, so the next request includes it.
- A stream error still works: `handleApiError` does not depend on `chatState`.

### 6.2 The latency figure while streaming

`if (!streamEmotionParsedRef.current && fullText.length === 0) updateStatus(now − start)`: the frozen `fullText.length` is always 0, so V1 sends a status update **on every token until the emotion decision is made**, and the last one wins. No status update happens on `streamComplete` or on errors.

### 6.3 The TTS session guard compares against a constant 0

Every TTS function runs inside the first‑render closure chain:

```
handleApiSuccess / handleStreamToken / handleStreamComplete / signal handlers
  → triggerTTSIfEnabled / captureStreamTextForTTS / flushTTSChunkBuffer
  → enqueueTTSChunk / playNextTTSChunkIfPossible
  → processNextTTSChunkIfPossible
```

So `processNextTTSChunkIfPossible` always stamps `ttsInFlightSessionIdRef = 0`, and the synthesis handlers compare against `0`. In practice a synthesis result is **dropped only if `prepareLiveTTSSession`/`cancelLiveTTS` reset the stamp to `-1` after it started and no new synthesis started in between**.

**Race (V1 bug):**
1. A new message resets the TTS session and immediately starts synthesis A′ (stamp 0).
2. The old synthesis A then completes. It is accepted as if it were current, and the stale audio plays.
3. A's completion also clears `ttsSynthInFlightRef`, so synthesis B′ can start while A′ is still running.

The `ttsSessionId` state itself is incremented, but only re‑renders.

### 6.4 Decision record: parity versus intended behavior

- **Option A (strict parity):** reproduce §6.1–6.3 exactly. Never change `chatState` inside the stream handlers; report latency on every token until the emotion is parsed; keep the constant‑0 guard.
- **Option B (recommended): implement the intended behavior,** which is what the code reads as if state were live. Only the streaming path changes:
  1. `on_stream_token`: after `full_text = display`, if the **live** `chat_state == WaitingApi`, set it to `Typing`. Optional hardening: only transition once `stream_emotion_parsed && !display.is_empty()`. That keeps a half‑received tag such as `"[SMI"` from being typed and later rewritten.
  2. `on_stream_complete`: if the live state is `WaitingApi`, set it to `Typing`.
  3. The typewriter skip branch (§9.3 step 3) **must** also change. Otherwise a skip while the stream is still running would either stall forever (no timer is rescheduled when the old `typeIndex < len`) or jump to `waitForAdvance` and drop the rest of the stream (when the old `typeIndex >= len`). The fix: after revealing, if `!stream_complete`, keep `chatState = Typing` and schedule `step` in 50 ms; go to `waitForAdvance` only when `stream_complete`.
  4. TTS guard: use a real generation counter (`generation += 1` on prepare/cancel; stamp each synthesis with the current generation; drop a result if `stamp != generation`).
  5. TTS tail flush on completion: always force‑flush (§11.5 bug 1).
  6. Latency: either keep "every token until parsed" or use the live `full_text.is_empty()`. This is cosmetic.

---

## 7. Chat state machine

```
                ┌──────────────── mount: startup greeting (always) ───────────────┐
                ▼                                                                  │
          ┌──────────┐ submit(normal) ┌────────────┐  responseReceived / error   ┌────────┐
 ┌───────▶│inputReady│───────────────▶│ waitingAPI │────────────────────────────▶│ typing │◀─┐
 │        └──────────┘                └────────────┘  (stream: see §6)           └────────┘  │
 │          ▲   │ /dev emotion help ───────────────────────────────────────────────▲  │  │     │ waitingForInput
 │          │   │                                                                    │  │  │     │ (pause ▼) ⇄ advance/auto
 │          │   └──── Ctrl+C / mobile キャンセル (from waitingAPI) ◀── (waitingAPI)  │  │  └─────┘
 │          │                                                                        │  ▼
 │          │  onAdvance / auto timer                ┌────────────────┐ all typed &   │
 │          └────────────────────────────────────────│ waitForAdvance │◀ streamComplete / skip
 │                                                   └────────────────┘
 └── bridge.applyImported (from ANY state) → inputReady
     responseReceived / errorOccurred arrive in ANY state → typing (late responses are displayed)
```

| Trigger | Precondition | State effect | Emotion effect |
|---|---|---|---|
| mount (E12) | event text non‑empty (always) | → `typing` (greeting) | event emotion |
| submit, normal text | `inputReady` | → `waitingAPI` | unchanged |
| submit `/dev emotion help` | `inputReady` | → `typing` (help text) | unchanged |
| submit other dev command | `inputReady` | stays `inputReady` | see §8.20 |
| `responseReceived` | any | → `typing` | parsed tag |
| `streamToken` / `streamComplete` | — | V1: none (§6.1) / Option B: `waitingAPI` → `typing` | tag when parsed |
| `errorOccurred` | any | → `typing` | `ANGRY` |
| typewriter end | `typing`, all typed, `streamComplete` | → `waitForAdvance` | unchanged |
| typewriter skip | `typing`, `streamComplete` or old `typeIndex ≥ len` | → `waitForAdvance` | unchanged |
| `onAdvance` | `typing` | none: `waitingForInput ? false : skipTyping = true` | unchanged |
| `onAdvance` / auto timer | `waitForAdvance` | → `inputReady` | `NORMAL` |
| auto timer | `typing && waitingForInput` | `waitingForInput = false` | unchanged |
| `Ctrl+C` / mobile cancel | `waitingAPI` | → `inputReady` | `NORMAL` |
| `bridge.applyImported` | any | → `inputReady` | `NORMAL` |
| idle 60 s | `inputReady`/`waitForAdvance` | none | `SLEEPING` |
| activity ≥ 800 ms after sleep | emotion `SLEEPING` | none | `NORMAL` |

---

## 8. Callbacks: detailed algorithms

### 8.1 `resetIdleTimer()` (L849–858, stable)
1. If `idleSecondsRef > 0`, set it to 0.
2. If `Date.now() − sleepTimestampRef < 800`, return. Activity within 800 ms of falling asleep resets the counter but does **not** wake her.
3. `setCurrentEmotionTag(prev => prev === "SLEEPING" ? "NORMAL" : prev)`.

### 8.2 `updateSystemPrompt(history): ChatMessage[]` (L1250–1268, deps `[turnCount]`)
1. `content = getSystemPromptForLanguage(getCurrentChatLanguageIndex())`.
2. `mem = MemoryManager.getMemoryContext()`; if it is non‑empty, `content += "\n\n" + mem`.
3. `dyn = MemoryManager.getDynamicContext(turnCount)`, using the **render's** `turnCount`, which is the count *before* the submit's `+1`. So the first message says `会話ターン数: 0`. If non‑empty, `content += "\n\n" + dyn`.
4. `next = [...history]`; **only if** `next[0]?.role === "system"`, replace `next[0]` with `{role:"system", content}`. Otherwise it returns the copy unchanged. **No system message is ever inserted** (see Open question #6, about import).
5. Return `next`. It is a pure function apart from `Math.random()` and the clock.

### 8.3 `updateStatus(latencyMs)` (L1330–1335)
`onLlmStatusUpdate(getLlmStatusFromSettings(latencyMs))`.

### 8.4 `prepareLiveTTSSession()` (L1337–1348, deps `[ttsSessionId]`), called by every normal submit
`setTtsSessionId(ttsSessionId + 1)`; `ttsInFlightSessionIdRef = -1`; empty both queues; `ttsSynthInFlightRef = false`; `ttsPlaybackBusyRef = false`; `ttsChunkBufferRef = ""`; `ttsProcessedIndexRef = 0`; `AudioService.stop()`. It runs even when TTS is disabled; that is harmless.

### 8.5 `cancelLiveTTS()` (L1350–1360, stable), called only by `handleApiError`
Same as 8.4, but with `setTtsSessionId(v => v + 1)`.

### 8.6 `processNextTTSChunkIfPossible()` (L1362–1373)
1. If `!TTSService.enabled || ttsSynthInFlightRef || textQueue.length === 0`, return.
2. `next = textQueue.shift()`; if `!next?.trim()`, return. This does not loop on to the next item, but it is unreachable, because only trimmed, non‑empty strings are enqueued.
3. `ttsSynthInFlightRef = true`; `ttsInFlightSessionIdRef = ttsSessionId` (effectively 0, §6.3); `void TTSService.synthesize(next)`.

   ⚠ On a cache hit, `synthesize` emits `synthesisComplete` **synchronously**, so the complete handler, then `playNext`, then possibly `processNext` again all run **re‑entrantly inside this call**.

### 8.7 `playNextTTSChunkIfPossible()` (L1375–1390)
1. If `ttsPlaybackBusyRef`, return.
2. If the audio queue is empty, call `processNextTTSChunkIfPossible()` and return.
3. `url = audioQueue.shift()`; if it is falsy, return.
4. `ttsPlaybackBusyRef = true`; `AudioService.playFromFile(url)`; then `processNextTTSChunkIfPossible()`, which pipelines: the next chunk is synthesized while this one plays.

### 8.8 `enqueueTTSChunk(text)` (L1392–1402)
`cleaned = text.trim()`; if it is empty, return; otherwise push it and call `processNextTTSChunkIfPossible()`. It does not call `playNext`; the synthesis‑complete handler does that.

### 8.9 `splitTextForTTS(text): string[]` (L1404–1426, stable). Verbatim:
```ts
const normalized = text.replace(/\s+/g, " ").trim();
if (!normalized) return [];
const chunks: string[] = []; let current = "";
for (const char of normalized) {                  // iterates code points
  current += char;
  const isBoundary = ["。", "！", "？", "!", "?", "\n"].includes(char);   // "\n" impossible after normalization
  if (isBoundary || current.length >= 120) {      // .length = UTF-16 units
    const chunk = current.trim(); if (chunk) chunks.push(chunk); current = "";
  }
}
if (current.trim()) chunks.push(current.trim());
return chunks;
```
`"."` is **not** a boundary. An English text made of plain sentences is cut only every 120 UTF‑16 units.

### 8.10 `triggerTTSIfEnabled(text, immediate)` (L1428–1447)
If TTS is disabled, return. `cleaned = stripEmotionTag(text)`; if it is empty, return. When `immediate`, call `enqueueTTSChunk(p)` for each `p` in `splitTextForTTS(cleaned)`; otherwise call `enqueueTTSChunk(cleaned)`. The only call site is `handleApiSuccess(display, true)`.

### 8.11 `flushTTSChunkBuffer(forceFlush)` (L1449–1487). Verbatim:
```ts
while (true) {
  let boundary = -1;
  for (let i = 0; i < buf.length; i += 1) {
    const char = buf[i];                                         // UTF-16 unit
    if (["。", "！", "？", "!", "?", "\n"].includes(char)) {
      boundary = i + 1;
      while (boundary < buf.length) {
        const nextChar = buf[boundary];
        if (["」", "』", "”", '"', ")", "）"].includes(nextChar)) boundary += 1; else break;
      }
      break;
    }
  }
  if (boundary < 0) break;
  const chunk = buf.slice(0, boundary).trim();
  buf = buf.slice(boundary).trimStart();
  if (chunk) enqueueTTSChunk(chunk);
}
if (forceFlush) { const tail = buf.trim(); buf = ""; if (tail) enqueueTTSChunk(tail); }
```
Here `buf` is `ttsChunkBufferRef.current`. There is no 120‑unit limit and no whitespace normalization.

### 8.12 `captureStreamTextForTTS(sourceText, complete)` (L1489–1509). Verbatim logic:
```ts
if (!TTSService.enabled) return;
if (ttsProcessedIndexRef.current > sourceText.length) ttsProcessedIndexRef.current = sourceText.length;
const delta = sourceText.slice(ttsProcessedIndexRef.current);
if (!delta) return;                              // ⚠ returns BEFORE the force flush even when complete === true
ttsProcessedIndexRef.current = sourceText.length;
ttsChunkBufferRef.current += delta.replace(/\[(NORMAL|SMILE|ANGRY|SAD|SURPRISED|BLUSH|WINK|DISGUST|SMUG|THINKING|PANIC)\]/gi, "");
flushTTSChunkBuffer(complete);
```
The delta assumes the display text only ever grows by appending. When it shrinks (for example, memory commands removed at completion, or `\n{3,}` collapsed), the index is clamped and nothing happens. When it changes in the middle (a tag completes, a think block closes), the delta can skip or repeat characters. Reproduce this algorithmically.

### 8.13 `applyMemoryCommands(text): string` (L1511–1524, stable)
`parsed = extractMemoryCommands(text)`. For each fact: `MemoryManager.addUserFact(fact)`, then `name = extractUserNameFromFact(fact)`; if a name was found, `MemoryManager.setUserName(name)`. For each event: `MemoryManager.addEpisodicMemory(event.summary, event.tags)`. Return `parsed.cleanedText`.

### 8.14 `finishAssistantMessage(displayText)` (L1526–1531)
`setConversationHistory(prev => [...prev, { role: "assistant", content: displayText }])`. The history therefore stores the **display** text: emotion tags, `<think>` blocks and memory commands are removed.

### 8.15 `handleApiSuccess(response)`, for non‑streaming responses (L1533–1563)
1. `noThink = response.replace(/<think>[\s\S]*?<\/think>/gi, "").trim()`.
2. `cleaned = applyMemoryCommands(noThink)`; if it is empty, `cleaned = "……"` (two U+2026).
3. `tag = parseEmotionTag(cleaned)` (first tag anywhere, else `NORMAL`); `setCurrentEmotionTag(tag)`; `MemoryManager.recordEmotion(tag)`.
4. `AchievementManager.notifyEvent("emotion", { emotion: tag })`, then `notifyEvent("chat", { totalInteractions: MemoryManager.totalInteractions })`. That value already includes this exchange (§10.1).
5. `display = stripEmotionTag(cleaned) || "……"`.
6. `finishAssistantMessage(display)`; `setFullText(display)`; reset the typewriter: `typeIndex = 0`, `typewriterStartIndex = 0`, `skipTyping = false`, `waitingForInput = false`, `typewriterClearPageOnResume = false`, `lastLoggedPageText = ""`, `dialogueText = ""`, `waitingIndicator = ""`; `streamComplete = true`; `chatState = "typing"`. This is the same reset block as error, startup and help ("**typewriter reset block**").
7. `triggerTTSIfEnabled(display, true)`.

The handler before it has already called `updateStatus(Date.now() − requestStartTimeRef)`.

### 8.16 `handleStreamToken(token)` (L1565–1617). Verbatim, with V1‑effective values annotated:
```ts
streamRawBufferRef.current += token;
if (!streamEmotionParsedRef.current && fullText.length === 0) {     // V1: fullText.length frozen at 0
  updateStatus(Date.now() - requestStartTimeRef.current);
}
let raw = streamRawBufferRef.current.replace(/<think>[\s\S]*?<\/think>/gi, "").trim();
if (!streamEmotionParsedRef.current) {
  if (raw.startsWith("[")) {
    const close = raw.indexOf("]");
    if (close > 0) {
      const maybeTag = raw.slice(1, close).trim().toUpperCase();
      if (["NORMAL","SMILE","ANGRY","SAD","SURPRISED","BLUSH","WINK","DISGUST","SMUG","THINKING","PANIC"].includes(maybeTag)) {
        setCurrentEmotionTag(maybeTag);
        MemoryManager.recordEmotion(maybeTag);
        raw = raw.slice(close + 1).trimStart();
        streamRawBufferRef.current = raw;          // buffer REPLACED by think-stripped, trimmed, tag-less remainder
        streamEmotionParsedRef.current = true;
      } else if (raw.length > 2 && !raw.startsWith("<")) {
        streamEmotionParsedRef.current = true;     // e.g. "[SAVE_FACT: …]" first → decided: no tag
      }
    }                                              // no "]" yet → undecided, wait
  } else if (raw.length > 2 && !raw.startsWith("<")) {
    streamEmotionParsedRef.current = true;         // plain text first → decided: no tag
  }                                                // starts with "<" (unclosed <think>) → wait
}
const display = stripEmotionTag(raw);
setFullText(display);
if (chatState === "waitingAPI") setChatState("typing");     // V1: never true (§6.1)
captureStreamTextForTTS(display, false);
```
Notes:
- Only a tag at the **very start** of the stream sets the emotion while streaming, and `[ normal ]` (with inner whitespace and any case) is accepted. Tags later in the text are only stripped from the display.
- No `emotion` achievement fires here; it fires at completion.

### 8.17 `handleStreamComplete(fullResponse)` (L1619–1652)
1. `raw = fullResponse.trim()`; if it is empty, `raw = streamRawBufferRef.trim()`.
2. `cleaned = applyMemoryCommands(raw)`. Memory commands are extracted from the full raw text, **including** any inside `<think>`; this differs from the non‑streaming path. Then `cleaned = stripEmotionTag(cleaned)`; if it is empty, `cleaned = "……"`.
3. `emotionForEvent = currentEmotionTagRef.current`. If `!streamEmotionParsedRef`: `tag = parseEmotionTag(raw)`, `emotionForEvent = tag`, `setCurrentEmotionTag(tag)`, `MemoryManager.recordEmotion(tag)`.
4. `notifyEvent("emotion", { emotion: emotionForEvent })`; `notifyEvent("chat", { totalInteractions })`.
5. `setFullText(cleaned)`; `captureStreamTextForTTS(cleaned, true)`; `finishAssistantMessage(cleaned)`; `setStreamComplete(true)`; `if (chatState === "waitingAPI") setChatState("typing")` (V1: never); `streamEmotionParsedRef = false`; `streamRawBufferRef = ""`.
   - The typewriter fields are **not** reset here (unlike success/error), because typing would already be under way in the intended design.
   - There is no `updateStatus` call.

### 8.18 `handleApiError(error)` (L1654–1696)
1. Map `error` to a friendly message (first match wins; `includes` is case‑sensitive):

| # | Condition (`error.includes(...)`) | Message (verbatim) |
|---|---|---|
| 1 | `"API Key"` ∨ `"API key"` ∨ `"設定されていません"` | `……APIキーが設定されてないみたいよ。CONFIGから設定してちょうだい。` |
| 2 | `"401"` ∨ `"Unauthorized"` | `APIキーが無効みたい……もう一度確認して設定し直してくれる？` |
| 3 | `"429"` ∨ `"rate limit"` ∨ `"RESOURCE_EXHAUSTED"` | `リクエストが多すぎるみたい。少し待ってからもう一度試してくれない？` |
| 4 | `"insufficient_quota"` ∨ `"quota_project"` | `Geminiの利用枠か課金設定に問題があるみたい。APIキーとプロジェクト設定を確認して。` |
| 5 | `"timeout"` ∨ `"Timeout"` | `応答がタイムアウトしたわ……ネットワークの状態を確認してみて。` |
| 6 | `"Cannot connect"` ∨ `"Network"` ∨ `"ネットワーク"` | `ネットワークに接続できないわ。インターネット接続を確認してちょうだい。` |
| 7 | `"403"` ∨ `"Forbidden"` | `このAPIへのアクセスが拒否されたわ。権限を確認してみて。` |
| 8 | `"500"` ∨ `"502"` ∨ `"503"` | `サーバー側でエラーが起きてるみたい。しばらくしてからもう一度試して。` |
| 9 | `"model"` ∨ `"Model"` | `指定されたモデルが見つからないみたい。CONFIGからモデル名を確認して。` |
| 10 | `"JSON"` | `サービスアカウントJSONキーの形式が正しいか確認して。` |
| 11 | `"Vertex"` ∨ `"アクセストークン"` | `Vertexの認証情報を確認して。APIキーかサービスアカウントJSONキーが必要よ。` |
| — | default | `何かエラーが起きたみたい……もう一度試してくれる？` |

2. `setCurrentEmotionTag("ANGRY")`; `setFullText(message)`; typewriter reset block; `streamComplete = true`; `chatState = "typing"`; `cancelLiveTTS()`. The error text is never spoken.
   - Nothing is added to the history, so the history keeps a dangling user message and the next request contains two consecutive user turns.
   - `MemoryManager.recordEmotion` is not called and no achievement fires.
   - The error text reaches the backlog through the typewriter.

Test vectors:
- `"API Key が設定されていません。CONFIGから設定してください。"` → #1.
- `"FreeAPIの全体リクエスト上限（65000回）に達しました。サービスを停止しています。"` → **default**.
- `"リクエストが多すぎます。1分ほど待ってから再度お試しください。"` → **default** (no "429" substring).
- `"Request failed (502)"` → #8.
- `"Vertex AI: サービスアカウントJSONキーの形式が不正です。"` → #10 ("JSON" is checked before "Vertex").
- Firefox `"NetworkError when attempting to fetch resource."` → #6.
- Chrome `"Failed to fetch"` → default.
- `"404 Not Found\n{… \"model\" …}"` → #9.

### 8.19 `submitMessage(raw)` (L1698–1799, deps `[chatState, conversationHistory, onBackLogAdd, prepareLiveTTSSession, updateSystemPrompt]`)
1. `text = raw.trim()`; if it is empty or `chatState !== "inputReady"`, return.
2. Dev commands: §8.20. They match **exactly** and case‑sensitively on the trimmed text, and return early.
3. Normal message:
   1. `userMessage = { role: "user", content: text }`.
   2. `setChatInput("")`.
   3. `setTurnCount(v => v + 1)`.
   4. `setConversationHistory(prev => { const next = [...prev, userMessage]; MemoryManager.trimConversationHistory(next, 30); MemoryManager.recordInteraction(); return updateSystemPrompt(next); })`.
      - ⚠ This updater has side effects. In production React it runs once, during the next render, which is after step 12 has already started the HTTP request. In dev StrictMode it runs twice. **Rust: do the trim and `recordInteraction()` exactly once, synchronously, in submit.**
   5. `onBackLogAdd("user", text)`.
   6. `requestStartTimeRef = Date.now()`; `streamRawBufferRef = ""`; `streamEmotionParsedRef = false`.
   7. `setStreamComplete(false)`; `setDialogueText("")`; `setWaitingIndicator(".")`; `setChatState("waitingAPI")`.
   8. `prepareLiveTTSSession()`.
   9. `provider = AppSettings.getInt("Config_ApiProvider", 0)`. The default here is **0**, while everywhere else it is 8. That is harmless: both are non‑streaming, and the key always exists after `initializeDefaults`.
   10. `nextMessages = updateSystemPrompt([...conversationHistory, userMessage])`.
       - This uses the render's **untrimmed** history, so the request can carry up to 31 non‑system messages at the moment the stored history gets trimmed.
       - It calls `updateSystemPrompt` a second time, so the request's random mood hint and seconds can differ from the stored copy. The stored copy is never sent or exported, so building the prompt once for both is behaviorally equivalent.
   11. If `provider ∈ {1, 3, 4, 5, 6}`, call `void AIService.sendChatStreaming(nextMessages)`.
   12. Otherwise call `void AIService.sendChat(nextMessages)`.

### 8.20 Dev commands (checked in this order, on the trimmed text)

| Command (exact) | Actions | Achievement |
|---|---|---|
| `/dev emotion help` | `setChatInput("")`; `setFullText(HELP)`; typewriter reset block; `streamComplete = true`; `chatState = "typing"` | none |
| text `startsWith("/dev emotion ")` | `parts = text.split(" ")` (single‑space split, so double spaces give empty parts). If `parts.length >= 3`: `m = parts[2]?.trim().toUpperCase()`; if `m` is non‑empty, `setCurrentEmotionTag(m)` (any string is accepted, e.g. `SNEEZE`, `SLEEPING` or garbage). Then `setChatInput("")`. | `notifyEvent("dev_command")` |
| `/dev motion reset` | `setCurrentEmotionTag("NORMAL")`; `setChatInput("")` | `dev_command` |
| `/dev achievements reset` | `setChatInput("")`; `AchievementManager.resetAll()` | `dev_command`. This re‑unlocks `dev_user` immediately and shows the "開発者" toast. |
| `/dev steiner` | `setChatInput("")` | `notifyEvent("secret_steiner")` |

Dev commands never reach the backlog, the history, `turnCount` or the API.

`HELP` (verbatim; `\n` are real newlines):
```
【サポートされているモーションID一覧】
・NORMAL (通常)
・SMILE (笑顔)
・ANGRY (怒り)
・SAD (悲しみ)
・SURPRISED (驚き)
・BLUSH (照れ)
・WINK (ウィンク)
・DISGUST (嫌悪)
・SMUG (どや顔)
・THINKING (考え中)
・PANIC (混乱)
・SLEEPING (居眠り)
・SNEEZE (クシャミ)

※コマンド例: /dev emotion smile で対象のモーションに変更します。
```
JS literal: `"【サポートされているモーションID一覧】\n・NORMAL (通常)\n・SMILE (笑顔)\n・ANGRY (怒り)\n・SAD (悲しみ)\n・SURPRISED (驚き)\n・BLUSH (照れ)\n・WINK (ウィンク)\n・DISGUST (嫌悪)\n・SMUG (どや顔)\n・THINKING (考え中)\n・PANIC (混乱)\n・SLEEPING (居眠り)\n・SNEEZE (クシャミ)\n\n※コマンド例: /dev emotion smile で対象のモーションに変更します。"`.

Because `"\n"` is a pause character, the help text pauses after **every line**. That is 15 pages, plus the final full‑text backlog entry and a double `▼` (§9.4).

Quirk: when you type `/dev emotion sleeping` and press **Enter**, the same keydown event then reaches the window activity listener (E1). `resetIdleTimer` sees an old `sleepTimestampRef`, so she wakes to `NORMAL` right away. Live2D receives `SLEEPING` and then `NORMAL`, which plays the startled wake‑up. When the command is sent with the mobile SEND button, `SLEEPING` sticks until the next activity event.

### 8.21 `onAdvance()` (L1801–1815, deps `[chatState, waitingForInput]`)
- In `typing`: if `waitingForInput`, set `setWaitingForInput(false)` (continue to the next page); otherwise set `setSkipTyping(true)` (reveal the rest). Return.
- In `waitForAdvance`: `setChatState("inputReady")`; `setCurrentEmotionTag("NORMAL")`; `setWaitingIndicator("")`.
- In `waitingAPI` or `inputReady`: no‑op.

### 8.22 `toggleSttRecording()` (L1817–1830, deps `[chatState]`)
1. If `chatState !== "inputReady"`, return.
2. `setSttStatusText("")`.
3. If `!STTService.enabled`, set `STTService.enabled = true`. ⚠ This **persists** `Config_Experimental_STT_Enabled = 1` and starts a health check.
4. If already recording: `STTService.stopRecording()`; return.
5. Otherwise `void STTService.startRecording()`.

There is no auto‑submit: the transcript only fills the input (§13).

---

## 9. Typewriter engine (E6). Exact algorithm

### 9.1 Concepts
- **Page:** the slice `fullText[typewriterStartIndex .. typeIndex]` currently shown in `.dialogue-text`.
- **Pause character:** `isPauseCharacter(char, nextChar)` (§3.9). When the character just typed is a pause character, typing stops: `waitingForInput = true` and `▼` is shown.
- **Clear on resume:** after a pause that is not the end of the text (or while the stream is incomplete), the next resume starts a new page at the current `typeIndex`.
- **Indices are UTF‑16 code units** (`charAt`, `.length`, `.slice`). See Open question #11.

### 9.2 Verbatim `step()` (L1108–1189)
```ts
const step = () => {
  if (menuPanelOpenRef.current || waitingForInputRef.current) return;

  if (typewriterClearPageOnResumeRef.current) {
    setTypewriterStartIndex(typeIndexRef.current);
    setDialogueText("");
    setLastLoggedPageText("");
    setTypewriterClearPageOnResume(false);
    setWaitingIndicator("");
    typingTimeoutRef.current = window.setTimeout(step, 10);
    return;
  }

  if (skipTypingRef.current) {
    const finished = fullTextRef.current.slice(typewriterStartIndexRef.current);
    setDialogueText(finished);
    setTypeIndex(fullTextRef.current.length);
    setSkipTyping(false);
    setWaitingForInput(false);
    if (finished.trim() && finished !== lastLoggedPageTextRef.current) {
      onBackLogAdd("assistant", finished);
      setLastLoggedPageText(finished);
    }
    if (streamCompleteRef.current || typeIndexRef.current >= fullTextRef.current.length) {   // typeIndexRef = OLD value
      setChatState("waitForAdvance");
      setWaitingIndicator("▼");
      setCurrentSpokenChar("");
    }
    return;                                                    // no reschedule (see §6.4 item 3)
  }

  const curIndex = typeIndexRef.current;
  const targetText = fullTextRef.current;

  if (curIndex < targetText.length) {
    let nextIndex = curIndex;
    if (curIndex === typewriterStartIndexRef.current) {         // at page start: skip leading whitespace
      while (nextIndex < targetText.length) {
        const current = targetText.charAt(nextIndex);
        if (![" ", "\n", "\r", "\t"].includes(current)) break;
        nextIndex += 1;
      }
    }
    const char = targetText.charAt(nextIndex);                  // "" if whitespace ran to the end
    setCurrentSpokenChar(char);
    nextIndex += 1;
    const pageText = targetText.slice(typewriterStartIndexRef.current, nextIndex);   // NOTE: includes the skipped leading whitespace
    setTypeIndex(nextIndex);
    setDialogueText(pageText);

    const nextChar = nextIndex < targetText.length ? targetText.charAt(nextIndex) : "";
    if (isPauseCharacter(char, nextChar)) {
      setWaitingForInput(true);
      setCurrentSpokenChar("");
      setTypewriterClearPageOnResume(nextIndex < targetText.length || !streamCompleteRef.current);
      if (pageText.trim()) {
        onBackLogAdd("assistant", pageText);
        setLastLoggedPageText(pageText);
      }
      setWaitingIndicator("▼");
      return;
    }

    const textSpeed = AppSettings.getFloat("Config_TextSpeed", 1.0);     // read on EVERY character
    const delay = charDelayForChar(char, textSpeed);
    typingTimeoutRef.current = window.setTimeout(step, delay);
  } else if (!streamCompleteRef.current) {
    typingTimeoutRef.current = window.setTimeout(step, 50);              // wait for more stream text
  } else {
    setCurrentSpokenChar("");
    if (lastLoggedPageTextRef.current !== targetText && targetText.trim()) {
      onBackLogAdd("assistant", targetText);                            // ⚠ FULL text, not the page
      setLastLoggedPageText(targetText);
    }
    setChatState("waitForAdvance");
    setWaitingIndicator("▼");
  }
};
```
The effect calls `step()` immediately whenever it (re)starts with `chatState === "typing"`, the menu closed and `!waitingForInput`. This covers entering `typing`, resuming after a pause, and closing the menu mid‑typing. Resuming after closing the menu does **not** wait out the remaining character delay.

### 9.3 Step branches, summarized
1. **Guard:** if the menu is open or typing is paused, stop without rescheduling.
2. **Clear page:** the new page starts at `typeIndex`; clear the box and the dedup marker; hide `▼`; the next step runs in **10 ms**.
3. **Skip:** reveal from the page start to the end; `typeIndex = len`. Log the revealed text unless it equals the last logged page. Go to `waitForAdvance ▼` if the stream is complete or the old index ≥ len. **Never reschedule.**
4. **Type one character:** at the page start, skip leading whitespace; show the character; then either pause (log the page) or schedule the next step after `charDelayForChar(char, Config_TextSpeed)`.
5. **Caught up, stream incomplete:** poll every **50 ms**.
6. **Caught up, stream complete:** log the full text if it differs from the last logged page; go to `waitForAdvance ▼`; clear the spoken character.

### 9.4 Worked examples (use them as unit tests; speed 1.0, so base = 50 ms)

**A. `"そうね。あなたは？"`** (len 9, complete):
- `そ`, `う`, `ね` are typed 50 ms apart.
- `。` (index 3): next is `あ`, so it pauses. Page `"そうね。"` is logged; `clearOnResume = true`; `▼`.
- Click: the clear branch sets `start = 4` and the box empties; 10 ms later typing continues with `あ…？`.
- `？` is followed by `""`, so it pauses again. Page `"あなたは？"` is logged; `clearOnResume = (9<9 || !complete) = false`; `▼`.
- Click: nothing to clear, `curIndex == len`, so the end branch runs. `lastLogged "あなたは？" ≠ full`, so **`"そうね。あなたは？"` is logged again**, then `waitForAdvance ▼`. The box still shows `"あなたは？"`.
- Click: `inputReady`, `NORMAL`.
- The backlog shows 3 assistant entries: `そうね。`, `あなたは？`, `そうね。あなたは？`.
- Two clicks are needed at the end with no visible change between them (the **double ▼**). In auto mode that means two `autoSpeed` waits at the end.

**B. `"了解"`** (no terminal punctuation): `了`, then `解` (followed by `""`, not a pause), then 50 ms later the end branch logs `"了解"` and shows `waitForAdvance ▼`. One click returns to input. One backlog entry.

**C. `"Hi! How are you?"`**: `!` is followed by a space, so it pauses with page `"Hi!"`. Resume: `start = 3`; the space is skipped for the *index*, but the page is sliced from `start`, so the box shows `" How are you?"` **with a leading space** (`white-space: pre-wrap`). At the end the full text is logged again.

**D. `"A。\n\nB"`**: it pauses at `。` (next is `\n`, not a closing bracket). Resume: `start = 2`; both `\n` are skipped; `B` is typed; the box shows `"\n\nB"`, so **two blank lines appear first**. The end branch logs `"A。\n\nB"`; the parent's `addBackLog` trims it.

**E. `"「そうよ。」って言ったの。"`**: `。` is followed by `」`, so it does not pause. `」` waits 75 ms (×1.5). The final `。` pauses; then a double `▼`.

**F. `"えっと……そうね"`**: each `…` waits 125 ms (×2.5).

**G. English periods:**
- `"e.g. this"`: pauses after `e.g.` (`.` followed by a space).
- `"3.14"`: no pause.
- `"Wait..."`: only the last `.` pauses (followed by `""`).

**H. Skip:** clicking at index 2 of example A sets `skipTyping`. On the next tick the box shows the whole `"そうね。あなたは？"`, it is logged once, and the state goes to `waitForAdvance ▼` with no further pauses. Skipping during page 2 shows and logs only `"あなたは？"` (no end‑branch duplicate).

**I. A newline inside text:** `"A\nB"` pauses at `\n` with page `"A\n"`. The second page is `"B"`. At the end `"A\nB"` is logged.

### 9.5 Timing table

| Item | Value |
|---|---|
| Base delay per character | `1000 / 20 / max(0.1, Config_TextSpeed)` ms. The default speed 1.0 gives 50 ms (20 chars/s). The config slider runs 0.1–3.0 in 0.1 steps, i.e. 500 ms down to 16.67 ms. |
| `、` `,` `…` | base × 2.5 |
| `」` `）` | base × 1.5 (ASCII `)` and `』` use the base delay) |
| Pause after the clear‑page branch | 10 ms |
| Stream polling while caught up | 50 ms |
| The delay applies **after** showing the character it was computed for | — |

### 9.6 Rust implementation guidance
- Model the typewriter as a pure struct in amadeus‑core that returns actions such as `Schedule(ms)`, `Paused`, `Finished` and `Idle`, plus side outputs (dialogue text, spoken character, backlog line). Drive it from the UI with a single timeout handle.
- Rust state updates are immediate. V1's "set, then rely on the re‑render before the next tick" therefore maps directly onto plain field mutation. **Exception:** keep the skip branch's comparison against the **old** `typeIndex` (read it before assigning `len`) when you want strict parity.
- Read `Config_TextSpeed` from the KV store for every character; the setting can change while typing is in progress.

---

## 10. Message‑flow timelines

### 10.1 Non‑streaming (default FreeAPI), input focused, user presses Enter
1. The input's React `onKeyDown` fires: `preventDefault()`, `submitMessage(chatInput)` (§8.19). The backlog gets the user line, the state becomes `waitingAPI` with indicator `"."`, and the fetch starts.
2. A microtask runs (React flushes discrete updates between native listeners):
   - The re‑render runs the history updater, which trims, calls `recordInteraction` and refreshes the system prompt.
   - The input panel unmounts.
   - The dialogue panel appears with an empty text, `"."`, and the PC hint `CTRL+Cでキャンセル` (or the mobile `キャンセル` button).
   - E5 starts the dots; E14 re‑registers with the new closure.
3. The same native keydown reaches **window**:
   - The E1 activity handler calls `resetIdleTimer`.
   - The E14 handler (new closure, `waitingAPI`) calls `onAdvance()`, which is a no‑op.
   - So there is **no double submit**. In Rust, make this order‑independent (§19.3).
4. The dots show `".." → "..." → "."` every 400 ms.
5. The response arrives. `updateStatus(latency)` runs, then `handleApiSuccess`: emotion, achievements, history, `fullText`, typewriter reset, `typing`, and the TTS chunks are queued.
6. E5 stops; E6 starts typing immediately; Live2D gets the emotion, `lipSync = 1` and `spokenChar` per character. Pages and pauses follow §9. Each page is logged.
7. When the text is finished: `waitForAdvance ▼`. Enter or click moves to `inputReady` with emotion `NORMAL`; the input panel returns **without focus**, so the user clicks it again (V1 has no autofocus).

### 10.2 Streaming (providers 1/3/4/5/6)
- **V1 actual:** see §6.1.
- **Option B:** as in 10.1, but `typing` starts at the first token. The typewriter types as text arrives (50 ms polling when caught up). At completion, `fullText` becomes the cleaned text, which may be shorter: if memory commands are removed, `typeIndex` can exceed the new length, and the end branch logs the full cleaned text while the box keeps the last typed slice.

### 10.3 Error
At any time an `errorOccurred` produces an ANGRY face and the mapped message typed out (pauses apply). Ongoing TTS is cancelled. Nothing goes into the history.

### 10.4 Cancel (`Ctrl+C` on PC / `キャンセル` on mobile), only in `waitingAPI`
`AIService.cancelStreaming()`; `inputReady`; `NORMAL`; indicator `""`; `chatInput = ""`.
- It does **not** stop TTS or reset the stream buffers.
- For non‑streaming providers the fetch is **not** cancelled. The late response is still displayed (it moves to `typing` from any state) and appended to the history.
- Because AIService signals are global, a response to a request made before a re‑mount is delivered to the new ChatPanel.

### 10.5 Startup greeting
See §17.

### 10.6 Import
See §23. The bridge's `applyImported` replaces the history and `turnCount`, sets `inputReady` and `NORMAL`, and **does not** touch the typewriter, TTS or in‑flight requests.

---

## 11. TTS pipeline (experimental)

### 11.1 Inputs
- **Non‑streaming:** `triggerTTSIfEnabled(display, true)` → `splitTextForTTS` → each chunk is enqueued.
- **Streaming:** every token calls `captureStreamTextForTTS(display, false)` and completion calls `captureStreamTextForTTS(cleaned, true)`. The delta is cleaned of tags, added to the buffer, and `flushTTSChunkBuffer` cuts it at `。！？!?\n`, extending the cut over the closers `」』”")）`.

### 11.2 Scheduling invariants
- At most **one synthesis** is in flight (`ttsSynthInFlightRef`).
- At most **one playback** at a time (`ttsPlaybackBusyRef`).
- **Pipelining:** once playback of chunk N has been requested, synthesis of N+1 starts. Finished audio waits in `ttsAudioQueue` until the current playback ends.
- Order is preserved, except in the stale‑session race (§6.3) and the double‑error skip (§11.5 bug 3).

### 11.3 Events
| Event | Handler action |
|---|---|
| `synthesisComplete(url)` | If the guard passes: `inFlight = false`; push the URL to the audio queue; `playNext()`. |
| `synthesisError` | If the guard passes: `inFlight = false`; `processNext()`. The failed chunk is skipped. |
| `playbackStarted` | `busy = true` |
| `playbackFinished` | `busy = false`; `playNext()` |
| `AudioService.errorOccurred` | `busy = false`; `playNext()` |

### 11.4 Resets
`prepareLiveTTSSession` runs on every submit and `cancelLiveTTS` runs on error. Both clear the queues, the flags, the buffer and the processed index, set the stamp to −1, and call `AudioService.stop()`.

Nothing stops TTS on `onAdvance`, on cancel, when the mini game opens, or when the dialogue is finished. Audio keeps playing after the text has finished; the next submit stops it.

### 11.5 Known V1 bugs (decide parity versus fix)
1. **The tail is never flushed when streaming.** At completion, `cleaned` usually equals (or is shorter than) the last token's display, so `delta === ""` and the function returns **before** `flushTTSChunkBuffer(true)`. Consequences:
   - A final sentence without `。！？!?` is never spoken.
   - An **English** answer that only uses `.` is **never spoken at all** in streaming mode.
2. Stale‑session race (§6.3).
3. `AudioService` can emit `errorOccurred` twice for one failure. The second handler call starts yet another chunk, and `playFromFile`'s internal `stop()` cuts off the chunk that had just started.

### 11.6 No synchronisation with the typewriter
The typewriter speed comes only from `Config_TextSpeed`; TTS playback runs independently. Lip sync comes from the typewriter, not from the audio (§12).

---

## 12. Live2D props, lip sync and spoken char (L1865–1870)

```tsx
<Live2DCanvas
  emotion={miniGameOpen ? miniGameEmotion : currentEmotionTag}
  lipSyncValue={miniGameOpen ? miniSpeak : isActivelySpeaking ? 1 : lipSyncValue}
  spokenChar={miniGameOpen ? "" : isActivelySpeaking ? currentSpokenChar : ""}
  eyeTrackingEnabled={eyeTrackingEnabled}
/>
```
- Effectively `lipSync = isActivelySpeaking ? 1 : 0`. The `lipSyncValue` state lags by one render when speech stops (E8), which is negligible.
- **Rust:** `lip = if mini_open { mini_speak } else if speaking { 1.0 } else { 0.0 }`.
- The mouth stops at every pause (`waitingForInput`) and at the end, even if TTS audio is still playing. During `typing`, it is open while characters remain *or* while TTS was busy at the last render.
- Emotions that reach the runtime:
  - the 11 tags;
  - `SLEEPING` (idle);
  - any string from `/dev emotion` (uppercased);
  - the mini‑game emotions.
- After the startup greeting, after errors and after replies, the emotion persists until `onAdvance`/auto at `waitForAdvance` resets it to `NORMAL`.

---

## 13. STT mic flow

- The button is shown only when `showAudioInput`, i.e. `localStorage.Config_AudioInputEnabled === "1"` **at mount**. No UI in V1 sets this key (see Open question #10), so it is hidden by default.
- **Click:** `toggleSttRecording` (§8.22).
  - Starting synchronously triggers `recordingStateChanged` → `sttRecording = true`. The label becomes `■` and the class gets `recording`.
  - The level bar `<div class="chat-mic-level"><span style="width: {round(level×100)}%"/></div>` appears while recording. The browser speech API path holds it at 50% (0.5); the MediaRecorder path animates the level.
- **Result:** the transcript replaces `chatInput` and the status becomes `音声入力: OK`. An empty transcript leaves the status `""`.
- **Error:** the status becomes `音声入力に失敗しました`. That covers no speech detected, too short, mic denied, not enabled and server errors.
- The status text lives inside the input panel. It is cleared only at the next toggle, persists across exchanges, and reappears when the input panel re‑mounts. **It is hard‑coded Japanese and not localized.**
- Recording stops on the second click, after 30 s, at the end of browser recognition, or on unmount.
- There is no auto‑submit. Enter or SEND sends the transcript; the global Enter handler works even when the input is not focused (§19).

---

## 14. Idle / sleep

- **Activity sources** (window listeners, E1): `mousemove` (only when the coordinates changed), `mousedown`, `keydown`, `touchstart`, `scroll`.
  - The `scroll` listener on window only sees document scrolling, which never happens here (the roots are `overflow: hidden`).
  - The input's `onChange` also calls `resetIdleTimer`.
  - E3 calls it when `activeOverlay` or `menuPanelOpen` changes.
- **Tick** (E2): once per second, only in `inputReady`/`waitForAdvance`. The interval restarts on every state or emotion change. `idle >= 60` and not already sleeping leads to `SLEEPING` with the timestamp stored. In `typing`/`waitingAPI` the counter is neither incremented nor reset.
- **While sleeping:**
  - The placeholder becomes `t("sleeping_placeholder", "Zzz... (キー入力やマウス移動で起こす)")`.
  - The input gets the class `sleeping-input`.
  - Live2D shows `SLEEPING`, unless the mini game is open.
- **Wake:** the first activity event at least 800 ms after falling asleep sets `NORMAL`, and the runtime plays a 2.5 s startled `SURPRISED`. Activity within 800 ms only resets the counter.
- Sleep can happen at `waitForAdvance` while a reply is still on screen.
- The mini game does not suspend the idle counter, but interacting with it produces activity events.

---

## 15. Auto mode (E7, L1201–1223). Verbatim logic

```ts
if (!autoMode) return;
if (menuPanelOpen) return;
const shouldRun = chatState === "waitForAdvance" || (chatState === "typing" && waitingForInput);
if (!shouldRun) return;
const timer = setTimeout(() => {
  if (chatState === "waitForAdvance") { setChatState("inputReady"); setCurrentEmotionTag("NORMAL"); setWaitingIndicator(""); }
  else if (waitingForInput) { setWaitingForInput(false); }
}, autoSpeed * 1000);
return () => clearTimeout(timer);
```
- `autoSpeed` is `Config_AutoSpeed` (default 3.0 s; the slider runs 1–10 s in 0.1 steps).
- The timer restarts on any dependency change, for example F3 toggling `autoMode` or a change of `autoSpeed`.
- Auto mode does not wait for TTS, and it waits twice at the end of answers that end with a pause character (§9.4 A).
- The "Auto" label is rendered by the parent, on PC only.

---

## 16. Achievements hooks and toast

### 16.1 Notifications ChatPanel sends
| When | Event |
|---|---|
| Non‑streaming success | `emotion` {tag}, then `chat` {totalInteractions} |
| Stream complete | `emotion` {currentEmotionTagRef or the parsed tag}, then `chat` {totalInteractions} |
| Startup greeting | `random_event` |
| `/dev emotion …`, `/dev motion reset`, `/dev achievements reset` | `dev_command` (after `resetAll` for the latter) |
| `/dev steiner` | `secret_steiner` |
| MiniGamePanel (child) | `game_play`, `game_win`, `game_perfect` |
| Parent export/import | `export`, `import` |

### 16.2 Toast
- E9 sets `{title, description, visible: true}` on every unlock.
- When several unlocks happen in one synchronous batch (for example the first reply unlocks `first_normal` and then `first_chat`), **only the last one is shown**: "初めまして" / "初めて紅莉栖と会話した".
- E10 hides the toast 4 s after the last unlock. CSS fades it in and out over 400 ms. The title and description stay in the DOM while it is hidden.
- DOM: `<div class="achievement-toast[ visible]" role="status"><div class="achievement-toast-title">★ {title}</div><div class="achievement-toast-desc">{description}</div></div>`. The title text is `` `★ ${title}` `` (U+2605 followed by a space), so before any unlock it renders `"★ "`.

---

## 17. Startup random event (E12, L1288–1312)

1. If `startupEventShownRef` is already set, return; otherwise set it (once per mount).
2. `RandomEventService.tryTriggerEvent()`; `text = getEventText()` (localized with `Config_Language`, falling back to the Japanese default); `emotion = getEventEmotion()`.
3. If `text` is empty, return. This never happens in practice.
4. `setCurrentEmotionTag(emotion)`; `setFullText(text)`; typewriter reset block; `streamComplete = true`; `chatState = "typing"`.
5. `AchievementManager.notifyEvent("random_event")`. The first time ever, this shows the toast "偶然の出会い" / "起動時ランダムイベントが発生した".

The greeting is **not** added to `conversationHistory` (the LLM never sees it), gets no TTS, and does not call `recordEmotion`. It **is** logged to the backlog as assistant lines by the typewriter. The time bucket was fixed at page load, not at mount.

---

## 18. Mini game and achievements triggers

- Both buttons render only when `chatState === "inputReady" && !miniGameOpen`.
  - `<button type="button" class="mini-game-button">MINI GAME</button>` runs `setMiniGameOpen(true)`. It does not touch `activeOverlay`.
  - `<button type="button" class="mobile-achievements-trigger-button">ACHIEVEMENTS</button>` runs `setActiveOverlay?.("achievements")`. CSS hides it on PC, and on PC `setActiveOverlay` is undefined anyway. On mobile the parent shows `AchievementPanel` while `activeOverlay === "achievements"`.
- While the mini game is open:
  - The input panel and both buttons are hidden.
  - `.live2d-shift` gets `live2d-shifted`. On PC that is `translateX(28%)`; on mobile it is `translateY(-14dvh) scale(0.85)` with `transform-origin: top center`, a 450 ms transition and the easing `cubic-bezier(.645,.045,.355,1)`.
  - Live2D receives the mini game's emotion and lip value, with `spokenChar = ""`.
  - The global keyboard handler is disabled.
- `MiniGamePanel` `onClose` (button, `Backspace` or `Escape`) runs `setMiniGameOpen(false)` and `setActiveOverlay?.("none")` (mobile only).
- `onEmotionChange` is `setMiniGameEmotion` and `onSpeakValueChange` is `setMiniSpeak`; both are stable setters.
- `miniGameEmotion` and `miniSpeak` keep their last values after close and are not shown again until the next open.
- The `OverlayPanel` variant `"minigame"` is never used.

---

## 19. Keyboard, pointer, focus and event ordering

### 19.1 Global keydown (E14, window). Verbatim logic
```ts
const onKeyDown = (event: KeyboardEvent) => {
  if (menuPanelOpen || miniGameOpen) return;
  if (event.key.toLowerCase() === "c" && event.ctrlKey) {
    if (chatState === "waitingAPI") {
      event.preventDefault();
      AIService.cancelStreaming();
      setChatState("inputReady"); setCurrentEmotionTag("NORMAL"); setWaitingIndicator(""); setChatInput("");
      return;
    }
  }
  if (event.key === "Enter") {
    if (chatState === "inputReady") { if (chatInput.trim()) submitMessage(chatInput); }
    else onAdvance();
  }
};
```
- `Ctrl+C` outside `waitingAPI` falls through: Ctrl+C copy still works and Enter is not involved.
- macOS also needs **Ctrl** (not Cmd).
- On non‑Latin layouts `event.key` may not be `"c"`, so the shortcut is not detected.
- Enter is handled even when focus is elsewhere. That includes text inputs in other panels: on PC every overlay is opened from the Tab menu, so `menuPanelOpen` stays true while it is open; on mobile the backlog and achievements overlays do **not** set `menuPanelOpen`.
- There is **no `isComposing` check** (Japanese IME), see Open question #8.
- There is no `repeat` check: holding Enter repeatedly advances or skips.

### 19.2 Input keydown (element)
`if (event.key === "Enter") { event.preventDefault(); submitMessage(chatInput); }` runs even when the input is empty; `submitMessage` ignores an empty text.

### 19.3 Ordering
For a native keydown in the input:
1. The element handler (React root delegation) runs.
2. React flushes, in a microtask between listeners: re‑render and passive effects.
3. The window listeners run in registration order: E1 (activity, registered once) and then E14 (re‑registered on every dependency change).

Because of step 2, E14 sees the post‑submit state, so one Enter produces exactly one submit.

To keep the same outcome in Leptos, make the handlers **order‑independent** rather than relying on listener order. Leptos 0.7/0.8 may install its delegated `keydown` listener on `window` itself, in which case the order relative to `window_event_listener` depends on which was registered first.
- Both the input handler and the global handler must read **live** state.
- `submit_message` must keep its `chat_state == InputReady` guard. Whichever handler runs second then sees `WaitingApi`: `submit` returns, and `on_advance` is a no‑op in `WaitingApi`. Either order gives one submit.
- Optionally use `on:keydown:undelegated` on the input so it runs at the element.
- Register the activity listener before the keyboard listener (parity for the §8.20 sleeping quirk).

### 19.4 Parent keys that affect ChatPanel (owned by the parent spec)
- `F3` toggles `Config_AutoMode`.
- `F11` toggles fullscreen.
- `Tab` toggles the PC menu (`menuPanelOpen`). The menu panel opens 450 ms later, and the parent **blurs** a focused input/textarea when the menu activates.
- Arrows/WASD/Enter/Backspace navigate the menu.
- Right‑click toggles the menu if `Config_RightClickMenu == 1`.

### 19.5 Pointer
- The dialogue box is `pointer-events: none`; only the full‑size transparent `.dialogue-advance-hitbox` (z 1) and the mobile `.cancel-streaming-button` (z 2) receive clicks.
- **On mobile `.dialogue-text` cannot be scrolled by touch**, even though it is `overflow-y: auto`, because pointer events are disabled (CSS reused as is).
- Clicking the hitbox in `waitingAPI` does nothing.

### 19.6 Focus
There is no `autoFocus` and no `.focus()` anywhere in ChatPanel. The input unmounts whenever the state is not `inputReady` or the mini game is open, so focus is lost after every exchange.

---

## 20. DOM structure (verbatim JSX, L1862–1994) and CSS

```tsx
<div className="chat-panel-root">
  <div className={`live2d-shift${miniGameOpen ? " live2d-shifted" : ""}`}>
    <Live2DCanvas …/>                                    {/* renders <div class="live2d-host"> + runtime canvas */}
  </div>

  {chatState !== "inputReady" && (
    <div className="dialogue-panel">
      <button type="button" className="dialogue-advance-hitbox" onClick={onAdvance}
        aria-label={t("advance_dialogue", configLanguage === 1 ? "Advance dialogue" : "会話を進める", configLanguage)} />
      <div className="name-text">{t("amadeus_kurisu", "アマデウス紅莉栖", configLanguage)}</div>
      <div className="dialogue-text">{dialogueText}</div>
      <div className="waiting-indicator">
        {chatState === "waitingAPI" || chatState === "waitForAdvance" || waitingForInput ? waitingIndicator : ""}
      </div>
      {chatState === "waitingAPI" &&
        (isMobileViewport ? (
          <button className="cancel-streaming-button"                      /* no type attribute */
            onClick={(e) => { e.preventDefault(); AIService.cancelStreaming(); setChatState("inputReady");
                              setCurrentEmotionTag("NORMAL"); setWaitingIndicator(""); setChatInput(""); }}>
            {t("cancel_button", "キャンセル", configLanguage)}
          </button>
        ) : (
          <div className="pc-cancel-hint">{t("cancel_hint", "CTRL+Cでキャンセル", configLanguage)}</div>
        ))}
    </div>
  )}

  {chatState === "inputReady" && !miniGameOpen && (
    <div className={`input-panel ${showAudioInput ? "has-audio-input" : ""}`}>        {/* "input-panel " w/ trailing space when false */}
      <input
        value={chatInput}
        onChange={(event) => { setChatInput(event.target.value); resetIdleTimer(); }}
        placeholder={currentEmotionTag === "SLEEPING"
          ? t("sleeping_placeholder", "Zzz... (キー入力やマウス移動で起こす)", configLanguage)
          : t("enter_message", "メッセージを入力", configLanguage)}
        className={`chat-input ${currentEmotionTag === "SLEEPING" ? "sleeping-input" : ""}`}
        onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); submitMessage(chatInput); } }}
      />                                                                                {/* no type/maxLength/autofocus */}
      <div className="chat-input-actions">
        {showAudioInput && (
          <button type="button" className={`chat-mic-button ${sttRecording ? "recording" : ""}`}
            onClick={toggleSttRecording}
            aria-label={t("speech_input", configLanguage === 1 ? "Speech input" : "音声入力", configLanguage)}>
            {sttRecording ? "■" : "🎤"}
          </button>
        )}
        <button type="button" className="chat-send-button" onClick={() => submitMessage(chatInput)}
          disabled={!chatInput.trim()}>
          {t("send", configLanguage === 1 ? "SEND" : "送信", configLanguage)}
        </button>
      </div>
      {sttRecording && (<div className="chat-mic-level"><span style={{ width: `${Math.round(sttRecordingLevel * 100)}%` }} /></div>)}
      {sttStatusText && <div className="chat-stt-hint">{sttStatusText}</div>}
    </div>
  )}

  {chatState === "inputReady" && !miniGameOpen && (
    <>
      <button type="button" className="mini-game-button" onClick={() => setMiniGameOpen(true)}>MINI GAME</button>
      <button type="button" className="mobile-achievements-trigger-button"
        onClick={() => { if (setActiveOverlay) setActiveOverlay("achievements"); }}>ACHIEVEMENTS</button>
    </>
  )}

  <MiniGamePanel visible={miniGameOpen} configLanguage={configLanguage}
    onClose={() => { setMiniGameOpen(false); if (setActiveOverlay) setActiveOverlay("none"); }}
    onEmotionChange={setMiniGameEmotion} onSpeakValueChange={setMiniSpeak} />

  <div className={`achievement-toast${achievementToast.visible ? " visible" : ""}`} role="status">
    <div className="achievement-toast-title">{`★ ${achievementToast.title}`}</div>
    <div className="achievement-toast-desc">{achievementToast.description}</div>
  </div>
</div>
```
Notes:
- **Conditional blocks unmount and remount** (they are not hidden). The input loses its DOM state each time.
- The class strings with trailing spaces (`"input-panel "`, `"chat-input "`, `"chat-mic-button "`) are harmless; emitting normalized class lists is fine.
- `.dialogue-text` relies on `white-space: pre-wrap`; leading and trailing whitespace and newlines in a page are visible.
- Glyphs: `▼` U+25BC, `■` U+25A0, `🎤` U+1F3A4, `★` U+2605, `……` U+2026 ×2.

### 20.1 CSS (reused verbatim from `app/amadeus/app/globals.css`; emit exactly these classes)

| Range | Content |
|---|---|
| L323–333 | `.chat-panel-root`: `position:absolute; inset:35px 0 0 0` (below the PC title bar), 500 ms fade‑in, `touch-action: manipulation`. |
| L335–349 | `.live2d-host` and its canvas: 3840×2160 centered, `pointer-events: auto`. |
| L351–366 | `.dialogue-panel`: bottom, 300 px high, `rgba(45,15,0,.94)`, z 5, `pointer-events: none`. The hitbox and cancel button re‑enable pointer events. |
| L368–398 | `.cancel-streaming-button` (bottom‑left, gold), `.pc-cancel-hint` (24 px white, bottom‑left 20 px). |
| L400–403 | `.sleeping-input`: italic, opacity .6. |
| L405–455 | `.dialogue-advance-hitbox`, `.name-text` (300×50, 36 px, `left: 50%; top: calc(50% + 112px - 25px); transform: translateX(-50%)`), `.dialogue-text` (1280 px wide, 36 px, line‑height 1, pre‑wrap), `.waiting-indicator` (right 80 px, bottom 35 px, 28 px, `rgba(255,153,0,.8)`). |
| L457–533 | `.input-panel` (left/right 50, bottom 25, height 60, `#4e1e15`, z 6), `.chat-input` (28 px, transparent), `.chat-send-button {display:none}` on PC, `.chat-input-actions`, `.chat-mic-button(.recording)`, `.chat-mic-level`, `.chat-stt-hint`. |
| L535–546 | `.auto-text` (parent PC label). |
| L677–722 | `@media (pointer: coarse), (max-width: 900px)`: makes the SEND button visible (96×44) and widens `.input-panel` padding. This **also applies on the PC layout** for touch laptops wider than 1200 px. |
| L873–896 | Scrollbar styling for `.dialogue-text`. |
| L1175–1214 | Achievement toast (top 30 px, 500×90, `#2a2a00`, orange border, z 40, 400 ms opacity). |
| L1216–1251 | `.live2d-shift(.live2d-shifted)`, `.mini-game-button` (PC: right 60, bottom 100, 140×50), `.mobile-achievements-trigger-button {display:none}`. |
| L2308–2451 | `.mobile-root` overrides: chat root `inset: calc(safe-top + 56px) 0 0 0` with no animation; Live2D host fixed and fullscreen; dialogue becomes a rounded card (left/right 16, bottom safe+16, min‑height 250, max‑height 50%, blur); name at top‑left (20 px, `#ffcc00`); text at 19 px / line‑height 1.6 and scrollable; input becomes a pill (64 px; padding‑right 104, or 152 with audio); SEND as an orange 84×44 pill; mic round. |
| L2586–2965 | Second mobile media block: mini‑game bottom sheet (46%, z 3000), achievements panel, **floating MINI GAME (left 16) and ACHIEVEMENTS (right 16) pills** at `bottom: safe+88px` (z 200), and the mobile `.live2d-shifted` transform. |

---

## 21. User‑visible strings (verbatim) and translation keys

### 21.1 Translated through `t(key, default, configLanguage)`

| Key | Default argument | Availability in the locale JSONs |
|---|---|---|
| `advance_dialogue` (aria) | `configLanguage === 1 ? "Advance dialogue" : "会話を進める"` | **missing in all 13**, so the default is used |
| `amadeus_kurisu` | `"アマデウス紅莉栖"` | present in all: ja `アマデウス紅莉栖`, en `Amadeus Kurisu`, zh `阿玛迪斯·红莉栖`, ko `아마데우스 쿠리스`, es/fr/de/pt/tr `Amadeus Kurisu`, ru `Амадеус Курису`, uk `Амадеус Курісу`, he `אמדאוס קוריסו`, ar `أماديوس كوريسو` |
| `cancel_button` | `"キャンセル"` | **missing in all**, so it always shows `キャンセル` |
| `cancel_hint` | `"CTRL+Cでキャンセル"` | present in all: ja `CTRL+Cでキャンセル`, en `CTRL+C to cancel`, zh `按 CTRL+C 取消`, ko `CTRL+C로 취소`, es `CTRL+C para cancelar`, fr `CTRL+C pour annuler`, de `CTRL+C zum Abbrechen`, ru `CTRL+C для отмены`, uk `CTRL+C для скасування`, pt `CTRL+C para cancelar`, tr `İptal etmek için CTRL+C`, he `CTRL+C לביטול`, ar `CTRL+C للإلغاء` |
| `sleeping_placeholder` | `"Zzz... (キー入力やマウス移動で起こす)"` | present in ja, en (`Zzz... (Press any key or move mouse to wake up)`), ko, es, uk, pt, tr, he, ar; **missing in zh, fr, de and ru**, which show the Japanese default |
| `enter_message` | `"メッセージを入力"` | present in all: ja `メッセージを入力`, en `Enter your message...`, … |
| `speech_input` (aria) | `configLanguage === 1 ? "Speech input" : "音声入力"` | **missing in all** |
| `send` | `configLanguage === 1 ? "SEND" : "送信"` | **missing in all**, so only English shows `SEND`; every other language, including zh and ko, shows `送信` |

Use the embedded locale JSONs (amadeus‑core i18n) and the same `t()` semantics, so these fallbacks reproduce automatically.

### 21.2 Hard‑coded strings (not localized)
`"MINI GAME"`, `"ACHIEVEMENTS"`, `"★ "`, `"▼"`, `"."`/`".."`/`"..."`, `"■"`, `"🎤"`, `"……"`, `"音声入力: OK"`, `"音声入力に失敗しました"`, the 12 error messages (§8.18), the help text (§8.20), and the parent's `"Auto"`.

---

## 22. Backlog contract (parent `addBackLog`, L4018–4043)

ChatPanel calls `onBackLogAdd(role, message)` in these places:
- `("user", text)` on every normal submit;
- `("assistant", pageText)` at every pause, when the page is non‑blank;
- `("assistant", finished)` on skip, when it is non‑blank and differs from the last logged page;
- `("assistant", fullText)` at the end, when it differs from the last logged page.

The parent then does the following:
1. `clean = raw.trim()`.
2. If `clean` starts with `"["` and `close = clean.indexOf("]") > 0`, then `clean = clean.slice(close+1).trim()`. This removes **any** leading bracket group, even user text such as `"[1] apple"`, which becomes `"apple"`.
3. Remove all emotion tags case‑insensitively and trim.
4. An empty result is dropped.
5. `role = role.toLowerCase()`; `nameColor` is `"#66CCFF"` for user/me, `"#FF6666"` for assistant/kurisu/amadeus, otherwise `"#808080"`.
6. Append `{role, nameColor, message: clean}`. The backlog is in memory only and is never persisted.

BackLogPanel shows the name as `t("you", lang==1?"You":"あなた")` for user, `t("amadeus_kurisu", …)` for assistant, and otherwise `role.toUpperCase()`.

---

## 23. Conversation bridge and import/export

### 23.1 Bridge (E13)
```ts
conversationRef.current = {
  getHistory: () => conversationHistoryRef.current,            // last rendered history, incl. system msg at [0]
  getTurnCount: () => turnCountRef.current,
  applyImported: (messages, turnCount) => {
    setConversationHistory(messages); setTurnCount(turnCount);
    setChatState("inputReady"); setCurrentEmotionTag("NORMAL");
  },
};
// cleanup: conversationRef.current = null
```
On a re‑mount, the old cleanup runs before the new effect, so the ref ends up set.

### 23.2 Parent export (`exportConversation`, L4045–4085), from the ConfigPanel "書き出す / Export" button
1. `history = bridge ? bridge.getHistory() : []`; `exportMessages = history.filter(m => m.role !== "system")`.
2. If there is no bridge or nothing to export, return `{success:false, message: t("export_no_data","会話データがありません")}` (en `No conversation data available`).
3. `timestamp = new Date().toISOString().replace(/[:.]/g, "-").substring(0, 19)`, which is UTC `YYYY-MM-DDTHH-MM-SS`. `fileName = \`chat_export_${timestamp}.json\``.
4. The JSON is `JSON.stringify({ version: 1, exportedAt: new Date().toISOString(), turnCount: bridge.getTurnCount(), messages: exportMessages }, null, 2)`, i.e. pretty‑printed with 2 spaces and `messages: [{role, content}]`.
5. It downloads through a Blob (`application/json`) and a temporary `<a download>`. Then `AchievementManager.notifyEvent("export")`.
6. It returns `{success:true, message: t("export_success","エクスポート完了: ") + fileName}`, or on an exception `t("export_failed","エクスポート失敗: ") + String(error)`.

### 23.3 Parent import (`importConversation(raw)`, L4087–4162), fed by the ConfigPanel file input (`accept=".json"`, read with `file.text()`)
1. `JSON.parse`. The result must be a non‑array object, otherwise `t("import_invalid","無効なファイル形式です")`.
2. `messages` must be an array; every entry must be an object with string `role` and string `content`, otherwise the same invalid message.
3. `turnCount = typeof parsed.turnCount === "number" ? parsed.turnCount : 0`. Any number is accepted, including floats and negatives.
4. `bridge.applyImported(imported.map(m => ({ role: toChatRole(m.role), content: m.content })), turnCount)`.
5. `setBackLogEntries([])`. Then for each non‑system message: if the role (lowercased) is assistant/kurisu/amadeus, each segment of `splitMessage(content)` goes to `addBackLog(role, segment)`; otherwise `addBackLog(role, content)`.
6. `AchievementManager.notifyEvent("import")`.
7. It returns `{success:true, message: t("import_success","インポート完了: ") + imported.length + t("import_turns"," ターン")}`. Note that `imported.length` counts **all** messages, not turns. On an exception it returns `t("import_failed","インポート失敗: ") + String(error)`.
8. **V1 behavior:** exports never contain the system message, so after an import the history has **no system prompt**. `updateSystemPrompt` never inserts one, so subsequent requests are sent **without persona or emotion‑tag instructions** (Open question #6).

---

## 24. LLM status reporting

`onLlmStatusUpdate(getLlmStatusFromSettings(latency))` is sent:
- at mount, with latency 0;
- on non‑streaming success, with `Date.now() − requestStart`;
- while streaming, on every token until the emotion has been decided (V1).

It is not sent on errors or on stream completion. The parent shows it in the mobile top‑bar marquee and the StatusPanel.

---

## 25. Storage keys touched (directly or through the services, within ChatPanel flows)

| Key | Format | Access |
|---|---|---|
| `Config_AudioInputEnabled` | `"0"`/`"1"` (int string) | Read once at mount. Nothing writes it. |
| `Config_EyeTracking` | int string | Read (reactive) |
| `Config_ChatLanguage` | int string (0 = follow UI; n = language n−1) | Read (reactive + prompt builder) |
| `Config_Language` | int string 0..12 | Read (prompt builder; parent prop; random‑event text) |
| `Config_TextSpeed` | float string (default `1.0`) | Read on every typed character |
| `Config_AutoMode` / `Config_AutoSpeed` | int / float strings | Read by the parent and passed as props |
| `Config_ApiProvider` | int string | Read at submit (default 0 here) and by the status helper (default 8) |
| `Config_ModelName_{n}`, `Config_ModelName`, `Config_OpenAICompatible`, `Config_OpenAIBaseUrl` | strings | Read by the status helper |
| `Config_Experimental_TTS_*`, `Config_Experimental_STT_*`, `Config_VoiceVol` | strings | Read by the services |
| `Config_Experimental_STT_Enabled` | `"1"` | **Written** when the mic button is clicked while STT is disabled |
| `RealAmadeus.Memory` | JSON `{userName,totalInteractions,lastSessionDate,recentEmotions[],userFacts[{content,strength,lastUpdated}],conversationSummaries[],episodicMemory[{timestamp,summary,tags[]}]}` | Written by MemoryManager on submit, reply, emotion and memory commands |
| `amadeus_achievements` | JSON `{unlocked[],dates{id:"YYYY-MM-DD HH:mm"},seenEmotions[],lastLaunchDate,launchStreak}` | Written by AchievementManager |

V2 must keep these key names and formats **identical** so that existing web users keep their data.

---

## 26. Network requests triggered (the services issue them; V2 goes through `Transport`)

| Request | Body | Response |
|---|---|---|
| `POST {base}/app/api/chat` (V1 base `/amadeus`) | JSON `ChatPayload` (§3.3) | `200 {"response": string}` / non‑2xx `{"error": string}` |
| `POST {base}/app/api/chat/stream` | same | SSE body: lines `data: {"type":"token","token":"…"}\n\n`, `data: {"type":"complete","fullResponse":"…"}\n\n`, `data: {"type":"error","error":"…"}\n\n`. Before streaming starts, errors come back as a JSON `{"error"}` with a non‑2xx status (for example a 429 rate limit). |
| `POST {base}/app/api/tts/synthesize` | JSON `{serverUrl, speaker, text, useCache: true}` | audio blob, or `{"error"}` |
| `POST {base}/app/api/stt/transcribe` | multipart: `audio` (`recording.webm`), `provider`, `language`, `serverUrl` | `{"text"}` / `{"error"}` |

ChatPanel itself never calls `fetch`.

---

## 27. Timers and constants (master table)

| What | Value | Where |
|---|---|---|
| Idle tick | 1000 ms interval | E2 |
| Sleep threshold | 60 ticks | E2 |
| Wake suppression after sleep | 800 ms | resetIdleTimer |
| Waiting dots | 400 ms interval, cycle `. → .. → ...` | E5 |
| Typewriter base | `1000/20/max(0.1, speed)` ms | charDelayForChar |
| Comma/ellipsis factor | ×2.5 for `、` `,` `…` | charDelayForChar |
| Closing‑bracket factor | ×1.5 for `」` `）` | charDelayForChar |
| Clear‑page step | 10 ms | E6 |
| Stream poll | 50 ms | E6 |
| Auto advance | `autoSpeed × 1000` ms (default 3000) | E7 |
| Toast visible | 4000 ms (+400 ms CSS fade) | E10 |
| History trim | > 30 non‑system → remove `n−30+4` | submit |
| TTS split length | 120 UTF‑16 units | splitTextForTTS |
| STT max recording | 30 000 ms | STTService |
| STT minimum blob | 1024 bytes | STTService |
| Mini‑game mouth | 120 ms random; duration `clamp(len×90, 800, 3000)` | MiniGamePanel |
| Live2D wake startle | 2.5 s | runtime |
| CSS chat fade‑in | 500 ms | `.chat-panel-root` |
| CSS Live2D shift | 450 ms, `cubic-bezier(0.645,0.045,0.355,1)`; PC `translateX(28%)`, mobile `translateY(-14dvh) scale(0.85)` | `.live2d-shift` |
| Mobile breakpoint | `width ≤ 900` or (coarse and `width ≤ 1200`), using `visualViewport.width` | useIsMobileViewport |
| Provider streaming set | `{1, 3, 4, 5, 6}` | submit |

---

## Rust mapping

### A. amadeus-core (wasm32 + native, no I/O). Pure logic, unit‑testable natively
- `amadeus_core::persona`
  - `pub const CHAT_PERSONA_PROMPTS: [&str; 8]` (verbatim from L64–161)
  - `pub const CHAT_LANGUAGE_NAMES: [&str; 13]`
  - `pub fn system_prompt_for_language(lang_index: i32) -> String`
  - `pub fn current_chat_language_index(kv: &dyn KvStore) -> i32`
- `amadeus_core::chat::emotion`
  - `pub const EMOTION_TAGS: [&str; 11]`
  - `parse_emotion_tag(&str) -> String`
  - `strip_emotion_tag(&str) -> String`
  - `strip_think_blocks(&str) -> String`
  - `remove_emotion_tags(&str) -> String` (case‑insensitive)
  - Use `regex` with `(?i)` / `(?s)`, or hand‑written scanners. Note that JS `\s` and `.trim()` differ slightly from Rust (U+FEFF); see the Open questions.
- `amadeus_core::chat::text`
  - `is_pause_character(c: char, next: Option<char>) -> bool`
  - `char_delay_ms(c: char, text_speed: f64) -> f64`
  - `split_message(&str) -> Vec<String>`
- `amadeus_core::chat::typewriter`
  - `pub struct Typewriter { full_text: String, type_index: usize, page_start: usize, clear_page_on_resume: bool, last_logged_page: String, skip: bool, waiting_for_input: bool, stream_complete: bool }`
  - `pub fn step(&mut self, text_speed: f64) -> StepOutput`, where `StepOutput { dialogue: Option<String>, spoken_char: Option<String>, backlog: Option<String>, indicator: Option<&'static str>, next: Next }` and `enum Next { After(f64 /*ms*/), Paused, ToWaitForAdvance, Stop }`
  - `pub fn reset_with(&mut self, text: String, stream_complete: bool)` implements the "typewriter reset block".
  - Provide `parity_skip: bool` to choose between §9.3 parity and the §6.4 fix.
- `amadeus_core::chat::stream`
  - `pub struct StreamEmotionParser { raw: String, decided: bool }`
  - `push_token(&mut self, token: &str) -> TokenOutcome { display: String, new_emotion: Option<String>, decided_now: bool, report_latency: bool }`
  - `reset()`
- `amadeus_core::chat::tts_chunker`
  - `split_text_for_tts(&str) -> Vec<String>`
  - `pub struct StreamTtsChunker { buffer: String, processed: usize }` with `capture(&mut self, source: &str, complete: bool) -> Vec<String>` (returns chunks to enqueue), plus a `force_tail_on_complete: bool` switch (§11.5 bug 1).
- `amadeus_core::chat::errors::friendly_api_error_message(error: &str) -> &'static str` (§8.18 table; ship the test vectors).
- `amadeus_core::chat::dev`
  - `pub enum DevCommand { EmotionHelp, SetEmotion(Option<String>), MotionReset, AchievementsReset, Steiner }`
  - `parse_dev_command(text: &str) -> Option<DevCommand>`
  - `pub const DEV_EMOTION_HELP_TEXT: &str`
- `amadeus_core::chat::prompt`
  - `build_system_content(lang_idx, memory_ctx: &str, dynamic_ctx: Option<&str>) -> String`
  - `refresh_system_message(history: &[ChatMessage], content: String) -> Vec<ChatMessage>` (replaces index 0 only if it is a system message).
- `amadeus_core::conversation`
  - `ChatMessage`, `ChatRole` (serde lowercase)
  - `to_chat_role(&str)`
  - `ConversationExport { version: u32 /*1*/, exported_at: String, turn_count: serde_json::Number, messages: Vec<ChatMessage> }` (pretty JSON, 2 spaces, field names `version`, `exportedAt`, `turnCount`, `messages`)
  - `parse_import(raw) -> Result<Imported, ImportError>`
  - `export_file_name(now_utc) -> String`
- `amadeus_core::backlog`
  - `BackLogEntry { role, name_color, message }`
  - `make_entry(role: &str, raw: &str) -> Option<BackLogEntry>`
- `amadeus_core::llm_status`
  - `LlmStatus`
  - `llm_status_from_settings(kv, latency_ms: i64)`
  - `default_model_for_provider(i32)`, `provider_display_name(i32, bool, &str)`
  - `pub const STREAMING_PROVIDERS: [i32; 5] = [1,3,4,5,6]`
- `amadeus_core::protocol`
  - `ChatPayload` (camelCase serde)
  - `#[serde(tag="type", rename_all="lowercase")] enum StreamEvent { Token{token}, Complete{full_response /*fullResponse*/}, Error{error} }`
- Memory, achievements and random events also live in core, over `KvStore`, as other specs define them.

### B. amadeus-ui (Leptos 0.8 CSR)
- `components/chat_panel.rs`: `#[component] pub fn ChatPanel(auto_mode: Signal<bool>, auto_speed: Signal<f64>, config_language: Signal<i32>, menu_panel_open: Signal<bool>, on_backlog_add: Callback<(String,String)>, on_llm_status: Callback<LlmStatus>, active_overlay: Signal<OverlayPanel>, set_active_overlay: Option<Callback<OverlayPanel>>, conversation_bridge: StoredValue<Option<ConversationBridge>>) -> impl IntoView`.
- `chat/controller.rs`: `pub struct ChatController`, cloneable through `Rc`. It owns:
  - **View signals** (`RwSignal`): `chat_state: ChatState`, `emotion: String`, `spoken_char`, `chat_input`, `dialogue_text`, `waiting_indicator`, `waiting_for_input`, `stt_recording`, `stt_level`, `stt_status`, `mini_game_open`, `mini_game_emotion`, `mini_speak`, `toast: ToastState`, plus `full_text_len`/`type_index`, or a memo `is_actively_speaking`.
  - **Inner** (`Rc<RefCell<ChatInner>>`): `history: Vec<ChatMessage>`, `turn_count: f64`, `typewriter: Typewriter`, `stream: StreamEmotionParser`, `request_start_ms: f64`, `tts: TtsPipeline`, `idle_seconds: u32`, `sleep_ts_ms: f64`, `last_mouse: (i32,i32)`, `typing_timeout: Option<TimeoutHandle>`, `last_language_key: String`, `startup_shown: bool`.
  - **Methods mirroring §8:** `submit_message`, `on_advance`, `cancel_waiting`, `toggle_stt`, `on_api_success`, `on_stream_token`, `on_stream_complete`, `on_api_error`, `typewriter_tick`, `reset_idle`, `apply_imported`.
- `chat/tts_pipeline.rs`: `TtsPipeline { text_q: VecDeque<String>, audio_q: VecDeque<String>, synth_in_flight: bool, playback_busy: bool, generation: u64, in_flight_gen: Option<u64> }`, with `enqueue`, `process_next`, `play_next`, `on_synth_complete(url, gen)`, `on_synth_error(gen)`, `on_playback_finished`, `reset`.
- `chat/idle.rs`: window listeners and the 1 s interval, re‑created on state/emotion change or simply always running and checking the live state (equivalent).
- `components/achievement_toast.rs`; `components/live2d_canvas.rs` (wraps amadeus‑live2d); `components/mini_game_panel.rs` (other spec).
- `hooks.rs`: `use_is_mobile_viewport() -> Signal<bool>` (`visualViewport` resize, window resize, `matchMedia("(pointer: coarse)")` change), `use_app_setting_int/float(key, default) -> Signal<_>` (subscribed to the KV change bus).
- `events.rs`: `pub struct Signal<A>`, a port of `lib/amadeus/signal.ts` with snapshot iteration. `connect` returns a guard; disconnect it in `on_cleanup`.
- `services/`: `AiClient` (built on `Transport`; `send_chat`, `send_chat_streaming` → async stream of `StreamEvent`; `cancel_streaming` through an `AbortHandle`), `TtsClient` (cache by hash → object URL), `AudioPlayer` (`HtmlAudioElement`; **emit at most one error per playback**), `SttClient` (`SpeechRecognition` via `js_sys::Reflect`, or `MediaRecorder` + `AnalyserNode`).
- `transport.rs`: `trait Transport { async fn chat(&self, p: ChatPayload) -> Result<String, String>; fn chat_stream(&self, p: ChatPayload) -> (impl Stream<Item=StreamEvent>, AbortHandle); async fn tts(&self, req) -> Result<Vec<u8>, String>; async fn stt(&self, form) -> Result<String, String>; }`, with `HttpTransport` (fetch + `ReadableStream` SSE line parser; the same line rules as §3.3) and `TauriTransport` (`invoke`; streaming over `tauri::ipc::Channel<StreamEvent>`).
- `file_io.rs`: `trait FileIo { fn save_json(name, contents); async fn pick_json() -> Option<String> }`. On the web it uses a Blob + `<a download>` and `<input type=file>`; on desktop it uses Tauri dialog + fs.
- **Timers:** `leptos::prelude::set_timeout_with_handle` / `set_interval_with_handle` (or `gloo-timers`). Keep handles in the inner struct and clear them on cleanup.
- **Global listeners:** `window_event_listener(ev::keydown, …)`. Register the activity listener **before** the keyboard listener and read **live** state in both (§19.3).
- **Re‑entrancy:** never hold a `RefCell` borrow while calling into services or callbacks. TTS cache hits, STT start and `AudioPlayer::stop` call back synchronously in V1. Either release borrows first, or defer emissions with `spawn_local` / a microtask; if you defer, verify that the ordering is unchanged.

### C. amadeus-desktop (Tauri 2)
Nothing is specific to ChatPanel beyond the transport commands: `chat_send(payload) -> Result<String,String>`, `chat_stream(payload, channel: Channel<StreamEvent>)` with a cancel command or `AbortHandle` id, `tts_synthesize(req) -> Vec<u8>`, `stt_transcribe(bytes, provider, language, server_url) -> String`. Window resizing below 900 px would switch to the mobile layout, and in V1 semantics that **resets the conversation**; see Open question #17.

---

## Open questions / risks

1. **Streaming freeze (V1 defect, §6.1).** Parity means streaming providers are unusable; the recommendation is Option B (§6.4), which also needs the skip‑branch fix. **The team or the user must decide.** Whichever is chosen, document it in the V2 changelog.
2. **Streaming TTS tail never flushed** (§11.5 bug 1). English answers that use only `.` are never spoken. Recommendation: fix it (always force‑flush on complete).
3. **TTS session guard is effectively constant** (§6.3). Recommendation: use a real generation counter. The difference is only observable in a rare race.
4. **Backlog duplicates the full text** at the end of multi‑page answers (§9.4 A). This is user‑visible. The default is parity, but confirm with the user.
5. **Double `▼` at the end** of answers that end with a pause character: an extra click, and two auto waits. Parity by default.
6. **Import drops the system prompt** (§23.3 point 8), so persona and emotion tags are lost after an import. Recommendation: fix it by re‑inserting `build_system_content(...)` at index 0 after `apply_imported`, unless strict parity is required.
7. **Cancel does not cancel non‑streaming requests**; late responses are still displayed and stored, and a response can cross a re‑mount (global signals). Consider request ids and ignoring stale responses. Parity keeps the current behavior.
8. **No IME `isComposing` check on Enter.** Japanese users confirming a conversion with Enter may submit prematurely in some browsers. Adding `if ev.is_composing() || ev.key_code() == 229 { return; }` is a behavior change; confirm first.
9. **No autofocus** on the chat input after each exchange (V1). Desktop users would likely expect autofocus; parity says none.
10. **`Config_AudioInputEnabled` has no writer in V1.** The mic button is effectively hidden unless localStorage is edited by hand. The repo has `patch_amadeus.js`, `patch_amadeus2.js` and `patch_amadeus3.js`, which show an intended dev command that was never applied: its anchor `if (apiName === "freeapi")` does not exist in the code, and git history has no trace of it. The command shape would be `parts = text.split(" ")`, `apiName = parts[2] === "audio"`, `action = parts[3] === "input"`, `toggle = parts[4] ∈ {enable, disable}`; the second token is unknown. It would set the key, update `showAudioInput`, and type `音声入力を有効化しました。` / `音声入力を無効化しました。`. Decide whether V2 adds it.
11. **UTF‑16 versus Unicode scalar indexing.** V1 indexes by UTF‑16 units (typewriter, `slice`, `.length >= 120`, the TTS buffer scan). With astral characters (emoji, rare kanji), V1 briefly shows half surrogate pairs. Recommendation: iterate by `char` in Rust. That differs from V1 only for astral characters; for the TTS 120 limit you can count `encode_utf16` units if exact parity is wanted.
12. **Whitespace semantics.** JS `\s` and `trim()` include U+FEFF; Rust `char::is_whitespace` does not. JS `\s` and Rust regex `\s` both include U+3000. This only matters for edge inputs.
13. **Missing translations** (`cancel_button`, `send`, `speech_input`, `advance_dialogue`, and `sleeping_placeholder` in zh/fr/de/ru) produce Japanese or English fallbacks. Keep `t()` semantics; only add keys if the user agrees.
14. **Hard‑coded Japanese** error, STT and help strings, even in other UI languages. Keep for parity.
15. **AudioService double error emission** can skip a TTS chunk (§11.5 bug 3). Recommendation: emit once per playback in V2.
16. **Event ordering and Leptos event delegation.** V1 avoids a double submit only because React flushes state and effects in a microtask between the root listener and the window listeners. In Leptos the delegated listener may itself live on `window`, so the handler order depends on registration order. Make every keyboard handler read live state and keep the `InputReady` guard in submit (§19.3). Test both orders: one Enter must produce exactly one request and no advance.
17. **Remount on a layout switch** resets the conversation (V1). For the desktop builds, resizing a window below 900 px (or `≤ 1200` with a coarse pointer) would wipe the chat. Consider keeping `ChatInner` in a parent‑owned context across layout switches. That is a deliberate deviation and needs a decision.
18. **Emotion detection is inconsistent:** streaming only recognizes a tag at the very start, while non‑streaming finds the first tag anywhere. Keep for parity.
19. **Side effects inside the React state updater** (`recordInteraction` and the trim) run twice in dev StrictMode. In Rust, run them exactly once in submit.
20. **`/dev emotion sleeping` + Enter** wakes her immediately (startle animation), because the activity listener runs after the submit (§8.20). Parity quirk.
21. **Achievements unlocked at construction** (launch streak) produce no toast, because the subscription comes later. To keep this, construct the achievements store at app start, before ChatPanel subscribes.
22. **Simultaneous unlocks:** only the last toast is visible. Parity.
23. **Mobile `.dialogue-text` cannot be scrolled** (`pointer-events: none`) and the CSS is reused verbatim. Long pages on small screens are cut off. Flag it to the user, or add an override in V2.
24. **RAG is unused in the V1 chat flow,** despite the `Config_Experimental_RAG_*` settings. V2 ChatPanel should not call RAG unless the scope is extended.
25. **`Config_ApiProvider` default mismatch** (0 in submit, 8 elsewhere). It is harmless because `initializeDefaults` writes 8, but keep both defaults for parity.
26. **Timing:** V1's typewriter relies on React re‑rendering between ticks; Rust mutation is immediate. Keep the immediate `step()` call on (re)start and the 10 ms clear‑page delay. Read the old `typeIndex` in the skip‑branch comparison if parity is chosen.
27. **Late or duplicate responses while in `typing`:** `handleApiSuccess` resets the typewriter even if typing is already running (for example during the startup greeting). The running step loop continues with the new text from index 0. Parity: the new text simply replaces the old.

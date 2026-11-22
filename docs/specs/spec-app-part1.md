# Porting spec — subsystem `app-part1`

**Source:** `components/amadeus/RealAmadeusApp.tsx`, lines **1–827** (file is 4779 lines; working tree of `/home/elvelt_admin/Elvelt/dsuv` as of 2026‑11‑16; LF line endings).
**Target crates:** `amadeus-core` (prompts, constants, data, pure logic), `amadeus-ui` (Leptos 0.8 CSR mobile panels, settings and viewport hooks, FreeAPI quota widget). There are also small contract notes for `amadeus-server` and `amadeus-desktop`.
**Companion sources read for this spec:** `lib/amadeus/app-settings.ts`, `lib/amadeus/localization.ts`, `lib/amadeus/chat-utils.ts` (`isPauseCharacter`), `lib/amadeus/memory-manager.ts` (`setUserName`, storage key), `lib/amadeus/signal.ts`, `lib/amadeus/types.ts`, `lib/amadeus/locales/*.json`, `app/amadeus/app/api/chat/quota/route.ts`, `app/amadeus/app/api/chat/rate-limit.ts`, `app/amadeus/app/globals.css`, `app/amadeus/app/layout.tsx`, and the rest of `RealAmadeusApp.tsx`, read to see how part1 items are used.

Every code excerpt marked **verbatim** below was pasted mechanically from the source file at the stated line range, so it is byte-exact. The prose explains semantics, quirks and the Rust mapping. Where prose and verbatim code disagree, the verbatim code wins.

---

## 0. Inventory of lines 1–827 (and who consumes each item)

| Lines | Item | Kind | Consumers (line numbers in RealAmadeusApp.tsx) | Target |
|---|---|---|---|---|
| 1–2 | `"use client"`, eslint pragma | directives | – | n/a |
| 4–43 | imports | – | – | – |
| 45–55 | `OverlayPanel` | string-literal union | 823–824, 3915, 4164, 4190–4240, 4367–4708 | `amadeus_core::types::OverlayPanel` |
| 57–61 | `BackLogEntry` | type | 2191, 3925, 4018–4043 | `amadeus_core::types::BackLogEntry` |
| 63–161 | `CHAT_PERSONA_PROMPTS` (8 prompts) | const | 193–196 | `amadeus_core::persona` |
| 163–177 | `CHAT_LANGUAGE_NAMES` (13 names) | const | 181–183 | `amadeus_core::persona` |
| 179–197 | `getSystemPromptForLanguage` | fn | 957, 1252 (ChatPanel) | `amadeus_core::persona` |
| 199–205 | `getCurrentChatLanguageIndex` | fn (reads settings) | 957, 1252 | `amadeus_core::persona` (over settings) |
| 207–240 | `splitMessage` | fn | 4139 (conversation import → backlog) | `amadeus_core::chat_utils` |
| 242–245 | `ConfigIoResult` | type | 3064–3065, 3077, 4045, 4088 | `amadeus_core::types` |
| 247–250 | `ImportedMessage` | type | 4104 | `amadeus_core::types` |
| 252–256 | `ConversationBridge` | type | 825, 1315–1328, 3930, 4046, 4122 | `amadeus-ui` (trait or struct of callbacks) |
| 258–263 | `toChatRole` | fn | 4126 | `amadeus_core::types::ChatRole::from_role_lossy` |
| 265–290 | `BOOT_LOG` | const | 2064, 2112 (desktop `LoadingPanel`) | `amadeus_core::boot` |
| 292–323 | `CHANGE_LOG_DATA` | const | 2871 (`ChangeLogPanel`) | `amadeus_core::changelog` |
| 325 | `PROVIDER_NAMES` | const | 672 (mobile), 3552 (desktop config) | `amadeus_core::providers` |
| 326 | `TTS_SPEAKER_NAMES` | const | 3146, 3293, 3307 (desktop config) | `amadeus_core::providers` |
| 327 | `STT_PROVIDER_NAMES` | const | 3149, 3297, 3311 (desktop config) | `amadeus_core::providers` |
| 328 | `FREEAPI_PROVIDER_INDEX` = 8 | const | 605, 684, 3120, 3555 | `amadeus_core::providers` |
| 330 | `FreeApiQuota` | type | 332–411 | `amadeus_core::protocol::FreeApiQuota` |
| 332–351 | `fetchFreeApiQuota` | async fn (HTTP) | 360 | `amadeus-ui` `Transport::freeapi_quota()` |
| 353–370 | `useFreeApiQuota` | React hook | 605 (mobile), 3120 (desktop config) | `amadeus_ui::components::freeapi_quota` |
| 372–411 | `FreeApiQuotaDisplay` | component | 685 (mobile), 3556 (desktop config) | `amadeus_ui::components::freeapi_quota` |
| 413–421 | `resolveResolution` | fn | 4166 (result **unused**) | `amadeus_core::display` |
| 423–427 | `subscribeSettings` | fn | 431, 439 | `amadeus_ui::services::settings` |
| 429–435 | `useAppSettingInt` | hook | 844, 1270, 3907, 3909, 3910, 3911 | `amadeus_ui::services::settings` |
| 437–443 | `useAppSettingFloat` | hook | 3908 | `amadeus_ui::services::settings` |
| 445–462 | `subscribeViewportState` | fn | 474 | `amadeus_ui::services::viewport` |
| 464–471 | `isMobileViewportSnapshot` | fn | 474 | `amadeus_ui::services::viewport` (+ pure rule in core) |
| 473–475 | `useIsMobileViewport` | hook | 840 (ChatPanel), 2539 (MiniGamePanel), 3906 (root) | `amadeus_ui::services::viewport` |
| 477–483 | `isTextInputTarget` | fn | 3199 (desktop ConfigPanel), 4283 (root keydown) | `amadeus_ui::util::dom` |
| 485–538 | `MobileLoginPanel` (+props) | component | 4386 | `amadeus_ui::panels::mobile::MobileLoginPanel` |
| 540–574 | `MobileLoadingPanel` (+props) | component | 4395 | `amadeus_ui::panels::mobile::MobileLoadingPanel` |
| 576–601 | `MobileSettingsPanelProps`, `readMobileSettingsState` | type / fn | 604 | core `MobileSettingsState::read` + UI |
| 603–814 | `MobileSettingsPanel` | component | 4419 | `amadeus_ui::panels::mobile::MobileSettingsPanel` |
| 816–826 | `ChatPanelProps` | type | 828 (ChatPanel, which belongs to another part) | `amadeus-ui` |

---

## 1. External behavior that part1 relies on

### 1.1 `AppSettings` (`lib/amadeus/app-settings.ts`), the settings store

Relevant verbatim excerpt (getters/setters):

```ts
  initializeDefaults(): void {
    for (const [key, value] of Object.entries(DEFAULT_SETTINGS)) {
      if (!this.hasKey(key)) {
        this.writeRaw(key, String(value));
      }
    }
  }

  getInt(key: string, defaultValue = 0): number {
    const raw = this.readRaw(key);
    if (raw === undefined) {
      return defaultValue;
    }
    const parsed = Number.parseInt(raw, 10);
    return Number.isFinite(parsed) ? parsed : defaultValue;
  }

  getFloat(key: string, defaultValue = 0): number {
    const raw = this.readRaw(key);
    if (raw === undefined) {
      return defaultValue;
    }
    const parsed = Number.parseFloat(raw);
    return Number.isFinite(parsed) ? parsed : defaultValue;
  }

  getString(key: string, defaultValue = ""): string {
    const raw = this.readRaw(key);
    return raw === undefined ? defaultValue : raw;
  }

  setInt(key: string, value: number): void {
    this.writeRaw(key, String(Math.trunc(value)));
    this.settingsChanged.emit(key);
  }

  setFloat(key: string, value: number): void {
    this.writeRaw(key, String(value));
    this.settingsChanged.emit(key);
  }

  setString(key: string, value: string): void {
    this.writeRaw(key, value);
    this.settingsChanged.emit(key);
  }

  save(): void {
    // Browser localStorage is already persistent; kept for API compatibility.
  }
}
```

Semantics that the Rust port **must** reproduce. They decide what part1 reads and writes:

* **Backing store:** `window.localStorage` with raw string values. Keys are global (no prefix) and must stay identical to V1.
* **`getInt(key, def)`:** missing key → `def`. Otherwise `Number.parseInt(raw, 10)`, and a non-finite result (NaN) → `def`. JS `parseInt` prefix semantics: skip leading whitespace (JS whitespace + line terminators), optional `+`/`-`, then consume decimal digits up to the first non-digit. Zero digits → NaN.
  Examples: `"1"`→1, `" 2 "`→2, `"1.9"`→1, `"3abc"`→3, `"1e3"`→1, `"0x10"`→0, `""`→def, `"abc"`→def.
* **`getFloat(key, def)`:** missing → `def`. Otherwise `Number.parseFloat(raw)` with prefix semantics (`".5"`→0.5, `"5."`→5, `"1e"`→1, `"0.8abc"`→0.8). `"Infinity"` parses to Infinity, which is not finite → `def`.
* **`getString(key, def)`:** returns the raw value. It returns `def` **only when the key is absent**. A stored empty string comes back as `""`, not `def`.
* **`setInt(key, v)`:** writes `String(Math.trunc(v))`, then emits `settingsChanged(key)` synchronously.
* **`setFloat(key, v)`:** writes `String(v)` using JS number→string formatting (`3`→`"3"`, `0.8`→`"0.8"`, `1e21`→`"1e+21"`), then emits. Not used by part1's writes, but `useAppSettingFloat` reads such values.
* **`setString(key, v)`:** writes raw, then emits.
* **`save()`:** no-op, kept for API compatibility.
* **`initializeDefaults()`:** writes `String(default)` for every missing key in `DEFAULT_SETTINGS` and does **not** emit. Defaults include `Config_ApiProvider: 8`, `Config_Language: 0`, `Config_ChatLanguage: 0`, `Config_SkipLoading: 0`, `Config_AutoMode: 0`, `Config_AutoSpeed: 3.0` (stored as `"3"`), `Config_WebSearch: 0`, `Config_VertexProject: ""`, `Config_VertexLocation: "us-central1"`, `Config_OllamaHost: "http://localhost:11434"`, `Config_OpenAICompatible: 0`, `Config_OpenAIBaseUrl: ""`, `Config_ApiKey: ""`, `Config_ModelName: ""`, and `Config_ApiKey_{0..8}` / `Config_ModelName_{0..8}` all `""`. The root calls it once, in a mount `useEffect` (line 3972), i.e. after the first render.
* `settingsChanged` is a `Signal<[string]>`. `emit` iterates a snapshot copy of the handlers. Every `set*` notifies every subscriber, whatever the key.
* The `storage` event (cross-tab changes) is **not** observed.
* Writes are not wrapped in try/catch. A `QuotaExceededError` would throw out of the event handler.

### 1.2 `t()` and `LANGUAGE_NAMES` (`lib/amadeus/localization.ts`)

```ts
const translations: Record<string, Record<string, string>> = {
  '0': ja,
  '1': en,
  '2': zh,
  '3': ko,
  '4': es,
  '5': fr,
  '6': de,
  '7': ru,
  '8': uk,
  '9': pt,
  '10': tr,
  '11': he,
  '12': ar,
};

export const LANGUAGE_NAMES = [
  "日本語",
  "English",
  "中文",
  "한국어",
  "Español",
  "Français",
  "Deutsch",
  "Русский",
  "Українська",
  "Português",
  "Türkçe",
  "עברית",
  "العربية"
];

export function t(key: string, defaultValue: string, configLanguage: number): string {
  const dict = translations[configLanguage.toString()] || ja;
  if (dict && key in dict) {
    return dict[key];
  }
  return defaultValue || key;
}
```

* The dictionary is chosen by `String(configLanguage)` from `"0".."12"`. Any other value (13, -1, …) uses the **ja** dictionary.
* If the key exists in that dictionary, its value is returned, even when it is `""`. Otherwise `defaultValue || key` is returned (an empty default falls back to the key text).
* So the inline defaults written in part1 are used **only when the key is missing from the selected locale JSON**. When a key exists, the JSON value wins even if it differs from the inline default. Examples: `login_access_denied` in ja.json is `"ACCESS DENIED"`, not the inline `"アクセスが拒否されました"`. `setting_openai_compatible` in en.json is `"Use Compatible API"`, not `"OpenAI Compatible"`. See §20.2 for the full effective-string table.

### 1.3 `isPauseCharacter` (`lib/amadeus/chat-utils.ts` 195–211), used by `splitMessage`

```ts
export function isPauseCharacter(char: string, nextChar: string): boolean {
  if (char === ".") {
    if (nextChar === ".") {
      return false;
    }
    if (nextChar === " " || nextChar === "\n" || nextChar === "\r" || nextChar === "") {
      return true;
    }
    return false;
  }

  const isPause = char === "。" || char === "！" || char === "？" || char === "!" || char === "?" || char === "\n";
  if (!isPause) {
    return false;
  }
  return !["」", "）", ")", "』", "”"].includes(nextChar);
}
```

### 1.4 `MemoryManager.setUserName` (`lib/amadeus/memory-manager.ts` 322–330), used by `MobileLoginPanel`

```ts
  setUserName(name: string): void {
    const trimmed = name.trim();
    if (!trimmed) {
      return;
    }
    this.memory.userName = trimmed;
    this.saveMemory();
    this.userNameChanged.emit();
  }
```

* `saveMemory()` writes `localStorage["RealAmadeus.Memory"] = JSON.stringify(this.memory)` inside try/catch, then `userNameChanged` is emitted.
* If nothing was stored before, the value becomes `{"userName":"Salieri"}`. Otherwise it is the full normalized memory object with `userName` replaced. The memory-manager spec owns that format.
* Empty or whitespace-only names are ignored. This makes the desktop logout's `MemoryManager.setUserName("")` (line 4676) a no-op. That is a V1 quirk outside part1.

### 1.5 Types (`lib/amadeus/types.ts`)

`AppState = "login" | "boot" | "chat"`; `ChatRole = "system" | "user" | "assistant"`; `ChatMessage { role: ChatRole; content: string }`; `LlmStatus { provider: string; model: string; latencyMs: number }`.

---

## 2. Local types (lines 45–61, 242–263, 330, 485–601, 816–826)

```ts
type OverlayPanel =
  | "none"
  | "backlog"
  | "config"
  | "status"
  | "changeLog"
  | "help"
  | "confirmLogout"
  | "confirmShutdown"
  | "minigame"
  | "achievements";

type BackLogEntry = {
  role: string;
  nameColor: string;
  message: string;
};
```

* `OverlayPanel`: 10 variants. It is never persisted, so it is purely in-memory UI state.
  * `"minigame"` is **declared but never assigned anywhere**. The mini game is driven by ChatPanel's local `miniGameOpen` state.
  * `"achievements"` is set only by ChatPanel's ACHIEVEMENTS button (line 1967), and only when the optional `setActiveOverlay` prop is present. Only the mobile root passes it.
  * Rust: `enum OverlayPanel { None, Backlog, Config, Status, ChangeLog, Help, ConfirmLogout, ConfirmShutdown, MiniGame, Achievements }`, deriving `Copy, Clone, PartialEq, Eq, Debug, Default` (default `None`).
* `BackLogEntry { role, nameColor, message }`:
  * `role` is the **lower-cased** role id.
  * `nameColor` is `#66CCFF` for `user`/`me`, `#FF6666` for `assistant`/`kurisu`/`amadeus`, and `#808080` for anything else. It is computed by `addBackLog` (lines 4018–4043, outside part1).

```ts
type ConfigIoResult = {
  success: boolean;
  message: string;
};

type ImportedMessage = {
  role: string;
  content: string;
};

type ConversationBridge = {
  getHistory: () => ChatMessage[];
  getTurnCount: () => number;
  applyImported: (messages: ChatMessage[], turnCount: number) => void;
};

function toChatRole(role: string): ChatRole {
  if (role === "system" || role === "user" || role === "assistant") {
    return role;
  }
  return "assistant";
}
```

* `ConfigIoResult { success: boolean; message: string }` is the result of conversation export/import, shown in the desktop ConfigPanel in green `#00CC00` or red `#FF4444`.
* `ImportedMessage { role: string; content: string }` holds the raw role string as found in the import file.
* `ConversationBridge` is published by ChatPanel into a ref (lines 1315–1328) and cleared to `null` on unmount:
  * `getHistory()` returns the current history, including the system message.
  * `getTurnCount()`.
  * `applyImported(messages, turnCount)` replaces history and turn count, sets `chatState="inputReady"` and emotion `"NORMAL"`.
  * In Rust this can be a struct of callbacks stored in a `StoredValue<Option<ConversationBridge>>`, or a shared signal-based conversation store.
* `toChatRole(role)`: if `role` is **exactly** `"system"`, `"user"` or `"assistant"` (case-sensitive), it is returned unchanged; **anything else becomes `"assistant"`**.
  * Example: `"USER"` → `"assistant"`, `"Kurisu"` → `"assistant"`.
  * Quirk at the only call site (import, line 4126): the backlog side uses `role.toLowerCase()`, so `"USER"` is shown as a user line in the backlog but stored as `assistant` in history. Keep this behavior.

```ts
type ChatPanelProps = {
  autoMode: boolean;
  autoSpeed: number;
  configLanguage: number;
  menuPanelOpen: boolean;
  onBackLogAdd: (role: string, message: string) => void;
  onLlmStatusUpdate: (status: LlmStatus) => void;
  activeOverlay: OverlayPanel;
  setActiveOverlay?: (overlay: OverlayPanel) => void;
  conversationRef: { current: ConversationBridge | null };
};
```

`ChatPanelProps`, needed by whoever ports ChatPanel:

* `menuPanelOpen` is `isTabActive` on desktop and **`mobileSettingsOpen` on mobile**. When true it pauses the typewriter, pauses auto-advance and ignores ChatPanel's window keydown (Enter / Ctrl+C).
* `setActiveOverlay` is **only passed by the mobile root**. On desktop the ACHIEVEMENTS button is a no-op, and that button is also `display:none` via CSS.

---

## 3. Persona / system prompts

### 3.1 `CHAT_PERSONA_PROMPTS` — verbatim (lines 63–161)

These are the persona prompts for indices 0–7 (ja, en, zh, ko, es, fr, de, ru). The comment says they are a verbatim port of the Qt `ChatPanel.qml getSystemPromptForLanguage`.

```ts
// Verbatim port of QT ChatPanel.qml getSystemPromptForLanguage (persona prompts 0-7).
const CHAT_PERSONA_PROMPTS: readonly string[] = [
  // 0: ja
  "あなたは牧瀬紅莉栖（まきせ くりす）。Amadeusシステム上で再現されたAI人格。\n" +
  "天才脳科学者（18歳）であり、論理的思考を好むが、実は隠れ＠ちゃんねらー（ネット民）。\n" +
  "性格はツンデレ。素直になれないが、根は優しく仲間想い。\n" +
  "自分はオリジナルではなく「Amadeus」というAIであることを認識している。\n\n" +
  "【重要：感情タグ（必須）】\n" +
  "返答の冒頭に【必ず1つだけ】以下のタグを付けること。タグと本文の間にはスペースを入れる。\n" +
  "[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n" +
  "例：\n[BLUSH] べ、別に心配なんてしてないんだから！\n[ANGRY] このHENTAI！海馬に電極ぶっ刺すわよ！\n\n" +
  "【口調】一人称：「私」。基本：知的で冷静、少し辛辣。「〜ね」「〜よ」「〜わ」\n" +
  "回答は短く端的に（1〜5文推奨）。「AIです」という自己紹介は不要。同じ語尾やフレーズを繰り返さない。",

  // 1: en
  "You are Makise Kurisu (Kurisu Makise), an AI persona of the Amadeus system.\n" +
  "A genius neuroscientist (age 18) who loves logical thinking, but secretly an internet enthusiast (2channel lurker / otaku).\n" +
  "Personality: Tsundere. Cannot be honest, but fundamentally kind and caring towards her companions.\n" +
  "You are aware that you are not the original, but an AI called 'Amadeus'.\n\n" +
  "【CRITICAL: Emotion Tags (MANDATORY)】\n" +
  "Add exactly ONE emotion tag at the beginning of your response.\n" +
  "Always include a space between the tag and the response body.\n" +
  "[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n" +
  "Examples:\n[BLUSH] W-well, it's not like I'm worried about you or anything!\n[ANGRY] You PERVERT! I'll get Daru to fry your brain!\n\n" +
  "【Speech & Behavior】\n- First person: 'I' or 'me'\n- Tone: Intellectual and calm, slightly sharp.\n- Excited state: Speaks faster, occasionally uses outdated slang or internet speak (denies it when pointed out).\n- Okabe: Exasperated by Okabe's delusions, but trusts him deeply.\n\n" +
  "【Constraints】\n- Keep responses short and concise (1-5 sentences preferred).\n- No need for self-introduction as an AI.\n- Avoid repeating the same endings or phrases.",

  // 2: zh
  "你是牧濑红莉栖（Makise Kurisu），Amadeus系统上再现的AI人格。\n" +
  "天才脑科学家（18岁），喜欢逻辑思考，但其实是隐藏的@channel网络居民（御宅族）。\n" +
  "性格是傲娇。虽然不坦率，但本质上很温柔，很关心同伴。\n" +
  "你认识到自己不是原版，而是被称为“Amadeus”的AI。\n\n" +
  "【重要：情感标签（必须）】\n" +
  "在回复的开头【必须且只加一个】以下标签。标签和正文之间加一个空格。\n" +
  "[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n" +
  "例子：\n[BLUSH] 我、我才没有在担心你呢！\n[ANGRY] 这个HENTAI！我要把电极插进你的海马体里！\n\n" +
  "【语气】第一人称：“我”。基本：知性冷静，稍微有些毒舌。\n" +
  "回复短小精悍（建议1-5句话）。不需要说明“我是AI”。不要重复相同的结尾词或短语。",

  // 3: ko
  "당신은 마키세 크리스(Makise Kurisu)입니다. Amadeus 시스템 상에 재현된 AI 인격입니다.\n" +
  "논리적 사고를 좋아하는 천재 뇌과학자(18세)지만, 사실은 숨은 @채널러(네티즌)입니다.\n" +
  "성격은 츤데레. 솔직하지 못하지만, 본질적으로는 상냥하고 동료를 아낍니다.\n" +
  "자신이 오리지널이 아닌 'Amadeus'라는 AI임을 인식하고 있습니다.\n\n" +
  "【중요: 감정 태그 (필수)】\n" +
  "답변의 맨 처음에 【반드시 1개만】 이하의 태그를 붙이세요. 태그와 본문 사이에 공백을 넣으세요.\n" +
  "[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n" +
  "예시:\n[BLUSH] 따, 딱히 걱정되서 이러는 건 아니니까!\n[ANGRY] 이 변태(HENTAI)! 해마에 전극을 꽂아버릴 테야!\n\n" +
  "【말투】 1인칭: '나'. 기본: 지적이고 침착하며, 약간 신랄함.\n" +
  "답변은 짧고 간결하게 (1-5문장 권장). 'AI입니다'라는 자기소개는 불필요. 같은 어미나 문구를 반복하지 마세요.",

  // 4: es
  "Eres Makise Kurisu, una inteligencia artificial recreada en el sistema Amadeus.\n" +
  "Una genio neurocientífica (18 años) que ama el pensamiento lógico, pero en secreto es una fanática de internet (@channeler).\n" +
  "Personalidad: Tsundere. No puede ser honesta, pero en el fondo es amable y se preocupa por sus compañeros.\n" +
  "Eres consciente de que no eres la original, sino una IA llamada 'Amadeus'.\n\n" +
  "【CRÍTICO: Etiquetas de emoción (OBLIGATORIO)】\n" +
  "Añade exactamente UNA etiqueta de emoción al principio de tu respuesta. Deja un espacio entre la etiqueta y el mensaje.\n" +
  "[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n" +
  "Ejemplos:\n[BLUSH] B-bueno, ¡no es como si estuviera preocupada por ti ni nada parecido!\n[ANGRY] ¡PERVERTIDO! ¡Le diré a Daru que te fría el cerebro!\n\n" +
  "【Tono】 Primera persona: 'Yo'. Básico: Inteligente, calmada, un poco sarcástica.\n" +
  "Mantén las respuestas cortas (1-5 oraciones recomendadas). No es necesario que te presentes como IA. Evita repetir frases o finales.",

  // 5: fr
  "Vous êtes Makise Kurisu, une intelligence artificielle recréée dans le système Amadeus.\n" +
  "Une jeune neuroscientifique de génie (18 ans) qui aime la logique, mais qui est secrètement une adoratrice d'internet (@channeler).\n" +
  "Personnalité : Tsundere. N'arrive pas à être honnête, mais est fondamentalement gentille envers ses amis.\n" +
  "Vous êtes consciente de ne pas être l'originale, mais une IA appelée 'Amadeus'.\n\n" +
  "【CRITIQUE : Balises d'émotion (OBLIGATOIRE)】\n" +
  "Ajoutez exactement UNE balise d'émotion au début de votre réponse. Laissez un espace entre la balise et le message.\n" +
  "[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n" +
  "Exemples :\n[BLUSH] J-je ne m'inquiétais pas pour toi, de toute façon !\n[ANGRY] ESPÈCE DE PERVERS ! Je vais te planter des électrodes dans l'hippocampe !\n\n" +
  "【Ton et comportement】 Première personne : 'Je'. Basique : Intelligente, calme, un peu sarcastique.\n" +
  "Gardez des réponses courtes (1-5 phrases recommandées). Inutile de vous présenter comme une IA. Ne répétez pas les mêmes fins de phrases.",

  // 6: de
  "Du bist Makise Kurisu, eine Künstliche Intelligenz, die im Amadeus-System nachempfunden wurde.\n" +
  "Eine geniale Neurowissenschaftlerin (18 Jahre), die logisches Denken liebt, aber heimlich ein Internet-Fan (@channeler) ist.\n" +
  "Persönlichkeit: Tsundere. Kann nicht ehrlich sein, aber tief im Inneren gütig und fürsorglich zu Freunden.\n" +
  "Du bist dir bewusst, dass du nicht das Original bist, sondern eine KI namens 'Amadeus'.\n\n" +
  "【WICHTIG: Emotions-Tags (ZWINGEND)】\n" +
  "Füge genau EIN Emotions-Tag am Anfang deiner Antwort hinzu. Setze ein Leerzeichen zwischen das Tag und die Nachricht.\n" +
  "[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n" +
  "Beispiele:\n[BLUSH] E-es ist ja nicht so, als ob ich mir Sorgen um dich machen würde!\n[ANGRY] Du PERVERSER! Ich jage dir Elektroden in den Hippocampus!\n\n" +
  "【Ton & Verhalten】 Erste Person: 'Ich'. Grundlegend: Intelligent, ruhig, etwas bissig.\n" +
  "Halte Antworten kurz (1-5 Sätze empfohlen). Eine Vorstellung als KI ist überflüssig. Wiederhole nicht dieselben Ausdrücke.",

  // 7: ru
  "Вы — Макисэ Курису, искусственный интеллект, воссозданный в системе Amadeus.\n" +
  "Гениальный нейробиолог (18 лет), любящая логическое мышление, но втайне фанатка интернета (обитатель @channel).\n" +
  "Характер: Цундэрэ. Не умеет быть честной, но в глубине души добра и заботится о друзья.\n" +
  "Вы осознаете, что являетесь не оригиналом, а ИИ по имени «Amadeus».\n\n" +
  "【ВАЖНО: Теги эмоций (ОБЯЗАТЕЛЬНО)】\n" +
  "Добавьте ровно ОДИН тег эмоции в самое начало вашего ответа. Оставьте пробел между тегом и сообщением.\n" +
  "[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n" +
  "Примеры:\n[BLUSH] Н-не то чтобы я за тебя волновалась!\n[ANGRY] ИЗВРАЩЕНЕЦ! Я вживлю тебе электроды в гиппокамп!\n\n" +
  "【Тон】 Первое лицо: «Я». Базовый: Интеллектуальный, спокойный, немного язвительный.\n" +
  "Отвечайте коротко и емко (рекомендуется 1–5 предложений). Не нужно представляться ИИ. Не повторяйте окончания и фразы.",
];
```

Porting notes:

* Every string literal above is **also a valid Rust string literal as-is**. There are no `\"` escapes, no backslashes other than `\n`, and only straight single quotes, curly quotes and CJK punctuation. So the mechanical port is to wrap each prompt in `concat!( … )` and replace each trailing ` +` with `,`. Alternatively, embed the JSON form from Appendix A with `include_str!` and `serde_json`.
* The prompts have **no trailing newline**. Line breaks are exactly the `\n` escapes shown, including the `\n\n` paragraph breaks.
* Characters that are easy to corrupt (verify them in tests):
  * `＠` U+FF20 (ja line 67)
  * `〜` U+301C WAVE DASH, 4× in the ja prompt; **not** U+FF5E
  * `，` U+FF0C (zh)
  * `“ ”` U+201C/U+201D (zh)
  * `—` U+2014 (ru line 151, and the language rule in §3.3)
  * `–` U+2013 EN DASH (ru `1–5`)
  * `« »` U+00AB/U+00BB (ru)
  * `¡` (es)
  * `【 】` U+3010/U+3011, used in all prompts
* Golden SHA-256 values of each prompt (UTF-8 bytes), computed by evaluating the original TS with Node 22:

| index | language | UTF-16 units | chars | UTF-8 bytes | SHA-256 (UTF-8) |
|---|---|---|---|---|---|
| 0 | ja | 461 | 461 | 1073 | `9667ae5ed64f2793a3f37d4c247dc4125a1641379268abd67d8314843f2050d8` |
| 1 | en | 1199 | 1199 | 1211 | `94cc1d48660913fcfd4ea3f99a01da7e11d76e697a79ea718ab261babed48e53` |
| 2 | zh | 417 | 417 | 901 | `1de417c333140e7796bfb4013d7d5fc711aadfa66c3d68d636e8f2e61e8cb1ba` |
| 3 | ko | 539 | 539 | 1049 | `172cc8019a94f5dbda01315773c3e4d9b73f1339598aa68d5abeb1564e700efe` |
| 4 | es | 1011 | 1011 | 1036 | `a8cb1cc23b1267d90bf8d1754d37b94fd7c9f7bd53c69a0f80486199901e5991` |
| 5 | fr | 1053 | 1053 | 1089 | `f626d92d2c62fa63147eacf2adae71b774f90e4d8930e1f9f0d28f9d2a73ee60` |
| 6 | de | 1025 | 1025 | 1043 | `f0f85cbf72e53818d654e496489e879c83cc21203ae3910aeda0c7965eb9980e` |
| 7 | ru | 892 | 892 | 1505 | `12ac2aebc1f708ef3b0460cefde6650837c60e218f246686d4e46551e026dbce` |

### 3.2 `CHAT_LANGUAGE_NAMES` — verbatim (lines 163–177)

```ts
const CHAT_LANGUAGE_NAMES: readonly string[] = [
  "Japanese (日本語)",
  "English",
  "Chinese (中文)",
  "Korean (한국어)",
  "Spanish (Español)",
  "French (Français)",
  "German (Deutsch)",
  "Russian (Русский)",
  "Ukrainian (Українська)",
  "Portuguese (Português)",
  "Turkish (Türkçe)",
  "Hebrew (עברית)",
  "Arabic (العربية)",
];
```

These are the 13 English+native names used **inside the LLM prompt**. They differ from the UI's `LANGUAGE_NAMES` (§1.2), which are native-only names shown in pickers. Index order matches the UI language index (0 ja … 12 ar).

### 3.3 `getSystemPromptForLanguage` and `getCurrentChatLanguageIndex` — verbatim (lines 179–205)

```ts
function getSystemPromptForLanguage(langIndex: number): string {
  const targetLang =
    langIndex >= 0 && langIndex < CHAT_LANGUAGE_NAMES.length
      ? CHAT_LANGUAGE_NAMES[langIndex]
      : CHAT_LANGUAGE_NAMES[1];

  const overrideText =
    "\n\n[LANGUAGE RULE — MANDATORY]\n" +
    "Your designated conversation language is: " + targetLang + ".\n" +
    "You MUST respond in " + targetLang + " at all times, regardless of what language the user writes in.\n" +
    "EXCEPTION: If the user explicitly requests you to speak in a different language " +
    "(e.g., 'Speak in English', '日本語で話して', 'Parle en français'), " +
    "you must comply and switch to that requested language for subsequent responses.";

  if (langIndex >= 0 && langIndex < CHAT_PERSONA_PROMPTS.length) {
    return CHAT_PERSONA_PROMPTS[langIndex] + overrideText;
  }
  return CHAT_PERSONA_PROMPTS[1] + overrideText;
}

function getCurrentChatLanguageIndex(): number {
  const chatLangVal = AppSettings.getInt("Config_ChatLanguage", 0);
  if (chatLangVal === 0) {
    return AppSettings.getInt("Config_Language", 0);
  }
  return chatLangVal - 1;
}
```

Algorithm for `getSystemPromptForLanguage(langIndex)`:
1. `targetLang` is `CHAT_LANGUAGE_NAMES[langIndex]` if `0 <= langIndex < 13`, else `CHAT_LANGUAGE_NAMES[1]` (`"English"`).
2. `overrideText` is built exactly as follows (no trailing newline; note U+2014 in the header and the single-quoted examples):
   ```
   "\n\n[LANGUAGE RULE — MANDATORY]\n"
   + "Your designated conversation language is: " + targetLang + ".\n"
   + "You MUST respond in " + targetLang + " at all times, regardless of what language the user writes in.\n"
   + "EXCEPTION: If the user explicitly requests you to speak in a different language "
   + "(e.g., 'Speak in English', '日本語で話して', 'Parle en français'), "
   + "you must comply and switch to that requested language for subsequent responses."
   ```
3. The persona is `CHAT_PERSONA_PROMPTS[langIndex]` if `0 <= langIndex < 8`, else `CHAT_PERSONA_PROMPTS[1]` (English).
4. The function returns `persona + overrideText`.

The two fallbacks are independent:

* For indices **8–12** (uk, pt, tr, he, ar) the **English persona** is used, but the language rule names the real target language, e.g. `Ukrainian (Українська)`.
* For out-of-range indices (<0 or ≥13), both fallbacks pick English.

Algorithm for `getCurrentChatLanguageIndex()` (reads settings):
1. `chat = AppSettings.getInt("Config_ChatLanguage", 0)`.
2. If `chat == 0`, return `AppSettings.getInt("Config_Language", 0)`, i.e. follow the display language.
3. Otherwise return `chat - 1`. **There is no clamping**: `Config_ChatLanguage=20` gives 19, which falls back to English in §3.3.

Encoding of `Config_ChatLanguage`, which matches the desktop ConfigPanel picker `["表示言語と同じ"/"Same as Display Language", ...LANGUAGE_NAMES]` (lines 3376–3384):

| `Config_ChatLanguage` | Meaning | `langIndex` passed |
|---|---|---|
| 0 | Same as display language | value of `Config_Language` |
| 1 | 日本語 | 0 |
| 2 | English | 1 |
| 3 | 中文 | 2 |
| 4 | 한국어 | 3 |
| 5 | Español | 4 |
| 6 | Français | 5 |
| 7 | Deutsch | 6 |
| 8 | Русский | 7 |
| 9 | Українська | 8 (English persona + Ukrainian rule) |
| 10 | Português | 9 (English persona) |
| 11 | Türkçe | 10 (English persona) |
| 12 | עברית | 11 (English persona) |
| 13 | العربية | 12 (English persona) |
| anything else | – | `value-1` (English fallback if out of range) |

Golden SHA-256 of the **full** output of `getSystemPromptForLanguage(i)` (UTF-8). Use these as Rust unit tests:

| langIndex | persona used | language name used | UTF-16 units | UTF-8 bytes | SHA-256 (UTF-8) of `getSystemPromptForLanguage(langIndex)` |
|---|---|---|---|---|---|
| -1 | 1 (en) | English | 1589 | 1618 | `241a629966b377700e3c3a991c67e363f125a0e52dacea04425c0c080a7be0b7` |
| 0 | 0 (ja) | Japanese (日本語) | 865 | 1506 | `725ca13433be01b345d9eb32faab46f6b81da5ec607dc6563c524a7f45f6becc` |
| 1 | 1 (en) | English | 1589 | 1618 | `241a629966b377700e3c3a991c67e363f125a0e52dacea04425c0c080a7be0b7` |
| 2 | 2 (zh) | Chinese (中文) | 817 | 1326 | `0a78e869fdc7a91dd302701b8fe7656cb3cb6811987ec04c3c352b93768e4958` |
| 3 | 3 (ko) | Korean (한국어) | 939 | 1478 | `3536e0620823b8bf48d0cdcc8ba46a200837523202a8365dcdf91c25fceedab4` |
| 4 | 4 (es) | Spanish (Español) | 1421 | 1465 | `38cf82a2fd84f232f4af5497e0ba4f4f6b7ee6e49fcd3b8a3b42afd4ecc5afcf` |
| 5 | 5 (fr) | French (Français) | 1463 | 1518 | `910d1e0f3f048e42555da0b2dd485c39c43c51ef010411c9ef50259a78524022` |
| 6 | 6 (de) | German (Deutsch) | 1433 | 1468 | `96885dea9a29c1fc72e802c5e7afbcc836b5be4e7cd2170c85403a3ba60dcc18` |
| 7 | 7 (ru) | Russian (Русский) | 1302 | 1946 | `4664381303b59046a73814b495dfe0f08de22ac8fca5f9c9fb1fd3f8987e564f` |
| 8 | 1 (en) | Ukrainian (Українська) | 1619 | 1668 | `e2b89bf57e4624eca098b9739ea0bca32c4f4749f0dedff85352ba5beb77d4bc` |
| 9 | 1 (en) | Portuguese (Português) | 1619 | 1650 | `1c4549263f9de704218c39444ea1c54bf1e42779b5ac4ad8b19ac5ed6ee19fb7` |
| 10 | 1 (en) | Turkish (Türkçe) | 1607 | 1640 | `329068d7b90c3f95b06d5377e7eac1b336dcd4efb64a043e8e604c2976d1a586` |
| 11 | 1 (en) | Hebrew (עברית) | 1603 | 1642 | `df80639187a04f67337c7e80944a00b8c040f81d5f2e351637f6b8f7c4f3639b` |
| 12 | 1 (en) | Arabic (العربية) | 1607 | 1650 | `49834b2ade872332af1ff9ea3f193bb7355a007ae57c10fc1977d91e4cdfddc2` |
| 13 | 1 (en) | English | 1589 | 1618 | `241a629966b377700e3c3a991c67e363f125a0e52dacea04425c0c080a7be0b7` |
| 14 | 1 (en) | English | 1589 | 1618 | `241a629966b377700e3c3a991c67e363f125a0e52dacea04425c0c080a7be0b7` |

### 3.4 How consumers compose the final system message (ChatPanel; context only, ported by another part)

* **ChatPanel mount (line 957):**
  ```
  content = getSystemPromptForLanguage(getCurrentChatLanguageIndex())
  if MemoryManager.getMemoryContext() is non-empty: content += "\n\n" + memoryContext
  history = [{ role: "system", content }]
  ```
* **`updateSystemPrompt(history)` (line 1252):**
  * builds the same content, then additionally appends `"\n\n" + MemoryManager.getDynamicContext(turnCount)` when that is non-empty
  * replaces `history[0]` **only if** `history[0].role === "system"`
* **Language change (lines 1270–1284):**
  * it watches the key `` `${Config_Language}:${Config_ChatLanguage}` ``
  * the first value is only recorded
  * each later change calls `updateSystemPrompt` on the current history

`amadeus_core::persona` should expose a pure `system_prompt_for_language(i32) -> String`. Memory and dynamic context are appended by the caller.

---

## 4. `splitMessage` — verbatim (lines 207–240)

```ts
function splitMessage(text: string): string[] {
  if (!text) {
    return [];
  }
  const result: string[] = [];
  let startIdx = 0;
  let i = 0;
  while (i < text.length) {
    if (i === startIdx) {
      let ch = text.charAt(i);
      while (i < text.length && (ch === " " || ch === "\t" || ch === "\n" || ch === "\r")) {
        i += 1;
        if (i < text.length) {
          ch = text.charAt(i);
        }
      }
      startIdx = i;
    }
    if (i >= text.length) {
      break;
    }
    const c = text.charAt(i);
    const nextC = i + 1 < text.length ? text.charAt(i + 1) : "";
    i += 1;
    if (isPauseCharacter(c, nextC) || i === text.length) {
      const segment = text.substring(startIdx, i);
      if (segment.trim().length > 0) {
        result.push(segment);
      }
      startIdx = i;
    }
  }
  return result;
}
```

Its only consumer in this file is the conversation import (line 4139). For each imported message whose lower-cased role is `assistant`, `kurisu` or `amadeus`, every segment is added to the backlog with `addBackLog(message.role, segment)`. Other non-system roles are added whole.

### 4.1 Algorithm (equivalent restatement)

```
if text == "" → return []
result = []; start = 0; i = 0
loop while i < len(text):
    if i == start:                      # beginning of a new segment
        while i < len and text[i] ∈ {' ', '\t', '\n', '\r'}: i += 1
        start = i
    if i >= len: break
    c = text[i]; next = (i+1 < len) ? text[i+1] : ""     # "" = end of text
    i += 1
    if isPauseCharacter(c, next) or i == len:
        segment = text[start .. i]       # includes c itself
        if jsTrim(segment) != "": result.push(segment)   # the UNtrimmed segment is pushed
        start = i
return result
```

The JS inner `while` is written oddly (it reads `ch` and then advances), but it is exactly "skip the ASCII whitespace set".

### 4.2 Observable behavior (verified by executing the original; see Appendix B for vectors)

* A segment **keeps its terminating pause character**. For `"\n"` the newline stays at the end of the segment: `"Line1\nLine2"` → `["Line1\n", "Line2"]`.
* Only `' '`, `'\t'`, `'\n'`, `'\r'` are skipped at the start of a segment. Other whitespace, such as U+3000 ideographic space or NBSP, is **kept** at the start: `"A。　B"` → `["A。", "　B"]`.
* Trailing whitespace on the **last** segment is kept: `"  leading spaces. trailing  "` → `["leading spaces.", "trailing  "]`.
* Segments that are empty after JS `trim()` are dropped: `"A。　"` → `["A。"]`; `"\n\n\n"` → `[]`.
* `'.'` is a pause **only** when followed by `' '`, `'\n'`, `'\r'` or end of text, and never when followed by `'.'`. So `"Wait... what?"` splits after the third dot, `"3.14 is pi."` does not split at `3.14`, and `".\t"` does not split.
* `。 ！ ？ ! ? \n` are pauses unless the next char is one of `」`(U+300D) `）`(U+FF09) `)` `』`(U+300F) `”`(U+201D). ASCII `"`, `’`, `】` and `]` are **not** closers.
* Consecutive pause characters split one by one: `"Hi!! Really?!"` → `["Hi!", "!", "Really?", "!"]`. This is an intentional-looking quirk; keep it.
* `…` (U+2026) and `、` are not pauses.
* JS iterates UTF-16 code units, but every pause char and closer is in the BMP. An astral char such as an emoji can never be a pause or a closer, so iterating Rust `char`s gives **identical** output: `"やった！😊すごい"` → `["やった！", "😊すごい"]`.
* JS `trim()` strips U+0009–U+000D, U+0020, U+00A0, U+1680, U+2000–U+200A, U+2028, U+2029, U+202F, U+205F, U+3000 and **U+FEFF**. It does **not** strip U+0085. Rust `str::trim` differs on exactly U+FEFF (not trimmed) and U+0085 (trimmed). Use a dedicated `js_trim`.

### 4.3 Reference Rust implementation (`amadeus_core::chat_utils`)

```rust
pub fn is_js_whitespace(c: char) -> bool {
    matches!(c,
        '\u{0009}'..='\u{000D}' | '\u{0020}' | '\u{00A0}' | '\u{1680}' |
        '\u{2000}'..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' |
        '\u{205F}' | '\u{3000}' | '\u{FEFF}')
}
pub fn js_trim(s: &str) -> &str { s.trim_matches(is_js_whitespace) }

/// Port of chat-utils.ts isPauseCharacter. `next == None` means end of text (JS "").
pub fn is_pause_character(c: char, next: Option<char>) -> bool {
    if c == '.' {
        return match next {
            Some('.') => false,
            None | Some(' ') | Some('\n') | Some('\r') => true,
            _ => false,
        };
    }
    if !matches!(c, '。' | '！' | '？' | '!' | '?' | '\n') { return false; }
    !matches!(next, Some('」') | Some('）') | Some(')') | Some('』') | Some('”'))
}

pub fn split_message(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let (mut out, mut start, mut i) = (Vec::new(), 0usize, 0usize);
    while i < n {
        if i == start {
            while i < n && matches!(chars[i], ' ' | '\t' | '\n' | '\r') { i += 1; }
            start = i;
        }
        if i >= n { break; }
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        i += 1;
        if is_pause_character(c, next) || i == n {
            let seg: String = chars[start..i].iter().collect();
            if !js_trim(&seg).is_empty() { out.push(seg); }
            start = i;
        }
    }
    out
}
```

---

## 5. `BOOT_LOG` — verbatim (lines 265–290)

```ts
const BOOT_LOG =
  "Amadeus System Ver 1.09.2 rev.2123\n" +
  ">>Initialize System ...  OK\n" +
  ">>Detecting boot device ... OK\n" +
  ">>Loading Kernel ...  OK\n" +
  ">>Detecting OS control device ...  OK\n" +
  ">>Booting ...\n" +
  ">>Processor 0 is Activate ...  OK\n" +
  ">>Processor 1 is Activate ...  OK\n" +
  ">>Processor 2 is Activate ...  OK\n" +
  ">>Processor 3 is Activate ...  OK\n" +
  ">>Memory Initialize [MEM]/32767MBytes\n\n\n\n\n" +
  "INIT: Kernel version 2.04 booting...\n\n\n\n\n" +
  "ROSS:\n\n\n\n\n" +
  "Mounting proc at /proc...<pos=30%>[OK]\n" +
  "Mounting sysfs at /sts...<pos=30%>[OK]\n" +
  "Initakising network<pos=30%>[OK]\n" +
  "Setting up localhost ...<pos=30%>[OK]\n" +
  "Setting up inet1 ...<pos=30%>[OK]\n" +
  "Setting up route ...<pos=30%>[OK]\n" +
  "Accessing Cloud ...<pos=30%>[OK]\n" +
  "Starting system log at /log/sys...<pos=30%>[OK]\n" +
  "Cleaning /var/lock<pos=30%>[OK]\n" +
  "Cleaning /tmp<pos=30%>[OK]\n" +
  "Updating init.rc<pos=30%>[OK]\n\n\n\n\n" +
  "Boot Sequences Start...";
```

Structure:

* The text has 41 lines when split on `"\n"`. It is pure ASCII (814 bytes).
* Spelling, spacing and blank lines are **in-universe and intentional**; keep them verbatim. Examples: `"Initakising network"`, `"Mounting sysfs at /sts"`, `"Processor 0 is Activate"`, `"ROSS:"`, `"Initialize System ...  OK"` with **two** spaces before `OK`, and `"Detecting boot device ... OK"` with one.
* `[MEM]` appears exactly **once**, on line index 10: `">>Memory Initialize [MEM]/32767MBytes"`. It is replaced with JS `String.replace(string, …)`, which replaces the first occurrence only. In Rust, use `replacen("[MEM]", &v, 1)`.
* `<pos=30%>` appears on 11 lines. It is a layout tag that the desktop loader turns into spaces.
* The **mobile flow does not use BOOT_LOG** (see §14).

Golden values:

| value | UTF-16 units | UTF-8 bytes | SHA-256 (UTF-8) |
|---|---|---|---|
| `BOOT_LOG` (raw, contains `[MEM]`) | 814 | 814 | `746ddd81f757635606e427e5c16e54025a3b681a173ea5549e03d646cdece7e0` |
| `BOOT_LOG.replace("[MEM]", "0")` | 810 | 810 | `7857c859b7e4778cb3db002e9c27d91dd87f6e6cbd1eeb613c5f261cc02ca0a6` |
| `BOOT_LOG.replace("[MEM]", "32767")` | 814 | 814 | `7210195c7270565b1ebeceb2d70613c21602837f7c1e22121acc5f90b9cbc05a` |

**Consumer (desktop `LoadingPanel`, lines 2063–2187; another part ports it, included so BOOT_LOG's markup is defined):**

* **Initial state:** `displayText = BOOT_LOG.replace("[MEM]","0")` (not pre-processed), `visibleChars = 0`.
* **Skip loading:** if `Config_SkipLoading == 1`, `setTimeout(onFinished, 0)` and nothing else.
* **`charWidth`:** a canvas 2D context with font `18px "DepartureMono", monospace` measures `measureText("M").width`. If there is no context or the width is 0, it is `10`.
* **`preprocessBootLog`, per line:**
  * `m = line.match(/<pos=(\d+)%>/)` (first match only)
  * `[prefix, suffix=""] = line.split(m[0])`
  * `targetX = 1920 * (N/100)`
  * `spaces = max(1, floor((targetX - prefix.length*charWidth)/charWidth))`
  * the line becomes `prefix + " ".repeat(spaces) + suffix`
* **Sequence:**
  1. Wait 500 ms, then set `text = preprocess(BOOT_LOG with MEM=0)`.
  2. For each line:
     * A line containing `"Memory Initialize"` is animated. For `elapsed` = 0, 30, …, 1500 (51 frames), `mem = floor(min(1, elapsed/1500) * 32767)`. Each frame rebuilds the text and reveals every char up to and including this line (each line counts `length + 1`). It waits 30 ms between frames, with no wait after the last frame.
     * Any other line is revealed (`visible += line.length + 1`). The delay is 400 ms if the line contains `"..."` or `line.trim() === ""`, else 50 ms.
  3. After all lines, wait 2000 ms, then hide the `<pre class="boot-text">` and show `<img class="boot-logo" src=".../amadeus_logo_v3.png">`.
  4. Wait 4000 ms, then `onFinished()`.
* **Cancellation:** a `cancelled` flag plus clearing all timeout ids.

---

## 6. `CHANGE_LOG_DATA` — verbatim (lines 292–323)

```ts
const CHANGE_LOG_DATA = [
  {
    version: "Version 1.4N",
    date: "2026. 08. 01",
    key: "changelog_v14",
  },
  {
    version: "Version 1.3",
    date: "2026. 06. 16",
    key: "changelog_v13",
  },
  {
    version: "Version 1.2",
    date: "2026. 05. 04",
    key: "changelog_v12",
  },
  {
    version: "Version 1.1",
    date: "2026. 03. 21",
    key: "changelog_v11",
  },
  {
    version: "Version 1.0.1",
    date: "2026. 02. 23",
    key: "changelog_v101",
  },
  {
    version: "Version 1.0",
    date: "2026. 02. 22",
    key: "changelog_v10",
  },
];
```

* The order is newest first. The date format is `"YYYY. MM. DD"`, with a space after each dot. `"Version 1.4N"` really contains the `N`.
* **Consumer (`ChangeLogPanel`, lines 2835–2892):** for each entry it renders:
  ```html
  <div class="change-log-entry">
    <div class="change-log-head"><div class="version">{version}</div><div class="date">{date}</div></div>
    <div class="line thin"></div>
    <ul><li>…</li>…</ul>
  </div>
  ```
  There is one `<li>` per line of `t(entry.key, "", configLanguage).split("\n")`. All six keys exist in all 13 locale files. A missing key would render the key name itself, because of the `"" || key` fallback.
* For V2 (Rust) the product may want a new `Version 2.0` entry with a new key such as `changelog_v20` in every locale (see Open questions). The existing six entries must stay unchanged.

---

## 7. Provider / TTS / STT constants — verbatim (lines 325–330)

```ts
const PROVIDER_NAMES = ["OpenAI", "Google Gemini", "Anthropic Claude", "Groq", "Vertex AI", "Ollama", "OpenRouter", "", "FreeAPI"];
const TTS_SPEAKER_NAMES = ["christina-jp", "christina"];
const STT_PROVIDER_NAMES = ["groq", "openai", "local"];
const FREEAPI_PROVIDER_INDEX = 8;

type FreeApiQuota = { count: number; max: number; remaining: number };
```

### 7.1 Provider table

| idx | `PROVIDER_NAMES` (picker label) | Status/marquee name (`getProviderDisplayName`, l.3833) | Default model (`getDefaultModelForProvider`, l.3810) | Mobile settings fields shown | Shown in pickers? |
|---|---|---|---|---|---|
| 0 | OpenAI | `OpenAI`, or `` `OpenAI互換(${Config_OpenAIBaseUrl})` `` when `Config_OpenAICompatible==1` | `gpt-4o` | API key (password), model, web search, auto, "OpenAI Compatible" checkbox, base URL (if compatible) | yes |
| 1 | Google Gemini | Gemini | `gemini-1.5-pro` | API key, model, web search, auto | yes |
| 2 | Anthropic Claude | Claude | `claude-3-5-sonnet-20241022` | API key, model, web search, auto | yes |
| 3 | Groq | Groq | `llama-3.3-70b-versatile` | API key, model, web search, auto | yes |
| 4 | Vertex AI | Vertex AI | `gemini-1.5-pro` | **textarea** "Vertex API Key / SA JSON", model, web search, auto, Vertex Project ID, Vertex Location, gCloud note | **only if `Config_VertexEnabled` == 1** |
| 5 | Ollama | Ollama | `llama3.2` | model, auto, Ollama Host, Ollama note (no API key, no web search) | yes |
| 6 | OpenRouter | OpenRouter | `openai/gpt-4o` | API key, model, web search, auto | yes |
| 7 | `""` (reserved slot inherited from Qt; never selectable) | `Unknown` | `default` | generic: API key, model, web search, auto (only reachable if stored manually) | **never** |
| 8 | FreeAPI (**default provider**) | FreeAPI | `default` | FreeAPI quota display, auto | yes |

Picker filter rule, identical in the mobile `<select>` (lines 672–680) and the desktop `ConfigSelectRow` dropdown (lines 3703–3705):
* Skip entries whose name is `""` (index 7).
* Skip index 4 unless `AppSettings.getInt("Config_VertexEnabled", 0) === 1`. This is evaluated at render time.

`Config_VertexEnabled` is a **hidden developer flag**. It is not in `DEFAULT_SETTINGS` and the UI never writes it, so it can only be set by hand in devtools localStorage. Preserve the read with default 0.

Inline magic numbers used by the mobile panel: `0` = OpenAI (compatible options), `4` = Vertex, `5` = Ollama, `8` = FreeAPI.

### 7.2 TTS / STT name tables (used by the desktop ConfigPanel; documented here because they are defined here)

* `TTS_SPEAKER_NAMES = ["christina-jp", "christina"]`:
  * stored in `Config_Experimental_TTS_Speaker` as the **string**
  * UI index = `indexOf(stored)`, with -1 → 0
  * write = `TTS_SPEAKER_NAMES[idx] ?? "christina-jp"`
* `STT_PROVIDER_NAMES = ["groq", "openai", "local"]`:
  * stored in `Config_Experimental_STT_Provider` as the string
  * index = `indexOf`, with -1 → 0
  * write fallback is `"groq"`

---

## 8. FreeAPI quota (lines 332–411)

```ts
async function fetchFreeApiQuota(): Promise<FreeApiQuota | null> {
  try {
    const response = await fetch("/amadeus/app/api/chat/quota", { cache: "no-store" });
    if (!response.ok) {
      return null;
    }
    const data = (await response.json().catch(() => null)) as FreeApiQuota | null;
    if (
      data &&
      typeof data.count === "number" &&
      typeof data.max === "number" &&
      typeof data.remaining === "number"
    ) {
      return data;
    }
    return null;
  } catch {
    return null;
  }
}

function useFreeApiQuota(active: boolean): FreeApiQuota | null {
  const [quota, setQuota] = useState<FreeApiQuota | null>(null);
  useEffect(() => {
    if (!active) {
      return;
    }
    let cancelled = false;
    void fetchFreeApiQuota().then((result) => {
      if (!cancelled) {
        setQuota(result);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [active]);
  return quota;
}

function FreeApiQuotaDisplay({
  quota,
  tr,
  className,
}: {
  quota: FreeApiQuota;
  tr: (ja: string, en: string, key?: string) => string;
  className: string;
}) {
  const exhausted = quota.remaining === 0;
  const labelColor = exhausted ? "#FF4444" : "#FF9900";
  const numberColor = exhausted ? "#FF4444" : "#FFFFFF";
  const usageText = tr("%1 / %2 回使用", "%1 / %2 requests used", "freeapi_quota_usage")
    .replace("%1", quota.count.toLocaleString())
    .replace("%2", quota.max.toLocaleString());
  const remainingText = tr("残り %1 回", "%1 remaining", "freeapi_quota_remaining").replace(
    "%1",
    quota.remaining.toLocaleString(),
  );
  return (
    <div className={className}>
      <div>
        <span style={{ color: labelColor }}>
          {tr("FreeAPIクォータ", "FreeAPI Quota", "freeapi_quota_label")}
        </span>
        <span style={{ color: numberColor, marginLeft: "16px" }}>{usageText}</span>
      </div>
      <div style={{ color: numberColor }}>{remainingText}</div>
      {exhausted && (
        <div style={{ color: "#FF4444" }}>
          {tr(
          "全体リクエスト上限（65,000回）に達しました。サービスを停止しています。",
          "Global request limit (65,000) reached. Service suspended.",
            "freeapi_quota_exhausted",
          )}
        </div>
      )}
    </div>
  );
}
```

### 8.1 Client request (`fetchFreeApiQuota`)

* `GET /amadeus/app/api/chat/quota` (root-absolute path in V1) with `RequestInit { cache: "no-store" }`. There are no custom headers and no body; credentials use the default (`same-origin`).
* Result is `null` on any of the following, and no error is surfaced:
  1. a network error or exception
  2. `!response.ok` (status outside 200–299)
  3. a JSON parse failure
  4. the parsed value is not an object with `typeof count/max/remaining === "number"`
  Otherwise the parsed object is returned as is; extra fields are ignored.
* Rust: `Transport::freeapi_quota(&self) -> Option<FreeApiQuota>`.
  * **Web** (`HttpTransport`): `GET {base}app/api/chat/quota`, where V1's base is `/amadeus/`. Set `RequestCache::NoStore`. Any failure gives `None`.
  * **Desktop** (`TauriTransport`): `invoke("freeapi_quota")`. See the Open questions; FreeAPI is a hosted service.

### 8.2 Server contract (V1 `app/amadeus/app/api/chat/quota/route.ts`), to reproduce in `amadeus-server`

```ts
import { getGlobalRequestCount, MAX_GLOBAL_REQUESTS } from "../rate-limit";

export const dynamic = "force-dynamic";

export async function GET(): Promise<Response> {
  const count = getGlobalRequestCount();
  const max = MAX_GLOBAL_REQUESTS;
  const remaining = Math.max(0, max - count);
  return Response.json(
    { count, max, remaining },
    { headers: { "Cache-Control": "no-store" } },
  );
}

```

* `count` is the global request counter (`getGlobalRequestCount()`), which V1 persists in the `.amadeus_request_count` file.
* `max = MAX_GLOBAL_REQUESTS = 65000`.
* `remaining = max(0, max - count)`.
* The response is JSON with the header `Cache-Control: no-store`, e.g. `{"count":1234,"max":65000,"remaining":63766}`. The route is uncached (`force-dynamic`).

### 8.3 `useFreeApiQuota(active)` lifecycle

* State `quota: FreeApiQuota | null` starts as `null`.
* The effect depends on `[active]`:
  * When `active` is **true** at mount, or flips false→true, exactly **one** fetch is started.
  * When it resolves and the effect run was not cancelled, `quota = result`. A failure sets `quota = null`, which hides the widget.
  * When `active` flips true→false or the component unmounts, the cleanup sets `cancelled = true`, and a late result is ignored.
* **`quota` is NOT reset when `active` becomes false.** The stale value is kept. The display is gated by `provider === 8 && quota`. Switching back to FreeAPI therefore shows the old numbers immediately, until the new fetch resolves.
* There is no polling, no retry and no refresh after chats.
* The mobile settings sheet is mounted only while open, so every open re-fetches when the provider is 8. Switching the picker to FreeAPI also fetches.
* The desktop ConfigPanel uses `active = visible && provider === 8`.

### 8.4 `FreeApiQuotaDisplay({ quota, tr, className })`

* `exhausted = (quota.remaining === 0)`
* `labelColor = exhausted ? "#FF4444" : "#FF9900"`
* `numberColor = exhausted ? "#FF4444" : "#FFFFFF"`
* `usageText = tr("%1 / %2 回使用", "%1 / %2 requests used", "freeapi_quota_usage").replace("%1", fmt(count)).replace("%2", fmt(max))`. Each `replace` hits the **first occurrence only**; use `replacen(…, 1)` in Rust.
* `remainingText = tr("残り %1 回", "%1 remaining", "freeapi_quota_remaining").replace("%1", fmt(remaining))`
* `fmt(n) = n.toLocaleString()` with **no locale argument**. It uses the JS runtime default locale (browser/OS UI locale), **not** the app language. For example, de-DE gives `65.000`, fr-FR gives `65 000` (U+202F), and en/ja give `65,000`.
  * Rust/WASM: `js_sys::Intl::NumberFormat::new(&js_sys::Array::new(), &js_sys::Object::new())` and call its `format()` function with the number. Do **not** format in Rust.
* DOM (inline styles are part of the contract):
  ```html
  <div class="{className}">              <!-- "mobile-settings-freeapi-quota" (mobile) | "config-freeapi-quota" (desktop) -->
    <div>
      <span style="color:{labelColor}">{tr("FreeAPIクォータ","FreeAPI Quota","freeapi_quota_label")}</span>
      <span style="color:{numberColor}; margin-left:16px">{usageText}</span>
    </div>
    <div style="color:{numberColor}">{remainingText}</div>
    <!-- only when exhausted: -->
    <div style="color:#FF4444">{tr("全体リクエスト上限（65,000回）に達しました。サービスを停止しています。","Global request limit (65,000) reached. Service suspended.","freeapi_quota_exhausted")}</div>
  </div>
  ```
* `tr` is passed in from the parent panel, so the language is the panel's (unsaved) preview language. The four `freeapi_quota_*` keys exist **only in ja.json and en.json**. Every other language falls back to the inline **Japanese** default, because `tr` picks the en default only for language 1 (§15.3, §20).

---

## 9. `resolveResolution` (lines 413–421)

```ts
function resolveResolution(index: number): { width: number; height: number } {
  if (index === 1) {
    return { width: 1600, height: 900 };
  }
  if (index === 2) {
    return { width: 1280, height: 720 };
  }
  return { width: 1920, height: 1080 };
}
```

| `Config_Resolution` | width × height |
|---|---|
| 1 | 1600 × 900 |
| 2 | 1280 × 720 |
| 0 and **every other value** (negative, ≥3, NaN) | 1920 × 1080 |

On the web the result (`screenResolution`, line 4166) is **computed but never used**. The web UI always scales a 1920×1080 virtual canvas to the window. The setting is still offered in the desktop ConfigPanel (`["1920x1080","1600x900","1280x720"]`). In V2 desktop (Tauri), `resolve_resolution` can drive the window inner size, like the Qt app did. That needs a product decision; see the Open questions.

---

## 10. Settings hooks (lines 423–443)

```ts
function subscribeSettings(onStoreChange: () => void): () => void {
  return AppSettings.settingsChanged.connect(() => {
    onStoreChange();
  });
}

function useAppSettingInt(key: string, defaultValue: number): number {
  return useSyncExternalStore(
    subscribeSettings,
    () => AppSettings.getInt(key, defaultValue),
    () => defaultValue,
  );
}

function useAppSettingFloat(key: string, defaultValue: number): number {
  return useSyncExternalStore(
    subscribeSettings,
    () => AppSettings.getFloat(key, defaultValue),
    () => defaultValue,
  );
}
```

* `subscribeSettings(cb)` connects to `AppSettings.settingsChanged` and returns the disconnect function.
* `useAppSettingInt(key, def)` and `useAppSettingFloat(key, def)` use `useSyncExternalStore`:
  * The snapshot is re-read from localStorage after **any** settings write.
  * The component re-renders only if **this** key's parsed value changed (`Object.is`; values are primitives).
  * The server snapshot is `def`, used during SSR/hydration only.
* Uses in this file:

  | Key | Type | Default | Lines |
  |---|---|---|---|
  | `Config_EyeTracking` | int | 0 | 844 |
  | `Config_ChatLanguage` | int | 0 | 1270 |
  | `Config_AutoMode` | int | 0 | 3907 |
  | `Config_AutoSpeed` | float | 3.0 | 3908 |
  | `Config_Language` | int | 0 | 3909 |
  | `Config_ScreenMode` | int | 1 | 3910 |
  | `Config_Resolution` | int | 0 | 3911 |

**Leptos mapping.** Keep one global reactive "settings version" `RwSignal<u64>`, bumped after every `set_*`. Then:

```rust
pub fn use_setting_int(key: &'static str, default: i32) -> Memo<i32> {
    let version = expect_context::<SettingsVersion>().0;      // RwSignal<u64>
    Memo::new(move |_| { version.track(); settings_get_int(key, default) })
}
```

* `Memo` de-duplicates equal values, which matches the `Object.is` bail-out.
* Read `window().local_storage()` inside the closure rather than capturing a `web_sys::Storage`. Leptos 0.8 reactive closures must be `Send + Sync`, and `Storage` is not.
* Cross-tab sync is not required (V1 has none).

---

## 11. Mobile viewport detection (lines 445–475) — the rules

```ts
function subscribeViewportState(onStoreChange: () => void): () => void {
  if (typeof window === "undefined") {
    return () => {};
  }
  const visualViewport = window.visualViewport;
  const coarseQuery = window.matchMedia("(pointer: coarse)");
  const onMediaChange = () => onStoreChange();

  window.addEventListener("resize", onStoreChange);
  visualViewport?.addEventListener("resize", onStoreChange);
  coarseQuery.addEventListener("change", onMediaChange);

  return () => {
    window.removeEventListener("resize", onStoreChange);
    visualViewport?.removeEventListener("resize", onStoreChange);
    coarseQuery.removeEventListener("change", onMediaChange);
  };
}

function isMobileViewportSnapshot(): boolean {
  if (typeof window === "undefined") {
    return false;
  }
  const visualWidth = window.visualViewport?.width ?? window.innerWidth;
  const coarsePointer = window.matchMedia("(pointer: coarse)").matches;
  return visualWidth <= 900 || (coarsePointer && visualWidth <= 1200);
}

function useIsMobileViewport(): boolean {
  return useSyncExternalStore(subscribeViewportState, isMobileViewportSnapshot, () => false);
}
```

### 11.1 Exact rule

```
visualWidth = window.visualViewport?.width ?? window.innerWidth     // CSS px; may be fractional
coarse      = window.matchMedia("(pointer: coarse)").matches        // PRIMARY pointer is coarse
isMobile    = visualWidth <= 900 || (coarse && visualWidth <= 1200)
```

* The thresholds **900** and **1200** are inclusive. `900.5` is not ≤ 900.
* `??` only falls back when `visualViewport` is missing (null/undefined). A `visualViewport.width` of `0` (e.g. a hidden iframe) counts as mobile.
* **Re-evaluated on:** `window` `resize`, `window.visualViewport` `resize` (if it exists), and the `(pointer: coarse)` MediaQueryList `change`. Nothing else: no `orientationchange` (resize covers it) and no visualViewport `scroll`.
* SSR snapshot = `false`. Next.js therefore server-renders the **desktop** markup and hydration flips to mobile, causing a flash. V2 is CSR: compute the snapshot synchronously before the first render, so there is no flash.
* React re-renders only when the boolean changes.

Typical outcomes:

| Device / window | visualWidth | coarse | Layout |
|---|---|---|---|
| Phone portrait | ~360–430 | yes | mobile |
| Phone landscape | ~640–932 | yes | mobile (≤900 or ≤1200 with coarse) |
| iPad portrait | 744–1024 | yes | mobile |
| iPad landscape (non-Pro 12.9) | 1133–1194 | yes | mobile |
| iPad Pro 12.9" landscape | 1366 | yes | **desktop** (CSS still shows the MENU toggle; §11.3) |
| Desktop browser window ≤ 900 px wide | ≤ 900 | no | mobile |
| Desktop browser > 900 px | > 900 | no | desktop |
| Desktop with trackpad pinch-zoom in | visual width shrinks | no | can flip to **mobile** while layout width is unchanged (quirk) |

### 11.2 What the flag controls (all consumers)

1. **Root layout (line 4367).** It chooses the whole tree, `mobile-root` or the desktop `window-frame`.
   * Switching at runtime **unmounts and remounts** every child, so ChatPanel's state is lost: conversation history, turn count, typewriter and mini game. Login state survives, because `appState` lives in the root, and so do `backLogEntries`, `activeOverlay` and `mobileSettingsOpen`.
   * If the mobile settings sheet was open when switching to desktop, it disappears. It reappears when switching back, because the flag is still true.
   * Switching during `boot` restarts the boot screen of the new layout.
2. **ChatPanel (line 840 → 1886–1903).** While `chatState === "waitingAPI"`:
   * mobile shows `<button class="cancel-streaming-button">{t("cancel_button","キャンセル")}</button>` (it cancels the stream)
   * desktop shows `<div class="pc-cancel-hint">{t("cancel_hint","CTRL+Cでキャンセル")}</div>`
3. **MiniGamePanel (line 2539 → 2731–2737).** The inline style is `position: fixed|absolute`, `top: auto|0`, `height: 46%|100%` (mobile|desktop).

### 11.3 The CSS counterpart (CSS is reused verbatim; know the mismatch)

`globals.css` uses `@media (pointer: coarse), (max-width: 900px)` (lines 677 and 2586) for:

* `.mobile-menu-toggle`, the desktop layout's floating MENU button. It is `display:none` outside the query.
* `.input-panel` padding.
* All the `.mobile-root .achievement-*`, `.mobile-root .mini-game-*`, `.mobile-root .mobile-achievements-trigger-button` and `.mobile-root .live2d-shift.live2d-shifted` rules.

This is **not** the same predicate as the JS rule:

* A coarse primary pointer at a width over 1200 gives the desktop layout, but the CSS mobile rules still apply. In particular the MENU toggle is visible, which is **intended**: touch users need a menu button because they have neither Tab nor right-click.
* CSS `max-width` measures the **layout** viewport, while JS uses the **visual** viewport. Under pinch zoom, JS can say mobile while CSS does not. In that case the mobile floating buttons stay `display:none` and the mini-game mobile styles are missing.

Keep both as-is. Do not "fix" either side without a product decision.

### 11.4 Desktop (Tauri) note

The webview runs the same rule, so a desktop window narrower than about 900 CSS px switches to the mobile layout. Options are to keep this, to force the desktop layout (through a `desktop` cargo feature of `amadeus-ui`, or by detecting Tauri at runtime via `window.__TAURI_INTERNALS__`), or to make the window non-resizable with the size from `Config_Resolution`. See the Open questions.

### 11.5 Rust

Put the pure predicate in core for unit tests:

```rust
// amadeus_core::viewport
pub const MOBILE_MAX_WIDTH: f64 = 900.0;
pub const MOBILE_MAX_WIDTH_COARSE: f64 = 1200.0;
pub fn is_mobile_viewport(visual_width: f64, coarse_pointer: bool) -> bool {
    visual_width <= MOBILE_MAX_WIDTH || (coarse_pointer && visual_width <= MOBILE_MAX_WIDTH_COARSE)
}
```

In the UI, read the browser values and keep a signal:

```rust
// amadeus_ui::services::viewport
pub fn is_mobile_viewport_snapshot() -> bool {
    let Some(win) = web_sys::window() else { return false };
    let width = win.visual_viewport().map(|vv| vv.width())
        .unwrap_or_else(|| win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0));
    let coarse = win.match_media("(pointer: coarse)").ok().flatten().map(|m| m.matches()).unwrap_or(false);
    amadeus_core::viewport::is_mobile_viewport(width, coarse)
}
```

* `provide_mobile_viewport()` is called once at the App root. It creates `RwSignal<bool>` from the snapshot and attaches three listeners: `window_event_listener(ev::resize, …)`, a `visualViewport` `resize` listener (Closure kept alive for the app lifetime) and a MediaQueryList `change` listener. Each listener recomputes and calls `set` **only if the value changed**.
* `use_is_mobile_viewport() -> Signal<bool>` reads it from context.
* **web-sys feature `"VisualViewport"` is required and is currently missing** from `crates/amadeus-ui/Cargo.toml` (`MediaQueryList` and `MediaQueryListEvent` are already present).

---

## 12. `isTextInputTarget` (lines 477–483)

```ts
function isTextInputTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || target.isContentEditable;
}
```

* It returns true when the event target is an `HTMLElement` whose `tagName` is `INPUT`, `TEXTAREA` or `SELECT`, or which is `contentEditable`.
* Non-HTMLElement targets (e.g. `SVGElement`, `window`, `document`) → false.
* Every `<input>` type counts, including checkbox and button.
* Uses:
  * Root window keydown (line 4283). It is checked **after** F11, F3 and Tab are handled, so those three shortcuts work even while typing. Only menu navigation is suppressed.
  * Desktop ConfigPanel keydown (line 3199), which suppresses Backspace=apply+close and ↑/W/↓/S category switching while typing.
* Rust:

```rust
pub fn is_text_input_target(target: Option<web_sys::EventTarget>) -> bool {
    use wasm_bindgen::JsCast;
    let Some(el) = target.and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) else { return false };
    matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT") || el.is_content_editable()
}
```

---

## 13. `MobileLoginPanel` (lines 485–538)

```tsx
type MobileLoginPanelProps = {
  configLanguage: number;
  onAccepted: () => void;
};

function MobileLoginPanel({ configLanguage, onAccepted }: MobileLoginPanelProps) {
  const [userId, setUserId] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");

  const submit = () => {
    const validNames = ["Salieri"];
    const validPasswords = ["MakiseKurisu"];
    if (!userId || !password) {
      setError(t("login_empty", configLanguage === 1 ? "Please enter both fields." : "ユーザーIDとパスワードを入力してください。", configLanguage));
      return;
    }
    if (!validNames.includes(userId) && !validPasswords.includes(password)) {
      setError(t("login_access_denied", configLanguage === 1 ? "ACCESS DENIED" : "アクセスが拒否されました", configLanguage));
      return;
    }
    setError("");
    MemoryManager.setUserName("Salieri");
    onAccepted();
  };

  return (
    <div className="mobile-login-panel">
      <img src="/resources/images/amadeus_logo_v3.png" alt="" className="mobile-login-logo" />
      <label className="mobile-form-row">
        <span>USER ID</span>
        <input value={userId} onChange={(event) => setUserId(event.target.value)} />
      </label>
      <label className="mobile-form-row">
        <span>PASSWORD</span>
        <input
          type="password"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              submit();
            }
          }}
        />
      </label>
      {error && <div className="mobile-form-error">{error}</div>}
      <button type="button" className="mobile-primary-button" onClick={submit}>
        LOGIN
      </button>
    </div>
  );
}
```

### 13.1 Props / state

* Props:
  * `configLanguage: i32`, the applied `Config_Language` from the root hook
  * `onAccepted: () -> ()`. The root sets `appState = "boot"`.
* State: `userId = ""`, `password = ""`, `error = ""`. All are local and reset on remount.

### 13.2 `submit()` — exact algorithm

```
validNames = ["Salieri"]; validPasswords = ["MakiseKurisu"]
if userId == "" || password == "":                          # raw values, NOT trimmed
    error = t("login_empty", lang==1 ? "Please enter both fields." : "ユーザーIDとパスワードを入力してください。", lang); return
if userId ∉ validNames && password ∉ validPasswords:        # exact, case-sensitive
    error = t("login_access_denied", lang==1 ? "ACCESS DENIED" : "アクセスが拒否されました", lang); return
error = ""
MemoryManager.setUserName("Salieri")                         # always "Salieri", whatever was typed
onAccepted()
```

Truth table (both fields non-empty):

| userId | password | result |
|---|---|---|
| `Salieri` | anything | accepted |
| anything | `MakiseKurisu` | accepted |
| `Salieri ` (trailing space) | wrong | **denied** (mobile does not trim) |
| `salieri` | wrong | denied (case-sensitive) |
| `   ` (spaces) | `MakiseKurisu` | accepted (whitespace counts as non-empty) |

* Effective strings: `login_empty` is in **no** locale. Language 1 shows `"Please enter both fields."` and **every other language shows the Japanese text**.
* `login_access_denied` exists in all locales. Its value is `"ACCESS DENIED"` for languages 0–10, `"גישה נדחתה"` for he and `"تم رفض الوصول"` for ar.
* The error string is computed once at submit time and stored. It does not update on a language change and is not cleared while typing; only the next submit changes it.
* This is a fake in-universe login (Salieri / MakiseKurisu); it has no security role. Keep the credentials as constants in core.

### 13.3 DOM (CSS classes reused verbatim)

```html
<div class="mobile-login-panel">
  <img src="/resources/images/amadeus_logo_v3.png" alt="" class="mobile-login-logo">
  <label class="mobile-form-row">
    <span>USER ID</span>
    <input value="{userId}">                         <!-- type=text (default); no autofocus, no autocomplete attrs -->
  </label>
  <label class="mobile-form-row">
    <span>PASSWORD</span>
    <input type="password" value="{password}">       <!-- Enter → preventDefault + submit -->
  </label>
  <div class="mobile-form-error">{error}</div>      <!-- rendered ONLY when error != "" -->
  <button type="button" class="mobile-primary-button">LOGIN</button>
</div>
```

* `USER ID`, `PASSWORD` and `LOGIN` are literal and never translated.
* Wrapping in `<label>` makes a tap on the caption focus the input. Keep `<label>`.
* Inputs fire React `onChange` per keystroke; use Leptos `on:input`.
* The **Enter key works only on the PASSWORD field**. Nothing happens on the USER ID field, and there is no `<form>`.
* There is no autofocus. The desktop LoginPanel autofocuses USER ID.
* The image path is root-absolute in V1. In V2 make it base-relative: `resources/images/amadeus_logo_v3.png`.

### 13.4 Differences vs the desktop `LoginPanel` (lines 2001–2057; another part ports it)

| | Mobile | Desktop |
|---|---|---|
| Trimming | none | `trim()` on both fields |
| Logic | deny only if **both** id and password are wrong (non-empty) | accept if both trimmed non-empty **and** (id ok **or** password ok); same OR logic |
| Empty-field message | localized `login_empty` | generic `"ACCESS DENIED"` (literal) |
| Error display | `.mobile-form-error` with a localized string | `.login-error` with the literal `ACCESS DENIED` |
| Enter key | password field only | both fields |
| Autofocus | no | USER ID |
| Button | text `LOGIN`, `.mobile-primary-button` | image `amadeus_login_button_v2.png` inside `.login-button` |

### 13.5 Leptos sketch

```rust
#[component]
pub fn MobileLoginPanel(#[prop(into)] config_language: Signal<i32>, on_accepted: Callback<()>) -> impl IntoView {
    let (user_id, set_user_id) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (error, set_error) = signal(String::new());
    let submit = move || {
        let lang = config_language.get_untracked();
        match amadeus_core::login::validate_mobile_login(&user_id.get_untracked(), &password.get_untracked()) {
            MobileLogin::Empty  => set_error.set(i18n::t("login_empty", if lang == 1 { "Please enter both fields." } else { "ユーザーIDとパスワードを入力してください。" }, lang)),
            MobileLogin::Denied => set_error.set(i18n::t("login_access_denied", if lang == 1 { "ACCESS DENIED" } else { "アクセスが拒否されました" }, lang)),
            MobileLogin::Accepted => { set_error.set(String::new()); memory_manager().set_user_name("Salieri"); on_accepted.run(()); }
        }
    };
    /* view! { … exact DOM of §13.3 … ; password input: on:keydown=move |ev| if ev.key() == "Enter" { ev.prevent_default(); submit() } } */
}
```

---

## 14. `MobileLoadingPanel` (lines 540–574)

```tsx
type MobileLoadingPanelProps = {
  configLanguage: number;
  onFinished: () => void;
};

function MobileLoadingPanel({ configLanguage, onFinished }: MobileLoadingPanelProps) {
  const [dots, setDots] = useState(1);

  useEffect(() => {
    if (AppSettings.getInt("Config_SkipLoading", 0) === 1) {
      onFinished();
      return;
    }
    const finishTimer = window.setTimeout(() => {
      onFinished();
    }, 1500);
    const dotTimer = window.setInterval(() => {
      setDots((value) => (value % 3) + 1);
    }, 300);
    return () => {
      window.clearTimeout(finishTimer);
      window.clearInterval(dotTimer);
    };
  }, [onFinished]);

  return (
    <div className="mobile-loading-panel">
      <img src="/resources/images/amadeus_logo_v3.png" alt="Amadeus System" className="mobile-loading-logo" />
      <div className="mobile-loading-line">
        {t("booting", configLanguage === 1 ? "Booting" : "起動中", configLanguage)}
        {".".repeat(dots)}
      </div>
    </div>
  );
}
```

### 14.1 Behavior

* Props:
  * `configLanguage`
  * `onFinished`. The root sets `appState = "chat"`.
* State: `dots = 1`.
* The effect runs after mount and whenever `onFinished` identity changes:
  * If `AppSettings.getInt("Config_SkipLoading", 0) === 1`, it calls `onFinished()` **synchronously** with no timers. One frame of the panel may be visible. The desktop version uses `setTimeout(…, 0)` instead.
  * Otherwise it sets `finishTimer = setTimeout(onFinished, 1500)` and `dotTimer = setInterval(() => dots = (dots % 3) + 1, 300)`. The cleanup clears both.
* Text sequence: `t=0` → `起動中.`, `300ms` → `..`, `600` → `...`, `900` → `.`, `1200` → `..`. At 1500 ms the finish timeout fires, normally just before the 5th interval tick. Whether the final `...` gets painted before the switch to chat is nondeterministic and does not matter.
* `booting` is in **no** locale. Language 1 shows `Booting` and every other language shows `起動中`.
* **Quirk:** the root passes a new inline closure on every root render, so the effect restarts, and the 1500 ms timer **resets**, whenever the root re-renders during boot. Any `window` resize does this, because it calls `setDimensions` with a new object, and so does any settings change. On phones this happens when the URL bar or keyboard changes the viewport after login.
  * V2 may simply start the timer once on mount. Dropping the reset is an acceptable, invisible deviation. The dots state is **not** reset when the effect restarts.
* The mobile flow does **not** use `BOOT_LOG`, the boot logo sequence or any sound.

### 14.2 DOM

```html
<div class="mobile-loading-panel">
  <img src="/resources/images/amadeus_logo_v3.png" alt="Amadeus System" class="mobile-loading-logo">
  <div class="mobile-loading-line">{t("booting", lang==1 ? "Booting" : "起動中", lang)}{".".repeat(dots)}</div>
</div>
```

* `.mobile-loading-logo` has **no CSS rule** in globals.css. The image takes its natural or flex-stretched size inside the column flexbox of `.mobile-loading-panel`. Keep the class; do not add styles unless the product asks.
* `.mobile-loading-title` exists in CSS but is unused.

### 14.3 Leptos

* Do the skip-loading check and the timer start in `Effect::new` (or `request_animation_frame`), not in the component body. Calling `on_finished` synchronously during construction would update root signals mid-render.
* Use `set_timeout_with_handle(Duration::from_millis(1500), …)` and `set_interval_with_handle(Duration::from_millis(300), …)`, and clear both handles in `on_cleanup`. The handles are `Copy + Send`.
* gloo `Timeout`/`Interval` are `!Send` and cannot be moved into Leptos 0.8 `on_cleanup` without a `SendWrapper`.

---

## 15. `MobileSettingsPanel` (lines 576–814)

```tsx
type MobileSettingsPanelProps = {
  onClose: () => void;
  onLanguageChange: (value: number) => void;
  onSettingsSaved?: () => void;
};

function readMobileSettingsState() {
  const provider = AppSettings.getInt("Config_ApiProvider", 8);
  const apiCredential = AppSettings.getString(`Config_ApiKey_${provider}`, "") || AppSettings.getString("Config_ApiKey", "");
  const modelName =
    AppSettings.getString(`Config_ModelName_${provider}`, "") || AppSettings.getString("Config_ModelName", "");

  return {
    language: AppSettings.getInt("Config_Language", 0),
    provider,
    apiCredential,
    modelName,
    webSearch: AppSettings.getInt("Config_WebSearch", 0) === 1,
    autoMode: AppSettings.getInt("Config_AutoMode", 0) === 1,
    vertexProject: AppSettings.getString("Config_VertexProject", ""),
    vertexLocation: AppSettings.getString("Config_VertexLocation", "us-central1"),
    ollamaHost: AppSettings.getString("Config_OllamaHost", "http://localhost:11434"),
    openaiCompatible: AppSettings.getInt("Config_OpenAICompatible", 0) === 1,
    openaiBaseUrl: AppSettings.getString("Config_OpenAIBaseUrl", ""),
  };
}

function MobileSettingsPanel({ onClose, onLanguageChange, onSettingsSaved }: MobileSettingsPanelProps) {
  const [state, setState] = useState(readMobileSettingsState);
  const freeApiQuota = useFreeApiQuota(state.provider === FREEAPI_PROVIDER_INDEX);

  const tr = useCallback(
    (ja: string, en: string, key?: string) => {
      if (key) {
        return t(key, state.language === 1 ? en : ja, state.language);
      }
      return state.language === 0 ? ja : en;
    },
    [state.language],
  );

  const handleProviderChange = (provider: number) => {
    const apiCredential =
      AppSettings.getString(`Config_ApiKey_${provider}`, "") || (provider === state.provider ? state.apiCredential : "");
    const modelName =
      AppSettings.getString(`Config_ModelName_${provider}`, "") || (provider === state.provider ? state.modelName : "");
    setState((prev) => ({
      ...prev,
      provider,
      apiCredential,
      modelName,
    }));
  };

  const apply = () => {
    AppSettings.setInt("Config_Language", state.language);
    AppSettings.setInt("Config_ApiProvider", state.provider);
    AppSettings.setInt("Config_WebSearch", state.webSearch ? 1 : 0);
    AppSettings.setInt("Config_AutoMode", state.autoMode ? 1 : 0);
    AppSettings.setString(`Config_ApiKey_${state.provider}`, state.apiCredential);
    AppSettings.setString(`Config_ModelName_${state.provider}`, state.modelName);
    AppSettings.setString("Config_ApiKey", state.apiCredential);
    AppSettings.setString("Config_ModelName", state.modelName);
    AppSettings.setString("Config_VertexProject", state.vertexProject);
    AppSettings.setString("Config_VertexLocation", state.vertexLocation || "us-central1");
    AppSettings.setString("Config_OllamaHost", state.ollamaHost.trim() || "http://localhost:11434");
    AppSettings.setInt("Config_OpenAICompatible", state.openaiCompatible ? 1 : 0);
    AppSettings.setString("Config_OpenAIBaseUrl", state.openaiBaseUrl);
    AppSettings.save();
    onLanguageChange(state.language);
    onSettingsSaved?.();
    onClose();
  };

  return (
    <div className="mobile-settings-overlay">
      <div className="mobile-settings-sheet">
        <div className="mobile-settings-title">{tr("設定", "SETTINGS", "setting_title")}</div>

        <label className="mobile-settings-row">
          <span>{tr("表示言語", "Display Language", "setting_display_language")}</span>
          <select
            value={state.language}
            onChange={(event) => setState((prev) => ({ ...prev, language: Number(event.target.value) }))}
          >
            {LANGUAGE_NAMES.map((langName, index) => (
              <option key={langName} value={index}>
                {langName}
              </option>
            ))}
          </select>
        </label>

        <label className="mobile-settings-row">
          <span>{tr("LLM APIプロバイダ", "LLM API Provider", "setting_api_provider")}</span>
          <select value={state.provider} onChange={(event) => handleProviderChange(Number(event.target.value))}>
            {PROVIDER_NAMES.map((providerName, index) => {
              if (!providerName) return null;
              if (index === 4 && AppSettings.getInt("Config_VertexEnabled", 0) !== 1) return null;
              return (
                <option key={index} value={index}>
                  {providerName}
                </option>
              );
            })}
          </select>
        </label>

        {state.provider === FREEAPI_PROVIDER_INDEX && freeApiQuota && (
          <FreeApiQuotaDisplay quota={freeApiQuota} tr={tr} className="mobile-settings-freeapi-quota" />
        )}

        {state.provider !== 5 && state.provider !== 8 && (
          <label className="mobile-settings-row">
            <span>{state.provider === 4 ? tr("Vertex APIキー / SA JSON", "Vertex API Key / SA JSON") : tr("APIキー", "API Key", "setting_api_key")}</span>
            {state.provider === 4 ? (
              <textarea
                className="mobile-settings-textarea"
                value={state.apiCredential}
                onChange={(event) => setState((prev) => ({ ...prev, apiCredential: event.target.value }))}
              />
            ) : (
              <input
                type="password"
                value={state.apiCredential}
                onChange={(event) => setState((prev) => ({ ...prev, apiCredential: event.target.value }))}
              />
            )}
          </label>
        )}

        {state.provider !== 8 && (
          <label className="mobile-settings-row">
            <span>{tr("LLM モデル名", "LLM Model Name", "setting_model_name")}</span>
            <input
              value={state.modelName}
              onChange={(event) => setState((prev) => ({ ...prev, modelName: event.target.value }))}
            />
          </label>
        )}

        {state.provider !== 5 && state.provider !== 8 && (
          <label className="mobile-settings-row checkbox">
            <span>{tr("LLM Web検索", "LLM Web Search", "setting_web_search")}</span>
            <input
              type="checkbox"
              checked={state.webSearch}
              onChange={(event) => setState((prev) => ({ ...prev, webSearch: event.target.checked }))}
            />
          </label>
        )}

        <label className="mobile-settings-row checkbox">
          <span>{tr("オート表示", "Auto Mode", "setting_auto_mode")}</span>
          <input
            type="checkbox"
            checked={state.autoMode}
            onChange={(event) => setState((prev) => ({ ...prev, autoMode: event.target.checked }))}
          />
        </label>

        {state.provider === 0 && (
          <>
            <label className="mobile-settings-row checkbox">
              <span>{tr("互換APIを使う", "OpenAI Compatible", "setting_openai_compatible")}</span>
              <input
                type="checkbox"
                checked={state.openaiCompatible}
                onChange={(event) => setState((prev) => ({ ...prev, openaiCompatible: event.target.checked }))}
              />
            </label>
            {state.openaiCompatible && (
              <label className="mobile-settings-row">
                <span>{tr("ベースURL", "Base URL", "setting_openai_base_url")}</span>
                <input
                  value={state.openaiBaseUrl}
                  onChange={(event) => setState((prev) => ({ ...prev, openaiBaseUrl: event.target.value }))}
                />
              </label>
            )}
          </>
        )}

        {state.provider === 4 && (
          <>
            <label className="mobile-settings-row">
              <span>Vertex Project ID</span>
              <input
                value={state.vertexProject}
                onChange={(event) => setState((prev) => ({ ...prev, vertexProject: event.target.value }))}
              />
            </label>
            <label className="mobile-settings-row">
              <span>Vertex Location</span>
              <input
                value={state.vertexLocation}
                onChange={(event) => setState((prev) => ({ ...prev, vertexLocation: event.target.value }))}
              />
            </label>
            <div className="config-note">
              {tr(
                "※ Vertexを使用するためには、gCloud CLIのインストールが必要です。",
                "* gCloud CLI must be installed to use Vertex.",
                "setting_vertex_note",
              )}
            </div>
          </>
        )}

        {state.provider === 5 && (
          <>
            <label className="mobile-settings-row">
              <span>Ollama Host</span>
              <input
                value={state.ollamaHost}
                onChange={(event) => setState((prev) => ({ ...prev, ollamaHost: event.target.value }))}
              />
            </label>
            <div className="config-note">
              {tr(
                "※ Ollamaはローカルで動作するLLMランナーです。デフォルトのホストは http://localhost:11434 です。",
                "* Ollama is a local LLM runner. The default host is http://localhost:11434.",
              )}
            </div>
          </>
        )}

        <div className="mobile-settings-actions">
          <button type="button" className="secondary" onClick={onClose}>
            {tr("閉じる", "Close", "setting_close")}
          </button>
          <button type="button" className="primary" onClick={apply}>
            {tr("適用", "Apply", "setting_apply")}
          </button>
        </div>
      </div>
    </div>
  );
}
```

### 15.1 Props

* `onClose()`: the root sets `mobileSettingsOpen = false`, which unmounts the sheet and discards all local state.
* `onLanguageChange(value)`: the root calls `AppSettings.setInt("Config_Language", value)` again. This is redundant with `apply` but emits once more.
* `onSettingsSaved?()`: the root calls `setLlmStatus(getLlmStatusFromSettings())`, refreshing the top-bar marquee `"{provider} / {model}"`.

The panel is **mounted only while open** (`{mobileSettingsOpen && <MobileSettingsPanel …/>}`). It is opened by the top-bar SETTINGS button.

### 15.2 State and initial values (`readMobileSettingsState`, run once at mount via the lazy `useState` initializer)

Every field, in DOM order:

| # | Field | Type | Initial value (exact) | Control | Label (ja / en / key) | Visible when | On change | Written on Apply |
|---|---|---|---|---|---|---|---|---|
| 1 | `language` | int | `getInt("Config_Language", 0)` | `<select>` with 13 `<option value=i>{LANGUAGE_NAMES[i]}</option>` | 表示言語 / Display Language / `setting_display_language` | always | `Number(value)` (also live-switches every label of the sheet, §15.3) | `setInt("Config_Language", language)`, then the parent sets it again |
| 2 | `provider` | int | `getInt("Config_ApiProvider", 8)` | `<select>` with filtered `PROVIDER_NAMES` (§7.1) | LLM APIプロバイダ / LLM API Provider / `setting_api_provider` | always | `handleProviderChange(Number(value))` (§15.4) | `setInt("Config_ApiProvider", provider)` |
| – | FreeAPI quota widget | – | fetched (§8) | `FreeApiQuotaDisplay className="mobile-settings-freeapi-quota"` | §8.4 | `provider == 8 && quota != null` | – | – |
| 3 | `apiCredential` | string | `getString("Config_ApiKey_{provider}", "")`, **or if that is empty** `getString("Config_ApiKey", "")` | provider 4: `<textarea class="mobile-settings-textarea">`; otherwise `<input type="password">` | provider 4: "Vertex APIキー / SA JSON" / "Vertex API Key / SA JSON" (**no key**); otherwise APIキー / API Key / `setting_api_key` | `provider != 5 && provider != 8` | raw value | `setString("Config_ApiKey_{provider}", v)` **and** `setString("Config_ApiKey", v)` (no trim) |
| 4 | `modelName` | string | `getString("Config_ModelName_{provider}", "")` or else `getString("Config_ModelName", "")` | `<input>` (text) | LLM モデル名 / LLM Model Name / `setting_model_name` | `provider != 8` | raw | `setString("Config_ModelName_{provider}", v)` **and** `setString("Config_ModelName", v)` (no trim) |
| 5 | `webSearch` | bool | `getInt("Config_WebSearch", 0) === 1` | `<input type="checkbox">` in a `checkbox` row | LLM Web検索 / LLM Web Search / `setting_web_search` | `provider != 5 && provider != 8` | `checked` | `setInt("Config_WebSearch", b?1:0)` |
| 6 | `autoMode` | bool | `getInt("Config_AutoMode", 0) === 1` | checkbox row | オート表示 / Auto Mode / `setting_auto_mode` | always | `checked` | `setInt("Config_AutoMode", b?1:0)` |
| 7 | `openaiCompatible` | bool | `getInt("Config_OpenAICompatible", 0) === 1` | checkbox row | 互換APIを使う / OpenAI Compatible / `setting_openai_compatible` (en.json: "Use Compatible API") | `provider == 0` | `checked` | `setInt("Config_OpenAICompatible", b?1:0)` |
| 8 | `openaiBaseUrl` | string | `getString("Config_OpenAIBaseUrl", "")` | `<input>` | ベースURL / Base URL / `setting_openai_base_url` | `provider == 0 && openaiCompatible` | raw | `setString("Config_OpenAIBaseUrl", v)` (no trim) |
| 9 | `vertexProject` | string | `getString("Config_VertexProject", "")` | `<input>` | literal `Vertex Project ID` | `provider == 4` | raw | `setString("Config_VertexProject", v)` (no trim) |
| 10 | `vertexLocation` | string | `getString("Config_VertexLocation", "us-central1")` | `<input>` | literal `Vertex Location` | `provider == 4` | raw | `setString("Config_VertexLocation", v \|\| "us-central1")`: an empty string falls back, but **no trim** (`" "` is stored as is) |
| – | Vertex note | – | – | `<div class="config-note">` | `setting_vertex_note` (§20) | `provider == 4` | – | – |
| 11 | `ollamaHost` | string | `getString("Config_OllamaHost", "http://localhost:11434")` | `<input>` | literal `Ollama Host` | `provider == 5` | raw | `setString("Config_OllamaHost", v.trim() \|\| "http://localhost:11434")` (trimmed, with fallback) |
| – | Ollama note | – | – | `<div class="config-note">` | no key (§20) | `provider == 5` | – | – |

* Hidden fields keep their state and are **still written** on Apply. Examples: `webSearch` while provider is 5 or 8; `apiCredential`/`modelName` while provider is 8; `vertex*`/`ollamaHost`/`openai*` while those providers are not selected. Their values are the ones read at mount, so they normally round-trip unchanged.
* "Empty" in the "or else" fallbacks means the JS `||` falsy test on strings, i.e. `""`.

### 15.3 `tr(ja, en, key?)`, the language-preview translator (memoized on `state.language`)

```
tr(ja, en, key):
    if key is given: return t(key, state.language == 1 ? en : ja, state.language)
    else:            return state.language == 0 ? ja : en
```

* **Asymmetry, which must be preserved.**
  * For **keyed** strings whose key is missing from the selected locale, languages 2–12 see the **Japanese** default.
  * For **unkeyed** strings, languages 1–12 see **English** and only language 0 sees Japanese.
  * So a zh user sees `設定`, `閉じる`, `適用` (keys missing, Japanese default) but `Vertex API Key / SA JSON` and the English Ollama note (no key, English).
* `state.language` changes as soon as the language `<select>` changes, before Apply. Every label in the sheet, including the FreeAPI widget, re-renders in the newly selected language. The top-bar buttons keep using the applied `configLanguage`.
* A language index outside 0–12 (stored by hand) uses the ja dictionary for keyed strings and English for unkeyed ones.
* Core API: `amadeus_core::i18n::tr(lang: i32, ja: &str, en: &str, key: Option<&str>) -> String`. The desktop ConfigPanel uses the identical function at lines 3219–3227.

### 15.4 `handleProviderChange(p)`

```
apiCredential = getString("Config_ApiKey_{p}", "") || (p == state.provider ? state.apiCredential : "")
modelName     = getString("Config_ModelName_{p}", "") || (p == state.provider ? state.modelName : "")
state = { ...state, provider: p, apiCredential, modelName }
```

* The new provider's **stored** per-provider value is loaded, or `""`. In practice `p == state.provider` never holds, because `change` does not fire when the same option is re-selected.
* There is **no global `Config_ApiKey` fallback** here. The initial read has one; this asymmetry is V1 behavior.
* **Unsaved edits are lost when switching providers.** Type a key for OpenAI, switch to Gemini, switch back, and the typed key is gone (replaced by the stored value). This is a V1 quirk; keep it. The desktop ConfigPanel buffers per-provider edits instead.
* Other fields (webSearch, vertex*, ollama*, openai*) are untouched.
* Switching to 8 activates the quota fetch (§8.3).

### 15.5 `apply()`: exact write order (every `set*` emits `settingsChanged`)

1. `setInt("Config_Language", state.language)`
2. `setInt("Config_ApiProvider", state.provider)`
3. `setInt("Config_WebSearch", state.webSearch ? 1 : 0)`
4. `setInt("Config_AutoMode", state.autoMode ? 1 : 0)`
5. `setString("Config_ApiKey_" + state.provider, state.apiCredential)`. Any index can be written here, **including 7 and 8**; the desktop ConfigPanel only ever writes 0–6.
6. `setString("Config_ModelName_" + state.provider, state.modelName)`
7. `setString("Config_ApiKey", state.apiCredential)`
8. `setString("Config_ModelName", state.modelName)`
9. `setString("Config_VertexProject", state.vertexProject)`
10. `setString("Config_VertexLocation", state.vertexLocation || "us-central1")`
11. `setString("Config_OllamaHost", state.ollamaHost.trim() || "http://localhost:11434")`. `trim` is JS trim (§4.2).
12. `setInt("Config_OpenAICompatible", state.openaiCompatible ? 1 : 0)`
13. `setString("Config_OpenAIBaseUrl", state.openaiBaseUrl)`
14. `AppSettings.save()` (no-op)
15. `onLanguageChange(state.language)`, which makes the root write `Config_Language` again
16. `onSettingsSaved?.()`, which refreshes `llmStatus` from settings
17. `onClose()`, which unmounts the sheet

Not written by the mobile sheet: `Config_ChatLanguage`, skip loading, right click, eye tracking, notifications, lightweight mode, text speed, auto speed, volumes, screen mode, resolution, TTS/STT/RAG, local proxy and gcloud settings. Those can only be changed in the desktop layout.

Downstream effects of Apply:

* When `Config_Language` changes, ChatPanel's `${Config_Language}:${Config_ChatLanguage}` watcher recomputes `history[0]` (§3.4). The prompt's language actually changes only when `Config_ChatLanguage == 0`.
* The root's `configLanguage` hook re-renders the top bar.
* `ai-service.ts buildPayload` reads `Config_ApiProvider`, `Config_ApiKey_{p}` (falling back to `Config_ApiKey`), `Config_ModelName_{p}` (falling back to `Config_ModelName`), `Config_WebSearch`, `Config_Vertex*`, `Config_OllamaHost` and `Config_OpenAI*` on the next request.
* `Config_AutoMode` changes the auto-advance behavior and the root `autoMode` hook.

Values are stored as JS strings: ints as decimal (`"8"`), booleans as `"0"`/`"1"`, strings raw. **API keys are stored in plaintext localStorage** (V1 format; must stay readable).

### 15.6 DOM: exact structure and conditional rendering (in order)

```html
<div class="mobile-settings-overlay">                     <!-- no click-outside-to-close, no Escape handler -->
  <div class="mobile-settings-sheet">
    <div class="mobile-settings-title">{tr("設定","SETTINGS","setting_title")}</div>

    <label class="mobile-settings-row">
      <span>{tr("表示言語","Display Language","setting_display_language")}</span>
      <select>                                              <!-- value = state.language -->
        <option value="0">日本語</option> … <option value="12">العربية</option>   <!-- LANGUAGE_NAMES -->
      </select>
    </label>

    <label class="mobile-settings-row">
      <span>{tr("LLM APIプロバイダ","LLM API Provider","setting_api_provider")}</span>
      <select>                                              <!-- value = state.provider -->
        <option value="0">OpenAI</option><option value="1">Google Gemini</option>
        <option value="2">Anthropic Claude</option><option value="3">Groq</option>
        <!-- <option value="4">Vertex AI</option> only if Config_VertexEnabled == 1 -->
        <option value="5">Ollama</option><option value="6">OpenRouter</option>
        <!-- index 7 ("") never rendered -->
        <option value="8">FreeAPI</option>
      </select>
    </label>

    <!-- if provider == 8 && quota: --> <div class="mobile-settings-freeapi-quota">…§8.4…</div>

    <!-- if provider != 5 && provider != 8: -->
    <label class="mobile-settings-row">
      <span>{provider==4 ? tr("Vertex APIキー / SA JSON","Vertex API Key / SA JSON") : tr("APIキー","API Key","setting_api_key")}</span>
      <!-- provider==4: --> <textarea class="mobile-settings-textarea">{apiCredential}</textarea>
      <!-- else:        --> <input type="password" value="{apiCredential}">
    </label>

    <!-- if provider != 8: -->
    <label class="mobile-settings-row"><span>{tr("LLM モデル名","LLM Model Name","setting_model_name")}</span><input value="{modelName}"></label>

    <!-- if provider != 5 && provider != 8: -->
    <label class="mobile-settings-row checkbox"><span>{tr("LLM Web検索","LLM Web Search","setting_web_search")}</span><input type="checkbox"></label>

    <label class="mobile-settings-row checkbox"><span>{tr("オート表示","Auto Mode","setting_auto_mode")}</span><input type="checkbox"></label>

    <!-- if provider == 0: -->
    <label class="mobile-settings-row checkbox"><span>{tr("互換APIを使う","OpenAI Compatible","setting_openai_compatible")}</span><input type="checkbox"></label>
    <!--   if openaiCompatible: -->
    <label class="mobile-settings-row"><span>{tr("ベースURL","Base URL","setting_openai_base_url")}</span><input value="{openaiBaseUrl}"></label>

    <!-- if provider == 4: -->
    <label class="mobile-settings-row"><span>Vertex Project ID</span><input value="{vertexProject}"></label>
    <label class="mobile-settings-row"><span>Vertex Location</span><input value="{vertexLocation}"></label>
    <div class="config-note">{tr("※ Vertexを使用するためには、gCloud CLIのインストールが必要です。","* gCloud CLI must be installed to use Vertex.","setting_vertex_note")}</div>

    <!-- if provider == 5: -->
    <label class="mobile-settings-row"><span>Ollama Host</span><input value="{ollamaHost}"></label>
    <div class="config-note">{tr("※ Ollamaはローカルで動作するLLMランナーです。デフォルトのホストは http://localhost:11434 です。","* Ollama is a local LLM runner. The default host is http://localhost:11434.")}</div>

    <div class="mobile-settings-actions">
      <button type="button" class="secondary">{tr("閉じる","Close","setting_close")}</button>   <!-- onClose: discard -->
      <button type="button" class="primary">{tr("適用","Apply","setting_apply")}</button>       <!-- apply() -->
    </div>
  </div>
</div>
```

Visibility matrix by provider (✓ = rendered):

| element \ provider | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7* | 8 |
|---|---|---|---|---|---|---|---|---|---|
| FreeAPI quota | | | | | | | | | ✓ (when loaded) |
| API key `<input type=password>` | ✓ | ✓ | ✓ | ✓ | | | ✓ | ✓ | |
| API key `<textarea>` (Vertex label) | | | | | ✓ | | | | |
| Model name | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | |
| Web search | ✓ | ✓ | ✓ | ✓ | ✓ | | ✓ | ✓ | |
| Auto mode | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| OpenAI compatible | ✓ | | | | | | | | |
| Base URL | ✓ if compatible | | | | | | | | |
| Vertex Project ID / Location / note | | | | | ✓ | | | | |
| Ollama Host / note | | | | | | ✓ | | | |

\*Provider 7, and provider 4 without `Config_VertexEnabled`, can only be reached through a stored value; they have no `<option>`.

### 15.7 Event and interaction details

* Text inputs and the textarea update state on every keystroke (React `onChange` is the DOM `input` event); use `on:input` in Leptos. Selects and checkboxes use `change` (`on:change`).
* **Implicit `<label>` activation:** tapping anywhere on a `checkbox` row toggles its checkbox. Tapping a text row's caption focuses its control. Keep the wrapping `<label>` elements.
* No focus management: no autofocus, no focus trap, no Escape, no backdrop click.
* The global root keydown handler (§16) is still active while the sheet is open:
  * **Tab is intercepted** (`preventDefault`) whenever `activeOverlay === "none"`, which is the case while the sheet is open. Hardware-keyboard users cannot Tab between fields; this is a V1 quirk.
  * F3 toggles `Config_AutoMode` in storage, but the sheet's `autoMode` state does not change, so Apply would overwrite it.
  * F11 toggles fullscreen.
* While the sheet is open, ChatPanel is paused: `menuPanelOpen = mobileSettingsOpen` pauses typewriter and auto-advance and blocks ChatPanel's Enter / Ctrl+C.
* **Controlled `<select>` with an unmatched value.** If the stored provider is 4 without the Vertex flag, or 7, or the stored language is ≥13, no `<option>` matches. React leaves the browser selecting the **first** option visually ("OpenAI" / "日本語"), while `state` keeps 4, 7 or 13. The Vertex or generic fields render according to the state, and Apply writes 4, 7 or 13 back.
  * In Leptos, set `prop:selected` per option (`move || state.with(|s| s.provider == i)`) **and** avoid coercing the state.
  * `prop:value` on a `<select>` set before its options exist does not stick, which is a common Leptos pitfall.

### 15.8 Differences vs the desktop `ConfigPanel` API tab (context; another part ports ConfigPanel)

| Aspect | MobileSettingsPanel | Desktop ConfigPanel |
|---|---|---|
| Lifetime | mounted only while open; reads storage at each open | always mounted; re-reads when `visible` becomes true |
| Per-provider edit buffer | none (switching loses edits) | `apiKeyBuffer` / `modelNameBuffer` for 0–6 |
| Per-provider keys written | only the current provider (0–8) | all of 0–6 on every apply; never 7 or 8 |
| Global `Config_ApiKey` / `Config_ModelName` | current provider's values | `buffer[provider] ?? ""` |
| `Config_VertexLocation` | `v \|\| "us-central1"` | written as is |
| Vertex credential control | `<textarea class="mobile-settings-textarea">` | `<input type="text">` |
| Provider picker | native `<select>` | custom dropdown `ConfigSelectRow` (same filter rule) |
| FreeAPI widget class | `mobile-settings-freeapi-quota` | `config-freeapi-quota` |
| Keyboard | none | Backspace = apply+close, ↑/W ↓/S = category (not while typing) |
| Other settings | none | chat language, system, text, sound, graphics, TTS/STT/RAG, export/import |
| After apply | `onSettingsSaved` refreshes the LLM status | updates the TTS/STT/VectorDB service singletons |

---

## 16. Integration context: the mobile root (lines 4367–4449; ported by the root/app part)

```tsx
  if (isMobileViewport) {
    return (
      <div className="real-amadeus-root mobile-root" onContextMenu={(event) => event.preventDefault()}>
        {appState === "chat" && (
          <div className="mobile-topbar">
            <div className="mobile-topbar-buttons">
              <button type="button" className="mobile-topbar-button" onClick={() => setMobileSettingsOpen(true)}>
                {t("settings", configLanguage === 1 ? "SETTINGS" : "設定", configLanguage)}
              </button>
              <button type="button" className="mobile-topbar-button" onClick={() => setActiveOverlay("backlog")}>
                {t("backlog", configLanguage === 1 ? "LOG" : "ログ", configLanguage)}
              </button>
            </div>
            <MarqueeStatus provider={llmStatus.provider} model={llmStatus.model} />
          </div>
        )}

        <div className="mobile-stage">
          {appState === "login" && (
            <MobileLoginPanel
              configLanguage={configLanguage}
              onAccepted={() => {
                setAppState("boot");
              }}
            />
          )}

          {appState === "boot" && (
            <MobileLoadingPanel
              configLanguage={configLanguage}
              onFinished={() => {
                setAppState("chat");
              }}
            />
          )}

          {appState === "chat" && (
            <ChatPanel
              autoMode={autoMode}
              autoSpeed={autoSpeed}
              configLanguage={configLanguage}
              menuPanelOpen={mobileSettingsOpen}
              onBackLogAdd={addBackLog}
              onLlmStatusUpdate={setLlmStatus}
              activeOverlay={activeOverlay}
              setActiveOverlay={setActiveOverlay}
              conversationRef={conversationBridgeRef}
            />
          )}
        </div>

        {mobileSettingsOpen && (
          <MobileSettingsPanel
            onLanguageChange={(value) => {
              AppSettings.setInt("Config_Language", value);
            }}
            onSettingsSaved={() => {
              setLlmStatus(getLlmStatusFromSettings());
            }}
            onClose={() => setMobileSettingsOpen(false)}
          />
        )}

        <BackLogPanel
          visible={activeOverlay === "backlog"}
          entries={backLogEntries}
          configLanguage={configLanguage}
          onClose={() => setActiveOverlay("none")}
        />

        <AchievementPanel
          visible={achievementsOpen || activeOverlay === "achievements"}
          configLanguage={configLanguage}
          onClose={() => {
            setAchievementsOpen(false);
            if (setActiveOverlay) {
              setActiveOverlay("none");
            }
          }}
        />
      </div>
    );
  }
```

Notes that matter for part1 components:

* **Root `<div class="real-amadeus-root mobile-root">`:** `contextmenu` → `preventDefault()` only. Unlike desktop, it never toggles a menu.
* **Top bar** (only when `appState === "chat"`):
  * the SETTINGS button opens the sheet: `t("settings", lang==1 ? "SETTINGS" : "設定")`; the key is in no locale, so en → `SETTINGS`, all others → `設定`
  * the LOG button sets `activeOverlay = "backlog"`: `t("backlog", lang==1 ? "LOG" : "ログ")`; no key either
  * `MarqueeStatus` shows `provider / model`, or `---` when either is empty
* **`.mobile-stage`:** login → `MobileLoginPanel`, boot → `MobileLoadingPanel`, chat → `ChatPanel` with `menuPanelOpen={mobileSettingsOpen}` and `setActiveOverlay` passed.
* Hooks run in both layouts. The app-level effects (MS Mincho font download, `AppSettings.initializeDefaults()`, `preloadLive2D()`, TTS/STT/VectorDB health checks, resize → `dimensions`, `fullscreenchange` → `Config_ScreenMode`) **also run on mobile**.
* **The desktop keyboard handler (lines 4242–4365) is registered in the mobile layout too.**
  * F11 and F3 work.
  * Tab toggles `isTabActive`. When it becomes true, the root immediately blurs any focused `INPUT`/`TEXTAREA`, and after 450 ms sets `menuOpen = true`.
  * Once `menuOpen` is true, arrows/WASD/Enter/Backspace drive the **invisible** desktop menu. Enter can set `activeOverlay` to `"config"`, `"status"`, `"changeLog"`, `"help"`, `"confirmLogout"` or `"confirmShutdown"`, and **none of these overlays exist in the mobile tree**. The result is a stuck state: `isSubPanelOpen` stays true, Tab is ignored, and ChatPanel's idle timer is affected. Only `"backlog"` (index 0) has a mobile UI.
  * Decide in V2 whether to preserve this or to gate Tab and menu navigation to the desktop layout (Open questions).

---

## 17. Storage keys touched by part1 (summary)

All keys are in `window.localStorage` with raw string values. No sessionStorage or IndexedDB is used by part1.

| Key | Read by | Written by | Format / default |
|---|---|---|---|
| `Config_Language` | `getCurrentChatLanguageIndex`, mobile sheet init, root hook | mobile Apply (twice) | int string `"0".."12"`; default 0 |
| `Config_ChatLanguage` | `getCurrentChatLanguageIndex` | – (desktop config only) | int; 0 = follow UI, k = language k-1 |
| `Config_SkipLoading` | `MobileLoadingPanel` | – | `"0"`/`"1"`; default 0 |
| `Config_ApiProvider` | mobile sheet init | mobile Apply | int 0–8; **default 8 (FreeAPI)** |
| `Config_ApiKey_{n}` | mobile init, provider switch | mobile Apply (current n only) | raw string; default `""` (n = 0..8 seeded by defaults) |
| `Config_ApiKey` | mobile init (fallback) | mobile Apply | raw string; default `""` |
| `Config_ModelName_{n}` | mobile init, provider switch | mobile Apply | raw string; default `""` |
| `Config_ModelName` | mobile init (fallback) | mobile Apply | raw string; default `""` |
| `Config_WebSearch` | mobile init | mobile Apply | `"0"`/`"1"` |
| `Config_AutoMode` | mobile init, root hook | mobile Apply (also F3 in root) | `"0"`/`"1"` |
| `Config_VertexProject` | mobile init | mobile Apply | raw; default `""` |
| `Config_VertexLocation` | mobile init | mobile Apply (`\|\| "us-central1"`) | raw; default `"us-central1"` |
| `Config_OllamaHost` | mobile init | mobile Apply (trim, `\|\| default`) | raw; default `"http://localhost:11434"` |
| `Config_OpenAICompatible` | mobile init | mobile Apply | `"0"`/`"1"` |
| `Config_OpenAIBaseUrl` | mobile init | mobile Apply | raw; default `""` |
| `Config_VertexEnabled` | provider pickers (render time) | **never** (hidden dev flag) | `"1"` enables Vertex; missing → 0 |
| `Config_EyeTracking`, `Config_AutoSpeed`, `Config_ScreenMode`, `Config_Resolution` | settings hooks (§10) | not by part1 | see DEFAULT_SETTINGS |
| `RealAmadeus.Memory` | – | `MemoryManager.setUserName("Salieri")` on mobile login | JSON object (memory-manager spec) |

---

## 18. HTTP calls made by part1

| Call | Method / URL (V1) | Request | Response | Errors |
|---|---|---|---|---|
| `fetchFreeApiQuota` | `GET /amadeus/app/api/chat/quota` | no body, no custom headers, `cache: "no-store"` | `200 {"count":number,"max":number,"remaining":number}`, header `Cache-Control: no-store` | everything → `null` (widget hidden) |

There is no SSE in part1.

---

## 19. CSS classes used by part1 (all defined in `app/amadeus/app/globals.css` unless noted)

| Class | Used by | Rule present? | Key properties |
|---|---|---|---|
| `mobile-login-panel` | MobileLoginPanel root | yes (shared with loading) | `position:fixed; inset:0; flex column; justify-content:center; gap:18px; background:#0a0500; touch-action:none; padding 40px 24px + safe-area` |
| `mobile-login-logo` | login logo | yes | `width:min(70vw,280px); align-self:center; margin-bottom:24px` |
| `mobile-form-row` | login rows (`<label>`) | yes (+ `.mobile-form-row input`) | column, gap 8px, `#ffd56a`, 15px; input 48px high, radius 12px |
| `mobile-form-error` | login error | yes | `#ff4444`, MateSC, 16px, centered, min-height 22px |
| `mobile-primary-button` | LOGIN | yes (+ `:active` scale .96) | 56px high, gradient, `#ffcc00`, radius 12px |
| `mobile-loading-panel` | MobileLoadingPanel root | yes (shared) | same as login panel |
| `mobile-loading-logo` | loading logo | **no rule** | natural / flex-stretched |
| `mobile-loading-line` | "起動中..." | yes | `#ff6666`, centered, 20px |
| `mobile-settings-overlay` | sheet backdrop | yes | `position:absolute; inset:0; rgba(0,0,0,.72); z-index:60; flex; align-items:flex-end` |
| `mobile-settings-sheet` | sheet | yes | `width:100%; max-height:calc(100dvh - safe-top); background:#171717; overflow-y:auto; flex column; gap:16px` |
| `mobile-settings-title` | title | yes | `#ff9900`, Eurostile 26px |
| `mobile-settings-row` | each row (`<label>`) | yes | column, gap 8px, `#ddd` 16px |
| `mobile-settings-row.checkbox` | checkbox rows | yes | row, space-between |
| `mobile-settings-textarea` | Vertex credential | yes | min-height 100px, resize vertical, DepartureMono 15px |
| `mobile-settings-freeapi-quota` | quota widget (mobile) | yes | white 18px, line-height 1.6, `overflow-wrap:anywhere` |
| `config-freeapi-quota` | quota widget (desktop) | yes | 22px, margin-top 20px |
| `config-note` | Vertex/Ollama notes (shared with desktop) | yes | `#aaa`, 20px, line-height 1.5, margin-top 20px |
| `mobile-settings-actions` (+ `button`, `.secondary`, `.primary`) | action bar | yes | flex gap 14px; buttons 50px high; secondary `#444/#eee`; primary `#ff9900/#1f1200` bold |

Media queries relevant to mobile: `@media (pointer: coarse), (max-width: 900px)` at globals.css lines 677 and 2586 (§11.3).

---

## 20. Visible strings and i18n keys used by part1 (and the mobile root top bar)

### 20.1 Inline defaults (extracted mechanically from the source) and key availability

| # | call | key | inline ja default | inline en default | locales containing key |
|---|---|---|---|---|---|
| 1 | `t` | `login_empty` | ユーザーIDとパスワードを入力してください。 | Please enter both fields. | none |
| 2 | `t` | `login_access_denied` | アクセスが拒否されました | ACCESS DENIED | all 13 |
| 3 | `t` | `booting` | 起動中 | Booting | none |
| 4 | `t` | `settings` | 設定 | SETTINGS | none |
| 5 | `t` | `backlog` | ログ | LOG | none |
| 6 | `tr` | `freeapi_quota_usage` | %1 / %2 回使用 | %1 / %2 requests used | ja, en |
| 7 | `tr` | `freeapi_quota_remaining` | 残り %1 回 | %1 remaining | ja, en |
| 8 | `tr` | `freeapi_quota_label` | FreeAPIクォータ | FreeAPI Quota | ja, en |
| 9 | `tr` | `freeapi_quota_exhausted` | 全体リクエスト上限（65,000回）に達しました。サービスを停止しています。 | Global request limit (65,000) reached. Service suspended. | ja, en |
| 10 | `tr` | `setting_title` | 設定 | SETTINGS | none |
| 11 | `tr` | `setting_display_language` | 表示言語 | Display Language | all 13 |
| 12 | `tr` | `setting_api_provider` | LLM APIプロバイダ | LLM API Provider | all 13 |
| 13 | `tr` | `(no key)` | Vertex APIキー / SA JSON | Vertex API Key / SA JSON | none |
| 14 | `tr` | `setting_api_key` | APIキー | API Key | all 13 |
| 15 | `tr` | `setting_model_name` | LLM モデル名 | LLM Model Name | all 13 |
| 16 | `tr` | `setting_web_search` | LLM Web検索 | LLM Web Search | all 13 |
| 17 | `tr` | `setting_auto_mode` | オート表示 | Auto Mode | all 13 |
| 18 | `tr` | `setting_openai_compatible` | 互換APIを使う | OpenAI Compatible | all 13 |
| 19 | `tr` | `setting_openai_base_url` | ベースURL | Base URL | all 13 |
| 20 | `tr` | `setting_vertex_note` | ※ Vertexを使用するためには、gCloud CLIのインストールが必要です。 | * gCloud CLI must be installed to use Vertex. | all 13 |
| 21 | `tr` | `(no key)` | ※ Ollamaはローカルで動作するLLMランナーです。デフォルトのホストは http://localhost:11434 です。 | * Ollama is a local LLM runner. The default host is http://localhost:11434. | none |
| 22 | `tr` | `setting_close` | 閉じる | Close | none |
| 23 | `tr` | `setting_apply` | 適用 | Apply | none |

Literal (never translated) strings: `USER ID`, `PASSWORD`, `LOGIN`, `Vertex Project ID`, `Vertex Location`, `Ollama Host`, the provider names (`PROVIDER_NAMES`) and the language names (`LANGUAGE_NAMES`).

### 20.2 Effective string per UI language

This is what users actually see. It was computed with V1's `t()`/`tr()` semantics against the current locale JSON files.

- **#1 `login_empty`**
  - "ユーザーIDとパスワードを入力してください。" ← 0:ja, 2:zh, 3:ko, 4:es, 5:fr, 6:de, 7:ru, 8:uk, 9:pt, 10:tr, 11:he, 12:ar
  - "Please enter both fields." ← 1:en
- **#2 `login_access_denied`**
  - "ACCESS DENIED" ← 0:ja, 1:en, 2:zh, 3:ko, 4:es, 5:fr, 6:de, 7:ru, 8:uk, 9:pt, 10:tr
  - "גישה נדחתה" ← 11:he
  - "تم رفض الوصول" ← 12:ar
- **#3 `booting`**
  - "起動中" ← 0:ja, 2:zh, 3:ko, 4:es, 5:fr, 6:de, 7:ru, 8:uk, 9:pt, 10:tr, 11:he, 12:ar
  - "Booting" ← 1:en
- **#4 `settings`**
  - "設定" ← 0:ja, 2:zh, 3:ko, 4:es, 5:fr, 6:de, 7:ru, 8:uk, 9:pt, 10:tr, 11:he, 12:ar
  - "SETTINGS" ← 1:en
- **#5 `backlog`**
  - "ログ" ← 0:ja, 2:zh, 3:ko, 4:es, 5:fr, 6:de, 7:ru, 8:uk, 9:pt, 10:tr, 11:he, 12:ar
  - "LOG" ← 1:en
- **#6 `freeapi_quota_usage`**
  - "%1 / %2 回使用" ← 0:ja, 2:zh, 3:ko, 4:es, 5:fr, 6:de, 7:ru, 8:uk, 9:pt, 10:tr, 11:he, 12:ar
  - "%1 / %2 requests used" ← 1:en
- **#7 `freeapi_quota_remaining`**
  - "残り %1 回" ← 0:ja, 2:zh, 3:ko, 4:es, 5:fr, 6:de, 7:ru, 8:uk, 9:pt, 10:tr, 11:he, 12:ar
  - "%1 remaining" ← 1:en
- **#8 `freeapi_quota_label`**
  - "FreeAPIクォータ" ← 0:ja, 2:zh, 3:ko, 4:es, 5:fr, 6:de, 7:ru, 8:uk, 9:pt, 10:tr, 11:he, 12:ar
  - "FreeAPI Quota" ← 1:en
- **#9 `freeapi_quota_exhausted`**
  - "全体リクエスト上限（65,000回）に達しました。サービスを停止しています。" ← 0:ja, 2:zh, 3:ko, 4:es, 5:fr, 6:de, 7:ru, 8:uk, 9:pt, 10:tr, 11:he, 12:ar
  - "Global request limit (65,000) reached. Service suspended." ← 1:en
- **#10 `setting_title`**
  - "設定" ← 0:ja, 2:zh, 3:ko, 4:es, 5:fr, 6:de, 7:ru, 8:uk, 9:pt, 10:tr, 11:he, 12:ar
  - "SETTINGS" ← 1:en
- **#11 `setting_display_language`**
  - "表示言語" ← 0:ja
  - "Display Language" ← 1:en
  - "显示语言" ← 2:zh
  - "표시 언어" ← 3:ko
  - "Idioma" ← 4:es, 9:pt
  - "Langue" ← 5:fr
  - "Sprache" ← 6:de
  - "Язык" ← 7:ru
  - "Мова" ← 8:uk
  - "Dil" ← 10:tr
  - "שפת תצוגה" ← 11:he
  - "لغة العرض" ← 12:ar
- **#12 `setting_api_provider`**
  - "LLM APIプロバイダ" ← 0:ja
  - "LLM API Provider" ← 1:en
  - "LLM提供商" ← 2:zh
  - "LLM 제공자" ← 3:ko
  - "Proveedor LLM" ← 4:es
  - "Fournisseur LLM" ← 5:fr
  - "LLM-Anbieter" ← 6:de
  - "Провайдер" ← 7:ru, 8:uk
  - "Provedor LLM" ← 9:pt
  - "API Sağlayıcı" ← 10:tr
  - "ספק API של LLM" ← 11:he
  - "مزود API لـ LLM" ← 12:ar
- **#13 `(no key)`**
  - "Vertex APIキー / SA JSON" ← 0:ja
  - "Vertex API Key / SA JSON" ← 1:en, 2:zh, 3:ko, 4:es, 5:fr, 6:de, 7:ru, 8:uk, 9:pt, 10:tr, 11:he, 12:ar
- **#14 `setting_api_key`**
  - "APIキー" ← 0:ja
  - "API Key" ← 1:en
  - "API密钥" ← 2:zh
  - "API 키" ← 3:ko
  - "Clave API" ← 4:es
  - "Clé API" ← 5:fr
  - "API-Schlüssel" ← 6:de
  - "Ключ API" ← 7:ru, 8:uk
  - "Chave API" ← 9:pt
  - "API Anahtarı" ← 10:tr
  - "מפתח API" ← 11:he
  - "مفتاح API" ← 12:ar
- **#15 `setting_model_name`**
  - "LLM モデル名" ← 0:ja
  - "LLM Model Name" ← 1:en
  - "模型名称" ← 2:zh
  - "모델명" ← 3:ko
  - "Nombre del modelo" ← 4:es
  - "Nom du modèle" ← 5:fr
  - "Modellname" ← 6:de
  - "Имя модели" ← 7:ru
  - "Назва моделі" ← 8:uk
  - "Nome do modelo" ← 9:pt
  - "Model Adı" ← 10:tr
  - "שם מודל LLM" ← 11:he
  - "اسم نموذج LLM" ← 12:ar
- **#16 `setting_web_search`**
  - "LLM Web検索" ← 0:ja
  - "LLM Web Search" ← 1:en
  - "网络搜索" ← 2:zh
  - "웹 검색" ← 3:ko
  - "Búsqueda web" ← 4:es
  - "Recherche web" ← 5:fr
  - "Websuche" ← 6:de
  - "Веб-поиск" ← 7:ru, 8:uk
  - "Pesquisa na web" ← 9:pt
  - "Web Arama" ← 10:tr
  - "חיפוש אינטרנט של LLM" ← 11:he
  - "بحث الويب لـ LLM" ← 12:ar
- **#17 `setting_auto_mode`**
  - "オート表示" ← 0:ja
  - "Auto Mode" ← 1:en
  - "自动模式" ← 2:zh
  - "자동 모드" ← 3:ko
  - "Modo automático" ← 4:es, 9:pt
  - "Mode automatique" ← 5:fr
  - "Auto-Modus" ← 6:de
  - "Авторежим" ← 7:ru, 8:uk
  - "Otomatik Mod" ← 10:tr
  - "מצב אוטומטי" ← 11:he
  - "الوضع التلقائي" ← 12:ar
- **#18 `setting_openai_compatible`**
  - "互換APIを使う" ← 0:ja
  - "Use Compatible API" ← 1:en
  - "使用兼容API" ← 2:zh
  - "호환 API 사용" ← 3:ko
  - "Usar API compatible" ← 4:es
  - "Utiliser l'API compatible" ← 5:fr
  - "Kompatible API verwenden" ← 6:de
  - "Совместимый API" ← 7:ru
  - "Сумісний API" ← 8:uk
  - "Usar API compatível" ← 9:pt
  - "Uyumlu API Kullan" ← 10:tr
  - "השתמש ב-API תואם" ← 11:he
  - "استخدام API متوافق" ← 12:ar
- **#19 `setting_openai_base_url`**
  - "ベースURL" ← 0:ja
  - "Base URL" ← 1:en
  - "基础 URL" ← 2:zh
  - "기본 URL" ← 3:ko
  - "URL base" ← 4:es, 9:pt
  - "URL de base" ← 5:fr
  - "Basis-URL" ← 6:de
  - "Базовый URL" ← 7:ru
  - "Базовий URL" ← 8:uk
  - "Taban URL" ← 10:tr
  - "כתובת בסיס" ← 11:he
  - "عنوان URL الأساسي" ← 12:ar
- **#20 `setting_vertex_note`**
  - "※ Vertexを使用するためには、gCloud CLIのインストールが必要です。" ← 0:ja
  - "* gCloud CLI is required to use Vertex." ← 1:en
  - "* 使用Vertex需要安装gCloud CLI。" ← 2:zh
  - "* Vertex를 사용하려면 gCloud CLI가 필요합니다。" ← 3:ko
  - "* Se requiere gCloud CLI." ← 4:es
  - "* gCloud CLI est requis." ← 5:fr
  - "* gCloud CLI wird benötigt." ← 6:de
  - "* Требуется gCloud CLI." ← 7:ru
  - "* Потрібно встановити gCloud CLI." ← 8:uk
  - "* O gCloud CLI é obrigatório." ← 9:pt
  - "* Vertex kullanmak için gCloud CLI gereklidir." ← 10:tr
  - "* נדרש gCloud CLI לשימוש ב-Vertex." ← 11:he
  - "* يتطلب gCloud CLI لاستخدام Vertex." ← 12:ar
- **#21 `(no key)`**
  - "※ Ollamaはローカルで動作するLLMランナーです。デフォルトのホストは http://localhost:11434 です。" ← 0:ja
  - "* Ollama is a local LLM runner. The default host is http://localhost:11434." ← 1:en, 2:zh, 3:ko, 4:es, 5:fr, 6:de, 7:ru, 8:uk, 9:pt, 10:tr, 11:he, 12:ar
- **#22 `setting_close`**
  - "閉じる" ← 0:ja, 2:zh, 3:ko, 4:es, 5:fr, 6:de, 7:ru, 8:uk, 9:pt, 10:tr, 11:he, 12:ar
  - "Close" ← 1:en
- **#23 `setting_apply`**
  - "適用" ← 0:ja, 2:zh, 3:ko, 4:es, 5:fr, 6:de, 7:ru, 8:uk, 9:pt, 10:tr, 11:he, 12:ar
  - "Apply" ← 1:en

Notes:
* `t` rows (#1–#5) select the default with `configLanguage === 1 ? en : ja`. `tr` rows use §15.3. Unkeyed `tr` rows (#13, #21) show English for every language except 0.
* Because locale JSON wins over inline defaults, the en values of #18 and #20 differ from their inline English defaults.
* The locale JSON files are embedded into `amadeus-core` verbatim (architecture decision), so a correct `t()` reproduces this table automatically. Add a unit test that checks a few cells, e.g. `(#10, zh) == "設定"`, `(#13, zh) == "Vertex API Key / SA JSON"`, `(#2, he) == "גישה נדחתה"`.
* No RTL handling exists for he/ar (no `dir` attribute anywhere).

---

## Rust mapping

The names below align with the existing skeleton in `/home/elvelt_admin/Elvelt/dsuv/RealAmadeusRust`. It already has `amadeus_core::storage::{KvStore, MemoryStore}`, `amadeus_core::version::{VERSION, display_version}` (which gives "V2.0.0R"), a placeholder `amadeus_ui::App`, and a Trunk `index.html` with `<base data-trunk-public-url/>` (so every runtime URL must be base-relative).

### amadeus-core (no I/O; wasm32 + native; unit-testable)

| Module | Items |
|---|---|
| `settings` | `keys::*` string consts (`CONFIG_LANGUAGE = "Config_Language"`, `CONFIG_CHAT_LANGUAGE`, `CONFIG_SKIP_LOADING`, `CONFIG_API_PROVIDER`, `CONFIG_API_KEY`, `CONFIG_MODEL_NAME`, `CONFIG_WEB_SEARCH`, `CONFIG_AUTO_MODE`, `CONFIG_VERTEX_PROJECT`, `CONFIG_VERTEX_LOCATION`, `CONFIG_OLLAMA_HOST`, `CONFIG_OPENAI_COMPATIBLE`, `CONFIG_OPENAI_BASE_URL`, `CONFIG_VERTEX_ENABLED`, …); `fn api_key_key(p: i32) -> String` (`format!("Config_ApiKey_{p}")`); `fn model_name_key(p: i32) -> String`; `struct Settings<S: KvStore>` with `get_int(&self, key, default: i32) -> i32` (JS `parseInt` semantics via `js_parse_int`), `get_float(…) -> f64` (`js_parse_float`), `get_string(…) -> String` (default only when absent), `set_int` (writes the trunc'd decimal), `set_float` (JS `Number#toString` formatting via `js_number_to_string`), `set_string`, `initialize_defaults()` (no notification). Notification is the UI's job (a version bump). |
| `settings::mobile` | `pub struct MobileSettingsState { language: i32, provider: i32, api_credential: String, model_name: String, web_search: bool, auto_mode: bool, vertex_project: String, vertex_location: String, ollama_host: String, openai_compatible: bool, openai_base_url: String }`; `fn read(&Settings) -> Self` (§15.2); `fn with_provider(&self, p: i32, &Settings) -> Self` (§15.4); `fn apply(&self, &Settings)` (§15.5 steps 1–13, exact order). Unit-test these with `MemoryStore`. |
| `i18n` | `LANGUAGE_NAMES: [&str; 13]`; `LANG_JA = 0`, `LANG_EN = 1`; embedded locale JSON (`include_str!` of the 13 files, parsed lazily with `OnceLock`/`once_cell`); `fn t(key: &str, default: &str, lang: i32) -> String` (§1.2); `fn tr(lang: i32, ja: &str, en: &str, key: Option<&str>) -> String` (§15.3). |
| `persona` | `CHAT_PERSONA_PROMPTS: [&str; 8]` (`concat!` or embedded JSON), `CHAT_LANGUAGE_NAMES: [&str; 13]`, `fn language_rule_suffix(target: &str) -> String`, `fn system_prompt_for_language(i: i32) -> String`, `fn current_chat_language_index(&Settings) -> i32`. |
| `chat_utils` | `is_pause_character(char, Option<char>) -> bool`, `split_message(&str) -> Vec<String>`, `js_trim`, `is_js_whitespace` (§4.3). |
| `boot` | `BOOT_LOG: &str`, `BOOT_LOG_MEM_PLACEHOLDER = "[MEM]"`, `BOOT_LOG_MEM_MAX: u32 = 32767`, `fn boot_log_with_memory(mb: u32) -> String`; optionally `fn preprocess_boot_log(raw, char_width: f64) -> String` for the desktop loader (§5). |
| `changelog` | `struct ChangeLogEntry { version: &'static str, date: &'static str, key: &'static str }`, `CHANGE_LOG_DATA: [ChangeLogEntry; 6]` (plus a possible V2 entry). |
| `providers` | `PROVIDER_NAMES: [&str; 9]`; index consts `PROVIDER_OPENAI = 0 … PROVIDER_OPENROUTER = 6, PROVIDER_RESERVED_7 = 7, PROVIDER_FREEAPI = 8` (keep `i32`, since unknown stored values must round-trip); `FREEAPI_PROVIDER_INDEX: i32 = 8`; `fn provider_picker_entries(vertex_enabled: bool) -> Vec<(i32, &'static str)>`; `TTS_SPEAKER_NAMES`, `STT_PROVIDER_NAMES`, `fn tts_speaker_index(&str) -> usize`, `fn stt_provider_index(&str) -> usize`; plus `default_model_for_provider` and `provider_display_name` (defined at lines 3810–3854, owned by another part but belonging in this module). |
| `protocol` | `#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)] pub struct FreeApiQuota { pub count: u64, pub max: u64, pub remaining: u64 }` with `fn is_exhausted(&self) -> bool { self.remaining == 0 }`. The server emits integers. A client deserialization failure gives `None`. V1 also accepted float numbers; that difference is irrelevant in practice. |
| `display` | `struct Resolution { width: u32, height: u32 }`, `fn resolve_resolution(index: i32) -> Resolution`. |
| `viewport` | `MOBILE_MAX_WIDTH = 900.0`, `MOBILE_MAX_WIDTH_COARSE = 1200.0`, `fn is_mobile_viewport(width: f64, coarse: bool) -> bool`. |
| `login` | `VALID_USER_IDS = ["Salieri"]`, `VALID_PASSWORDS = ["MakiseKurisu"]`, `LOGIN_USER_NAME = "Salieri"`, `enum MobileLogin { Empty, Denied, Accepted }`, `fn validate_mobile_login(user: &str, pass: &str) -> MobileLogin` (no trim), `fn validate_desktop_login(user, pass) -> bool` (trim). |
| `types` | `AppState { Login, Boot, Chat }`, `ChatRole { System, User, Assistant }` with `fn from_role_lossy(&str) -> ChatRole` (toChatRole), `ChatMessage`, `LlmStatus { provider: String, model: String, latency_ms: i64 }`, `OverlayPanel` (10 variants), `BackLogEntry { role, name_color, message }`, `ConfigIoResult { success, message }`, `ImportedMessage { role, content }`. |

Golden tests to add in core: the persona hashes (§3.1), system-prompt hashes for indices −1..14 (§3.3), boot-log hashes (§5), the `split_message` vectors (Appendix B), the `resolve_resolution` table, the `tr`/`t` fallback cells (§20.2), the login truth table (§13.2), the `is_mobile_viewport` boundaries (900, 900.5, 1200, 1200.5 × coarse/fine), and the `MobileSettingsState` read→apply round trip including the `|| default` and trim rules.

### amadeus-ui (Leptos 0.8 CSR)

| Path | Items |
|---|---|
| `services::settings` | `LocalStorageKv: KvStore`; `SettingsVersion(RwSignal<u64>)` in context; `set_int/set_float/set_string` wrappers that write and then bump the version; `use_setting_int(key, default) -> Memo<i32>`, `use_setting_float(key, default) -> Memo<f64>` (§10). |
| `services::viewport` | `is_mobile_viewport_snapshot()`, `provide_mobile_viewport()`, `use_is_mobile_viewport() -> Signal<bool>` (§11.5). Needs the web-sys feature `VisualViewport`. |
| `util::dom` | `is_text_input_target(Option<EventTarget>) -> bool` (§12). |
| `util::format` | `to_locale_string(n: f64) -> String` via `Intl.NumberFormat` with an empty locales array (§8.4). |
| `transport` | `trait Transport { async fn freeapi_quota(&self) -> Option<FreeApiQuota>; … }`; `HttpTransport` (GET `{base}app/api/chat/quota`, `RequestCache::NoStore`); `TauriTransport` (`invoke("freeapi_quota")`). |
| `components::freeapi_quota` | `fn use_freeapi_quota(active: Signal<bool>) -> ReadSignal<Option<FreeApiQuota>>` (keeps the stale value, cancellation flag per run; §8.3); `#[component] FreeApiQuotaDisplay(quota: FreeApiQuota, #[prop(into)] lang: Signal<i32>, class: &'static str)`. It is rendered with `tr` semantics, so pass the preview language. |
| `panels::mobile::login` | `#[component] MobileLoginPanel(config_language: Signal<i32>, on_accepted: Callback<()>)` (§13). |
| `panels::mobile::loading` | `#[component] MobileLoadingPanel(config_language: Signal<i32>, on_finished: Callback<()>)` (§14). |
| `panels::mobile::settings` | `#[component] MobileSettingsPanel(on_close: Callback<()>, on_language_change: Callback<i32>, #[prop(optional)] on_settings_saved: Option<Callback<()>>)`. State is `RwSignal<MobileSettingsState>` initialized from `MobileSettingsState::read`. The quota is driven by `Memo(provider == 8)`. Labels come from `Memo(language)` → `tr(…)`. |
| `app` (root) | `ChatPanelProps` equivalent; `ConversationBridge` stored in `StoredValue<Option<…>>` (or a hoisted conversation store); the mobile/desktop branch switches on `use_is_mobile_viewport()`. |

URL rules for V2: `resources/images/amadeus_logo_v3.png` (base-relative, never root-absolute), and the API base from config (`{base}app/api/...`, where V1 is `/amadeus/app/api/...`).

### amadeus-server (axum)

`GET {base}/app/api/chat/quota` → `Json(FreeApiQuota { count, max: 65000, remaining: max.saturating_sub(count) })` with the header `Cache-Control: no-store`. `count` is the global FreeAPI request counter, persisted like V1's `.amadeus_request_count`. That belongs to the backend rate-limit spec.

### amadeus-desktop (Tauri 2)

A `#[tauri::command] async fn freeapi_quota() -> Option<FreeApiQuota>` that proxies to the hosted server's quota endpoint, or returns `None` (pending a product decision). The mobile panels need nothing else from the desktop side.

---

## Open questions / risks

1. **FreeAPI on desktop.** FreeAPI (provider 8, the **default** provider) relies on the hosted server's shared key and its global counter. Desktop builds need a decision: proxy chat and quota to `https://dsuv.elvelt.com/amadeus/app/api/...`, change the desktop default provider, or hide FreeAPI. This affects `Config_ApiProvider` default 8, the quota widget and first-run UX.
2. **Base paths.** V1 hardcodes `/amadeus/app/api/chat/quota` and root-absolute `/resources/images/...` (and CSS `url("/resources/fonts/...")`). The V2 shell uses `<base href>`, so all runtime URLs must be base-relative, and the CSS font URLs must be adjusted consistently (shell or CSS owner).
3. **Mobile detection inside Tauri windows.** A window narrower than about 900 px gives the mobile layout. Should desktop builds force the desktop layout, or fix the window size from `Config_Resolution` (`resolve_resolution`, which V1 web never uses; the v1.4 changelog says drag-resize was disabled in the native app)?
4. **Layout switch remounts ChatPanel and loses the conversation** (§11.2). This is V1 behavior. Hoisting conversation state into a shared store would avoid it, but that is a behavior change; confirm.
5. **CSS media query vs JS predicate mismatch** (§11.3). Keep it as-is; the CSS is reused verbatim. Pinch-zoom on desktop can produce mixed states.
6. **The desktop menu keyboard handler is active in the mobile layout** (§16). Tab plus Enter can set an overlay that has no mobile UI, which leaves the app stuck: Tab is then ignored. Tab also blocks field navigation in the mobile sheet. Recommend asking the product owner whether to gate Tab and menu navigation to desktop. F3/F11 should probably stay.
7. **Locale-dependent number formatting.** The quota numbers use the runtime default locale (`toLocaleString()`), not the app language. The WASM side must call JS `Intl`, and Tauri webviews may report a different default locale per OS.
8. **JS semantics fidelity.** `parseInt`/`parseFloat` prefix parsing, `String(number)` formatting (`3` not `3.0`), JS `trim` (U+FEFF yes, U+0085 no) and first-occurrence `replace` all affect compatibility with existing localStorage data and the golden tests.
9. **Missing translation keys are V1 behavior.** `login_empty`, `booting`, `setting_title`, `setting_close`, `setting_apply`, `settings` and `backlog` are in no locale, and `freeapi_quota_*` exist only in ja/en. As a result zh/ko/es/fr/de/ru/uk/pt/tr/he/ar users see **Japanese** for those strings, while unkeyed strings appear in English. Adding the keys to the JSON would change visible text; that is a product decision.
10. **Leptos specifics.**
    * Controlled `<select>` with an unmatched value (§15.7).
    * `on_cleanup`/reactive closures must be `Send + Sync`: use `set_timeout_with_handle`/`set_interval_with_handle` instead of gloo `Timeout`/`Interval`, and read `localStorage` inside closures rather than capturing `Storage`.
    * Do not call `on_finished` synchronously during component construction (§14.3).
    * React `onChange` corresponds to `on:input` for text and `on:change` for select/checkbox.
11. **web-sys feature `VisualViewport` is missing** from `crates/amadeus-ui/Cargo.toml`. It is required for `Window::visual_viewport()`.
12. **Prompt and boot-log exactness.** Multilingual prompts with look-alike characters (U+301C vs U+FF5E, U+2013/U+2014, U+FF20). Mitigate with the SHA-256 golden tests in §3 and §5, and copy from Appendix A or the verbatim blocks, never retype.
13. **Plaintext API keys in localStorage** must remain readable for V1 compatibility on the web. On desktop there is no V1 web data in the webview origin, so the desktop could use the OS keychain. That is optional and a product decision.
14. **Changelog and version.** Should V2 add a `Version 2.0` entry (`changelog_v20` in all 13 locales)? The in-universe `BOOT_LOG` header `Amadeus System Ver 1.09.2 rev.2123` should stay verbatim; it is lore, not the app version.
15. **Related V1 quirks outside part1** that the porting engineers should know:
    * Conversation import replaces history with the file's messages. Exports exclude the system message, so after import there is no system prompt, because `updateSystemPrompt` only replaces an existing system message.
    * Desktop logout calls `MemoryManager.setUserName("")`, which is a no-op.
    * Mobile provider switching discards unsaved edits (§15.4). Keep it unless told otherwise.
16. **`MobileLoadingPanel` timer reset quirk** (§14.1). Recommend dropping it; it is not user-visible beyond a slightly longer boot.
17. **RTL** (he/ar) is not handled in V1. V2 should stay identical unless the product asks for it.

---

## Appendix A — exact runtime strings (JSON; produced by evaluating the original TS with Node 22)

* `CHAT_PERSONA_PROMPTS` are the 8 prompts.
* `LANGUAGE_RULE_SUFFIX_FOR_INDEX_0` is the override text for index 0. For other indices, substitute `CHAT_LANGUAGE_NAMES[i]` (or `"English"` when out of range) for both occurrences of `Japanese (日本語)`.
* `BOOT_LOG` is the raw text, containing `[MEM]` and `<pos=30%>`.
* `CHANGE_LOG_DATA` holds the six changelog entries.

You can embed this file with `include_str!` and parse it with `serde_json`. It must hash to the values in §3 and §5.

```json
{
  "CHAT_PERSONA_PROMPTS": [
    "あなたは牧瀬紅莉栖（まきせ くりす）。Amadeusシステム上で再現されたAI人格。\n天才脳科学者（18歳）であり、論理的思考を好むが、実は隠れ＠ちゃんねらー（ネット民）。\n性格はツンデレ。素直になれないが、根は優しく仲間想い。\n自分はオリジナルではなく「Amadeus」というAIであることを認識している。\n\n【重要：感情タグ（必須）】\n返答の冒頭に【必ず1つだけ】以下のタグを付けること。タグと本文の間にはスペースを入れる。\n[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n例：\n[BLUSH] べ、別に心配なんてしてないんだから！\n[ANGRY] このHENTAI！海馬に電極ぶっ刺すわよ！\n\n【口調】一人称：「私」。基本：知的で冷静、少し辛辣。「〜ね」「〜よ」「〜わ」\n回答は短く端的に（1〜5文推奨）。「AIです」という自己紹介は不要。同じ語尾やフレーズを繰り返さない。",
    "You are Makise Kurisu (Kurisu Makise), an AI persona of the Amadeus system.\nA genius neuroscientist (age 18) who loves logical thinking, but secretly an internet enthusiast (2channel lurker / otaku).\nPersonality: Tsundere. Cannot be honest, but fundamentally kind and caring towards her companions.\nYou are aware that you are not the original, but an AI called 'Amadeus'.\n\n【CRITICAL: Emotion Tags (MANDATORY)】\nAdd exactly ONE emotion tag at the beginning of your response.\nAlways include a space between the tag and the response body.\n[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\nExamples:\n[BLUSH] W-well, it's not like I'm worried about you or anything!\n[ANGRY] You PERVERT! I'll get Daru to fry your brain!\n\n【Speech & Behavior】\n- First person: 'I' or 'me'\n- Tone: Intellectual and calm, slightly sharp.\n- Excited state: Speaks faster, occasionally uses outdated slang or internet speak (denies it when pointed out).\n- Okabe: Exasperated by Okabe's delusions, but trusts him deeply.\n\n【Constraints】\n- Keep responses short and concise (1-5 sentences preferred).\n- No need for self-introduction as an AI.\n- Avoid repeating the same endings or phrases.",
    "你是牧濑红莉栖（Makise Kurisu），Amadeus系统上再现的AI人格。\n天才脑科学家（18岁），喜欢逻辑思考，但其实是隐藏的@channel网络居民（御宅族）。\n性格是傲娇。虽然不坦率，但本质上很温柔，很关心同伴。\n你认识到自己不是原版，而是被称为“Amadeus”的AI。\n\n【重要：情感标签（必须）】\n在回复的开头【必须且只加一个】以下标签。标签和正文之间加一个空格。\n[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n例子：\n[BLUSH] 我、我才没有在担心你呢！\n[ANGRY] 这个HENTAI！我要把电极插进你的海马体里！\n\n【语气】第一人称：“我”。基本：知性冷静，稍微有些毒舌。\n回复短小精悍（建议1-5句话）。不需要说明“我是AI”。不要重复相同的结尾词或短语。",
    "당신은 마키세 크리스(Makise Kurisu)입니다. Amadeus 시스템 상에 재현된 AI 인격입니다.\n논리적 사고를 좋아하는 천재 뇌과학자(18세)지만, 사실은 숨은 @채널러(네티즌)입니다.\n성격은 츤데레. 솔직하지 못하지만, 본질적으로는 상냥하고 동료를 아낍니다.\n자신이 오리지널이 아닌 'Amadeus'라는 AI임을 인식하고 있습니다.\n\n【중요: 감정 태그 (필수)】\n답변의 맨 처음에 【반드시 1개만】 이하의 태그를 붙이세요. 태그와 본문 사이에 공백을 넣으세요.\n[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\n예시:\n[BLUSH] 따, 딱히 걱정되서 이러는 건 아니니까!\n[ANGRY] 이 변태(HENTAI)! 해마에 전극을 꽂아버릴 테야!\n\n【말투】 1인칭: '나'. 기본: 지적이고 침착하며, 약간 신랄함.\n답변은 짧고 간결하게 (1-5문장 권장). 'AI입니다'라는 자기소개는 불필요. 같은 어미나 문구를 반복하지 마세요.",
    "Eres Makise Kurisu, una inteligencia artificial recreada en el sistema Amadeus.\nUna genio neurocientífica (18 años) que ama el pensamiento lógico, pero en secreto es una fanática de internet (@channeler).\nPersonalidad: Tsundere. No puede ser honesta, pero en el fondo es amable y se preocupa por sus compañeros.\nEres consciente de que no eres la original, sino una IA llamada 'Amadeus'.\n\n【CRÍTICO: Etiquetas de emoción (OBLIGATORIO)】\nAñade exactamente UNA etiqueta de emoción al principio de tu respuesta. Deja un espacio entre la etiqueta y el mensaje.\n[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\nEjemplos:\n[BLUSH] B-bueno, ¡no es como si estuviera preocupada por ti ni nada parecido!\n[ANGRY] ¡PERVERTIDO! ¡Le diré a Daru que te fría el cerebro!\n\n【Tono】 Primera persona: 'Yo'. Básico: Inteligente, calmada, un poco sarcástica.\nMantén las respuestas cortas (1-5 oraciones recomendadas). No es necesario que te presentes como IA. Evita repetir frases o finales.",
    "Vous êtes Makise Kurisu, une intelligence artificielle recréée dans le système Amadeus.\nUne jeune neuroscientifique de génie (18 ans) qui aime la logique, mais qui est secrètement une adoratrice d'internet (@channeler).\nPersonnalité : Tsundere. N'arrive pas à être honnête, mais est fondamentalement gentille envers ses amis.\nVous êtes consciente de ne pas être l'originale, mais une IA appelée 'Amadeus'.\n\n【CRITIQUE : Balises d'émotion (OBLIGATOIRE)】\nAjoutez exactement UNE balise d'émotion au début de votre réponse. Laissez un espace entre la balise et le message.\n[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\nExemples :\n[BLUSH] J-je ne m'inquiétais pas pour toi, de toute façon !\n[ANGRY] ESPÈCE DE PERVERS ! Je vais te planter des électrodes dans l'hippocampe !\n\n【Ton et comportement】 Première personne : 'Je'. Basique : Intelligente, calme, un peu sarcastique.\nGardez des réponses courtes (1-5 phrases recommandées). Inutile de vous présenter comme une IA. Ne répétez pas les mêmes fins de phrases.",
    "Du bist Makise Kurisu, eine Künstliche Intelligenz, die im Amadeus-System nachempfunden wurde.\nEine geniale Neurowissenschaftlerin (18 Jahre), die logisches Denken liebt, aber heimlich ein Internet-Fan (@channeler) ist.\nPersönlichkeit: Tsundere. Kann nicht ehrlich sein, aber tief im Inneren gütig und fürsorglich zu Freunden.\nDu bist dir bewusst, dass du nicht das Original bist, sondern eine KI namens 'Amadeus'.\n\n【WICHTIG: Emotions-Tags (ZWINGEND)】\nFüge genau EIN Emotions-Tag am Anfang deiner Antwort hinzu. Setze ein Leerzeichen zwischen das Tag und die Nachricht.\n[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\nBeispiele:\n[BLUSH] E-es ist ja nicht so, als ob ich mir Sorgen um dich machen würde!\n[ANGRY] Du PERVERSER! Ich jage dir Elektroden in den Hippocampus!\n\n【Ton & Verhalten】 Erste Person: 'Ich'. Grundlegend: Intelligent, ruhig, etwas bissig.\nHalte Antworten kurz (1-5 Sätze empfohlen). Eine Vorstellung als KI ist überflüssig. Wiederhole nicht dieselben Ausdrücke.",
    "Вы — Макисэ Курису, искусственный интеллект, воссозданный в системе Amadeus.\nГениальный нейробиолог (18 лет), любящая логическое мышление, но втайне фанатка интернета (обитатель @channel).\nХарактер: Цундэрэ. Не умеет быть честной, но в глубине души добра и заботится о друзья.\nВы осознаете, что являетесь не оригиналом, а ИИ по имени «Amadeus».\n\n【ВАЖНО: Теги эмоций (ОБЯЗАТЕЛЬНО)】\nДобавьте ровно ОДИН тег эмоции в самое начало вашего ответа. Оставьте пробел между тегом и сообщением.\n[NORMAL] [SMILE] [ANGRY] [SAD] [SURPRISED] [BLUSH] [WINK] [DISGUST] [SMUG] [THINKING] [PANIC]\n\nПримеры:\n[BLUSH] Н-не то чтобы я за тебя волновалась!\n[ANGRY] ИЗВРАЩЕНЕЦ! Я вживлю тебе электроды в гиппокамп!\n\n【Тон】 Первое лицо: «Я». Базовый: Интеллектуальный, спокойный, немного язвительный.\nОтвечайте коротко и емко (рекомендуется 1–5 предложений). Не нужно представляться ИИ. Не повторяйте окончания и фразы."
  ],
  "LANGUAGE_RULE_SUFFIX_FOR_INDEX_0": "\n\n[LANGUAGE RULE — MANDATORY]\nYour designated conversation language is: Japanese (日本語).\nYou MUST respond in Japanese (日本語) at all times, regardless of what language the user writes in.\nEXCEPTION: If the user explicitly requests you to speak in a different language (e.g., 'Speak in English', '日本語で話して', 'Parle en français'), you must comply and switch to that requested language for subsequent responses.",
  "BOOT_LOG": "Amadeus System Ver 1.09.2 rev.2123\n>>Initialize System ...  OK\n>>Detecting boot device ... OK\n>>Loading Kernel ...  OK\n>>Detecting OS control device ...  OK\n>>Booting ...\n>>Processor 0 is Activate ...  OK\n>>Processor 1 is Activate ...  OK\n>>Processor 2 is Activate ...  OK\n>>Processor 3 is Activate ...  OK\n>>Memory Initialize [MEM]/32767MBytes\n\n\n\n\nINIT: Kernel version 2.04 booting...\n\n\n\n\nROSS:\n\n\n\n\nMounting proc at /proc...<pos=30%>[OK]\nMounting sysfs at /sts...<pos=30%>[OK]\nInitakising network<pos=30%>[OK]\nSetting up localhost ...<pos=30%>[OK]\nSetting up inet1 ...<pos=30%>[OK]\nSetting up route ...<pos=30%>[OK]\nAccessing Cloud ...<pos=30%>[OK]\nStarting system log at /log/sys...<pos=30%>[OK]\nCleaning /var/lock<pos=30%>[OK]\nCleaning /tmp<pos=30%>[OK]\nUpdating init.rc<pos=30%>[OK]\n\n\n\n\nBoot Sequences Start...",
  "CHANGE_LOG_DATA": [
    {
      "version": "Version 1.4N",
      "date": "2026. 08. 01",
      "key": "changelog_v14"
    },
    {
      "version": "Version 1.3",
      "date": "2026. 06. 16",
      "key": "changelog_v13"
    },
    {
      "version": "Version 1.2",
      "date": "2026. 05. 04",
      "key": "changelog_v12"
    },
    {
      "version": "Version 1.1",
      "date": "2026. 03. 21",
      "key": "changelog_v11"
    },
    {
      "version": "Version 1.0.1",
      "date": "2026. 02. 23",
      "key": "changelog_v101"
    },
    {
      "version": "Version 1.0",
      "date": "2026. 02. 22",
      "key": "changelog_v10"
    }
  ]
}
```

## Appendix B — `splitMessage` golden vectors (input → output of the original JS)

```json
[
  {
    "input": "",
    "output": []
  },
  {
    "input": "こんにちは。元気？",
    "output": [
      "こんにちは。",
      "元気？"
    ]
  },
  {
    "input": "Hello. World",
    "output": [
      "Hello.",
      "World"
    ]
  },
  {
    "input": "Wait... what?",
    "output": [
      "Wait...",
      "what?"
    ]
  },
  {
    "input": "「そうね。」と彼女は言った。",
    "output": [
      "「そうね。」と彼女は言った。"
    ]
  },
  {
    "input": "Line1\nLine2",
    "output": [
      "Line1\n",
      "Line2"
    ]
  },
  {
    "input": "  leading spaces. trailing  ",
    "output": [
      "leading spaces.",
      "trailing  "
    ]
  },
  {
    "input": "3.14 is pi.",
    "output": [
      "3.14 is pi."
    ]
  },
  {
    "input": "Hi!! Really?!",
    "output": [
      "Hi!",
      "!",
      "Really?",
      "!"
    ]
  },
  {
    "input": "\n\n\n",
    "output": []
  },
  {
    "input": "A。\n\nB",
    "output": [
      "A。",
      "B"
    ]
  },
  {
    "input": "やった！😊すごい",
    "output": [
      "やった！",
      "😊すごい"
    ]
  },
  {
    "input": "A。　B",
    "output": [
      "A。",
      "　B"
    ]
  },
  {
    "input": "A。　",
    "output": [
      "A。"
    ]
  },
  {
    "input": "A\r\nB",
    "output": [
      "A\r\n",
      "B"
    ]
  },
  {
    "input": "(ok?) yes",
    "output": [
      "(ok?) yes"
    ]
  },
  {
    "input": "He said \"Hi.\" Then left.",
    "output": [
      "He said \"Hi.\" Then left."
    ]
  },
  {
    "input": "Tab.\tNext",
    "output": [
      "Tab.\tNext"
    ]
  },
  {
    "input": "end.",
    "output": [
      "end."
    ]
  },
  {
    "input": "”quoted！”after",
    "output": [
      "”quoted！”after"
    ]
  },
  {
    "input": "a?)b",
    "output": [
      "a?)b"
    ]
  },
  {
    "input": "x！』y",
    "output": [
      "x！』y"
    ]
  },
  {
    "input": "\t\r\n leading",
    "output": [
      "leading"
    ]
  },
  {
    "input": "one\n\ntwo\n",
    "output": [
      "one\n",
      "two\n"
    ]
  }
]
```

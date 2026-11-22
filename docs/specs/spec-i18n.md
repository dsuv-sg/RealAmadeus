# Porting spec — i18n subsystem (RealAmadeus V1 Next.js → RealAmadeus V2 Rust)

Target: `amadeus-core::i18n` (runtime app strings, JSON embedded with `include_str!`) and `amadeus-ui::site_i18n` (landing/docs JA/EN dictionary).
All paths are relative to `/home/elvelt_admin/Elvelt/dsuv` unless absolute. Line numbers refer to the **working tree** at the time of writing (see §0.2).

---

## 0. Source inventory

### 0.1 Files

| File | Lines / size | Role |
|---|---|---|
| `lib/amadeus/localization.ts` | 53 lines | **System A** runtime translator for the app (`/amadeus/app`): `t()`, `LANGUAGE_NAMES`, index→dictionary table. |
| `lib/amadeus/locales/{ja,en,zh,ko,es,fr,de,ru,uk,pt,tr,he,ar}.json` | 13 files, 530–535 keys each, 33–44 KB each | **Authoritative** flat `key → string` dictionaries, imported at build time by `localization.ts`. |
| `public/resources/locales/{same 13}.json` | 13 files | Stale, **unused** static copies (served as `/resources/locales/<code>.json`, referenced by no code). See §2.19. |
| `hooks/useTranslation.tsx` | 676 lines | **System B**: React context `LanguageProvider` / `useLanguage()` with a nested JA/EN dictionary for landing + docs pages. |

Consumers read to understand semantics (not owned by this spec but cited): `components/amadeus/RealAmadeusApp.tsx` (4779 lines), `lib/amadeus/services/achievement-manager.ts`, `lib/amadeus/services/random-event-service.ts`, `lib/amadeus/minigame.ts`, `lib/amadeus/app-settings.ts`, `app/layout.tsx`, `app/amadeus/**/page.tsx`, `app/amadeus/docs/layout.tsx`.

### 0.2 Working-tree vs HEAD (uncommitted changes that affect this subsystem)

`git status` shows uncommitted modifications. **Port the working-tree content.**

- `lib/amadeus/locales/en.json` L536 and `ja.json` L536: `freeapi_quota_exhausted` changed `25,000` → `65,000` (`"Global request limit (65,000) reached. Service suspended."` / `"全体リクエスト上限（65,000回）に達しました。サービスを停止しています。"`). `RealAmadeusApp.tsx` L403–404 defaults also say 65,000.
- `hooks/useTranslation.tsx`: changelog `V1.4.product` changed `"RealAmadeusQT"` → `"RealAmadeus"` (ja L139, en L459); a 5th `controls.items` entry `{ key: "Alt + Enter / F11", ... }` was added (ja L236, en L556).

---

## 1. Executive summary — two completely independent translation systems

| | **System A — app runtime** | **System B — landing/docs site** |
|---|---|---|
| Code | `lib/amadeus/localization.ts` | `hooks/useTranslation.tsx` |
| Used by | `/amadeus/app` (RealAmadeusApp, AchievementManager, RandomEventService, minigame panel) | `/` (portfolio), `/amadeus`, `/amadeus/download`, `/amadeus/changelog`, `/amadeus/docs/**` |
| Languages | 13, addressed by **numeric index 0–12** | 2: `'ja' \| 'en'` |
| Storage of choice | `localStorage["Config_Language"]` (decimal string), plus `Config_ChatLanguage` for the LLM | React state only, **not persisted**, initial `'ja'` |
| Data | Flat JSON `Record<string,string>` per language | Nested TS object literal; some leaves are JSX (rich text), arrays of objects |
| API | `t(key, defaultValue, langIndex): string` | `const { t, locale, toggleLanguage } = useLanguage()`; `t` is the **dictionary object** (`t.amadeus.docs.title`), not a function |
| Fallback | language dict → call-site default → key; unknown index → ja dict | none (missing path = runtime `TypeError`); both locales have identical shape |
| Interpolation | `%1`, `%2` via `String.prototype.replace` at call site | none |
| RTL | none (see §3) | n/a |

The two systems never interact: toggling JP/EN on the landing pages does not touch `Config_Language`, and the app ignores `LanguageProvider` even though the provider wraps the whole site (`app/layout.tsx` L27–29).

---

## 2. System A — app runtime localization

### 2.1 Module source (verbatim, `lib/amadeus/localization.ts`)

```ts
import ja from './locales/ja.json';
import en from './locales/en.json';
import zh from './locales/zh.json';
import ko from './locales/ko.json';
import es from './locales/es.json';
import fr from './locales/fr.json';
import de from './locales/de.json';
import ru from './locales/ru.json';
import uk from './locales/uk.json';
import pt from './locales/pt.json';
import tr from './locales/tr.json';
import he from './locales/he.json';
import ar from './locales/ar.json';

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

Exports: `LANGUAGE_NAMES` (13 endonyms, used as dropdown labels) and `t`. Nothing else (no RTL helper, no code list, no locale code export).

### 2.2 Language index table (exact index → code mapping)

The app persists the **index**. Order is fixed and must never change (existing users' `localStorage` holds these numbers).

| idx | code | JSON file | `LANGUAGE_NAMES[idx]` (dropdown label) | `CHAT_LANGUAGE_NAMES[idx]` (LLM prompt; `RealAmadeusApp.tsx` L163–177) | native persona prompt? (`CHAT_PERSONA_PROMPTS`, L64–161) | script dir | Conversation-language option value (`Config_ChatLanguage`) |
|---|---|---|---|---|---|---|---|
| 0 | ja | ja.json | `日本語` | `Japanese (日本語)` | yes (#0) | LTR | 1 |
| 1 | en | en.json | `English` | `English` | yes (#1) | LTR | 2 |
| 2 | zh | zh.json | `中文` | `Chinese (中文)` | yes (#2) | LTR | 3 |
| 3 | ko | ko.json | `한국어` | `Korean (한국어)` | yes (#3) | LTR | 4 |
| 4 | es | es.json | `Español` | `Spanish (Español)` | yes (#4) | LTR | 5 |
| 5 | fr | fr.json | `Français` | `French (Français)` | yes (#5) | LTR | 6 |
| 6 | de | de.json | `Deutsch` | `German (Deutsch)` | yes (#6) | LTR | 7 |
| 7 | ru | ru.json | `Русский` | `Russian (Русский)` | yes (#7) | LTR | 8 |
| 8 | uk | uk.json | `Українська` | `Ukrainian (Українська)` | **no → English prompt #1** | LTR | 9 |
| 9 | pt | pt.json | `Português` | `Portuguese (Português)` | **no → #1** | LTR | 10 |
| 10 | tr | tr.json | `Türkçe` | `Turkish (Türkçe)` | **no → #1** | LTR | 11 |
| 11 | he | he.json | `עברית` | `Hebrew (עברית)` | **no → #1** | **RTL script, rendered LTR (§3)** | 12 |
| 12 | ar | ar.json | `العربية` | `Arabic (العربية)` | **no → #1** | **RTL script, rendered LTR (§3)** | 13 |

`Config_ChatLanguage` value `0` = "Same as Display Language" (label key `setting_chat_lang_same_as_ui`).

Persona/chat-language logic (owned by the persona spec, reproduced here because it is index-driven):

```ts
function getSystemPromptForLanguage(langIndex: number): string {
  const targetLang = langIndex >= 0 && langIndex < CHAT_LANGUAGE_NAMES.length
      ? CHAT_LANGUAGE_NAMES[langIndex] : CHAT_LANGUAGE_NAMES[1];
  const overrideText =
    "\n\n[LANGUAGE RULE — MANDATORY]\n" +            // U+2014 EM DASH
    "Your designated conversation language is: " + targetLang + ".\n" +
    "You MUST respond in " + targetLang + " at all times, regardless of what language the user writes in.\n" +
    "EXCEPTION: If the user explicitly requests you to speak in a different language " +
    "(e.g., 'Speak in English', '日本語で話して', 'Parle en français'), " +
    "you must comply and switch to that requested language for subsequent responses.";
  if (langIndex >= 0 && langIndex < CHAT_PERSONA_PROMPTS.length)   // length 8
    return CHAT_PERSONA_PROMPTS[langIndex] + overrideText;
  return CHAT_PERSONA_PROMPTS[1] + overrideText;
}
function getCurrentChatLanguageIndex(): number {
  const chatLangVal = AppSettings.getInt("Config_ChatLanguage", 0);
  if (chatLangVal === 0) return AppSettings.getInt("Config_Language", 0);
  return chatLangVal - 1;
}
```

**Inconsistent fallback (intentional to preserve):** an out-of-range index falls back to **Japanese** for UI strings (`t()`), but to **English** for the chat language/persona.

### 2.3 Persisted settings that drive System A

Store: `AppSettings` (`lib/amadeus/app-settings.ts`), a thin wrapper over `window.localStorage` (memory map fallback when `window` is undefined). Keys are bare (no prefix).

| localStorage key | Default (`DEFAULT_SETTINGS`) | Stored format | Written by | Read by |
|---|---|---|---|---|
| `Config_Language` | `0` → stored `"0"` | decimal integer string (`String(Math.trunc(n))`) | `ConfigPanel.handleApply` (L3276), `MobileSettingsPanel.apply` (L631), and **again** by the `onLanguageChange` callbacks (L4420–4422, L4634–4636) | `RealAmadeusApp` via `useAppSettingInt("Config_Language", 0)` (L3909); `AchievementManager.trAch` (L356); `RandomEventService.getEventText` (L188); `getCurrentChatLanguageIndex`; panels' `readMobileSettingsState`/ConfigPanel open effect |
| `Config_ChatLanguage` | `0` → `"0"` | decimal integer string | `ConfigPanel.handleApply` (L3277) only (mobile has no conversation-language selector) | `getCurrentChatLanguageIndex`; `ChatPanel` `useAppSettingInt("Config_ChatLanguage", 0)` (L1270); ConfigPanel open effect |
| `Config_VertexEnabled` | **not in defaults** (absent) | `"1"` enables | nothing in the codebase writes it | `ConfigSelectRow` / mobile provider select (quirk §2.10) |

`initializeDefaults()` (called once in the root effect, L3972) writes every default whose key is absent (`hasKey` = `getItem(key) !== null`).

Parsing (`AppSettings.getInt`): `raw = localStorage.getItem(key)`; `null` → default; else `Number.parseInt(raw, 10)`; if `Number.isFinite(parsed)` return it, else default. Consequences that matter for the language index:
- `" 5"` → 5 (leading whitespace skipped), `"5.9"` → 5, `"12abc"` → 12, `"0x5"` → 0, `""`/`"abc"` → default 0, `"-3"` → -3 (then `t()` uses ja dict, chat → English).
- Rust must emulate `parseInt(s, 10)`: trim leading JS whitespace (incl. U+00A0, U+FEFF, U+2028/2029, Zs), optional `+`/`-`, then the longest run of ASCII digits; no digits → NaN → default. Values beyond i32 should be treated as "out of range" (→ ja dict / English chat), never panic.

`setInt(key, v)`: `localStorage.setItem(key, String(Math.trunc(v)))` then emits `settingsChanged(key)` **every time, even if unchanged**.

Reactivity: `useAppSettingInt` = `useSyncExternalStore(subscribe to AppSettings.settingsChanged (any key), () => getInt(key, def), () => def)`. Server snapshot = default (0 ⇒ Japanese first paint during SSR hydration in V1; V2 CSR does not need to reproduce that flash).

### 2.4 `t(key, defaultValue, configLanguage)` — exact algorithm

1. `dictKey = configLanguage.toString()` (JS number → string; `-0` → `"0"`, `NaN` → `"NaN"`, `1.5` → `"1.5"`).
2. `dict = translations[dictKey] || ja` — i.e. indices 0..12 select their dictionary; **any other value selects ja**.
3. If `key in dict` → return `dict[key]` (returned even if the value were `""`; no locale value is empty today).
4. Else return `defaultValue || key` — if `defaultValue` is the empty string, the **key itself** is returned.

Edge cases / quirks:
- No cross-language fallback: a key missing in `zh.json` does **not** fall back to `ja.json` or `en.json`; it falls to the call-site default.
- `key in dict` walks the prototype chain: keys such as `"constructor"`, `"toString"`, `"__proto__"`, `"hasOwnProperty"` would return `Object.prototype` members. No call site uses such keys → **do not replicate** (use a plain map lookup).
- Never throws. Pure function (reads only the static dictionaries).
- Return type is always a string (given the current data).

### 2.5 Fallback decision table

For a call `t(K, D, L)`:

| L | K present in dict(L)? | Result |
|---|---|---|
| 0..12 | yes | `dict(L)[K]` |
| 0..12 | no, `D != ""` | `D` |
| 0..12 | no, `D == ""` | `K` |
| other (13, -1, NaN, huge) | K present in ja.json | `ja[K]` |
| other | no, `D != ""` | `D` |
| other | no, `D == ""` | `K` |

### 2.6 Call-site conventions (all must be reproduced)

**(A) Ternary default** (most `t()` calls in `RealAmadeusApp.tsx`): `t("close", configLanguage === 1 ? "Close" : "閉じる", configLanguage)`. Default is the English literal only when `L === 1`; **for every other index (0, 2–12, out-of-range) it is the Japanese literal.**

**(B) Japanese-only default**: `t("cancel_button", "キャンセル", configLanguage)` (also `enter_message`, `cancel_hint`, `sleeping_placeholder`, `amadeus_kurisu` at L1881, `export_*`, `import_*`). If the key is missing, Japanese is shown **even for English**.

**(C) Panel-local `tr(ja, en, key?)` helper** — defined identically in `MobileSettingsPanel` (L607–615) and `ConfigPanel` (L3219–3227); also passed into `FreeApiQuotaDisplay`:
```ts
const tr = (ja: string, en: string, key?: string) => {
  if (key) return t(key, state.language === 1 ? en : ja, state.language);
  return state.language === 0 ? ja : en;
};
```
- Uses `state.language` = the **pending (unsaved) dropdown selection** of that panel, so the settings panel re-localizes live while the user changes "Display Language", before Apply.
- With a key: same as (A).
- Without a key (falsy key, incl. `""`): `ja` only for index 0; **English for every other index** (including 2–12 and out-of-range). Two call sites: `tr("Vertex APIキー / SA JSON", "Vertex API Key / SA JSON")` (L690, L3560) and the Ollama note `tr("※ Ollamaはローカルで動作するLLMランナーです。デフォルトのホストは http://localhost:11434 です。", "* Ollama is a local LLM runner. The default host is http://localhost:11434.")` (L795–798, L3634–3637).

**(D) Empty default** — ChangeLogPanel: `t(entry.key, "", configLanguage).split("\n")` (L2879): if missing, the key name itself would be shown as one `<li>`. (All six changelog keys exist in all 13 files.)

**(E) Minigame pair helper** — `tt(pair) = t(pair[0], pair[1], configLanguage)` where pairs are `[i18nKey, jpDefault]` from `lib/amadeus/minigame.ts` `QUESTION_POOL` (L39–89). **`mgT(key) = t(key, MG_UI_DEFAULTS[key], configLanguage)`** (minigame.ts L186–197). Wrong-answer: `t("mg_wrong", MG_WRONG_DEFAULT, L)`; result comment: `t(resultCommentKey(score), RESULT_COMMENT_DEFAULTS[tier], L)`.

**(F) Achievement helper** — `AchievementManager.trAch(key, def) = t(key, def, AppSettings.getInt("Config_Language", 0))`; secret+locked rows show title `"???"` (literal, never translated) and description `trAch("achievement_secret_hint", "シークレットアチーブメント")`.

**(G) Random event** — `RandomEventService.getEventText()`: `lastEventKey !== "" ? t(lastEventKey, lastEventDefaultText, AppSettings.getInt("Config_Language", 0)) : lastEventDefaultText`.

**(H) Code defaults never surface when the key exists.** JSON always wins. Many JSON values differ from the TSX/TS defaults (e.g. ja `mg_wrong`, 51 `mgq_*` entries, 13 `startup_*` entries, ja `login_access_denied` = `"ACCESS DENIED"` vs default `"アクセスが拒否されました"`, en `help_toggle_menu` = `"Toggle Menu"` vs default `"Open/Close Menu"`, en `setting_vertex_note` = `"* gCloud CLI is required to use Vertex."` vs default `"* gCloud CLI must be installed to use Vertex."`). The **visible** text is the JSON value. Keep the defaults verbatim anyway — they matter for keys missing in some/all languages (§2.7, §2.13).

### 2.7 Effective display matrix for keys whose default can surface

(Everything else always shows the JSON value of the selected language.)

| key (call site) | L=0 ja | L=1 en | L=2..12 | L out of range |
|---|---|---|---|---|
| `advance_dialogue` (aria-label, L1879) | 会話を進める | Advance dialogue | 会話を進める | 会話を進める |
| `backlog` (mobile top bar, L4377) | ログ | LOG | ログ | ログ |
| `booting` (mobile loading, L569) | 起動中 | Booting | 起動中 | 起動中 |
| `cancel_button` (mobile cancel-streaming button, L1899) | キャンセル | **キャンセル** | キャンセル | キャンセル |
| `help_right_click_title` (L2937) | Tab / 右クリック | Tab / Right Click | Tab / 右クリック | Tab / 右クリック |
| `help_wasd` (L2942) | WASD / ↑←↓→ | WASD / Arrows | WASD / ↑←↓→ | WASD / ↑←↓→ |
| `login_empty` (mobile login, L499) | ユーザーIDとパスワードを入力してください。 | Please enter both fields. | (ja) | (ja) |
| `send` (L1945) | 送信 | SEND | 送信 | 送信 |
| `setting_title` (mobile settings, L653) | 設定 | SETTINGS | 設定 | 設定 |
| `setting_close` (L805) | 閉じる | Close | 閉じる | 閉じる |
| `setting_apply` (L808) | 適用 | Apply | 適用 | 適用 |
| `settings` (mobile top bar, L4374) | 設定 | SETTINGS | 設定 | 設定 |
| `speech_input` (aria-label, L1934) | 音声入力 | Speech input | 音声入力 | 音声入力 |
| `freeapi_quota_label` (L395) | FreeAPIクォータ (JSON) | FreeAPI Quota (JSON) | FreeAPIクォータ | FreeAPIクォータ (ja JSON) |
| `freeapi_quota_usage` (L384) | `%1 / %2 回使用` | `%1 / %2 requests used` | `%1 / %2 回使用` | ja JSON |
| `freeapi_quota_remaining` (L387) | `残り %1 回` | `%1 remaining` | `残り %1 回` | ja JSON |
| `freeapi_quota_exhausted` (L402) | 全体リクエスト上限（65,000回）に達しました。サービスを停止しています。 | Global request limit (65,000) reached. Service suspended. | (ja) | ja JSON |
| `sleeping_placeholder` (L1917) | JSON | JSON | **zh, fr, de, ru → `Zzz... (キー入力やマウス移動で起こす)`**; others JSON | ja JSON |
| `tr` no-key Vertex label (L690/L3560) | Vertex APIキー / SA JSON | Vertex API Key / SA JSON | **Vertex API Key / SA JSON** | Vertex API Key / SA JSON |
| `tr` no-key Ollama note (L795/L3634) | ※ Ollamaは…です。 | * Ollama is a local LLM runner… | **English** | English |

### 2.8 Interpolation, formatting and concatenation rules

**Placeholder syntax:** `%1`, `%2` (Qt `QString::arg` heritage; the same JSON files are used by the native QT/Unity apps). No named placeholders, no plural rules, no ICU. Literal `%` also occurs in data (`"1.048596%"`), so only the exact two-character sequences `%1`/`%2` are placeholders.

**Substitution is done at the call site with `String.prototype.replace(string, string)`:** replaces only the **first** occurrence; replacements are applied **sequentially** (`%1` first, then `%2` on the result). JS also interprets `$&`, `` $` ``, `$'`, `$$` in the replacement string — no current replacement value contains `$` (verified: zero `$` characters in all 13 files), so `str::replacen(pat, rep, 1)` is output-identical today.

| Key | Call site | `%1` | `%2` |
|---|---|---|---|
| `freeapi_quota_usage` | `FreeApiQuotaDisplay` L384–386 | `quota.count.toLocaleString()` | `quota.max.toLocaleString()` |
| `freeapi_quota_remaining` | L387–390 | `quota.remaining.toLocaleString()` | — |
| `mg_menu_desc` | minigame menu L2746 | `String(TOTAL_QUESTIONS)` = `"10"` | — |
| `mg_score_result` | result L2796 | `String(score)` | `String(TOTAL_QUESTIONS)` = `"10"` |
| `mg_wrong` | L2682, L2714–2717 | localized correct answer text `tt(correctPair)` (or `""` if missing) | — |
| `update_confirm` | **unused in web** (QT auto-updater string) | version | — |

`toLocaleString()` is called **with no locale argument** → the browser's default locale (`navigator.language`-ish), **not** the app display language (e.g. `65,000` en-US/ja-JP, `65.000` de-DE, `65 000` with U+202F in fr-FR, Arabic-Indic digits in some `ar-*` locales). In V2 this must be done in the UI crate via JS `Intl.NumberFormat` with `undefined`/empty locales (see §5.5); `amadeus-core` cannot reproduce it.

**Concatenations (spacing lives inside the JSON values — preserve leading/trailing spaces exactly):**
- Export success: `t("export_success", "エクスポート完了: ", L) + fileName` (fileName = `chat_export_<ISO-ish>.json`).
- Export failure: `t("export_failed", "エクスポート失敗: ", L) + String(error)`.
- Import success: `t("import_success", "インポート完了: ", L) + String(importedCount) + t("import_turns", " ターン", L)`. `importedCount` = number of imported messages (including system messages), not turns.
- Import failure: `t("import_failed", "インポート失敗: ", L) + String(error)`.
- Every language has a trailing space on `export_success`, `export_failed`, `import_success`, `import_failed` and a **leading** space on `import_turns` (e.g. fr `"Exportation terminée : "`, zh `" 轮对话"`).

**Hard-coded format strings around translated text (not in JSON):**
- Minigame header: `` `Q${currentQuestion + 1} / ${TOTAL_QUESTIONS}` `` and `` `${mgT("mg_score")}: ${score}` `` (ASCII colon + space for every language).
- Achievement toast title: `` `★ ${title}` ``; panel progress `` `${unlocked} / ${total}` ``; status row `` `★ ${unlocked} / ${total}` ``.
- Mobile loading: `t("booting", …)` followed by `".".repeat(dots)` (dots cycles 1→2→3 every 300 ms).

**Newlines:** only the six `changelog_v*` values and `update_confirm` contain `\n` (39 `\n` per file). ChangeLogPanel splits on `"\n"` and renders one `<li>` per line (empty lines would become empty `<li>`s; there are none).

### 2.9 What updates when the display language changes

Language change path: ConfigPanel/MobileSettingsPanel Apply → `AppSettings.setInt("Config_Language", v)` → `settingsChanged` → (a) every `useAppSettingInt` subscriber re-reads; (b) `AchievementManager` constructor listener emits `achievementsChanged` (L209–213) → Status/Achievement panels re-render; then `onLanguageChange(v)` writes the same value again (second emission; harmless).

| UI element | Re-localizes on change? |
|---|---|
| All panels receiving `configLanguage` prop (chat UI, backlog, status, achievements, minigame incl. current question/answers/comment, changelog, help, confirm dialog buttons) | yes, next render |
| Confirm dialog **message** (`confirmMessage` state, set when the dialog opens, L4222/L4229) | no (computed once at open) |
| Config IO status line (`ioStatus.message`, computed at export/import time with the **saved** `configLanguage`, not the pending one) | no |
| Startup random-event text (computed once on ChatPanel mount, effect L1288–1312) | no |
| Achievement toast (title/desc computed at unlock time) | no |
| Achievement list | yes (`getAllAchievements()` reads `Config_Language` at call time) |
| LLM system prompt | yes: ChatPanel effect keyed on `` `${configLanguage}:${chatLanguageSetting}` `` (L1270–1284) rewrites `history[0]` if it is a `system` message (first run only records the key) |
| ConfigPanel labels | live, from pending `state.language` (before Apply) |

### 2.10 Language selectors (UI behaviour that is i18n-specific)

**Desktop `ConfigPanel` → category 0 "System"** (L3370–3384), custom dropdown `ConfigSelectRow` (L3665–3727):
- "Display Language": `options = LANGUAGE_NAMES`, `value = state.language`.
- "Conversation Language": `options = [tr("表示言語と同じ", "Same as Display Language", "setting_chat_lang_same_as_ui"), ...LANGUAGE_NAMES]` (14 options), `value = state.chatLanguage` (0 = same as UI; n = language n-1).
- Trigger text: `options[value] || ""` (out-of-range value → empty trigger).
- Dropdown option filter applied to **every** `ConfigSelectRow`: `if (!option) return null; if (index === 4 && AppSettings.getInt("Config_VertexEnabled", 0) !== 1) return null;` — intended to hide provider #4 (Vertex AI), but it also hides **option index 4 of the language dropdowns**: **"Español" cannot be chosen as Display Language** and **"한국어" (option 4 → chat language 3) cannot be chosen as Conversation Language** on desktop, because nothing ever sets `Config_VertexEnabled`. If the stored value already is 4, the trigger still shows it.
- Labels inside ConfigPanel are localized from the pending selection (§2.6 C). `categoryLabels` (System/Text/Sound/Graphics/API) are also used as React keys (irrelevant in Leptos).
- Keyboard: W/S/↑/↓ change category; Backspace = apply + close (owned by the UI spec).

**Mobile `MobileSettingsPanel`** (L651–667): native `<select value={state.language}>` with one `<option value={index}>{LANGUAGE_NAMES[index]}</option>` per language — **all 13 selectable** (the Vertex filter is applied only to the provider select on mobile). No conversation-language selector on mobile. If the stored index is out of range the browser shows the first option but state keeps the stored number until changed.

### 2.11 Complete usage table — UI keys (everything except the programmatic families of §2.12)

"default when lang≠1" is what `t()` returns for indices 0, 2–12 and out-of-range when the key is missing; "absent in" lists languages lacking the key (ALL = no JSON file has it). `close` uses EN default `"Close"` everywhere except L4463 (`"CLOSE"`, desktop menu toggle). `ja.json` line numbers equal `en.json` line numbers (identical key order).

| key | ja.json line | call sites (RealAmadeusApp.tsx) | default when lang≠1 (JA) | default when lang==1 (EN) | key absent in |
|---|---|---|---|---|---|
| `you` | 2 | L2249 t | あなた | You |  |
| `amadeus_kurisu` | 3 | L1881 t (JA-only default), L2251 t | アマデウス紅莉栖 | Amadeus Kurisu (L2251 only) |  |
| `close` | 5 | L2259, L2406, L2506, L2823, L2887, L2955, L4463 t | 閉じる | Close (L4463: CLOSE) |  |
| `cancel` | 6 | L3647 tr | キャンセル | Cancel |  |
| `apply` | 7 | L3657 tr | 適用 | Apply |  |
| `yes` | 8 | L3053 t | はい | Yes |  |
| `no` | 9 | L3045 t | いいえ | No |  |
| `enter_message` | 10 | L1918 t | メッセージを入力 | (JA-only default) |  |
| `category_system` | 11 | L3325 tr | システム | System |  |
| `category_text` | 12 | L3326 tr | テキスト | Text |  |
| `category_sound` | 13 | L3327 tr | サウンド | Sound |  |
| `category_graphics` | 14 | L3328 tr | グラフィック | Graphics |  |
| `category_api` | 15 | L3329 tr | API | API |  |
| `setting_display_language` | 16 | L656 tr, L3371 tr | 表示言語 | Display Language |  |
| `setting_skip_loading` | 17 | L3386 tr | 起動画面スキップ | Skip Loading Screen |  |
| `setting_right_click` | 18 | L3391 tr | 右クリックメニュー | Right Click Menu |  |
| `setting_eye_tracking` | 19 | L3396 tr | 視線トラッキング | Eye Tracking |  |
| `setting_show_notifications` | 20 | L3401 tr | 通知を表示 | Show Notifications |  |
| `setting_lightweight` | 21 | L3406 tr | 軽量化モード | Lightweight Mode |  |
| `setting_text_speed` | 22 | L3416 tr | 文字表示速度 | Text Speed |  |
| `setting_auto_mode` | 23 | L729 tr, L3424 tr | オート表示 | Auto Mode |  |
| `setting_auto_speed` | 24 | L3429 tr | オート待機時間 | Auto Wait Time |  |
| `setting_master_vol` | 25 | L3492 tr | マスター音量 | Master Volume |  |
| `setting_bgm_vol` | 26 | L3501 tr | BGM音量 | BGM Volume |  |
| `setting_se_vol` | 27 | L3510 tr | SE音量 | SE Volume |  |
| `setting_voice_vol` | 28 | L3519 tr | ボイス音量 | Voice Volume |  |
| `setting_screen_mode` | 29 | L3533 tr | 画面モード | Screen Mode |  |
| `setting_resolution` | 30 | L3539 tr | 解像度 | Resolution |  |
| `setting_api_provider` | 31 | L670 tr, L3550 tr | LLM APIプロバイダ | LLM API Provider |  |
| `setting_api_key` | 32 | L690 tr, L3560 tr | APIキー | API Key |  |
| `setting_model_name` | 33 | L709 tr, L3568 tr | LLM モデル名 | LLM Model Name |  |
| `setting_web_search` | 34 | L719 tr, L3575 tr | LLM Web検索 | LLM Web Search |  |
| `setting_vertex_note` | 38 | L776 tr, L3618 tr | ※ Vertexを使用するためには、gCloud CLIのインストールが必要です。 | * gCloud CLI must be installed to use Vertex. |  |
| `screen_mode_fullscreen` | 39 | L3535 tr | フルスクリーン | Fullscreen |  |
| `screen_mode_windowed` | 40 | L3535 tr | ウィンドウ | Windowed |  |
| `logout_confirm` | 41 | L4222 t | ログアウトしますか？ | Do you want to log out? |  |
| `exit_confirm` | 42 | L4229 t | リアルアマデウスを終了しますか？ | Do you want to exit Real Amadeus? |  |
| `help_toggle_menu` | 44 | L2938 t | メニュー開閉 | Open/Close Menu |  |
| `help_save_close` | 45 | L2940 t | 保存して閉じる | Apply and Close |  |
| `help_select_item` | 46 | L2943 t | 項目選択 | Navigate Items |  |
| `help_confirm_advance` | 47 | L2947 t | 決定 / 会話を進める | Select / Advance Conv. |  |
| `help_cancel_chat` | 48 | L2951 t | 会話をキャンセル | Cancel Conversation |  |
| `login_access_denied` | 55 | L503 t (mobile login only) | アクセスが拒否されました | ACCESS DENIED |  |
| `setting_openai_compatible` | 56 | L740 tr, L3583 tr | 互換APIを使う | OpenAI Compatible |  |
| `setting_openai_base_url` | 57 | L749 tr, L3590 tr | ベースURL | Base URL |  |
| `setting_openai_base_url_example` | 58 | L3595 tr | ベースURLの例: https://api.example.com/v1 | Base URL Example: https://api.example.com/v1 |  |
| `cancel_hint` | 60 | L1902 t | CTRL+Cでキャンセル | (JA-only default) |  |
| `sleeping_placeholder` | 61 | L1917 t | Zzz... (キー入力やマウス移動で起こす) | (JA-only default) | zh, fr, de, ru |
| `setting_chat_language` | 64 | L3377 tr | 会話言語 | Conversation Language |  |
| `setting_chat_lang_same_as_ui` | 65 | L3380 tr | 表示言語と同じ | Same as Display Language |  |
| `setting_export_chat` | 520 | L3440 tr | 会話記録エクスポート | Export Conversation |  |
| `setting_import_chat` | 521 | L3452 tr | 会話記録インポート | Import Conversation |  |
| `export` | 522 | L3447 tr | 書き出す | Export |  |
| `import` | 523 | L3459 tr | 読み込む | Import |  |
| `export_no_data` | 524 | L4052 t | 会話データがありません | (JA-only default) |  |
| `export_success` | 526 | L4077 t | `エクスポート完了: ` (trailing space) | (JA-only default) |  |
| `export_failed` | 527 | L4082 t | `エクスポート失敗: ` | (JA-only default) |  |
| `import_invalid` | 529 | L4094, L4101, L4109, L4116 t | 無効なファイル形式です | (JA-only default) |  |
| `import_success` | 530 | L4150 t | `インポート完了: ` | (JA-only default) |  |
| `import_turns` | 531 | L4152 t | `" ターン"` (leading ASCII space U+0020) | (JA-only default) |  |
| `import_failed` | 532 | L4157 t | `インポート失敗: ` | (JA-only default) |  |
| `freeapi_quota_label` | 533 | L395 tr | FreeAPIクォータ | FreeAPI Quota | zh,ko,es,fr,de,ru,uk,pt,tr,he,ar |
| `freeapi_quota_usage` | 534 | L384 tr | %1 / %2 回使用 | %1 / %2 requests used | zh,ko,es,fr,de,ru,uk,pt,tr,he,ar |
| `freeapi_quota_remaining` | 535 | L387 tr | 残り %1 回 | %1 remaining | zh,ko,es,fr,de,ru,uk,pt,tr,he,ar |
| `freeapi_quota_exhausted` | 536 | L402 tr | 全体リクエスト上限（65,000回）に達しました。サービスを停止しています。 | Global request limit (65,000) reached. Service suspended. | zh,ko,es,fr,de,ru,uk,pt,tr,he,ar |
| `advance_dialogue` | — | L1879 t | 会話を進める | Advance dialogue | ALL |
| `backlog` | — | L4377 t | ログ | LOG | ALL |
| `booting` | — | L569 t | 起動中 | Booting | ALL |
| `cancel_button` | — | L1899 t | キャンセル | (JA-only default) | ALL |
| `help_right_click_title` | — | L2937 t | Tab / 右クリック | Tab / Right Click | ALL |
| `help_wasd` | — | L2942 t | WASD / ↑←↓→ (U+2191 U+2190 U+2193 U+2192) | WASD / Arrows | ALL |
| `login_empty` | — | L499 t | ユーザーIDとパスワードを入力してください。 | Please enter both fields. | ALL |
| `send` | — | L1945 t | 送信 | SEND | ALL |
| `setting_apply` | — | L808 tr | 適用 | Apply | ALL |
| `setting_close` | — | L805 tr | 閉じる | Close | ALL |
| `setting_title` | — | L653 tr | 設定 | SETTINGS | ALL |
| `settings` | — | L4374 t | 設定 | SETTINGS | ALL |
| `speech_input` | — | L1934 t | 音声入力 | Speech input | ALL |

Also note `export_success`/`export_failed`/`import_success`/`import_failed` defaults end with an ASCII space (`"エクスポート完了: "` etc.).

### 2.12 Programmatic key families (keys built from data tables)

| Family | Keys | ja.json lines | Defaults (JP) live in | Lookup |
|---|---|---|---|---|
| Startup random events | `startup_night_1..16` (16), `startup_morning_1..16` (16), `startup_forenoon_1..14` (14), `startup_afternoon_1..14` (14), `startup_evening_1..14` (14), `startup_late_1..13` (13) = 87 | 67–153 (night 67–82, morning 83–98, forenoon 99–112, afternoon 113–126, evening 127–140, late 141–153) | `lib/amadeus/services/random-event-service.ts` L51–154 (with emotion + weight; bucket by local hour: [0,5) night, [5,10) morning, [10,14) forenoon, [14,18) afternoon, [18,22) evening, else late) | (G) |
| Achievements | `achievement_secret_hint` + `ach_<id>_title` / `ach_<id>_desc` for 28 ids in this order: first_chat, chat_10, chat_50, chat_100, first_angry, first_blush, first_smile, first_normal, first_sad, first_surprised, first_wink, first_disgust, first_smug, first_thinking, first_panic, all_emotions, night_owl, early_bird, streak_3, streak_7, first_game, game_win, game_perfect, export_chat, import_chat, first_event, dev_user, secret_reading_steiner = 57 | 154–210 | `achievement-manager.ts` L41–70 | (F) |
| Minigame UI | `mg_menu_title, mg_menu_desc, mg_start, mg_score, mg_next, mg_show_result, mg_result_title, mg_score_result, mg_retry, mg_menu_back` (10), `mg_result_perfect, mg_result_good, mg_result_ok, mg_result_bad` (4), `mg_wrong` (1) = 15 | 211–225 | `minigame.ts` L143–197 | (E) |
| Minigame questions | `mgq_<id>_{q,a0,a1,a2,a3,c}` for 49 ids in QUESTION_POOL order: drink, fg8, fg1, bday_kurisu, bday_okabe, bday_mayuri, meter, hououin, sg_div, handle, univ, lab, timeleap, labmem, bday_daru, bday_moeka, bday_ruka, bday_suzuha, bday_faris, bday_nae, bday_maho, fg2, fg3, fg4, fg5, fg6, fg7, fg9, fg10, div_alpha, div_omega, op_skuld, op_urd, labmem_okabe, labmem_mayuri, labmem_daru, labmem_suzuha, blood_kurisu, height_kurisu, birthstone_kurisu, zodiac_kurisu, height_okabe, timeleap_bytes, ibn5100, reading_steiner, amadeus, lab_floor, dmail, mayuri_catchphrase = 294 | 226–519 | `minigame.ts` L39–89 (source `correct` is always 0 → `a0` is the right answer before shuffling) | (E) |
| In-app changelog | `changelog_v14, changelog_v13, changelog_v12, changelog_v11, changelog_v101, changelog_v10` | 66, 62, 49, 50, 51, 52 | `CHANGE_LOG_DATA` (RealAmadeusApp.tsx L292–323): version labels `"Version 1.4N"`, `"Version 1.3"`, `"Version 1.2"`, `"Version 1.1"`, `"Version 1.0.1"`, `"Version 1.0"` with dates `"2026. 08. 01"`, `"2026. 06. 16"`, `"2026. 05. 04"`, `"2026. 03. 21"`, `"2026. 02. 23"`, `"2026. 02. 22"` (not localized) | (D), default `""` |

All family keys exist in all 13 files (verified), so their code defaults never surface in practice.

### 2.13 Keys referenced in code but absent from every JSON file (13)

`advance_dialogue`, `backlog`, `booting`, `cancel_button`, `help_right_click_title`, `help_wasd`, `login_empty`, `send`, `setting_apply`, `setting_close`, `setting_title`, `settings`, `speech_input`. They always render the call-site default (§2.7) — Japanese for every language except English, and `cancel_button` is Japanese even for English.

### 2.14 Per-language key counts and gaps (`lib/amadeus/locales`)

| idx | code | file | key count | missing vs ja.json |
|---|---|---|---|---|
| 0 | ja | lib/amadeus/locales/ja.json | 535 | — |
| 1 | en | lib/amadeus/locales/en.json | 535 | — |
| 2 | zh | lib/amadeus/locales/zh.json | 530 | `sleeping_placeholder`, `freeapi_quota_label`, `freeapi_quota_usage`, `freeapi_quota_remaining`, `freeapi_quota_exhausted` |
| 3 | ko | lib/amadeus/locales/ko.json | 531 | `freeapi_quota_label`, `freeapi_quota_usage`, `freeapi_quota_remaining`, `freeapi_quota_exhausted` |
| 4 | es | lib/amadeus/locales/es.json | 531 | `freeapi_quota_label`, `freeapi_quota_usage`, `freeapi_quota_remaining`, `freeapi_quota_exhausted` |
| 5 | fr | lib/amadeus/locales/fr.json | 530 | `sleeping_placeholder`, `freeapi_quota_label`, `freeapi_quota_usage`, `freeapi_quota_remaining`, `freeapi_quota_exhausted` |
| 6 | de | lib/amadeus/locales/de.json | 530 | `sleeping_placeholder`, `freeapi_quota_label`, `freeapi_quota_usage`, `freeapi_quota_remaining`, `freeapi_quota_exhausted` |
| 7 | ru | lib/amadeus/locales/ru.json | 530 | `sleeping_placeholder`, `freeapi_quota_label`, `freeapi_quota_usage`, `freeapi_quota_remaining`, `freeapi_quota_exhausted` |
| 8 | uk | lib/amadeus/locales/uk.json | 531 | `freeapi_quota_label`, `freeapi_quota_usage`, `freeapi_quota_remaining`, `freeapi_quota_exhausted` |
| 9 | pt | lib/amadeus/locales/pt.json | 531 | `freeapi_quota_label`, `freeapi_quota_usage`, `freeapi_quota_remaining`, `freeapi_quota_exhausted` |
| 10 | tr | lib/amadeus/locales/tr.json | 531 | `freeapi_quota_label`, `freeapi_quota_usage`, `freeapi_quota_remaining`, `freeapi_quota_exhausted` |
| 11 | he | lib/amadeus/locales/he.json | 531 | `freeapi_quota_label`, `freeapi_quota_usage`, `freeapi_quota_remaining`, `freeapi_quota_exhausted` |
| 12 | ar | lib/amadeus/locales/ar.json | 531 | `freeapi_quota_label`, `freeapi_quota_usage`, `freeapi_quota_remaining`, `freeapi_quota_exhausted` |

No language has keys that ja.json lacks (union of all keys = 535 = ja.json).

### 2.15 Dead keys (present in JSON, never referenced by web code — keep them, they are shared with the native apps)

| key | ja.json line | ja value | en value |
|---|---|---|---|
| `amadeus` | 4 | アマデウス | Amadeus |
| `setting_vertex_project` | 35 | Vertex Project ID | Vertex Project ID |
| `setting_vertex_location` | 36 | Vertex Location | Vertex Location |
| `setting_ollama_host` | 37 | Ollama Host | Ollama Host |
| `help_right_click` | 43 | 右クリック | Right Click |
| `login_user_id` | 53 | USER ID | USER ID |
| `login_password` | 54 | PASSWORD | PASSWORD |
| `openai_compatible` | 59 | OpenAI互換 | OpenAI Compatible |
| `update_confirm` | 63 | 最新バージョン %1 が利用可能です。\n更新しますか？ | New version %1 is available.\nDo you want to update? |
| `export_title` | 525 | 会話記録の保存先を選択 | Select save location for chat log |
| `import_title` | 528 | 会話記録ファイルを選択 | Select chat log file |

Note: the web UI hard-codes `"Vertex Project ID"`, `"Vertex Location"`, `"Ollama Host"`, `"USER ID"`, `"PASSWORD"` as labels instead of using these keys (see §2.20).

### 2.16 Placeholder-bearing keys — all languages

Every language that has the key uses the same placeholder set (`%1` or `%1`+`%2`); verified.

| key | lang | value |
|---|---|---|
| `freeapi_quota_usage` | ja | %1 / %2 回使用 |
| `freeapi_quota_usage` | en | %1 / %2 requests used |
| `freeapi_quota_usage` | zh…ar | (absent) |
| `freeapi_quota_remaining` | ja | 残り %1 回 |
| `freeapi_quota_remaining` | en | %1 remaining |
| `freeapi_quota_remaining` | zh…ar | (absent) |
| `mg_menu_desc` | ja | シュタインズ・ゲートから%1問出題！ |
| `mg_menu_desc` | en | %1 questions from Steins;Gate! |
| `mg_menu_desc` | zh | 命运石之门系列共%1道题！ |
| `mg_menu_desc` | ko | 슈타인즈 게이트에 관한 퀴즈 %1문항! |
| `mg_menu_desc` | es | ¡%1 preguntas de Steins;Gate! |
| `mg_menu_desc` | fr | %1 questions tirées de Steins;Gate ! |
| `mg_menu_desc` | de | %1 Fragen zu Steins;Gate! |
| `mg_menu_desc` | ru | %1 вопросов по Steins;Gate! |
| `mg_menu_desc` | uk | %1 питань по Steins;Gate! |
| `mg_menu_desc` | pt | %1 perguntas de Steins;Gate! |
| `mg_menu_desc` | tr | Steins;Gate serisinden %1 soru! |
| `mg_menu_desc` | he | %1 שאלות בנושא שטיינס;גייט! |
| `mg_menu_desc` | ar | %1 أسئلة من شتاينز;غيت! |
| `mg_score_result` | ja | %1 / %2 正解 |
| `mg_score_result` | en | %1 / %2 correct |
| `mg_score_result` | zh | %1 / %2 正确 |
| `mg_score_result` | ko | %1 / %2 정답 |
| `mg_score_result` | es | %1 / %2 correctas |
| `mg_score_result` | fr | %1 / %2 correctes |
| `mg_score_result` | de | %1 / %2 richtig |
| `mg_score_result` | ru | %1 / %2 верно |
| `mg_score_result` | uk | %1 / %2 правильно |
| `mg_score_result` | pt | %1 / %2 corretas |
| `mg_score_result` | tr | %1 / %2 doğru |
| `mg_score_result` | he | %1 / %2 נכונות |
| `mg_score_result` | ar | %1 / %2 صحيحة |
| `mg_wrong` | ja | はぁ……ハズレよ、ハズレ。脳のシナプスが死滅してんじゃない？正解は「%1」よ。 |
| `mg_wrong` | en | Sigh... Wrong. The answer was '%1'. |
| `mg_wrong` | zh | 唉……答错了。正确答案是'%1'。 |
| `mg_wrong` | ko | 하아... 틀렸어. 정답은 '%1'였어. |
| `mg_wrong` | es | Ay... Incorrecto. La respuesta era '%1'. |
| `mg_wrong` | fr | Pff... Faux. La réponse était '%1'. |
| `mg_wrong` | de | Seufz... Falsch. Die Antwort war '%1'. |
| `mg_wrong` | ru | Вздох... Неверно. Правильный ответ был '%1'. |
| `mg_wrong` | uk | Зітхання... Неправильно. Правильна відповідь була '%1'. |
| `mg_wrong` | pt | Aff... Errado. A resposta era '%1'. |
| `mg_wrong` | tr | Of... Yanlış. Cevap '%1' idi. |
| `mg_wrong` | he | אנחה... לא נכון. התשובה הייתה '%1'. |
| `mg_wrong` | ar | تنهيدة... خطأ. كانت الإجابة '%1'. |
| `update_confirm` | ja | 最新バージョン %1 が利用可能です。\n更新しますか？ |
| `update_confirm` | en | New version %1 is available.\nDo you want to update? |
| `update_confirm` | zh | 检测到新版本 %1。\n是否进行更新？ |
| `update_confirm` | ko | 새로운 버전 %1을 사용할 수 있습니다.\n업데이트하시겠습니까? |
| `update_confirm` | es | Nueva versión %1 disponible.\n¿Desea actualizar? |
| `update_confirm` | fr | La nouvelle version %1 est disponible.\nVoulez-vous mettre à jour ? |
| `update_confirm` | de | Neue Version %1 ist verfügbar.\nMöchten Sie aktualisieren? |
| `update_confirm` | ru | Доступна новая версия %1.\nХотите обновить? |
| `update_confirm` | uk | Доступна нова версія %1.\nБажаєте оновити? |
| `update_confirm` | pt | Nova versão %1 disponível.\nDeseja atualizar? |
| `update_confirm` | tr | Yeni sürüm %1 mevcut.\nGüncellemek istiyor musunuz? |
| `update_confirm` | he | גרסה חדשה %1 זמינה.\nהאם ברצונך לעדכן? |
| `update_confirm` | ar | الإصدار الجديد %1 متاح.\nهل تريد التحديث؟ |

Code-side placeholder defaults: `MG_UI_DEFAULTS.mg_menu_desc = "シュタインズ・ゲートから%1問出題！"`, `mg_score_result = "%1 / %2 正解"`, `MG_WRONG_DEFAULT = "はぁ…不正解よ。答えは「%1」だったわ。"`.

### 2.17 JSON file format facts (all 26 files verified programmatically; ja.json and en.json read line-by-line)

- UTF-8, **no BOM**, LF line endings, 2-space indentation, one `"key": "value"` per line, final newline. Valid strict JSON (no comments, no trailing commas).
- Top level is a **flat object**; every value is a **string** (no nesting, arrays, numbers or nulls). Keys are `[a-z0-9_]+`. **No duplicate keys** in any file.
- Only escapes used: `\n` (39 per file). No `\uXXXX`, no `\"`. ⇒ values with `\n` **cannot be deserialized as borrowed `&'static str`** by serde (escapes force allocation) — see §5.3.
- No empty values. No HTML/markdown. No `$`.
- Leading/trailing whitespace exists only in `export_success`, `export_failed`, `import_success`, `import_failed` (trailing space) and `import_turns` (leading space), in all 13 languages (meaningful, §2.8).
- Invisible characters: `he.json` `changelog_v101` contains **two U+200B ZERO WIDTH SPACE** characters (inside `בעת ​​מזעור`). Preserve byte-for-byte.
- Content leftovers (port verbatim, do not "fix" silently): `ko.json` uses katakana in `사이リューム 세이버` (`mgq_fg8_a2`, `mgq_fg1_a2`, `mgq_fg2_a2`, `mgq_fg5_a2`, `mgq_fg6_a0`, `mgq_fg6_c`, `mgq_fg7_a1`); `zh.json` uses `ふふ` in `startup_forenoon_11`, `startup_evening_5`, `startup_late_5`. Many short values are identical to ja (numbers, `API`, `USER ID`, dates in zh, etc. — 54–105 per language), which is expected.
- Key order: `en`, `zh`, `ko`, `es`, `fr`, `de`, `ru`, `uk`, `pt`, `tr` follow ja.json order (minus missing keys). `he` and `ar` place `setting_chat_language`/`setting_chat_lang_same_as_ui` at lines 17–18 (right after `setting_display_language`) instead of 64–65. Order is irrelevant to lookup; never iterate dictionaries for display.

### 2.18 Flat key tree (ja.json / en.json line numbers)

```
<root>  (flat; 535 keys in ja/en)
├─ Core UI & settings                         L2–66 (65 keys)
│   you, amadeus_kurisu, amadeus, close, cancel, apply, yes, no, enter_message,              L2–10
│   category_{system,text,sound,graphics,api},                                               L11–15
│   setting_display_language, setting_skip_loading, setting_right_click,
│   setting_eye_tracking, setting_show_notifications, setting_lightweight,
│   setting_text_speed, setting_auto_mode, setting_auto_speed,
│   setting_master_vol, setting_bgm_vol, setting_se_vol, setting_voice_vol,
│   setting_screen_mode, setting_resolution, setting_api_provider, setting_api_key,
│   setting_model_name, setting_web_search, setting_vertex_project,
│   setting_vertex_location, setting_ollama_host, setting_vertex_note,                       L16–38
│   screen_mode_fullscreen, screen_mode_windowed,                                            L39–40
│   logout_confirm, exit_confirm,                                                             L41–42
│   help_right_click, help_toggle_menu, help_save_close, help_select_item,
│   help_confirm_advance, help_cancel_chat,                                                   L43–48
│   changelog_v12, changelog_v11, changelog_v101, changelog_v10,                              L49–52
│   login_user_id, login_password, login_access_denied,                                       L53–55
│   setting_openai_compatible, setting_openai_base_url, setting_openai_base_url_example,
│   openai_compatible,                                                                        L56–59
│   cancel_hint, sleeping_placeholder, changelog_v13, update_confirm,                         L60–63
│   setting_chat_language, setting_chat_lang_same_as_ui, changelog_v14                        L64–66
├─ startup_{night 1–16, morning 1–16, forenoon 1–14, afternoon 1–14, evening 1–14, late 1–13}  L67–153 (87)
├─ achievement_secret_hint, ach_<28 ids>_{title,desc}                                         L154–210 (57)
├─ mg_menu_title … mg_menu_back (10), mg_result_{perfect,good,ok,bad}, mg_wrong               L211–225 (15)
├─ mgq_<49 ids>_{q,a0,a1,a2,a3,c}                                                             L226–519 (294)
├─ setting_export_chat, setting_import_chat, export, import, export_no_data, export_title,
│  export_success, export_failed, import_title, import_invalid, import_success,
│  import_turns, import_failed                                                                 L520–532 (13)
└─ freeapi_quota_{label,usage,remaining,exhausted}   (ja/en only)                             L533–536 (4)
```
Other languages: same groups, line numbers shifted by missing keys (and by the he/ar reordering).

### 2.19 `lib/amadeus/locales` vs `public/resources/locales` — diff and authority

- 11 of 13 files are **byte-identical** (zh, ko, es, fr, de, ru, uk, pt, tr, he, ar).
- `ja.json`, `en.json` differ only at the end: the `public` copies stop at `"import_failed": "…"` (531 keys) and **lack the four `freeapi_quota_*` keys** (lines 533–536 of the lib copies). Everything else is identical.
- **Authoritative: `lib/amadeus/locales/`** — it is the only set imported into the bundle (`localization.ts`). The `public/` copies are referenced by **no code** (grep over app/, components/, lib/, hooks/, public/, scripts/, next.config.ts); they are merely served statically by Next.js at `/resources/locales/<code>.json` and appear to mirror the native QT app layout (`RealAmadeusQT_clone/src/localizationmanager.cpp` loads `resources/locales/<langCode>.json`). Both sets were added in the same commit (`bb8b2ef`); only the lib set received the FreeAPI keys afterwards.
- V2: embed the **lib** files. Whether to keep serving the stale copies at `/resources/locales/*.json` is an open question (harmless if `public/resources` is served verbatim by `amadeus-server`).

### 2.20 User-visible strings in the app that are NOT localized (keep hard-coded for parity)

- Panel titles / labels: `BACKLOG`, `SYSTEM STATUS`, `CHANGE LOG`, `HELP`, `CONFIG`, `ACHIEVEMENTS`, `MINI GAME`, `Amadeus System` (title bar), `Auto` (auto-mode badge), `MENU` (desktop toggle when closed; when open it is `t("close", L===1?"CLOSE":"閉じる")`).
- Desktop menu labels: `BACK\nLOG`, `CONFIG`, `STATUS`, `CHANGE\nLOG`, `HELP`, `FULL\nSCREEN` / `WINDOW`, `LOGOUT`, `SHUT\nDOWN`, `CLOSE\nMENU`.
- Login: desktop `USER ID`, `PASSWORD`, error `ACCESS DENIED` (hard-coded, L2026/L2039/L2054); mobile `USER ID`, `PASSWORD`, button `LOGIN` (mobile error texts *are* localized: `login_empty`, `login_access_denied`).
- Help entry titles `Backspace`, `Enter`, `Ctrl+C`.
- Config labels `Vertex Project ID`, `Vertex Location`, `Ollama Host`; resolution options `1920x1080`, `1600x900`, `1280x720`; provider names `PROVIDER_NAMES`.
- Status rows: `SYSTEM_VERSION` = `Real Amadeus v1.4N(Build 20260801_001)`, `LIVE2D_MODEL`, `OPERATOR`, `NETWORK` (`ONLINE`/`OFFLINE`), `LLM_MODEL`, `AVERAGE_LATENCY` (`Ping: … ms`, `--- ms`), credits block, achievement icons `★ ☆ ?`, secret title `???`.
- Provider display name `OpenAI互換(${baseUrl})` (Japanese for all languages, `getProviderDisplayName`).
- **API error messages shown as Kurisu dialogue are Japanese only** (`handleApiError`, L1656–1678, 12 messages).
- STT status `音声入力: OK` / `音声入力に失敗しました` (L999, L1003); `/dev` help text (L1706–1720) Japanese only.
- Boot log text (`BOOT_LOG`, L265–290) English only.

### 2.21 Locale-sensitive operations NOT tied to the display language

- `Number.prototype.toLocaleString()` (FreeAPI quota numbers) → browser default locale.
- `MemoryManager`: `localeCompare(b, "ja")` sorting and `toLocaleString("ja-JP", {month,day,hour,minute:"2-digit"})` for summaries — always Japanese.
- STT language: separate setting `Config_Experimental_STT_Language` (default `"ja"`), mapped `ja→ja-JP`, `en→en-US`, else passthrough (`stt-service.ts` L360–369). Not linked to `Config_Language`.
- Changelog site page clock: `toLocaleTimeString('en-US', …)`.

---

## 3. RTL handling (Hebrew idx 11, Arabic idx 12)

**V1 has no RTL support at all in the web app.** Verified:
- No `dir` attribute anywhere; root layout renders `<html lang="en" className="dark">` statically (`app/layout.tsx` L20) for every language; `/amadeus/app` layout adds only `<div class="amadeus-root h-screen w-screen overflow-hidden">`.
- No `direction`, `unicode-bidi`, `:dir()`, `:lang()` or `[dir=rtl]` rules in `app/amadeus/app/globals.css`; no RTL-specific code in TS/TSX; no `isRtl` helper; `lang` attribute never changes with `Config_Language`.
- No Hebrew/Arabic fonts are bundled (`public/resources/fonts`: Noto Serif, Noto Serif CJK sc/tc/kr, MateSC, Eurostile, DepartureMono, Liberation Sans); glyphs come from OS fallback fonts. The `Noto Serif JP` face with CJK `unicode-range` and `MS Mincho` local/remote loading are CJK-oriented.

Resulting V1 behaviour when he/ar is selected: strings are inserted into LTR block containers; the browser's Unicode Bidi Algorithm reorders Hebrew/Arabic runs internally, but the paragraph base direction stays LTR, so layout, alignment, flex order, input caret direction and trailing neutrals (e.g. `?`, `…`, `:`) follow LTR rules (e.g. `האם ברצונך להתנתק?` shows `?` at the right end). Mixed strings like `CTRL+C לביטול`, `%1 / %2 נכונות` render in LTR order. LLM replies in he/ar are typed into the same LTR dialogue box.

V2 requirement for parity: **do not** add `dir="rtl"`, mirrored layouts or `lang` changes by default. Provide `Lang::is_rtl()` in core for future use only. (Whether to improve this — e.g. `dir="auto"` on text nodes — is an open question, §6.)

---

## 4. System B — landing/docs dictionary (`hooks/useTranslation.tsx`)

### 4.1 API (verbatim semantics)

```tsx
'use client';
type Locale = 'ja' | 'en';
const translations = { ja: {...}, en: {...} } as any;            // L7–648
const LanguageContext = createContext<{ locale: Locale; toggleLanguage: () => void; t: typeof translations['ja'] } | undefined>(undefined);

export const LanguageProvider = ({ children }) => {
  const [locale, setLocale] = useState<Locale>('ja');           // initial 'ja', never persisted
  const toggleLanguage = () => setLocale(prev => (prev === 'ja' ? 'en' : 'ja'));
  return <LanguageContext.Provider value={{ locale, toggleLanguage, t: translations[locale] }}>{children}</LanguageContext.Provider>;
};

export const useLanguage = () => {
  const context = useContext(LanguageContext);
  if (!context) throw new Error('useLanguage must be used within a LanguageProvider');
  return context;
};
```

- Provider placement: `app/layout.tsx` wraps the **entire site** (`<body>…<LanguageProvider>{children}</LanguageProvider>`). State therefore survives client-side navigations between site pages (Next `<Link>`) and resets to `'ja'` on full reload / new tab / external navigation. No localStorage, cookie, URL parameter or `navigator.language` detection.
- `t` is the dictionary **object** for the current locale. Consumers do property access (`t.amadeus.docs.download.title`), `.map` over arrays, and render JSX leaves directly. There is no fallback: a missing path would throw `TypeError` (both locales have identical structure — verified token-by-token, 249 property names each).
- Toggle buttons on every site page render `{locale === 'ja' ? 'EN' : 'JP'}` (shows the target language) and call `toggleLanguage` (`app/page.tsx` L19–25, `app/amadeus/page.tsx` L56–58, `app/amadeus/download/page.tsx` L56–58, `app/amadeus/changelog/page.tsx` L56–58, `app/amadeus/docs/layout.tsx` L39–44 desktop + L57–62 mobile). Exact markup belongs to the site-pages spec.
- Inline locale ternaries (not in the dictionary): `app/amadeus/changelog/page.tsx` L102 `{locale === 'ja' ? 'ダウンロードへ' : 'BACK TO DOWNLOAD'}`, L118 `{locale === 'ja' ? '戻る' : 'Return'}`.

### 4.2 Dictionary key tree (ja L8–327, en L328–647; identical shapes)

```
Dict
├─ common                                         ja L9–13   / en L329–333
│   ├─ returnMain: string                          UNUSED
│   ├─ systemOnline: string                        app/page.tsx (portfolio root, outside app/amadeus)
│   └─ terminateSession: string                    UNUSED
├─ home                                           ja L14–34  / en L334–354   (portfolio root page app/page.tsx only)
│   ├─ role, tagline, activeProjects: string
│   ├─ projects.amadeus.{title,desc}, projects.elvelt.{title,desc}: string
│   └─ socials.{twitter,github,youtube,support}.{label,sub}: string
└─ amadeus                                        ja L35–326 / en L355–646
    ├─ status.{system,ver,connecting}              ja L36–40   UNUSED
    ├─ button.init                                 ja L41–43   UNUSED
    ├─ panels.{goal,status,access}.{title,content} ja L44–48   UNUSED
    ├─ hud.{logic,voice,memory}                    ja L49–53   UNUSED
    ├─ accessRestricted.{title,desc,button}        ja L54–58   UNUSED
    ├─ login.{userId,password,connect,placeholderId,error}  ja L59–65  UNUSED
    ├─ preparation.{title,message,status}          ja L66–70   UNUSED
    ├─ release                                     ja L71–82 / en L391–402
    │   ├─ title, message, githubRepo, docs, webVersion          → app/amadeus/page.tsx (docs also → changelog page)
    │   ├─ download                                              → app/amadeus/page.tsx, app/amadeus/download/page.tsx
    │   └─ windowsDownload, macosDownload, linuxDownload, lightweightWindowsDownload → app/amadeus/download/page.tsx
    └─ docs                                        ja L83–325 / en L403–645
        ├─ title: string                           UNUSED
        ├─ back: string                            docs/layout.tsx (sidebar "← {back}" link to /amadeus)
        ├─ sidebar.{gettingStarted,download,login,controls,apiSetup,apiOpenAI,apiGemini,apiClaude,apiGroq,apiVertex}   ja L86–97 → docs/layout.tsx
        ├─ gettingStarted.{title,desc,loginInfo}   ja L98–102 → docs/page.tsx
        ├─ download                                ja L103–132 / en L423–452 → docs/download/page.tsx
        │   ├─ title, desc, standardTitle, standardDesc, lightweightTitle, lightweightDesc,
        │   │  recommendedEnv, standardEnv, lightweightEnv, windows, mac, linux, downloadBtn, notice: string
        │   ├─ releasesPage: string                UNUSED
        │   ├─ fallbackStandard: RICH              (§4.3)
        │   ├─ fallbackLightweight: RICH           (§4.3)
        │   ├─ steps: {title, content: string}[2]  rendered via .slice(2) → NEVER rendered (quirk)
        │   └─ fileStructure.{title,windows,mac,linux,windowsQT}: string
        ├─ changelog                               ja L133–219 / en L453–539
        │   ├─ title  → amadeus/page.tsx, amadeus/download/page.tsx, amadeus/changelog/page.tsx, docs/download/page.tsx
        │   ├─ desc   → amadeus/changelog/page.tsx, docs/download/page.tsx
        │   └─ versions: {version, product (NEVER rendered), items: string[]}[6]
        │        V1.4 (9 items) · V1.3 (14) · V1.2 [product RealAmadeusQT] (9) · V1.1 (6) · V1.0.1 (4) · V1.0 (2)
        ├─ loginDoc.{title,desc,idLabel,idValue="Salieri",passLabel,passValue="MakiseKurisu"}  ja L220–227 → docs/login/page.tsx
        ├─ controls.{title,desc, items: {key,desc}[5]}   ja L228–238 → docs/controls/page.tsx
        ├─ api.{title,desc, steps: string[4], notice, modelLabels.{title,available,recommended}}  ja L239–254
        │        → docs/api/page.tsx; modelLabels also used by every provider page
        ├─ apiOpenAI.{title,desc, modelList.{available: string[10], recommended}, steps: {title, content: string|RICH}[4]}  ja L255–268
        ├─ apiGemini   … steps[3], available[10]                                                   ja L269–281
        ├─ apiClaude   … steps[4], available[7]                                                    ja L282–295
        ├─ apiGroq     … steps[3], available[5]                                                    ja L296–308
        └─ apiVertex   … steps[6] (steps[0] and steps[4] are RICH), available[10]                  ja L309–324
```

Data that is identical in both locales (copy once): `modelList` arrays and recommended values —
- OpenAI: `["gpt-5.5","gpt-5.5-pro","gpt-5.5-mini","gpt-5.4","gpt-5.4-pro","gpt-5.4-mini","o3-mini","o1","gpt-4o","gpt-4o-mini"]`, recommended `"gpt-5.5"`.
- Gemini and Vertex: `["gemini-3.5-flash","gemini-3.5-pro","gemini-3.1-pro-preview","gemini-3.1-flash-lite-preview","gemini-omni-flash","gemini-omni","gemini-3-flash-preview","gemini-3-pro-preview","gemini-2.5-flash","gemini-2.5-pro"]`, recommended `"gemini-3.5-flash"`.
- Claude: `["claude-fable-5","claude-opus-latest","claude-sonnet-latest","claude-haiku-latest","claude-opus-4-8","claude-sonnet-4-6","claude-haiku-4-5"]`, recommended `"claude-sonnet-latest"`.
- Groq: `["llama-3.3-70b-versatile","llama-3.1-8b-instant","llama-3.2-11b-vision-preview","qwen/qwen3-32b","mixtral-8x7b-32768"]`, recommended `"llama-3.3-70b-versatile"`.
- Also identical: `home.role` (`Lead Architect at ELVELT`), project titles, social labels, `common.systemOnline`, `amadeus.status.*`, `login.userId/password/placeholderId`, `release.title` (`SYSTEM V1.4 RELEASED`), `release.windowsDownload/macosDownload/linuxDownload`, `docs.title` (`DOCUMENTATION`), sidebar API labels, `loginDoc.idLabel/idValue/passLabel/passValue`, the `key` column of `controls.items` except items #2 and #3 (1-based: `Tab / 右クリック` vs `Tab / Right-click`, `W A S D / 矢印` vs `W A S D / Arrows`).

All other text must be copied verbatim from the file: **ja L8–327, en L328–647** (do not paraphrase; Japanese punctuation such as `（）`, `「」`, `※` must be preserved).

### 4.3 Rich-text (JSX) leaves — exact content as segments

All links share: `target="_blank" rel="noopener noreferrer" className="text-[#d49e38] underline hover:text-[#f0b95b] transition-colors"`. `<code>` has no class. Spaces adjacent to the link are significant.

| Leaf | ja (segments) | en (segments) |
|---|---|---|
| `docs.download.fallbackStandard` (ja L118 / en L438) | `上記のボタンからダウンロードできない場合は、` · LINK(`https://github.com/dsuv-sg/RealAmadeus/releases`, `RealAmadeus Releases`) · `（通常版）から直接ダウンロードしてください。` | `If the download buttons above do not work, please download directly from ` · LINK(same, `RealAmadeus Releases`) · ` (Standard).` |
| `docs.download.fallbackLightweight` (L119 / L439) | `上記のボタンからダウンロードできない場合は、` · LINK(`https://github.com/dsuv-sg/RealAmadeusQT/releases`, `RealAmadeusQT Releases`) · `（軽量化版）から直接ダウンロードしてください。` | `If the download buttons above do not work, please download directly from ` · LINK(same, `RealAmadeusQT Releases`) · ` (Lightweight).` |
| `docs.apiOpenAI.steps[0].content` (L263 / L583) | LINK(`https://platform.openai.com/`, `OpenAI Platform (platform.openai.com)`) · ` にアクセスし、アカウントを登録します。` | `Visit ` · LINK(same) · ` and register.` |
| `docs.apiGemini.steps[0].content` (L277 / L597) | LINK(`https://aistudio.google.com/`, `Google AI Studio (aistudio.google.com)`) · ` にアクセスし、Googleアカウントでログインします。` | `Visit ` · LINK(same) · ` and log in.` |
| `docs.apiClaude.steps[0].content` (L290 / L610) | LINK(`https://console.anthropic.com/`, `Anthropic Console (console.anthropic.com)`) · ` にアクセスし登録します。` | `Register at the ` · LINK(same) · `.` |
| `docs.apiGroq.steps[0].content` (L304 / L624) | LINK(`https://console.groq.com/`, `Groq Console (console.groq.com)`) · ` にアクセスしログインします。` | `Access the ` · LINK(same) · ` and log in.` |
| `docs.apiVertex.steps[0].content` (L317 / L637) | LINK(`https://console.cloud.google.com/`, `Google Cloud (console.cloud.google.com)`) · ` の登録を行います。` | `Register for ` · LINK(same) · `.` |
| `docs.apiVertex.steps[4].content` (L321 / L641) | LINK(`https://dl.google.com/dl/cloudsdk/channels/rapid/GoogleCloudSDKInstaller.exe?hl=ja`, `Google Cloud CLI (gcloud) Installer`) · ` をダウンロード、インストールしてセットアップします。その後、コマンドプロンプト(cmd)を開き、` · CODE(`gcloud auth login`) · `を実行してログインします。` | `Download and install the ` · LINK(same) · `. After installation, open Command Prompt (cmd) and run ` · CODE(`gcloud auth login`) · ` to authenticate.` |

All other `steps[i].content` values are plain strings.

### 4.4 Quirks to preserve (or consciously fix)

1. `docs.download.steps` has 2 entries but both download sections render `steps.slice(2)` → empty list; the step markup never appears.
2. `changelog.versions[].product` is never rendered.
3. Unused subtrees (listed in §4.2) are dead data; port them only if the portfolio/old landing UI is ported.
4. `as any` cast: no compile-time safety in V1; Rust structs will add it.
5. Locale not persisted; initial `'ja'` regardless of browser language; `<html lang="en">` static.
6. Hard-coded English in site pages around dictionary values (e.g. `Step {i + 1}: {title}`, `Windows` / `MacOS` / `Linux` file-structure labels, `Project Amadeus` / `Documentation` mobile docs header, `SYSTEM_ONLINE` on the changelog page) — owned by the site-pages spec.

---

## 5. Rust mapping

### 5.1 Layout

```
crates/amadeus-core/
  locales/{ja,en,zh,ko,es,fr,de,ru,uk,pt,tr,he,ar}.json   ← byte-identical copies of V1 lib/amadeus/locales/*.json (working tree)
  src/i18n/mod.rs        Lang, LANGUAGE_NAMES, CHAT_LANGUAGE_NAMES, t(), tr(), ja_en(), fill(), effective_chat_lang()
  src/i18n/dict.rs       include_str! table + lazy parse
  src/i18n/keys.rs       (optional) `pub const` key names used by Rust code, e.g. `pub const CLOSE: &str = "close";`
  tests/i18n.rs          parity tests (§5.7)
crates/amadeus-ui/
  src/i18n_ui.rs         Leptos glue: language signal, `use_t()` closure, number formatting via Intl
  src/site_i18n/{mod.rs, ja.rs, en.rs}   System B dictionary as static typed data
```

### 5.2 Core API (recommended signatures)

```rust
// amadeus-core/src/i18n/mod.rs
pub const LANG_COUNT: usize = 13;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Lang { Ja = 0, En = 1, Zh = 2, Ko = 3, Es = 4, Fr = 5, De = 6, Ru = 7, Uk = 8, Pt = 9, Tr = 10, He = 11, Ar = 12 }

impl Lang {
    pub const ALL: [Lang; 13] = [/* in index order */];
    pub fn from_index(i: i64) -> Option<Lang>;          // 0..=12 only
    pub fn index(self) -> u8;
    pub fn code(self) -> &'static str;                  // "ja","en","zh","ko","es","fr","de","ru","uk","pt","tr","he","ar"
    pub fn native_name(self) -> &'static str;           // LANGUAGE_NAMES[i]
    pub fn chat_name(self) -> &'static str;             // CHAT_LANGUAGE_NAMES[i]
    pub fn is_rtl(self) -> bool;                        // He | Ar  (informational only; V1 never applies it)
}

pub const LANGUAGE_NAMES: [&str; 13] = ["日本語","English","中文","한국어","Español","Français","Deutsch","Русский","Українська","Português","Türkçe","עברית","العربية"];
pub const CHAT_LANGUAGE_NAMES: [&str; 13] = ["Japanese (日本語)","English","Chinese (中文)","Korean (한국어)","Spanish (Español)","French (Français)","German (Deutsch)","Russian (Русский)","Ukrainian (Українська)","Portuguese (Português)","Turkish (Türkçe)","Hebrew (עברית)","Arabic (العربية)"];

/// Exact port of V1 `t(key, defaultValue, configLanguage)`.
/// `lang` is the raw stored index (may be out of range → ja dictionary).
pub fn t<'a>(key: &'a str, default: &'a str, lang: i64) -> &'a str {
    let idx = if (0..13).contains(&lang) { lang as usize } else { 0 };
    match dict::get(idx, key) {           // -> Option<&'static str>
        Some(v) => v,
        None if !default.is_empty() => default,
        None => key,
    }
}

/// V1 ternary default `L === 1 ? en : ja`.
pub fn ja_en<'a>(lang: i64, ja: &'a str, en: &'a str) -> &'a str { if lang == 1 { en } else { ja } }

/// V1 panel helper `tr(ja, en, key?)` (pass the panel's *pending* language).
pub fn tr<'a>(ja: &'a str, en: &'a str, key: Option<&'a str>, lang: i64) -> &'a str {
    match key {
        Some(k) if !k.is_empty() => t(k, ja_en(lang, ja, en), lang),
        _ => if lang == 0 { ja } else { en },
    }
}

/// V1 `.replace("%1", a).replace("%2", b)` — first occurrence only, applied sequentially.
pub fn fill(template: &str, args: &[&str]) -> String {
    let mut s = template.to_owned();
    for (i, a) in args.iter().enumerate() { s = s.replacen(&format!("%{}", i + 1), a, 1); }
    s
}

/// V1 getCurrentChatLanguageIndex(): 0 => display language, n => n-1 (may be out of range).
pub fn effective_chat_lang(config_language: i64, config_chat_language: i64) -> i64 {
    if config_chat_language == 0 { config_language } else { config_chat_language - 1 }
}
```

Notes:
- Keep the raw `i64` index API (settings hold raw ints; out-of-range behaviour must match). Use `Lang` only where a valid language is guaranteed.
- `fill` intentionally ignores JS `$` replacement patterns (no `$` in data); document in code.
- The persona module uses `CHAT_LANGUAGE_NAMES`/`effective_chat_lang` and falls back to **English** for out-of-range (unlike `t`).
- Settings parsing helper (in core settings module): `parse_js_int(raw: &str) -> Option<i64>` emulating `parseInt(raw, 10)` (§2.3).

### 5.3 Embedding and parsing

```rust
// amadeus-core/src/i18n/dict.rs
use std::{collections::HashMap, sync::OnceLock};

static RAW: [&str; 13] = [
    include_str!("../../locales/ja.json"), include_str!("../../locales/en.json"),
    include_str!("../../locales/zh.json"), include_str!("../../locales/ko.json"),
    include_str!("../../locales/es.json"), include_str!("../../locales/fr.json"),
    include_str!("../../locales/de.json"), include_str!("../../locales/ru.json"),
    include_str!("../../locales/uk.json"), include_str!("../../locales/pt.json"),
    include_str!("../../locales/tr.json"), include_str!("../../locales/he.json"),
    include_str!("../../locales/ar.json"),
];

static DICTS: OnceLock<Vec<HashMap<String, String>>> = OnceLock::new();

fn dicts() -> &'static [HashMap<String, String>] {
    DICTS.get_or_init(|| RAW.iter().map(|s| serde_json::from_str(s).expect("valid locale JSON")).collect())
}

pub fn get(idx: usize, key: &str) -> Option<&'static str> {
    dicts()[idx].get(key).map(String::as_str)
}
```

- Values must be **owned** (`String` or `Cow<'static,str>`): `\n` escapes prevent zero-copy `&'static str` deserialization.
- `OnceLock` works on `wasm32-unknown-unknown` (single-threaded) and native (Tauri, server). No I/O at runtime (compile-time embedding) → satisfies "amadeus-core has no I/O".
- Alternative (optional): a `build.rs` that parses the JSON (serde_json as build-dependency), rejects duplicate keys/non-string values, and emits sorted `&'static [(&str, &str)]` tables (binary search) — zero runtime parse, smaller WASM (no serde_json needed at runtime if not otherwise used). Either approach must produce identical lookups.
- Bundle all 13 languages (V1 bundles all 13 JSON modules into the client JS). ≈490 KB of text.

### 5.4 UI integration (amadeus-ui, Leptos 0.8 CSR)

- Language state: a settings store exposing per-key signals backed by `localStorage` with the **same keys and string formats** (`Config_Language`, `Config_ChatLanguage`); `let lang: Signal<i64> = settings.int("Config_Language", 0);`. Any write to the key (even same value) should notify listeners like `settingsChanged`, but avoid effect loops (V1 writes twice on apply).
- Translator closure for components: `let tx = move |k: &'static str, d: &'static str| amadeus_core::i18n::t(k, d, lang.get()).to_string();` and `ja_en(lang.get(), "閉じる", "Close")` for ternary defaults. Reproduce each call site's exact default from §2.11.
- Settings panels keep a **local pending** language signal; their `tr` uses it (live preview), committing to the store only on Apply.
- `ConfigSelectRow` quirk (index-4 Vertex filter hiding Español / 한국어): implement behind a parity flag (`const PARITY_VERTEX_FILTER_ALL_SELECTS: bool = true`) or fix intentionally — decision in §6.
- AchievementManager port must re-emit "changed" when `Config_Language` is written (V1 L209–213); RandomEvent text is resolved once at startup.
- `<html lang>` and `dir`: leave as V1 (`lang="en"`, no dir) in the web `index.html` (Trunk) and Tauri window.

### 5.5 Number formatting for the FreeAPI quota (UI crate only)

```rust
// amadeus-ui: equivalent of `n.toLocaleString()` with the runtime default locale
pub fn to_locale_string(n: f64) -> String {
    let nf = js_sys::Intl::NumberFormat::new(&js_sys::Array::new(), &js_sys::Object::new());
    nf.format().call1(&wasm_bindgen::JsValue::NULL, &n.into())
        .ok().and_then(|v| v.as_string()).unwrap_or_else(|| n.to_string())
}
```
Do not pass `""` as a locale (`RangeError`). Usage text = `fill(tr("%1 / %2 回使用", "%1 / %2 requests used", Some("freeapi_quota_usage"), pending_lang), &[&to_locale_string(count), &to_locale_string(max)])`.

### 5.6 System B port (`amadeus-ui::site_i18n`)

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum SiteLocale { Ja, En }
impl SiteLocale {
    pub fn toggled(self) -> Self { match self { Self::Ja => Self::En, Self::En => Self::Ja } }
    pub fn toggle_label(self) -> &'static str { match self { Self::Ja => "EN", Self::En => "JP" } }
}
pub enum Seg { Text(&'static str), Link { href: &'static str, text: &'static str }, Code(&'static str) }
pub type Rich = &'static [Seg];
pub enum StepContent { Plain(&'static str), Rich(Rich) }
pub struct Step { pub title: &'static str, pub content: StepContent }
pub struct ChangelogVersion { pub version: &'static str, pub product: &'static str, pub items: &'static [&'static str] }
pub struct SiteDict { pub common: Common, pub home: Home, pub amadeus: AmadeusSite /* mirrors §4.2 exactly */ }
pub static JA: SiteDict = /* ja.rs */; pub static EN: SiteDict = /* en.rs */;
pub fn dict(l: SiteLocale) -> &'static SiteDict;
```
- Provide `RwSignal<SiteLocale>` (initial `Ja`) via `provide_context` in the root `App` (equivalent to the provider in `app/layout.tsx`); `use_language()` returns `(locale, toggle, dict)`; missing context → `panic!("useLanguage must be used within a LanguageProvider")`.
- Render `Seg::Link` as `<a href target="_blank" rel="noopener noreferrer" class="text-[#d49e38] underline hover:text-[#f0b95b] transition-colors">`, `Seg::Code` as `<code>`.
- Keep the `steps.slice(2)` and unused-`product` behaviour unless the site spec decides otherwise.

### 5.7 Tests (amadeus-core `tests/i18n.rs`)

1. All 13 files parse; key counts = 535, 535, 530, 531, 531, 530, 530, 530, 531, 531, 531, 531, 531.
2. For every lang 0..12 and every ja.json key: `t(k, "D", lang)` == file value if present else `"D"`; `t(k, "", lang)` == value or `k`.
3. Out-of-range (`-1`, `13`, `i64::MAX`) behaves like index 0.
4. The 13 keys of §2.13 are absent everywhere; the freeapi/sleeping gaps match §2.14 (guards against accidental "fixes" that change visible text).
5. Placeholder parity: for each key, the set of `%\d` tokens is identical across languages that have it.
6. `fill("%1 / %2 正解", &["3","10"]) == "3 / 10 正解"`; `fill(mg_wrong_ja, &["1.048596%"])` keeps the `%`.
7. `tr` matrix of §2.7 (incl. no-key English for 2–12 and `cancel_button` Japanese for English).
8. Byte-equality check (optional CI script) between `crates/amadeus-core/locales/*.json` and V1 `lib/amadeus/locales/*.json`.

---

## 6. Open questions / risks

1. **Parity vs. fixes.** V1 has visible i18n bugs: (a) Español not selectable as display language and 한국어 not selectable as conversation language on desktop (index-4 Vertex filter applied to all dropdowns); (b) 13 keys missing from every JSON → Japanese shown for 11 non-JA/EN languages; (c) `cancel_button` Japanese for English users; (d) FreeAPI quota strings Japanese for 11 languages; (e) `sleeping_placeholder` Japanese for zh/fr/de/ru; (f) API error messages, STT status and `/dev` help Japanese only. Need an owner decision: strict parity (default recommendation for V2.0) or fix list for V2.x. Adding missing keys to JSON silently changes visible text.
2. **RTL**: V1 renders he/ar in LTR layout with no `dir`. Adding `dir="rtl"`/`dir="auto"` would alter layout of reused CSS. Decide whether V2 keeps parity.
3. **Stale `public/resources/locales`**: unused by code, missing FreeAPI keys in ja/en. Keep serving for URL parity or drop? Must not be used as the embedding source.
4. **Uncommitted working-tree changes** (65,000 quota text; V1.4 product; 5th controls item) — confirm they are intended before freezing V2 data.
5. **Hard-coded quota limit text** (`65,000`) in `freeapi_quota_exhausted` while usage numbers come from the server's `max` — will diverge if the server limit changes.
6. **Locale-dependent number formatting** (`toLocaleString()` uses browser locale, not app language) must be done through JS `Intl` in WASM; tests in core cannot cover it.
7. **Desktop (Tauri) storage origin differs** from the web origin → existing web users' `Config_Language` is not available on desktop; desktop starts at the default (0 = 日本語) unless a migration/import path is added.
8. **serde borrowing pitfall**: deserializing into `&'static str` fails on `\n` escapes; use owned strings or build.rs codegen.
9. **JS `replace` `$`-pattern semantics** are not replicated by `replacen`; identical today (no `$` in data or arguments) but would diverge if translators add `$&`-like text to *replacement* values (answers).
10. **`key in dict` prototype-chain quirk** intentionally not replicated (no call site affected).
11. **System B scope**: `home.*` and `common.systemOnline` serve only the DSUV portfolio root page (outside `app/amadeus`); several `amadeus.*` subtrees are dead. Decide whether V2 ports the portfolio page and dead data.
12. **Data oddities** (he ZWSPs, ko katakana, zh `ふふ`, code/JSON default mismatches) must be ported verbatim; a well-meaning cleanup would change user-visible text.
13. **Double settings writes/emissions** on Apply (panel + `onLanguageChange`) — harmless in V1; in a signal-based Leptos store make sure duplicate notifications cannot cause effect loops.

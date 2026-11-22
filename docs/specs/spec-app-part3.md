# Porting spec — app-part3: RealAmadeusApp panels

**Login, Loading (boot log), BackLog, Status (+StatusRow), Achievements, result-comment tiers, MiniGame, ChangeLog, Help (+HelpEntry), ConfirmationDialog**

- Source of truth: `/home/elvelt_admin/Elvelt/dsuv/components/amadeus/RealAmadeusApp.tsx` lines **1997–3060** (working tree of 2026-11-16).
  The working tree contains *uncommitted* changes inside this range (MiniGamePanel got `className="mini-game-panel"`, `useIsMobileViewport()` and mobile inline positioning) plus 388 new CSS lines (mobile mini-game / achievements). This spec documents the **working tree**, not HEAD.
- Target: `amadeus-ui` (Leptos 0.8 CSR, wasm) for components; pure logic in `amadeus-core`. No component here performs HTTP, Tauri IPC, sessionStorage or IndexedDB access. localStorage is touched only indirectly through `MemoryManager` / `AchievementManager` / `AppSettings` (§14).
- CSS is reused verbatim from `/home/elvelt_admin/Elvelt/dsuv/app/amadeus/app/globals.css` (plain CSS, not Tailwind utilities, for every class in this range). The DOM/class structure below must therefore be reproduced exactly.

---

## 0. Source map

| Lines | Symbol | Kind / signature |
|---|---|---|
| 1997–1999 | `LoginPanelProps` | `{ onAccepted: () => void }` |
| 2001–2057 | `LoginPanel` | desktop login screen |
| 2059–2061 | `LoadingPanelProps` | `{ onFinished: () => void }` |
| 2063–2187 | `LoadingPanel` | desktop boot-log typing sequence |
| 2189–2194 | `BackLogPanelProps` | `{ visible: boolean; entries: BackLogEntry[]; configLanguage: number; onClose: () => void }` |
| 2196–2264 | `BackLogPanel` | conversation history overlay |
| 2266–2274 | `StatusPanelProps` | `{ visible; llmStatus: LlmStatus; configLanguage; isLoggedIn: boolean; achievementsOpen: boolean; onOpenAchievements: () => void; onClose: () => void }` |
| 2276–2411 | `StatusPanel` | SYSTEM STATUS overlay |
| 2413–2424 | `StatusRow` | `({ label: string; value: string /* HTML */ })` |
| 2426–2430 | `AchievementPanelProps` | `{ visible; configLanguage; onClose }` |
| 2432–2511 | `AchievementPanel` | achievement list overlay |
| 2513 | `RESULT_COMMENT_TIERS` | `readonly ResultCommentTier[] = ["perfect","good","ok","bad"]` |
| 2515–2522 | `resultCommentDefaultForKey(key: ResultCommentKey): string` | fn |
| 2524–2530 | `MiniGamePanelProps` | `{ visible; configLanguage; onClose; onEmotionChange: (emotion: string) => void; onSpeakValueChange: (value: number) => void }` |
| 2532–2827 | `MiniGamePanel` | Steins;Gate quiz |
| 2829–2833 | `ChangeLogPanelProps` | `{ visible; configLanguage; onClose }` |
| 2835–2892 | `ChangeLogPanel` | |
| 2894–2898 | `HelpPanelProps` | `{ visible; configLanguage; onClose }` |
| 2900–2960 | `HelpPanel` | |
| 2962–2970 | `HelpEntry` | `({ title: string; description: string })` |
| 2972–2978 | `ConfirmationDialogProps` | `{ visible; message: string; configLanguage; onConfirm: () => void; onCancel: () => void }` |
| 2980–3059 | `ConfirmationDialog` | Yes/No modal |

Definitions used from the same file but outside the range: `BackLogEntry` (57–61), `BOOT_LOG` (265–290), `CHANGE_LOG_DATA` (292–323), `useIsMobileViewport` + helpers (445–475). Imports: `AppSettings` (`lib/amadeus/app-settings.ts`), `MemoryManager` (`lib/amadeus/memory-manager.ts`), minigame data/logic (`lib/amadeus/minigame.ts`), `AchievementManager` (`lib/amadeus/services/achievement-manager.ts`), `LlmStatus` (`lib/amadeus/types.ts`), `t` (`lib/amadeus/localization.ts`).

None of these components is exported; all are internal to `RealAmadeusApp.tsx`.

---

## 1. Shared semantics (dependencies these panels rely on)

### 1.1 `t(key, defaultValue, configLanguage)` (`lib/amadeus/localization.ts:47-53`)

```ts
const dict = translations[configLanguage.toString()] || ja;   // "0".."12" → ja,en,zh,ko,es,fr,de,ru,uk,pt,tr,he,ar; anything else → ja
if (dict && key in dict) return dict[key];                      // returned even if the value is ""
return defaultValue || key;                                     // "" default → the key itself is returned
```

- Language index order: `0 ja, 1 en, 2 zh, 3 ko, 4 es, 5 fr, 6 de, 7 ru, 8 uk, 9 pt, 10 tr, 11 he, 12 ar` (files `lib/amadeus/locales/<code>.json`).
- Recurring pattern in this range: `t(key, configLanguage === 1 ? "<English>" : "<日本語>", configLanguage)`. The inline default is only visible when the key is missing from the selected locale. **Verified:** every key used in this range exists in all 13 locale files **except** `help_right_click_title` and `help_wasd`, which exist in **no** locale → their inline fallback is always shown (English for lang 1, Japanese for every other language, including zh/ko/…). See §10 and Open questions.
- Also verified: all 294 `mgq_*` keys, all 56 `ach_*` keys, all 6 `changelog_*` keys and all `mg_*` keys exist in all 13 locales. The `ja.json` values of 50 `mgq_*` keys differ from the JP defaults in `minigame.ts` (e.g. `mgq_drink_c`: default `べ、別に好きってわけじゃないんだから！` vs ja.json `べ、別にドクペが好きってわけじゃないんだからね！これは知的飲料だから、科学者として当然の選択よ！`). **Displayed text always comes from the locale JSON**; code defaults are fallback only. Embed the JSON files verbatim.

### 1.2 `configLanguage`
Parent passes `useAppSettingInt("Config_Language", 0)` (reactive on `AppSettings.settingsChanged`). All panel strings re-render live when the language changes.

### 1.3 `Signal` (`lib/amadeus/signal.ts`)
`connect(handler) → disconnect fn`; `emit(...args)` calls every handler over a snapshot copy of the handler set. Used here: `AchievementManager.achievementsChanged` (no args), `MemoryManager.userNameChanged` (no args).

### 1.4 `AppSettings.getInt(key, default)` (`lib/amadeus/app-settings.ts:92-99`)
Reads `localStorage[key]`; missing → `default`; else `Number.parseInt(raw, 10)`; non-finite (NaN) → `default`. JS `parseInt` semantics (leading whitespace skipped, trailing garbage ignored: `"1abc"` → 1). Used here only for `Config_SkipLoading` (default 0; stored as the string `"0"`/`"1"`).

### 1.5 `MemoryManager` subset (`lib/amadeus/memory-manager.ts`)
- localStorage key **`RealAmadeus.Memory`** (JSON object; full shape owned by the memory spec). Field used here: `userName?: string`.
- `get userName(): string` → `memory.userName ?? ""`.
- `setUserName(name)`: `trimmed = name.trim(); if (!trimmed) return;` (**empty → silent no-op, nothing saved, no signal**) else `memory.userName = trimmed; saveMemory()` (writes the whole JSON; swallow+`console.warn` on storage error) `; userNameChanged.emit()`.
- `userNameChanged: Signal<[]>`.

### 1.6 `AchievementManager` subset (`lib/amadeus/services/achievement-manager.ts`)
- Singleton created at module load (loads localStorage key **`amadeus_achievements`**, updates launch streak). State JSON: `{ unlocked: string[], dates: Record<id, "YYYY-MM-DD HH:MM">, seenEmotions: string[], lastLaunchDate: "YYYY-MM-DD", launchStreak: number }`.
- `achievementsChanged: Signal<[]>` — emitted after any unlock, after `resetAll()`, and whenever `AppSettings.settingsChanged` fires with key `"Config_Language"`.
- `achievementUnlocked: Signal<[id, title, description]>` — consumed by ChatPanel's toast (not in this range; toast visible 4000 ms).
- `get unlockedCount` → `state.unlocked.length` (**counts any stored id, even ids not in the 28 definitions**).
- `get totalCount` → `28` (definitions length).
- `getAllAchievements(): AchievementView[]` — one view per definition, in definition order (§6.4):
  ```ts
  interface AchievementView { id: string; icon: string; secret: boolean; unlocked: boolean; title: string; description: string; unlockedAt: string }
  // unlocked = state.unlocked.includes(id)
  // if (secret && !unlocked) { title = "???"; description = t("achievement_secret_hint", "シークレットアチーブメント", lang) }
  // else { title = t(titleKey, defaultTitle, lang); description = t(descKey, defaultDesc, lang) }
  // unlockedAt = state.dates[id] ?? ""
  // lang = AppSettings.getInt("Config_Language", 0)   (NOT the configLanguage prop)
  ```
- `notifyEvent(type)`: used here with `"game_play"` → unlock `first_game`; `"game_win"` → `game_win`; `"game_perfect"` → `game_perfect`. `unlock(id)` is idempotent; on first unlock it stores `dates[id] = "YYYY-MM-DD HH:MM"` (local time), saves, emits `achievementUnlocked(id, title, desc)` then `achievementsChanged()`.

### 1.7 `useIsMobileViewport()` (RealAmadeusApp.tsx 445–475)
```ts
visualWidth = window.visualViewport?.width ?? window.innerWidth;
coarse = matchMedia("(pointer: coarse)").matches;
isMobile = visualWidth <= 900 || (coarse && visualWidth <= 1200);
```
Re-evaluated on `window.resize`, `visualViewport.resize`, and `change` of the `(pointer: coarse)` media query. Server snapshot `false`. Used in this range only by `MiniGamePanel` (the parent uses it to choose the desktop vs mobile tree, §11).

### 1.8 Common overlay pattern (BackLog, Status, Achievement, MiniGame, ChangeLog, Help, Confirmation)
1. The panel is **always mounted** while its parent tree is mounted; it is shown/hidden only via inline styles on an outer wrapper:
   `opacity: visible ? 1 : 0; pointer-events: visible ? "auto" : "none"; transition: opacity <D> ease-in-out; position: absolute; top:0; left:0; right:0; bottom:0; z-index: <Z>`.
   Consequences that must be preserved: fade-in **and** fade-out animate; DOM and internal state (scroll position of lists, mini-game progress, clock text) persist while hidden.
2. Inner `.overlay-panel` / `.dialog-overlay` have a CSS `animation: overlay-fade 250ms linear` that runs **once at mount** (while invisible) — it has no visible effect in practice. The visible fade is the wrapper transition.
3. Keyboard: a `keydown` listener is attached to **`window`** only while `visible` is true and removed when hidden/unmounted. Listeners run in bubble phase on window, so they also see keys typed inside inputs.
4. Close button: `<button type="button" class="overlay-close-button">{t("close", configLanguage === 1 ? "Close" : "閉じる", configLanguage)}</button>`.

| Panel | wrapper class | D | Z (inline) | extra inline styles |
|---|---|---|---|---|
| BackLogPanel | `backlog-panel-wrapper` (no CSS rule exists for it) | `0.3s` | 3000 | `width:100%; height:100%; overflow:hidden` |
| StatusPanel | — (no class) | `0.3s` | 2000 | |
| AchievementPanel | — | `0.25s` | 2500 | |
| MiniGamePanel | root itself is `div.mini-game-panel` | `0.25s` | 50 | `position: isMobile?"fixed":"absolute"; top: isMobile?"auto":0; bottom:0; left:0; right:0; height: isMobile?"46%":"100%"` |
| ChangeLogPanel | — | `0.3s` | 2000 | |
| HelpPanel | — | `0.3s` | 2000 | |
| ConfirmationDialog | — | `0.3s` | 3000 | |

React serializes `top: 0` as `top:0` (no px) and `zIndex` unitless. Exact inline-string formatting is not significant; values are.

### 1.9 Static assets referenced (root-absolute URLs, no base path)
- `/resources/images/amadeus_logo_v3.png` (login logo, boot logo)
- `/resources/images/amadeus_login_button_v2.png` (login button)
- `/resources/images/Amadeus_BG.png` (`.overlay-bg` of BackLog/Status/Achievement/ChangeLog/Help)
- Fonts via CSS: `DepartureMono` (`/resources/fonts/DepartureMono-Regular.otf`, boot text), `MateSC` (login labels/error), `MS Mincho` (inputs, status), `Noto Serif CJK JP`/`Noto Serif JP`, `Liberation Sans`.
V1 serves `public/resources/**` at site root `/resources/...` (Next has no basePath; the app page itself is at `/amadeus/app`). The V2 server must keep serving them at `/resources/...` (or the UI must prefix a configured base consistently).

---

## 2. LoginPanel (2001–2057) — desktop only

### 2.1 State
| name | type | initial |
|---|---|---|
| `userId` | string | `""` |
| `password` | string | `""` |
| `errorVisible` | boolean | `false` |

### 2.2 `attemptLogin()` (exact)
```ts
const validNames = ["Salieri"];
const validPasswords = ["MakiseKurisu"];
if (userId.trim() && password.trim() &&
    (validNames.includes(userId.trim()) || validPasswords.includes(password.trim()))) {
  setErrorVisible(false);
  MemoryManager.setUserName("Salieri");   // ALWAYS "Salieri", never the typed id
  onAccepted();
} else {
  setErrorVisible(true);
}
```
Truth table (after JS `trim()`; comparisons case-sensitive):
| userId | password | result |
|---|---|---|
| blank | anything | denied |
| anything | blank | denied |
| `Salieri` | any non-blank | **accepted** |
| any non-blank | `MakiseKurisu` | **accepted** |
| other | other | denied |

- On success `MemoryManager.setUserName("Salieri")` overwrites any name previously learned from chat, persists `RealAmadeus.Memory`, and emits `userNameChanged` (StatusPanel OPERATOR re-renders). Then `onAccepted()` → parent `setAppState("boot")` → LoginPanel unmounts, LoadingPanel mounts.
- On failure: `ACCESS DENIED` appears and stays (no timer, no shake, no field clearing). It disappears only by a successful login (unmount).
- JS `String.prototype.trim` removes ECMAScript WhiteSpace+LineTerminator (incl. U+FEFF, U+00A0, U+2028/2029; excludes U+0085). Rust `str::trim` differs on U+FEFF/U+0085 — use a `js_trim` helper for exact parity.

### 2.3 DOM (exact)
```html
<div class="login-panel">
  <img src="/resources/images/amadeus_logo_v3.png" alt="logo" class="login-logo">
  <div class="login-row user-id-row">
    <label>USER ID</label>
    <input value="{userId}">                      <!-- autoFocus; type omitted (= text) -->
  </div>
  <div class="login-row password-row">
    <label>PASSWORD</label>
    <input type="password" value="{password}">
  </div>
  <button type="button" class="login-button">
    <img src="/resources/images/amadeus_login_button_v2.png" alt="login">
  </button>
  <!-- only when errorVisible: -->
  <div class="login-error">ACCESS DENIED</div>
</div>
```
- Labels are **not** associated with inputs (no `for`/`htmlFor`); clicking a label does not focus anything. Keep.
- Labels/“ACCESS DENIED”/“USER ID”/“PASSWORD” are hard-coded English (not localized) on desktop.

### 2.4 Events, keyboard, focus
- `input.onChange` → controlled value update (both inputs).
- `onKeyDown` on **both** inputs: `if (event.key === "Enter") attemptLogin();` (no `preventDefault`).
- `button.onClick` → `attemptLogin()`.
- Focus: USER ID input is focused on mount (`autoFocus`; React calls `.focus()` during commit). Leptos: the `autofocus` attribute is not reliable for dynamically inserted nodes — call `HtmlInputElement::focus()` after mount (e.g. `NodeRef` + `Effect`/`request_animation_frame`).
- Cross-cutting (parent, part4): the global `keydown` handler intercepts **Tab** whenever no overlay is open (`preventDefault` + toggles the side menu), and right-click toggles the menu (if `Config_RightClickMenu` = 1). The menu is mounted regardless of `appState`, so on the login screen Tab does **not** move focus to PASSWORD; it opens the menu and the 450 ms effect blurs the focused input. This is V1 behavior (see Open questions).

### 2.5 Layout facts (from CSS 204–296; reused verbatim)
Login sits inside `.system-panel` (1920×1080 virtual viewport scaled by the parent) above `.system-bg`, below the 35 px `.title-bar` ("Amadeus System"). Logo 700×700 centered at (50%, 50%−182px). USER ID row top `calc(50% + 106.5px)`, PASSWORD row `calc(50% + 176.5px)`, inputs 600×50 at x −300px from center, labels right-aligned ending at −319px, label colors `#ffe600` / `#ffd700`, input text `#ffd700` 36px MS Mincho. Button 65×65 at `left: calc(50% + 301.5px); top: calc(50% + 169px)`. Error red `#ff0000`, 20px MateSC bold, at `top: calc(50% + 271.5px)`, `translate(calc(-50% - 220px), -50%)`.

### 2.6 Mobile counterpart (out of range, for orientation only — specified in app-part1)
On mobile the parent renders `MobileLoginPanel` (lines 485–538) instead. Differences: no trimming (`!userId || !password` → localized `login_empty` error — key missing in all locales → fallback `"Please enter both fields."`/`"ユーザーIDとパスワードを入力してください。"`), mismatch → `t("login_access_denied", …)`; Enter only on the password field (with `preventDefault`); same `setUserName("Salieri")`.

---

## 3. LoadingPanel (2063–2187) — desktop boot sequence

### 3.1 Constants
`BOOT_LOG` (RealAmadeusApp.tsx 265–290), **verbatim** (JS string concatenation; `\n` are newlines):
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
Notes: typos (`Initakising`, `/sts`, `is Activate`, double spaces before `OK`) are intentional verbatim copies of the Unity/QT originals — keep them. `<pos=N%>` is a Unity TextMeshPro rich-text tag emulated with spaces (§3.3). The log has **41 lines** (index 0–40); line 10 is the memory line; lines 11–14, 16–19, 21–24, 36–39 are empty.

Magic numbers: start delay **500 ms**; per-line delay **50 ms**, or **400 ms** if the line contains `"..."` or is blank after trim; memory animation **duration 1500 ms, step 30 ms**, max **32767**; post-log wait **2000 ms**; logo display **4000 ms**; virtual width **1920**; font `18px "DepartureMono", monospace`; fallback char width **10**.

### 3.2 State
| name | initial | meaning |
|---|---|---|
| `displayText` | `BOOT_LOG.replace("[MEM]", "0")` (unprocessed, still contains `<pos=30%>`) | full text, revealed progressively |
| `visibleChars` | `0` | number of UTF-16 code units of `displayText` shown |
| `showLogo` | `false` | swap text for logo |

### 3.3 Algorithm (effect on mount; deps `[onFinished]`)
```text
if AppSettings.getInt("Config_SkipLoading", 0) == 1:
    setTimeout(onFinished, 0)            // cleanup clears it
    return

cancelled = false; timeoutIds = []
wait(ms) = Promise resolved by setTimeout(ms) (id pushed to timeoutIds)

charWidth = estimateCharWidth():
    ctx = document.createElement("canvas").getContext("2d"); if !ctx → 10
    ctx.font = '18px "DepartureMono", monospace'
    return ctx.measureText("M").width || 10          // 0 → 10

preprocessBootLog(raw):
    for each line of raw.split("\n"):
        m = line.match(/<pos=(\d+)%>/)               // first match only
        if !m: keep line
        else:
            percent = parseInt(m[1], 10)
            [prefix, suffix = ""] = line.split(m[0])  // JS split on ALL occurrences; take parts[0], parts[1]
            targetX = 1920 * (percent / 100)          // 1920*(30/100) == 576.0 exactly in IEEE double
            prefixWidth = prefix.length * charWidth   // UTF-16 length (ASCII here)
            spaces = max(1, floor((targetX - prefixWidth) / charWidth))
            line = prefix + " ".repeat(spaces) + suffix
    return lines.join("\n")

buildDisplayText(mem) = preprocessBootLog(BOOT_LOG.replace("[MEM]", String(mem)))   // first occurrence

runBootSequence():
    await wait(500); if cancelled return
    text = buildDisplayText(0); setDisplayText(text); lines = text.split("\n"); visible = 0
    for lineIndex in 0 ..< lines.length:
        if cancelled return
        line = lines[lineIndex]
        if line.includes("Memory Initialize"):
            for elapsed = 0; elapsed <= 1500; elapsed += 30:        // 51 frames: 0,30,…,1500
                if cancelled return
                progress = min(1, elapsed / 1500)
                mem = floor(progress * 32767)                     // keep this exact operation order (f64)
                text = buildDisplayText(mem); setDisplayText(text); lines = text.split("\n")
                visible = Σ_{i=0..lineIndex, i<lines.length} (lines[i].length + 1)
                setVisibleChars(visible)
                if elapsed < 1500: await wait(30)
            continue                                              // no extra delay after the memory line
        visible += line.length + 1
        setVisibleChars(visible)
        delay = (line.includes("...") || line.trim() === "") ? 400 : 50
        await wait(delay)
    await wait(2000); if cancelled return
    setShowLogo(true)
    await wait(4000); if !cancelled → onFinished()

cleanup: cancelled = true; clearTimeout(every id)   // pending promises never resolve
```
Details:
- The `<pos>` column is independent of the prefix: `prefix + floor(576/cw − prefix)` = `floor(576/cw)` (when ≥ 1 space). So every `[OK]` starts at column `floor(576 / charWidth)`: **50** with DepartureMono loaded (advance 350/550 em = 11.4545 px at 18 px, computed from the OTF hmtx table), **53** with a ≈10.8 px fallback monospace (Courier New 0.600 em, DejaVu Sans Mono / Menlo 0.602 em), **58** with Consolas (0.5498 em = 9.90 px; Chrome's default `monospace` on Windows), 57 with the hard 10 px fallback.
- Font-timing quirk: measurement happens synchronously at mount, while the `<pre>` is still empty, so on the first boot of a page session DepartureMono is normally **not yet loaded** → the platform's fallback monospace width is measured → column ≈53 (Linux/macOS) or ≈58 (Windows/Consolas). On a later boot in the same session (logout → login) the font is loaded → column 50. (QT measured after its FontLoader → 50.) The rendered glyphs are DepartureMono once loaded (`font-display: swap`), so only the column count differs.
- `visibleChars` may exceed `displayText.length` by 1 after the last line (trailing `+1`); `slice` clamps. Rust must clamp (`min(len)`); the text is ASCII, so byte index = UTF-16 index, but prefer char-safe slicing.
- The memory line shows `0` first, then `655, 1310, …, 32767` (`floor(k/50*32767)` computed as `floor(min(1, (k*30)/1500) * 32767)`), number width changes so `visibleChars` is recomputed every frame.
- Rendering: `{!showLogo && <pre class="boot-text">{displayText.slice(0, visibleChars)}</pre>}` and `{showLogo && <img src="/resources/images/amadeus_logo_v3.png" alt="logo" class="boot-logo">}`. The `<pre>` is removed when the logo appears; the logo appears instantly (no fade). No autoscroll (the 41 lines × 21.6 px + 70 px padding = 956 px fit in 1080).

### 3.4 Nominal timeline (ms from mount; setTimeout drift adds ~1 ms per wait in practice)
| t | event |
|---|---|
| 0 | mount, empty `<pre class="boot-text">` |
| 500 | line 0 `Amadeus System Ver 1.09.2 rev.2123` (then 50) |
| 550 / 950 / 1350 / 1750 / 2150 | lines 1–5 (each contains `...` → 400) |
| 2550 / 2950 / 3350 / 3750 | lines 6–9 Processor 0–3 (400 each) |
| 4150 → 5650 | memory line animates 0 → 32767 (51 frames, 30 ms apart) |
| 5650, 6050, 6450, 6850 | blank lines 11–14 (400 each; line 11 appears in the same tick as the final memory frame) |
| 7250 | `INIT: Kernel version 2.04 booting...` (400) |
| 7650 … 8850 | blank 16–19 |
| 9250 | `ROSS:` (**50**) |
| 9300 … 10500 | blank 21–24 |
| 10900, 11300 | Mounting proc / sysfs (400) |
| 11700 | `Initakising network` (**50**) |
| 11750 … 13350 | localhost, inet1, route, Cloud, system log (400 each) |
| 13750, 13800, 13850 | Cleaning /var/lock, Cleaning /tmp, Updating init.rc (**50** each) |
| 13900 … 15100 | blank 36–39 |
| 15500 | `Boot Sequences Start...` (400) |
| 17900 | text replaced by logo (`showLogo = true`) |
| **21900** | `onFinished()` → parent `setAppState("chat")` |

### 3.5 Quirks
- **Effect restart (do not replicate):** deps are `[onFinished]` and the parent passes a fresh arrow every render (`<LoadingPanel onFinished={() => setAppState("chat")} />`). Any parent re-render during boot (window resize → `dimensions`, Tab/right-click → `isTabActive`, fullscreen change → settings) cancels and restarts the whole sequence: after 500 ms `visibleChars` falls back to line 0 (text "shrinks"); if the logo was already shown it stays shown (state not reset) and `onFinished` is delayed by a full new run. Leptos components run once, so a natural port does not have this bug — intended behavior is one uninterrupted run.
- `Config_SkipLoading` is read once at mount. With 1, the panel renders one empty frame and finishes on the next macrotask.
- Mobile uses `MobileLoadingPanel` (lines 540–574, app-part1): 1500 ms timer + `Booting`/`起動中` dots every 300 ms (`t("booting", …)` – key missing in all locales).

---

## 4. BackLogPanel (2196–2264)

### 4.1 Input type
```ts
type BackLogEntry = { role: string; nameColor: string; message: string };   // lines 57-61
```
Produced by the parent's `addBackLog(role, rawMessage)` (part4, lines 4018–4043): message trimmed; a leading `[...]` block is removed (`if startsWith("[")` and `indexOf("]") > 0` → slice after it, trim); then every `[NORMAL|SMILE|ANGRY|SAD|SURPRISED|BLUSH|WINK|DISGUST|SMUG|THINKING|PANIC]` (case-insensitive, global) removed; trimmed; empty → not added. `role = role.toLowerCase()`; `nameColor = "#66CCFF"` for `user`/`me`, `"#FF6666"` for `assistant`/`kurisu`/`amadeus`, else `"#808080"`. List is append-only (reset to `[]` only on conversation import; **not** cleared on logout).

### 4.2 Effects
1. While `visible`: window keydown → `if (event.key === "Backspace") { event.preventDefault(); onClose(); }`. (Escape does nothing.)
2. On `[entries, visible]` change while visible: `contentRef.scrollTop = contentRef.scrollHeight` (scroll to bottom; runs after paint). Opening the panel always jumps to the newest entry; new entries while open also auto-scroll. Leptos: do it after the DOM update (e.g. `request_animation_frame`).

### 4.3 DOM (exact)
```html
<div class="backlog-panel-wrapper" style="opacity; pointer-events; transition:opacity 0.3s ease-in-out; position:absolute; top:0; left:0; right:0; bottom:0; width:100%; height:100%; overflow:hidden; z-index:3000">
  <div class="dialog-overlay backlog-panel">
    <img src="/resources/images/Amadeus_BG.png" alt="" class="overlay-bg">
    <div class="overlay-header">
      <h2>BACKLOG</h2>
      <div class="line"></div>
    </div>
    <div class="backlog-content">                         <!-- contentRef -->
      <!-- per entry (key `${role}-${index}`): -->
      <div class="backlog-row backlog-row-{role}">
        <div class="backlog-name" style="color: {nameColor}">{name}</div>
        <div class="backlog-message">{message}</div>       <!-- plain text; CSS pre-wrap keeps \n -->
      </div>
    </div>
    <button type="button" class="overlay-close-button">{t("close", lang===1?"Close":"閉じる", lang)}</button>
  </div>
</div>
```
`name`:
- `role === "user" || role === "me"` → `t("you", lang===1 ? "You" : "あなた", lang)`
- `role === "assistant" || "kurisu" || "amadeus"` → `t("amadeus_kurisu", lang===1 ? "Amadeus Kurisu" : "アマデウス紅莉栖", lang)`
- otherwise → `role.toUpperCase()` (e.g. `SYSTEM`).

Note the BackLog uses **`dialog-overlay`** (semi-transparent `rgba(26,26,26,0.7)`, flex-centered, z 130) rather than `overlay-panel`; all children are absolutely positioned so centering is irrelevant. Title `BACKLOG` is not localized.

### 4.4 Desktop vs mobile
Same component in both trees. Desktop: first child of `.viewport`. Mobile: CSS 2459–2581 turns `.dialog-overlay.backlog-panel` into a fixed full-screen flex column (`100vw × 100dvh`, `rgba(10,5,0,0.95)`, blur 12px), header becomes static 24px, rows become rounded cards (user/me rows get a right `#ffcc00` border instead of the left `#ff9900` one), close button becomes a full-width 46px pill. Opened on mobile from the top-bar `LOG` button (part4), on desktop from menu item 0 (BACK LOG).

---

## 5. StatusPanel (2276–2411) and StatusRow (2413–2424) — desktop only

### 5.1 State
| name | initial | update |
|---|---|---|
| `clock` | `""` | every 1000 ms while visible (+ immediately on open) |
| `networkReachable` | `true` | `navigator.onLine` every 5000 ms while visible (**not** checked immediately on open) |
| achievement revision | 0 | bumped by `AchievementManager.achievementsChanged` (subscribed for the panel's whole lifetime) |
| user revision | 0 | bumped by `MemoryManager.userNameChanged` (lifetime) |
| `mounted` | false → true after first effect | SSR guard only; in CSR treat as always true |

### 5.2 Visible-only effect (deps `[achievementsOpen, onClose, visible]`)
```text
if !visible: return
update(): now = new Date() (local time)
          clock = `${YYYY}/${MM}/${DD} ${HH}:${mm}:${ss}`   // year unpadded, others padStart(2,"0"), 24h
update(); timer = setInterval(update, 1000)
networkTimer = setInterval(() => setNetworkReachable(navigator.onLine), 5000)
keydown: if (achievementsOpen) return;                       // AchievementPanel on top owns the keys
         if (key === "Backspace") { preventDefault(); onClose(); }
cleanup: clear both intervals, remove listener
```
- Interval phase is relative to the moment the panel opened (seconds may lag real time by < 1 s). The last clock text stays in the DOM after closing.
- Quirk (do not replicate): `onClose` is a new arrow every parent render, so in V1 the intervals restart on every parent re-render while open (resets the 5 s network poll).
- Escape does **not** close the StatusPanel.

### 5.3 DOM (exact)
```html
<div style="opacity; pointer-events; transition:opacity 0.3s ease-in-out; position:absolute; top:0; left:0; right:0; bottom:0; z-index:2000">
  <div class="overlay-panel status-panel">
    <img src="/resources/images/Amadeus_BG.png" alt="" class="overlay-bg">
    <div class="overlay-header">
      <h2>SYSTEM STATUS</h2>
      <div class="line"></div>
    </div>
    <div class="status-clock">{clock}</div>
    <div class="status-grid">
      <!-- 6 × StatusRow, in this order -->
    </div>
    <button type="button" class="status-achievement-row">        <!-- onClick → onOpenAchievements() -->
      <span class="status-achievement-label">ACHIEVEMENTS</span>
      <span class="status-achievement-value">★ {unlockedCount} / {totalCount}</span>
      <span class="status-achievement-underline" aria-hidden="true"></span>
    </button>
    <div class="status-credits">Provided by ELVELT/Real Amadeus Project<br>Developed by DSUV<br>Designed by DSUV/Amane<br>Translated with assistance by Dezpot<br>This project is a derivative work of Steins;Gate 0<br>Real Amadeus v1.4N(Build 20260801_001)</div>
    <button type="button" class="overlay-close-button">{t("close", …)}</button>
  </div>
</div>
```
`StatusRow`:
```html
<div class="status-row">
  <div class="status-label">{label}</div>
  <div class="status-value">{value inserted as innerHTML (dangerouslySetInnerHTML)}</div>
</div>
```

### 5.4 Rows (label → value), exact
| # | label | value (HTML string) |
|---|---|---|
| 1 | `SYSTEM_VERSION` | `Real Amadeus v1.4N(Build 20260801_001)` |
| 2 | `LIVE2D_MODEL` | `Live2DKurisu v1.0` |
| 3 | `OPERATOR` | `isLoggedIn ? (MemoryManager.userName ? MemoryManager.userName : "Salieri") : "---"` |
| 4 | `NETWORK` | online → `<span class='good'>ONLINE</span>`; offline → `<span class='bad'>OFFLINE</span>` |
| 5 | `LLM_MODEL` | ``llmStatus.provider && llmStatus.model ? `${provider} / ${model}` : "---"`` (both must be non-empty) |
| 6 | `AVERAGE_LATENCY` | `latencyMs < 0` → `--- ms`; else `Ping: <span class='${cls}'>${Math.round(latencyMs)} ms</span>` with `cls = latencyMs > 3000 ? "bad" : latencyMs > 1000 ? "warn" : "good"` (thresholds on the **unrounded** value; 1000 → good, 3000 → warn) |

- All labels, `ACHIEVEMENTS`, `SYSTEM STATUS` and the credits are hard-coded English. Only the close button is localized.
- `ACHIEVEMENTS` value: `★ ${unlockedCount} / ${totalCount}` (with star; space after star). Before mount V1 renders `★ 0 / 28` (SSR guard) — irrelevant for CSR.
- CSS colors: `.good #44ff44`, `.warn #ffff44`, `.bad #ff4444`; label `#ff9900`; hover on the achievement row brightens label/value and shows a 1px underline at 0.6 opacity (CSS 1027–1037).
- `isLoggedIn` = parent `appState === "chat"`. During chat OPERATOR shows the stored name or `Salieri`. The side menu (and therefore STATUS, CHANGE LOG, HELP, BACK LOG, CONFIG, LOGOUT, SHUT DOWN) is **also reachable on the login and boot screens** via Tab / right-click (the menu is not gated by `appState`); there OPERATOR shows `---`.
- `LlmStatus = { provider: string; model: string; latencyMs: number }`. Parent semantics (part4/part2): initial `{ "", "", -1 }`; at app mount `getLlmStatusFromSettings()` (latency −1 → `--- ms`); **ChatPanel mount calls `updateStatus(0)` → `Ping: 0 ms` (good)**; afterwards latency = `Date.now() − requestStart` at the first stream token or at the full response. Despite the label it is the *last* latency, not an average. `provider` comes from `getProviderDisplayName` (e.g. `FreeAPI`, `Gemini`, `OpenAI互換(${baseUrl})`).
- Security: values are injected as HTML. `OPERATOR` can contain a name extracted from LLM output and `LLM_MODEL` contains user-entered model names / base URLs → V1 is HTML-injectable. See Rust mapping (render structured, escaped values).

### 5.5 Keyboard interplay with AchievementPanel
`achievementsOpen` (parent state) is passed in; while it is true the StatusPanel ignores Backspace and the AchievementPanel (z 2500 > 2000) handles Backspace/Escape → closes only the achievements, returning to the status panel.

---

## 6. AchievementPanel (2432–2511)

### 6.1 Behavior
- Subscribes to `AchievementManager.achievementsChanged` for its lifetime (re-render on unlock, reset, or language change).
- While visible: window keydown → `if (key === "Backspace" || key === "Escape") { preventDefault(); onClose(); }`.
- Data (CSR): `achievements = AchievementManager.getAllAchievements()`, `unlockedCount = AchievementManager.unlockedCount`, `totalCount = AchievementManager.totalCount` (V1 shows `[]`, `0 / 0` before mount — SSR only).

### 6.2 DOM (exact)
```html
<div style="opacity; pointer-events; transition:opacity 0.25s ease-in-out; position:absolute; top:0; left:0; right:0; bottom:0; z-index:2500">
  <div class="overlay-panel achievement-panel">
    <img src="/resources/images/Amadeus_BG.png" alt="" class="overlay-bg">
    <div class="achievement-panel-title">ACHIEVEMENTS</div>
    <div class="achievement-panel-progress">{unlockedCount} / {totalCount}</div>   <!-- no star here -->
    <div class="achievement-list">
      <!-- per achievement (key = id), definition order -->
      <div class="achievement-row {unlocked ? 'unlocked' : 'locked'}">
        <div class="achievement-icon {unlocked ? 'unlocked' : 'locked'}">{unlocked ? "★" : secret ? "?" : "☆"}</div>
        <div class="achievement-text">
          <div class="achievement-row-title">{title}</div>
          <div class="achievement-row-desc">{description}</div>
        </div>
        <!-- only if unlocked && unlockedAt !== "" -->
        <div class="achievement-date">{unlockedAt}</div>                       <!-- "YYYY-MM-DD HH:MM" verbatim -->
      </div>
    </div>
    <button type="button" class="overlay-close-button">{t("close", …)}</button>
  </div>
</div>
```
Class strings are exactly `"achievement-row unlocked"`, `"achievement-row locked"`, `"achievement-icon unlocked"`, `"achievement-icon locked"`.

### 6.3 Listing format rules
| state | icon | title | description | date |
|---|---|---|---|---|
| unlocked (secret or not) | `★` | localized title | localized desc | shown if `dates[id]` non-empty |
| locked, not secret | `☆` | localized title (dimmed `#888888`) | localized desc (`#666666`) | — |
| locked, secret | `?` | `???` | `t("achievement_secret_hint", "シークレットアチーブメント")` | — |

Progress shows `state.unlocked.length / 28` (may disagree with the number of ★ rows if storage contains unknown ids — keep).

### 6.4 Definitions (order = display order; `icon` field is never rendered)
| # | id | secret | ja title | ja desc | en title | en desc |
|---|---|---|---|---|---|---|
| 1 | first_chat | no | 初めまして | 初めて紅莉栖と会話した | Nice to Meet You | Had your first conversation with Kurisu |
| 2 | chat_10 | no | 常連 | 10回会話した | Regular | Had 10 conversations |
| 3 | chat_50 | no | 親友 | 50回会話した | Close Friend | Had 50 conversations |
| 4 | chat_100 | no | ラボメン | 100回会話した | Lab Member | Had 100 conversations |
| 5 | first_angry | no | 怒らせた | 初めて紅莉栖を怒らせた | You Made Her Mad | Made Kurisu angry for the first time |
| 6 | first_blush | no | 照れさせた | 初めて紅莉栖を照れさせた | You Made Her Blush | Made Kurisu blush for the first time |
| 7 | first_smile | no | 笑顔 | 初めて紅莉栖を笑顔にした | Smile | Made Kurisu smile for the first time |
| 8 | first_normal | no | 平常心 | 初めて紅莉栖の平常な姿を見た | Calm & Collected | Saw Kurisu in her normal state for the first time |
| 9 | first_sad | no | 悲しませた | 初めて紅莉栖を悲しませた | You Made Her Sad | Made Kurisu sad for the first time |
| 10 | first_surprised | no | 驚かせた | 初めて紅莉栖を驚かせた | Surprise! | Surprised Kurisu for the first time |
| 11 | first_wink | no | ウインク | 初めて紅莉栖にウインクされた | Wink | Got a wink from Kurisu for the first time |
| 12 | first_disgust | no | 呆れさせた | 初めて紅莉栖を呆れさせた | Disgusted | Disgusted Kurisu for the first time |
| 13 | first_smug | no | ドヤ顔 | 初めて紅莉栖のドヤ顔を見た | Smug Face | Saw Kurisu's smug face for the first time |
| 14 | first_thinking | no | 考えさせた | 初めて紅莉栖を考え込ませた | Deep Thought | Made Kurisu think deeply for the first time |
| 15 | first_panic | no | パニック | 初めて紅莉栖をパニックにした | Panic! | Made Kurisu panic for the first time |
| 16 | all_emotions | no | 感情マスター | 全ての感情を引き出した | Emotion Master | Drew out every emotion |
| 17 | night_owl | no | 夜更かし | 深夜(0-4時)に会話した | Night Owl | Chatted late at night (0-4 AM) |
| 18 | early_bird | no | 早起き | 朝(5-7時)に会話した | Early Bird | Chatted in the morning (5-7 AM) |
| 19 | streak_3 | no | 3日連続 | 3日連続で起動した | 3-Day Streak | Launched 3 days in a row |
| 20 | streak_7 | no | 1週間 | 7日連続で起動した | One Week | Launched 7 days in a row |
| 21 | first_game | no | ゲーマー | 初めてミニゲームをプレイした | Gamer | Played a mini-game for the first time |
| 22 | game_win | no | 勝利 | ミニゲームで勝利した | Victory | Won a mini-game |
| 23 | game_perfect | no | パーフェクト | ミニゲームで全問正解した | Perfect | Got every question right in a mini-game |
| 24 | export_chat | no | 記録者 | 会話記録をエクスポートした | Archivist | Exported the conversation log |
| 25 | import_chat | no | 時間旅行者 | 会話記録をインポートした | Time Traveler | Imported a conversation log |
| 26 | first_event | no | 偶然の出会い | 起動時ランダムイベントが発生した | Chance Encounter | A random event occurred at startup |
| 27 | dev_user | no | 開発者 | 開発コマンドを使用した | Developer | Used a developer command |
| 28 | secret_reading_steiner | **yes** | リーディングシュタイナー | 世界線を超えた秘密を発見した | Reading Steiner | Discovered a secret that transcends world lines |

Keys: `ach_<id>_title` / `ach_<id>_desc` (ja/en values above are from the locale JSON; code defaults in `achievement-manager.ts:41-70` are identical for ja). `achievement_secret_hint`: ja `シークレットアチーブメント`, en `Secret Achievement`.

### 6.5 Where it is opened (parent)
- Desktop: `visible = achievementsOpen`, opened by StatusPanel's `ACHIEVEMENTS` row (`setAchievementsOpen(true)`), `onClose = () => setAchievementsOpen(false)`.
- Mobile: `visible = achievementsOpen || activeOverlay === "achievements"`, opened by ChatPanel's `ACHIEVEMENTS` floating button (`setActiveOverlay("achievements")`), `onClose` sets both `achievementsOpen=false` and `activeOverlay="none"`. Mobile CSS (2588–2707, inside `@media (pointer: coarse), (max-width: 900px)`) makes it a fixed full-screen flex column, smaller fonts, date absolutely positioned bottom-right 10px, close button full-width pill.

---

## 7. Result comment tiers (2513–2522) + minigame library contract

```ts
const RESULT_COMMENT_TIERS: readonly ResultCommentTier[] = ["perfect", "good", "ok", "bad"];

function resultCommentDefaultForKey(key: ResultCommentKey): string {
  for (const tier of RESULT_COMMENT_TIERS) {
    if (RESULT_COMMENT_KEYS[tier] === key) return RESULT_COMMENT_DEFAULTS[tier];
  }
  return RESULT_COMMENT_DEFAULTS.bad;
}
```
From `lib/amadeus/minigame.ts` (all string values verbatim; `resultCommentKey`/`resultEmotion` bodies condensed to one line, logic unchanged — source lines 33, 135–171, 186–197):
```ts
export const TOTAL_QUESTIONS = 10;
export const RESULT_COMMENT_KEYS = { perfect: "mg_result_perfect", good: "mg_result_good", ok: "mg_result_ok", bad: "mg_result_bad" } as const;
export function resultCommentKey(score) { if (score === TOTAL_QUESTIONS) return perfect; if (score >= 6) return good; if (score >= 1) return ok; return bad; }
export function resultEmotion(score)    { if (score === TOTAL_QUESTIONS) return "SMUG"; if (score >= 6) return "SMILE"; if (score >= 1) return "NORMAL"; return "DISGUST"; }
export const RESULT_COMMENT_DEFAULTS = {
  perfect: "「パーフェクト！…まあ、当然の結果よね。」",
  good: "「ふーん、まあまあね。悪くないわ。」",
  ok: "「…もう少し勉強した方がいいんじゃない？」",
  bad: "「はぁ…全滅？本当にシュタゲ好きなの？」",
};
export const MG_WRONG_KEY = "mg_wrong";
export const MG_WRONG_DEFAULT = "はぁ…不正解よ。答えは「%1」だったわ。";
export const MG_UI_DEFAULTS = {
  mg_menu_title: "紅莉栖とシュタゲクイズ",
  mg_menu_desc: "シュタインズ・ゲートから%1問出題！",
  mg_start: "スタート",
  mg_score: "スコア",
  mg_next: "次の問題",
  mg_show_result: "結果を見る",
  mg_result_title: "結果",
  mg_score_result: "%1 / %2 正解",
  mg_retry: "もう一度",
  mg_menu_back: "メニューへ",
} as const;
```
| score | tier / key | emotion | ja text actually displayed (ja.json) | en (en.json) |
|---|---|---|---|---|
| 10 | perfect / `mg_result_perfect` | SMUG | 「パーフェクト！……ま、まあ、私のことだし当然の結果よねっ！あんたにしてはよくやったわ、褒めてあげる！」 | Perfect! ...Well, obviously. |
| 6–9 | good / `mg_result_good` | SMILE | 「ふーん、まあまあね。天才の私には遠く及ばないけど、一般人にしては合格点なんじゃない？」 | Hmm, not bad. Not bad at all. |
| 1–5 | ok / `mg_result_ok` | NORMAL | 「……あんた、もしかして岡部並みに記憶力が退化してんじゃないの？もう少し脳トレしなさい！」 | ...Wait, is your memory deteriorating like Okabe's? You need to do some brain training! |
| 0 | bad / `mg_result_bad` | DISGUST | 「はぁ……全滅？あんたの海馬、ちょっとスキャンしてバグがないか調べた方がいいわね……」 | Sigh... A total wipeout? Do you even like Steins;Gate? |

`generateQuestions()` (minigame.ts 110–129): Fisher–Yates (`for i = n-1 down to 1: j = floor(Math.random()*(i+1)); swap`) over pool indices 0..48; take first `min(10, 49)`; for each, Fisher–Yates over answer positions `[0,1,2,3]`; `a[k] = src.a[order[k]]`; `correct = k` where `order[k] === src.correct` (source `correct` is always 0, i.e. `a0` is the right answer). Returns `{ q, a, correct, c }` (text pairs `[i18nKey, jpDefault]`). Question pool: 49 entries, verbatim in Appendix A (minigame.ts 39–89). The minigame core module is probably owned by another spec; this panel only needs the API above.

---

## 8. MiniGamePanel (2532–2827)

### 8.1 State and refs
| name | type | initial |
|---|---|---|
| `isMobile` | `useIsMobileViewport()` | — |
| `gameState` | `"menu" \| "playing" \| "result"` | `"menu"` |
| `questions` | `MiniGameQuestion[]` | `[]` |
| `currentQuestion` | number | 0 |
| `score` | number | 0 |
| `selectedAnswer` | number | −1 |
| `answerRevealed` | boolean | false |
| `kurisuEmotion` | string | `"SMILE"` |
| `speakIntervalRef` / `speakStopTimeoutRef` | timer ids | null |
| `gameStateRef` / `scoreRef` | mirror latest `gameState` / `score` (assigned during render) | |

Helpers: `tt(pair) = t(pair[0], pair[1], configLanguage)`; `mgT(key) = t(key, MG_UI_DEFAULTS[key], configLanguage)`.

### 8.2 Lip-sync simulation
```text
stopSpeaking():  clearInterval(speakInterval) & null; clearTimeout(speakStop) & null; onSpeakValueChange(0)
startSpeaking(text):
    clear existing interval/timeout (if any)
    ms = max(800, min(3000, text.length * 90))       // text.length = UTF-16 code units
    onSpeakValueChange(Math.random())                 // immediately, value in [0,1)
    speakInterval = setInterval(() => onSpeakValueChange(Math.random()), 120)
    speakStop = setTimeout(() => { clearInterval(speakInterval); speakInterval = null; speakStop = null; onSpeakValueChange(0) }, ms)
```
Spoken only for answer comments (never for the result screen). With ja.json, correct-answer comments are 20–81 UTF-16 units → 1800–3000 ms; ja `mg_wrong` template is 39 units before substitution.

### 8.3 Effects (in declaration order)
1. `[visible, stopSpeaking]` ("QT onVisibleChanged parity"): if visible → `kurisuEmotion = gameStateRef==="playing" ? "THINKING" : gameStateRef==="result" ? resultEmotion(scoreRef) : "SMILE"`; else → `stopSpeaking()` (this also runs once at mount, sending `onSpeakValueChange(0)`).
2. `[visible, kurisuEmotion, onEmotionChange]`: if visible → `onEmotionChange(kurisuEmotion)`. Emotion changes while hidden are not propagated. (On open, React first sends the previous emotion then the recomputed one in the next commit — a negligible flash; sending only the final value is acceptable.)
3. unmount: clear interval/timeout (no callback).
4. while visible: window keydown → `if (key === "Backspace" || key === "Escape") { preventDefault(); onClose(); }`. There is **no** keyboard answering (no 1–4 / arrows / Enter). ChatPanel's Enter/Ctrl+C handler is disabled while `miniGameOpen`.

### 8.4 Actions
```text
startGame():                       // START button and RETRY button
    stopSpeaking()
    questions = generateQuestions(); currentQuestion = 0; score = 0; selectedAnswer = -1; answerRevealed = false
    gameState = "playing"; kurisuEmotion = "THINKING"
    AchievementManager.notifyEvent("game_play")          // first time → unlock first_game (+ toast)

selectAnswer(index):
    if answerRevealed: return
    question = questions[currentQuestion]; if !question: return
    isCorrect = index === question.correct
    correctText = question.a[question.correct] ? tt(question.a[question.correct]) : ""
    selectedAnswer = index; answerRevealed = true
    if isCorrect: score += 1 (functional update); kurisuEmotion = "SMILE"
    else:         kurisuEmotion = "ANGRY"
    comment = isCorrect ? tt(question.c)
                        : t("mg_wrong", MG_WRONG_DEFAULT, lang).replace("%1", correctText)   // first occurrence only
    startSpeaking(comment)

nextQuestion():                    // NEXT / SHOW RESULT button
    stopSpeaking()
    next = currentQuestion + 1
    currentQuestion = next; selectedAnswer = -1; answerRevealed = false
    if next >= questions.length:
        gameState = "result"; kurisuEmotion = resultEmotion(score)
        if score > 0:  notifyEvent("game_win")         // ≥1 correct counts as a "win"
        if score === 10: notifyEvent("game_perfect")
    else:
        kurisuEmotion = "THINKING"

menu back button: gameState = "menu"; kurisuEmotion = "SMILE"   (questions/score untouched)
close (button / Backspace / Escape): onClose()  — game state is kept; reopening resumes (playing → THINKING, result → result emotion)
```
- `score` in `nextQuestion` is the render-time value, already including the last answer (separate click).
- `game_win` and `game_perfect` fire synchronously in sequence; ChatPanel's toast shows only the last one (`game_perfect`) because the second `setAchievementToast` overwrites the first.
- JS `replace("%1", x)` with a string pattern interprets `$&`, `` $` ``, `$'`, `$$` in `x`; verified no answer text in any locale contains `$`, so Rust `replacen("%1", x, 1)` is exact.

### 8.5 Derived render values
```text
activeQuestion = currentQuestion < questions.length ? questions[currentQuestion] : undefined
currentComment = ""
if answerRevealed && activeQuestion:
    currentComment = selectedAnswer === activeQuestion.correct
        ? tt(activeQuestion.c)
        : t("mg_wrong", MG_WRONG_DEFAULT, lang).replace("%1", activeQuestion.a[correct] ? tt(that) : "")
resultKey = resultCommentKey(score)
resultCommentText = t(resultKey, resultCommentDefaultForKey(resultKey), lang)
```
(The comment is recomputed on render, so changing language while revealed re-localizes the bubble but not the speech duration.)

### 8.6 DOM (exact)
```html
<div class="mini-game-panel" style="opacity:{1|0}; pointer-events:{auto|none}; transition:opacity 0.25s ease-in-out;
     position:{fixed if isMobile else absolute}; top:{auto if isMobile else 0}; bottom:0; left:0; right:0; z-index:50; height:{46% if isMobile else 100%}">
  <div class="mini-game-title">MINI GAME</div>

  <div class="mini-game-content">
    <!-- gameState === "menu" -->
    <div class="mini-game-menu">
      <div class="mg-menu-title">{mgT("mg_menu_title")}</div>
      <div class="mg-menu-desc">{mgT("mg_menu_desc").replace("%1", "10")}</div>
      <button type="button" class="mg-start-button">{mgT("mg_start")}</button>              <!-- startGame -->
    </div>

    <!-- gameState === "playing" && activeQuestion -->
    <div class="mini-game-playing">
      <div class="mg-header">
        <span class="mg-question-counter">Q{currentQuestion+1} / 10</span>
        <span class="mg-score-label">{mgT("mg_score")}: {score}</span>                       <!-- ASCII ": " in all languages -->
      </div>
      <div class="mg-question-box">{tt(activeQuestion.q)}</div>
      <div class="mg-answers">
        <!-- 4 buttons, key `${activeQuestion.q[0]}-${index}` -->
        <button type="button" class="mg-answer[ revealed][ correct| wrong]" disabled="{answerRevealed}">  <!-- selectAnswer(index) -->
          <span class="mg-answer-text">{tt(answer)}</span>
        </button>
      </div>
      <div class="mg-spacer"></div>
      <!-- only when answerRevealed -->
      <button type="button" class="mg-next-button">{currentQuestion + 1 >= 10 ? mgT("mg_show_result") : mgT("mg_next")}</button>  <!-- nextQuestion -->
    </div>

    <!-- gameState === "result" -->
    <div class="mini-game-result">
      <div class="mg-result-title">{mgT("mg_result_title")}</div>
      <div class="mg-score-result">{mgT("mg_score_result").replace("%1", score).replace("%2", "10")}</div>
      <div class="mg-result-comment">{resultCommentText}</div>
      <div class="mg-result-buttons">
        <button type="button" class="mg-retry-button">{mgT("mg_retry")}</button>            <!-- startGame -->
        <button type="button" class="mg-menu-back-button">{mgT("mg_menu_back")}</button>    <!-- menu + SMILE -->
      </div>
    </div>
  </div>

  <!-- answerRevealed && currentComment !== "" -->
  <div class="mg-comment-bubble">{currentComment}</div>

  <button type="button" class="overlay-close-button mg-close-button">{t("close", lang===1?"Close":"閉じる", lang)}</button>
</div>
```
Answer class computation (exact):
```ts
let stateClass = "";
if (answerRevealed) { if (index === correct) stateClass = " correct"; else if (index === selectedAnswer) stateClass = " wrong"; }
className = `mg-answer${answerRevealed ? " revealed" : ""}${stateClass}`;
// → "mg-answer" | "mg-answer revealed" | "mg-answer revealed correct" | "mg-answer revealed wrong"
```
- Counter text uses the constant `TOTAL_QUESTIONS` (10); the end-of-game test uses `questions.length` (also 10). The playing block renders only if `activeQuestion` exists.
- `MINI GAME` title is not localized. `mg-menu-desc` example ja: `シュタインズ・ゲートから10問出題！`; `mg-score-result` ja: `3 / 10 正解`.

### 8.7 Parent integration (ChatPanel, part2, lines 941–943, 1832–1987)
- `miniGameOpen` (false), `miniGameEmotion` ("NORMAL"), `miniSpeak` (0) live in ChatPanel. Panel props: `visible=miniGameOpen`, `onClose = () => { setMiniGameOpen(false); setActiveOverlay?.("none") }` (desktop ChatPanel gets no `setActiveOverlay`), `onEmotionChange = setMiniGameEmotion`, `onSpeakValueChange = setMiniSpeak`.
- `Live2DCanvas emotion = miniGameOpen ? miniGameEmotion : currentEmotionTag`, `lipSyncValue = miniGameOpen ? miniSpeak : …`, `spokenChar = miniGameOpen ? "" : …`.
- Wrapper `div.live2d-shift` gets `live2d-shifted` while open: desktop `translateX(28%)` (450 ms cubic-bezier(0.645,0.045,0.355,1)); mobile `translateY(-14dvh) scale(0.85)`.
- While open, ChatPanel hides the input panel, the `MINI GAME` button and the `ACHIEVEMENTS` (mobile) button (`chatState === "inputReady" && !miniGameOpen`).
- Mini-game emotions do **not** feed emotion achievements (only chat emotions call `notifyEvent("emotion")`).
- The `"minigame"` value of the parent `OverlayPanel` union is never set anywhere (dead).
- Desktop has **no** global-overlay flag while the mini-game is open, so Tab still opens the side menu over it (V1 behavior).

### 8.8 Layout (CSS 1253–1511 desktop, 2710–2925 mobile)
Desktop (inside `.chat-panel-root`, 1920×1045): transparent panel (no `.mini-game-panel` desktop rule) over the right-shifted Kurisu; title at (40,40) 64px `#ff9900`; content box left 60, top 140, bottom 80, width 50%; answers in a 2-column grid (65px high, ellipsis); bubble at right 120 / bottom 180, width 600, `rgba(45,15,0,0.9)` with `#ff9900` border radius 12; close button bottom-right (z-index 4). Correct `#004400`, wrong `#440000`, hover border `#ff9900` only when not revealed.
Mobile: bottom sheet `position: fixed !important; height: 46% !important; z-index: 3000 !important`, rounded top, `rgba(12,6,0,0.96)`; bubble floats above the sheet (`top: -125px`); close button becomes a small pill at the sheet's top-right; answers 38 px high with wrapped centered text; spacer hidden; next button full width. Inline styles and the `!important` CSS agree.

---

## 9. ChangeLogPanel (2835–2892) — desktop only

### 9.1 Data (RealAmadeusApp.tsx 292–323; values verbatim, reformatted one entry per line)
```ts
const CHANGE_LOG_DATA = [
  { version: "Version 1.4N",  date: "2026. 08. 01", key: "changelog_v14" },
  { version: "Version 1.3",   date: "2026. 06. 16", key: "changelog_v13" },
  { version: "Version 1.2",   date: "2026. 05. 04", key: "changelog_v12" },
  { version: "Version 1.1",   date: "2026. 03. 21", key: "changelog_v11" },
  { version: "Version 1.0.1", date: "2026. 02. 23", key: "changelog_v101" },
  { version: "Version 1.0",   date: "2026. 02. 22", key: "changelog_v10" },
];
```
Body text = `t(entry.key, "", configLanguage).split("\n")` → one `<li>` per line (empty default → a missing key would render the key name itself as a single item; all locales have all six keys). ja/en texts in Appendix B.

### 9.2 Behavior
While visible: window keydown → Backspace → `preventDefault(); onClose()`. No Escape, no keyboard scrolling. Scroll position of `.change-log-content` persists across openings (always mounted).

### 9.3 DOM (exact)
```html
<div style="opacity; pointer-events; transition:opacity 0.3s ease-in-out; position:absolute; top:0; left:0; right:0; bottom:0; z-index:2000">
  <div class="overlay-panel changelog-panel">
    <img src="/resources/images/Amadeus_BG.png" alt="" class="overlay-bg">
    <div class="overlay-header">
      <h2>CHANGE LOG</h2>
      <div class="line"></div>
    </div>
    <div class="change-log-content">
      <!-- per entry (key = version) -->
      <div class="change-log-entry">
        <div class="change-log-head">
          <div class="version">{version}</div>
          <div class="date">{date}</div>
        </div>
        <div class="line thin"></div>
        <ul>
          <li>{line}</li>          <!-- key `${version}-${idx}` -->
        </ul>
      </div>
    </div>
    <button type="button" class="overlay-close-button">{t("close", …)}</button>
  </div>
</div>
```
`CHANGE LOG`, versions and dates are not localized.

---

## 10. HelpPanel (2900–2960) and HelpEntry (2962–2970) — desktop only

### 10.1 Behavior
While visible: window keydown → Backspace → `preventDefault(); onClose()`.

### 10.2 DOM (exact)
```html
<div style="opacity; pointer-events; transition:opacity 0.3s ease-in-out; position:absolute; top:0; left:0; right:0; bottom:0; z-index:2000">
  <div class="overlay-panel help-panel">
    <img src="/resources/images/Amadeus_BG.png" alt="" class="overlay-bg">
    <div class="overlay-header">
      <h2>HELP</h2>
      <div class="line"></div>
    </div>
    <div class="help-content">
      <!-- 5 × HelpEntry -->
    </div>
    <button type="button" class="overlay-close-button">{t("close", …)}</button>
  </div>
</div>
```
`HelpEntry`:
```html
<div class="help-entry">
  <div class="title">{title}</div>
  <div class="line thin"></div>
  <div class="description">{description}</div>
</div>
```

### 10.3 Entries (exact calls and what is actually shown)
| # | title expression | shown title | description expression | shown desc ja / en |
|---|---|---|---|---|
| 1 | `t("help_right_click_title", lang===1 ? "Tab / Right Click" : "Tab / 右クリック", lang)` | key missing in all locales → lang 1: `Tab / Right Click`; every other language: `Tab / 右クリック` | `t("help_toggle_menu", lang===1 ? "Open/Close Menu" : "メニュー開閉", lang)` | メニュー開閉 / Toggle Menu |
| 2 | literal `Backspace` | `Backspace` | `t("help_save_close", lang===1 ? "Apply and Close" : "保存して閉じる", lang)` | 保存して閉じる / Save and Close |
| 3 | `t("help_wasd", lang===1 ? "WASD / Arrows" : "WASD / ↑←↓→", lang)` | key missing → lang 1: `WASD / Arrows`; others: `WASD / ↑←↓→` | `t("help_select_item", lang===1 ? "Navigate Items" : "項目選択", lang)` | 項目選択 / Select Item |
| 4 | literal `Enter` | `Enter` | `t("help_confirm_advance", lang===1 ? "Select / Advance Conv." : "決定 / 会話を進める", lang)` | 決定 / 会話を進める / Confirm / Advance |
| 5 | literal `Ctrl+C` | `Ctrl+C` | `t("help_cancel_chat", lang===1 ? "Cancel Conversation" : "会話をキャンセル", lang)` | 会話をキャンセル / Cancel conversation |

(The inline English defaults differ from en.json, but en.json wins because the keys exist.) A similar key `help_right_click` exists in all locales (`右クリック` / `Right Click` …) but is not used. Other locales' description strings: Appendix C.

---

## 11. ConfirmationDialog (2980–3059) — desktop only

### 11.1 State and effects
- `selectedIndex` initial **1** (0 = No, 1 = Yes).
- On `visible` becoming true: `setTimeout(() => setSelectedIndex(1), 0)` (cleared if hidden first). The index is **not** reset on close.
- While visible: window keydown (re-registered when `selectedIndex`/callbacks change, so it always reads the current index):
  | key | action | preventDefault |
  |---|---|---|
  | `ArrowLeft` or `key.toLowerCase() === "a"` | `selectedIndex = 0` | no |
  | `ArrowRight` or `key.toLowerCase() === "d"` | `selectedIndex = 1` | no |
  | `Backspace` | `onCancel()` | yes |
  | `Enter` | `selectedIndex === 1 ? onConfirm() : onCancel()` | yes |
  No Escape, no Up/Down, no Tab handling. (preventDefault on Enter also suppresses the implicit click of whatever button had focus, e.g. the menu label that opened the dialog.)
- Mouse: `onMouseEnter` No → 0, Yes → 1; click No → `onCancel`, click Yes → `onConfirm` (clicks ignore `selectedIndex`).

### 11.2 DOM (exact)
```html
<div style="opacity; pointer-events; transition:opacity 0.3s ease-in-out; position:absolute; top:0; left:0; right:0; bottom:0; z-index:3000">
  <div class="dialog-overlay">
    <div class="confirmation-dialog">
      <div class="message">{message}</div>
      <button type="button" class="choice no[ selected]">{t("no", lang===1 ? "No" : "いいえ", lang)}</button>
      <button type="button" class="choice yes[ selected]">{t("yes", lang===1 ? "Yes" : "はい", lang)}</button>
    </div>
  </div>
</div>
```
Class strings: `"choice no"` / `"choice no selected"`, `"choice yes"` / `"choice yes selected"`. CSS: 800×300 box `#1a1a1a` with `#ffc900` border; message 36px centered at top 70; buttons 200×60 at top 220 (No left 100, Yes right 100); selected = `#ffc900` background, black text.

### 11.3 Instances (parent, part4 lines 4670–4702)
Two always-mounted instances share the parent's `confirmMessage` state (set when the menu item is executed, so it does not re-localize while open; the Yes/No labels do):
- **Logout** (`activeOverlay === "confirmLogout"`, message `t("logout_confirm", lang===1 ? "Do you want to log out?" : "ログアウトしますか？")`): confirm → `MemoryManager.setUserName("")` (**no-op**, see §1.5), `setAppState("login")`, `setActiveOverlay("none")`, `setIsTabActive(false)`, `setMenuOpen(false)`; cancel → `setActiveOverlay("none")`.
- **Shutdown** (`activeOverlay === "confirmShutdown"`, message `t("exit_confirm", lang===1 ? "Do you want to exit Real Amadeus?" : "リアルアマデウスを終了しますか？")`): confirm → `setActiveOverlay("none")`, `setMenuOpen(false)`, `window.close()`, then after 100 ms `if (history.length > 1) history.back()`, then after a further 300 ms `location.href = "https://dsuv.elvelt.com/amadeus"`; cancel → `setActiveOverlay("none")`.

---

## 12. Parent wiring summary (where each panel lives)

**Desktop tree** (`.real-amadeus-root > .window-frame > .viewport` [1920×1080, `transform: scale(uiScale)`]), in DOM order:
1. `BackLogPanel` — `visible = activeOverlay === "backlog"`, `entries = backLogEntries`, `onClose → setActiveOverlay("none")`.
2. `.system-panel` → `LoginPanel` (appState `login`, `onAccepted → setAppState("boot")`) | `LoadingPanel` (`boot`, `onFinished → setAppState("chat")`) | `ChatPanel` (`chat`; contains `MiniGamePanel`).
3. `.menu-panel` (side menu: 0 BACKLOG, 1 CONFIG, 2 STATUS, 3 WINDOW/FULL SCREEN, 4 CHANGE LOG, 5 LOGOUT, 6 HELP, 7 SHUT DOWN, 8 CLOSE MENU).
4. `ConfigPanel` (part4).
5. `StatusPanel` — `visible = activeOverlay === "status"`, `isLoggedIn = appState === "chat"`, `achievementsOpen`, `onOpenAchievements → setAchievementsOpen(true)`, `onClose → setActiveOverlay("none")`.
6. `AchievementPanel` — `visible = achievementsOpen`, `onClose → setAchievementsOpen(false)`.
7. `ChangeLogPanel` — `activeOverlay === "changeLog"`.
8. `HelpPanel` — `activeOverlay === "help"`.
9. `ConfirmationDialog` (logout), 10. `ConfirmationDialog` (shutdown).

**Mobile tree** (`.real-amadeus-root.mobile-root`): `MobileLoginPanel` / `MobileLoadingPanel` / `ChatPanel` (with `setActiveOverlay`) inside `.mobile-stage`; then optional `MobileSettingsPanel`; `BackLogPanel` (`activeOverlay === "backlog"`); `AchievementPanel` (`achievementsOpen || activeOverlay === "achievements"`). Status, ChangeLog, Help and Confirmation are **not rendered** on mobile. Switching between trees remounts all panels (state lost).

Global-handler interplay (owned by part2/part4, listed so the port keeps the same routing):
- Main handler (`window keydown`): F11 fullscreen toggle; F3 auto-mode toggle; Tab toggles menu only when `activeOverlay === "none"`; menu navigation (WASD/arrows/Enter/Backspace) only when `menuOpen && activeOverlay === "none"` and the target is not a text input.
- ChatPanel handler: returns early when `menuPanelOpen` (desktop: `isTabActive`) or `miniGameOpen`; otherwise Ctrl+C cancels streaming, Enter submits/advances.
- `achievementsOpen` is separate from `activeOverlay`; on desktop it is only set while `activeOverlay === "status"`, so `isSubPanelOpen` stays true.

## 13. Keyboard summary for this range
| key | when | component | effect | preventDefault |
|---|---|---|---|---|
| Enter | focus in USER ID or PASSWORD | LoginPanel | attemptLogin | no |
| Backspace | BackLog visible | BackLogPanel | onClose | yes |
| Backspace | Status visible and `!achievementsOpen` | StatusPanel | onClose | yes |
| Backspace / Escape | Achievement visible | AchievementPanel | onClose | yes |
| Backspace / Escape | MiniGame visible | MiniGamePanel | onClose | yes |
| Backspace | ChangeLog visible | ChangeLogPanel | onClose | yes |
| Backspace | Help visible | HelpPanel | onClose | yes |
| ArrowLeft / a / A | Confirm visible | ConfirmationDialog | select No | no |
| ArrowRight / d / D | Confirm visible | ConfirmationDialog | select Yes | no |
| Backspace | Confirm visible | ConfirmationDialog | onCancel | yes |
| Enter | Confirm visible | ConfirmationDialog | confirm if Yes selected, else cancel | yes |
Key auto-repeat simply re-invokes the (idempotent) close callbacks.

## 14. Persistence touched (indirectly)
| key | written by | when | format |
|---|---|---|---|
| `RealAmadeus.Memory` | `MemoryManager.setUserName("Salieri")` | successful desktop login | whole memory JSON with `"userName":"Salieri"` |
| `amadeus_achievements` | `AchievementManager.unlock` via `notifyEvent("game_play" / "game_win" / "game_perfect")` | START/RETRY; game end | `{unlocked, dates, seenEmotions, lastLaunchDate, launchStreak}` |
| `Config_SkipLoading` | read only (`getInt`, default 0) | LoadingPanel mount | `"0"` / `"1"` |
| `Config_Language` | read (via parent `configLanguage`; AchievementManager reads it directly) | render | integer string |
Keys and formats must stay byte-compatible with V1 (same origin users keep data).

## 15. CSS reference (globals.css line ranges; copy the whole file verbatim)
| lines | rules |
|---|---|
| 3–65 | @font-face (MateSC, Eurostile, DepartureMono, Liberation Sans, MS Mincho, Yu Mincho, Noto Serif JP ×2, Noto Serif, Noto Serif CJK KR) |
| 67–95 | global reset: `* { user-select: none }`, inputs selectable, no focus outlines |
| 160–202 | `.system-panel`, `.menu-wallpaper`, `.title-bar`, `.system-bg` (login/loading container) |
| 204–296 | login (`.login-panel` … `.login-error`) |
| 298–321 | `.loading-panel` (padding 70px 0 0 60px), `.boot-text` (`#ff0000`, DepartureMono 18px, line-height 1.2, pre-wrap), `.boot-logo` (900×900 centered) |
| 726–748 | `.overlay-panel` (z 110, overlay-fade 250ms), `.overlay-panel.closing` (unused), `@keyframes overlay-fade` |
| 750–822 | `.overlay-bg`, `.overlay-header` (+ status/help/changelog variants), `.line`, `.line.thin`, `.overlay-close-button` (210×70 at right 100 / bottom 60, `#464646`, 32px) |
| 824–896 | backlog content/rows + custom scrollbars (white track, `#ff9900` thumb, 10px) for backlog/changelog/help |
| 898–1037 | status clock/grid/rows/value colors/credits/achievement row |
| 1039–1173 | achievement panel/list/rows/icons/date (list scrollbar hidden) |
| 1175–1214 | achievement toast (ChatPanel) |
| 1216–1251 | `.live2d-shift(.live2d-shifted)`, `.mini-game-button`, `.mobile-achievements-trigger-button { display:none }` |
| 1253–1511 | mini game |
| 1513–1560 | change log |
| 1562–1590 | help |
| 1592–1657 | `.dialog-overlay` (z 130, rgba(26,26,26,0.7), overlay-fade), `.dialog-overlay.closing` (unused), confirmation dialog |
| 2051–2056, 2308–2315 | `.mobile-root` base |
| 2459–2581 | mobile backlog overrides (not inside a media query) |
| 2583–2965 | `@media (pointer: coarse), (max-width: 900px)` mobile achievements + mini game + floating buttons + live2d shift |
`.closing` classes are never applied by the web code (QT leftovers). `.backlog-panel-wrapper` and desktop `.mini-game-panel` have no rules.

---

## Appendix A — Question pool (verbatim, `lib/amadeus/minigame.ts` lines 39–89)

49 entries; `[i18nKey, jpDefault]` pairs; `correct: 0` everywhere (a0 is the right answer). Displayed strings come from the locale JSON (`mgq_*` keys exist in all 13 locales); the JP defaults below are fallbacks only.

```ts
export const QUESTION_POOL: readonly MiniGameEntry[] = [
  { q: ["mgq_drink_q", "紅莉栖の好きな飲み物は？"], a: [["mgq_drink_a0", "Dr. Pepper"], ["mgq_drink_a1", "コーラ"], ["mgq_drink_a2", "コーヒー"], ["mgq_drink_a3", "紅茶"]], correct: 0, c: ["mgq_drink_c", "べ、別に好きってわけじゃないんだから！"] },
  { q: ["mgq_fg8_q", "未来ガジェット8号機の名前は？"], a: [["mgq_fg8_a0", "電話レンジ（仮）"], ["mgq_fg8_a1", "ビット粒子砲"], ["mgq_fg8_a2", "サイリウム・セーバー"], ["mgq_fg8_a3", "モアッド・スネーク"]], correct: 0, c: ["mgq_fg8_c", "電話レンジ（仮）。命名センスは問わないで。"] },
  { q: ["mgq_fg1_q", "未来ガジェット1号機の名前は？"], a: [["mgq_fg1_a0", "ビット粒子砲"], ["mgq_fg1_a1", "タケコプカメラー"], ["mgq_fg1_a2", "サイリウム・セーバー"], ["mgq_fg1_a3", "攻殻機動迷彩ボール"]], correct: 0, c: ["mgq_fg1_c", "ビット粒子砲。光るだけだけどね。"] },
  { q: ["mgq_bday_kurisu_q", "牧瀬紅莉栖の誕生日は？"], a: [["mgq_bday_kurisu_a0", "7月25日"], ["mgq_bday_kurisu_a1", "7月28日"], ["mgq_bday_kurisu_a2", "12月14日"], ["mgq_bday_kurisu_a3", "2月1日"]], correct: 0, c: ["mgq_bday_kurisu_c", "7月25日。…なんで知ってるの？"] },
  { q: ["mgq_bday_okabe_q", "岡部倫太郎の誕生日は？"], a: [["mgq_bday_okabe_a0", "12月14日"], ["mgq_bday_okabe_a1", "7月25日"], ["mgq_bday_okabe_a2", "5月19日"], ["mgq_bday_okabe_a3", "9月27日"]], correct: 0, c: ["mgq_bday_okabe_c", "12月14日。鳳凰院凶真の誕生日でもあるわ。"] },
  { q: ["mgq_bday_mayuri_q", "椎名まゆりの誕生日は？"], a: [["mgq_bday_mayuri_a0", "2月1日"], ["mgq_bday_mayuri_a1", "7月7日"], ["mgq_bday_mayuri_a2", "11月9日"], ["mgq_bday_mayuri_a3", "4月3日"]], correct: 0, c: ["mgq_bday_mayuri_c", "2月1日。絶対に忘れちゃダメなんだから。"] },
  { q: ["mgq_meter_q", "世界線の変動率を測る装置の名前は？"], a: [["mgq_meter_a0", "ダイバージェンスメーター"], ["mgq_meter_a1", "リーディングシュタイナー"], ["mgq_meter_a2", "タイムリープマシン"], ["mgq_meter_a3", "電話レンジ（仮）"]], correct: 0, c: ["mgq_meter_c", "ダイバージェンスメーター。岡部が作ったのよ。"] },
  { q: ["mgq_hououin_q", "岡部倫太郎のもう一つの名は？"], a: [["mgq_hououin_a0", "鳳凰院凶真"], ["mgq_hououin_a1", "クリスティーナ"], ["mgq_hououin_a2", "助手"], ["mgq_hououin_a3", "The Zombie"]], correct: 0, c: ["mgq_hououin_c", "鳳凰院凶真。…中二病だけど。"] },
  { q: ["mgq_sg_div_q", "「シュタインズ・ゲート」世界線の変動率は？"], a: [["mgq_sg_div_a0", "1.048596%"], ["mgq_sg_div_a1", "0.571024%"], ["mgq_sg_div_a2", "1.130426%"], ["mgq_sg_div_a3", "1.064756%"]], correct: 0, c: ["mgq_sg_div_c", "1.048596%。当然知ってるわよね？"] },
  { q: ["mgq_handle_q", "紅莉栖の＠ちゃんねるハンドルネームは？"], a: [["mgq_handle_a0", "栗悟飯とカメハメ波"], ["mgq_handle_a1", "クリスティーナ"], ["mgq_handle_a2", "助手"], ["mgq_handle_a3", "The Zombie"]], correct: 0, c: ["mgq_handle_c", "栗悟飯とカメハメ波よ。…って、あんた＠ちゃんねる見てるんじゃないでしょうね？！"] },
  { q: ["mgq_univ_q", "紅莉栖が在籍していた大学は？"], a: [["mgq_univ_a0", "ヴィクトル・コンドリア大学"], ["mgq_univ_a1", "東京大学"], ["mgq_univ_a2", "MIT"], ["mgq_univ_a3", "ケンブリッジ大学"]], correct: 0, c: ["mgq_univ_c", "ヴィクトル・コンドリア大学。脳科学の研究よ。"] },
  { q: ["mgq_lab_q", "秋葉原にあるラボの名前は？"], a: [["mgq_lab_a0", "未来ガジェット研究所"], ["mgq_lab_a1", "SERN"], ["mgq_lab_a2", "円卓会議"], ["mgq_lab_a3", "ブラウン管工房"]], correct: 0, c: ["mgq_lab_c", "未来ガジェット研究所。オンボロビルの2階よ。"] },
  { q: ["mgq_timeleap_q", "記憶を過去に送る機械の名前は？"], a: [["mgq_timeleap_a0", "タイムリープマシン"], ["mgq_timeleap_a1", "タイムマシン"], ["mgq_timeleap_a2", "Dメール"], ["mgq_timeleap_a3", "ダイバージェンスメーター"]], correct: 0, c: ["mgq_timeleap_c", "タイムリープマシン。記憶を36バイトに圧縮するの。"] },
  { q: ["mgq_labmem_q", "紅莉栖のラボメンナンバーは？"], a: [["mgq_labmem_a0", "004"], ["mgq_labmem_a1", "001"], ["mgq_labmem_a2", "003"], ["mgq_labmem_a3", "008"]], correct: 0, c: ["mgq_labmem_c", "004。私よ。何か文句ある？"] },
  { q: ["mgq_bday_daru_q", "橋田至（ダル）の誕生日は？"], a: [["mgq_bday_daru_a0", "5月19日"], ["mgq_bday_daru_a1", "6月6日"], ["mgq_bday_daru_a2", "8月30日"], ["mgq_bday_daru_a3", "9月27日"]], correct: 0, c: ["mgq_bday_daru_c", "5月19日。スーパーハカーの誕生日よ。"] },
  { q: ["mgq_bday_moeka_q", "桐生萌郁の誕生日は？"], a: [["mgq_bday_moeka_a0", "6月6日"], ["mgq_bday_moeka_a1", "5月19日"], ["mgq_bday_moeka_a2", "8月30日"], ["mgq_bday_moeka_a3", "11月9日"]], correct: 0, c: ["mgq_bday_moeka_c", "6月6日。シャイニング・フィンガーね。"] },
  { q: ["mgq_bday_ruka_q", "漆原るかの誕生日は？"], a: [["mgq_bday_ruka_a0", "8月30日"], ["mgq_bday_ruka_a1", "9月27日"], ["mgq_bday_ruka_a2", "7月7日"], ["mgq_bday_ruka_a3", "4月3日"]], correct: 0, c: ["mgq_bday_ruka_c", "8月30日。性別を間違えちゃダメよ。"] },
  { q: ["mgq_bday_suzuha_q", "阿万音鈴羽の誕生日は？"], a: [["mgq_bday_suzuha_a0", "9月27日"], ["mgq_bday_suzuha_a1", "8月30日"], ["mgq_bday_suzuha_a2", "5月31日"], ["mgq_bday_suzuha_a3", "11月2日"]], correct: 0, c: ["mgq_bday_suzuha_c", "9月27日。バイト戦士の誕生日よ。"] },
  { q: ["mgq_bday_faris_q", "フェイリス・ニャンニアンの誕生日は？"], a: [["mgq_bday_faris_a0", "4月3日"], ["mgq_bday_faris_a1", "2月1日"], ["mgq_bday_faris_a2", "7月7日"], ["mgq_bday_faris_a3", "11月9日"]], correct: 0, c: ["mgq_bday_faris_c", "4月3日。ニャニャン…私はやらないわよ。"] },
  { q: ["mgq_bday_nae_q", "天王寺綯の誕生日は？"], a: [["mgq_bday_nae_a0", "11月9日"], ["mgq_bday_nae_a1", "11月2日"], ["mgq_bday_nae_a2", "7月7日"], ["mgq_bday_nae_a3", "12月14日"]], correct: 0, c: ["mgq_bday_nae_c", "11月9日。いじめちゃダメよ。"] },
  { q: ["mgq_bday_maho_q", "比屋定真帆の誕生日は？"], a: [["mgq_bday_maho_a0", "11月2日"], ["mgq_bday_maho_a1", "11月9日"], ["mgq_bday_maho_a2", "8月23日"], ["mgq_bday_maho_a3", "6月13日"]], correct: 0, c: ["mgq_bday_maho_c", "11月2日。私の同僚…みたいなものよ。"] },
  { q: ["mgq_fg2_q", "未来ガジェット2号機の名前は？"], a: [["mgq_fg2_a0", "タケコプカメラー"], ["mgq_fg2_a1", "ビット粒子砲"], ["mgq_fg2_a2", "サイリウム・セーバー"], ["mgq_fg2_a3", "モアッド・スネーク"]], correct: 0, c: ["mgq_fg2_c", "タケコプカメラー。飛べないけどね。"] },
  { q: ["mgq_fg3_q", "未来ガジェット3号機の名前は？"], a: [["mgq_fg3_a0", "もしかしてオラオラですかーっ!?"], ["mgq_fg3_a1", "モアッド・スネーク"], ["mgq_fg3_a2", "バーローのアレ"], ["mgq_fg3_a3", "びっくりメガネちゃん"]], correct: 0, c: ["mgq_fg3_c", "「もしかしてオラオラですかーっ!?」…どういう名前よ。"] },
  { q: ["mgq_fg4_q", "未来ガジェット4号機の名前は？"], a: [["mgq_fg4_a0", "モアッド・スネーク"], ["mgq_fg4_a1", "ビット粒子砲"], ["mgq_fg4_a2", "攻殻機動迷彩ボール"], ["mgq_fg4_a3", "ホーミング・ディーヴァ"]], correct: 0, c: ["mgq_fg4_c", "モアッド・スネーク。ただのヘビよ。"] },
  { q: ["mgq_fg5_q", "未来ガジェット5号機の名前は？"], a: [["mgq_fg5_a0", "またつまらぬものを繋げてしまった"], ["mgq_fg5_a1", "タケコプカメラー"], ["mgq_fg5_a2", "サイリウム・セーバー"], ["mgq_fg5_a3", "ダーリンのばかぁ"]], correct: 0, c: ["mgq_fg5_c", "「またつまらぬものを繋げてしまった」…長いわよ。"] },
  { q: ["mgq_fg6_q", "未来ガジェット6号機の名前は？"], a: [["mgq_fg6_a0", "サイリウム・セーバー"], ["mgq_fg6_a1", "ビット粒子砲"], ["mgq_fg6_a2", "攻殻機動迷彩ボール"], ["mgq_fg6_a3", "電話レンジ（仮）"]], correct: 0, c: ["mgq_fg6_c", "サイリウム・セーバー。“改”もあるわよ。"] },
  { q: ["mgq_fg7_q", "未来ガジェット7号機の名前は？"], a: [["mgq_fg7_a0", "攻殻機動迷彩ボール"], ["mgq_fg7_a1", "サイリウム・セーバー"], ["mgq_fg7_a2", "モアッド・スネーク"], ["mgq_fg7_a3", "びっくりメガネちゃん"]], correct: 0, c: ["mgq_fg7_c", "攻殻機動迷彩ボール。要するに透明化ね。"] },
  { q: ["mgq_fg9_q", "未来ガジェット9号機の名前は？"], a: [["mgq_fg9_a0", "帰還の女神（ホーミング・ディーヴァ）"], ["mgq_fg9_a1", "びっくりメガネちゃん"], ["mgq_fg9_a2", "バーローのアレ"], ["mgq_fg9_a3", "ダーリンのばかぁ"]], correct: 0, c: ["mgq_fg9_c", "泣き濡れし女神の帰還 ホーミング・ディーヴァよ。"] },
  { q: ["mgq_fg10_q", "未来ガジェット10号機の名前は？"], a: [["mgq_fg10_a0", "びっくりメガネちゃん"], ["mgq_fg10_a1", "バーローのアレ"], ["mgq_fg10_a2", "ホーミング・ディーヴァ"], ["mgq_fg10_a3", "モアッド・スネーク"]], correct: 0, c: ["mgq_fg10_c", "びっくりメガネちゃん。ただのメガネよ。"] },
  { q: ["mgq_div_alpha_q", "α世界線（本編開始）のダイバージェンスは？"], a: [["mgq_div_alpha_a0", "0.571024%"], ["mgq_div_alpha_a1", "1.048596%"], ["mgq_div_alpha_a2", "1.130426%"], ["mgq_div_alpha_a3", "-0.275349%"]], correct: 0, c: ["mgq_div_alpha_c", "0.571024%。まゆりが…なんでもないわ。"] },
  { q: ["mgq_div_omega_q", "フェイリスENDのΩ世界線の変動率は？"], a: [["mgq_div_omega_a0", "-0.275349%"], ["mgq_div_omega_a1", "0.000000%"], ["mgq_div_omega_a2", "0.456903%"], ["mgq_div_omega_a3", "0.571024%"]], correct: 0, c: ["mgq_div_omega_c", "-0.275349%。負の値、フェイリスの世界線よ。"] },
  { q: ["mgq_op_skuld_q", "ゼロで紅莉栖を救う作戦の名前は？"], a: [["mgq_op_skuld_a0", "オペレーション・スクルド"], ["mgq_op_skuld_a1", "オペレーション・ウルド"], ["mgq_op_skuld_a2", "オペレーション・アークライト"], ["mgq_op_skuld_a3", "オペレーション・アマテラス"]], correct: 0, c: ["mgq_op_skuld_c", "オペレーション・スクルド。運命の三姉妹の名よ。"] },
  { q: ["mgq_op_urd_q", "次のうち、作中に登場するオペレーション名は？"], a: [["mgq_op_urd_a0", "オペレーション・ウルド"], ["mgq_op_urd_a1", "オペレーション・オーディン"], ["mgq_op_urd_a2", "オペレーション・トール"], ["mgq_op_urd_a3", "オペレーション・ロキ"]], correct: 0, c: ["mgq_op_urd_c", "オペレーション・ウルド。これも三姉妹の名よ。"] },
  { q: ["mgq_labmem_okabe_q", "岡部のラボメンナンバーは？"], a: [["mgq_labmem_okabe_a0", "001"], ["mgq_labmem_okabe_a1", "002"], ["mgq_labmem_okabe_a2", "003"], ["mgq_labmem_okabe_a3", "008"]], correct: 0, c: ["mgq_labmem_okabe_c", "001。創設者よ。鳳凰院凶真だけど。"] },
  { q: ["mgq_labmem_mayuri_q", "まゆりのラボメンナンバーは？"], a: [["mgq_labmem_mayuri_a0", "002"], ["mgq_labmem_mayuri_a1", "001"], ["mgq_labmem_mayuri_a2", "004"], ["mgq_labmem_mayuri_a3", "006"]], correct: 0, c: ["mgq_labmem_mayuri_c", "002。まゆりの番号よ。"] },
  { q: ["mgq_labmem_daru_q", "ダルのラボメンナンバーは？"], a: [["mgq_labmem_daru_a0", "003"], ["mgq_labmem_daru_a1", "004"], ["mgq_labmem_daru_a2", "005"], ["mgq_labmem_daru_a3", "007"]], correct: 0, c: ["mgq_labmem_daru_c", "003。スーパーハカーね。"] },
  { q: ["mgq_labmem_suzuha_q", "鈴羽のラボメンナンバーは？"], a: [["mgq_labmem_suzuha_a0", "008"], ["mgq_labmem_suzuha_a1", "006"], ["mgq_labmem_suzuha_a2", "007"], ["mgq_labmem_suzuha_a3", "005"]], correct: 0, c: ["mgq_labmem_suzuha_c", "008。最後のラボメンよ。"] },
  { q: ["mgq_blood_kurisu_q", "紅莉栖の血液型は？"], a: [["mgq_blood_kurisu_a0", "A型"], ["mgq_blood_kurisu_a1", "B型"], ["mgq_blood_kurisu_a2", "O型"], ["mgq_blood_kurisu_a3", "AB型"]], correct: 0, c: ["mgq_blood_kurisu_c", "A型。何か聞きたいことでも？"] },
  { q: ["mgq_height_kurisu_q", "紅莉栖の身長は？"], a: [["mgq_height_kurisu_a0", "160cm"], ["mgq_height_kurisu_a1", "155cm"], ["mgq_height_kurisu_a2", "165cm"], ["mgq_height_kurisu_a3", "170cm"]], correct: 0, c: ["mgq_height_kurisu_c", "160cm。小さいとか言わないでよ！"] },
  { q: ["mgq_birthstone_kurisu_q", "紅莉栖の誕生石は？"], a: [["mgq_birthstone_kurisu_a0", "ルビー"], ["mgq_birthstone_kurisu_a1", "ターコイズ"], ["mgq_birthstone_kurisu_a2", "パール"], ["mgq_birthstone_kurisu_a3", "ダイヤモンド"]], correct: 0, c: ["mgq_birthstone_kurisu_c", "ルビー。7月の誕生石よ。"] },
  { q: ["mgq_zodiac_kurisu_q", "紅莉栖の星座は？"], a: [["mgq_zodiac_kurisu_a0", "獅子座"], ["mgq_zodiac_kurisu_a1", "射手座"], ["mgq_zodiac_kurisu_a2", "水瓶座"], ["mgq_zodiac_kurisu_a3", "魚座"]], correct: 0, c: ["mgq_zodiac_kurisu_c", "獅子座。何か意外？"] },
  { q: ["mgq_height_okabe_q", "岡部の身長は？"], a: [["mgq_height_okabe_a0", "177cm"], ["mgq_height_okabe_a1", "170cm"], ["mgq_height_okabe_a2", "180cm"], ["mgq_height_okabe_a3", "165cm"]], correct: 0, c: ["mgq_height_okabe_c", "177cm。あいつ、高いのよね。"] },
  { q: ["mgq_timeleap_bytes_q", "タイムリープマシンで記憶を送れるのは何バイト？"], a: [["mgq_timeleap_bytes_a0", "36バイト"], ["mgq_timeleap_bytes_a1", "128バイト"], ["mgq_timeleap_bytes_a2", "1KB"], ["mgq_timeleap_bytes_a3", "512バイト"]], correct: 0, c: ["mgq_timeleap_bytes_c", "36バイト。記憶を圧縮するのよ。"] },
  { q: ["mgq_ibn5100_q", "IBN5100の元ネタとなったPCは？"], a: [["mgq_ibn5100_a0", "IBM5100"], ["mgq_ibn5100_a1", "Apple II"], ["mgq_ibn5100_a2", "PC-9801"], ["mgq_ibn5100_a3", "MSX"]], correct: 0, c: ["mgq_ibn5100_c", "IBM5100。伝説のPCよ。"] },
  { q: ["mgq_reading_steiner_q", "世界線移動後も記憶を保つ能力の名前は？"], a: [["mgq_reading_steiner_a0", "リーディングシュタイナー"], ["mgq_reading_steiner_a1", "ダイバージェンスメーター"], ["mgq_reading_steiner_a2", "タイムリープ"], ["mgq_reading_steiner_a3", "アマデウス"]], correct: 0, c: ["mgq_reading_steiner_c", "リーディングシュタイナー。岡部の…能力よ。"] },
  { q: ["mgq_amadeus_q", "人間の記憶を保存するAI研究システムの名前は？"], a: [["mgq_amadeus_a0", "アマデウス"], ["mgq_amadeus_a1", "SERN"], ["mgq_amadeus_a2", "円卓会議"], ["mgq_amadeus_a3", "ジョン・タイター"]], correct: 0, c: ["mgq_amadeus_c", "アマデウス。それは…私よ。"] },
  { q: ["mgq_lab_floor_q", "未来ガジェット研究所は何階にある？"], a: [["mgq_lab_floor_a0", "2階"], ["mgq_lab_floor_a1", "1階"], ["mgq_lab_floor_a2", "3階"], ["mgq_lab_floor_a3", "地下1階"]], correct: 0, c: ["mgq_lab_floor_c", "2階。ブラウン管工房の上よ。"] },
  { q: ["mgq_dmail_q", "過去へ送るメールの呼び名は？"], a: [["mgq_dmail_a0", "Dメール"], ["mgq_dmail_a1", "Eメール"], ["mgq_dmail_a2", "タイムメール"], ["mgq_dmail_a3", "Mメール"]], correct: 0, c: ["mgq_dmail_c", "Dメール。過去へのメールよ。"] },
  { q: ["mgq_mayuri_catchphrase_q", "まゆりの定番の挨拶は？"], a: [["mgq_mayuri_catchphrase_a0", "トゥットゥルー"], ["mgq_mayuri_catchphrase_a1", "ニャニャン"], ["mgq_mayuri_catchphrase_a2", "エル・プサイ・コングルゥ"], ["mgq_mayuri_catchphrase_a3", "いらっしゃい"]], correct: 0, c: ["mgq_mayuri_catchphrase_c", "トゥットゥルー。うつるから使わないで。"] },
];
```

## Appendix B — Change-log texts (verbatim JSON string values; `\n` separates `<li>` items)

**`changelog_v14`**

- ja: `"起動メッセージを実装しました。\nミニゲーム（クイズ）を実装しました。\nアチーブメント機能を追加しました。\n会話言語を表示言語とは別に設定できるようになりました。\n会話記録のエクスポート/インポート機能を追加しました。\nMS 明朝の動的ロード・自動ダウンロードなど、フォントレンダリングを改善しました。\nドラッグによるウィンドウサイズ変更を無効化しました。\nヘブライ語/アラビア語 のサポートを追加しました。\nローカライズに使用されるフォントをNotoSerifに変更しました。"`
- en: `"Implemented startup messages.\nImplemented a mini-game (quiz).\nAdded an achievements feature.\nThe conversation language can now be set independently from the display language.\nAdded export/import support for conversation logs.\nImproved font rendering, including dynamic loading and automatic downloading of MS Mincho.\nDisabled window resizing via dragging.\nAdded support for Hebrew and Arabic.\nChanged the fonts used for localization to Noto Serif."`

**`changelog_v13`**

- ja: `"Live2Dの居眠りモーションを実装しました。\nOpenAI互換APIの使用とベースURL指定機能を実装しました。\nUIを調整しました。\n応答ロード中のキャンセル機能を実装しました。\nLive2Dのリップシンクを実装しました。\nアスペクト比を維持したスケーリングを行うように修正しました。\nAPIストリーミング通信時のデッドロックを修正しました。\nAPIキーを暗号化保存するようにし、セキュリティを向上させました。\nRAGデータの永続化やベクトルDB接続の最適化を実装しました。\nポルトガル語/ウクライナ語/トルコ語のサポートを追加しました。\n起動時の画面点滅および一部環境での起動失敗を防ぐため、デフォルトの画面モードをウィンドウモードに変更しました。\n起動時の最新リリース自動チェックおよび自動アップデーターを実装しました。\n日本語以外の言語でも会話テキストが文末（ピリオド等）で適切に区切られるように改善しました。\nローカライズの管理を最適化しました。"`
- en: `"Implemented Live2D nodding animations.\nImplemented OpenAI compatible API support and custom base URL inputs.\nAdjusted the UI.\nImplemented cancel feature during API loading.\nImplemented Live2D lip-sync.\nFixed to scale while maintaining the aspect ratio.\nFixed deadlocks in API streaming.\nImproved security by encrypting API keys.\nImplemented RAG data persistence and optimized VectorDatabase connections.\nAdded support for Portuguese, Ukrainian, and Turkish.\nChanged the default startup screen mode to Windowed to prevent screen flickering and startup crashes on certain hardware configurations.\nImplemented automatic update checks at startup and an auto-updater.\nImproved sentence splitting in non-Japanese languages to ensure dialogue wraps correctly at sentence endings (periods, etc.).\nOptimized localization management."`

**`changelog_v12`**

- ja: `"中国語/韓国語/スペイン語/フランス語/ドイツ語/ロシア語 のサポートを追加しました。\n視線トラッキングを追加しました。\nデスクトップ通知機能を実装しました。\n軽量化モードを追加しました。\nAlt+Enter、F11 での画面モード切替を実装しました。\nAPIプロバイダーにOllamaとOpenRouterを追加しました。\nUIのバグを修正しました。\nAPIキーのセキュリティを向上させました。\nパフォーマンスを改善しました。"`
- en: `"Added support for Chinese, Korean, Spanish, French, German, and Russian.\nAdded eye tracking.\nImplemented desktop notification feature.\nAdded lightweight mode.\nImplemented screen mode toggle with Alt+Enter and F11.\nAdded Ollama and OpenRouter API providers.\nFixed UI bugs.\nImproved security for API keys.\nImproved performance."`

**`changelog_v11`**

- ja: `"メニュー画面のクリック選択を実装しました。\n日本語/英語の言語切り替えを実装しました。\n各種画面に[閉じる/キャンセル/適用]ボタンを実装しました。\nチェンジログ/バックログ用に、スクロールバーを実装しました。\n視認性の向上のため、UIの一部を変更しました。\n軽量化版のリリースを開始しました。"`
- en: `"Implemented click selection for the menu screen.\nAdded language switching between Japanese and English.\nAdded [Close/Cancel/Apply] buttons to various screens.\nImplemented scrollbars for ChangeLog and BackLog.\nUpdated UI elements for better visibility.\nStarted release of the lightweight version."`

**`changelog_v101`**

- ja: `"GPU使用率が異常に高くなってしまう問題を修正しました。\nフルスクリーン状態での最小化時に、ウィンドウが異常に小さくなってしまう問題を修正しました。\nログアウト後の再ログインが不可能になってしまう問題を修正しました。\n一部AIサービスの利用時にて、感情タグが表示されてしまう問題を修正しました。"`
- en: `"Fixed an issue where GPU usage was abnormally high.\nFixed a window scaling issue when minimizing in fullscreen.\nFixed an issue where re-logging in after logout was impossible.\nFixed a bug where emotion tags were appearing for some AI services."`

**`changelog_v10`**

- ja: `"リアルアマデウスの最初のバージョンをリリースしました。\n基本的な会話機能のみを備えています。"`
- en: `"Released the first version of Real Amadeus.\nIncludes basic conversation features."`

Other languages: same keys in `lib/amadeus/locales/{zh,ko,es,fr,de,ru,uk,pt,tr,he,ar}.json`.

## Appendix C — All-locale values of the keys used in this range (for parity tests)

Language index: 0 ja, 1 en, 2 zh, 3 ko, 4 es, 5 fr, 6 de, 7 ru, 8 uk, 9 pt, 10 tr, 11 he, 12 ar. `help_right_click_title` and `help_wasd` are absent from every locale (inline fallback always used). `help_right_click` is listed for reference only (unused).

### C.1 Short keys

| key | ja | en | zh | ko | es | fr | de | ru | uk | pt | tr | he | ar |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `close` | 閉じる | Close | 关闭 | 닫기 | Cerrar | Fermer | Schließen | Закрыть | Закрити | Fechar | Kapat | סגור | إغلاق |
| `yes` | はい | Yes | 是 | 예 | Sí | Oui | Ja | Да | Так | Sim | Evet | כן | نعم |
| `no` | いいえ | No | 否 | 아니오 | No | Non | Nein | Нет | Ні | Não | Hayır | לא | لا |
| `you` | あなた | You | 你 | 당신 | Tú | Vous | Du | Вы | Ви | Você | Sen | אתה | أنت |
| `amadeus_kurisu` | アマデウス紅莉栖 | Amadeus Kurisu | 阿玛迪斯·红莉栖 | 아마데우스 쿠리스 | Amadeus Kurisu | Amadeus Kurisu | Amadeus Kurisu | Амадеус Курису | Амадеус Курісу | Amadeus Kurisu | Amadeus Kurisu | אמדאוס קוריסו | أماديوس كوريسو |
| `help_toggle_menu` | メニュー開閉 | Toggle Menu | 打开/关闭菜单 | 메뉴 열기/닫기 | Abrir/cerrar menú | Ouvrir/fermer menu | Menü umschalten | Открыть/закрыть меню | Відкрити/закрити меню | Abrir/fechar menu | Menüyü Aç/Kapat | החלפת תפריט | تبديل القائمة |
| `help_save_close` | 保存して閉じる | Save and Close | 保存并关闭 | 저장하고 닫기 | Guardar y cerrar | Sauvegarder et fermer | Speichern und schließen | Сохранить и закрыть | Зберегти та закрити | Salvar e fechar | Kaydet ve Kapat | שמור וסגור | حفظ وإغلاق |
| `help_select_item` | 項目選択 | Select Item | 选择项目 | 항목 선택 | Seleccionar elemento | Sélectionner | Element auswählen | Выбор элемента | Вибір елемента | Selecionar item | Öge Seç | בחר פריט | تحديد عنصر |
| `help_confirm_advance` | 決定 / 会話を進める | Confirm / Advance | 确认 / 推进对话 | 결정 / 대화 진행 | Confirmar / Avanzar | Confirmer / Avancer | Bestätigen / Fortfahren | Подтвердить / Продолжить | Підтвердити / Продовжити | Confirmar / Avançar | Onayla / Devam Et | אישור / התקדמות | تأكيد / تقدم |
| `help_cancel_chat` | 会話をキャンセル | Cancel conversation | 取消对话 | 대화 취소 | Cancelar conversación | Annuler la conversation | Gespräch abbrechen | Отменить беседу | Скасувати розмову | Cancelar conversa | Sohbeti İptal Et | בטל שיחה | إلغاء المحادثة |
| `help_right_click` | 右クリック | Right Click | 右键 | 우클릭 | Clic derecho | Clic droit | Rechtsklick | Правый клик | Правий клік | Botão direito | Sağ Tık | קליק ימני | الزر الأيمن |
| `achievement_secret_hint` | シークレットアチーブメント | Secret Achievement | 隐藏成就 | 비밀 업적 | Logro secreto | Succès secret | Geheimer Erfolg | Секретное достижение | Таємне досягнення | Conquista secreta | Gizli Başarım | הישג סודי | إنجاز سري |
| `mg_menu_title` | 紅莉栖とシュタゲクイズ | Steins;Gate Quiz with Kurisu | 红莉栖与命运石之门问答 | 크리스와 슈타게 퀴즈 | Quiz de Steins;Gate con Kurisu | Quiz Steins;Gate avec Kurisu | Steins;Gate-Quiz mit Kurisu | Викторина по Steins;Gate с Курису | Вікторина по Steins;Gate з Курісу | Quiz de Steins;Gate com a Kurisu | Kurisu ile Steins;Gate Bilgi Yarışması | חידון שטיינס;גייט עם קוריסו | مسابقة شتاينز;غيت مع كوريسو |
| `mg_menu_desc` | シュタインズ・ゲートから%1問出題！ | %1 questions from Steins;Gate! | 命运石之门系列共%1道题！ | 슈타인즈 게이트에 관한 퀴즈 %1문항! | ¡%1 preguntas de Steins;Gate! | %1 questions tirées de Steins;Gate ! | %1 Fragen zu Steins;Gate! | %1 вопросов по Steins;Gate! | %1 питань по Steins;Gate! | %1 perguntas de Steins;Gate! | Steins;Gate serisinden %1 soru! | %1 שאלות בנושא שטיינס;גייט! | %1 أسئلة من شتاينز;غيت! |
| `mg_start` | スタート | Start | 开始 | 시작 | Empezar | Commencer | Start | Начать | Почати | Começar | Başlat | התחל | ابدأ |
| `mg_score` | スコア | Score | 得分 | 점수 | Puntuación | Score | Punktzahl | Счёт | Рахунок | Pontuação | Puan | ציון | النقاط |
| `mg_next` | 次の問題 | Next Question | 下一题 | 다음 문제 | Siguiente pregunta | Question suivante | Nächste Frage | Следующий вопрос | Наступне запитання | Próxima pergunta | Sonraki Soru | השאלה הבאה | السؤال التالي |
| `mg_show_result` | 結果を見る | See Results | 查看结果 | 결과 보기 | Ver resultados | Voir les résultats | Ergebnisse ansehen | Показать результаты | Показати результати | Ver resultados | Sonuçları Gör | הצג תוצאות | عرض النتائج |
| `mg_result_title` | 結果 | Results | 结果 | 결과 | Resultados | Résultats | Ergebnisse | Результаты | Результати | Resultados | Sonuçlar | תוצאות | النتائج |
| `mg_score_result` | %1 / %2 正解 | %1 / %2 correct | %1 / %2 正确 | %1 / %2 정답 | %1 / %2 correctas | %1 / %2 correctes | %1 / %2 richtig | %1 / %2 верно | %1 / %2 правильно | %1 / %2 corretas | %1 / %2 doğru | %1 / %2 נכונות | %1 / %2 صحيحة |
| `mg_retry` | もう一度 | Try Again | 再试一次 | 다시 하기 | Reintentar | Réessayer | Nochmal versuchen | Ещё раз | Спробувати ще | Tentar de novo | Tekrar Dene | נסה שוב | حاول مجددًا |
| `mg_menu_back` | メニューへ | Menu | 菜单 | 메뉴 | Menú | Menu | Menü | Меню | Меню | Menu | Menü | תפריט | القائمة |
| `logout_confirm` | ログアウトしますか？ | Do you want to log out? | 您确定要登出吗？ | 로그아웃하시겠습니까? | ¿Quieres cerrar sesión? | Voulez-vous vous déconnecter? | Möchten Sie sich abmelden? | Вы хотите выйти? | Ви хочете вийти? | Deseja encerrar a sessão? | Çıkış yapmak istiyor musunuz? | האם ברצונך להתנתק? | هل تريد تسجيل الخروج؟ |
| `exit_confirm` | リアルアマデウスを終了しますか？ | Do you want to exit Real Amadeus? | 要退出Real Amadeus吗？ | Real Amadeus를 종료하시겠습니까? | ¿Quieres salir de Real Amadeus? | Voulez-vous quitter Real Amadeus? | Möchten Sie Real Amadeus beenden? | Вы хотите выйти из Real Amadeus? | Ви дійсно хочете вийти з Real Amadeus? | Deseja sair do Real Amadeus? | Real Amadeus'tan çıkmak istiyor musunuz? | האם ברצונך לצאת מ-Real Amadeus? | هل تريد الخروج من Real Amadeus؟ |

### C.2 Long keys

**`mg_wrong`**

| # | lang | value |
|---|---|---|
| 0 | ja | はぁ……ハズレよ、ハズレ。脳のシナプスが死滅してんじゃない？正解は「%1」よ。 |
| 1 | en | Sigh... Wrong. The answer was '%1'. |
| 2 | zh | 唉……答错了。正确答案是'%1'。 |
| 3 | ko | 하아... 틀렸어. 정답은 '%1'였어. |
| 4 | es | Ay... Incorrecto. La respuesta era '%1'. |
| 5 | fr | Pff... Faux. La réponse était '%1'. |
| 6 | de | Seufz... Falsch. Die Antwort war '%1'. |
| 7 | ru | Вздох... Неверно. Правильный ответ был '%1'. |
| 8 | uk | Зітхання... Неправильно. Правильна відповідь була '%1'. |
| 9 | pt | Aff... Errado. A resposta era '%1'. |
| 10 | tr | Of... Yanlış. Cevap '%1' idi. |
| 11 | he | אנחה... לא נכון. התשובה הייתה '%1'. |
| 12 | ar | تنهيدة... خطأ. كانت الإجابة '%1'. |

**`mg_result_perfect`**

| # | lang | value |
|---|---|---|
| 0 | ja | 「パーフェクト！……ま、まあ、私のことだし当然の結果よねっ！あんたにしてはよくやったわ、褒めてあげる！」 |
| 1 | en | Perfect! ...Well, obviously. |
| 2 | zh | 满分！……哼，理所当然啦。 |
| 3 | ko | 완벽해! ...뭐, 당연하지. |
| 4 | es | ¡Perfecto! ...Bueno, obvio. |
| 5 | fr | Parfait ! ...Eh bien, évidemment. |
| 6 | de | Perfekt! ...Naja, offensichtlich. |
| 7 | ru | Идеально! ...Ну, разумеется. |
| 8 | uk | Ідеально! ...Ну, звісно. |
| 9 | pt | Perfeito! ...Bem, óbvio. |
| 10 | tr | Mükemmel! ...Eh, tabii ki. |
| 11 | he | מושלם! ...טוב, מובן מאליו. |
| 12 | ar | مثالي! ...حسنًا، بالطبع. |

**`mg_result_good`**

| # | lang | value |
|---|---|---|
| 0 | ja | 「ふーん、まあまあね。天才の私には遠く及ばないけど、一般人にしては合格点なんじゃない？」 |
| 1 | en | Hmm, not bad. Not bad at all. |
| 2 | zh | 嗯，还不错。还算可以。 |
| 3 | ko | 흠, 나쁘지 않네. 꽤 괜찮아. |
| 4 | es | Hmm, nada mal. Nada mal en absoluto. |
| 5 | fr | Hmm, pas mal. Pas mal du tout. |
| 6 | de | Hmm, nicht schlecht. Gar nicht schlecht. |
| 7 | ru | Хм, неплохо. Совсем неплохо. |
| 8 | uk | Хм, непогано. Зовсім непогано. |
| 9 | pt | Hmm, nada mal. Nada mal mesmo. |
| 10 | tr | Hmm, fena değil. Hiç fena değil. |
| 11 | he | המם, לא רע. בכלל לא רע. |
| 12 | ar | همم، ليس سيئًا. ليس سيئًا أبدًا. |

**`mg_result_ok`**

| # | lang | value |
|---|---|---|
| 0 | ja | 「……あんた、もしかして岡部並みに記憶力が退化してんじゃないの？もう少し脳トレしなさい！」 |
| 1 | en | ...Wait, is your memory deteriorating like Okabe's? You need to do some brain training! |
| 2 | zh | ……你该不会像冈部那样记忆力衰退了吧？多去做做脑力训练！ |
| 3 | ko | ……너, 설마 오카베처럼 기억력이 감퇴한 거 아니야? 뇌 운동 좀 더 해! |
| 4 | es | ...¿Acaso se te está deteriorando la memoria como a Okabe? ¡Deberías entrenar un poco más el cerebro! |
| 5 | fr | ...Ta mémoire flancherait-elle comme celle d'Okabe ? Tu devrais faire un peu de gymnastique cérébrale ! |
| 6 | de | ...Lässt dein Gedächtnis etwa nach wie bei Okabe? Du solltest dein Gehirn etwas mehr trainieren! |
| 7 | ru | ...У тебя что, память ухудшилась, как у Окабэ? Потренируй мозг! |
| 8 | uk | ...У тебе що, пам'ять погіршилась, як в Окабе? Потренуй мізок! |
| 9 | pt | ...Sua memória está falhando como a do Okabe? Você deveria treinar mais o cérebro! |
| 10 | tr | ...Yoksa hafızan Okabe gibi zayıflıyor mu? Beynini biraz daha çalıştırmalısın! |
| 11 | he | ...האם הזיכרון שלך נחלش כמו של אוקאבה? כדאי שתאמן קצת את המוח! |
| 12 | ar | ...هل تراجعت ذاكرتك مثل أوكابي؟ يجب أن تمرن عقلك أكثر قليلاً! |

**`mg_result_bad`**

| # | lang | value |
|---|---|---|
| 0 | ja | 「はぁ……全滅？あんたの海馬、ちょっとスキャンしてバグがないか調べた方がいいわね……」 |
| 1 | en | Sigh... A total wipeout? Do you even like Steins;Gate? |
| 2 | zh | 唉……全军覆没？你到底喜不喜欢Steins;Gate啊？ |
| 3 | ko | 하아... 전멸이야? 정말 Steins;Gate 좋아하는 거 맞아? |
| 4 | es | Ay... ¿Un desastre total? ¿De verdad te gusta Steins;Gate? |
| 5 | fr | Pff... Un échec total ? Tu aimes vraiment Steins;Gate ? |
| 6 | de | Seufz... Ein Totalausfall? Magst du Steins;Gate überhaupt? |
| 7 | ru | Вздох... Полный провал? Тебе вообще нравится Steins;Gate? |
| 8 | uk | Зітхання... Повний провал? Тобі взагалі подобається Steins;Gate? |
| 9 | pt | Aff... Um desastre total? Você gosta mesmo de Steins;Gate? |
| 10 | tr | Of... Tam bir fiyasko? Steins;Gate'i gerçekten seviyor musun? |
| 11 | he | אנחה... כישלון מוחלט? אתה בכלל אוהב את Steins;Gate? |
| 12 | ar | تنهيدة... فشل ذريع؟ هل تحب Steins;Gate أصلًا؟ |

---

## Rust mapping

### amadeus-core (pure, wasm + native, unit-tested)
| module | items |
|---|---|
| `text` | `js_trim(&str) -> &str` (ECMAScript whitespace set), `utf16_len(&str) -> usize`, `replace_first(s, pat, with) -> String` |
| `i18n` (shared spec) | `t(key, default, lang: i32) -> String`; helper `t_ej(key, en_default, ja_default, lang)` = `t(key, if lang == 1 { en } else { ja }, lang)` |
| `auth` | `VALID_NAMES = ["Salieri"]`, `VALID_PASSWORDS = ["MakiseKurisu"]`, `LOGIN_OPERATOR_NAME = "Salieri"`, `fn validate_login(user_id, password) -> bool` (§2.2 incl. trimming) |
| `boot_log` | `BOOT_LOG: &str` (verbatim), `MEM_MAX = 32767`, `START_DELAY_MS = 500`, `LINE_DELAY_MS = 50`, `SLOW_LINE_DELAY_MS = 400`, `MEM_DURATION_MS = 1500`, `MEM_STEP_MS = 30`, `POST_LOG_WAIT_MS = 2000`, `LOGO_SHOW_MS = 4000`, `VIRTUAL_WIDTH = 1920.0`, `FALLBACK_CHAR_WIDTH = 10.0`; `fn preprocess_boot_log(raw, char_width: f64) -> String`; `fn build_display_text(mem: u32, char_width) -> String`; `enum BootStep { Show { text: Rc<str>, visible: usize }, Wait(u32), ShowLogo, Finish }`; `fn boot_script(char_width) -> Vec<BootStep>` (pure expansion of §3.3; tests: last `Show` at 15 500 ms, `Finish` at 21 900 ms, 51 memory frames, `[OK]` column 53 @10.8 px and 50 @11.4545 px) |
| `backlog` | `struct BackLogEntry { role, name_color, message }` (serde camelCase not needed — UI-only), `fn backlog_display_name(role, lang) -> String`; parent-side `fn clean_backlog_message(raw) -> Option<String>` + `fn backlog_name_color(role)` (part4 owns, list here for reuse) |
| `status` | `SYSTEM_VERSION_TEXT`, `LIVE2D_MODEL_TEXT`, `CREDITS: [&str; 6]`; `fn format_clock(y, mo, d, h, mi, s) -> String`; `enum LatencyClass { Good, Warn, Bad }` + `css()`; `fn latency_class(ms: f64)`; `enum StatusValue { Text(String), Colored { prefix: &'static str, class: &'static str, text: String } }`; `fn operator_value(is_logged_in, user_name)`, `fn network_value(online)`, `fn llm_model_value(provider, model)`, `fn latency_value(ms)` (round with `f64::round`, identical to JS for ms ≥ 0) |
| `achievements` (shared spec) | `AchievementManager<S: KvStore>`, `AchievementView`, `unlocked_count()`, `TOTAL = 28`, `all_views(lang)`, `notify_event(&str)` returning unlocked ids for toasts |
| `minigame` (shared spec) | `TOTAL_QUESTIONS`, `QUESTION_POOL`, `generate_questions(rng: &mut dyn FnMut() -> f64)`, `ResultTier { Perfect, Good, Ok, Bad }` with `key()`/`default_text()` (= `resultCommentDefaultForKey`), `result_tier(score)`, `result_emotion(score) -> &'static str`, `MG_UI_DEFAULTS`, `MG_WRONG_KEY/DEFAULT`, `speak_duration_ms(text) = (utf16_len*90).clamp(800, 3000)` |
| `minigame::session` | `enum GameState { Menu, Playing, Result }`; `struct MiniGameSession { state, questions, current, score, selected: Option<usize>, revealed }` with `start(questions) -> [Event::GamePlay]`, `select(i) -> Option<AnswerOutcome { correct: bool }>`, `next() -> NextOutcome { finished: bool, events: Vec<&'static str /* "game_win" | "game_perfect" */> }`, `back_to_menu()`, `emotion_on_show()`; UI owns timers and i18n |
| `changelog` | `struct ChangeLogEntry { version, date, key }`, `CHANGE_LOG_DATA: [ChangeLogEntry; 6]` (append a V2 entry only if product decides — see questions), `fn changelog_lines(key, lang) -> Vec<String>` (`t(key, "", lang).split('\n')`) |
| `help` | `enum HelpText { Literal(&'static str), Key { key, en, ja } }`, `HELP_ENTRIES: [(HelpText, HelpText); 5]` |

### amadeus-ui (Leptos 0.8 CSR)
| file | component / fn |
|---|---|
| `src/ui/overlay.rs` | `fn fade_style(visible: bool, secs: &'static str, z: i32) -> String`; `fn use_visible_keydown(visible: Signal<bool>, handler: impl Fn(web_sys::KeyboardEvent) + 'static)` — attaches `window_event_listener(ev::keydown, …)` when visible becomes true, removes the handle when false and `on_cleanup`; read other signals with `get_untracked()` inside handlers |
| `src/ui/viewport.rs` | `fn use_is_mobile_viewport() -> Signal<bool>` (§1.7) |
| `src/stores.rs` | reactive wrappers: `AchievementsStore { revision: RwSignal<u64> }` bumped from `achievementsChanged`; `MemoryStore { user_name: RwSignal<String> }` bumped from `userNameChanged`; `LangSignal` from `Config_Language` |
| `src/panels/login.rs` | `#[component] LoginPanel(on_accepted: Callback<()>)` — `NodeRef<html::Input>` focused after mount |
| `src/panels/loading.rs` | `#[component] LoadingPanel(on_finished: Callback<()>)`; `fn measure_char_width() -> f64` (canvas `measureText("M")`, `|| 10.0`); executes `boot_script` with `spawn_local` + `gloo_timers::future::TimeoutFuture`, checking an `Rc<Cell<bool>> cancelled` flag after **every** await (Rust futures keep running unlike cleared JS timeouts); `on_cleanup` sets the flag |
| `src/panels/backlog.rs` | `#[component] BackLogPanel(visible: Signal<bool>, entries: Signal<Vec<BackLogEntry>>, lang: Signal<i32>, on_close: Callback<()>)`; `NodeRef<html::Div>`; effect tracking `entries` + `visible` → `request_animation_frame(scroll_top = scroll_height)` |
| `src/panels/status.rs` | `#[component] StatusPanel(visible, llm_status: Signal<LlmStatus>, lang, is_logged_in: Signal<bool>, achievements_open: Signal<bool>, on_open_achievements: Callback<()>, on_close: Callback<()>)`; `#[component] StatusRow(label: &'static str, value: Signal<StatusValue>)` rendering `<div class="status-value">` with text or `{prefix}<span class=…>{text}</span>`; clock via `set_interval_with_handle(1000)` + `js_sys::Date`; network via `set_interval_with_handle(5000)` + `navigator.on_line()` |
| `src/panels/achievements.rs` | `#[component] AchievementPanel(visible, lang, on_close)`; `<For each=views key=|v| v.id.clone()>` |
| `src/panels/minigame.rs` | `#[component] MiniGamePanel(visible, lang, on_close: Callback<()>, on_emotion_change: Callback<String>, on_speak_value_change: Callback<f64>)`; `StoredValue<Option<IntervalHandle>>`, `StoredValue<Option<TimeoutHandle>>`; `js_sys::Math::random()` for lip-sync and shuffles |
| `src/panels/changelog.rs` | `#[component] ChangeLogPanel(visible, lang, on_close)` |
| `src/panels/help.rs` | `#[component] HelpPanel(visible, lang, on_close)`, `#[component] HelpEntry(title: Signal<String>, description: Signal<String>)` |
| `src/panels/confirm.rs` | `#[component] ConfirmationDialog(visible, message: Signal<String>, lang, on_confirm: Callback<()>, on_cancel: Callback<()>)`; `RwSignal<u8>` selected; reset to 1 via `set_timeout(…, 0)` on open |

Implementation notes:
- Keep panels always mounted and drive the wrapper `style` reactively (do not wrap them in `<Show>`), otherwise fade-out, scroll persistence and mini-game resume break.
- Callbacks are captured once (component bodies run once) — this intentionally removes V1's restart-on-rerender quirks (§3.5, §5.2).
- `web-sys` features missing from `crates/amadeus-ui/Cargo.toml` for this range: **`CanvasRenderingContext2d`**, **`TextMetrics`** (char-width measurement) and **`VisualViewport`** (mobile detection). `Navigator`, `KeyboardEvent`, `HtmlCanvasElement` are already listed.
- Platform: all panels are identical on Web and Tauri (pure UI in the webview). Only the parent's shutdown action differs (§11.3) — Transport/desktop layer should expose an `exit_app()`.
- Copy `app/amadeus/app/globals.css` verbatim into `apps/web/style/amadeus.css` (currently a 2-line placeholder) and the images/fonts into `assets/public/resources/`.

## Open questions / risks
1. **Version strings for V2.** `SYSTEM_VERSION` and the last credits line hard-code `Real Amadeus v1.4N(Build 20260801_001)`; `amadeus_core::version::display_version()` yields `V2.0.0R`. Decide whether V2 shows its own version (recommended: derive from `version.rs`) and whether CHANGE LOG gets a `Version 2.0` entry (needs a new `changelog_v20` key in all 13 locales).
2. **HTML injection in StatusRow.** V1 injects `OPERATOR` (may come from an LLM-extracted user name) and `LLM_MODEL` (user-typed model / base URL) as innerHTML. Recommend structured, escaped rendering (`StatusValue`) — identical DOM for normal strings, but names containing `<`/`&` will render literally instead of as markup.
3. **Boot-log `[OK]` column depends on font timing and platform** (first boot measures the platform fallback monospace: ≈53 Linux/macOS, ≈58 Windows/Consolas; 50 once DepartureMono is loaded). Choose: replicate (measure at mount) or `await document.fonts.load('18px "DepartureMono"')` during the 500 ms start delay for a deterministic 50 (QT behavior).
4. **React quirks not to replicate:** LoadingPanel restarts on any parent re-render (resize/Tab/right-click during boot); StatusPanel intervals reset on parent re-render. Confirm product wants the clean behavior.
5. **Tab on the login screen** opens the side menu instead of moving focus (global handler, part4). Parity vs. usability decision.
6. **Logout does not clear the operator name** (`setUserName("")` is a no-op); harmless because login always sets `Salieri`, but OPERATOR keeps any chat-learned name until the next login. Keep for parity unless memory spec changes it.
7. **Missing locale keys** `help_right_click_title`, `help_wasd`: non-English languages show Japanese help titles. Keep (parity) or add keys (would be a visible change; could reuse `help_right_click` as `Tab / {help_right_click}`).
8. **Locale data oddity:** `he.json` `mg_result_ok` contains an Arabic letter (`נחלش`). Embed verbatim; flag to translators.
9. **Hidden-but-mounted buttons stay focusable** (pointer-events:none does not block keyboard). V1 shares this; keep DOM order identical so behavior matches, don't add focus traps.
10. **`unlockedCount` vs rows** can disagree if storage holds unknown ids (count = raw array length). Preserve in `AchievementManager`.
11. **JS string semantics:** lengths/slices are UTF-16 (`text.length*90`, `visibleChars`, `<pos>` prefix length) and `trim()` uses the ECMAScript whitespace set — use helpers, not Rust byte/char defaults.
12. **Mobile mini-game/achievement UI is uncommitted work in progress** (last commit message: "Save current state before mobile UI improvements for quiz and achievements"). Re-diff `RealAmadeusApp.tsx` 2532–2827 and `globals.css` 2583–2965 before final porting.
13. **Shutdown on desktop**: V1 calls `window.close()` + history/redirect to `https://dsuv.elvelt.com/amadeus`; Tauri must exit the process instead; web build keeps V1 behavior (part4 owns, ConfirmationDialog just calls `on_confirm`).
14. **Timer drift**: V1 chains ~90 `setTimeout`s for the boot (21.9 s nominal). Using `TimeoutFuture` gives equivalent drift; do not "optimize" into a single absolute schedule unless parity tests allow ±some ms.
15. Leptos 0.8 effects run asynchronously after signal changes; scroll-to-bottom and focus must be scheduled after DOM insertion (`request_animation_frame`), mirroring React's after-paint `useEffect`.

---

# RealAmadeus V2 — Porting spec: **pages** subsystem (landing, download, changelog, docs)

Target: `amadeus-ui` (Leptos 0.8 CSR + leptos_router 0.8.17) with data in `amadeus-core`.
Source of truth: the V1 **working tree** of `/home/elvelt_admin/Elvelt/dsuv`, read on 2026-11-16. Several of these files have uncommitted edits (V1.3 → V1.4 download URLs, V1.4 changelog `product`, and the new `Alt + Enter / F11` control row). This spec follows the working tree.

---

## 0. Scope and conventions

### 0.1 Files covered (all read completely, line counts in parentheses)

| File | Export | Role |
|---|---|---|
| `app/amadeus/page.tsx` (147) | `AmadeusPage` | Landing `/amadeus` |
| `app/amadeus/layout.tsx` (14) | `AmadeusLayout` + `metadata.title = "Amadeus"` | Pass-through layout (context) |
| `app/amadeus/download/page.tsx` (192) | `AmadeusDownloadPage` | `/amadeus/download` |
| `app/amadeus/changelog/page.tsx` (136) | `ChangelogPage` | `/amadeus/changelog` |
| `app/amadeus/docs/layout.tsx` (127) | `DocsLayout` | Sidebar shell for `/amadeus/docs/**` |
| `app/amadeus/docs/components.tsx` (25) | `renderSection(icon, title, content)` | Section block |
| `app/amadeus/docs/page.tsx` (48) | `DocsIndex` | `/amadeus/docs` |
| `app/amadeus/docs/download/page.tsx` (197) | `DownloadPage` | `/amadeus/docs/download` |
| `app/amadeus/docs/login/page.tsx` (52) | `LoginPage` | `/amadeus/docs/login` |
| `app/amadeus/docs/controls/page.tsx` (43) | `ControlsPage` | `/amadeus/docs/controls` |
| `app/amadeus/docs/api/page.tsx` (44) | `ApiSetupPage` | `/amadeus/docs/api` |
| `app/amadeus/docs/api/openai/page.tsx` (72) | `OpenAIPage` | `/amadeus/docs/api/openai` |
| `app/amadeus/docs/api/gemini/page.tsx` (71) | `GeminiPage` | `/amadeus/docs/api/gemini` |
| `app/amadeus/docs/api/claude/page.tsx` (72) | `ClaudePage` | `/amadeus/docs/api/claude` |
| `app/amadeus/docs/api/groq/page.tsx` (74) | `GroqPage` | `/amadeus/docs/api/groq` |
| `app/amadeus/docs/api/vertex/page.tsx` (97) | `VertexPage` | `/amadeus/docs/api/vertex` |
| `hooks/useTranslation.tsx` (676) | `LanguageProvider`, `useLanguage` + dictionary | i18n used by every page above |
| `components/KofiWidget.tsx` (64), `components/ElveltBadge.tsx` (18) | — | **Not used by these pages.** Only the portfolio `app/page.tsx` uses them. Specced in Appendix D for completeness. |

Context also read: `app/layout.tsx` (root layout), `app/globals.css` (root CSS), `next.config.ts`, `package.json`, `app/amadeus/app/layout.tsx` and the global selectors of `app/amadeus/app/globals.css` (for the CSS-leak analysis), and the compiled `.next` chunks. The chunks were used to **verify** styled-jsx scoping, JSX whitespace and Tailwind output. Also read: `node_modules/lucide-react@0.563.0`, Next's built-in 404 fallback, and the existing V2 skeleton (`RealAmadeusRust/docs/ARCHITECTURE.md`, `apps/web/index.html`, `Trunk.toml`, `apps/desktop/tauri.conf.json`, `crates/amadeus-ui/src/patterns.rs`, `crates/amadeus-core/src/version.rs`, and leptos_router 0.8.17 sources).

### 0.2 Conventions

* `{base}` is the V2 router base. It is `"/amadeus"` on the Web build (Trunk `public_url = "/amadeus/"`) and `""` on desktop (Tauri, `public_url "/"`). See §7.
* `{static}` is the V2 static-asset prefix. Per ARCHITECTURE.md rule 4, V1 root files such as `/amadeus_logo.png` become base-relative `static/amadeus_logo.png`. All page images already exist in `RealAmadeusRust/assets/public/static/`.
* `t` is the active dictionary, `translations[locale]` in V1 and `site_text(locale)` in V2. `t.amadeus.release.title` is written `release.title` when unambiguous.
* `«…»` marks verbatim user-visible text. Code blocks with `class="…"` contain **verbatim** class strings. Keep every token, including no-op ones (`animate-in`, `fade-in`, `glow-on-hover` on the landing).
* JSX camelCase SVG attributes are given in DOM form: `stroke-linecap`, `stroke-linejoin`, `stroke-width`, `fill-rule`, `clip-rule`.
* "V1 quirk" means intentional or accidental behaviour that the V2 port must reproduce unless the text explicitly says not to.

### 0.3 Key components found (summary)

1. **Terminal chrome**, shared by Landing, Download and Changelog: a 50 ms clock, a 16×40 random-binary background that **re-randomizes on every clock tick**, a grid overlay, a fixed status header with the `EN`/`JP` toggle, and page-scoped styled-jsx CSS.
2. **Docs shell**: a 10-entry sidebar with exact or prefix active matching, a mobile collapsible menu, two language toggles (desktop and mobile), and a scanline overlay.
3. **Content pages**: one generic section block (`renderSection`), a step timeline, image frames, a models box, and a changelog timeline.
4. **i18n**: 2 locales (`ja` default, `en`). The choice is held in React state only, with **no persistence**. Some values are **rich text** (links, `<code>`).
5. **Download links** point to V1 GitHub Releases (V1.4U Unity builds and V1.4Q Qt builds).

---

## 1. Global context

### 1.1 Route map

| V1 URL | V1 component | V2 route (relative to `{base}`) | Proposed V2 component (`amadeus-ui`) |
|---|---|---|---|
| `/amadeus` | `AmadeusPage` | `/` | `pages::landing::LandingPage` |
| `/amadeus/download` | `AmadeusDownloadPage` | `/download` | `pages::download::DownloadPage` |
| `/amadeus/changelog` | `ChangelogPage` | `/changelog` | `pages::changelog::ChangelogPage` |
| `/amadeus/docs` | `DocsLayout` > `DocsIndex` | `/docs` | `pages::docs::DocsLayout` > `DocsIndexPage` |
| `/amadeus/docs/download` | `DocsLayout` > `DownloadPage` | `/docs/download` | `DocsDownloadPage` |
| `/amadeus/docs/login` | `DocsLayout` > `LoginPage` | `/docs/login` | `DocsLoginPage` |
| `/amadeus/docs/controls` | `DocsLayout` > `ControlsPage` | `/docs/controls` | `DocsControlsPage` |
| `/amadeus/docs/api` | `DocsLayout` > `ApiSetupPage` | `/docs/api` | `DocsApiPage` |
| `/amadeus/docs/api/openai` | `DocsLayout` > `OpenAIPage` | `/docs/api/openai` | `ProviderDocPage { provider: OpenAi }` |
| `/amadeus/docs/api/gemini` | `DocsLayout` > `GeminiPage` | `/docs/api/gemini` | `ProviderDocPage { provider: Gemini }` |
| `/amadeus/docs/api/claude` | `DocsLayout` > `ClaudePage` | `/docs/api/claude` | `ProviderDocPage { provider: Claude }` |
| `/amadeus/docs/api/groq` | `DocsLayout` > `GroqPage` | `/docs/api/groq` | `ProviderDocPage { provider: Groq }` |
| `/amadeus/docs/api/vertex` | `DocsLayout` > `VertexPage` | `/docs/api/vertex` | `ProviderDocPage { provider: Vertex }` |
| `/amadeus/app` | (app subsystem) | `/app` | (app subsystem) |
| anything else | Next built-in 404 | fallback | `pages::not_found::NotFoundPage` (§4) |

`DocsLayout` is a **persistent layout**: it stays mounted while navigating between docs pages. Its mobile-menu state survives those navigations, although every nav link closes the menu anyway. V2 should model it as a leptos_router `ParentRoute` with an `<Outlet/>`.

Links that leave the site: the landing's «Return» goes to `/` (the DSUV portfolio, outside `{base}`), plus all GitHub and provider URLs (§1.8).

### 1.2 Document shell shared by all pages (V1)

The root layout `app/layout.tsx` wraps every route:

```html
<html lang="en" class="dark">
<head>
  <meta http-equiv="Cache-Control" content="no-cache, no-store, must-revalidate">
  <meta http-equiv="Pragma" content="no-cache">
  <meta http-equiv="Expires" content="0">
  <title>Amadeus</title>   <!-- metadata from app/amadeus/layout.tsx; root default "DSUV" -->
  <meta name="description" content="Personal Portfolio of DSUV - Lead Architect @ ELVELT"> <!-- inherited; NOT overridden for /amadeus/** -->
  <link rel="icon" href="/amadeus/icon.png?…" type="image/png" sizes="1024x1024"> <!-- app/amadeus/icon.png, byte-identical to public/amadeus_logo.png (md5 25379e9fbf305948d7f8be631bc007b2) -->
  <meta name="viewport" content="width=device-width, initial-scale=1"> <!-- Next default -->
</head>
<body class="bg-black text-white min-h-screen selection:bg-purple-500/30 selection:text-white">
  <!-- LanguageProvider: context only, no DOM -->
  <!-- page content -->
</body></html>
```

* The pages are `'use client'` and cannot export metadata, so **every** page title is «Amadeus». No per-route title change is needed in V2.
* `class="dark"` has no effect: no `dark:` utilities are used, and there is no custom dark variant.
* `app/globals.css` (Appendix C, verbatim) is loaded on every page. Its **unlayered** `body{background:var(--background);color:var(--foreground);font-family:Arial,Helvetica,sans-serif;overflow-x:hidden}` overrides the body utility classes. Every page sets `font-mono` and its own colours on its root element, so what remains visible is `overflow-x:hidden` and the black page background.
* The V2 `apps/web/index.html` already has `<title>Amadeus</title>`, the `static/amadeus_logo.png` icons, `lang="ja"` (V1 uses `lang="en"`) and description «Real Amadeus Web». These differences are harmless.
* **V2 action:** give `<body>` the V1 body classes, and put the V1 root `globals.css` content after `@import "tailwindcss";` in `apps/web/style/tailwind.css`. The pages rely on that `@theme`/`body` setup. Coordinate with the CSS spec.

### 1.3 HTTP-level behaviour (input for `amadeus-server`)

* **Response headers.** `next.config.ts` matches both `/amadeus/:path*` and `/(.*)` and sends `Cache-Control: no-store, no-cache, must-revalidate, proxy-revalidate, max-age=0`, `Pragma: no-cache`, `Expires: 0`, `Surrogate-Control: no-store`, `CDN-Cache-Control: no-store`. Only `/resources/:path*` is overridden, because the last matching rule wins: `Cache-Control: public, max-age=86400, stale-while-revalidate=604800`, `Pragma: public`, `Expires: 86400`, `Surrogate-Control: public, max-age=86400`, `CDN-Cache-Control: public, max-age=86400`. The page images (`/amadeus_logo.png`, `/OpenAI_STEP1.png`, …) therefore come back **no-store** and are re-downloaded on every visit. V2 may legitimately cache `static/*`; this is a performance improvement, not a behaviour change.
* **Trailing slash.** Next's default `trailingSlash:false` answers `/amadeus/docs/` (and `/amadeus/`) with **308** to the slash-less URL. The sidebar's exact matches (`=== '/amadeus/docs'`) depend on this. V2 must normalize the pathname before matching (§3.4.2). The server *may* also send the 308s for parity.
* **Unknown paths** return **404** with Next's default page (§4). The axum SPA fallback must still serve `index.html` so the router can render `NotFoundPage`. Return status 404 for paths not in §1.1 if cheap; otherwise 200 is acceptable.
* Page routes answer `200` with the SPA `index.html`.

### 1.4 i18n mechanism used by these pages (`hooks/useTranslation.tsx`)

```ts
type Locale = 'ja' | 'en';
const translations = { ja: {...}, en: {...} } as any;    // lines 7–648
const LanguageContext = createContext<{ locale, toggleLanguage, t } | undefined>(undefined);
export const LanguageProvider = ({ children }) => {
  const [locale, setLocale] = useState<Locale>('ja');                   // DEFAULT 'ja'
  const toggleLanguage = () => setLocale(prev => (prev === 'ja' ? 'en' : 'ja'));
  return <LanguageContext.Provider value={{ locale, toggleLanguage, t: translations[locale] }}>{children}</...>;
};
export const useLanguage = () => { const c = useContext(LanguageContext); if (!c) throw new Error('useLanguage must be used within a LanguageProvider'); return c; };
```

Behaviour that V2 must reproduce:

1. **Default locale `ja`.** There is no browser-language detection.
2. **No persistence.** No localStorage, sessionStorage or cookie is read or written. A full page reload always shows `ja`. **These pages touch no storage keys at all.**
3. The provider sits in the **root** layout, so the locale **survives client-side navigation** between Landing, Download, Changelog and the Docs pages (and the portfolio `/`). In V2, provide one `RwSignal<SiteLocale>` above the `<Router>`.
4. Toggle button label = the language you would switch **to**: «EN» when the locale is `ja`, «JP» when it is `en`. Note «JP», not «JA».
5. This locale is **independent** of the Web app's own 13-language setting (`Config_Language` in `lib/amadeus/app-settings.ts`, dictionary `lib/amadeus/locales/*.json`). The pages never read `Config_Language`.
6. The dictionary is **inline** in `hooks/useTranslation.tsx`: `ja` at lines 8–327 and `en` at lines 328–647. Some values are **React elements** (rich text with `<a>` and `<code>`) rather than strings. Appendix A gives the complete `amadeus` subtree of both locales as JSON, extracted programmatically (exact strings), with rich text encoded as segments (§1.4.2). The `common.*` and `home.*` subtrees (ja lines 9–34, en lines 329–354) belong only to the portfolio page and are out of scope.
7. `<html lang>` is NOT updated on toggle (it stays `en` in V1).

#### 1.4.1 Key usage matrix (every `t.…` reference in the in-scope pages)

| Key (under `t.amadeus.`) | Used by |
|---|---|
| `release.title`, `release.message` | Landing h1 / p |
| `release.download` | Landing DOWNLOAD button; Download page h1 |
| `release.githubRepo`, `release.webVersion` | Landing buttons |
| `release.docs` | Landing Docs button; Changelog page Docs button |
| `release.windowsDownload`, `release.macosDownload`, `release.linuxDownload`, `release.lightweightWindowsDownload` | Download page cards |
| `docs.back` | Docs sidebar back link |
| `docs.sidebar.{gettingStarted,download,login,controls,apiSetup,apiOpenAI,apiGemini,apiClaude,apiGroq,apiVertex}` | Docs sidebar labels |
| `docs.gettingStarted.{title(×2),desc,loginInfo}` | Docs index |
| `docs.download.{title,desc,standardTitle,standardDesc,lightweightTitle,lightweightDesc,recommendedEnv(×2),standardEnv,lightweightEnv,windows(×2),mac,linux,downloadBtn(×4),fallbackStandard,fallbackLightweight,steps(.slice(2) ×2),fileStructure.{title(×2),windows,mac,linux,windowsQT},notice(×2)}` | Docs download |
| `docs.changelog.title` | Landing button; Download page link; Changelog page h1; Docs download section title |
| `docs.changelog.desc`, `docs.changelog.versions` | Changelog page; Docs download section |
| `docs.loginDoc.{title(×2),desc,idLabel,idValue,passLabel,passValue}` | Docs login |
| `docs.controls.{title(×2),desc,items}` | Docs controls |
| `docs.api.{title(×2),desc,steps,notice}` | Docs API index |
| `docs.api.modelLabels.{title,available,recommended}` | All 5 provider pages |
| `docs.{apiOpenAI,apiGemini,apiClaude,apiGroq,apiVertex}.{title,desc,steps,modelList.available,modelList.recommended}` | Provider pages |

**Never rendered** but present: `status.*`, `button.init`, `panels.*`, `hud.*`, `accessRestricted.*`, `login.*`, `preparation.*`, `docs.title`, `docs.download.releasesPage`, `docs.download.steps` (only ever rendered as `.slice(2)` of a 2-element array, which is empty), and `docs.changelog.versions[].product`. Port them into the data for completeness, but no UI uses them.

**Literal (untranslated) strings** in the pages: «SYSTEM_ONLINE», «Return» (landing), «Return to System», «Select your platform to initialize deployment», «Universal Edition», «Lightweight Edition», «Deployment Package», «Optimized Deployment», «V1.4U», «V1.4Q», «Project Amadeus», «Documentation» (mobile docs bar), «SYSTEM.LOGIN_INFO», «Step », «Windows», «MacOS», «Linux» (file-structure labels), «▶», «◆», «⚠️» (U+26A0 U+FE0F), «←», and all `alt` texts. **Locale-inline strings** appear only on the Changelog page: «ダウンロードへ» / «BACK TO DOWNLOAD» and «戻る» / «Return».

#### 1.4.2 Rich-text values and their encoding

Eight values per locale are React fragments: `docs.download.fallbackStandard`, `docs.download.fallbackLightweight`, `docs.apiOpenAI.steps[0].content`, `docs.apiGemini.steps[0].content`, `docs.apiClaude.steps[0].content`, `docs.apiGroq.steps[0].content`, `docs.apiVertex.steps[0].content` and `docs.apiVertex.steps[4].content`. They contain only text, `<a>` and (Vertex step 5 only) `<code>` with no attributes.

All `<a>` in the dictionary have **identical** attributes, verified programmatically:

```html
<a href="{href}" target="_blank" rel="noopener noreferrer" class="text-[#d49e38] underline hover:text-[#f0b95b] transition-colors">{text}</a>
```

Encoding used in Appendix A: plain values are JSON strings, and rich values are `{"rich":[ … ]}` with segments `{"text":"…"}`, `{"link":"https://…","text":"…"}` or `{"code":"…"}`. Render the segments in order as sibling inline nodes: text node, the `<a>` above, or `<code>{code}</code>` with no class. Whitespace inside segments is significant (e.g. `" にアクセスし、…"` begins with a space).

### 1.5 CSS

#### 1.5.1 Tailwind

* V1 uses `tailwindcss@4.1.18` via `@tailwindcss/postcss`. V2 Trunk is pinned to the same version, so identical class strings produce identical CSS.
* Breakpoints (v4 defaults): `sm` 40rem (640px), `md` 48rem (768px), `lg` 64rem (1024px).
* v4 compiles `hover:`/`group-hover:` inside `@media (hover:hover)`, so hover styles do not stick on touch devices. The styled-jsx `:hover` rules (§1.5.3) are **not** wrapped and do stick.
* Default transition: `150ms cubic-bezier(.4,0,.2,1)`.
* **No animation plugin is installed** (`tailwindcss-animate` / `tw-animate-css` are absent from `package.json`). `animate-in` and `fade-in` therefore generate **nothing**. This was verified against the compiled CSS, which contains no `.animate-in` or `.fade-in`. `duration-700` on those wrappers only sets an unused `transition-duration`. **The docs pages have no entry animation.** Keep the tokens. Do NOT add a fade.
* Compiled theme values relevant here: `--font-mono: ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,"Liberation Mono","Courier New",monospace`; `--animate-pulse: pulse 2s cubic-bezier(.4,0,.6,1) infinite` with `@keyframes pulse{50%{opacity:.5}}`; `--tracking-widest:.1em`; `--leading-loose:2`; `--leading-relaxed:1.625`; `--blur-3xl:64px`; `.max-h-screen{max-height:100vh}`. `selection:` applies to the element and to descendants' `::selection`.
* Tailwind v4 has `text-shadow-*` utilities (since 4.1), but `text-shadow-glow`, `glow-on-hover` and `animate-scroll-bg` are **not** utilities. Verified: they are absent from the compiled CSS. They exist only via styled-jsx (§1.5.3).

#### 1.5.2 Arbitrary values and inline styles (must survive the Tailwind `@source` scan of `.rs` files)

Arbitrary classes: `bg-[#050505]`, `text-[#d49e38]` (and `/40 /60 /70 /80 /90`, plus `hover:text-[#d49e38]`), `text-[#888]`, `selection:bg-[#d49e38]`, `drop-shadow-[0_0_5px_rgba(212,158,56,0.5)]` (docs login values), `hover:border-[#d49e38]/60`, `hover:bg-[#d49e38]` (and `/5 /10`), `bg-[linear-gradient(rgba(0,0,0,0)_0%,rgba(0,0,0,1)_90%)]`, `h-[200%]`, `text-[10px]`, `text-[9px]`, `text-[8px]`, `text-[11px]`, `tracking-[0.3em]`, `tracking-[0.2em]`, `tracking-[0.4em]`, `tracking-[0.5em]`, `border-[#d49e38]/30` (and `/20 /40 /50 /60 /80`), `drop-shadow-[0_0_20px_rgba(212,158,56,0.3)]`, `drop-shadow-[0_0_8px_rgba(212,158,56,0.5)]`, `drop-shadow-[0_0_5px_rgba(212,158,56,0.6)]`, `drop-shadow-[0_0_2px_rgba(212,158,56,0.3)]`, `via-[#d49e38]/50`, `from-[#d49e38]`, `from-[#d49e38]/60`, `via-[#d49e38]/10`, `from-[#d49e38]/10`, `to-[#d49e38]/30`, `md:h-[calc(100vh-5rem)]`, `w-[2px]`, `-left-[9px]`, `min-w-[160px]`, `min-w-[120px]`, `group-hover:translate-y-[2px]`, `shadow-[0_0_20px_rgba(212,158,56,0.05)]`, `shadow-[0_0_20px_rgba(212,158,56,0.15)]`, `shadow-[inset_0_0_30px_rgba(0,0,0,0.8)]`, `shadow-[inset_0_0_20px_rgba(0,0,0,0.6)]`, `shadow-[0_0_15px_rgba(212,158,56,0.1)]`, `shadow-[0_0_15px_rgba(212,158,56,0.05)]`, `shadow-[0_0_30px_rgba(212,158,56,0.05)]`, `shadow-[0_0_8px_rgba(212,158,56,0.2)]`, `bg-[linear-gradient(rgba(212,158,56,0.05)_1px,transparent_1px),linear-gradient(90deg,rgba(212,158,56,0.05)_1px,transparent_1px)]`, `bg-[linear-gradient(rgba(212,158,56,0.03)_1px,transparent_1px),linear-gradient(90deg,rgba(212,158,56,0.03)_1px,transparent_1px)]`, `bg-[size:4px_4px]`, `bg-[#111]`, `bg-[#d49e38]/5` (and `/10 /20 /40`), `opacity-[0.03]`, `hover:text-[#f0b95b]` (dictionary links), `marker:text-[#d49e38]`, `marker:font-bold`, `opacity-85`.

Inline styles, which must be set as `style` attributes and not classes:

| Where | Inline style |
|---|---|
| Grid overlay (Landing/Download/Changelog) | `background-image: linear-gradient(#333 1px, transparent 1px), linear-gradient(90deg, #333 1px, transparent 1px); background-size: 40px 40px;` |
| Landing logo `<img>` | `animation-duration: 4s;` (overrides `animate-pulse`'s 2s, so the logo pulses over 4 s) |
| Docs scanline overlay | `background-image: repeating-linear-gradient(0deg, transparent, transparent 1px, #d49e38 1px, #d49e38 2px);` |

**Rule for the Rust port:** every class string must appear **literally** in a `.rs` file so Tailwind's scanner sees it. Never build class names with `format!` pieces such as `format!("border-{}", c)`. Conditional classes must be selected between complete literal strings (§3.4).

#### 1.5.3 Page-scoped CSS (styled-jsx) and its scoping quirk

Three pages contain `<style jsx>` blocks. styled-jsx adds a class `jsx-<hash>` to every **lowercase** (native) JSX element of that component and rewrites selectors to `.cls.jsx-<hash>`. Capitalized components such as `<Link>`, `ArrowLeft` and `GitCommitVertical` do **not** get the class. Verified in `node_modules/styled-jsx/dist/babel/index.js` and in the compiled chunks `.next/static/chunks/b0b18a5d77620f10.js` (landing, scope `jsx-a50ea49eb5e28131`), `78df1b6f99143cfa.js` (download, `jsx-dff50417e31d0221`) and `d340c7790fbef0be.js` (changelog, `jsx-f0fe92232cf20a4b`). `@keyframes scroll-bg` is emitted **unscoped**.

Source CSS per page, verbatim:

```css
/* Landing app/amadeus/page.tsx */
.text-shadow-glow { text-shadow: 0 0 10px rgba(212, 158, 56, 0.5); }
.glow-on-hover:hover { box-shadow: 0 0 20px rgba(212, 158, 56, 0.4); }
@keyframes scroll-bg { 0% { transform: translateY(0); } 100% { transform: translateY(-50%); } }
.animate-scroll-bg { animation: scroll-bg 60s linear infinite; }

/* Download app/amadeus/download/page.tsx */
.text-shadow-glow { text-shadow: 0 0 15px rgba(212, 158, 56, 0.6); }
.glow-on-hover:hover { box-shadow: 0 0 30px rgba(212, 158, 56, 0.3); }
@keyframes scroll-bg { 0% { transform: translateY(0); } 100% { transform: translateY(-50%); } }
.animate-scroll-bg { animation: scroll-bg 60s linear infinite; }

/* Changelog app/amadeus/changelog/page.tsx */
.text-shadow-glow { text-shadow: 0 0 15px rgba(212, 158, 56, 0.6); }
@keyframes scroll-bg { 0% { transform: translateY(0); } 100% { transform: translateY(-50%); } }
.animate-scroll-bg { animation: scroll-bg 60s linear infinite; }
```

**V1 quirk Q1 (verified):** on the Landing page, `glow-on-hover` sits on a `<Link>` (component), so the `<a>` it renders has no `jsx-a50ea49eb5e28131` class. The selector `.glow-on-hover.jsx-a50ea49eb5e28131:hover` never matches, and **the landing DOWNLOAD button has no glow**: only its `hover:bg`/`hover:text` change. On the Download page the cards are native `<a>`, so the glow **does** apply. The `text-shadow-glow` h1s and the `animate-scroll-bg` divs are native everywhere, so those apply.

**V2 implementation.** Put these rules in one global, unlayered stylesheet (for example `apps/web/style/site.css`, or after `@import "tailwindcss";` outside any `@layer`), scoped by a data attribute on each page root so that the verbatim class lists stay untouched. The specificity (0,2,0) matches V1's `.x.jsx-h`:

```css
[data-site-page="landing"]   .text-shadow-glow { text-shadow: 0 0 10px rgba(212, 158, 56, 0.5); }
/* landing: deliberately NO .glow-on-hover rule (V1 quirk Q1) */
[data-site-page="download"]  .text-shadow-glow { text-shadow: 0 0 15px rgba(212, 158, 56, 0.6); }
[data-site-page="download"]  .glow-on-hover:hover { box-shadow: 0 0 30px rgba(212, 158, 56, 0.3); }
[data-site-page="changelog"] .text-shadow-glow { text-shadow: 0 0 15px rgba(212, 158, 56, 0.6); }
@keyframes scroll-bg { 0% { transform: translateY(0); } 100% { transform: translateY(-50%); } }
.animate-scroll-bg { animation: scroll-bg 60s linear infinite; }
```

The extra `data-site-page` attribute on `<main>` is the only DOM addition. If product decides the landing glow *should* work, add `[data-site-page="landing"] .glow-on-hover:hover { box-shadow: 0 0 20px rgba(212, 158, 56, 0.4); }` (listed under Open questions).

#### 1.5.4 CSS isolation hazard: the Web app's global CSS must not leak into these pages

`app/amadeus/app/globals.css` (the app's CSS; in V2 it becomes `apps/web/style/amadeus.css`, loaded globally by `index.html`) contains **unlayered element-level** rules:

```css
* { box-sizing: border-box; margin: 0; padding: 0; user-select: none; -webkit-user-select: none; }   /* line 67 */
input, textarea { user-select: text; -webkit-user-select: text; }
*:focus { outline: none; }
*:focus-visible { outline: none !important; }
select:focus, select:focus-visible, .config-input-control:focus, .config-input-control:focus-visible { outline: none !important; border-color: #333 !important; box-shadow: none !important; }
button, input, select { font: inherit; }
```

Unlayered CSS beats every layered Tailwind utility regardless of specificity. If this file is global in the V2 SPA, the `*` rule wipes out **every** margin and padding utility on these pages (`p-8`, `px-10`, `mb-12`, `mx-auto`, `space-y-*`, …), disables text selection, and removes focus outlines.

In V1 this file is only loaded under `/amadeus/app`. It most likely persists after a client-side back navigation from the app, because React 19 never unloads `precedence` stylesheets, so V1 probably shows broken pages after "app → Back". That is a latent V1 bug and **must not** be reproduced.

**Required V2 fix** (coordinate with the CSS spec): scope those rules to the app root while keeping zero specificity, for example `:where(.amadeus-root, .amadeus-root *) { box-sizing:border-box; margin:0; padding:0; user-select:none; -webkit-user-select:none; }`, and likewise for the focus rules. Alternatives are to toggle `<link disabled>` per route, or to load the app CSS only while `/app` is mounted. Acceptance test: the Landing and Docs pages render identically whether or not `/app` has been visited first.

### 1.6 Icons

#### 1.6.1 Lucide render contract (`lucide-react@0.563.0`)

`<Icon className="X"/>` renders the following (exact markup per icon in Appendix B):

```html
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-{kebab-name} X" aria-hidden="true">…children…</svg>
```

* The `width`/`height` attributes are overridden by the `w-*`/`h-*` classes. Tailwind preflight makes `svg` `display:block; vertical-align:middle`.
* `lucide` and `lucide-*` classes have no CSS anywhere. Keeping them is optional, but recommended for DOM parity.
* `aria-hidden="true"` is always present (no a11y props are passed).

#### 1.6.2 Where each lucide icon is used (icon, class, location)

| Icon | `class` (after `lucide lucide-…`) | Location |
|---|---|---|
| `FileText` | `w-4 h-4` | Landing «Docs» button |
| `GitCommitVertical` | `w-4 h-4` | Landing «Changelog» button |
| `ArrowLeft` | `w-3 h-3 group-hover:-translate-x-1 transition-transform` | Landing footer «Return»; Download «Return to System»; Changelog fixed footer |
| `Terminal` | `w-4 h-4` | Docs sidebar: gettingStarted **and** login (same icon) |
| `Download` | `w-4 h-4` | Docs sidebar: download |
| `Keyboard` | `w-4 h-4` | Docs sidebar: controls |
| `Key` | `w-4 h-4` | Docs sidebar: apiSetup |
| `Briefcase` | `w-4 h-4` | Docs sidebar: all 5 providers |
| `Menu` / `X` | `w-5 h-5` | Docs mobile menu toggle (closed / open) |
| `Terminal` | `w-5 h-5` | Docs index section icon |
| `Command` | `w-5 h-5` | Docs controls section icon |
| `User` | `w-5 h-5` | Docs login section icon |
| `Download` | `w-5 h-5` | Docs download «standard» section icon |
| `Cpu` | `w-5 h-5 text-[#d49e38]` | Docs download recommended-env boxes (×2) |
| `Monitor` | `w-6 h-6 text-[#d49e38] hover:scale-110 transition-transform` | Docs download buttons (×4) |
| `Zap` | `w-5 h-5` | Docs download «lightweight» section icon |
| `GitCommitVertical` | `w-5 h-5` | Docs download «changelog» section icon |
| `Key` | `w-5 h-5` | Docs API index section icon |
| `GitCommitVertical` | `w-3 h-3` | Download page changelog link |
| `GitCommitVertical` | `w-6 h-6 text-[#d49e38]` | Changelog page title |
| `ArrowLeft` | `w-4 h-4` | Changelog page «ダウンロードへ / BACK TO DOWNLOAD» button |
| `ArrowUpRight` | `w-3 h-3 opacity-50 group-hover:opacity-100` | ElveltBadge only (out of scope) |

Imported but unused: `Monitor` in `docs/login/page.tsx`, and `Briefcase` in all five provider pages (those pages render no lucide icon). The 15 icons used by the pages are ArrowLeft, FileText, GitCommitVertical, Menu, X, Terminal, Keyboard, Key, Briefcase, Download, Command, User, Monitor, Zap and Cpu.

#### 1.6.3 Inline (non-lucide) SVGs, verbatim (no `xmlns`/`width`/`height` attributes in V1)

```html
<!-- S1: Heroicons-v1 "download" (stroke 2). Landing DOWNLOAD button -->
<svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"></path></svg>

<!-- S2: same path, stroke 1.5, w-6 h-6. Download page cards (×4) -->
<svg class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"></path></svg>

<!-- S3: GitHub mark. Landing GitHub button. NOTE the typo "A10.019 10019" (should be 10.019) -->
<svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24"><path fill-rule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10019 0 0022 12.017C22 6.484 17.522 2 12 2z" clip-rule="evenodd"></path></svg>

<!-- S4: Heroicons-v1 "eye" (stroke 2). Landing «Web版 / Web Version» button -->
<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path></svg>

<!-- S5: Heroicons-v1 "document-text" (stroke 2). Changelog page «Documentation» button -->
<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path></svg>
```

**V1 quirk Q2 (verified by rendering in headless Chrome 148):** the S3 typo `A10.019 10019` turns the final arc into an elliptical excursion clipped by the viewBox. The GitHub mark is drawn with a **filled square-ish bottom-right corner** instead of a round edge. Copy the path verbatim for parity, or fix it to `10.019` (Open question; trivial).

In Leptos `view!` use SVG elements (`<svg>`, `<path>`), or `<svg … inner_html=PATHS/>`. An HTML fragment parsed with an `<svg>` context creates SVG-namespace children. Do **not** wrap icons in an extra `<span>`, because flex gaps would change.

### 1.7 Images (all exist in V1 `public/` and in V2 `assets/public/static/`)

| V1 src | V2 src | Size (px) | Bytes | `alt` | Page |
|---|---|---|---|---|---|
| `/amadeus_logo.png` | `static/amadeus_logo.png` | 1024×1024 | 597235 | «Amadeus Logo» | Landing |
| `/amadeus_login.png` | `static/amadeus_login.png` | 1920×1080 | 577230 | «Amadeus Login Screen» | Docs login |
| `/OpenAI_STEP1.png` | `static/OpenAI_STEP1.png` | 1920×1080 | 49031 | «Step 1» | OpenAI |
| `/OpenAI_STEP2.png` | … | 1920×1080 | 115950 | «Step 2» | OpenAI |
| `/OpenAI_STEP3_1.png` `/OpenAI_STEP3_2.png` `/OpenAI_STEP3_3.png` | … | 1920×1080, 1920×1080, **1800×1012** | 76628, 104605, 88836 | «Step 3 - Part 1..3» | OpenAI |
| `/OpenAI_STEP4.png` | … | 1920×1080 | 355782 | «Step 4» | OpenAI |
| `/Gemini_STEP1.png` | … | 1920×1080 | 321213 | «Step 1» | Gemini |
| `/Gemini_STEP2_1.png` … `_3` | … | 1920×1080 | 112711, 146744, 127964 | «Step 2 - Part 1..3» | Gemini |
| `/Gemini_STEP3.png` | … | 1920×1080 | 357691 | «Step 3» | Gemini |
| `/Claude_STEP1.png`, `/Claude_STEP2.png` | … | 1920×1080 | 99728, 124017 | «Step 1», «Step 2» | Claude |
| `/Claude_STEP3_1.png` … `_3` | … | 1920×1080 | 92028, 110769, 117777 | «Step 3 - Part 1..3» | Claude |
| `/Claude_STEP4.png` | … | 1920×1080 | 357287 | «Step 4» | Claude |
| `/Groq_STEP1.png` | … | 1920×1080 | 110290 | «Step 1» | Groq |
| `/Groq_STEP2_1.png` … `_4` | … | 1920×1080 | 172445, 58938, 72595, 74871 | «Step 2 - Part 1..4» | Groq |
| `/Groq_STEP4.png` | … | 1920×1080 | 355038 | **«Step 3»** (V1 quirk Q5: file STEP4 for step 3; `Groq_STEP3.png` does not exist) | Groq |
| `/Vertex_STEP1.png` | … | 1920×1080 | 257280 | «Step 1» | Vertex |
| `/Vertex_STEP2_1.png` … `_4` | … | 1920×1080 | 213867, 131628, 146427, 175783 | «Step 2 - Part 1..4» | Vertex |
| `/Vertex_STEP3_1.png` … `_3` | … | **1920×1079**, 1920×1080, 1920×1080 | 227121, 181043, 116818 | «Step 3 - Part 1..3» | Vertex |
| `/Vertex_STEP4_1.png` … `_6` | … | 1920×1080 | 230456, 301316, 180124, 297939, 181907, 159836 | «Step 4 - Part 1..6» | Vertex |
| `/Vertex_STEP5.png` | … | 1920×1080 | 46850 | «Step 5» | Vertex |
| `/Vertex_STEP6_1.png`, `_2` | … | 1920×1080 | 218112, 381816 | «Step 6 - Part 1..2» | Vertex |

* No `<img>` has `width`/`height`/`loading`/`decoding` attributes. All load eagerly, so there is layout shift on load. Page weights: Vertex ≈ 3.45 MB (17 images), Gemini ≈ 1.07 MB, Claude ≈ 0.90 MB, Groq ≈ 0.84 MB, OpenAI ≈ 0.79 MB. Adding `loading="lazy"` in V2 is an optional improvement with no visual change (Open question).
* V2 uses base-relative `src="static/…"`. The `<base href>` that Trunk injects makes this resolve to `/amadeus/static/…` (web) or `/static/…` (desktop).

### 1.8 External URLs (verbatim)

| URL | Where | Opening |
|---|---|---|
| `https://github.com/dsuv-sg/RealAmadeus` | Landing «GitHubリポジトリ / GitHub Repository» | `target="_blank" rel="noopener noreferrer"` |
| `https://github.com/dsuv-sg/RealAmadeus/releases/download/RealAmadeusV1.4/RealAmadeus_V1.4U_Win64.zip` | Download page card; Docs download button | same tab (no target), so the browser downloads the file and stays on the page |
| `https://github.com/dsuv-sg/RealAmadeus/releases/download/RealAmadeusV1.4/RealAmadeus_V1.4U_MacUniversal.zip` | same | same tab |
| `https://github.com/dsuv-sg/RealAmadeus/releases/download/RealAmadeusV1.4/RealAmadeus_V1.4U_Linux.zip` | same | same tab |
| `https://github.com/dsuv-sg/RealAmadeusQT/releases/download/RealAmadeusV1.4/RealAmadeus_V1.4Q_Win64.zip` | Download page lightweight card; Docs download lightweight button | same tab |
| `https://github.com/dsuv-sg/RealAmadeus/releases` | dictionary `fallbackStandard` | `_blank` |
| `https://github.com/dsuv-sg/RealAmadeusQT/releases` | dictionary `fallbackLightweight` | `_blank` |
| `https://platform.openai.com/` | OpenAI step 1 | `_blank` |
| `https://aistudio.google.com/` | Gemini step 1 | `_blank` |
| `https://console.anthropic.com/` | Claude step 1 | `_blank` |
| `https://console.groq.com/` | Groq step 1 | `_blank` |
| `https://console.cloud.google.com/` | Vertex step 1 | `_blank` |
| `https://dl.google.com/dl/cloudsdk/channels/rapid/GoogleCloudSDKInstaller.exe?hl=ja` | Vertex step 5 | `_blank` |
| `/` | Landing «Return» | Next `<Link>` (client-side in V1); a plain full navigation outside `{base}` in V2 |

V1 constant names in `docs/download/page.tsx`: `windowsUrl`, `macUrl`, `linuxUrl`, `windowsLiteUrl`. `download/page.tsx` repeats the same four URLs inline.

---

## 2. Shared "terminal chrome" (Landing, Download, Changelog)

Each of the three pages contains an identical copy of the following. In V2, factor it into components.

### 2.1 Clock (`time` state)

```ts
const [time, setTime] = useState('');
useEffect(() => {
  const updateTime = () => {
    const now = new Date();
    setTime(now.toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit', fractionalSecondDigits: 2 }));
  };
  updateTime();                                   // immediately on mount
  const interval = setInterval(updateTime, 50);   // every 50 ms
  return () => clearInterval(interval);           // on unmount
}, []);
```

* Output format: `HH:MM:SS.cc` in **local time**. The hour is `00`–`23` (resolved `hourCycle` = `h23`; verified with Node 22 and Chrome 148). `cc` = **truncated** centiseconds `floor(ms / 10)`, zero-padded to 2 digits. Examples: 00:05:07.379 → «00:05:07.37»; 09:03:07.005 → «09:03:07.00»; 23:59:59.999 → «23:59:59.99». Older engines printed «24» for the midnight hour under `hour12:false`. V2 should always print 00–23.
* The initial value is `''`; the first value appears synchronously in the mount effect. In V2 CSR, initialise the signal with the current formatted time, or `''` and set it immediately. There is no visible difference.
* The 50 ms period means ~20 updates per second, with displayed centiseconds advancing by ~5.
* Rust: `format!("{:02}:{:02}:{:02}.{:02}", h, m, s, ms / 10)` from `js_sys::Date::new_0()` using `get_hours()`, `get_minutes()`, `get_seconds()` and `get_milliseconds()` (local). Do not use Intl.

### 2.2 Binary background (and the re-randomization quirk)

JSX (landing and download; the changelog variant differs as noted):

```tsx
<div className="fixed inset-0 z-0 opacity-20 pointer-events-none overflow-hidden font-mono text-2xl leading-normal text-[#888]">
  <div className="absolute inset-0 bg-[linear-gradient(rgba(0,0,0,0)_0%,rgba(0,0,0,1)_90%)] z-10" />
  <div className="animate-scroll-bg absolute top-0 left-0 w-full h-[200%] grid grid-cols-2 md:grid-cols-4 gap-8 p-4">
    {Array.from({ length: 16 }).map((_, colIndex) => (
      <div key={colIndex} className="flex flex-col gap-2 break-all">
        {Array.from({ length: 40 }).map((_, i) => (
          <div key={i} className="opacity-50">
            {i % 3 === 0 ? '' : `/${Math.random().toString(2).substring(2, 20)}`}
            {i % 5 === 0 && <div className="h-4" />} {/* Spacer */}
          </div>
        ))}
      </div>
    ))}
  </div>
</div>
```

Exact DOM to produce (V2):

```html
<div class="fixed inset-0 z-0 opacity-20 pointer-events-none overflow-hidden font-mono text-2xl leading-normal text-[#888]">
  <div class="absolute inset-0 bg-[linear-gradient(rgba(0,0,0,0)_0%,rgba(0,0,0,1)_90%)] z-10"></div>
  <div class="animate-scroll-bg absolute top-0 left-0 w-full h-[200%] grid grid-cols-2 md:grid-cols-4 gap-8 p-4">
    <!-- 16 columns, c = 0..15 -->
    <div class="flex flex-col gap-2 break-all">
      <!-- 40 cells, i = 0..39 -->
      <div class="opacity-50">{cell_text(i)}{ if i % 5 == 0 { <div class="h-4"></div> } }{ if TRAILING_SPACE { " " } }</div>
    </div>
  </div>
</div>
```

* `cell_text(i)` is `""` if `i % 3 == 0`, otherwise `"/" + bits18()`. Per column: 26 text cells (i ∉ {0,3,…,39}, 14 multiples of 3) and 8 spacers (i ∈ {0,5,…,35}). Totals: 416 strings and 128 spacers.
* `bits18()` reproduces `Math.random().toString(2).substring(2, 20)` exactly. Let `r = Math.random()` (in V2: `js_sys::Math::random()`), `scaled = r * 262144.0` (2^18; exact), `bits = floor(scaled)`, `s = format!("{bits:018b}")`. If `scaled.fract() == 0.0` (the binary expansion ends within 18 digits, e.g. r=0.5 or r=0), strip trailing `'0'`s from `s`, because JS prints no trailing zeros: `0.5` → «1» and `0` → «» (cell «/»). In practice the result is always 18 digits; 2,000,000 samples in Node gave 18 every time.
* **TRAILING_SPACE.** Landing and Download have `{/* Spacer */}` after the spacer expression on the same line, so JSX emits an extra `" "` text child in **every** cell (verified in the compiled chunks). Changelog has no comment, so there is no space. The space collapses to zero height and has **no visible effect**. Reproducing it is optional; it is listed for DOM parity.
* **V1 quirk Q3: re-randomization at ~20 Hz.** `Math.random()` is called during render with no memoization, and the page re-renders on every clock tick (state `time` changes every 50 ms) and on every locale toggle. As a result **all 416 strings are regenerated about every 50 ms**: the background is a constantly flickering binary field while the whole grid slowly scrolls. V2 must regenerate every text cell on each clock tick. Recommended: a shared `tick` signal (or the clock signal itself) tracked by each cell's text closure. Spacer and space nodes are static.
* Scroll: `animation: scroll-bg 60s linear infinite`, translating from 0 to −50% of the grid's own height (`h-[200%]` = 200% of the fixed viewport-size parent, so −50% ≈ one viewport height) over 60 s. It then **jumps** back to 0 (hard loop; the content is not periodic). The grid content (4 rows × ~1400 px on desktop) overflows its 200vh box and is clipped by the parent's `overflow-hidden`.
* Layering: the parent has `fixed inset-0 z-0 opacity-20` and text `#888` at `text-2xl` (24px) with `leading-normal` (1.5). The gradient child (`z-10`) fades to black toward the bottom (0 → 1 alpha at 90%). `pointer-events-none`. `break-all` makes the 19-character strings wrap inside narrow columns: on mobile there are 2 columns, each about `(100vw − 2·16px − 32px)/2` wide.
* There is no SSR in V2 (CSR only), so V1's hydration mismatch (server and client random strings differ) has no counterpart.

### 2.3 Grid overlay (identical on all three pages)

```html
<div class="fixed inset-0 pointer-events-none z-0 opacity-10" style="background-image: linear-gradient(#333 1px, transparent 1px), linear-gradient(90deg, #333 1px, transparent 1px); background-size: 40px 40px;"></div>
```

### 2.4 Header status bar (identical on all three pages)

```html
<div class="fixed top-0 left-0 w-full p-8 flex justify-between items-start z-50 opacity-80 hover:opacity-100 transition-opacity">
  <div class="flex flex-col gap-1 text-[10px] tracking-[0.3em] font-bold">
    <div class="flex items-center gap-2 text-[#d49e38]">
      <span class="w-1.5 h-1.5 bg-[#d49e38] animate-pulse rounded-full"></span>
      SYSTEM_ONLINE
    </div>
    <span class="opacity-50 font-mono">{time}</span>
  </div>
  <button class="text-[10px] tracking-[0.2em] border border-[#d49e38]/30 px-3 py-1 hover:bg-[#d49e38] hover:text-black transition-all">{locale == Ja ? "EN" : "JP"}</button>
</div>
```

* `on:click` on the button calls `toggle_language()` (§1.4). The `<button>` has no `type` attribute, which is harmless because there is no form.
* The bar is a **full-width fixed strip** (≈ 32px padding + content + 32px) at `z-50` without `pointer-events-none`. It intercepts clicks on whatever scrolls beneath it (V1 quirk; reproduce).
* «SYSTEM_ONLINE» is a literal, not `t.common.systemOnline`.

---

## 3. Pages

### 3.1 Landing — `/amadeus` (`AmadeusPage`)

State: `time` (§2.1). Context: `t`, `locale`, `toggleLanguage`. No props. No other effects.

```html
<main data-site-page="landing" class="min-h-screen bg-[#050505] text-[#d49e38] font-mono relative overflow-hidden flex flex-col justify-center items-center selection:bg-[#d49e38] selection:text-black">
  {BinaryBackground trailing_space=true}   <!-- §2.2 -->
  {GridOverlay}                            <!-- §2.3 -->
  {StatusHeader}                           <!-- §2.4 -->

  <!-- Central Content -->
  <div class="relative z-10 flex flex-col items-center justify-center p-12 w-full max-w-4xl">
    <!-- Logo Container (No Rings as requested) -->
    <div class="relative w-64 h-64 md:w-96 md:h-96 mb-12 flex items-center justify-center">
      <img src="{static}/amadeus_logo.png" alt="Amadeus Logo" class="w-full h-full object-contain filter drop-shadow-[0_0_20px_rgba(212,158,56,0.3)] animate-pulse" style="animation-duration: 4s;">
    </div>
    <!-- Status Message -->
    <div class="text-center space-y-4">
      <h1 class="text-xl md:text-2xl font-bold tracking-[0.3em] text-[#d49e38] text-shadow-glow uppercase opacity-90">{t.amadeus.release.title}</h1>
      <div class="h-px w-24 bg-gradient-to-r from-transparent via-[#d49e38]/50 to-transparent mx-auto"></div>
      <p class="text-xs md:text-sm text-[#d49e38]/80 tracking-widest font-mono max-w-md mx-auto leading-loose animate-pulse">{t.amadeus.release.message}</p>
    </div>
    <!-- Download, Repo, Docs and Changelog Buttons -->
    <div class="mt-12 flex flex-wrap justify-center gap-4 w-full max-w-4xl font-mono">
      <a href="{base}/download" class="px-10 py-4 border border-[#d49e38]/80 text-[#d49e38] text-sm tracking-[0.3em] font-bold hover:bg-[#d49e38] hover:text-black transition-all glow-on-hover flex items-center justify-center gap-2 group uppercase">
        {S1 svg}{t.amadeus.release.download}
      </a>
      <a href="https://github.com/dsuv-sg/RealAmadeus" target="_blank" rel="noopener noreferrer" class="px-6 py-4 border border-[#d49e38]/30 text-[#d49e38]/70 text-sm tracking-widest hover:border-[#d49e38] hover:text-[#d49e38] transition-all flex items-center justify-center gap-2">
        {S3 svg}{t.amadeus.release.githubRepo}
      </a>
      <a href="{base}/docs" class="px-6 py-4 border border-[#d49e38]/30 text-[#d49e38]/70 text-sm tracking-widest hover:border-[#d49e38] hover:text-[#d49e38] transition-all flex items-center justify-center gap-2">
        {FileText "w-4 h-4"}{t.amadeus.release.docs}
      </a>
      <a href="{base}/changelog" class="px-6 py-4 border border-[#d49e38]/30 text-[#d49e38]/70 text-sm tracking-widest hover:border-[#d49e38] hover:text-[#d49e38] transition-all flex items-center justify-center gap-2">
        {GitCommitVertical "w-4 h-4"}{t.amadeus.docs.changelog.title}
      </a>
      <a href="{base}/app" class="px-6 py-4 border border-[#d49e38]/80 text-[#d49e38] text-sm tracking-widest font-bold hover:bg-[#d49e38] hover:text-black transition-all flex items-center justify-center gap-2">
        {S4 svg}{t.amadeus.release.webVersion}
      </a>
    </div>
  </div>

  <!-- Footer (direct child of <main>, after the central content) -->
  <div class="relative z-10 mt-12 mb-8 w-full text-center">
    <a href="/" class="inline-flex items-center gap-2 text-[10px] tracking-[0.3em] text-[#d49e38]/40 hover:text-[#d49e38] transition-colors uppercase group">
      {ArrowLeft "w-3 h-3 group-hover:-translate-x-1 transition-transform"}
      <span>Return</span>
    </a>
  </div>
</main>
```

Displayed values. The `uppercase` class affects only Latin text.

| Element | ja | en |
|---|---|---|
| h1 `release.title` | «SYSTEM V1.4 RELEASED» | «SYSTEM V1.4 RELEASED» |
| p `release.message` (pulsing 2s) | «Real Amadeus V1.4が正式にリリースされました。» | «REAL AMADEUS V1.4 HAS BEEN OFFICIALLY RELEASED.» |
| DOWNLOAD (`uppercase`) | «ダウンロード» | «DOWNLOAD» |
| GitHub | «GitHubリポジトリ» | «GitHub Repository» |
| Docs | «ドキュメント» | «Documentation» |
| Changelog | «変更履歴» | «CHANGELOG» |
| Web | «Web版» | «Web Version» |

Behaviour:

* The DOWNLOAD button has **no glow** (Q1). On hover it gets a gold background and black text, so the stroke-based S1 icon turns black too.
* «Web版» goes to the in-SPA route `/app`.
* «Return» is a plain `<a href="/">`. On Web (base `/amadeus`) leptos_router does not intercept it, because it is outside the base, so it becomes a full navigation to the portfolio. On desktop (base `""`), `/` is the landing itself. See §7.
* Mobile: logo 16rem (`w-64 h-64`); md+: 24rem. The buttons wrap (`flex-wrap`).

### 3.2 Download — `/amadeus/download` (`AmadeusDownloadPage`)

State: `time` (§2.1). Same chrome as the landing.

```html
<main data-site-page="download" class="min-h-screen bg-[#050505] text-[#d49e38] font-mono relative overflow-hidden flex flex-col justify-center items-center selection:bg-[#d49e38] selection:text-black">
  {BinaryBackground trailing_space=true}
  {GridOverlay}
  {StatusHeader}

  <!-- Central Content -->
  <div class="relative z-10 flex flex-col items-center justify-center p-12 w-full max-w-5xl">
    <!-- Page Title -->
    <div class="text-center space-y-4 mb-10">
      <h1 class="text-2xl md:text-3xl font-bold tracking-[0.4em] text-[#d49e38] text-shadow-glow uppercase opacity-90">{t.amadeus.release.download}</h1>
      <div class="h-px w-32 bg-gradient-to-r from-transparent via-[#d49e38]/50 to-transparent mx-auto"></div>
      <p class="text-[10px] md:text-xs text-[#d49e38]/60 tracking-[0.3em] font-mono uppercase">Select your platform to initialize deployment</p>
    </div>
    <!-- Changelog Link -->
    <div class="mb-16">
      <a href="{base}/changelog" class="inline-flex items-center gap-2 text-[10px] tracking-[0.3em] text-[#d49e38]/40 hover:text-[#d49e38] transition-colors uppercase border border-[#d49e38]/20 px-4 py-2 hover:border-[#d49e38]/60">
        {GitCommitVertical "w-3 h-3"}{t.amadeus.docs.changelog.title}
      </a>
    </div>
    <!-- Platform Selection -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-12 w-full max-w-5xl">
      <!-- Left Side: Normal Version (Universal) -->
      <div class="space-y-8">
        <div class="flex items-center gap-4 mb-10">
          <div class="h-px flex-1 bg-gradient-to-r from-transparent to-[#d49e38]/30"></div>
          <h2 class="text-[11px] tracking-[0.5em] font-bold opacity-60 uppercase whitespace-nowrap px-4">Universal Edition</h2>
          <div class="h-px flex-1 bg-gradient-to-l from-transparent to-[#d49e38]/30"></div>
        </div>
        <div class="flex flex-col gap-4">
          {DownloadCard(href=WIN_U,   caption="Deployment Package", label=t.amadeus.release.windowsDownload, badge="V1.4U")}
          {DownloadCard(href=MAC_U,   caption="Deployment Package", label=t.amadeus.release.macosDownload,   badge="V1.4U")}
          {DownloadCard(href=LINUX_U, caption="Deployment Package", label=t.amadeus.release.linuxDownload,   badge="V1.4U")}
        </div>
      </div>
      <!-- Right Side: Lightweight Version -->
      <div class="space-y-8">
        <div class="flex items-center gap-4 mb-10">
          <div class="h-px flex-1 bg-gradient-to-r from-transparent to-[#d49e38]/30"></div>
          <h2 class="text-[11px] tracking-[0.5em] font-bold opacity-60 uppercase whitespace-nowrap px-4">Lightweight Edition</h2>
          <div class="h-px flex-1 bg-gradient-to-l from-transparent to-[#d49e38]/30"></div>
        </div>
        <div class="flex flex-col gap-4">
          {DownloadCard(href=WIN_Q, caption="Optimized Deployment", label=t.amadeus.release.lightweightWindowsDownload, badge="V1.4Q")}
        </div>
      </div>
    </div>
    <div class="mt-12">
      <a href="{base}" class="inline-flex items-center gap-2 text-[10px] tracking-[0.3em] text-[#d49e38]/40 hover:text-[#d49e38] transition-colors uppercase group">
        {ArrowLeft "w-3 h-3 group-hover:-translate-x-1 transition-transform"}
        <span>Return to System</span>
      </a>
    </div>
  </div>
</main>
```

`DownloadCard(href, caption, label, badge)`:

```html
<a href="{href}" class="px-6 py-6 border border-[#d49e38]/40 text-[#d49e38] hover:bg-[#d49e38] hover:text-black transition-all glow-on-hover flex items-center justify-between group bg-black/40 backdrop-blur-sm">
  <div class="flex items-center gap-4">
    {S2 svg}
    <div class="flex flex-col items-start">
      <span class="text-[9px] tracking-widest opacity-50 uppercase mb-0.5">{caption}</span>
      <span class="text-sm tracking-[0.2em] font-bold">{label}</span>
    </div>
  </div>
  <span class="text-[10px] opacity-40 group-hover:opacity-100 transition-opacity uppercase font-bold">{badge}</span>
</a>
```

* `WIN_U`, `MAC_U`, `LINUX_U` and `WIN_Q` are the four zip URLs in §1.8. The links have no `target`, so the download starts in the same tab.
* The cards **glow** on hover with `box-shadow 0 0 30px rgba(212,158,56,.3)`, unscoped by `(hover:hover)`.
* Labels: ja «Windows», «MacOS», «Linux», «軽量化版Windows»; en «Windows», «MacOS», «Linux», «Lightweight Windows». The title shows «ダウンロード» / «DOWNLOAD».
* Layout: mobile has one column with the Lightweight block below Universal; md+ has two columns.
* `{base}` with an empty path: Web → `/amadeus`, desktop → `/`.
* §6 proposes the V2 content for this page.

### 3.3 Changelog — `/amadeus/changelog` (`ChangelogPage`)

State: `time` (§2.1).

```html
<main data-site-page="changelog" class="min-h-screen bg-[#050505] text-[#d49e38] font-mono relative overflow-hidden flex flex-col items-center selection:bg-[#d49e38] selection:text-black">
  {BinaryBackground trailing_space=false}
  {GridOverlay}
  {StatusHeader}

  <!-- Central Content -->
  <div class="relative z-10 flex flex-col items-center justify-center p-12 w-full max-w-4xl pt-32 pb-24">
    <!-- Page Title -->
    <div class="text-center space-y-4 mb-16">
      <div class="flex items-center justify-center gap-3 mb-2">
        {GitCommitVertical "w-6 h-6 text-[#d49e38]"}
        <h1 class="text-2xl md:text-3xl font-bold tracking-[0.4em] text-[#d49e38] text-shadow-glow uppercase opacity-90">{t.amadeus.docs.changelog.title}</h1>
      </div>
      <div class="h-px w-32 bg-gradient-to-r from-transparent via-[#d49e38]/50 to-transparent mx-auto"></div>
      <p class="text-xs md:text-sm text-[#d49e38]/80 tracking-widest font-mono max-w-md mx-auto leading-loose">{t.amadeus.docs.changelog.desc}</p>
    </div>
    <!-- Changelog Timeline -->
    <div class="w-full max-w-2xl space-y-12">
      <!-- for ver in t.amadeus.docs.changelog.versions (6 entries, V1.4 … V1.0, in dictionary order) -->
      <div class="relative pl-8 border-l-2 border-[#d49e38]/30">
        <div class="absolute -left-[9px] top-0 w-4 h-4 rounded-full bg-[#050505] border-2 border-[#d49e38]"></div>
        <div class="mb-4"><span class="text-xl font-bold text-[#d49e38] tracking-widest">{ver.version}</span></div>
        <ul class="space-y-3">
          <!-- for item in ver.items -->
          <li class="flex items-start gap-3 text-sm opacity-80 leading-relaxed">
            <span class="text-[#d49e38] mt-1.5 text-[8px]">◆</span>
            <span>{item}</span>
          </li>
        </ul>
      </div>
    </div>
    <!-- Navigation -->
    <div class="mt-20 flex flex-col md:flex-row gap-4 w-full max-w-3xl justify-center items-center font-mono">
      <a href="{base}/download" class="w-full md:w-auto px-6 py-4 border border-[#d49e38]/30 text-[#d49e38]/70 text-sm tracking-widest hover:border-[#d49e38] hover:text-[#d49e38] transition-all flex items-center justify-center gap-2">
        {ArrowLeft "w-4 h-4"}{locale == Ja ? "ダウンロードへ" : "BACK TO DOWNLOAD"}
      </a>
      <a href="{base}/docs/download" class="w-full md:w-auto px-6 py-4 border border-[#d49e38]/30 text-[#d49e38]/70 text-sm tracking-widest hover:border-[#d49e38] hover:text-[#d49e38] transition-all flex items-center justify-center gap-2">
        {S5 svg}{t.amadeus.release.docs}
      </a>
    </div>
  </div>

  <!-- Footer (direct child of <main>) -->
  <div class="fixed bottom-8 left-0 w-full text-center z-50">
    <a href="{base}" class="inline-flex items-center gap-2 text-[10px] tracking-[0.3em] text-[#d49e38]/40 hover:text-[#d49e38] transition-colors uppercase group">
      {ArrowLeft "w-3 h-3 group-hover:-translate-x-1 transition-transform"}
      <span>{locale == Ja ? "戻る" : "Return"}</span>
    </a>
  </div>
</main>
```

* `ver.product` is **not** rendered.
* The «Documentation» button goes to `/docs/download`, which is the page that also embeds the changelog.
* The fixed footer strip at `bottom-8` is full width with no `pointer-events-none`, so it blocks clicks beneath it (V1 quirk). `pb-24` leaves room for it at the end of the page.
* `<main>` has no `justify-center` (unlike Landing/Download), so content starts at the top with `pt-32`.

### 3.4 Docs layout (`DocsLayout({ children })`)

#### 3.4.1 Route table (sidebar order, verbatim)

| # | V1 path | V2 path (base-relative) | Label key | ja label | en label | Icon (`w-4 h-4`) | `exact` | sub-item |
|---|---|---|---|---|---|---|---|---|
| 1 | `/amadeus/docs` | `/docs` | `docs.sidebar.gettingStarted` | «はじめに» | «Getting Started» | Terminal | **true** | no |
| 2 | `/amadeus/docs/download` | `/docs/download` | `docs.sidebar.download` | «ダウンロード» | «Download» | Download | false | no |
| 3 | `/amadeus/docs/login` | `/docs/login` | `docs.sidebar.login` | «ログイン方法» | «Login Instructions» | Terminal | false | no |
| 4 | `/amadeus/docs/controls` | `/docs/controls` | `docs.sidebar.controls` | «操作方法» | «Controls» | Keyboard | false | no |
| 5 | `/amadeus/docs/api` | `/docs/api` | `docs.sidebar.apiSetup` | «APIの取得と設定» | «API Setup» | Key | **true** | no |
| 6 | `/amadeus/docs/api/openai` | `/docs/api/openai` | `docs.sidebar.apiOpenAI` | «OpenAI API» | «OpenAI API» | Briefcase | false | **yes** |
| 7 | `/amadeus/docs/api/gemini` | `/docs/api/gemini` | `docs.sidebar.apiGemini` | «Google Gemini API» | «Google Gemini API» | Briefcase | false | **yes** |
| 8 | `/amadeus/docs/api/claude` | `/docs/api/claude` | `docs.sidebar.apiClaude` | «Anthropic Claude API» | «Anthropic Claude API» | Briefcase | false | **yes** |
| 9 | `/amadeus/docs/api/groq` | `/docs/api/groq` | `docs.sidebar.apiGroq` | «Groq API» | «Groq API» | Briefcase | false | **yes** |
| 10 | `/amadeus/docs/api/vertex` | `/docs/api/vertex` | `docs.sidebar.apiVertex` | «Google Vertex AI» | «Google Vertex AI» | Briefcase | false | **yes** |

#### 3.4.2 Algorithms

```ts
const isActive = (path, exact) => exact ? pathname === path : pathname.startsWith(path);   // plain STRING prefix
const isSubItem = route.path.split('/').length > 4;                                        // '/amadeus/docs/api/openai' → 5 parts
```

* `pathname` comes from Next's `usePathname()`. It never has a trailing slash, because Next 308-redirects slash URLs, and it never has a query string.
* V2: `current = strip_base(location.pathname)` with any trailing `/` removed (except for the root). Then `is_active(current, item.path, item.exact)` uses the same string semantics. Do **not** rely on leptos_router `<A>`'s `aria-current`, which uses segment-wise matching. The results would match for these routes, but classes must come from the V1 rule.
* `isSubItem` depends on the absolute path depth. With base-relative V2 paths the threshold would be `> 3`. Store an explicit `sub_item: bool` instead (true for items 6–10).
* Active state examples: on `/docs/api/openai`, item 6 is active; item 5 is **not** (exact); item 1 is not (exact). On `/docs/download`, item 2 is active. On `/docs`, only item 1 is active.

#### 3.4.3 State and events

* `is_mobile_menu_open: bool`, default `false`.
* Mobile toggle button: `on:click` → `open = !open`.
* Every nav item: `on:click` → `open = false`, then navigation; the router handles the anchor.
* The «← back» link has **no** click handler. Navigating to `{base}` unmounts the layout, so the state is dropped.
* Both language toggle buttons (desktop and mobile) call `toggle_language()`.
* There are no keyboard shortcuts, no focus management, no `aria-expanded`/`aria-label` (the toggle's only content is an `aria-hidden` icon), and no effects or timers.
* Navigation scrolls to the top. V1 Next scrolls the new page segment into view; leptos_router `<A>` and intercepted `<a>` default to scrolling to the top, which is equivalent here.

#### 3.4.4 DOM

```html
<div class="min-h-screen bg-black text-[#d49e38] font-mono selection:bg-[#d49e38] selection:text-black md:pt-20">
  <!-- Desktop Language Toggle -->
  <div class="hidden md:block fixed top-8 right-8 z-50">
    <button class="text-[10px] tracking-[0.2em] border border-[#d49e38]/30 bg-black/50 backdrop-blur-sm px-3 py-1 hover:bg-[#d49e38] hover:text-black transition-all">{locale == Ja ? "EN" : "JP"}</button>
  </div>

  <div class="max-w-7xl mx-auto flex flex-col md:flex-row relative">
    <!-- Mobile Menu Toggle & Top Bar -->
    <div class="md:hidden sticky top-0 bg-black/90 backdrop-blur-md border-b border-[#d49e38]/20 flex items-center h-14 px-4 z-40">
      <div class="flex-1 flex flex-col justify-center">
        <span class="text-[10px] tracking-[0.3em] font-bold text-[#d49e38]/80 leading-none mb-1 uppercase">Project Amadeus</span>
        <span class="text-[8px] tracking-[0.2em] opacity-40 uppercase">Documentation</span>
      </div>
      <div class="flex items-center gap-3">
        <button class="text-[9px] border border-[#d49e38]/30 px-2 py-0.5 hover:bg-[#d49e38] hover:text-black transition-all uppercase tracking-tighter">{locale == Ja ? "EN" : "JP"}</button>
        <button class="p-1.5 border border-[#d49e38]/30 hover:bg-[#d49e38]/10 transition-colors">{open ? X "w-5 h-5" : Menu "w-5 h-5"}</button>
      </div>
    </div>

    <!-- Sidebar Navigation -->
    <aside class="{ASIDE_BASE} {open ? ASIDE_OPEN : ASIDE_CLOSED}">
      <div class="p-6 space-y-6">
        <a href="{base}" class="inline-flex items-center text-xs opacity-60 hover:opacity-100 transition-opacity mb-4">← {t.amadeus.docs.back}</a>
        <nav class="space-y-1">
          <!-- for item in DOCS_NAV -->
          <a href="{base}{item.path}" class="{NAV_BASE} {item.sub_item ? NAV_SUB : ''} {active ? NAV_ACTIVE : NAV_INACTIVE}" on:click=close_menu>
            <span class="{active ? 'opacity-100' : 'opacity-50'}">{icon "w-4 h-4"}</span>
            {label}
          </a>
        </nav>
      </div>
    </aside>

    <!-- Main Content Area -->
    <main class="flex-1 p-6 md:p-10 lg:p-16 min-h-screen">
      <div class="max-w-3xl">{<Outlet/>}</div>
    </main>
  </div>

  <!-- Global scanline overlay that matches amadeus page -->
  <div class="fixed inset-0 pointer-events-none z-50 mix-blend-overlay opacity-[0.03]" style="background-image: repeating-linear-gradient(0deg, transparent, transparent 1px, #d49e38 1px, #d49e38 2px);"></div>
</div>
```

Class constants (verbatim tokens, normalized whitespace; V1 builds them with multi-line template literals):

```text
ASIDE_BASE   = "w-full md:w-64 flex-shrink-0 border-r border-[#d49e38]/20 md:sticky md:top-20 md:self-start md:h-[calc(100vh-5rem)] md:overflow-y-auto transition-all duration-300 ease-in-out z-30 bg-black/95 backdrop-blur-sm"
ASIDE_OPEN   = "max-h-screen border-b border-[#d49e38]/20"
ASIDE_CLOSED = "max-h-0 md:max-h-screen overflow-hidden md:overflow-auto border-b-0"
NAV_BASE     = "flex items-center gap-3 px-3 py-2 text-sm transition-all"
NAV_SUB      = "ml-4 border-l border-[#d49e38]/20"
NAV_ACTIVE   = "bg-[#d49e38]/10 text-[#d49e38] border-l-2 border-l-[#d49e38] font-bold"
NAV_INACTIVE = "opacity-70 hover:opacity-100 hover:bg-[#d49e38]/5 border-l-2 border-l-transparent"
```

For Tailwind scanning, write each **full** combination as a literal, or keep the constants as literals and join them with spaces at runtime. Joining complete literal tokens is fine; synthesizing tokens is not. The relative precedence of `border-l` vs `border-l-2` and `border-[#d49e38]/20` vs `border-l-[#d49e38]` is decided by Tailwind's internal sort and is identical in V2.

Back-link text = «← » + `docs.back`: ja «← システムへ戻る», en «← RETURN TO SYSTEM».

Responsive and animation behaviour (all V1 quirks to keep):

* **md+**: the aside is always visible (`md:max-h-screen md:overflow-auto`), sticky below the 5rem top padding, and scrolls independently (`md:h-[calc(100vh-5rem)] md:overflow-y-auto`). The mobile bar is hidden; the desktop toggle is fixed at the top right over a `bg-black/50` blurred pill.
* **< md**: the aside is collapsed with `max-h-0 overflow-hidden`. Opening it sets `max-h-screen` and **removes** `overflow-hidden`. With `transition-all duration-300 ease-in-out` the max-height animates over 300 ms; while opening, the links overflow visibly over the content below until the box catches up. Closing re-adds `overflow-hidden` immediately and then animates to 0.
* The aside is **not** sticky on mobile. If the user has scrolled down, opening the menu expands it at the top of the document, off-screen (quirk).
* `border-r` also applies on mobile (right edge of a full-width aside). If the menu was left open and the window is widened, `border-b` remains on desktop.
* The scanline overlay is fixed at `z-50` with `mix-blend-overlay` at 3% opacity, and covers everything including the toggles. It is click-through.

### 3.5 `renderSection(icon, title, content)` (`docs/components.tsx`)

A plain function, not a component (no state).

```html
<section class="mb-14 relative group">
  <!-- Elegant glowing side border -->
  <div class="absolute -left-5 top-2 bottom-0 w-[2px] bg-gradient-to-b from-[#d49e38]/60 via-[#d49e38]/10 to-transparent"></div>
  <h2 class="text-xl md:text-2xl font-bold mb-6 flex items-center gap-3">
    <span class="relative flex items-center justify-center w-8 h-8 rounded-sm bg-black border border-[#d49e38]/30 overflow-hidden">
      <div class="absolute inset-0 bg-[#d49e38]/10 group-hover:bg-[#d49e38]/20 transition-colors duration-500"></div>
      <span class="text-[#d49e38] drop-shadow-[0_0_5px_rgba(212,158,56,0.6)] z-10">{icon}</span>
    </span>
    <span class="tracking-widest drop-shadow-[0_0_2px_rgba(212,158,56,0.3)] border-b border-[#d49e38]/20 pb-1 flex-1">{title}</span>
  </h2>
  <div class="text-sm md:text-base leading-loose text-[#d49e38]/80 pl-2 max-w-4xl">{content}</div>
</section>
```

`<div>` inside `<span>`/`<h2>` is invalid HTML nesting. That is fine for V2, which creates the DOM programmatically in CSR. Never round-trip this markup through an HTML parser or SSR. `group-hover` reacts to hovering the **whole section**.

### 3.6 Common docs page header (all 10 docs pages)

```html
<div class="animate-in fade-in duration-700">            <!-- page wrapper; the animation classes are no-ops (§1.5.1) -->
  <h1 class="text-3xl md:text-4xl font-bold mb-2 tracking-widest text-[#d49e38] drop-shadow-[0_0_8px_rgba(212,158,56,0.5)]">{page title}</h1>
  <div class="h-px w-full bg-gradient-to-r from-[#d49e38] to-transparent opacity-50 mb-10"></div>
  …page body…
</div>
```

### 3.7 Docs index — `/amadeus/docs` (`DocsIndex`)

Title `docs.gettingStarted.title`: ja «はじめに», en «GETTING STARTED».

```html
{header(docs.gettingStarted.title)}
<div class="grid grid-cols-1 gap-8">
  {section(Terminal "w-5 h-5", docs.gettingStarted.title,
    <p class="tracking-wide text-[#d49e38]/90">{docs.gettingStarted.desc}</p>
    <div class="relative mt-8 group bg-transparent">
      <div class="absolute inset-0 bg-gradient-to-r from-[#d49e38]/10 to-transparent opacity-50 border border-[#d49e38]/20 rounded-sm"></div>
      <!-- Terminal decorative header -->
      <div class="flex items-center justify-between px-3 py-1.5 border-b border-[#d49e38]/20 bg-[#d49e38]/5">
        <span class="text-[10px] tracking-widest opacity-60 font-bold">SYSTEM.LOGIN_INFO</span>
        <div class="flex gap-1">
          <div class="w-1.5 h-1.5 rounded-full bg-[#d49e38]/40"></div>
          <div class="w-1.5 h-1.5 rounded-full bg-[#d49e38]/40"></div>
        </div>
      </div>
      <div class="p-5 pl-6 border-l-2 border-[#d49e38] shadow-[0_0_20px_rgba(212,158,56,0.05)]">
        <p class="font-medium opacity-100 text-[#d49e38] tracking-widest leading-relaxed flex items-start gap-3">
          <span class="text-[#d49e38] animate-pulse mt-0.5">▶</span>
          {docs.gettingStarted.loginInfo}
        </p>
      </div>
    </div>
  )}
</div>
```

### 3.8 Docs download — `/amadeus/docs/download` (`DownloadPage` in `docs/download/page.tsx`)

```html
{header(docs.download.title)}                         <!-- «ダウンロード» / «DOWNLOAD» -->
<p class="mb-10 opacity-90 leading-relaxed text-sm md:text-base tracking-wide text-[#d49e38]/90">{docs.download.desc}</p>
<div class="grid grid-cols-1 gap-10">
  {section(Download "w-5 h-5", docs.download.standardTitle, STANDARD_BODY)}
  {section(Zap "w-5 h-5", docs.download.lightweightTitle, LIGHTWEIGHT_BODY)}
  {section(GitCommitVertical "w-5 h-5", docs.changelog.title, CHANGELOG_BODY)}
</div>
```

STANDARD_BODY:

```html
<p class="tracking-wide text-[#d49e38]/90 mb-6">{docs.download.standardDesc}</p>
<!-- Recommended Environment -->
<div class="flex items-center gap-3 mb-8 p-3 bg-[#d49e38]/5 border border-[#d49e38]/20 rounded-sm">
  {Cpu "w-5 h-5 text-[#d49e38]"}
  <div>
    <span class="text-xs tracking-widest opacity-60 uppercase font-bold block">{docs.download.recommendedEnv}</span>
    <span class="text-sm font-bold text-[#d49e38] tracking-wide">{docs.download.standardEnv}</span>
  </div>
</div>
<!-- Direct Download Buttons -->
<div class="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-10">
  {DocsDlButton(windowsUrl, docs.download.windows)}
  {DocsDlButton(macUrl,     docs.download.mac)}
  {DocsDlButton(linuxUrl,   docs.download.linux)}
</div>
<div class="mb-10 p-4 bg-[#d49e38]/5 border border-[#d49e38]/20 rounded-sm">
  <p class="text-sm opacity-80 leading-relaxed">{rich docs.download.fallbackStandard}</p>
</div>
<div class="space-y-12"><!-- docs.download.steps.slice(2) — ALWAYS EMPTY (V1 quirk Q4) --></div>
<!-- File Structure -->
<div class="mt-12 p-5 bg-[#d49e38]/5 border border-[#d49e38]/20 rounded-sm shadow-[0_0_15px_rgba(212,158,56,0.05)] relative overflow-hidden">
  <div class="absolute left-0 top-0 bottom-0 w-1 bg-[#d49e38]"></div>
  <h3 class="text-sm font-bold tracking-widest text-[#d49e38] mb-4 uppercase">{docs.download.fileStructure.title}</h3>
  <div class="space-y-4">
    <div class="flex flex-col gap-1">
      <span class="text-xs tracking-widest opacity-60 uppercase font-bold">Windows</span>
      <span class="text-sm opacity-80 leading-relaxed">{docs.download.fileStructure.windows}</span>
    </div>
    <div class="flex flex-col gap-1">
      <span class="text-xs tracking-widest opacity-60 uppercase font-bold">MacOS</span>
      <span class="text-sm opacity-80 leading-relaxed">{docs.download.fileStructure.mac}</span>
    </div>
    <div class="flex flex-col gap-1">
      <span class="text-xs tracking-widest opacity-60 uppercase font-bold">Linux</span>
      <span class="text-sm opacity-80 leading-relaxed">{docs.download.fileStructure.linux}</span>
    </div>
  </div>
</div>
<p class="mt-6 text-xs opacity-60 tracking-wide">{docs.download.notice}</p>
```

LIGHTWEIGHT_BODY is identical in structure except for:

* `lightweightDesc` replaces `standardDesc`, and `lightweightEnv` replaces `standardEnv`.
* The button grid (same 3-column grid classes) holds **one** button: `DocsDlButton(windowsLiteUrl, docs.download.windows)`.
* The fallback text is `fallbackLightweight`.
* The steps div is again empty.
* The file-structure box has only the «Windows» row, with `fileStructure.windowsQT`.
* The same `notice` paragraph follows.

`DocsDlButton(href, label)`:

```html
<a href="{href}" class="relative flex flex-col items-center gap-2 p-4 border border-[#d49e38]/30 bg-black/40 hover:bg-[#d49e38]/10 transition-all rounded-sm">
  {Monitor "w-6 h-6 text-[#d49e38] hover:scale-110 transition-transform"}
  <span class="text-sm font-bold tracking-widest text-[#d49e38]">{label}</span>
  <span class="text-[10px] tracking-wider opacity-60 border border-[#d49e38]/30 px-2 py-0.5 hover:bg-[#d49e38] hover:text-black transition-all">{docs.download.downloadBtn}</span>
</a>
```

Step item template (never rendered here because of Q4, but it is the same template as the provider pages without `mb-4` on `<p>` and without the image frame):

```html
<div class="relative pl-6 border-l-2 border-[#d49e38]/30">
  <div class="absolute -left-[9px] top-1 w-4 h-4 rounded-full bg-black border-2 border-[#d49e38]"></div>
  <h3 class="text-lg font-bold text-[#d49e38] mb-2">Step {i + 1}: {step.title}</h3>
  <p class="opacity-80 text-sm md:text-base leading-relaxed">{step.content}</p>
</div>
```

**V1 quirk Q4:** `steps` has exactly 2 entries, so `.slice(2)` is empty in both sections and the «Download the asset / Extract» steps are **never shown**. The empty `div.space-y-12` stays in the DOM; the margins collapse through it either way. Reproduce by rendering the empty div. The `arr` parameter of the map callback is unused.

CHANGELOG_BODY:

```html
<p class="tracking-wide text-[#d49e38]/90 mb-8">{docs.changelog.desc}</p>
<div class="space-y-10">
  <!-- for ver in docs.changelog.versions -->
  <div class="relative pl-6 border-l-2 border-[#d49e38]/30">
    <div class="absolute -left-[9px] top-0 w-4 h-4 rounded-full bg-black border-2 border-[#d49e38]"></div>
    <div class="mb-3"><span class="text-lg font-bold text-[#d49e38] tracking-widest">{ver.version}</span></div>
    <ul class="space-y-2">
      <li class="flex items-start gap-3 text-sm opacity-80 leading-relaxed">
        <span class="text-[#d49e38] mt-1.5 text-[8px]">◆</span>
        <span>{item}</span>
      </li>
    </ul>
  </div>
</div>
```

How this differs from the Changelog page timeline: `pl-6` vs `pl-8`, `bg-black` vs `bg-[#050505]` dot, `mb-3` vs `mb-4`, `text-lg` vs `text-xl`, `space-y-2` vs `space-y-3`, `space-y-10` vs `space-y-12`.

### 3.9 Docs login — `/amadeus/docs/login` (`LoginPage`)

Title `docs.loginDoc.title`: ja «システムログイン», en «SYSTEM LOGIN».

```html
{header(docs.loginDoc.title)}
<div class="grid grid-cols-1 gap-8">
  {section(User "w-5 h-5", docs.loginDoc.title,
    <p class="tracking-wide text-[#d49e38]/90 mb-6">{docs.loginDoc.desc}</p>
    <!-- Login Screen Image -->
    <div class="w-full relative rounded-sm overflow-hidden mb-8 border border-[#d49e38]/40 shadow-[0_0_20px_rgba(212,158,56,0.15)] group">
      <img src="{static}/amadeus_login.png" alt="Amadeus Login Screen" class="w-full h-auto object-cover opacity-85 group-hover:opacity-100 transition-opacity duration-500">
      <div class="absolute inset-0 pointer-events-none shadow-[inset_0_0_30px_rgba(0,0,0,0.8)]"></div>
      <div class="absolute inset-0 pointer-events-none bg-[linear-gradient(rgba(212,158,56,0.05)_1px,transparent_1px),linear-gradient(90deg,rgba(212,158,56,0.05)_1px,transparent_1px)] bg-[size:4px_4px]"></div>
    </div>
    <div class="space-y-4 max-w-xs bg-black/50 p-6 border border-[#d49e38]/30 relative rounded-sm shadow-[0_0_20px_rgba(212,158,56,0.05)]">
      <div class="flex items-center justify-between border-b border-[#d49e38]/20 pb-2">
        <span class="opacity-70 text-xs tracking-widest uppercase font-bold">{docs.loginDoc.idLabel}</span>
        <span class="text-[#d49e38] font-bold text-lg tracking-wider drop-shadow-[0_0_5px_rgba(212,158,56,0.5)]">{docs.loginDoc.idValue}</span>
      </div>
      <div class="flex items-center justify-between border-b border-[#d49e38]/20 pb-2">
        <span class="opacity-70 text-xs tracking-widest uppercase font-bold">{docs.loginDoc.passLabel}</span>
        <span class="text-[#d49e38] font-bold text-lg tracking-wider drop-shadow-[0_0_5px_rgba(212,158,56,0.5)]">{docs.loginDoc.passValue}</span>
      </div>
    </div>
  )}
</div>
```

Values (both locales): idLabel «User ID», idValue «Salieri», passLabel «Password», passValue «MakiseKurisu». The labels are displayed uppercase via CSS.

### 3.10 Docs controls — `/amadeus/docs/controls` (`ControlsPage`)

Title `docs.controls.title`: ja «キーボード操作», en «KEYBOARD CONTROLS».

```html
{header(docs.controls.title)}
<div class="grid grid-cols-1 gap-8">
  {section(Command "w-5 h-5", docs.controls.title,
    <p class="mb-8 tracking-wide text-[#d49e38]/90">{docs.controls.desc}</p>
    <div class="space-y-4">
      <!-- for item in docs.controls.items (5 rows) -->
      <div class="group flex flex-col sm:flex-row sm:items-center gap-4 border-b border-[#d49e38]/10 pb-4 transition-colors hover:bg-[#d49e38]/5 p-2 rounded-sm -ml-2">
        <div class="relative">
          <!-- Keycap styling -->
          <div class="absolute inset-0 bg-[#d49e38]/20 blur-md rounded-md scale-90 opacity-0 group-hover:opacity-100 transition-opacity"></div>
          <div class="relative bg-[#111] border-2 border-b-4 border-[#d49e38]/80 text-[#d49e38] font-bold px-4 py-1.5 rounded-lg text-xs min-w-[160px] text-center transform transition-transform group-hover:translate-y-[2px] group-hover:border-b-2 mt-0">{item.key}</div>
        </div>
        <span class="opacity-80 text-sm md:text-base tracking-wide flex-1">{item.desc}</span>
      </div>
    </div>
  )}
</div>
```

Note that `pb-4` and `p-2` conflict; Tailwind's sort decides, identically in V2. The keycap appears to press down on row hover (translate 2px, bottom border 4px → 2px).

Items: ja `Enter`/«決定 / ログインの実行», «Tab / 右クリック»/«システムメニューの開閉», «W A S D / 矢印»/«メニューや設定項目の移動», `Backspace`/«バックログを閉じる / 設定の保存と終了», «Alt + Enter / F11»/«フルスクリーン / ウィンドウモード切替». en: Appendix A.

### 3.11 Docs API setup — `/amadeus/docs/api` (`ApiSetupPage`)

Title `docs.api.title`: ja «API設定», en «API CONFIGURATION».

```html
{header(docs.api.title)}
<div class="grid grid-cols-1 gap-8">
  {section(Key "w-5 h-5", docs.api.title,
    <p class="mb-6 tracking-wide leading-relaxed text-[#d49e38]/90">{docs.api.desc}</p>
    <div class="bg-[#111] p-6 md:p-8 border border-[#d49e38]/30 relative overflow-hidden rounded-sm shadow-[0_0_30px_rgba(212,158,56,0.05)]">
      <div class="absolute -top-10 -right-10 w-40 h-40 bg-[#d49e38]/10 blur-3xl rounded-full"></div>
      <ol class="list-decimal pl-6 space-y-5 opacity-90 relative z-10 text-sm md:text-base marker:text-[#d49e38] marker:font-bold">
        <!-- for step in docs.api.steps (4 plain strings) -->
        <li class="pl-3 leading-relaxed tracking-wide text-[#d49e38]/80">{step}</li>
      </ol>
    </div>
    <div class="mt-8 p-4 bg-black/50 border-l-2 border-[#d49e38] flex gap-3 items-start opacity-70">
      <span class="text-xl">⚠️</span>
      <p class="text-sm md:text-xs leading-relaxed tracking-wide">{docs.api.notice}</p>
    </div>
  )}
</div>
```

V1 quirk: the notice text is `text-sm md:text-xs`, i.e. **smaller on desktop**.

### 3.12 Provider pages — `/amadeus/docs/api/{openai,gemini,claude,groq,vertex}`

All five pages share one template; only the dictionary key, the image table and the trailing divider differ. They do **not** use `renderSection`. `isConfigStep` (`i === arr.length - 1`) is computed but unused.

```html
<div class="animate-in fade-in duration-700">
  <h1 class="text-3xl md:text-4xl font-bold mb-2 tracking-widest text-[#d49e38] drop-shadow-[0_0_8px_rgba(212,158,56,0.5)]">{P.title}</h1>
  <div class="h-px w-full bg-gradient-to-r from-[#d49e38] to-transparent opacity-50 mb-10"></div>
  <p class="mb-8 opacity-90 leading-relaxed text-sm md:text-base">{P.desc}</p>
  <div class="space-y-12">
    <!-- for (i, step) in P.steps -->
    <div class="relative pl-6 border-l-2 border-[#d49e38]/30">
      <div class="absolute -left-[9px] top-1 w-4 h-4 rounded-full bg-black border-2 border-[#d49e38]"></div>
      <h3 class="text-lg font-bold text-[#d49e38] mb-2">Step {i + 1}: {step.title}</h3>
      <p class="opacity-80 text-sm md:text-base leading-relaxed mb-4">{rich-or-plain step.content}</p>
      <div class="w-full relative rounded-sm overflow-hidden shadow-[0_0_15px_rgba(212,158,56,0.1)] group border border-[#d49e38]/30">
        <!-- images for step i (table below); exactly one of: -->
        <!-- (a) single image -->
        <img src="{static}/{file}" alt="Step {i+1}" class="w-full h-auto object-cover opacity-85 group-hover:opacity-100 transition-opacity duration-300">
        <!-- (b) multiple images -->
        <div class="flex flex-col gap-2 bg-black/50 p-2">
          <img src="{static}/{file_j}" alt="Step {i+1} - Part {j+1}" class="w-full h-auto object-cover opacity-85 group-hover:opacity-100 transition-opacity duration-300 border border-[#d49e38]/20">
        </div>
        <!-- always: -->
        <div class="absolute inset-0 pointer-events-none shadow-[inset_0_0_20px_rgba(0,0,0,0.6)]"></div>
        <div class="absolute inset-0 pointer-events-none bg-[linear-gradient(rgba(212,158,56,0.03)_1px,transparent_1px),linear-gradient(90deg,rgba(212,158,56,0.03)_1px,transparent_1px)] bg-[size:4px_4px]"></div>
      </div>
    </div>
  </div>
  <!-- Models Info -->
  <div class="mt-12 p-5 bg-[#d49e38]/5 border border-[#d49e38]/20 rounded-sm shadow-[0_0_15px_rgba(212,158,56,0.05)] relative overflow-hidden">
    <div class="absolute left-0 top-0 bottom-0 w-1 bg-[#d49e38]"></div>
    <h3 class="text-sm font-bold tracking-widest text-[#d49e38] mb-4 uppercase">{docs.api.modelLabels.title}</h3>
    <div class="space-y-3">
      <div class="flex flex-col sm:flex-row sm:items-start gap-1 sm:gap-4">
        <span class="text-xs tracking-widest opacity-60 uppercase min-w-[120px] pt-1">{docs.api.modelLabels.available}:</span>
        <div class="flex flex-wrap gap-2 flex-1">
          <!-- for m in P.modelList.available -->
          <span class="text-xs font-mono bg-black/50 border border-[#d49e38]/30 px-2 py-1 rounded-sm text-[#d49e38]/80">{m}</span>
        </div>
      </div>
      <div class="flex flex-col sm:flex-row sm:items-center gap-1 sm:gap-4">
        <span class="text-xs tracking-widest opacity-60 uppercase min-w-[120px]">{docs.api.modelLabels.recommended}:</span>
        <span class="text-xs sm:text-sm font-mono font-bold text-[#d49e38] bg-[#d49e38]/10 px-2 py-1 rounded-sm border border-[#d49e38]/50 shadow-[0_0_8px_rgba(212,158,56,0.2)]">{P.modelList.recommended}</span>
      </div>
    </div>
  </div>
  <!-- Groq and Vertex ONLY: -->
  <div class="mt-12 pt-8 border-t border-white/10 flex justify-center"></div>
</div>
```

* `«{label}:»` is the label immediately followed by a colon: «利用可能:» / «AVAILABLE:» (CSS uppercase), «推奨:» / «RECOMMENDED:». Box title: «利用可能なモデル» / «SUPPORTED MODELS».
* Step heading: the literal «Step » + (i+1) + «: » + title, in both locales («Step 1: アカウント作成»).
* The image frame `div` is rendered for **every** step, even when the step has no image (never the case with the current data). Step counts are equal across locales: OpenAI 4, Gemini 3, Claude 4, Groq 3, Vertex 6.

Image table (`alt` derived as above; `[]` = single, `[a, b, …]` = multi):

| Provider (`P` key) | Step 1 | Step 2 | Step 3 | Step 4 | Step 5 | Step 6 | Trailing divider |
|---|---|---|---|---|---|---|---|
| OpenAI (`docs.apiOpenAI`) | `OpenAI_STEP1.png` | `OpenAI_STEP2.png` | [`OpenAI_STEP3_1.png`, `OpenAI_STEP3_2.png`, `OpenAI_STEP3_3.png`] | `OpenAI_STEP4.png` | — | — | no |
| Gemini (`docs.apiGemini`) | `Gemini_STEP1.png` | [`Gemini_STEP2_1.png`, `Gemini_STEP2_2.png`, `Gemini_STEP2_3.png`] | `Gemini_STEP3.png` | — | — | — | no |
| Claude (`docs.apiClaude`) | `Claude_STEP1.png` | `Claude_STEP2.png` | [`Claude_STEP3_1.png`, `Claude_STEP3_2.png`, `Claude_STEP3_3.png`] | `Claude_STEP4.png` | — | — | no |
| Groq (`docs.apiGroq`) | `Groq_STEP1.png` | [`Groq_STEP2_1.png` … `Groq_STEP2_4.png`] | **`Groq_STEP4.png`** (alt «Step 3») | — | — | — | **yes** |
| Vertex (`docs.apiVertex`) | `Vertex_STEP1.png` | [`Vertex_STEP2_1.png` … `_4`] | [`Vertex_STEP3_1.png` … `_3`] | [`Vertex_STEP4_1.png` … `_6`] | `Vertex_STEP5.png` | [`Vertex_STEP6_1.png`, `Vertex_STEP6_2.png`] | **yes** |

Titles: «OpenAI API», «Google Gemini API», «Anthropic Claude API», «Groq API», «Google Vertex AI» (same in both locales). Descriptions, steps and model lists are in Appendix A. Recommended models: `gpt-5.5`, `gemini-3.5-flash`, `claude-sonnet-latest`, `llama-3.3-70b-versatile`, `gemini-3.5-flash`.

---

## 4. Not-found page (fallback)

V1 has no custom `not-found.tsx`. Next renders `HTTPAccessErrorFallback(status=404, message="This page could not be found.")` inside the **root** layout (NOT inside the docs layout, even for `/amadeus/docs/xyz`). Verified from `next/dist/client/components/http-access-fallback/error-fallback.js` and `styles/access-error-styles.js`:

```html
<title>404: This page could not be found.</title>
<div style="font-family:system-ui,&quot;Segoe UI&quot;,Roboto,Helvetica,Arial,sans-serif,&quot;Apple Color Emoji&quot;,&quot;Segoe UI Emoji&quot;;height:100vh;text-align:center;display:flex;flex-direction:column;align-items:center;justify-content:center">
  <div>
    <style>body{color:#000;background:#fff;margin:0}.next-error-h1{border-right:1px solid rgba(0,0,0,.3)}@media (prefers-color-scheme:dark){body{color:#fff;background:#000}.next-error-h1{border-right:1px solid rgba(255,255,255,.3)}}</style>
    <h1 class="next-error-h1" style="display:inline-block;margin:0 20px 0 0;padding:0 23px 0 0;font-size:24px;font-weight:500;vertical-align:top;line-height:49px">404</h1>
    <div style="display:inline-block"><h2 style="font-size:14px;font-weight:400;line-height:49px;margin:0">This page could not be found.</h2></div>
  </div>
</div>
```

V2: `<Routes fallback=NotFoundPage>`. Two options:

* Reproduce the markup above, setting `document.title` to «404: This page could not be found.» while mounted and restoring «Amadeus» on cleanup. The inline `<style>` changes `body` globally while mounted; restore it on unmount, or scope it.
* Or redirect to `{base}` (Open question).

Use static routes per provider rather than an `api/:provider` param route, so that `/docs/api/foo` falls through to NotFound as in V1.

---

## 5. Timing, ordering and quirk summary

| # | Item | Value / behaviour |
|---|---|---|
| T1 | Clock | 50 ms `setInterval`; first update synchronous at mount; cleared on unmount |
| T2 | Clock format | `HH:MM:SS.cc`, local time, 00–23, centiseconds truncated |
| T3 | Binary re-randomization | All 416 strings regenerate on every clock tick (~20 Hz) and on locale toggle |
| T4 | Background scroll | `scroll-bg 60s linear infinite`, 0 → −50% of its own (200%) height, hard loop |
| T5 | Pulses | `animate-pulse` = 2 s cycle (status dot, landing message, docs «▶»); landing logo 4 s via inline `animation-duration` |
| T6 | Docs sidebar | `max-height` transition 300 ms ease-in-out (mobile) |
| T7 | Hover transitions | Tailwind default 150 ms; `duration-300` (provider images), `duration-500` (section icon background, login image) |
| Q1 | Landing DOWNLOAD glow | Never applies (styled-jsx does not tag `<Link>`) |
| Q2 | GitHub SVG | Typo `10019` gives a filled bottom-right corner |
| Q3 | Background flicker | See T3 |
| Q4 | Docs download steps | `slice(2)` of a 2-element array, so nothing renders |
| Q5 | Groq step 3 image | `Groq_STEP4.png` with alt «Step 3» |
| Q6 | Locale | Default `ja`, not persisted, survives SPA navigation, label `EN`/`JP` |
| Q7 | Docs entry animation | `animate-in fade-in` are no-ops |
| Q8 | Fixed strips | Header strip (top, all 3 chrome pages) and Changelog footer strip intercept clicks |
| Q9 | API notice size | `text-sm md:text-xs` |
| Q10 | Mobile docs menu | Not sticky; opens at document top even when scrolled; overflow visible during the open animation |
| Q11 | Two changelog datasets | The site changelog (this dictionary) is **not** the in-app ChangeLog panel (`lib/amadeus/locales/*.json` keys `changelog_v14…changelog_v10`). The texts differ: V1.3 «Implemented sleep motion for Live2D» vs «Implemented Live2D nodding animations». Keep both. |
| Q12 | Unused code | `Monitor` import (login), `Briefcase` import and `isConfigStep` (providers), `arr` param (docs download), `releasesPage`, `steps` |

---

## 6. Proposal: the V2 download page listing Windows / macOS / Linux (and Web) from GitHub Releases

Goal (user request): ship Windows, macOS, Linux and Web from one codebase **smoothly**. The download page must never need a code edit per release, and it must keep working if GitHub's API is unavailable.

### 6.1 Release coordinates: single source of truth in `amadeus-core::release`

```rust
pub const RELEASE_REPO: &str = "dsuv-sg/RealAmadeusRust";      // = workspace `repository` (Cargo.toml)
pub const RELEASES_PAGE: &str = "https://github.com/dsuv-sg/RealAmadeusRust/releases";
pub fn tag_for(version: &str) -> String { format!("v{version}") }   // e.g. "v2.0.0" (Open question: or "RealAmadeusV2.0.0" to mirror V1's tag style)
pub fn asset_url(tag: &str, file: &str) -> String { format!("https://github.com/{RELEASE_REPO}/releases/download/{tag}/{file}") }
pub fn latest_asset_url(file: &str) -> String { format!("https://github.com/{RELEASE_REPO}/releases/latest/download/{file}") }
```

Asset naming follows the existing hints in `xtask/src/util.rs` tests (`RealAmadeus_V2.0.0R_Linux_amd64.deb`, `RealAmadeus_V2.0.0R_Linux_x86_64.AppImage`) and ARCHITECTURE.md (`RealAmadeus_V2.x.yR_Web_Linux-x64.tar.gz`). `{v}` = `VERSION` (e.g. `2.0.0`); the display form is `display_version()` = `V2.0.0R`.

| Key | OS | Kind | File name template | Recommended for OS |
|---|---|---|---|---|
| `win-nsis` | Windows | NSIS installer | `RealAmadeus_V{v}R_Windows_x64-setup.exe` | **yes** |
| `win-msi` | Windows | MSI | `RealAmadeus_V{v}R_Windows_x64.msi` | |
| `mac-dmg` | macOS | DMG (universal) | `RealAmadeus_V{v}R_macOS_universal.dmg` | **yes** |
| `mac-app` | macOS | `.app.tar.gz` (universal) | `RealAmadeus_V{v}R_macOS_universal.app.tar.gz` | |
| `linux-appimage` | Linux | AppImage | `RealAmadeus_V{v}R_Linux_x86_64.AppImage` | **yes** |
| `linux-deb` | Linux | .deb | `RealAmadeus_V{v}R_Linux_amd64.deb` | |
| `linux-rpm` | Linux | .rpm | `RealAmadeus_V{v}R_Linux_x86_64.rpm` | |
| `web-server` | Web (self-host) | server + frontend tarball | `RealAmadeus_V{v}R_Web_Linux-x64.tar.gz` | |
| `checksums` | — | SHA256 list | `SHA256SUMS.txt` | |

**Requirement:** `xtask` (which renames Tauri bundle outputs) and the UI must both use this **one** table (`amadeus_core::release::ASSETS`), so names cannot drift. Optionally, CI also uploads version-less aliases (e.g. `RealAmadeus_Windows_x64-setup.exe`). Then `latest_asset_url()` links never go stale, even if the web server is not redeployed.

### 6.2 Data model (I/O-free; wasm + host)

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "lowercase")]
pub enum Os { Windows, Macos, Linux, Web }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssetSpec { pub key: &'static str, pub os: Os, pub kind_label_key: &'static str, pub template: &'static str, pub recommended: bool }
pub const ASSETS: &[AssetSpec] = &[ /* table 6.1 */ ];
pub fn file_name(spec: &AssetSpec, version: &str) -> String;           // template.replace("{v}", version)
pub fn classify(file: &str) -> Option<&'static AssetSpec>;              // match by suffix after "RealAmadeus_V{ver}R_" (version-agnostic)
#[derive(Clone, Debug, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct ReleaseAsset { pub name: String, pub url: String, pub size: u64, pub download_count: u64 }
#[derive(Clone, Debug, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct ReleaseInfo { pub tag: String, pub name: String, pub published_at: String, pub html_url: String, pub assets: Vec<ReleaseAsset> }
pub fn static_release(version: &str) -> ReleaseInfo;                    // built from ASSETS + tag_for(version); size/download_count = 0, published_at = ""
pub fn group_by_os(info: &ReleaseInfo) -> Vec<(Os, Vec<(&'static AssetSpec, &ReleaseAsset)>)>; // order: Windows, macOS, Linux, Web; recommended first
```

### 6.3 Fetch strategy (progressive enhancement)

1. **Render immediately** from `static_release(amadeus_core::version::VERSION)`. This works offline, on desktop, and when GitHub rate-limits.
2. **Enhance asynchronously** with `ReleaseInfo` for the **latest** release:
   * **Web:** call a new **additive** endpoint `GET {base}/app/api/releases/latest` on `amadeus-server`. It is not part of the frozen V1 contract, so adding it is safe. The server fetches `https://api.github.com/repos/dsuv-sg/RealAmadeusRust/releases/latest` with headers `Accept: application/vnd.github+json`, `X-GitHub-Api-Version: 2022-11-28`, `User-Agent: RealAmadeus-Server` and optional `Authorization: Bearer $GITHUB_TOKEN`. It maps the result to `ReleaseInfo` (`tag_name`→`tag`, `name`, `published_at`→`publishedAt`, `html_url`→`htmlUrl`, `assets[].{name, browser_download_url→url, size, download_count→downloadCount}`). It caches the result in memory for 600 s, serves the stale cache on upstream errors, and returns `502 {"error":"..."}` if there is no cache. Drafts and prereleases are excluded by `/latest`; GitHub returns 404 if no release exists.
   * **Desktop:** an IPC command `releases_latest` calls the same `amadeus-backend::releases::fetch_latest()`.
   * **Fallback:** if the call fails, keep the static list and show the small note in §6.5. Optional: the browser may call api.github.com directly (CORS is allowed; 60 requests/hour/IP unauthenticated).
3. When the latest tag differs from the build version, show the latest release's links, because they are newer. On desktop also show «新しいバージョンがあります» / «A newer version is available» (ties into the updater; Open question).

### 6.4 OS detection and highlighting

`detect_os() -> Option<Os>`, evaluated once:

1. `navigator.userAgentData?.platform` (Chromium): "Windows" → Windows, "macOS" → Macos, "Linux"/"Chrome OS" → Linux, "Android" → mobile.
2. Else `navigator.platform`: starts with "Win" → Windows. Starts with "Mac" → Macos, unless `navigator.maxTouchPoints > 1` (iPadOS) → mobile. Contains "Linux" → Linux, unless the userAgent contains "Android" → mobile.
3. Else userAgent `/iPhone|iPad|iPod|Android/` → mobile.

Mobile → highlight the «Web版 / Web Version» card instead (desktop builds are useless there). The highlighted OS column gets `border-[#d49e38]` on its recommended card plus a tag «推奨» / «RECOMMENDED».

### 6.5 Page layout (V2 `/download`), reusing V1 visual language and classes

* Keep §3.2's chrome, title block and changelog link verbatim. Below the subtitle, add a version line `<p class="text-[10px] tracking-[0.3em] opacity-60 uppercase">{display_version} · {published_at date YYYY-MM-DD if known}</p>`.
* Replace the 2-column grid with `<div class="grid grid-cols-1 md:grid-cols-3 gap-12 w-full max-w-5xl">` containing three columns, «Windows», «macOS» and «Linux». Each column uses V1's section header (`flex items-center gap-4 mb-10` + gradient rules + `h2.text-[11px] tracking-[0.5em] …`) and lists its assets as `DownloadCard`s (§3.2 markup unchanged):
  * `caption` = «Deployment Package» for the recommended asset, else «Alternative Package».
  * `label` = the kind label (strings below).
  * `badge` = `display_version()` (e.g. «V2.0.0R»).
  * `href` = the asset URL.
* Under the grid, a full-width row has:
  * a «Web» card linking to `{base}/app` (label = `release.webVersion`, caption «Browser Deployment», badge «WEB»);
  * a «Self-host» card (`web-server` asset);
  * small links «SHA256SUMS» and «すべてのリリース / All Releases» (`RELEASES_PAGE`, `_blank`).
* A collapsible «旧バージョン (V1) / Legacy (V1)» section (`<details>`) contains V1's two blocks **verbatim** (Universal V1.4U ×3 + Lightweight V1.4Q ×1, same URLs and markup as §3.2), so existing users can still get V1.
* If the enhancement failed: `<p class="text-[10px] opacity-40 tracking-widest">` with «リリース情報を取得できませんでした。既定のリンクを表示しています。» / «Could not fetch release info. Showing default links.»
* Proposed new dictionary keys under `release.v2` (ja / en):
  * `nsis` «インストーラー (.exe)» / «Installer (.exe)»
  * `msi` «MSIパッケージ (.msi)» / «MSI Package (.msi)»
  * `dmg` «ディスクイメージ (.dmg)» / «Disk Image (.dmg)»
  * `appTar` «アプリアーカイブ (.app.tar.gz)» / «App Archive (.app.tar.gz)»
  * `appimage` «AppImage» / «AppImage»
  * `deb` «Debian / Ubuntu (.deb)» / same
  * `rpm` «Fedora / openSUSE (.rpm)» / same
  * `selfHost` «セルフホスト版サーバー (Linux x64)» / «Self-host Server (Linux x64)»
  * `recommended` «推奨» / «RECOMMENDED»
  * `allReleases` «すべてのリリース» / «All Releases»
  * `legacy` «旧バージョン (V1)» / «Legacy (V1)»
  * `fetchFailed` as above
  * `newer` «新しいバージョンがあります» / «A newer version is available»
* Landing copy should move to V2. Proposal: `release.title` = «SYSTEM V{major}.{minor} RELEASED», and `release.message` = «Real Amadeus V{major}.{minor}が正式にリリースされました。» / «REAL AMADEUS V{major}.{minor} HAS BEEN OFFICIALLY RELEASED.», with values computed from `VERSION`, e.g. «SYSTEM V2.0 RELEASED». Product decision; see Open questions.
* `/docs/download`: keep V1's sections (Unity/Qt) under a «Legacy (V1)» heading. Add a first section, «RealAmadeus V2 (Rust)», in the same `renderSection` style with three `DocsDlButton`s (Windows/macOS/Linux recommended assets) and short OS notes (Windows SmartScreen for an unsigned installer; macOS Gatekeeper / `xattr -dr com.apple.quarantine /Applications/RealAmadeus.app` if not notarized; Linux `chmod +x *.AppImage`). The exact copy is a product decision.

---

## 7. Platform notes

### 7.1 Web (`amadeus-server`, base `/amadeus`)

* The router base is `"/amadeus"`, derived at runtime from `document.baseURI` (`<base href="/amadeus/">`): take its pathname and strip the trailing `/`.
* **Pitfall (verified in leptos_router 0.8.17 `use_resolved_path`):** absolute hrefs (starting with `/`) are **not** prefixed with the router base. `<A href="/docs">` would point to `/docs`. Always build hrefs with `site_href(path)` = base + path (path `""` → base, or `"/"` when the base is empty).
* `use_location().pathname` includes the base (e.g. `/amadeus/docs`), so strip it before matching (§3.4.2).
* leptos_router intercepts clicks on **any** same-origin `<a>` whose path starts with the base, if the click has no modifier keys, no `target`, no `download` and no `rel=external`. Plain `<a href="{base}/…">` therefore navigates client-side like V1's `<Link>`. `target="_blank"` and cross-origin links go to the browser. `<a href="/">` (landing «Return») is outside the base, so it is a full navigation.
* Serve `index.html` for all §1.1 routes (SPA fallback), and `static/*` from the Trunk dist.

### 7.2 Desktop (Tauri 2, base `""`, window URL `app`)

* The window opens at `app` (`tauri.conf.json`), so the pages are reachable only via in-app links. Recommend keeping them for offline docs.
* **External links must open in the system browser.** In the Tauri webview, `target="_blank"` does nothing by default, and same-tab navigation to github.com or a `.zip` would leave the app or fail (WKWebView). Route every external URL (§1.8, and the §6 assets) through `platform::open_external(url)` (tauri-plugin-opener on desktop; normal anchor behaviour on web). Implement this as a capture-phase click handler on the page roots, or by rendering desktop links with `on:click` + `prevent_default`.
* Landing «Return» (`/`) would loop to the landing itself when the base is empty. On desktop, hide it or point it to `https://dsuv.elvelt.com/` via `open_external` (Open question).
* The GitHub repo link should probably become the V2 repo (Open question).
* The V1 app's Shutdown dialog calls `window.history.back()` and then hard-navigates to `https://dsuv.elvelt.com/amadeus` (`RealAmadeusApp.tsx:4698`). On Web that lands on this landing page with a full reload, so the locale resets to `ja`. That is the app subsystem's concern, but the landing URL must stay `https://dsuv.elvelt.com/amadeus`.

## Appendix A — Site dictionary data (verbatim, extracted programmatically)

This is the `translations.ja.amadeus` / `translations.en.amadeus` subtree of `hooks/useTranslation.tsx` (ja lines 35–326, en lines 355–646). It was produced by compiling the TSX with esbuild using a custom JSX factory and evaluating the result, so every string is byte-exact. Rich text is encoded as in §1.4.2. All links share `target="_blank" rel="noopener noreferrer" class="text-[#d49e38] underline hover:text-[#f0b95b] transition-colors"`; that attribute set was verified to be uniform and dropped from the segments. Proposed V2 location: `crates/amadeus-core/locales/site/ja.json` and `.../en.json`, wrapped as `{"amadeus": …}` or used as-is as the root.

Generator scripts (scratchpad): `tools/dump_site_i18n.cjs` (TSX → JSON with rich segments) and `tools/compact_site_i18n.cjs` (amadeus subtree, link attributes removed).

### A.1 `ja`

```json
{
  "status": {
    "system": "SYSTEM",
    "ver": "VER 1.04a",
    "connecting": "CONNECTING TO KURISU..."
  },
  "button": {
    "init": "システム初期化"
  },
  "panels": {
    "goal": {
      "title": "PROJECT GOAL",
      "content": "Steins;Gate 0で描かれたAmadeus AIシステムの完全再現。"
    },
    "status": {
      "title": "CURRENT STATUS",
      "content": "メモリグラフ実装フェーズ。文脈維持効率: 87%。"
    },
    "access": {
      "title": "ACCESS LEVEL",
      "content": "制限あり。ニューラルネットワーク調整には開発者キーが必要です。"
    }
  },
  "hud": {
    "logic": "LOGIC ENGINE",
    "voice": "VOICE SYNTHESIS",
    "memory": "MEMORY CORE"
  },
  "accessRestricted": {
    "title": "アクセス制限",
    "desc": "Amadeusシステムへのパブリックアクセスは現在、許可された研究者に限定されています。",
    "button": "接続を開始..."
  },
  "login": {
    "userId": "USER ID",
    "password": "PASSWORD",
    "connect": "接続",
    "placeholderId": "Salieri",
    "error": "認証エラー: IDまたはパスワードが無効です。"
  },
  "preparation": {
    "title": "SYSTEM PREPARATION",
    "message": "準備中",
    "status": "INITIALIZING..."
  },
  "release": {
    "title": "SYSTEM V1.4 RELEASED",
    "message": "Real Amadeus V1.4が正式にリリースされました。",
    "windowsDownload": "Windows",
    "macosDownload": "MacOS",
    "linuxDownload": "Linux",
    "lightweightWindowsDownload": "軽量化版Windows",
    "download": "ダウンロード",
    "githubRepo": "GitHubリポジトリ",
    "docs": "ドキュメント",
    "webVersion": "Web版"
  },
  "docs": {
    "title": "DOCUMENTATION",
    "back": "システムへ戻る",
    "sidebar": {
      "gettingStarted": "はじめに",
      "download": "ダウンロード",
      "login": "ログイン方法",
      "controls": "操作方法",
      "apiSetup": "APIの取得と設定",
      "apiOpenAI": "OpenAI API",
      "apiGemini": "Google Gemini API",
      "apiClaude": "Anthropic Claude API",
      "apiGroq": "Groq API",
      "apiVertex": "Google Vertex AI"
    },
    "gettingStarted": {
      "title": "はじめに",
      "desc": "RealAmadeusは、Steins;Gate 0に登場する人工知能「Amadeus」のシステムを再現することを目的としたプロジェクトです。このドキュメントでは、本システムを操作し、フル機能を利用するための手順を解説します。",
      "loginInfo": "本システムは研究目的で開発されています。AIとの高度な対話を行うには、初期設定およびAPI構成が必要です。"
    },
    "download": {
      "title": "ダウンロード",
      "desc": "RealAmadeusは通常版と軽量化版の2種類を提供しています。使用環境に応じて適切なバージョンを選択し、GitHub Releasesからダウンロードしてください。",
      "standardTitle": "通常版 RealAmadeus",
      "standardDesc": "Unityエンジンで開発されています。美しいグラフィックを楽しめ、Windows/Mac/Linuxで使用できます。",
      "lightweightTitle": "軽量化版 RealAmadeusQT",
      "lightweightDesc": "Qt/QMLで開発されています。軽快な動作が特長です。",
      "recommendedEnv": "推奨環境",
      "standardEnv": "GPU搭載環境 / RAM1GB以上の環境",
      "lightweightEnv": "GPU非搭載 / 低スペック環境",
      "windows": "Windows",
      "mac": "MacOS",
      "linux": "Linux",
      "downloadBtn": "ダウンロード",
      "releasesPage": "Releasesページ",
      "fallbackStandard": {
        "rich": [
          {
            "text": "上記のボタンからダウンロードできない場合は、"
          },
          {
            "link": "https://github.com/dsuv-sg/RealAmadeus/releases",
            "text": "RealAmadeus Releases"
          },
          {
            "text": "（通常版）から直接ダウンロードしてください。"
          }
        ]
      },
      "fallbackLightweight": {
        "rich": [
          {
            "text": "上記のボタンからダウンロードできない場合は、"
          },
          {
            "link": "https://github.com/dsuv-sg/RealAmadeusQT/releases",
            "text": "RealAmadeusQT Releases"
          },
          {
            "text": "（軽量化版）から直接ダウンロードしてください。"
          }
        ]
      },
      "steps": [
        {
          "title": "OSに応じたアセットをダウンロード",
          "content": "Assetsセクションから、ご利用のOSに対応したファイルをダウンロードします。Windows/MacOS/Linuxの各ビルドが提供されています（軽量化版はWindowsのみ）。"
        },
        {
          "title": "ファイルを解凍",
          "content": "ダウンロードした圧縮ファイルを任意のフォルダに解凍してください。"
        }
      ],
      "fileStructure": {
        "title": "ファイル構成について",
        "windows": "解凍後、Buildフォルダ内に RealAmadeus.exe（実行ファイル）、RealAmadeus_Data/（アセット・データ）、UnityPlayer.dll、MonoBleedingEdge/、D3D12/ などの実行環境ファイルが含まれます。フォルダ構成を維持したまま RealAmadeus.exe を実行してください。",
        "mac": "解凍後、Buildフォルダ内に Mac.app バンドルが含まれます。Mac.app をアプリケーションフォルダなどに移動して利用してください。バンドル内には実行ファイル（MacOS/RealAmadeus）、Frameworks/、PlugIns/、Resources/、MonoBleedingEdge/ が含まれます。",
        "linux": "解凍後、Buildフォルダ内に RealAmadeus.x86_64（実行ファイル）、RealAmadeus_Data/（データ）、UnityPlayer.so、libdecor-0.so.0、libdecor-cairo.so などが含まれます。実行権限を付与してから起動してください。",
        "windowsQT": "解凍後、Buildフォルダ内に appRealAmadeusPC.exe（実行ファイル）、RealAmadeusPC/（QMLリソース）、Qt6*.dll（多数）、platforms/、qml/、imageformats/、Live2DCubismCore.dll、FrameworkShaders/、translations/ などが含まれます。フォルダ構成を維持したまま appRealAmadeusPC.exe を実行してください。"
      },
      "notice": "※ Windows以外のOSでは動作未確認の場合があります。問題が発生した場合は、GitHub Issuesにて報告をお願いします。"
    },
    "changelog": {
      "title": "変更履歴",
      "desc": "RealAmadeusの主なアップデート内容です。",
      "versions": [
        {
          "version": "V1.4",
          "product": "RealAmadeus",
          "items": [
            "起動メッセージを実装",
            "ミニゲーム（クイズ）を実装",
            "アチーブメント機能を追加",
            "会話言語を表示言語とは別に設定できるようになった",
            "会話記録のエクスポート/インポート機能を追加",
            "MS明朝の動的ロード・自動ダウンロードなど、フォントレンダリングを改善",
            "ドラッグによるウィンドウサイズ変更を無効化",
            "ヘブライ語/アラビア語のサポートを追加",
            "ローカライズに使用されるフォントをNotoSerifに変更"
          ]
        },
        {
          "version": "V1.3",
          "product": "RealAmadeus",
          "items": [
            "Live2Dの居眠りモーションを実装",
            "OpenAI互換APIの使用とベースURL指定機能を実装",
            "UIを調整",
            "応答ロード中のキャンセル機能を実装",
            "Live2Dのリップシンクを実装",
            "アスペクト比を維持したスケーリングを行うように修正",
            "APIストリーミング通信時のデッドロックを修正",
            "APIキーを暗号化保存するようにし、セキュリティを向上",
            "RAGデータの永続化やベクトルDB接続の最適化を実装",
            "ポルトガル語/ウクライナ語/トルコ語のサポートを追加",
            "起動時の画面点滅および一部環境での起動失敗を防ぐため、デフォルトの画面モードをウィンドウモードに変更",
            "起動時の最新リリース自動チェックおよび自動アップデーターを実装",
            "日本語以外の言語でも会話テキストが文末（ピリオド等）で適切に区切られるように改善",
            "ローカライズの管理を最適化"
          ]
        },
        {
          "version": "V1.2",
          "product": "RealAmadeusQT",
          "items": [
            "中国語/韓国語/スペイン語/フランス語/ドイツ語/ロシア語 のサポートを追加",
            "視線トラッキングを追加",
            "デスクトップ通知機能を実装",
            "軽量化モードを追加",
            "Alt+Enter、F11 での画面モード切替を実装",
            "APIプロバイダーにOllamaとOpenRouterを追加",
            "UIのバグを修正",
            "APIキーのセキュリティを向上",
            "パフォーマンスを改善"
          ]
        },
        {
          "version": "V1.1",
          "product": "RealAmadeus",
          "items": [
            "メニュー画面のクリック選択を実装",
            "日本語/英語の言語切り替えを実装",
            "各種画面に[閉じる/キャンセル/適用]ボタンを実装",
            "チェンジログ/バックログ用にスクロールバーを実装",
            "視認性の向上のため、UIの一部を変更",
            "軽量化版のリリースを開始"
          ]
        },
        {
          "version": "V1.0.1",
          "product": "RealAmadeus",
          "items": [
            "GPU使用率が異常に高くなってしまう問題を修正",
            "フルスクリーン状態での最小化時にウィンドウが異常に小さくなってしまう問題を修正",
            "ログアウト後の再ログインが不可能になってしまう問題を修正",
            "一部AIサービスの利用時に感情タグが表示されてしまう問題を修正"
          ]
        },
        {
          "version": "V1.0",
          "product": "RealAmadeus",
          "items": [
            "リアルアマデウスの最初のバージョンをリリース",
            "基本的な会話機能を実装"
          ]
        }
      ]
    },
    "loginDoc": {
      "title": "システムログイン",
      "desc": "システムにアクセスするための認証情報です。起動画面で以下の資格情報を入力してください。",
      "idLabel": "User ID",
      "idValue": "Salieri",
      "passLabel": "Password",
      "passValue": "MakiseKurisu"
    },
    "controls": {
      "title": "キーボード操作",
      "desc": "Amadeusシステム内の各種操作はキーボードおよびマウスで行います。",
      "items": [
        {
          "key": "Enter",
          "desc": "決定 / ログインの実行"
        },
        {
          "key": "Tab / 右クリック",
          "desc": "システムメニューの開閉"
        },
        {
          "key": "W A S D / 矢印",
          "desc": "メニューや設定項目の移動"
        },
        {
          "key": "Backspace",
          "desc": "バックログを閉じる / 設定の保存と終了"
        },
        {
          "key": "Alt + Enter / F11",
          "desc": "フルスクリーン / ウィンドウモード切替"
        }
      ]
    },
    "api": {
      "title": "API設定",
      "desc": "AIと対話を行うには、外部のLLMプロバイダのAPIキーが必要です。",
      "steps": [
        "システム起動後、「Tab」キーまたは「右クリック」でメニューを開きます。",
        "メニューから「CONFIG」を選択し、「API設定」タブへ進みます。",
        "「APIプロバイダ」のドロップダウンから使用するAIモデルを選択します。",
        "提供されている「APIキー」と「モデル名」を入力し、設定を保存（Backspace）してください。"
      ],
      "notice": "※設定したAPIキーはローカルに保存され、通信は直接プロバイダのサーバーへ行われます。開発元（DSUV）に情報が送信されることはありません。",
      "modelLabels": {
        "title": "利用可能なモデル",
        "available": "利用可能",
        "recommended": "推奨"
      }
    },
    "apiOpenAI": {
      "title": "OpenAI API",
      "desc": "ChatGPTなどの高精度モデルが利用可能です。設定が簡単で安定しています。",
      "modelList": {
        "available": [
          "gpt-5.5",
          "gpt-5.5-pro",
          "gpt-5.5-mini",
          "gpt-5.4",
          "gpt-5.4-pro",
          "gpt-5.4-mini",
          "o3-mini",
          "o1",
          "gpt-4o",
          "gpt-4o-mini"
        ],
        "recommended": "gpt-5.5"
      },
      "steps": [
        {
          "title": "アカウント作成",
          "content": {
            "rich": [
              {
                "link": "https://platform.openai.com/",
                "text": "OpenAI Platform (platform.openai.com)"
              },
              {
                "text": " にアクセスし、アカウントを登録します。"
              }
            ]
          }
        },
        {
          "title": "課金設定",
          "content": "APIを利用するにはクレジットカードの登録（Pay as you go）が必要です。Billingセクションから設定してください。"
        },
        {
          "title": "キーの生成",
          "content": "API Keysページから「Create new secret key」をクリックしてキー（sk-...）を生成します。"
        },
        {
          "title": "Amadeus側の設定",
          "content": "システムメニューの CONFIG > API設定 を開き、「OpenAI」を選択し、取得したAPIキーと利用したいモデル名（例: gpt-5.5）を入力して保存します。"
        }
      ]
    },
    "apiGemini": {
      "title": "Google Gemini API",
      "desc": "高速かつ大コンテキストに対応。Googleアカウントで簡単に取得でき、無料枠も存在します。",
      "modelList": {
        "available": [
          "gemini-3.5-flash",
          "gemini-3.5-pro",
          "gemini-3.1-pro-preview",
          "gemini-3.1-flash-lite-preview",
          "gemini-omni-flash",
          "gemini-omni",
          "gemini-3-flash-preview",
          "gemini-3-pro-preview",
          "gemini-2.5-flash",
          "gemini-2.5-pro"
        ],
        "recommended": "gemini-3.5-flash"
      },
      "steps": [
        {
          "title": "Google AI Studio",
          "content": {
            "rich": [
              {
                "link": "https://aistudio.google.com/",
                "text": "Google AI Studio (aistudio.google.com)"
              },
              {
                "text": " にアクセスし、Googleアカウントでログインします。"
              }
            ]
          }
        },
        {
          "title": "プロジェクト作成とキー生成",
          "content": "「Get API key」メニューを選択し、新しいプロジェクトに対してキーを生成します。"
        },
        {
          "title": "Amadeus側の設定",
          "content": "システムメニューの CONFIG > API設定 を開き、「Google Gemini」を選択し、取得したAPIキーと利用したいモデル名（例: gemini-3.5-flash）を入力して保存します。"
        }
      ]
    },
    "apiClaude": {
      "title": "Anthropic Claude API",
      "desc": "極めて自然な日本語と高い推論能力を持ちます。複雑な対話に最適です。",
      "modelList": {
        "available": [
          "claude-fable-5",
          "claude-opus-latest",
          "claude-sonnet-latest",
          "claude-haiku-latest",
          "claude-opus-4-8",
          "claude-sonnet-4-6",
          "claude-haiku-4-5"
        ],
        "recommended": "claude-sonnet-latest"
      },
      "steps": [
        {
          "title": "アカウント登録",
          "content": {
            "rich": [
              {
                "link": "https://console.anthropic.com/",
                "text": "Anthropic Console (console.anthropic.com)"
              },
              {
                "text": " にアクセスし登録します。"
              }
            ]
          }
        },
        {
          "title": "課金設定",
          "content": "Settings > Billing からクレジットを追加します。"
        },
        {
          "title": "キーの生成",
          "content": "API Keys セクションから「Create Key」をクリックしてキー（sk-ant-...）を取得します。"
        },
        {
          "title": "Amadeus側の設定",
          "content": "システムメニューの CONFIG > API設定 を開き、「Anthropic Claude」を選択し、取得したAPIキーと利用したいモデル名（例: claude-sonnet-latest）を入力して保存します。"
        }
      ]
    },
    "apiGroq": {
      "title": "Groq API",
      "desc": "驚異的な推論速度を誇るLPU搭載プラットフォーム。レスポンスの速さが特徴です。",
      "modelList": {
        "available": [
          "llama-3.3-70b-versatile",
          "llama-3.1-8b-instant",
          "llama-3.2-11b-vision-preview",
          "qwen/qwen3-32b",
          "mixtral-8x7b-32768"
        ],
        "recommended": "llama-3.3-70b-versatile"
      },
      "steps": [
        {
          "title": "Groq Console",
          "content": {
            "rich": [
              {
                "link": "https://console.groq.com/",
                "text": "Groq Console (console.groq.com)"
              },
              {
                "text": " にアクセスしログインします。"
              }
            ]
          }
        },
        {
          "title": "キーの生成",
          "content": "API Keys セクションから「Create API Key」を選択し、キーを生成します。"
        },
        {
          "title": "Amadeus側の設定",
          "content": "システムメニューの CONFIG > API設定 を開き、「Groq」を選択し、取得したAPIキーと利用したいモデル名（例: llama-3.3-70b-versatile）を入力して保存します。"
        }
      ]
    },
    "apiVertex": {
      "title": "Google Vertex AI",
      "desc": "GCP上のエンタープライズ向けAI開発基盤。セキュリティと大規模運用に優れています。RealAmadeusでは、内部でPowerShell経由でGoogle Cloud CLIを呼び出し、認証情報を取得します。",
      "modelList": {
        "available": [
          "gemini-3.5-flash",
          "gemini-3.5-pro",
          "gemini-3.1-pro-preview",
          "gemini-3.1-flash-lite-preview",
          "gemini-omni-flash",
          "gemini-omni",
          "gemini-3-flash-preview",
          "gemini-3-pro-preview",
          "gemini-2.5-flash",
          "gemini-2.5-pro"
        ],
        "recommended": "gemini-3.5-flash"
      },
      "steps": [
        {
          "title": "登録",
          "content": {
            "rich": [
              {
                "link": "https://console.cloud.google.com/",
                "text": "Google Cloud (console.cloud.google.com)"
              },
              {
                "text": " の登録を行います。"
              }
            ]
          }
        },
        {
          "title": "トライアルの有効化",
          "content": "トライアルアカウントを有効化します。"
        },
        {
          "title": "プロジェクトの作成",
          "content": "Google Cloud プロジェクトを作成します。"
        },
        {
          "title": "Vertex AI APIの有効化",
          "content": "プロジェクトで Vertex AI API を有効化します。"
        },
        {
          "title": "Google Cloud CLIのインストール",
          "content": {
            "rich": [
              {
                "link": "https://dl.google.com/dl/cloudsdk/channels/rapid/GoogleCloudSDKInstaller.exe?hl=ja",
                "text": "Google Cloud CLI (gcloud) Installer"
              },
              {
                "text": " をダウンロード、インストールしてセットアップします。その後、コマンドプロンプト(cmd)を開き、"
              },
              {
                "code": "gcloud auth login"
              },
              {
                "text": "を実行してログインします。"
              }
            ]
          }
        },
        {
          "title": "Amadeus側の設定",
          "content": "STEP3で作成したプロジェクトのIDを確認します。システムメニューのCONFIGから「Vertex AI」を選択し、利用するプロジェクトIDとモデル名（例: gemini-3.5-flash）を設定します。認証トークンはシステムがgcloud経由で自動取得します。"
        }
      ]
    }
  }
}
```

### A.2 `en`

```json
{
  "status": {
    "system": "SYSTEM",
    "ver": "VER 1.04a",
    "connecting": "CONNECTING TO KURISU..."
  },
  "button": {
    "init": "INITIALIZE SYSTEM"
  },
  "panels": {
    "goal": {
      "title": "PROJECT GOAL",
      "content": "Complete reproduction of the Amadeus AI system logic as depicted in Steins;Gate 0."
    },
    "status": {
      "title": "CURRENT STATUS",
      "content": "Memory Graph Implementation Phase. Context maintenance efficiency: 87%."
    },
    "access": {
      "title": "ACCESS LEVEL",
      "content": "Restricted. Developer key required for neural network adjustments."
    }
  },
  "hud": {
    "logic": "LOGIC ENGINE",
    "voice": "VOICE SYNTHESIS",
    "memory": "MEMORY CORE"
  },
  "accessRestricted": {
    "title": "Access Restricted",
    "desc": "Public access to the Amadeus system is currently limited to authorized researchers.",
    "button": "INITIATE_CONNECTION..."
  },
  "login": {
    "userId": "USER ID",
    "password": "PASSWORD",
    "connect": "CONNECT",
    "placeholderId": "Salieri",
    "error": "AUTH ERROR: INVALID ID OR PASSWORD"
  },
  "preparation": {
    "title": "SYSTEM PREPARATION",
    "message": "PREPARING...",
    "status": "INITIALIZING..."
  },
  "release": {
    "title": "SYSTEM V1.4 RELEASED",
    "message": "REAL AMADEUS V1.4 HAS BEEN OFFICIALLY RELEASED.",
    "windowsDownload": "Windows",
    "macosDownload": "MacOS",
    "linuxDownload": "Linux",
    "lightweightWindowsDownload": "Lightweight Windows",
    "download": "DOWNLOAD",
    "githubRepo": "GitHub Repository",
    "docs": "Documentation",
    "webVersion": "Web Version"
  },
  "docs": {
    "title": "DOCUMENTATION",
    "back": "RETURN TO SYSTEM",
    "sidebar": {
      "gettingStarted": "Getting Started",
      "download": "Download",
      "login": "Login Instructions",
      "controls": "Controls",
      "apiSetup": "API Setup",
      "apiOpenAI": "OpenAI API",
      "apiGemini": "Google Gemini API",
      "apiClaude": "Anthropic Claude API",
      "apiGroq": "Groq API",
      "apiVertex": "Google Vertex AI"
    },
    "gettingStarted": {
      "title": "GETTING STARTED",
      "desc": "RealAmadeus aims to reproduce the artificial intelligence system 'Amadeus' featured in Steins;Gate 0. This documentation explains the procedures to operate the system and utilize its full capabilities.",
      "loginInfo": "This system is developed for research purposes. Initial setup and API configuration are required to engage in advanced dialogue with the AI."
    },
    "download": {
      "title": "DOWNLOAD",
      "desc": "RealAmadeus provides two versions: the standard edition and the lightweight edition. Choose the appropriate version for your environment and download it from GitHub Releases.",
      "standardTitle": "Standard Edition RealAmadeus",
      "standardDesc": "Developed with the Unity engine. Enjoy beautiful graphics on Windows, MacOS, and Linux.",
      "lightweightTitle": "Lightweight Edition RealAmadeusQT",
      "lightweightDesc": "Developed with Qt/QML. Features lightweight performance.",
      "recommendedEnv": "Recommended Environment",
      "standardEnv": "GPU-equipped environment / Environment with 1GB+ RAM",
      "lightweightEnv": "No GPU / Low-spec environment",
      "windows": "Windows",
      "mac": "MacOS",
      "linux": "Linux",
      "downloadBtn": "DOWNLOAD",
      "releasesPage": "Releases Page",
      "fallbackStandard": {
        "rich": [
          {
            "text": "If the download buttons above do not work, please download directly from "
          },
          {
            "link": "https://github.com/dsuv-sg/RealAmadeus/releases",
            "text": "RealAmadeus Releases"
          },
          {
            "text": " (Standard)."
          }
        ]
      },
      "fallbackLightweight": {
        "rich": [
          {
            "text": "If the download buttons above do not work, please download directly from "
          },
          {
            "link": "https://github.com/dsuv-sg/RealAmadeusQT/releases",
            "text": "RealAmadeusQT Releases"
          },
          {
            "text": " (Lightweight)."
          }
        ]
      },
      "steps": [
        {
          "title": "Download the Asset for Your OS",
          "content": "From the Assets section, download the file corresponding to your OS. Builds for Windows, MacOS, and Linux are provided (Lightweight edition is Windows only)."
        },
        {
          "title": "Extract the Files",
          "content": "Extract the downloaded archive to a folder of your choice."
        }
      ],
      "fileStructure": {
        "title": "About the File Structure",
        "windows": "After extraction, the Build folder contains RealAmadeus.exe (executable), RealAmadeus_Data/ (assets and data), UnityPlayer.dll, MonoBleedingEdge/, D3D12/, and other runtime files. Please keep the folder structure intact and run RealAmadeus.exe.",
        "mac": "After extraction, the Build folder contains the Mac.app bundle. Move Mac.app to your Applications folder or another suitable location. The bundle includes the executable (MacOS/RealAmadeus), Frameworks/, PlugIns/, Resources/, and MonoBleedingEdge/.",
        "linux": "After extraction, the Build folder contains RealAmadeus.x86_64 (executable), RealAmadeus_Data/ (data), UnityPlayer.so, libdecor-0.so.0, libdecor-cairo.so, and other files. Please grant execution permissions before launching.",
        "windowsQT": "After extraction, the Build folder contains appRealAmadeusPC.exe (executable), RealAmadeusPC/ (QML resources), numerous Qt6*.dll files, platforms/, qml/, imageformats/, Live2DCubismCore.dll, FrameworkShaders/, translations/, and other required libraries. Please keep the folder structure intact and run appRealAmadeusPC.exe."
      },
      "notice": "* Functionality on non-Windows operating systems is unconfirmed. If you encounter any issues, please report them via GitHub Issues."
    },
    "changelog": {
      "title": "CHANGELOG",
      "desc": "Major updates for RealAmadeus.",
      "versions": [
        {
          "version": "V1.4",
          "product": "RealAmadeus",
          "items": [
            "Implemented startup messages",
            "Implemented a mini-game (quiz)",
            "Added an achievements feature",
            "Conversation language can now be set independently from the display language",
            "Added export/import support for conversation logs",
            "Improved font rendering, including dynamic loading and automatic downloading of MS Mincho",
            "Disabled window resizing via dragging",
            "Added support for Hebrew and Arabic",
            "Changed localization fonts to Noto Serif"
          ]
        },
        {
          "version": "V1.3",
          "product": "RealAmadeus",
          "items": [
            "Implemented sleep motion for Live2D",
            "Implemented use of OpenAI compatible APIs and base URL specification",
            "Adjusted UI",
            "Implemented response loading cancellation",
            "Implemented Live2D lip-sync",
            "Fixed scaling to maintain aspect ratio",
            "Fixed deadlock during API streaming communication",
            "Improved security by encrypting API keys",
            "Implemented RAG data persistence and vector DB connection optimization",
            "Added support for Portuguese, Ukrainian, and Turkish",
            "Changed default screen mode to windowed to prevent flickering and startup failures",
            "Implemented automatic latest release checks and auto-updater on startup",
            "Improved conversation text segmentation for non-Japanese languages",
            "Optimized localization management"
          ]
        },
        {
          "version": "V1.2",
          "product": "RealAmadeusQT",
          "items": [
            "Added support for Chinese, Korean, Spanish, French, German, and Russian",
            "Added eye tracking",
            "Implemented desktop notification feature",
            "Added lightweight mode",
            "Implemented screen mode toggle with Alt+Enter and F11",
            "Added Ollama and OpenRouter API providers",
            "Fixed UI bugs",
            "Improved security for API keys",
            "Improved performance"
          ]
        },
        {
          "version": "V1.1",
          "product": "RealAmadeus",
          "items": [
            "Implemented click selection for the menu screen",
            "Added language switching between Japanese and English",
            "Added [Close/Cancel/Apply] buttons to various screens",
            "Implemented scrollbars for ChangeLog and BackLog",
            "Updated UI elements for better visibility",
            "Started release of the lightweight version"
          ]
        },
        {
          "version": "V1.0.1",
          "product": "RealAmadeus",
          "items": [
            "Fixed an issue where GPU usage was abnormally high",
            "Fixed a window scaling issue when minimizing in fullscreen",
            "Fixed an issue where re-logging in after logout was impossible",
            "Fixed a bug where emotion tags were appearing for some AI services"
          ]
        },
        {
          "version": "V1.0",
          "product": "RealAmadeus",
          "items": [
            "Released the first version of Real Amadeus",
            "Implemented basic conversation features"
          ]
        }
      ]
    },
    "loginDoc": {
      "title": "SYSTEM LOGIN",
      "desc": "Authentication details required to access the system. Enter the following credentials.",
      "idLabel": "User ID",
      "idValue": "Salieri",
      "passLabel": "Password",
      "passValue": "MakiseKurisu"
    },
    "controls": {
      "title": "KEYBOARD CONTROLS",
      "desc": "Various operations within the Amadeus system are performed via keyboard and mouse.",
      "items": [
        {
          "key": "Enter",
          "desc": "Confirm / Execute Login"
        },
        {
          "key": "Tab / Right-click",
          "desc": "Open/Close System Menu"
        },
        {
          "key": "W A S D / Arrows",
          "desc": "Navigate menus and settings"
        },
        {
          "key": "Backspace",
          "desc": "Close Backlog / Save Settings & Exit"
        },
        {
          "key": "Alt + Enter / F11",
          "desc": "Toggle Fullscreen / Windowed Mode"
        }
      ]
    },
    "api": {
      "title": "API CONFIGURATION",
      "desc": "To converse with the AI, you must configure an external LLM provider API key.",
      "steps": [
        "After booting, press 'Tab' or 'Right-Click' to open the system menu.",
        "Select 'CONFIG' and proceed to the 'API Settings' tab.",
        "Select your preferred AI model from the 'API Provider' dropdown.",
        "Input the 'API Key' and 'Model Name', then save your settings (Backspace)."
      ],
      "notice": "* All API keys are stored locally. Communications occur directly with the provider servers. DSUV does not collect or transmit this data.",
      "modelLabels": {
        "title": "SUPPORTED MODELS",
        "available": "Available",
        "recommended": "Recommended"
      }
    },
    "apiOpenAI": {
      "title": "OpenAI API",
      "desc": "High precision models like ChatGPT. Easy to configure and extremely stable.",
      "modelList": {
        "available": [
          "gpt-5.5",
          "gpt-5.5-pro",
          "gpt-5.5-mini",
          "gpt-5.4",
          "gpt-5.4-pro",
          "gpt-5.4-mini",
          "o3-mini",
          "o1",
          "gpt-4o",
          "gpt-4o-mini"
        ],
        "recommended": "gpt-5.5"
      },
      "steps": [
        {
          "title": "Create Account",
          "content": {
            "rich": [
              {
                "text": "Visit "
              },
              {
                "link": "https://platform.openai.com/",
                "text": "OpenAI Platform (platform.openai.com)"
              },
              {
                "text": " and register."
              }
            ]
          }
        },
        {
          "title": "Billing Setup",
          "content": "You must register a credit card (Pay as you go) via the Billing section."
        },
        {
          "title": "Generate Key",
          "content": "Click 'Create new secret key' on the API Keys page to get your sk-... key."
        },
        {
          "title": "Configure Amadeus",
          "content": "Open CONFIG > API Settings from the system menu. Enter the API key and desired model name (e.g., gpt-5.5) and save."
        }
      ]
    },
    "apiGemini": {
      "title": "Google Gemini API",
      "desc": "Fast and supports large context windows. Easy to acquire with a Google Account, includes a free tier.",
      "modelList": {
        "available": [
          "gemini-3.5-flash",
          "gemini-3.5-pro",
          "gemini-3.1-pro-preview",
          "gemini-3.1-flash-lite-preview",
          "gemini-omni-flash",
          "gemini-omni",
          "gemini-3-flash-preview",
          "gemini-3-pro-preview",
          "gemini-2.5-flash",
          "gemini-2.5-pro"
        ],
        "recommended": "gemini-3.5-flash"
      },
      "steps": [
        {
          "title": "Google AI Studio",
          "content": {
            "rich": [
              {
                "text": "Visit "
              },
              {
                "link": "https://aistudio.google.com/",
                "text": "Google AI Studio (aistudio.google.com)"
              },
              {
                "text": " and log in."
              }
            ]
          }
        },
        {
          "title": "Create Project & Key",
          "content": "Select 'Get API key' and generate a new key for a new project."
        },
        {
          "title": "Configure Amadeus",
          "content": "Open CONFIG > API Settings from the system menu. Enter the API key and desired model name (e.g., gemini-3.5-flash) and save."
        }
      ]
    },
    "apiClaude": {
      "title": "Anthropic Claude API",
      "desc": "Provides extremely natural Japanese and advanced reasoning. Ideal for complex dialogues.",
      "modelList": {
        "available": [
          "claude-fable-5",
          "claude-opus-latest",
          "claude-sonnet-latest",
          "claude-haiku-latest",
          "claude-opus-4-8",
          "claude-sonnet-4-6",
          "claude-haiku-4-5"
        ],
        "recommended": "claude-sonnet-latest"
      },
      "steps": [
        {
          "title": "Registration",
          "content": {
            "rich": [
              {
                "text": "Register at the "
              },
              {
                "link": "https://console.anthropic.com/",
                "text": "Anthropic Console (console.anthropic.com)"
              },
              {
                "text": "."
              }
            ]
          }
        },
        {
          "title": "Billing Setup",
          "content": "Add credits via Settings > Billing."
        },
        {
          "title": "Generate Key",
          "content": "Click 'Create Key' in the API Keys section to get your sk-ant-... key."
        },
        {
          "title": "Configure Amadeus",
          "content": "Open CONFIG > API Settings from the system menu. Enter the API key and desired model name (e.g., claude-sonnet-latest) and save."
        }
      ]
    },
    "apiGroq": {
      "title": "Groq API",
      "desc": "LPU-powered platform with astonishing inference speeds. Characterized by rapid responses.",
      "modelList": {
        "available": [
          "llama-3.3-70b-versatile",
          "llama-3.1-8b-instant",
          "llama-3.2-11b-vision-preview",
          "qwen/qwen3-32b",
          "mixtral-8x7b-32768"
        ],
        "recommended": "llama-3.3-70b-versatile"
      },
      "steps": [
        {
          "title": "Groq Console",
          "content": {
            "rich": [
              {
                "text": "Access the "
              },
              {
                "link": "https://console.groq.com/",
                "text": "Groq Console (console.groq.com)"
              },
              {
                "text": " and log in."
              }
            ]
          }
        },
        {
          "title": "Generate Key",
          "content": "Select 'Create API Key' in the API Keys section to generate your key."
        },
        {
          "title": "Configure Amadeus",
          "content": "Open CONFIG > API Settings from the system menu. Enter the API key and desired model name (e.g., llama-3.3-70b-versatile) and save."
        }
      ]
    },
    "apiVertex": {
      "title": "Google Vertex AI",
      "desc": "Enterprise AI development platform on GCP. RealAmadeus automatically retrieves credentials by invoking the Google Cloud CLI via PowerShell.",
      "modelList": {
        "available": [
          "gemini-3.5-flash",
          "gemini-3.5-pro",
          "gemini-3.1-pro-preview",
          "gemini-3.1-flash-lite-preview",
          "gemini-omni-flash",
          "gemini-omni",
          "gemini-3-flash-preview",
          "gemini-3-pro-preview",
          "gemini-2.5-flash",
          "gemini-2.5-pro"
        ],
        "recommended": "gemini-3.5-flash"
      },
      "steps": [
        {
          "title": "Registration",
          "content": {
            "rich": [
              {
                "text": "Register for "
              },
              {
                "link": "https://console.cloud.google.com/",
                "text": "Google Cloud (console.cloud.google.com)"
              },
              {
                "text": "."
              }
            ]
          }
        },
        {
          "title": "Enable Trial Account",
          "content": "Enable your Google Cloud free trial account."
        },
        {
          "title": "Create Project",
          "content": "Create a new Google Cloud Project."
        },
        {
          "title": "Enable Vertex AI API",
          "content": "Enable the Vertex AI API for your project."
        },
        {
          "title": "Install Google Cloud CLI",
          "content": {
            "rich": [
              {
                "text": "Download and install the "
              },
              {
                "link": "https://dl.google.com/dl/cloudsdk/channels/rapid/GoogleCloudSDKInstaller.exe?hl=ja",
                "text": "Google Cloud CLI (gcloud) Installer"
              },
              {
                "text": ". After installation, open Command Prompt (cmd) and run "
              },
              {
                "code": "gcloud auth login"
              },
              {
                "text": " to authenticate."
              }
            ]
          }
        },
        {
          "title": "Configure Amadeus",
          "content": "Check the Project ID created in Step 3. Select 'Vertex AI' in the CONFIG menu. Enter your Project ID and Model Name (e.g., gemini-3.5-flash). The system will automatically fetch the auth token via gcloud."
        }
      ]
    }
  }
}
```

## Appendix B — Lucide icon markup (rendered with react-dom/server from lucide-react 0.563.0)

Replace `{CLASS}` with the class from §1.6.2. Attribute order matches React's output. `key` props are not rendered.

#### ArrowLeft

```html
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-arrow-left {CLASS}" aria-hidden="true"><path d="m12 19-7-7 7-7"></path><path d="M19 12H5"></path></svg>
```

#### FileText

```html
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-file-text {CLASS}" aria-hidden="true"><path d="M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z"></path><path d="M14 2v5a1 1 0 0 0 1 1h5"></path><path d="M10 9H8"></path><path d="M16 13H8"></path><path d="M16 17H8"></path></svg>
```

#### GitCommitVertical

```html
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-git-commit-vertical {CLASS}" aria-hidden="true"><path d="M12 3v6"></path><circle cx="12" cy="12" r="3"></circle><path d="M12 15v6"></path></svg>
```

#### Menu

```html
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-menu {CLASS}" aria-hidden="true"><path d="M4 5h16"></path><path d="M4 12h16"></path><path d="M4 19h16"></path></svg>
```

#### X

```html
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-x {CLASS}" aria-hidden="true"><path d="M18 6 6 18"></path><path d="m6 6 12 12"></path></svg>
```

#### Terminal

```html
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-terminal {CLASS}" aria-hidden="true"><path d="M12 19h8"></path><path d="m4 17 6-6-6-6"></path></svg>
```

#### Keyboard

```html
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-keyboard {CLASS}" aria-hidden="true"><path d="M10 8h.01"></path><path d="M12 12h.01"></path><path d="M14 8h.01"></path><path d="M16 12h.01"></path><path d="M18 8h.01"></path><path d="M6 8h.01"></path><path d="M7 16h10"></path><path d="M8 12h.01"></path><rect width="20" height="16" x="2" y="4" rx="2"></rect></svg>
```

#### Key

```html
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-key {CLASS}" aria-hidden="true"><path d="m15.5 7.5 2.3 2.3a1 1 0 0 0 1.4 0l2.1-2.1a1 1 0 0 0 0-1.4L19 4"></path><path d="m21 2-9.6 9.6"></path><circle cx="7.5" cy="15.5" r="5.5"></circle></svg>
```

#### Briefcase

```html
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-briefcase {CLASS}" aria-hidden="true"><path d="M16 20V4a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16"></path><rect width="20" height="14" x="2" y="6" rx="2"></rect></svg>
```

#### Download

```html
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-download {CLASS}" aria-hidden="true"><path d="M12 15V3"></path><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><path d="m7 10 5 5 5-5"></path></svg>
```

#### Command

```html
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-command {CLASS}" aria-hidden="true"><path d="M15 6v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3V6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3"></path></svg>
```

#### User

```html
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-user {CLASS}" aria-hidden="true"><path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"></path><circle cx="12" cy="7" r="4"></circle></svg>
```

#### Monitor

```html
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-monitor {CLASS}" aria-hidden="true"><rect width="20" height="14" x="2" y="3" rx="2"></rect><line x1="8" x2="16" y1="21" y2="21"></line><line x1="12" x2="12" y1="17" y2="21"></line></svg>
```

#### Zap

```html
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-zap {CLASS}" aria-hidden="true"><path d="M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z"></path></svg>
```

#### Cpu

```html
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-cpu {CLASS}" aria-hidden="true"><path d="M12 20v2"></path><path d="M12 2v2"></path><path d="M17 20v2"></path><path d="M17 2v2"></path><path d="M2 12h2"></path><path d="M2 17h2"></path><path d="M2 7h2"></path><path d="M20 12h2"></path><path d="M20 17h2"></path><path d="M20 7h2"></path><path d="M7 20v2"></path><path d="M7 2v2"></path><rect x="4" y="4" width="16" height="16" rx="2"></rect><rect x="8" y="8" width="8" height="8" rx="1"></rect></svg>
```

#### ArrowUpRight

```html
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-arrow-up-right {CLASS}" aria-hidden="true"><path d="M7 7h10v10"></path><path d="M7 17 17 7"></path></svg>
```

## Appendix C — V1 root `app/globals.css` (verbatim, 69 lines; must be in the V2 base stylesheet)

```css
@import "tailwindcss";

@theme {
  --color-amadeus: #d49e38;
  --font-amadeus: "Courier New", Courier, monospace;
  
  --animate-glitch-1: glitch-1 2.5s infinite linear alternate-reverse;
  --animate-glitch-2: glitch-2 3s infinite linear alternate-reverse;
  --animate-scanline: scanline 8s linear infinite;
  
  @keyframes glitch-1 {
    0% { clip-path: inset(20% 0 80% 0); }
    20% { clip-path: inset(60% 0 1% 0); }
    40% { clip-path: inset(40% 0 50% 0); }
    60% { clip-path: inset(80% 0 5% 0); }
    80% { clip-path: inset(10% 0 70% 0); }
    100% { clip-path: inset(30% 0 20% 0); }
  }
  @keyframes glitch-2 {
    0% { clip-path: inset(10% 0 60% 0); }
    20% { clip-path: inset(30% 0 20% 0); }
    40% { clip-path: inset(70% 0 20% 0); }
    60% { clip-path: inset(20% 0 60% 0); }
    80% { clip-path: inset(50% 0 30% 0); }
    100% { clip-path: inset(10% 0 80% 0); }
  }
  @keyframes scanline {
    0% { transform: translateY(-100%); }
    100% { transform: translateY(100%); }
  }
}

:root {
  --background: #000000;
  --foreground: #ededed;
}

body {
  background: var(--background);
  color: var(--foreground);
  font-family: Arial, Helvetica, sans-serif;
  overflow-x: hidden;
}

.glitch-text {
  position: relative;
}
.glitch-text::before,
.glitch-text::after {
  content: attr(data-text);
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: black;
}
.glitch-text::before {
  left: 2px;
  text-shadow: -1px 0 #ff0000;
  clip-path: inset(0 0 0 0);
  animation: glitch-1 2s infinite linear alternate-reverse;
}
.glitch-text::after {
  left: -2px;
  text-shadow: -1px 0 #0000ff;
  clip-path: inset(0 0 0 0);
  animation: glitch-2 2s infinite linear alternate-reverse;
}
```

## Appendix D — Components listed in scope but NOT used by these pages

`grep` over `app/`, `components/` and `hooks/` shows that both components are imported only by `app/page.tsx`, the DSUV portfolio at `/`, which is outside RealAmadeus. **No V2 page needs them.** They are specced here in case product wants them on the V2 landing.

### D.1 `components/KofiWidget.tsx` (`'use client'`, default export `KofiWidget`)

* Constants: `containerId = 'kofi-widget-container-dsuv'`; script `https://storage.ko-fi.com/cdn/scripts/overlay-widget.js`.
* Renders `<div id="kofi-widget-container-dsuv"></div>`.
* Effect, runs once:
  1. If the ref `isInitialized` is true, return.
  2. Define `initKofi()`: get the container by id (return if missing). If `window.kofiWidgetOverlay` exists, set `container.innerHTML = ''` (prevents duplication), then call
     `kofiWidgetOverlay.draw('dsuv_sg', { 'type': 'floating-chat', 'floating-chat.donateButton.text': 'Support me', 'floating-chat.donateButton.background-color': '#FF5E5B', 'floating-chat.donateButton.text-color': '#fff' }, 'kofi-widget-container-dsuv')`
     and set `isInitialized = true`. Errors go to `console.error('Error drawing Ko-fi widget:', e)`.
  3. If `document.querySelector('script[src="https://storage.ko-fi.com/cdn/scripts/overlay-widget.js"]')` exists: call `initKofi()` if `window.kofiWidgetOverlay` is already defined, else add a `'load'` listener calling `initKofi`. The listener is never removed (minor leak).
  4. Otherwise create a `<script src=… async>` with `onload = initKofi` and append it to `document.body`.
* Cleanup: `isInitialized = false`; empty the container's `innerHTML`.
* V2 note: this is third-party JS, which conflicts with the "only JS is Live2DCubismCore" rule in ARCHITECTURE.md. Skip it unless product explicitly asks.

### D.2 `components/ElveltBadge.tsx` (server component, default export `ElveltBadge`)

```html
<div class="fixed bottom-6 right-6 z-50">
  <a href="https://elvelt.com" target="_blank" class="flex items-center gap-2 px-4 py-2 bg-black/50 backdrop-blur-md border border-white/10 rounded-full text-xs text-gray-400 hover:text-white hover:border-white/30 transition-all group">
    <div class="w-2 h-2 rounded-full bg-white/20 group-hover:bg-green-500 transition-colors"></div>
    <span>Powered by ELVELT</span>
    {ArrowUpRight "w-3 h-3 opacity-50 group-hover:opacity-100"}
  </a>
</div>
```

It is a Next `<Link>` to an external URL with no `rel`; modern browsers imply `noopener` for `_blank`.


---

## Rust mapping

All names are recommendations. They are consistent with the existing skeleton (`crates/amadeus-ui/src/patterns.rs` already compiles `<Router base="/amadeus">`, `ParentRoute path!("/docs")`, `set_interval_with_handle` + `on_cleanup`, against leptos 0.8.23 / leptos_router 0.8.17).

### R.1 `amadeus-core` (wasm32 + native, no I/O)

```text
crates/amadeus-core/
├── locales/site/ja.json        ← Appendix A.1 (amadeus subtree)
├── locales/site/en.json        ← Appendix A.2
└── src/
    ├── site_i18n.rs            SiteLocale, SiteText (typed tree), RichText/Inline, site_text()
    ├── site.rs                 pure page helpers: clock, binary cell, path normalisation, active matching, DocProvider tables, legacy V1 URLs
    ├── release.rs              §6: RELEASE_REPO, ASSETS table, file_name/classify/static_release/group_by_os, ReleaseInfo DTOs
    └── changelog.rs            (re-export) site changelog types = SiteText.docs.changelog.versions; the in-app changelog stays separate (app spec)
```

```rust
// site_i18n.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SiteLocale { #[default] Ja, En }
impl SiteLocale {
    pub fn toggled(self) -> Self { match self { Self::Ja => Self::En, Self::En => Self::Ja } }
    /// Label of the language toggle button = the language you switch TO (V1: 'EN' when ja, 'JP' when en).
    pub fn toggle_label(self) -> &'static str { match self { Self::Ja => "EN", Self::En => "JP" } }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum Inline { Link { link: String, text: String }, Code { code: String }, Text { text: String } } // Link FIRST (it also has `text`)

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum RichText { Plain(String), Rich { rich: Vec<Inline> } }

// Typed tree mirroring Appendix A (field names = JSON keys, camelCase via #[serde(rename_all = "camelCase")]).
pub struct SiteText { pub release: ReleaseText, pub docs: DocsText, /* status, button, panels, hud, accessRestricted, login, preparation: optional, unused */ }
pub struct ReleaseText { pub title: String, pub message: String, pub windows_download: String, pub macos_download: String, pub linux_download: String,
                         pub lightweight_windows_download: String, pub download: String, pub github_repo: String, pub docs: String, pub web_version: String }
pub struct DocsText { pub title: String, pub back: String, pub sidebar: SidebarText, pub getting_started: GettingStartedText, pub download: DocsDownloadText,
                      pub changelog: ChangelogText, pub login_doc: LoginDocText, pub controls: ControlsText, pub api: ApiText,
                      #[serde(rename = "apiOpenAI")] pub api_openai: ProviderDoc, pub api_gemini: ProviderDoc, pub api_claude: ProviderDoc,
                      pub api_groq: ProviderDoc, pub api_vertex: ProviderDoc }
pub struct DocsDownloadText { /* title, desc, standardTitle, …, fallbackStandard: RichText, fallbackLightweight: RichText,
                                 steps: Vec<Step>, fileStructure: FileStructure, notice */ }
pub struct Step { pub title: String, pub content: RichText }
pub struct ChangelogText { pub title: String, pub desc: String, pub versions: Vec<ChangelogVersion> }
pub struct ChangelogVersion { pub version: String, pub product: String, pub items: Vec<String> }
pub struct ControlsText { pub title: String, pub desc: String, pub items: Vec<ControlItem> }   // ControlItem { key, desc }
pub struct ApiText { pub title: String, pub desc: String, pub steps: Vec<String>, pub notice: String, pub model_labels: ModelLabels }
pub struct ProviderDoc { pub title: String, pub desc: String, pub model_list: ModelList, pub steps: Vec<Step> } // ModelList { available: Vec<String>, recommended: String }

/// Parsed once per locale from include_str!; panics only on a malformed embedded file, which a unit test guarantees cannot ship.
pub fn site_text(locale: SiteLocale) -> &'static SiteText; // std::sync::OnceLock<SiteText> ×2 (works on wasm32)
```

Two JSON keys break `rename_all = "camelCase"` and need explicit renames: `apiOpenAI` (camelCase of `api_openai` is `apiOpenai`) → `#[serde(rename = "apiOpenAI")]`, and `windowsQT` in `fileStructure` (camelCase gives `windowsQt`) → `#[serde(rename = "windowsQT")]`. Every other key round-trips through camelCase. Plain Rust `static` data is a valid alternative to JSON; JSON is recommended for parity with the app's i18n and for editability.

```rust
// site.rs
pub const CLOCK_INTERVAL_MS: u64 = 50;
pub const BINARY_COLUMNS: usize = 16;
pub const BINARY_ROWS: usize = 40;

/// V1 `toLocaleTimeString('en-US', {hour12:false, …, fractionalSecondDigits: 2})`.
pub fn format_clock(h: u32, m: u32, s: u32, ms: u32) -> String { format!("{h:02}:{m:02}:{s:02}.{:02}", ms / 10) }

/// V1 `i % 3 === 0 ? '' : '/' + Math.random().toString(2).substring(2, 20)`, byte-exact. `rnd` ∈ [0,1).
pub fn binary_cell_text(row: usize, rnd: f64) -> String {
    if row % 3 == 0 { return String::new(); }
    let scaled = rnd * 262_144.0;                       // 2^18, exact
    let mut s = format!("{:018b}", scaled.floor() as u32);
    if scaled.fract() == 0.0 { while s.ends_with('0') { s.pop(); } } // JS prints no trailing zeros when the expansion ends early
    format!("/{s}")
}
pub fn binary_cell_has_spacer(row: usize) -> bool { row % 5 == 0 }

/// Strip the router base and a trailing slash: ("/amadeus/docs/", "/amadeus") → "/docs"; ("/amadeus", "/amadeus") → "/"; ("/docs", "") → "/docs".
pub fn normalize_site_path(pathname: &str, base: &str) -> String;
/// V1 DocsLayout.isActive: exact → equality, else plain string prefix.
pub fn is_active(current: &str, item_path: &str, exact: bool) -> bool { if exact { current == item_path } else { current.starts_with(item_path) } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocProvider { OpenAi, Gemini, Claude, Groq, Vertex }
impl DocProvider {
    pub const ALL: [DocProvider; 5] = [Self::OpenAi, Self::Gemini, Self::Claude, Self::Groq, Self::Vertex];
    pub fn slug(self) -> &'static str;                          // "openai" | "gemini" | "claude" | "groq" | "vertex"
    pub fn step_images(self) -> &'static [&'static [&'static str]]; // §3.12 table; len 1 → single style, >1 → multi style
    pub fn has_trailing_divider(self) -> bool { matches!(self, Self::Groq | Self::Vertex) }
    pub fn doc(self, t: &SiteText) -> &ProviderDoc;
}
pub fn step_image_alt(step_index: usize, part_index: Option<usize>) -> String; // "Step {i+1}" | "Step {i+1} - Part {j+1}"

pub mod legacy_v1 {
    pub const REPO_URL: &str = "https://github.com/dsuv-sg/RealAmadeus";
    pub const WIN_U: &str = "https://github.com/dsuv-sg/RealAmadeus/releases/download/RealAmadeusV1.4/RealAmadeus_V1.4U_Win64.zip";
    pub const MAC_U: &str = "https://github.com/dsuv-sg/RealAmadeus/releases/download/RealAmadeusV1.4/RealAmadeus_V1.4U_MacUniversal.zip";
    pub const LINUX_U: &str = "https://github.com/dsuv-sg/RealAmadeus/releases/download/RealAmadeusV1.4/RealAmadeus_V1.4U_Linux.zip";
    pub const WIN_Q: &str = "https://github.com/dsuv-sg/RealAmadeusQT/releases/download/RealAmadeusV1.4/RealAmadeus_V1.4Q_Win64.zip";
    pub const BADGE_U: &str = "V1.4U";
    pub const BADGE_Q: &str = "V1.4Q";
}
```

### R.2 `amadeus-ui` (Leptos 0.8 CSR)

```text
crates/amadeus-ui/src/
├── router.rs              AppRouter: <Router base> + <Routes fallback=NotFoundPage> (tree below)
├── platform.rs            is_tauri(), open_external(url)   (Tauri opener via IPC on desktop; no-op/anchor default on web)
├── site/
│   ├── mod.rs             SiteLocaleCtx(RwSignal<SiteLocale>), provide_site_locale(), use_site() -> SiteCtx { locale, text: Signal<&'static SiteText>, toggle: Callback<()> }
│   ├── base.rs            router_base() -> String (from document.baseURI), SiteBase ctx, site_href(path) -> String, use_site_path() -> Memo<String>
│   ├── icons.rs           enum Lucide {ArrowLeft, FileText, GitCommitVertical, Menu, X, Terminal, Keyboard, Key, Briefcase, Download, Command, User, Monitor, Zap, Cpu};
│   │                      fn lucide(icon: Lucide, class: &'static str) -> impl IntoView;
│   │                      fn svg_download(class: &'static str, stroke_width: &'static str), svg_github(), svg_eye(), svg_document()   (§1.6.3)
│   ├── chrome.rs          use_clock() -> ReadSignal<String>; #[component] BinaryBackground(trailing_space: bool, tick: Signal<String>);
│   │                      #[component] GridOverlay(); #[component] StatusHeader(time: Signal<String>); #[component] LangToggle(class: &'static str)
│   └── rich.rs            #[component] RichTextView(text: Signal<&'static RichText>)   (§1.4.2 link markup)
└── pages/
    ├── landing.rs         LandingPage                         (§3.1)
    ├── download.rs        DownloadPage + DownloadCard          (§3.2 now; §6 for V2 content)
    ├── changelog.rs       ChangelogPage                        (§3.3)
    ├── not_found.rs       NotFoundPage                         (§4)
    └── docs/
        ├── mod.rs         DocsLayout (+ DOCS_NAV: [DocsNavItem; 10], class literals)   (§3.4)
        ├── section.rs     DocSection(icon, title, children) = renderSection; DocsPageHeader(title); StepItem; ImageFrame; ModelsInfo
        ├── index.rs       DocsIndexPage     (§3.7)
        ├── download.rs    DocsDownloadPage + DocsDlButton (§3.8)
        ├── login.rs       DocsLoginPage     (§3.9)
        ├── controls.rs    DocsControlsPage  (§3.10)
        ├── api.rs         DocsApiPage       (§3.11)
        └── provider.rs    ProviderDocPage { provider: DocProvider }  (§3.12)
```

Router (static provider routes, so `/docs/api/foo` falls through to NotFound):

```rust
#[component]
pub fn AppRouter() -> impl IntoView {
    provide_site_locale();                       // above <Router>: survives navigation like V1's root LanguageProvider
    let base = router_base();                    // "/amadeus" (web) | "" (desktop)
    provide_context(SiteBase(base.clone()));
    view! {
        <Router base=base>
            <Routes fallback=|| view! { <NotFoundPage/> }>
                <Route path=path!("/") view=LandingPage/>
                <Route path=path!("/download") view=DownloadPage/>
                <Route path=path!("/changelog") view=ChangelogPage/>
                <ParentRoute path=path!("/docs") view=DocsLayout>
                    <Route path=path!("") view=DocsIndexPage/>
                    <Route path=path!("download") view=DocsDownloadPage/>
                    <Route path=path!("login") view=DocsLoginPage/>
                    <Route path=path!("controls") view=DocsControlsPage/>
                    <Route path=path!("api") view=DocsApiPage/>
                    <Route path=path!("api/openai") view=|| view! { <ProviderDocPage provider=DocProvider::OpenAi/> }/>
                    <Route path=path!("api/gemini") view=|| view! { <ProviderDocPage provider=DocProvider::Gemini/> }/>
                    <Route path=path!("api/claude") view=|| view! { <ProviderDocPage provider=DocProvider::Claude/> }/>
                    <Route path=path!("api/groq")   view=|| view! { <ProviderDocPage provider=DocProvider::Groq/> }/>
                    <Route path=path!("api/vertex") view=|| view! { <ProviderDocPage provider=DocProvider::Vertex/> }/>
                </ParentRoute>
                <Route path=path!("/app") view=AmadeusAppPage/>   // app subsystem
            </Routes>
        </Router>
    }
}
```

Check against leptos_router 0.8.17 that `path!("")` matches `/amadeus` **and** `/amadeus/`, and that `path!("/")` under base matches `/amadeus`. Add a wasm test or manual QA for both URL forms.

Idioms:

* **Links:** `<a href=site_href("/docs") class="…">`. leptos_router intercepts it (same origin, under base). Do not use `<A href="/docs">`, which yields an unprefixed href. `<A>` also adds `aria-current` (harmless but not V1 DOM).
* **Clock and background:** `let time = use_clock();`. In `BinaryBackground`, each text cell is `{move || { time.track(); binary_cell_text(i, js_sys::Math::random()) }}`. The spacer and the optional `" "` are static children. 16×40 cells are built once with iterators (`(0..16).map(|_| view!{…}).collect_view()`).
* **Locale-dependent text:** `{move || site.text.get().release.title.clone()}`. The whole page does not need to re-render.
* **Conditional classes:** select complete literals: `class=move || if active() { NAV_TOP_ACTIVE } else { NAV_TOP_INACTIVE }`. There are four nav literals (top/sub × active/inactive) and two aside literals (open/closed), each a full V1 token list concatenated in V1 order.
* **Page-scoped CSS:** `<main data-site-page="landing" class="…">` plus the §1.5.3 rules in `apps/web/style/site.css`. Import it from `tailwind.css` after `@import "tailwindcss";`, unlayered.
* **Images:** `src=format!("static/{file}")` (base-relative through `<base href>`).
* **External links on desktop:** `on:click=move |ev| if is_tauri() { ev.prevent_default(); open_external(url) }`.

### R.3 `amadeus-backend` / `amadeus-server` / `amadeus-desktop` (only for the §6 enhancement)

* `amadeus-backend::releases::fetch_latest(client: &reqwest::Client, repo: &str, token: Option<&str>) -> Result<ReleaseInfo>`, with `ReleaseCache { value: Option<(Instant, ReleaseInfo)> }` and a TTL of 600 s.
* `amadeus-server`: `GET {base}/app/api/releases/latest` returns `ReleaseInfo` JSON (camelCase); on failure with no cache it returns 502 `{"error": "..."}`. Optional env `GITHUB_TOKEN`.
* `amadeus-desktop`: `#[tauri::command] async fn releases_latest() -> Result<ReleaseInfo, String>`, plus tauri-plugin-opener for `open_external`.
* Transport: add `Transport::releases_latest()` (HTTP on web, IPC on desktop), following the existing Transport abstraction.

### R.4 Tests to add

* `format_clock(0,5,7,379)=="00:05:07.37"`, `(9,3,7,5)=="09:03:07.00"`, `(23,59,59,999)=="23:59:59.99"`.
* `binary_cell_text(0,_)==""`, `(39,_)==""`, `(1,0.5)=="/1"`, `(1,0.0)=="/"`, `(1,0.1)=="/000110011001100110"` (V1/V8 value). For random non-dyadic inputs the result matches `^/[01]{18}$`.
* `is_active` and `normalize_site_path`, using the examples in §3.4.2 and `site.rs`.
* i18n parity: both locales parse. Provider step counts are (4,3,4,3,6); controls has 5 items; changelog has 6 versions with identical `version` strings; model lists are identical across locales; and every `DocProvider::step_images().len()` equals the step count.
* `release::file_name` / `classify` round-trip for every `ASSETS` entry with a different version (e.g. `2.1.3`).
* CSS scan guard (CI): extract every class token from the page `.rs` files and assert that each arbitrary-value class (§1.5.2) is present in the built `dist/*.css`.
* Visual QA: compare V1 and V2 screenshots of each route in ja and en at 390px and 1440px width, after visiting `/app` first (CSS-leak regression, §1.5.4).

---

## Open questions / risks

1. **CSS leak (highest risk).** The app's unlayered `* { margin:0; padding:0; user-select:none }` and focus resets (V2 `amadeus.css`, loaded globally by `index.html`) would destroy every spacing utility on these pages. They must be scoped, e.g. with `:where(.amadeus-root …)`, or loaded only on `/app`. Coordinate with the CSS and app specs. V1 probably has this bug after back-navigation from the app; do not reproduce it.
2. **Tailwind extraction from Rust.** Any class assembled with `format!` or split across string pieces will silently disappear from the CSS. Use full literals and the CI scan guard in R.4. The heavy arbitrary values (`md:h-[calc(100vh-5rem)]`, `bg-[linear-gradient(…)]`, `bg-[size:4px_4px]`) are the most likely to be lost.
3. **Router base pitfalls.** Absolute `<A href>` values are not base-prefixed. `use_location().pathname` includes the base. Desktop base is `""` while web is `/amadeus`. Centralize `site_href` / `normalize_site_path` and test both builds.
4. **Desktop external links and downloads.** These need tauri-plugin-opener (`target="_blank"` and same-tab GitHub zip links do not work or leave the webview). Decide whether landing, download and docs are reachable on desktop at all; the window starts at `app`.
5. **Stale V1 content.** The landing still says «SYSTEM V1.4 RELEASED». Docs describe Unity/Qt editions, V1 file layouts, the PowerShell-based gcloud flow, desktop key bindings (Tab/right-click, WASD, Backspace, Alt+Enter/F11) and the V1 login screenshot. Product must decide which texts change for V2 (§6.5 proposes keys and copy). The login credentials «Salieri» / «MakiseKurisu» must match whatever the V2 app accepts.
6. **Release naming contract.** The §6 asset table and tag format (`v2.0.0` vs V1-style `RealAmadeusV2.0.0`) must be shared by `xtask`, CI and the UI (`amadeus_core::release::ASSETS`). Also decide whether CI uploads version-less aliases for `releases/latest/download/…` links. The GitHub repo link should probably change from `dsuv-sg/RealAmadeus` to `dsuv-sg/RealAmadeusRust`.
7. **Quirks to keep or fix.** Default: keep, for parity. Trivial to fix if product prefers: GitHub SVG typo (Q2); missing landing DOWNLOAD glow (Q1); empty docs download steps (Q4, probably intended `slice(0, 2)`); Groq step 3 using `Groq_STEP4.png` (Q5, intentional reuse?); smaller API notice on desktop; mobile docs menu opening off-screen (Q10); fixed strips blocking clicks (Q8); 20 Hz background flicker (Q3, could be reduced for `prefers-reduced-motion`).
8. **Locale persistence.** V1 persists nothing, and the spec keeps it that way (no new storage keys). If product wants persistence, use a new key (e.g. `Site_Locale`) or seed from the app's `Config_Language` (0 → ja, otherwise en). This is a deliberate deviation. `<html lang>` could follow the locale for accessibility (V1 keeps `en`).
9. **CSR first paint / SEO.** V1 server-rendered these pages (instant content, crawlable). V2 shows nothing until the wasm bundle loads. Mitigations: black `<body>` background (no white flash), a minimal static splash in `index.html` removed on mount, a small wasm (`wasm-opt z` is already configured), and optional pre-rendering of the docs routes later.
10. **Not-found behaviour.** Replicate Next's 404 page (with a temporary `document.title`), or redirect to `{base}`? Should the server return HTTP 404 for unknown paths, which requires a route list in `amadeus-server`?
11. **Performance and network.** 416 text nodes are rewritten at 20 Hz (V1 re-rendered the whole page, so V2 is cheaper). Images load eagerly (Vertex ≈ 3.45 MB); `loading="lazy"` is optional. V1 served root images as `no-store`; V2 can cache `static/*`.
12. **Accessibility deviations (optional).** The mobile menu button has no accessible name (icon is `aria-hidden`). Adding `aria-label`/`aria-expanded` would be a harmless deviation from V1.
13. **Third-party widgets.** KofiWidget and ElveltBadge are not used by these pages. Ko-fi would add third-party JS, conflicting with ARCHITECTURE.md's "only Live2DCubismCore JS" rule.
14. **Two changelog datasets.** The site changelog (Appendix A `docs.changelog`) and the in-app changelog (`lib/amadeus/locales` `changelog_v1x`) differ in wording. Both need V2 entries going forward; decide whether to unify them in `amadeus-core::changelog`.
15. **leptos_router route-matching details to verify.** Empty child path `path!("")` with and without trailing slash; scroll-to-top on navigation (`<A scroll>` default true; intercepted `<a>` scrolls unless `noscroll`); `ParentRoute` keeping `DocsLayout` mounted across docs navigations, which the mobile-menu state relies on.

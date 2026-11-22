# Porting spec: CSS subsystem (RealAmadeusWeb V1 → RealAmadeus V2 / Rust)

Subsystem: **css**. Target crates: `amadeus-ui` (Leptos 0.8 CSR, Trunk build), `amadeus-desktop` (Tauri 2, uses the same Trunk `dist/`), `amadeus-server` (axum, serves `dist/` and resources), `amadeus-live2d` (canvas creation contract only).

All paths are relative to `/home/elvelt_admin/Elvelt/dsuv` unless absolute. "L123" means line 123 of the file named in that section; when no file is named, `app/amadeus/app/globals.css` is meant.

---

## 0. Provenance / snapshot facts (read first)

| Fact | Value |
|---|---|
| Snapshot | working tree on 2026-11-16, branch `main`, HEAD `7e45174` |
| `app/amadeus/app/globals.css` vs HEAD | **modified, uncommitted**: +388 lines = L1249–1252 (`.mobile-achievements-trigger-button { display:none }`) and L2583–2966 (whole "Mobile Overrides for Mini Game and Achievements" media block). HEAD has CRLF line endings, working tree is LF. |
| Which version to port | **The working tree version** (2966 lines). The newest production build (`.next/static/chunks/1b90ae59afeb13d0.css`, built 2026-11-12) contains the new rules (`translateY(-14dvh)`, `.mobile-achievements-trigger-button`), i.e. what users currently get. |
| V1 compiled CSS (reference for parity tests) | `.next/static/chunks/301a5e2f106dc67e.css` = root Tailwind bundle (from `app/globals.css`), `.next/static/chunks/1b90ae59afeb13d0.css` = amadeus app CSS (from `app/amadeus/app/globals.css`). Dev equivalents: `.next/dev/static/chunks/app_globals_71f961d1.css`, `.next/dev/static/chunks/app_amadeus_app_globals_bdf5e8d4.css`. |
| Toolchain V1 | Next 16.1.6 (Turbopack; CSS minified by Lightning CSS with Next's default browserslist **`chrome 111, edge 111, firefox 111, safari 16.4`** from `node_modules/next/dist/shared/lib/modern-browserslist-target.js`), `tailwindcss` 4.1.18, `@tailwindcss/postcss` 4.1.18. `postcss.config.mjs` = `{ plugins: { "@tailwindcss/postcss": {} } }`. **No `tailwind.config.*`**, no `.browserslistrc`, no `browserslist` key in package.json. |
| Tailwind plugins | none installed (no `tailwindcss-animate`, no `tw-animate-css`). |

---

## 1. Executive summary (the facts that break a naive port)

1. **The app itself uses zero Tailwind utilities.** `components/amadeus/RealAmadeusApp.tsx` and `Live2DCanvas.tsx` only use ~200 hand-written classes from `app/amadeus/app/globals.css`. The only Tailwind utilities on the app's DOM come from two layouts: `<body class="bg-black text-white min-h-screen selection:bg-purple-500/30 selection:text-white">` (root layout) and `<div class="amadeus-root h-screen w-screen overflow-hidden">` (app layout). `h-screen`, `w-screen`, `bg-black`, `text-white` are **overridden** (no-ops) by unlayered rules (§3.3).
2. **Tailwind preflight IS load-bearing.** The app relies on Tailwind v4.1.18 `@layer base` (inherited `line-height:1.5` from `html`, `border:0 solid` on every element, `img/canvas{display:block}`, `img{max-width:100%;height:auto}`, form-control resets incl. transparent button background and `color:inherit`, `-webkit-tap-highlight-color:transparent`, `h1–h6{font-size:inherit;font-weight:inherit}`). The Rust build must ship the identical preflight, **layered**, loaded before the app CSS.
3. **The app CSS is unlayered** and therefore beats every Tailwind layer regardless of specificity. Never wrap it in `@layer`, never scope/prefix its selectors, never reorder it (equal-specificity overrides depend on source order; §12.4). Copy L67–2966 as one verbatim block.
4. **Fonts:** 1 remote `@import` (Google Fonts Noto Serif JP 300/400/700) + 10 `@font-face` rules with **absolute `/resources/fonts/...` URLs (10 occurrences, 9 files, ≈73.6 MB including three 24.5 MB CJK OTFs)** that must move under the base path. Plus a **runtime JS FontFace loader** that downloads `msmincho.ttc` from `raw.githubusercontent.com` (§5.4).
5. **Overlays are always mounted.** Visibility is driven by inline-styled wrapper `<div>`s (`opacity`, `pointer-events`, `transition: opacity 0.3s|0.25s ease-in-out`, `z-index` 2000/2500/3000/50). The CSS mount animations (`overlay-fade`, `menu-bg-fade`, `menu-content-fade`) therefore play once at page load while invisible. If the Rust UI conditionally mounts panels, fades and stacking will differ. Reproduce the wrappers exactly (§7.9).
6. **Mobile vs desktop is decided twice, differently.** JS chooses the DOM branch with `visualWidth <= 900 || (pointer:coarse && visualWidth <= 1200)`; CSS uses `@media (pointer: coarse), (max-width: 900px)`. Touch devices wider than 1200 px get the **desktop DOM + mobile media rules** (MENU toggle button, SEND button). Port both exactly (§8).
7. **Live2D canvas:** CSS says `width:3840px;height:2160px`, but the runtime creates the canvas with inline `display:block;width:100%;height:100%;pointer-events:none` and the React wrapper sets inline `pointer-events:auto;width:100%;height:100%`. The inline styles win; the Rust canvas must carry the same inline styles or the model renders at 2× size (§7.5.2).
8. **V1 shipped CSS was post-processed by Lightning CSS**: 6 × `-webkit-backdrop-filter` were added, duplicate `.amadeus-root` rules merged, numbers re-serialized (`scale(1.666667)` → `scale(1.66667)`). macOS WKWebView (Safari engine) needs the prefix, so the Rust build must replicate (§13).
9. **Dead CSS:** `app/amadeus/app/page.module.css` is never imported (create-next-app template). Inside globals.css: `.closing` rules, `.config-runtime*`, `.config-subtitle`, `.mobile-loading-title`, three never-referenced `@font-face` families, and more (§12.2). Keep them (verbatim copy) but do not build features around them.

---

## 2. Source file inventory and disposition

| File | Lines | Imported by | Role | V2 disposition |
|---|---|---|---|---|
| `app/amadeus/app/globals.css` | 2966 | `app/amadeus/app/layout.tsx` (`import "./globals.css"`) | All app styling (desktop 1920×1080 stage + mobile layout), fonts | Split: L1 → fonts file (or `<link>`), L3–65 → `amadeus-fonts.css` (URLs rewritten), L67–2966 → `amadeus-app.css` **verbatim** |
| `app/globals.css` | 69 | `app/layout.tsx` (root layout of the whole DSUV site) | Tailwind v4 entry (`@import "tailwindcss"`), custom `@theme`, `:root` vars, `body` rule, `.glitch-text` | `tailwind.css` input for Tailwind CLI 4.1.18, verbatim plus `@source` lines |
| `app/amadeus/app/page.module.css` | 142 | **nothing** (grep for `page.module`/`module.css` finds no importer) | create-next-app template (`.page`, `.main`, `.intro`, `.ctas`, `a.primary`, `a.secondary`, `.logo`, uses `var(--font-geist-sans)`) | **Do not port** (dead) |
| styled-jsx `<style jsx>` blocks in `app/amadeus/page.tsx`, `app/amadeus/changelog/page.tsx`, `app/amadeus/download/page.tsx` | – | page components | `.text-shadow-glow`, `.glow-on-hover:hover`, `@keyframes scroll-bg`, `.animate-scroll-bg` (page-scoped, values differ per page) | `site-pages.css` with page-scoped selectors (§4.5), web only, only if landing/docs pages are ported |
| Tailwind utilities in `app/amadeus/{page,changelog/page,download/page}.tsx`, `app/amadeus/docs/**`, `hooks/useTranslation.tsx` | – | – | landing/docs styling | Generated by Tailwind CLI from the Rust sources (§4.4) |

---

## 3. How V1 assembles the document for `/amadeus/app` (cascade context)

### 3.1 DOM chain

```html
<html lang="en" class="dark">                          <!-- app/layout.tsx -->
<head>
  <!-- the relative order of the two stylesheets is what matters (root layout CSS first, nested layout CSS second);
       the order of the other head tags below is approximate -->
  <link rel="stylesheet" href="/_next/static/chunks/301a5e2f106dc67e.css">  <!-- root Tailwind bundle -->
  <link rel="stylesheet" href="/_next/static/chunks/1b90ae59afeb13d0.css">  <!-- amadeus globals.css -->
  <meta http-equiv="Cache-Control" content="no-cache, no-store, must-revalidate">
  <meta http-equiv="Pragma" content="no-cache">
  <meta http-equiv="Expires" content="0">
  <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
  <meta name="theme-color" content="#000000">
  <title>Amadeus</title> ... (metadata, icons: /amadeus_logo.png, manifest /amadeus/app/manifest.webmanifest)
</head>
<body class="bg-black text-white min-h-screen selection:bg-purple-500/30 selection:text-white">
  <!-- LanguageProvider (hooks/useTranslation.tsx) renders no DOM -->
  <div class="amadeus-root h-screen w-screen overflow-hidden">   <!-- app/amadeus/app/layout.tsx -->
    <div class="real-amadeus-root"> ... </div>                   <!-- or "real-amadeus-root mobile-root" -->
  </div>
  <next-route-announcer>…</next-route-announcer>  <!-- Next internal, not styled -->
</body>
</html>
```

Required in V2 `index.html`: `lang="en"` (affects CJK glyph selection with the Noto Serif CJK OTFs via `locl`; keep "en" for parity), `class="dark"` (no effect, `dark:` variant is media-based in v4 and unused; keep for parity), the same body classes, `viewport-fit=cover` (required for non-zero `env(safe-area-inset-*)` on iOS), theme-color `#000000`.

### 3.2 Stylesheet order

1. Root Tailwind bundle: `@layer properties` → `@layer theme` → `@layer base` (preflight) → `@layer components;` (empty) → `@layer utilities` → **unlayered** `:root{--background:#000;--foreground:#ededed}`, `body{...}`, `.glitch-text...`, `@property --tw-*`, `@keyframes pulse/glitch-1/glitch-2`.
2. Amadeus bundle: `@import "https://fonts.googleapis.com/css2?family=Noto+Serif+JP:wght@300;400;700&display=swap";` (imported rules are treated as positioned here, i.e. before the local `@font-face`s) → 10 local `@font-face` → all app rules, **unlayered**.

### 3.3 Cascade consequences (unlayered beats layered for normal declarations)

| Element | Declared by Tailwind (layered) | Overridden by (unlayered) | Effective in V1 |
|---|---|---|---|
| `body` | `.bg-black{background-color:var(--color-black)}` | `body{background:var(--background)}` (#000) | #000 |
| `body` | `.text-white{color:var(--color-white)}` | `body{color:var(--foreground)}` | **#ededed** (not white) |
| `body` | `.min-h-screen{min-height:100vh}` | – | min-height 100vh |
| `body` | preflight `html,:host{font-family:ui-sans-serif,...}` | `body{font-family:Arial, Helvetica, sans-serif; overflow-x:hidden}` | Arial (then `.amadeus-root` overrides for the app subtree) |
| `div.amadeus-root` | `.h-screen{height:100vh}`, `.w-screen{width:100vw}`, `.overflow-hidden{overflow:hidden}` | `.amadeus-root{width:100%;height:100%;min-height:100dvh;overflow:hidden;...}` | width 100% of body, height `100%` of an auto-height parent ⇒ auto, min-height 100dvh ⇒ effectively 100dvh tall. **The three Tailwind classes are no-ops.** |
| every element | preflight `*{margin:0;padding:0;box-sizing:border-box;border:0 solid}` | app `*{box-sizing:border-box;margin:0;padding:0;user-select:none;-webkit-user-select:none}` | same values + user-select none |
| `button,input,select` | preflight `font:inherit;...;color:inherit;background-color:#0000;border-radius:0;opacity:1;letter-spacing:inherit` | app `button,input,select{font:inherit}` (L111–115) | font inherited **including line-height** (the `font` shorthand resets line-height to inherit) |
| `textarea` | preflight `font:inherit;...` | (app rule does not list textarea) | inherited via preflight only |
| `:focus` | preflight `:-moz-focusring{outline:auto}` | app `*:focus{outline:none}`, `*:focus-visible{outline:none!important}` | no focus rings anywhere |
| `::selection` (body subtree) | `.selection\:bg-purple-500\/30` / `.selection\:text-white` | – | selection bg `#ac4bff4d` (or `color-mix(in oklab, var(--color-purple-500) 30%, transparent)`), text #fff. Only visible inside `input`/`textarea` because `* {user-select:none}`. |
| everything | preflight `html{line-height:1.5}` | only where the app sets `line-height` | **all app text without an explicit line-height uses 1.5** (e.g. `.config-row`, `.help-entry .description`, `.mg-question-box`, `.overlay-close-button`, `.custom-select-option`, `.mobile-settings-row`). Without preflight these would be `normal` → visible layout drift. |

Conclusion: V2 must load (a) Tailwind's base layer with these exact rules, (b) the root unlayered `:root`/`body` rules, (c) the app CSS unlayered, in that order.

---

## 4. Tailwind v4 dependency

### 4.1 Root input `app/globals.css` (verbatim, 69 lines)

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

Usage audit (grep over `app/`, `components/`, `hooks/`, `lib/`): `--color-amadeus`, `--font-amadeus`, `animate-glitch-1`, `animate-glitch-2`, `animate-scanline`, `.glitch-text`, `data-text` are **used nowhere**. Tailwind therefore emitted no `--color-amadeus`/`--font-amadeus` variables and no `scanline` keyframes; it did emit `@keyframes glitch-1/glitch-2` because the plain `.glitch-text` rules reference them. Keep the file verbatim (harmless) so the `:root`/`body` rules survive.

### 4.2 Exact Tailwind output the APP consumes (from V1 build `301a5e2f106dc67e.css`)

Utilities (inside `@layer utilities`):

```css
.h-screen{height:100vh}
.w-screen{width:100vw}
.overflow-hidden{overflow:hidden}
.bg-black{background-color:var(--color-black)}
.text-white{color:var(--color-white)}
.min-h-screen{min-height:100vh}
.selection\:bg-purple-500\/30 ::selection{background-color:#ac4bff4d}
@supports (color:color-mix(in lab, red, red)){.selection\:bg-purple-500\/30 ::selection{background-color:color-mix(in oklab,var(--color-purple-500)30%,transparent)}}
.selection\:bg-purple-500\/30::selection{background-color:#ac4bff4d}
@supports (color:color-mix(in lab, red, red)){.selection\:bg-purple-500\/30::selection{background-color:color-mix(in oklab,var(--color-purple-500)30%,transparent)}}
.selection\:text-white ::selection,.selection\:text-white::selection{color:var(--color-white)}
```

Also emitted and harmless: `.visible{visibility:visible}` (Tailwind utility whose name collides with the app's `.achievement-toast.visible` class; V1 had it because the scanner saw the token `visible`).

Theme variables needed by the above + preflight (from `@layer theme`): `--font-sans: ui-sans-serif,system-ui,sans-serif,"Apple Color Emoji","Segoe UI Emoji","Segoe UI Symbol","Noto Color Emoji"`, `--font-mono: ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,"Liberation Mono","Courier New",monospace`, `--color-black:#000`, `--color-white:#fff`, `--color-purple-500:#ac4bff` (overridden to `lab(52.0183% 66.11 -78.2316)` inside `@supports (color:lab(0% 0 0))`), `--default-font-family:var(--font-sans)`, `--default-mono-font-family:var(--font-mono)`.

Full V1 theme layer (reference; the V2 Tailwind CLI will emit its own, containing only the variables V2 sources use):

```css
@layer theme{:root,:host{--font-sans:ui-sans-serif,system-ui,sans-serif,"Apple Color Emoji","Segoe UI Emoji","Segoe UI Symbol","Noto Color Emoji";--font-mono:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,"Liberation Mono","Courier New",monospace;--color-red-400:#ff6568;--color-red-500:#fb2c36;--color-green-500:#00c758;--color-blue-500:#3080ff;--color-purple-500:#ac4bff;--color-gray-400:#99a1af;--color-gray-500:#6a7282;--color-gray-600:#4a5565;--color-black:#000;--color-white:#fff;--spacing:.25rem;--container-xs:20rem;--container-md:28rem;--container-2xl:42rem;--container-3xl:48rem;--container-4xl:56rem;--container-5xl:64rem;--container-7xl:80rem;--text-xs:.75rem;--text-xs--line-height:calc(1/.75);--text-sm:.875rem;--text-sm--line-height:calc(1.25/.875);--text-base:1rem;--text-base--line-height:calc(1.5/1);--text-lg:1.125rem;--text-lg--line-height:calc(1.75/1.125);--text-xl:1.25rem;--text-xl--line-height:calc(1.75/1.25);--text-2xl:1.5rem;--text-2xl--line-height:calc(2/1.5);--text-3xl:1.875rem;--text-3xl--line-height:calc(2.25/1.875);--text-4xl:2.25rem;--text-4xl--line-height:calc(2.5/2.25);--text-5xl:3rem;--text-5xl--line-height:1;--text-6xl:3.75rem;--text-6xl--line-height:1;--text-7xl:4.5rem;--text-7xl--line-height:1;--text-8xl:6rem;--text-8xl--line-height:1;--font-weight-light:300;--font-weight-medium:500;--font-weight-bold:700;--font-weight-black:900;--tracking-tighter:-.05em;--tracking-tight:-.025em;--tracking-wide:.025em;--tracking-wider:.05em;--tracking-widest:.1em;--leading-normal:1.5;--leading-relaxed:1.625;--leading-loose:2;--radius-sm:.25rem;--radius-md:.375rem;--radius-lg:.5rem;--radius-2xl:1rem;--radius-3xl:1.5rem;--ease-out:cubic-bezier(0,0,.2,1);--ease-in-out:cubic-bezier(.4,0,.2,1);--animate-pulse:pulse 2s cubic-bezier(.4,0,.6,1)infinite;--blur-sm:8px;--blur-md:12px;--blur-3xl:64px;--default-transition-duration:.15s;--default-transition-timing-function:cubic-bezier(.4,0,.2,1);--default-font-family:var(--font-sans);--default-mono-font-family:var(--font-mono)}
@supports (color:lab(0% 0 0)){:root,:host{--color-red-400:lab(63.7053% 60.745 31.3109);--color-red-500:lab(55.4814% 75.0732 48.8528);--color-green-500:lab(70.5521% -66.5147 45.8073);--color-blue-500:lab(54.1736% 13.3369 -74.6839);--color-purple-500:lab(52.0183% 66.11 -78.2316);--color-gray-400:lab(65.9269% -.832707 -8.17473);--color-gray-500:lab(47.7841% -.393182 -10.0268);--color-gray-600:lab(35.6337% -1.58697 -10.8425)}}}
```

### 4.3 Preflight (V1 `@layer base`, verbatim from the build, one rule per line)

```css
@layer base{*,:after,:before,::backdrop{box-sizing:border-box;border:0 solid;margin:0;padding:0}
::file-selector-button{box-sizing:border-box;border:0 solid;margin:0;padding:0}
html,:host{-webkit-text-size-adjust:100%;tab-size:4;line-height:1.5;font-family:var(--default-font-family,ui-sans-serif,system-ui,sans-serif,"Apple Color Emoji","Segoe UI Emoji","Segoe UI Symbol","Noto Color Emoji");font-feature-settings:var(--default-font-feature-settings,normal);font-variation-settings:var(--default-font-variation-settings,normal);-webkit-tap-highlight-color:transparent}
hr{height:0;color:inherit;border-top-width:1px}
abbr:where([title]){-webkit-text-decoration:underline dotted;text-decoration:underline dotted}
h1,h2,h3,h4,h5,h6{font-size:inherit;font-weight:inherit}
a{color:inherit;-webkit-text-decoration:inherit;-webkit-text-decoration:inherit;-webkit-text-decoration:inherit;-webkit-text-decoration:inherit;text-decoration:inherit}
b,strong{font-weight:bolder}
code,kbd,samp,pre{font-family:var(--default-mono-font-family,ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,"Liberation Mono","Courier New",monospace);font-feature-settings:var(--default-mono-font-feature-settings,normal);font-variation-settings:var(--default-mono-font-variation-settings,normal);font-size:1em}
small{font-size:80%}
sub,sup{vertical-align:baseline;font-size:75%;line-height:0;position:relative}
sub{bottom:-.25em}
sup{top:-.5em}
table{text-indent:0;border-color:inherit;border-collapse:collapse}
:-moz-focusring{outline:auto}
progress{vertical-align:baseline}
summary{display:list-item}
ol,ul,menu{list-style:none}
img,svg,video,canvas,audio,iframe,embed,object{vertical-align:middle;display:block}
img,video{max-width:100%;height:auto}
button,input,select,optgroup,textarea{font:inherit;font-feature-settings:inherit;font-variation-settings:inherit;letter-spacing:inherit;color:inherit;opacity:1;background-color:#0000;border-radius:0}
::file-selector-button{font:inherit;font-feature-settings:inherit;font-variation-settings:inherit;letter-spacing:inherit;color:inherit;opacity:1;background-color:#0000;border-radius:0}
:where(select:is([multiple],[size])) optgroup{font-weight:bolder}
:where(select:is([multiple],[size])) optgroup option{padding-inline-start:20px}
::file-selector-button{margin-inline-end:4px}
::placeholder{opacity:1}
@supports (not ((-webkit-appearance:-apple-pay-button))) or (contain-intrinsic-size:1px){::placeholder{color:currentColor}
@supports (color:color-mix(in lab, red, red)){::placeholder{color:color-mix(in oklab,currentcolor 50%,transparent)}
}
}
textarea{resize:vertical}
::-webkit-search-decoration{-webkit-appearance:none}
::-webkit-date-and-time-value{min-height:1lh;text-align:inherit}
::-webkit-datetime-edit{display:inline-flex}
::-webkit-datetime-edit-fields-wrapper{padding:0}
::-webkit-datetime-edit{padding-block:0}
::-webkit-datetime-edit-year-field{padding-block:0}
::-webkit-datetime-edit-month-field{padding-block:0}
::-webkit-datetime-edit-day-field{padding-block:0}
::-webkit-datetime-edit-hour-field{padding-block:0}
::-webkit-datetime-edit-minute-field{padding-block:0}
::-webkit-datetime-edit-second-field{padding-block:0}
::-webkit-datetime-edit-millisecond-field{padding-block:0}
::-webkit-datetime-edit-meridiem-field{padding-block:0}
::-webkit-calendar-picker-indicator{line-height:1}
:-moz-ui-invalid{box-shadow:none}
button,input:where([type=button],[type=reset],[type=submit]){appearance:button}
::file-selector-button{appearance:button}
::-webkit-inner-spin-button{height:auto}
::-webkit-outer-spin-button{height:auto}
[hidden]:where(:not([hidden=until-found])){display:none!important}
}
```

Load-bearing preflight behaviors for the app (would break if preflight were missing):

| Preflight rule | App consequence |
|---|---|
| `html{line-height:1.5}` | inherited by every app element lacking `line-height` (see §3.3) |
| `*{border:0 solid}` | default `border-style:solid; border-width:0` → `.achievement-row{border:1px solid}` gets its color from `.unlocked/.locked`; buttons without explicit borders have none |
| `img,…,canvas{display:block;vertical-align:middle}` | no inline baseline gaps under `.menu-icon img`, `.login-button img`, `.config-select-arrow img`, the Live2D canvas |
| `img,video{max-width:100%;height:auto}` | `.mobile-loading-logo` has **no app rule** → it is `display:block; max-width:100%; height:auto` and is stretched across the flex column (align-items:stretch) of `.mobile-loading-panel`; other images set explicit sizes (unlayered wins over `height:auto`), but `max-width:100%` still applies to all of them (no effect at current sizes) |
| `button,…{background-color:#0000;color:inherit;border-radius:0}` | buttons that don't set a background are transparent (e.g. `.status-achievement-row` also sets it, others rely on it) |
| `h1–h6{font-size:inherit;font-weight:inherit}` | `.overlay-header h2` sets 64px/400 explicitly; mobile backlog h2 24px; consistent |
| `ol,ul,menu{list-style:none}` | change-log `ul` (also set by app) |
| `code,kbd,samp,pre{font-family:mono;font-size:1em}` | `pre.boot-text` overrides family/size explicitly |
| `-webkit-tap-highlight-color:transparent` on html | no grey tap flashes on mobile web |
| `::placeholder{color:color-mix(currentcolor 50%)}` | only `.chat-input` has a placeholder and overrides it |

### 4.4 Landing/docs pages Tailwind dependency (only if those pages are ported)

Files: `app/amadeus/page.tsx` (landing `/amadeus`), `app/amadeus/changelog/page.tsx`, `app/amadeus/download/page.tsx`, `app/amadeus/docs/layout.tsx`, `app/amadeus/docs/components.tsx`, `app/amadeus/docs/page.tsx`, `app/amadeus/docs/download/page.tsx`, `app/amadeus/docs/controls/page.tsx`, `app/amadeus/docs/login/page.tsx`, plus JSX inside `hooks/useTranslation.tsx` (links with `text-[#d49e38] underline hover:text-[#f0b95b] transition-colors`, and bare `<code>`).

Complete token inventory (309 tokens; every one must appear literally in a scanned V2 source file so the Tailwind scanner generates it):

```
-left-5 -left-[9px] -ml-2 absolute animate-in animate-pulse animate-scroll-bg backdrop-blur-md backdrop-blur-sm
bg-[#050505] bg-[#111] bg-[#d49e38] bg-[#d49e38]/10 bg-[#d49e38]/20 bg-[#d49e38]/40 bg-[#d49e38]/5
bg-[linear-gradient(rgba(0,0,0,0)_0%,rgba(0,0,0,1)_90%)]
bg-[linear-gradient(rgba(212,158,56,0.05)_1px,transparent_1px),linear-gradient(90deg,rgba(212,158,56,0.05)_1px,transparent_1px)]
bg-[size:4px_4px] bg-black bg-black/40 bg-black/50 bg-black/90 bg-black/95 bg-gradient-to-b bg-gradient-to-l bg-gradient-to-r
bg-transparent block blur-md border border-2 border-[#d49e38] border-[#d49e38]/10 border-[#d49e38]/20 border-[#d49e38]/30
border-[#d49e38]/40 border-[#d49e38]/80 border-b border-b-0 border-b-4 border-l border-l-2 border-l-[#d49e38]
border-l-transparent border-r bottom-0 bottom-8 break-all drop-shadow-[0_0_20px_rgba(212,158,56,0.3)]
drop-shadow-[0_0_2px_rgba(212,158,56,0.3)] drop-shadow-[0_0_5px_rgba(212,158,56,0.5)] drop-shadow-[0_0_5px_rgba(212,158,56,0.6)]
drop-shadow-[0_0_8px_rgba(212,158,56,0.5)] duration-300 duration-500 duration-700 ease-in-out fade-in filter fixed flex flex-1
flex-col flex-shrink-0 flex-wrap font-bold font-medium font-mono from-[#d49e38] from-[#d49e38]/10 from-[#d49e38]/60
from-transparent gap-1 gap-10 gap-12 gap-2 gap-3 gap-4 gap-8 glow-on-hover grid grid-cols-1 grid-cols-2 group
group-hover:-translate-x-1 group-hover:bg-[#d49e38]/20 group-hover:border-b-2 group-hover:opacity-100
group-hover:translate-y-[2px] h-1.5 h-14 h-3 h-4 h-5 h-6 h-64 h-8 h-[200%] h-auto h-full h-px hidden hover:bg-[#d49e38]
hover:bg-[#d49e38]/10 hover:bg-[#d49e38]/5 hover:border-[#d49e38] hover:border-[#d49e38]/60 hover:opacity-100 hover:scale-110
hover:text-[#d49e38] hover:text-[#f0b95b] hover:text-black inline-flex inset-0 items-center items-start justify-between
justify-center leading-loose leading-none leading-normal leading-relaxed left-0 lg:p-16 max-h-0 max-h-screen max-w-2xl
max-w-3xl max-w-4xl max-w-5xl max-w-7xl max-w-md max-w-xs mb-0.5 mb-1 mb-10 mb-12 mb-14 mb-16 mb-2 mb-3 mb-4 mb-6 mb-8
md:block md:flex-row md:grid-cols-2 md:grid-cols-4 md:h-96 md:h-[calc(100vh-5rem)] md:hidden md:max-h-screen md:overflow-auto
md:overflow-y-auto md:p-10 md:pt-20 md:self-start md:sticky md:text-2xl md:text-3xl md:text-4xl md:text-base md:text-sm
md:text-xs md:top-20 md:w-64 md:w-96 md:w-auto min-h-screen min-w-[160px] mix-blend-overlay ml-4 mt-0 mt-0.5 mt-1.5 mt-12
mt-20 mt-6 mt-8 mx-auto object-contain object-cover opacity-0 opacity-10 opacity-100 opacity-20 opacity-40 opacity-50
opacity-60 opacity-70 opacity-80 opacity-85 opacity-90 opacity-[0.03] overflow-hidden p-1.5 p-12 p-2 p-3 p-4 p-5 p-6 p-8
pb-1 pb-2 pb-24 pb-4 pl-2 pl-6 pl-8 pointer-events-none pt-32 px-10 px-2 px-3 px-4 px-6 py-0.5 py-1 py-1.5 py-2 py-4 py-6
relative right-8 rounded-full rounded-lg rounded-md rounded-sm scale-90 selection:bg-[#d49e38] selection:text-black
shadow-[0_0_15px_rgba(212,158,56,0.05)] shadow-[0_0_20px_rgba(212,158,56,0.05)] shadow-[0_0_20px_rgba(212,158,56,0.15)]
shadow-[inset_0_0_30px_rgba(0,0,0,0.8)] sm:flex-row sm:grid-cols-3 sm:items-center space-y-1 space-y-10 space-y-12 space-y-2
space-y-3 space-y-4 space-y-6 space-y-8 sticky text-2xl text-3xl text-[#888] text-[#d49e38] text-[#d49e38]/40
text-[#d49e38]/60 text-[#d49e38]/70 text-[#d49e38]/80 text-[#d49e38]/90 text-[10px] text-[11px] text-[8px] text-[9px]
text-center text-lg text-shadow-glow text-sm text-xl text-xs to-[#d49e38]/30 to-transparent top-0 top-1 top-2 top-8
tracking-[0.2em] tracking-[0.3em] tracking-[0.4em] tracking-[0.5em] tracking-tighter tracking-wide tracking-wider
tracking-widest transform transition-all transition-colors transition-opacity transition-transform underline uppercase
via-[#d49e38]/10 via-[#d49e38]/50 w-1 w-1.5 w-24 w-3 w-32 w-4 w-5 w-6 w-64 w-8 w-[2px] w-full whitespace-nowrap z-0 z-10
z-30 z-40 z-50
```

Notes:
- `animate-in`, `fade-in` produce **no CSS** in V1 (plugin not installed; verified absent from the build). `duration-700` does generate a transition-duration but without a transition it does nothing. Dead in V1 → keep or drop, no visual effect.
- `animate-scroll-bg`, `text-shadow-glow`, `glow-on-hover` are **not Tailwind**; they come from styled-jsx (§4.5).
- `flex-shrink-0` is still generated by Tailwind 4.1.18 (verified present).
- Tailwind v4 wraps every `hover:` variant in `@media (hover:hover)` (verified in the V1 build) → hover styles never apply on touch-only devices.
- Breakpoints (v4 defaults): `sm` ≥ 40rem (640 px), `md` ≥ 48rem (768 px), `lg` ≥ 64rem (1024 px).
- lucide-react icons (`^0.563.0`; ArrowLeft, FileText, GitCommitVertical, Menu, X, Terminal, Keyboard, Key, Briefcase, Download, Zap, Monitor, Cpu, Command, User) render `<svg ... width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-<name> <className>">`; V2 must inline identical SVG paths (from lucide 0.563) with the same Tailwind classes.
- The docs sidebar links to `/amadeus/docs/api`, `/amadeus/docs/api/{openai,gemini,claude,groq,vertex}` but **those routes do not exist** in V1 (no `app/amadeus/docs/api/` directory) → 404 in V1.

### 4.5 styled-jsx blocks (verbatim) and how to port them

`app/amadeus/page.tsx` (landing):
```css
.text-shadow-glow {
    text-shadow: 0 0 10px rgba(212, 158, 56, 0.5);
}
.glow-on-hover:hover {
    box-shadow: 0 0 20px rgba(212, 158, 56, 0.4);
}
@keyframes scroll-bg {
    0% { transform: translateY(0); }
    100% { transform: translateY(-50%); }
}
.animate-scroll-bg {
    animation: scroll-bg 60s linear infinite;
}
```
`app/amadeus/changelog/page.tsx`:
```css
.text-shadow-glow {
    text-shadow: 0 0 15px rgba(212, 158, 56, 0.6);
}
@keyframes scroll-bg {
    0% { transform: translateY(0); }
    100% { transform: translateY(-50%); }
}
.animate-scroll-bg {
    animation: scroll-bg 60s linear infinite;
}
```
`app/amadeus/download/page.tsx`:
```css
.text-shadow-glow {
    text-shadow: 0 0 15px rgba(212, 158, 56, 0.6);
}
.glow-on-hover:hover {
    box-shadow: 0 0 30px rgba(212, 158, 56, 0.3);
}
@keyframes scroll-bg {
    0% { transform: translateY(0); }
    100% { transform: translateY(-50%); }
}
.animate-scroll-bg {
    animation: scroll-bg 60s linear infinite;
}
```

Scoping quirk: styled-jsx adds its scope class only to lowercase DOM elements (and to capitalized identifiers bound in the same function scope) — see `node_modules/styled-jsx/dist/babel/index.js` L22258. On the landing page `glow-on-hover` sits on a `<Link>` (module-level import) ⇒ **no scope class ⇒ the glow never applies in V1**. On the download page it sits on `<a>` ⇒ works. `.text-shadow-glow` is on `<h1>` (works); `.animate-scroll-bg` on `<div>` (works). Next uses the SWC port of styled-jsx with the same rule — confirm visually if exact parity matters.

V2 port: one `site-pages.css`, each page's root `<main>` gets a page class (`page-landing`, `page-changelog`, `page-download`), selectors become `.page-landing .text-shadow-glow {…}` etc. For landing parity, do not put `glow-on-hover` on the DOWNLOAD link (or omit the landing `.glow-on-hover` rule). Inline styles in these pages (grid overlay `backgroundImage: 'linear-gradient(#333 1px, transparent 1px), linear-gradient(90deg, #333 1px, transparent 1px)'`, `backgroundSize: '40px 40px'`; docs scanline overlay `repeating-linear-gradient(0deg, transparent, transparent 1px, #d49e38 1px, #d49e38 2px)`; logo `animationDuration: '4s'`) must be copied as inline styles.

### 4.6 Tailwind build pipeline for Rust (recommended)

- Use the **Tailwind v4.1.18 standalone CLI** (no Node needed). With Trunk: `Trunk.toml` `[tools] tailwindcss = "4.1.18"` and `<link data-trunk rel="tailwind-css" href="styles/tailwind.css" />` (verify the Trunk version in use supports Tailwind v4 inputs; otherwise invoke the CLI from a `pre_build` hook: `tailwindcss -i styles/tailwind.css -o styles/dist/tailwind.css --minify`).
- Make sources explicit (paths are relative to the CSS file):
  ```css
  @import "tailwindcss" source(none);
  @source "../src/**/*.rs";
  @source "../index.html";
  ```
  (`source(none)` avoids scanning `resources/`, `target/`, locale JSON. V1 scanned the whole DSUV repo; extra utilities are harmless, missing ones are not.)
- Write every Tailwind class as a **complete string literal** in Rust (`view!{ <div class="max-h-0 md:max-h-screen ..."> }`, or `if active { "bg-[#d49e38]/10 text-[#d49e38] ..." } else { "..." }`). Never build Tailwind class names with `format!`; avoid Leptos `class:foo=` for Tailwind classes containing `/`, `[`, `:` (scanner/macro friction). The app's own classes (`backlog-row-{role}` etc.) are not Tailwind and may be dynamic.
- Body classes live in `index.html` → it must be in `@source`.

---

## 5. Fonts

### 5.1 V1 font block (globals.css L1–65, verbatim)

```css
@import url('https://fonts.googleapis.com/css2?family=Noto+Serif+JP:wght@300;400;700&display=swap');

@font-face {
  font-family: "MateSC";
  src: url("/resources/fonts/MateSC-Regular.ttf") format("truetype");
  font-display: swap;
}

@font-face {
  font-family: "Eurostile";
  src: url("/resources/fonts/Eurostile Condensed Bold.otf") format("opentype");
  font-display: swap;
}

@font-face {
  font-family: "DepartureMono";
  src: url("/resources/fonts/DepartureMono-Regular.otf") format("opentype");
  font-display: swap;
}

@font-face {
  font-family: "Liberation Sans";
  src: url("/resources/fonts/LiberationSans.ttf") format("truetype");
  font-display: swap;
}

@font-face {
  font-family: "MS Mincho";
  src: local("MS Mincho"), url("/resources/fonts/NotoSerifCJKsc-Regular.otf") format("opentype");
  font-display: swap;
}

@font-face {
  font-family: "Yu Mincho";
  src: local("Yu Mincho"), url("/resources/fonts/NotoSerifCJKtc-Regular.otf") format("opentype");
  font-display: swap;
}

@font-face {
  font-family: "Noto Serif JP";
  src: url("/resources/fonts/NotoSerifCJKsc-Regular.otf") format("opentype");
  font-weight: 400;
  font-display: swap;
  unicode-range: U+3000-303F, U+3040-309F, U+30A0-30FF, U+4E00-9FFF, U+FF00-FFEF;
}

@font-face {
  font-family: "Noto Serif JP";
  src: url("/resources/fonts/NotoSerif-Light.ttf") format("truetype");
  font-weight: 300;
  font-display: swap;
}

@font-face {
  font-family: "Noto Serif";
  src: url("/resources/fonts/NotoSerif-Regular.ttf") format("truetype");
  font-weight: 300;
  font-display: swap;
}

@font-face {
  font-family: "Noto Serif CJK KR";
  src: url("/resources/fonts/NotoSerifCJKkr-Regular.otf") format("opentype");
  font-display: swap;
}
```

### 5.2 Font face table

| # | Family | Source | Weight | unicode-range | File size (`public/resources/fonts/`) | Referenced by any font stack? |
|---|---|---|---|---|---|---|
| G | Noto Serif JP (Google) | `https://fonts.googleapis.com/css2?family=Noto+Serif+JP:wght@300;400;700&display=swap` (woff2 slices from fonts.gstatic.com) | 300, 400, 700 | Google's subsets | network | yes |
| 1 | MateSC | `/resources/fonts/MateSC-Regular.ttf` | (normal) | all | 69,364 B | yes |
| 2 | Eurostile | `/resources/fonts/Eurostile Condensed Bold.otf` (**filename contains spaces**) | (normal) | all | 25,608 B | yes |
| 3 | DepartureMono | `/resources/fonts/DepartureMono-Regular.otf` | (normal) | all | 84,480 B | yes |
| 4 | Liberation Sans | `/resources/fonts/LiberationSans.ttf` | (normal) | all | 350,200 B | yes |
| 5 | MS Mincho | `local("MS Mincho")`, then `/resources/fonts/NotoSerifCJKsc-Regular.otf` | (normal) | all | 24,543,080 B | yes |
| 6 | Yu Mincho | `local("Yu Mincho")`, then `/resources/fonts/NotoSerifCJKtc-Regular.otf` | (normal) | all | 24,541,904 B | **no** (never downloaded) |
| 7 | Noto Serif JP | `/resources/fonts/NotoSerifCJKsc-Regular.otf` | 400 | `U+3000-303F, U+3040-309F, U+30A0-30FF, U+4E00-9FFF, U+FF00-FFEF` | (same file as #5) | yes |
| 8 | Noto Serif JP | `/resources/fonts/NotoSerif-Light.ttf` (Latin/Greek/Cyrillic only) | 300 | all | 506,024 B | yes |
| 9 | Noto Serif | `/resources/fonts/NotoSerif-Regular.ttf` (declared weight 300!) | 300 | all | 616,196 B | **no** |
| 10 | Noto Serif CJK KR | `/resources/fonts/NotoSerifCJKkr-Regular.otf` | (normal) | all | 24,539,640 B | **no** |
| – | "Noto Serif CJK JP" | **no @font-face** — only a locally installed system font (common on Linux) | | | | used in 27 stacks |

Total on disk ≈ 73.6 MB. All `font-display: swap`.

Precedence subtleties (preserve by keeping order: Google sheet first, local faces after):
- For the same family and descriptors, the **later-defined** face wins. Local "Noto Serif JP" faces (#7 400/CJK range, #8 300/all) override Google's faces of the same weight. For a CJK character at weight 300 the browser first tries #8 (NotoSerif-Light, no CJK glyphs) and falls back to Google's 300 slices; at weight 400, #7 (24.5 MB NotoSerifCJKsc) wins for CJK ranges; weight 700 comes only from Google.
- Offline (desktop without network) the Google faces are missing → bold Latin text in "Noto Serif JP" stacks falls through to the next family ("MS Mincho" or "Noto Serif CJK JP"). V1 has the same behavior offline.
- `Eurostile` (file is Condensed **Bold**) and `MateSC` are declared with normal weight; selectors request `font-weight:700` (`.menu-label`, `.small-menu-label`, `.login-row label`, `.login-error`, `.mobile-form-error`) → browsers apply **synthetic bold** on top. Parity requires the same declarations (do not add `font-weight:700` to the faces).
- `html lang="en"` + the region-default glyphs of NotoSerifCJK**sc** = Simplified-Chinese glyph forms for the local CJK fallbacks. Keep `lang="en"`.

### 5.3 Font stack → selector map (app CSS)

| Stack | Selectors (globals.css line ranges; explicit weight in parentheses) |
|---|---|
| `"Noto Serif JP", "MS Mincho", serif` (base) | `.amadeus-root` L104–109 (w300, inherited by everything), `.cancel-streaming-button` L368–382, `.pc-cancel-hint` L388–398 (w300), `.mobile-topbar-status` L2083–2094 (w300), `.mobile-form-row` L2144–2152 (w700), `.mobile-primary-button` L2175–2188 (w700), `.mobile-loading-line` L2201–2206, `.mobile-settings-row` L2237–2245 (w300), `.mobile-settings-row input/select, .mobile-settings-textarea` L2253–2264, `.mobile-settings-freeapi-quota` L2273–2280 (w300), `.mobile-settings-actions button` L2288–2295, `.mobile-root .name-text` L2356–2366 (w700), `.mobile-root .dialogue-text` L2368–2386 (w300), `.mobile-root .chat-input` L2415–2419 (w300) |
| `"MS Mincho", "Noto Serif JP", serif` | `.status-clock` L898, `.status-label` L931, `.status-value` L943, `.status-credits` L966, `.status-achievement-label` L997, `.status-achievement-value` L1007, `.achievement-panel-title` L1039, `.mini-game-button` L1226, `.mini-game-title` L1253 |
| `"MS Mincho", serif` | `.login-row input` L244, `.change-log-entry .version` L1531 (w700), `.change-log-entry .date` L1542, `.config-title` L1670, `.mobile-form-row input` L2154 |
| `"Noto Serif CJK JP", "Noto Serif JP", serif` | `.achievement-panel-progress` L1050, `.achievement-row-title` L1132 (w700), `.achievement-row-desc` L1149, `.achievement-date` L1165, `.achievement-toast-title` L1201 (w700), `.achievement-toast-desc` L1209, `.mg-menu-title` L1289, `.mg-menu-desc` L1296, `.mg-start-button` L1305, `.mg-question-counter` L1333, `.mg-score-label` L1339, `.mg-question-box` L1345, `.mg-answer-text` L1399, `.mg-next-button` L1413, `.mg-result-title` L1438, `.mg-score-result` L1444, `.mg-result-comment` L1450 (italic), `.mg-retry-button, .mg-menu-back-button` L1466, `.mg-comment-bubble` L1492 |
| `"Noto Serif JP", "Noto Serif CJK JP", "MS Mincho", serif` | `.confirmation-dialog .choice` L1631 (w300), `.config-main` L1663, `.config-io-button` L1767, `.config-io-status` L1796, `.config-input-control` L1802 (w300), `.custom-select-trigger` L1844 (w300), `.custom-select-option` L1884 (w300), `.config-freeapi-quota` L1994 |
| `"Noto Serif JP", serif` | `.waiting-indicator` L443 (w300) |
| `"Eurostile", sans-serif` | `.menu-label` L630 (w700), `.small-menu-label` L651 (w700), `.mobile-menu-toggle` (media) L678, `.mobile-topbar-button` L2072, `.mobile-loading-title` L2194 (unused), `.mobile-settings-title` L2229 |
| `"Eurostile", "Noto Serif JP", sans-serif` | (media block 2) `.mobile-root .mini-game-button, .mobile-root .mobile-achievements-trigger-button` L2928–2947 (w700) |
| `"MateSC", serif` | `.login-row label` L226 (w700), `.login-error` L286 (w700), `.mobile-form-error` L2166 (w700) |
| `"DepartureMono", monospace` | `.boot-text` L305, `.mobile-settings-textarea` L2266 |
| `"Liberation Sans", sans-serif` | `.title-bar` L180 |
| `inherit` | `.config-sidebar button` L1692 (inherits `.config-main` stack) |

No inline `fontFamily` anywhere in the TSX. No RTL handling anywhere (Hebrew/Arabic UI languages render LTR layout with bidi text runs only).

### 5.4 Runtime MS Mincho loader (RealAmadeusApp.tsx L3932–3969) — must be ported

Runs **once, after the first render of `RealAmadeusApp`** (React `useEffect(..., [])`), both desktop and mobile branches.

Algorithm:
1. `cancelled = false`; `controller = new AbortController()`; `timeoutId = setTimeout(() => controller.abort(), 8000)`.
2. Async:
   1. If `document.fonts.check('16px "MS Mincho"')` → return (the `finally` still clears the timeout).
   2. `response = await fetch("https://raw.githubusercontent.com/vslabs/winfonts/master/msmincho.ttc", { signal })`; if `!response.ok` → return.
   3. `buffer = await response.arrayBuffer()`; if `cancelled` → return.
   4. `fontFace = new FontFace("MS Mincho", buffer)` (no descriptors → weight/style normal, full unicode range); `await fontFace.load()`; if `cancelled` → return.
   5. `document.fonts.add(fontFace)`.
   6. `catch` → silently ignore (comment: "Intentionally silent: CSS already falls back to Noto Serif JP."). `finally` → `clearTimeout(timeoutId)`.
3. Cleanup (unmount): `cancelled = true; controller.abort(); clearTimeout(timeoutId)`.

Behavioral notes:
- Because the CSS declares an "MS Mincho" `@font-face`, `check()` returns false until that face's status is `loaded`. The effect runs right after first paint, so on most machines (even Windows with MS Mincho installed, timing-dependent) the TTC download starts; on non-Windows machines the 24.5 MB `NotoSerifCJKsc` fallback may download in parallel.
- A JS-added face is appended after the CSS-connected faces → it **wins** for "MS Mincho" once loaded (TTC index 0 = MS Mincho).
- 8000 ms abort covers the whole fetch+body; failures are silent.
- Rust: `web_sys::FontFaceSet::check_with_text`/`check`, `web_sys::AbortController`, `gloo_net`/`web_sys::window().fetch_with_request`, `web_sys::FontFace::new_with_array_buffer`, `FontFace::load()` (Promise → `JsFuture`), `document.fonts().add(&face)`. Desktop: requires CSP `connect-src https://raw.githubusercontent.com`. Licensing of redistributing/fetching MS Mincho is a risk (§Open questions).

### 5.5 V2 fonts file (`styles/amadeus-fonts.css`) — ready to use

URLs made **relative to the stylesheet** (CSS `url()` resolves against the CSS file's URL, not the document/`<base>`). Valid when the processed CSS is emitted at the dist root next to `resources/` (Trunk's default for `rel="css"`): web `public_url=/amadeus/` → `/amadeus/resources/fonts/...`; desktop `public_url=/` → `/resources/fonts/...`. If the CSS is emitted into a subfolder, prefix `../`.

```css
@import url('https://fonts.googleapis.com/css2?family=Noto+Serif+JP:wght@300;400;700&display=swap');

@font-face {
  font-family: "MateSC";
  src: url("resources/fonts/MateSC-Regular.ttf") format("truetype");
  font-display: swap;
}

@font-face {
  font-family: "Eurostile";
  src: url("resources/fonts/Eurostile Condensed Bold.otf") format("opentype");
  font-display: swap;
}

@font-face {
  font-family: "DepartureMono";
  src: url("resources/fonts/DepartureMono-Regular.otf") format("opentype");
  font-display: swap;
}

@font-face {
  font-family: "Liberation Sans";
  src: url("resources/fonts/LiberationSans.ttf") format("truetype");
  font-display: swap;
}

@font-face {
  font-family: "MS Mincho";
  src: local("MS Mincho"), url("resources/fonts/NotoSerifCJKsc-Regular.otf") format("opentype");
  font-display: swap;
}

@font-face {
  font-family: "Yu Mincho";
  src: local("Yu Mincho"), url("resources/fonts/NotoSerifCJKtc-Regular.otf") format("opentype");
  font-display: swap;
}

@font-face {
  font-family: "Noto Serif JP";
  src: url("resources/fonts/NotoSerifCJKsc-Regular.otf") format("opentype");
  font-weight: 400;
  font-display: swap;
  unicode-range: U+3000-303F, U+3040-309F, U+30A0-30FF, U+4E00-9FFF, U+FF00-FFEF;
}

@font-face {
  font-family: "Noto Serif JP";
  src: url("resources/fonts/NotoSerif-Light.ttf") format("truetype");
  font-weight: 300;
  font-display: swap;
}

@font-face {
  font-family: "Noto Serif";
  src: url("resources/fonts/NotoSerif-Regular.ttf") format("truetype");
  font-weight: 300;
  font-display: swap;
}

@font-face {
  font-family: "Noto Serif CJK KR";
  src: url("resources/fonts/NotoSerifCJKkr-Regular.otf") format("opentype");
  font-display: swap;
}
```

Alternative (equivalent cascade): drop the `@import` line and put `<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Noto+Serif+JP:wght@300;400;700&display=swap">` in `index.html` **before** the `amadeus-fonts.css` link (Google faces must be defined before local ones). Do not concatenate this file into another file unless the `@import` stays first.

---

## 6. Absolute URLs that must be base-path aware (complete list)

Recommended scheme: Trunk `public_url` = `/amadeus/` (web) or `/` (desktop); `index.html` gets `<base data-trunk-public-url />`; all app-relative URLs are written without a leading slash (`resources/...`, `app/api/...`) or via a helper `asset("resources/...")` that prefixes `public_url`. Result: web resources at `/amadeus/resources/...`, desktop at `/resources/...`, API at `{public_url}app/api/...` (= `/amadeus/app/api/...` on web, matching V1). The server should additionally keep serving the legacy root `/resources/*` (the V1 Live2D runtime hard-codes `/resources/...`; old cached pages).

| Where | V1 URL (verbatim) | Count | V2 |
|---|---|---|---|
| CSS @import | `https://fonts.googleapis.com/css2?family=Noto+Serif+JP:wght@300;400;700&display=swap` | 1 | keep (external) |
| CSS @font-face | `/resources/fonts/MateSC-Regular.ttf` | 1 | `resources/fonts/…` (relative) |
| CSS @font-face | `/resources/fonts/Eurostile Condensed Bold.otf` | 1 | same; space in filename → server must percent-decode `%20` (tower-http `ServeDir` and Tauri asset protocol do) |
| CSS @font-face | `/resources/fonts/DepartureMono-Regular.otf` | 1 | relative |
| CSS @font-face | `/resources/fonts/LiberationSans.ttf` | 1 | relative |
| CSS @font-face | `/resources/fonts/NotoSerifCJKsc-Regular.otf` | 2 (MS Mincho, Noto Serif JP 400) | relative |
| CSS @font-face | `/resources/fonts/NotoSerifCJKtc-Regular.otf` | 1 | relative |
| CSS @font-face | `/resources/fonts/NotoSerif-Light.ttf` | 1 | relative |
| CSS @font-face | `/resources/fonts/NotoSerif-Regular.ttf` | 1 | relative |
| CSS @font-face | `/resources/fonts/NotoSerifCJKkr-Regular.otf` | 1 | relative |
| TSX `<img>` | `/resources/images/amadeus_logo_v3.png` | 4 (MobileLogin, MobileLoading, LoginPanel, LoadingPanel) | `asset()` |
| TSX `<img>` | `/resources/images/amadeus_login_button_v2.png` | 1 | `asset()` |
| TSX `<img>` | `/resources/images/Amadeus_BG.png` | 7 (system-bg + 6 overlay-bg) | `asset()` |
| TSX `<img>` | `/resources/images/RealAmadeus_Menu_BG_v3.jpg` | 1 | `asset()` |
| TSX `<img>` | `/resources/images/Menu.png` | 1 | `asset()` |
| TSX `<img>` | `` `/resources/images/${icon}.png` `` and `` `/resources/images/${icon}Selected.png` ``, icon ∈ {Backlog, Config, Status, ChangeLog, Help} | 10 files | `asset()` |
| TSX `<img>` | `/resources/images/DownAllow.png` | 1 | `asset()` |
| Live2D | `/resources/live2d/live2dcubismcore.min.js`, `/resources/live2d/reama-cubism-runtime.js?v=19` (Live2DCanvas.tsx); runtime constants `"/resources/models/AmadeusKurisu5.0/"`, `"/resources/live2d/shaders/"` (reama-cubism-runtime.js L2676–2677, L11548) | – | live2d spec; make the prefix configurable |
| Metadata | icons `/amadeus_logo.png` (×3), manifest `/amadeus/app/manifest.webmanifest`; manifest `start_url: "/amadeus/app"`, icon `/favicon.ico`; file-convention `app/amadeus/app/icon.png` | – | app-shell spec |
| Landing/docs | `/amadeus_logo.png`, `/amadeus_login.png`, links `/amadeus`, `/amadeus/app`, `/amadeus/docs...`, `/amadeus/download`, `/amadeus/changelog`, `/` | – | router links |
| API (FYI) | `/amadeus/app/api/chat`, `/amadeus/app/api/chat/stream`, `/amadeus/app/api/chat/quota`, `/amadeus/app/api/stt/health`, `/amadeus/app/api/stt/transcribe`, `/amadeus/app/api/tts/health`, `/amadeus/app/api/tts/synthesize`, `/amadeus/app/api/rag/embed`, `/amadeus/app/api/rag/health` | – | transport spec |
| External | `https://raw.githubusercontent.com/vslabs/winfonts/master/msmincho.ttc`, `https://dsuv.elvelt.com/amadeus` (shutdown redirect), GitHub release links | – | keep |

Images present in `public/resources/images/`: Amadeus_BG.png, amadeus_login_button_v2.png, amadeus_logo_v3.png, app_icon.ico (unused by web), Backlog(.png|Selected.png), ChangeLog(...), Config(...), DownAllow.png, Help(...), Menu.png, RealAmadeus_Menu_BG_v3.jpg, Status(...). No `url()` image references exist in any CSS.

Caching (from `next.config.ts`; last matching rule wins): `/resources/:path*` → `Cache-Control: public, max-age=86400, stale-while-revalidate=604800`, `Pragma: public`, `Expires: 86400`, `Surrogate-Control: public, max-age=86400`, `CDN-Cache-Control: public, max-age=86400`; everything else (incl. CSS) → `no-store, no-cache, must-revalidate, proxy-revalidate, max-age=0`, `Pragma: no-cache`, `Expires: 0`, `Surrogate-Control: no-store`, `CDN-Cache-Control: no-store`. Fonts (24.5 MB files) depend on the 1-day cache; apply the same headers to the new `{base}resources/*` path. MIME: `.otf`→`font/otf`, `.ttf`→`font/ttf`.

---

## 7. DOM ↔ CSS contract (structure, classes, conditions, inline styles)

Notation: `tag.class1.class2[.cond]` means class `cond` is present only under the stated condition. Literal V1 class strings matter only as token sets (trailing spaces / `class=""` are harmless). `t(key, default, lang)` = localization helper; `lang==1` = English UI. All texts are verbatim.

### 7.1 Desktop root (`RealAmadeusApp`, `!isMobileViewport`)

```
div.real-amadeus-root                         onContextMenu: preventDefault(); if AppSettings.getInt("Config_RightClickMenu",1)==1 → toggleMenuPanel()
├─ [appState=="chat"] button.mobile-menu-toggle[type=button]
│     text: isTabActive ? t("close", lang==1?"CLOSE":"閉じる") : "MENU"      (display:none unless media block 1 matches)
└─ div.window-frame.{fullscreen|windowed}      Config_ScreenMode (default 1): 0→"fullscreen", else "windowed"
      style="width: {1920*s}px; height: {1080*s}px"
   └─ div.viewport   style="transform: scale({s})"          (transform-origin: top left from CSS)
      ├─ BackLogPanel  (wrapper z-index 3000)               ← FIRST child of .viewport
      ├─ div.system-panel  style="transform: translate({150*p}px, {-12*p}px) scale({1 + (0.6 - 1)*p})"   p = isTabActive ? 1 : 0
      │  ├─ img.menu-wallpaper   src=…/RealAmadeus_Menu_BG_v3.jpg alt=""
      │  ├─ div.title-bar > span "Amadeus System"
      │  ├─ img.system-bg        src=…/Amadeus_BG.png alt=""
      │  ├─ [appState=="login"] LoginPanel
      │  ├─ [appState=="boot"]  LoadingPanel
      │  └─ [appState=="chat"]  ChatPanel(menuPanelOpen=isTabActive) + [autoMode] div.auto-text "Auto"
      ├─ div.menu-panel   (inline style §7.7)
      ├─ ConfigPanel       (wrapper z 2000)   visible = activeOverlay=="config"
      ├─ StatusPanel       (wrapper z 2000)   visible = activeOverlay=="status"
      ├─ AchievementPanel  (wrapper z 2500)   visible = achievementsOpen
      ├─ ChangeLogPanel    (wrapper z 2000)   visible = activeOverlay=="changeLog"
      ├─ HelpPanel         (wrapper z 2000)   visible = activeOverlay=="help"
      ├─ ConfirmationDialog(wrapper z 3000)   visible = activeOverlay=="confirmLogout"
      └─ ConfirmationDialog(wrapper z 3000)   visible = activeOverlay=="confirmShutdown"
```

Scaling math (L3979–3989, L4166–4170): on mount and on `window.resize`: `scale = min(innerWidth/1920, innerHeight/1080)`, `dimensions = {width: 1920*scale, height: 1080*scale, scale}`; initial state before the effect `{1920, 1080, 1}`. `outerWidth = "${dimensions.width}px"`, `uiScale = dimensions.scale`. Numbers are stringified with JS `Number→String` (shortest round-trip, e.g. `0.7111111111111111`). The `Config_Resolution` setting is read (`resolveResolution`) but **never used for layout** (dead in web). The flex `.real-amadeus-root` centers the frame. With `.windowed` the 1 px `#666` border sits inside the frame box (border-box), so the scaled viewport overflows by 2 px and the right/bottom 2 px are clipped by `overflow:hidden` (V1 quirk).

`system-panel` transform values: p=0 → `translate(0px, 0px) scale(1)` (JS prints `-12*0` as `0`; Rust prints `-0` → normalize), p=1 → `translate(150px, -12px) scale(0.6)`. CSS `transition: transform 500ms linear; transform-origin: center`. `.menu-wallpaper` (`scale(1.666667)`, left -250, top 20, z 0) is inside the panel so that at panel scale 0.6 it appears at ≈1:1 behind the shrunken panel; at scale 1 it is hidden under `.system-bg` and clipped by `.window-frame{overflow:hidden}`.

Menu open sequence: `isTabActive=true` (Tab key, right-click, MENU toggle) → system panel animates 500 ms; after **450 ms** `setMenuOpen(true)` → `.menu-panel` opacity 0→1 over 0.3 s. Closing sets both false immediately. When `isTabActive` becomes true, a focused INPUT/TEXTAREA is blurred.

### 7.2 Mobile root (`isMobileViewport`)

```
div.real-amadeus-root.mobile-root            onContextMenu: preventDefault()
├─ [appState=="chat"] div.mobile-topbar
│   ├─ div.mobile-topbar-buttons
│   │   ├─ button.mobile-topbar-button[type=button]  t("settings", lang==1?"SETTINGS":"設定")   → mobileSettingsOpen=true
│   │   └─ button.mobile-topbar-button[type=button]  t("backlog", lang==1?"LOG":"ログ")         → activeOverlay="backlog"
│   └─ MarqueeStatus: div.mobile-topbar-status[.marquee] > span.mobile-topbar-status-inner  text
├─ div.mobile-stage
│   ├─ [login] MobileLoginPanel
│   ├─ [boot]  MobileLoadingPanel
│   └─ [chat]  ChatPanel(menuPanelOpen=mobileSettingsOpen, setActiveOverlay)
├─ [mobileSettingsOpen] MobileSettingsPanel       (conditionally mounted, no fade)
├─ BackLogPanel      (wrapper z 3000)  visible = activeOverlay=="backlog"
└─ AchievementPanel  (wrapper z 2500)  visible = achievementsOpen || activeOverlay=="achievements"
```
Not present on mobile: title bar, system-bg, menu panel, config/status/changelog/help/confirm panels, `.auto-text`, `.mobile-menu-toggle`.

MarqueeStatus (L3873–3902): `text = provider && model ? "{provider} / {model}" : "---"`. On mount and whenever `text` changes: `checkOverflow()` immediately, again after `setTimeout(100)`, and on every `window.resize`; `isOverflowing = inner.offsetWidth > container.clientWidth`. When true the container gets `marquee` → inner span `padding-left:100%` + `animation: mobile-topbar-marquee 12s linear infinite` (translateX 0 → -100%). Quirk: once marquee is on, `offsetWidth` includes the 100% padding, so it never turns off on resize until `text` changes.

### 7.3 LoginPanel (desktop)

```
div.login-panel
├─ img.login-logo  src=…/amadeus_logo_v3.png alt="logo"
├─ div.login-row.user-id-row  > label "USER ID" + input (autoFocus; Enter → attemptLogin)
├─ div.login-row.password-row > label "PASSWORD" + input[type=password] (Enter → attemptLogin)
├─ button.login-button[type=button] > img src=…/amadeus_login_button_v2.png alt="login"
└─ [errorVisible] div.login-error "ACCESS DENIED"
```
`<label>`s have no `for`; clicking them does not focus. Geometry (1920×1080 stage): logo 700×700 centered at (960, 358); rows: label right edge x=641, input x 660–1260 (600×50), user-id row top 646.5, password row top 716.5; button 65×65 at (1261.5, 709); error centered ~ (740, 811.5).

### 7.4 LoadingPanel (desktop)

```
div.loading-panel
├─ [!showLogo] pre.boot-text  {displayText.slice(0, visibleChars)}     (red #ff0000, DepartureMono 18px, pre-wrap)
└─ [showLogo]  img.boot-logo src=…/amadeus_logo_v3.png alt="logo"      (900×900 centered)
```

### 7.5 ChatPanel (both branches)

```
div.chat-panel-root                     (desktop: inset 35px 0 0 0 + 500 ms fade-in on mount; mobile: inset safe+56px 0 0 0, animation none)
├─ div.live2d-shift[.live2d-shifted]    live2d-shifted ⇔ miniGameOpen   (class string "live2d-shift" + (open ? " live2d-shifted" : ""))
│   └─ div.live2d-host  (Live2DCanvas) > canvas
├─ [chatState != "inputReady"] div.dialogue-panel
│   ├─ button.dialogue-advance-hitbox[type=button][aria-label=t("advance_dialogue", lang==1?"Advance dialogue":"会話を進める")]   (empty, inset 0)
│   ├─ div.name-text      t("amadeus_kurisu","アマデウス紅莉栖")
│   ├─ div.dialogue-text  {dialogueText}
│   ├─ div.waiting-indicator  {(chatState=="waitingAPI"||chatState=="waitForAdvance"||waitingForInput) ? waitingIndicator : ""}
│   └─ [chatState=="waitingAPI"]
│        isMobileViewport ? button.cancel-streaming-button  t("cancel_button","キャンセル")
│                         : div.pc-cancel-hint              t("cancel_hint","CTRL+Cでキャンセル")
├─ [inputReady && !miniGameOpen] div.input-panel[.has-audio-input]       has-audio-input ⇔ showAudioInput (initial Config_AudioInputEnabled==1)
│   ├─ input.chat-input[.sleeping-input]   sleeping-input ⇔ currentEmotionTag=="SLEEPING"
│   │     placeholder = SLEEPING ? t("sleeping_placeholder","Zzz... (キー入力やマウス移動で起こす)") : t("enter_message","メッセージを入力")
│   ├─ div.chat-input-actions
│   │   ├─ [showAudioInput] button.chat-mic-button[.recording][type=button][aria-label=t("speech_input", lang==1?"Speech input":"音声入力")]
│   │   │     text: sttRecording ? "■" : "🎤"
│   │   └─ button.chat-send-button[type=button][disabled = !chatInput.trim()]   t("send", lang==1?"SEND":"送信")
│   ├─ [sttRecording] div.chat-mic-level > span style="width: {Math.round(sttRecordingLevel*100)}%"
│   └─ [sttStatusText] div.chat-stt-hint {sttStatusText}
├─ [inputReady && !miniGameOpen] button.mini-game-button[type=button] "MINI GAME"
├─ [inputReady && !miniGameOpen] button.mobile-achievements-trigger-button[type=button] "ACHIEVEMENTS"   (display:none except mobile media block 2)
├─ MiniGamePanel (always mounted, §7.8)
└─ div.achievement-toast[.visible][role=status]
     ├─ div.achievement-toast-title "★ {title}"      (note: "★" + space)
     └─ div.achievement-toast-desc  {description}
```
Toast: `visible` set on `AchievementManager.achievementUnlocked`; removed after **4000 ms** (timer restarts whenever the toast state object changes); CSS fades opacity over 400 ms linear; `pointer-events:none`.

`.dialogue-panel` itself has `pointer-events:none`; only `.dialogue-advance-hitbox` and `.cancel-streaming-button` receive pointer events (L363–366). Desktop text layout inside the 300 px panel: `.dialogue-text` box 1280×200 at top 38 px, centered; `.name-text` 300×50 at top 237 px (below the text); `.waiting-indicator` right 80/bottom 35. Desktop `.dialogue-text` has no overflow rule (visible), mobile has `overflow-y:auto` with the custom scrollbar (white track, orange thumb, 10 px).

#### 7.5.1 Desktop input panel

`left:50px; right:50px; bottom:25px; height:60px; padding:5px 65px 5px 16px` (border-box). `.chat-send-button` is `display:none` unless media block 1 matches (then 96×44 and input padding becomes `5px 170px 5px 10px`). Mic button 44×44 at right 10/top 8.

#### 7.5.2 Live2D canvas contract (amadeus-live2d ↔ CSS)

- V1 runtime (`public/resources/live2d/reama-cubism-runtime.js` L18196–18202) creates `<canvas style="display:block;width:100%;height:100%;pointer-events:none">` inside `.live2d-host`, sizes the backing store to `clientWidth*devicePixelRatio × clientHeight*devicePixelRatio` (`resizeCanvas`, L17628–17633; `CanvasSize = "auto"`, L2666).
- Live2DCanvas.tsx (L104–116) then sets inline `pointerEvents="auto"`, `width="100%"`, `height="100%"` (retry once after 100 ms if the canvas does not exist yet). On unmount: `WEBGL_lose_context.loseContext()` and `host.innerHTML=""`.
- CSS `.live2d-host canvas{position:absolute;left:50%;top:50%;width:3840px;height:2160px;transform:translate(-50%,-50%);pointer-events:auto}` → width/height are overridden by the inline 100%; the canvas exactly covers the host (desktop host = 1920×1045 stage px).
- Mobile: `.mobile-root .live2d-host{position:fixed;inset:0;z-index:0}`; `.mobile-root .live2d-host canvas{position:absolute;inset:0;width:100%!important;height:100%!important;object-fit:contain;object-position:center center;transform:none;pointer-events:auto}`.
- V2 requirement: the Rust-created canvas must carry inline `display:block; width:100%; height:100%; pointer-events:auto` (final V1 state). Without the inline size the model renders at 3840×2160 CSS px.
- Mobile quirk: when the minigame opens, `.live2d-shift` gains a `transform` (media block 2) → it becomes the containing block of the `position:fixed` host, so the host shrinks from full viewport to the chat area (viewport minus top bar) and is then translated/scaled; `object-fit:contain` keeps the backing store's aspect. Removing `.live2d-shifted` resets `transform-origin` to center instantly while the transform transitions back.

### 7.6 Auto text
`[appState=="chat" && autoMode] div.auto-text "Auto"` — child of `.system-panel` (desktop only), 200×50 at x 1660, y 87.5, 70 px `#ff9900`, z 7.

### 7.7 Menu panel (desktop)

```
div.menu-panel  style="opacity: {menuOpen?1:0}; pointer-events: {menuOpen?'auto':'none'}; transition: opacity 0.3s ease-in-out;
                       transform: scaleX({dimensions.width/(1920*uiScale)}) scaleY({dimensions.height/(1080*uiScale)}); transform-origin: top left"
├─ img.menu-bg src=…/Menu.png alt=""
└─ div.menu-content
    ├─ MenuItem ×5   (button.menu-icon[.selected] + button.menu-label[.selected])
    └─ SmallMenuLabel ×4 (button.small-menu-label[.selected])
```
The scale factors are always exactly 1 (both terms are `1920*scale`), but the transform still creates a stacking context — keep it.

MenuItem: `button.menu-icon[.selected][type=button] style="left:{x-45}px; top:{y-45}px"` > `img.menu-icon-normal src=…/{Icon}.png alt=""` + `img.menu-icon-selected src=…/{Icon}Selected.png alt=""`; `button.menu-label[.selected][type=button] style="left:{lx-100}px; top:{ly-25}px"` with text (newline rendered by `white-space:pre-line`). SmallMenuLabel: `button.small-menu-label[.selected][type=button] style="left:{x-100}px; top:{y-25}px"`.

| Index | Element | Label (verbatim) | icon center (x,y) | label center (x,y) | resulting left/top |
|---|---|---|---|---|---|
| 0 | MenuItem Backlog | `BACK\nLOG` | (270,146) | (441,147) | icon 225/101, label 341/122 |
| 1 | MenuItem Config | `CONFIG` | (270,320) | (441,321) | 225/275, 341/296 |
| 2 | MenuItem Status | `STATUS` | (270,494) | (441,495) | 225/449, 341/470 |
| 4 | MenuItem ChangeLog | `CHANGE\nLOG` | (270,674) | (441,669) | 225/629, 341/644 |
| 6 | MenuItem Help | `HELP` | (270,848) | (441,843) | 225/803, 341/818 |
| 3 | SmallMenuLabel | screenMode==0 ? `FULL\nSCREEN` : `WINDOW` | – | (135,584) | 35/559 |
| 5 | SmallMenuLabel | `LOGOUT` | – | (135,758) | 35/733 |
| 7 | SmallMenuLabel | `SHUT\nDOWN` | – | (135,950) | 35/925 |
| 8 | SmallMenuLabel | `CLOSE\nMENU` | – | (405,950) | 305/925 |

Selected visuals: `.menu-icon.selected` swaps to the Selected image (opacity) + `drop-shadow(0 0 6px rgba(255,255,255,0.6)) brightness(1.2)`; selected labels become #fff with `drop-shadow(0 0 4px …)`.

### 7.8 MiniGamePanel (always mounted inside ChatPanel)

```
div.mini-game-panel  style="opacity: {visible?1:0}; pointer-events: {visible?'auto':'none'}; transition: opacity 0.25s ease-in-out;
                            position: {isMobile?'fixed':'absolute'}; top: {isMobile?'auto':'0px'}; bottom: 0px; left: 0px; right: 0px;
                            z-index: 50; height: {isMobile?'46%':'100%'}"           (isMobile = useIsMobileViewport())
├─ div.mini-game-title "MINI GAME"
├─ div.mini-game-content
│   ├─ [gameState=="menu"] div.mini-game-menu
│   │     div.mg-menu-title {mg_menu_title} ; div.mg-menu-desc {mg_menu_desc with "%1"→TOTAL_QUESTIONS} ; button.mg-start-button {mg_start}
│   ├─ [gameState=="playing" && activeQuestion] div.mini-game-playing
│   │     div.mg-header > span.mg-question-counter "Q{current+1} / {TOTAL}" + span.mg-score-label "{mg_score}: {score}"
│   │     div.mg-question-box {question}
│   │     div.mg-answers > button.mg-answer[.revealed][.correct|.wrong][type=button][disabled=revealed] > span.mg-answer-text {answer}
│   │         class = "mg-answer" + (revealed?" revealed":"") + (revealed ? (i==correct?" correct": i==selected?" wrong":"") : "")
│   │     div.mg-spacer
│   │     [revealed] button.mg-next-button  (current+1>=TOTAL ? mg_show_result : mg_next)
│   └─ [gameState=="result"] div.mini-game-result
│         div.mg-result-title ; div.mg-score-result (mg_score_result "%1"→score, "%2"→TOTAL) ; div.mg-result-comment
│         div.mg-result-buttons > button.mg-retry-button + button.mg-menu-back-button
├─ [revealed && comment!=""] div.mg-comment-bubble {comment}
└─ button.overlay-close-button.mg-close-button[type=button]  t("close", lang==1?"Close":"閉じる")
```
Desktop: transparent full-stage layer; quiz occupies left half (`.mini-game-content` left 60, top 140, bottom 80, width 50%); Live2D shifts right `translateX(28%)` over 450 ms `cubic-bezier(0.645,0.045,0.355,1)`; bubble at right 120/bottom 180, 600 px wide.
Mobile (media block 2): `!important` rules override the inline style → fixed bottom sheet, `height:46%`, `z-index:3000`, top border 2 px `#ff9900`, radius 16 16 0 0, blur 12 px; comment bubble floats **125 px above** the sheet (`top:-125px`, z 3100); close button becomes a 28 px pill at top-right; Live2D shifts `translateY(-14dvh) scale(0.85)` with `transform-origin: top center`.

### 7.9 Overlay wrappers (exact inline styles; always mounted)

| Panel | Wrapper class | Inline style (React serialization order) |
|---|---|---|
| BackLogPanel | `backlog-panel-wrapper` | `opacity: 1|0; pointer-events: auto|none; transition: opacity 0.3s ease-in-out; position: absolute; top: 0px; left: 0px; right: 0px; bottom: 0px; width: 100%; height: 100%; overflow: hidden; z-index: 3000;` |
| StatusPanel | (none) | `opacity; pointer-events; transition: opacity 0.3s ease-in-out; position: absolute; top/left/right/bottom: 0px; z-index: 2000;` |
| AchievementPanel | (none) | same but `transition: opacity 0.25s ease-in-out; z-index: 2500;` |
| ChangeLogPanel | (none) | 0.3s, z 2000 |
| HelpPanel | (none) | 0.3s, z 2000 |
| ConfirmationDialog (×2) | (none) | 0.3s, z 3000 |
| ConfigPanel | (none) | 0.3s, z 2000 |
| MiniGamePanel | `mini-game-panel` | see §7.8 |
| Menu panel | `menu-panel` | see §7.7 |

Because the wrappers are positioned with a z-index, each is its own stacking context; the CSS z-indexes of the inner `.overlay-panel` (110), `.config-panel` (120), `.dialog-overlay` (130) only order things inside one wrapper. **Keep the wrappers** — removing them changes the global stacking order.

### 7.10 BackLogPanel

```
div.backlog-panel-wrapper
└─ div.dialog-overlay.backlog-panel
   ├─ img.overlay-bg src=…/Amadeus_BG.png alt=""
   ├─ div.overlay-header > h2 "BACKLOG" + div.line
   ├─ div.backlog-content      (scrollTop = scrollHeight whenever entries change while visible, and on becoming visible)
   │   └─ div.backlog-row.backlog-row-{role}  ×N       role = lower-cased role string (normally "user" | "assistant"; imported files may carry any role, e.g. "me")
   │       ├─ div.backlog-name style="color: {nameColor}"
   │       │     user|me → t("you", lang==1?"You":"あなた"); assistant|kurisu|amadeus → t("amadeus_kurisu", lang==1?"Amadeus Kurisu":"アマデウス紅莉栖"); else role.toUpperCase()
   │       └─ div.backlog-message {message}
   └─ button.overlay-close-button[type=button]  t("close", lang==1?"Close":"閉じる")
```
nameColor: user/me `#66CCFF`, assistant/kurisu/amadeus `#FF6666`, else `#808080`. Backspace closes while visible. Mobile styling targets `.backlog-row-user` / `.backlog-row-me` (right orange-yellow bar).

### 7.11 StatusPanel

```
div(wrapper) > div.overlay-panel.status-panel
├─ img.overlay-bg
├─ div.overlay-header > h2 "SYSTEM STATUS" + div.line
├─ div.status-clock  "YYYY/MM/DD HH:MM:SS" (zero-padded local time; updated every 1000 ms while visible; "" initially)
├─ div.status-grid > StatusRow ×6 = div.status-row > div.status-label {label} + div.status-value (innerHTML = value)
├─ button.status-achievement-row[type=button]  (→ open achievements)
│     span.status-achievement-label "ACHIEVEMENTS"
│     span.status-achievement-value "★ {unlocked} / {total}"   (before mount: "★ 0 / 28")
│     span.status-achievement-underline[aria-hidden=true]
├─ div.status-credits   lines separated by <br>:
│     Provided by ELVELT/Real Amadeus Project
│     Developed by DSUV
│     Designed by DSUV/Amane
│     Translated with assistance by Dezpot
│     This project is a derivative work of Steins;Gate 0
│     Real Amadeus v1.4N(Build 20260801_001)
└─ button.overlay-close-button
```
Rows (label → value, value is **HTML**):
- `SYSTEM_VERSION` → `Real Amadeus v1.4N(Build 20260801_001)`
- `LIVE2D_MODEL` → `Live2DKurisu v1.0`
- `OPERATOR` → isLoggedIn ? (MemoryManager.userName || "Salieri") : `---`   (userName is injected as HTML — V1 quirk)
- `NETWORK` → `<span class='good'>ONLINE</span>` | `<span class='bad'>OFFLINE</span>` (navigator.onLine polled every 5000 ms while visible; initial true)
- `LLM_MODEL` → provider && model ? `{provider} / {model}` : `---`
- `AVERAGE_LATENCY` → latencyMs < 0 ? `--- ms` : `Ping: <span class='{bad if >3000, warn if >1000, else good}'>{Math.round(ms)} ms</span>`
CSS colors: `.status-value .good #44ff44`, `.warn #ffff44`, `.bad #ff4444`. Backspace closes unless the achievements panel is open.

### 7.12 AchievementPanel

```
div(wrapper z2500, 0.25s) > div.overlay-panel.achievement-panel
├─ img.overlay-bg
├─ div.achievement-panel-title "ACHIEVEMENTS"
├─ div.achievement-panel-progress "{unlocked} / {total}"      (before mount "0 / 0")
├─ div.achievement-list > div.achievement-row.{unlocked|locked}  ×N
│     div.achievement-icon.{unlocked|locked}  unlocked ? "★" : (secret ? "?" : "☆")
│     div.achievement-text > div.achievement-row-title {title} + div.achievement-row-desc {description}
│     [unlocked && unlockedAt != ""] div.achievement-date {unlockedAt}
└─ button.overlay-close-button
```
Backspace or Escape closes while visible. Scrollbar hidden (`scrollbar-width:none`, `::-webkit-scrollbar{display:none}`).

### 7.13 ChangeLogPanel / HelpPanel

```
div(wrapper) > div.overlay-panel.changelog-panel
├─ img.overlay-bg ; div.overlay-header > h2 "CHANGE LOG" + div.line
├─ div.change-log-content > div.change-log-entry ×N
│     div.change-log-head > div.version {version} + div.date {date}
│     div.line.thin
│     ul > li ×  t(entry.key, "", lang).split("\n")
└─ button.overlay-close-button

div(wrapper) > div.overlay-panel.help-panel
├─ img.overlay-bg ; div.overlay-header > h2 "HELP" + div.line
├─ div.help-content > div.help-entry ×5 > div.title + div.line.thin + div.description
└─ button.overlay-close-button
```
Help entries (title / description):
1. `t("help_right_click_title", lang==1?"Tab / Right Click":"Tab / 右クリック")` / `t("help_toggle_menu", lang==1?"Open/Close Menu":"メニュー開閉")`
2. `Backspace` / `t("help_save_close", lang==1?"Apply and Close":"保存して閉じる")`
3. `t("help_wasd", lang==1?"WASD / Arrows":"WASD / ↑←↓→")` / `t("help_select_item", lang==1?"Navigate Items":"項目選択")`
4. `Enter` / `t("help_confirm_advance", lang==1?"Select / Advance Conv.":"決定 / 会話を進める")`
5. `Ctrl+C` / `t("help_cancel_chat", lang==1?"Cancel Conversation":"会話をキャンセル")`

### 7.14 ConfirmationDialog

```
div(wrapper z3000) > div.dialog-overlay > div.confirmation-dialog
├─ div.message {message}       (logout: t("logout_confirm", lang==1?"Do you want to log out?":"ログアウトしますか？"); shutdown: t("exit_confirm", lang==1?"Do you want to exit Real Amadeus?":"リアルアマデウスを終了しますか？"))
├─ button.choice.no[.selected][type=button]   t("no", lang==1?"No":"いいえ")    mouseenter→selected=0, click→cancel
└─ button.choice.yes[.selected][type=button]  t("yes", lang==1?"Yes":"はい")    mouseenter→selected=1, click→confirm
```
`selectedIndex` initial 1; reset to 1 via `setTimeout(0)` each time it becomes visible. Keys (while visible): ArrowLeft/`a` → 0, ArrowRight/`d` → 1, Backspace → cancel, Enter → selected==1 ? confirm : cancel.

### 7.15 ConfigPanel (desktop)

```
div(wrapper z2000) > div.overlay-panel.config-panel
├─ img.overlay-bg
├─ div.config-main
│   ├─ div.config-title "CONFIG"
│   ├─ div.config-sidebar > button[type=button][class="active"|""] ×5
│   │     labels tr(ja,en,key): ("システム","System","category_system"), ("テキスト","Text","category_text"), ("サウンド","Sound","category_sound"),
│   │                           ("グラフィック","Graphics","category_graphics"), ("API","API","category_api")
│   └─ div.config-content
│       ├─ div.config-section-title {label of active category}
│       ├─ div.line.thin
│       └─ div.config-form (categories 0–3) | div.config-form.dense (category 4 = API)
└─ div.config-footer > button.cancel[type=button] tr("キャンセル","Cancel","cancel") + button.apply[type=button] tr("適用","Apply","apply")
```
Row components (exact DOM):
- **ConfigSelectRow**: `div.config-row style="display: flex; align-items: center;"` > `span.config-label {label}` + `span.config-select-shell` > `div.custom-select-trigger[.open]` (click toggles; text `options[value] || ""`) + `[open] div.custom-select-dropdown > div.custom-select-option ×` (skip empty option strings; **skip index 4 unless `Config_VertexEnabled==1` — applied to every select, e.g. it also hides "Español" in Display Language and "한국어" in Conversation Language; V1 quirk**) + `span.config-select-arrow[aria-hidden=true] style="pointer-events: none;" > img src=…/DownAllow.png alt=""`. Closes on document `mousedown` outside the shell.
- **ConfigInputRow**: `label.config-row > span.config-label + input.config-input-control[type=text|password][disabled?]`.
- **ConfigCheckRow**: `label.config-row.checkbox-row > span.config-label + span.config-checkbox-shell > input[type=checkbox]` (CSS draws a 40×40 #333 box with a 24×24 #ff9900 inner square when checked).
- **ConfigSliderRow**: `label.config-row.slider-row > span.config-label + div.slider-wrap > input[type=range][min][max][step][value] style="--fill: {((value-min)/(max-min))*100}%"` + `div.slider-value {asPercent ? Math.round(value/max*100)+"%" : value.toFixed(1)+(suffix??"")}`. `--fill` drives the WebKit track gradient `linear-gradient(to right, #ff9900 var(--fill,0%), #333 var(--fill,0%))`; Firefox uses `::-moz-range-progress`.
- IO rows (Text category): `div.config-io-separator`; `div.config-row > span.config-label + button.config-io-button.export`; `div.config-row > span.config-label + button.config-io-button.import + input[type=file][accept=".json"].config-import-input` (hidden); `[message] div.config-io-status style="color: #00CC00|#FF4444"`.
- `div.config-note` texts; FreeApiQuotaDisplay with `className="config-freeapi-quota"`.
Focus quirk: `select:focus, .config-input-control:focus { border-color:#333 !important }` (L88–95) beats `.config-input-control:focus{border-color:#ff9900}` (L1814–1817) → focus border stays #333.

### 7.16 FreeApiQuotaDisplay (desktop config + mobile sheet)

```
div.{config-freeapi-quota | mobile-settings-freeapi-quota}
├─ div > span style="color: {exhausted?#FF4444:#FF9900}" tr("FreeAPIクォータ","FreeAPI Quota","freeapi_quota_label")
│        span style="color: {exhausted?#FF4444:#FFFFFF}; margin-left: 16px"  tr("%1 / %2 回使用","%1 / %2 requests used","freeapi_quota_usage") with %1=count.toLocaleString(), %2=max.toLocaleString()
├─ div style="color: {numberColor}"  tr("残り %1 回","%1 remaining","freeapi_quota_remaining") with %1=remaining.toLocaleString()
└─ [remaining==0] div style="color: #FF4444" tr("全体リクエスト上限（65,000回）に達しました。サービスを停止しています。","Global request limit (65,000) reached. Service suspended.","freeapi_quota_exhausted")
```
`toLocaleString()` uses the browser default locale (use `js_sys::Intl::NumberFormat` with default locale in Rust, not a fixed format).

### 7.17 Mobile panels

```
div.mobile-login-panel                                  (position:fixed; inset 0; #0a0500; flex column centered; touch-action none)
├─ img.mobile-login-logo src=…/amadeus_logo_v3.png alt=""
├─ label.mobile-form-row > span "USER ID"  + input
├─ label.mobile-form-row > span "PASSWORD" + input[type=password]   (Enter → submit)
├─ [error] div.mobile-form-error {error}
└─ button.mobile-primary-button[type=button] "LOGIN"

div.mobile-loading-panel
├─ img.mobile-loading-logo src=…/amadeus_logo_v3.png alt="Amadeus System"     ← NO CSS rule for this class (preflight sizing only)
└─ div.mobile-loading-line  t("booting", lang==1?"Booting":"起動中") + "." × dots   (dots 1→2→3→1 every 300 ms; finish after 1500 ms, or immediately if Config_SkipLoading==1)

div.mobile-settings-overlay > div.mobile-settings-sheet
├─ div.mobile-settings-title tr("設定","SETTINGS","setting_title")
├─ label.mobile-settings-row > span tr("表示言語","Display Language","setting_display_language") + select(LANGUAGE_NAMES)
├─ label.mobile-settings-row > span tr("LLM APIプロバイダ","LLM API Provider","setting_api_provider") + select (skip empty names; skip 4 unless VertexEnabled)
├─ [provider==8 && quota] FreeApiQuotaDisplay className="mobile-settings-freeapi-quota"
├─ [provider∉{5,8}] label.mobile-settings-row > span (provider==4 ? tr("Vertex APIキー / SA JSON","Vertex API Key / SA JSON") : tr("APIキー","API Key","setting_api_key"))
│                    + (provider==4 ? textarea.mobile-settings-textarea : input[type=password])
├─ [provider!=8] label.mobile-settings-row > span tr("LLM モデル名","LLM Model Name","setting_model_name") + input
├─ [provider∉{5,8}] label.mobile-settings-row.checkbox > span tr("LLM Web検索","LLM Web Search","setting_web_search") + input[type=checkbox]
├─ label.mobile-settings-row.checkbox > span tr("オート表示","Auto Mode","setting_auto_mode") + input[type=checkbox]
├─ [provider==0] label.mobile-settings-row.checkbox > span tr("互換APIを使う","OpenAI Compatible","setting_openai_compatible") + checkbox
│                [compatible] label.mobile-settings-row > span tr("ベースURL","Base URL","setting_openai_base_url") + input
├─ [provider==4] label.mobile-settings-row > span "Vertex Project ID" + input ; label.mobile-settings-row > span "Vertex Location" + input
│                div.config-note tr("※ Vertexを使用するためには、gCloud CLIのインストールが必要です。","* gCloud CLI must be installed to use Vertex.","setting_vertex_note")
├─ [provider==5] label.mobile-settings-row > span "Ollama Host" + input
│                div.config-note tr("※ Ollamaはローカルで動作するLLMランナーです。デフォルトのホストは http://localhost:11434 です。","* Ollama is a local LLM runner. The default host is http://localhost:11434.")
└─ div.mobile-settings-actions > button.secondary[type=button] tr("閉じる","Close","setting_close") + button.primary[type=button] tr("適用","Apply","setting_apply")
```
Notes: `.mobile-settings-row input` also matches the checkboxes (border, #111 bg, min-height 46 px, padding, radius 4) — keep native `<input type=checkbox>`. Native `<select>` here is affected by `select:focus{border-color:#333 !important}`. `.config-note` (desktop class) is reused in the sheet.

---

## 8. Responsive behavior (JS branch vs CSS media)

### 8.1 JS (RealAmadeusApp.tsx L445–475)

```ts
function isMobileViewportSnapshot(): boolean {
  if (typeof window === "undefined") return false;
  const visualWidth = window.visualViewport?.width ?? window.innerWidth;
  const coarsePointer = window.matchMedia("(pointer: coarse)").matches;
  return visualWidth <= 900 || (coarsePointer && visualWidth <= 1200);
}
// re-evaluated on: window "resize", visualViewport "resize", matchMedia("(pointer: coarse)") "change"
// server snapshot (SSR) = false
```
Used by: `RealAmadeusApp` (DOM branch), `ChatPanel` (cancel button vs hint), `MiniGamePanel` (inline style values).

### 8.2 CSS media queries in the app CSS

| Lines | Query | Selectors | Applies to |
|---|---|---|---|
| L677–722 | `(pointer: coarse), (max-width: 900px)` | `.mobile-menu-toggle` (fixed, top/left safe+10 px, min-width 88, h44, z 400, flex), `.input-panel` (padding `5px 170px 5px 10px`), `.chat-send-button` (block 96×44, `#6a2817`, 24 px), `.chat-send-button:disabled` (opacity .5, cursor default), `.chat-mic-button` (44×44, 22 px) | **both DOMs** (unscoped) |
| L2586–2965 | `(pointer: coarse), (max-width: 900px)` | everything prefixed `.mobile-root` (achievements full-screen sheet, minigame bottom sheet, floating trigger buttons, Live2D shift) | mobile DOM only |
| (none) | – | L2051–2581 `.mobile-root …` / `.mobile-*` rules | mobile DOM, regardless of media |

Verbatim L677–722:
```css
@media (pointer: coarse), (max-width: 900px) {
  .mobile-menu-toggle {
    position: fixed;
    top: calc(env(safe-area-inset-top, 0px) + 10px);
    left: calc(env(safe-area-inset-left, 0px) + 10px);
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 88px;
    height: 44px;
    border: 1px solid rgba(255, 153, 0, 0.6);
    background: rgba(20, 10, 0, 0.82);
    color: #ffb347;
    font-family: "Eurostile", sans-serif;
    font-size: 18px;
    letter-spacing: 0.04em;
    z-index: 400;
  }

  .input-panel {
    padding: 5px 170px 5px 10px;
  }

  .chat-send-button {
    display: block;
    width: 96px;
    height: 44px;
    border: 1px solid rgba(255, 153, 0, 0.7);
    background: #6a2817;
    color: #fff;
    font-size: 24px;
    line-height: 1;
    cursor: pointer;
  }

  .chat-send-button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .chat-mic-button {
    width: 44px;
    height: 44px;
    font-size: 22px;
  }
}
```

### 8.3 Divergence matrix

| Device state | JS branch | Media matches | Result |
|---|---|---|---|
| fine pointer, width > 900 | desktop | no | normal desktop |
| fine pointer, width ≤ 900 | mobile | yes | normal mobile |
| coarse pointer, width ≤ 1200 | mobile | yes | normal mobile |
| **coarse pointer, width > 1200** (iPad Pro landscape, touch tablets) | desktop | yes | desktop stage + fixed "MENU"/"閉じる" toggle (top-left, z 400, outside the scaled frame) + visible SEND button in input panel (padding-right 170) |
| fine pointer, pinch-zoomed so `visualViewport.width ≤ 900 < innerWidth` | mobile | no (max-width uses layout viewport) | mobile DOM **without** media block 2 (achievements/minigame keep desktop absolute geometry, Live2D shift = `translateX(28%)`) — edge case |

Within the mobile DOM, `.mobile-root …` rules (specificity 0,2,0+) override the media block 1 values: `.mobile-root .input-panel` padding `10px 104px 10px 16px` (`.has-audio-input` → padding-right 152 px), `.mobile-root .chat-send-button` 84×44 pill `#ff9900`. Surviving properties from block 1 on mobile: send button `line-height:1; cursor:pointer` and the `:disabled` rule.

### 8.4 Desktop shell inside Tauri

Desktop windows narrower than or equal to 900 px will render the **mobile** UI (same as V1 in a narrow browser window). Decide whether `tauri.conf.json` sets `minWidth` > 900 (Open questions).

---

## 9. Stacking contexts and z-index map

### 9.1 Desktop

```
viewport (transform: scale → stacking context & containing block for all desktop UI)
├─ [z 3000] BackLog wrapper (DOM 1st) ─ .dialog-overlay.backlog-panel z130 ─ header z2, content z2, close z3
├─ [z auto, transform → own context] .system-panel
│     .menu-wallpaper z0 · .title-bar z10 · .system-bg (auto) · login/boot panels (auto)
│     .chat-panel-root (not a context after its fade) children participate here:
│        .live2d-shift (context only while shifted/transforming) > .live2d-host z0
│        .dialogue-panel z5 (inside: hitbox z1; name/text/indicator/cancel z2)
│        .input-panel z6 · .auto-text z7 · .mini-game-button z10 · .achievement-toast z40
│        .mini-game-panel (inline) z50 (inside: title z3, content z3, bubble z3, close z4)
├─ [z 99] .menu-panel (inside: .menu-bg z1, .menu-content z2)
├─ [z 2000] Config wrapper ─ .overlay-panel.config-panel z120 (later rule beats .overlay-panel z110) ─ .config-main z2, .config-footer z3, .config-select-arrow z2, .custom-select-dropdown z100
├─ [z 2000] Status wrapper ─ .overlay-panel z110 ─ clock/grid/credits/achievement-row z2, close z3
├─ [z 2500] Achievement wrapper ─ .overlay-panel z110 ─ title/progress/list z2, close z3
├─ [z 2000] ChangeLog wrapper ─ content z2 ; [z 2000] Help wrapper ─ content z2
├─ [z 3000] Confirm(logout) wrapper ─ .dialog-overlay z130
└─ [z 3000] Confirm(shutdown) wrapper ─ .dialog-overlay z130
Outside the frame: .mobile-menu-toggle (position:fixed, z 400) when media block 1 matches.
```
Ties at z 3000 resolve by DOM order (confirm dialogs above backlog). Wrapper `opacity:0` also forms a context, but z-index already does.

### 9.2 Mobile

```
.real-amadeus-root.mobile-root (position:fixed → stacking context; containing block = viewport)
├─ .mobile-topbar z30 (absolute, gradient bg)
├─ .mobile-stage (absolute, no z) → children participate in root context:
│     .mobile-login-panel / .mobile-loading-panel (fixed, z auto)
│     .chat-panel-root (absolute, inset safe+56 0 0 0) →
│        .live2d-shift → .live2d-host (fixed, z0; containing block switches to .live2d-shift when shifted)
│        .dialogue-panel z5 · .input-panel z6 · .achievement-toast z40
│        .mini-game-button / .mobile-achievements-trigger-button (fixed, z200, media)
│        .mini-game-panel (fixed !important, z 3000 !important) (inside: title z2, bubble z3100, close z10)
├─ .mobile-settings-overlay z60 (absolute inset 0)
├─ [z 3000] BackLog wrapper ─ .mobile-root .dialog-overlay.backlog-panel (fixed, z3000 inside wrapper)
└─ [z 2500] Achievement wrapper ─ .mobile-root .overlay-panel.achievement-panel (fixed, z3000 inside wrapper, media)
```
Effective order on mobile: minigame sheet (3000, earlier DOM) < backlog (3000, later DOM); achievements wrapper (2500) is **below** the minigame sheet; settings overlay (60) is below the minigame sheet and the trigger buttons (200). The fixed inner panels escape the wrappers' `overflow:hidden` (wrappers are not their containing block).

---

## 10. Motion: animations, transitions, timers

| What | Where | Spec |
|---|---|---|
| `@keyframes chat-fade-in` | `.chat-panel-root` L323–333 | `animation: chat-fade-in 500ms linear` (opacity 0→1) on ChatPanel mount; mobile `animation: none` |
| system panel shrink | `.system-panel` L160–166 + inline transform | `transition: transform 500ms linear`, origin center |
| menu open delay | JS L4004–4016 | `setMenuOpen(true)` 450 ms after `isTabActive` turns true |
| menu panel fade | inline | `transition: opacity 0.3s ease-in-out` |
| `@keyframes menu-bg-fade` / `menu-content-fade` | `.menu-bg` 250 ms linear, `.menu-content` 300 ms linear (L554–593) | play once on mount while menu is invisible (effectively never seen) |
| `.menu-panel.closing …`, `.overlay-panel.closing`, `.dialog-overlay.closing` | L573–583, L733–738, L1603–1608 | `opacity:0; transition: opacity 200ms linear` — **class never applied** (dead) |
| `@keyframes overlay-fade` | `.overlay-panel`, `.dialog-overlay` 250 ms linear (L726–748, L1592–1601) | plays on mount only (overlays always mounted → invisible) |
| overlay wrappers | inline | `opacity 0.3s ease-in-out` (backlog, status, changelog, help, confirm, config); `0.25s` (achievements, minigame) |
| achievement toast | `.achievement-toast` L1175–1199 | `transition: opacity 400ms linear`; JS hides after 4000 ms |
| Live2D shift | `.live2d-shift` L1216–1224; mobile L2960–2964 | `transition: transform 450ms cubic-bezier(0.645, 0.045, 0.355, 1)`; desktop `translateX(28%)`; mobile `translateY(-14dvh) scale(0.85) !important`, origin `top center !important` |
| status achievement row hover | L997–1037 | color/opacity `150ms linear` (label → #ffb84d, value → #ffd9a0, underline opacity 0.6) |
| minigame buttons | L1226–1490 | `.mini-game-button` opacity/background 150 ms linear (opacity .85 → 1, bg #333 → #3d3d3d on hover); `.mg-start-button`/`.mg-next-button` background 150 ms (→ #ffad33); `.mg-answer` border-color/background 150 ms; retry/menu-back `filter: brightness(1.15)` 150 ms |
| config IO buttons | L1767–1790 | `filter 150ms linear`, hover brightness(1.15) |
| cancel streaming button | L368–386 | `transition: all 0.2s ease`; hover bg rgba(255,153,0,0.2), color #fff |
| mobile primary button | L2175–2192 | `transition: transform 0.1s`; `:active` scale(0.96) + shadow |
| mobile trigger buttons | L2928–2947 | `transition: background 150ms ease, border-color 150ms ease` (replaces the desktop opacity transition on `.mini-game-button`) |
| `@keyframes mobile-topbar-marquee` | L2101–2114 | 12 s linear infinite, translateX(0) → translateX(-100%) |
| JS timers (CSS-relevant) | – | MarqueeStatus re-check +100 ms and on resize; mobile loading 1500 ms / dots 300 ms; status clock 1000 ms; network poll 5000 ms; confirm reset `setTimeout(0)`; Live2D canvas style retry 100 ms; MS Mincho abort 8000 ms |

---

## 11. CSS custom properties

| Variable | Defined | Consumed | Notes |
|---|---|---|---|
| `--background: #000000`, `--foreground: #ededed` | root `:root` (app/globals.css) | `body` | unlayered |
| `--fill` | inline on each `input[type=range]` (ConfigSliderRow) | `.slider-wrap input[type="range"]::-webkit-slider-runnable-track` | fallback `0%`; value `${((v-min)/(max-min))*100}%` printed with JS number formatting |
| Tailwind theme vars (`--color-*`, `--font-sans`, `--font-mono`, `--default-*`, `--tw-*`) | Tailwind output | preflight + utilities | see §4.2 |
| `--color-amadeus`, `--font-amadeus`, `--animate-glitch-1/2`, `--animate-scanline` | root `@theme` | nothing | not emitted |
| `--text-primary`, `--button-*`, `--font-geist-sans` | page.module.css | – | dead file |

The app CSS uses no other `var()` and no `@property`.

---

## 12. globals.css selector index, dead code, quirks, order sensitivity

### 12.1 Line-range index by component (L = app/amadeus/app/globals.css)

| Lines | Component | Selectors |
|---|---|---|
| 1 | fonts | `@import url(Google Noto Serif JP)` |
| 3–65 | fonts | 10 × `@font-face` (§5) |
| 67–115 | globals | `*`, `input, textarea`, `*:focus`, `*:focus-visible`, `select:focus…/.config-input-control:focus…` (!important), `.amadeus-root` ×2 (97–102, 104–109), `button, input, select` |
| 117–202 | desktop shell | `.real-amadeus-root`, `.window-frame`, `.window-frame.windowed`, `.viewport`, `.system-panel`, `.menu-wallpaper`, `.title-bar`, `.system-bg` |
| 204–296 | desktop login | `.login-panel`, `.login-logo`, `.login-row`, `.login-row label`, `.user-id-row label`, `.login-row input`, `.user-id-row`, `.password-row`, `.password-row label`, `.login-button`, `.login-button img`, `.login-error` |
| 298–321 | desktop boot | `.loading-panel`, `.boot-text`, `.boot-logo` |
| 323–546 | chat | `.chat-panel-root` + `@keyframes chat-fade-in`, `.live2d-host`, `.live2d-host canvas`, `.dialogue-panel`, `.dialogue-panel .dialogue-advance-hitbox, .dialogue-panel .cancel-streaming-button`, `.cancel-streaming-button(:hover)`, `.pc-cancel-hint`, `.sleeping-input`, `.dialogue-advance-hitbox`, `.name-text`, `.dialogue-text`, `.waiting-indicator`, `.input-panel`, `.chat-input(::placeholder)`, `.chat-send-button`, `.chat-input-actions`, `.chat-mic-button(.recording)`, `.chat-mic-level(span)`, `.chat-stt-hint`, `.auto-text` |
| 548–673 | menu | `.menu-panel`, `.menu-bg`, `.menu-content`, `.menu-panel.closing .menu-bg/.menu-content`, `@keyframes menu-bg-fade/menu-content-fade`, `.menu-icon`, `.menu-icon img`, `.menu-icon .menu-icon-selected`, `.menu-icon.selected .menu-icon-selected`, `.menu-icon.selected`, `.menu-label.selected, .small-menu-label.selected`, `.menu-label`, `.small-menu-label`, `.mobile-menu-toggle` |
| 677–722 | media block 1 | §8.2 |
| 726–822 | overlay common | `.overlay-panel(.closing)`, `@keyframes overlay-fade`, `.overlay-bg`, `.overlay-header(h2)`, `.status-panel .overlay-header`, `.help-panel .overlay-header`, `.changelog-panel .overlay-header(h2)`, `.line`, `.line.thin`, `.overlay-close-button` |
| 824–896 | backlog + scrollbars | `.backlog-content`, `.backlog-row`, `.backlog-name`, `.backlog-message`, `::-webkit-scrollbar(-track/-thumb)` for `.backlog-content, .change-log-content, .help-content, .config-form, .dialogue-text` (10 px, track #fff, thumb #ff9900) |
| 898–1037 | status | `.status-clock`, `.status-grid`, `.status-row`, `.status-label`, `.status-value(.good/.warn/.bad)`, `.status-credits`, `.status-achievement-row/-label/-value/-underline` + `:hover` variants |
| 1039–1214 | achievements + toast | `.achievement-panel-title`, `.achievement-panel-progress`, `.achievement-list(::-webkit-scrollbar)`, `.achievement-row(.unlocked/.locked)`, `.achievement-icon(.unlocked/.locked)`, `.achievement-text`, `.achievement-row-title`, `.achievement-row-desc` (+ unlocked/locked colors), `.achievement-date`, `.achievement-toast(.visible)`, `.achievement-toast-title`, `.achievement-toast-desc` |
| 1216–1511 | live2d shift + minigame | `.live2d-shift(.live2d-shifted)`, `.mini-game-button(:hover)`, `.mobile-achievements-trigger-button{display:none}` (1249–1251), `.mini-game-title`, `.mini-game-content`, `.mini-game-menu/.mini-game-playing/.mini-game-result`, `.mg-*` (menu-title, menu-desc, start-button, header, question-counter, score-label, question-box, answers, answer + states, answer-text, spacer, next-button, result-title, score-result, result-comment, result-buttons, retry-button, menu-back-button, comment-bubble, close-button) |
| 1513–1590 | changelog, help | `.change-log-content`, `.change-log-entry .change-log-head/.version/.date/ul`, `.help-content`, `.help-entry .title/.line.thin/.description` |
| 1592–1657 | dialog | `.dialog-overlay(.closing)`, `.confirmation-dialog`, `.message`, `.choice(.no/.yes/.selected)` |
| 1659–2049 | config | `.config-panel`, `.config-main`, `.config-title`, `.config-sidebar(button, button.active)`, `.config-content`, `.config-section-title`, `.config-content > .line.thin`, `.config-form(.dense)`, `.config-row`, `.config-label`, `.config-io-*`, `.config-import-input`, `.config-input-control(:focus)`, `.config-select-shell`, `.config-select-arrow(img)` (img filter `brightness(0) saturate(100%) invert(62%) sepia(98%) saturate(1661%) hue-rotate(4deg) brightness(102%) contrast(103%)` = orange tint), `.custom-select-trigger/-dropdown(+scrollbar)/-option(:hover)`, `.config-checkbox-shell`, checkbox + `:checked::after`, `.slider-wrap` (gap 300, width 680), range track/thumb (webkit + moz), `.slider-value`, `.config-subtitle`, `.config-note`, `.config-freeapi-quota`, `.config-runtime(.good/.warn/.bad)`, `.config-footer(button/.cancel/.apply)` |
| 2051–2306 | mobile shell & panels | `.real-amadeus-root.mobile-root`, `.mobile-topbar`, `.mobile-topbar-button`, `.mobile-topbar-status(-inner)(.marquee)`, `@keyframes mobile-topbar-marquee`, `.mobile-stage`, `.mobile-login-panel, .mobile-loading-panel`, `.mobile-login-logo`, `.mobile-form-row(input)`, `.mobile-form-error`, `.mobile-primary-button(:active)`, `.mobile-loading-title`, `.mobile-loading-line`, `.mobile-settings-*` |
| 2308–2457 | mobile chat overrides | `.mobile-root`, `.mobile-root .chat-panel-root/.live2d-host/canvas/.dialogue-panel/.name-text/.dialogue-text/.waiting-indicator/.input-panel(.has-audio-input)/.chat-input/.chat-send-button/.chat-input-actions/.chat-mic-button/.auto-text`, `.mobile-topbar-buttons` |
| 2459–2581 | mobile backlog | `.mobile-root .dialog-overlay.backlog-panel`, `… .overlay-header(h2)`, `… .backlog-content`, `… .backlog-row(.backlog-row-user/.backlog-row-me)`, `… .backlog-name`, `… .backlog-message`, `… .overlay-close-button` |
| 2583–2965 | media block 2 | achievements sheet (2588–2707), minigame sheet (2710–2925), trigger buttons (2928–2957), live2d shift (2960–2964) |

### 12.2 Dead / unreachable CSS (keep verbatim; do not rely on it)

- `.menu-panel.closing …` (L573–583), `.overlay-panel.closing` (L733–738), `.dialog-overlay.closing` (L1603–1608): `closing` is never set.
- `.config-runtime` + `.good/.warn/.bad` (L2003–2021), `.config-subtitle` (L1977–1985), `.mobile-loading-title` (L2194–2199): never rendered.
- `.backlog-row-me` (L2531): only reachable when an imported conversation has role "me".
- `.custom-select-dropdown::-webkit-scrollbar*` (L1873–1882): dropdown has `overflow-y: visible` → never scrolls.
- `.dialogue-text::-webkit-scrollbar*`: only effective on mobile (desktop text box doesn't scroll).
- `.live2d-host canvas { width:3840px; height:2160px }`: overridden by inline styles.
- `.config-input-control:focus { border-color:#ff9900 }`: overridden by the `!important` #333 rule.
- `@font-face` "Yu Mincho", "Noto Serif", "Noto Serif CJK KR": never referenced.
- Mount-only animations (`overlay-fade`, `menu-bg-fade`, `menu-content-fade`): invisible.
- Redundant: `.user-id-row label{color:#ffe600}` (same as `.login-row label`), `.config-sidebar button.active{background:#333333}` (same as base), `.config-content > .line.thin{background:#ff9900}` (same as `.line`), duplicate `.amadeus-root` blocks.
- Classes emitted by TSX with **no** CSS: `fullscreen`, `open` (custom-select-trigger), `slider-row`, `menu-icon-normal`, `backlog-panel-wrapper`, `mobile-loading-logo`, `backlog-row-assistant` (etc.).

### 12.3 Invalid / ignored declarations (kept by V1's Lightning CSS; browsers drop them)

- `.overlay-bg { object-fit: stretch; }` (L755) — not a valid value; computed `fill` (stretches anyway).
- `.viewport { image-rendering: high-quality; }` (L155) — unsupported in Chromium/WebKit; ignored.
- `-ms-overflow-style: none` (L1073) — IE only.

### 12.4 Order-sensitive pairs (why the block must stay in original order)

1. `.real-amadeus-root` (L117: `position:relative`) vs `.mobile-root` (L2308: `position:fixed`) — equal specificity, later wins → mobile root is fixed.
2. `.chat-send-button{display:none}` (L483) vs media `.chat-send-button{display:block}` (L700); same for `.mobile-menu-toggle` (L671 vs L678) and `.input-panel` padding (L457 vs L696).
3. `.overlay-panel{z-index:110}` (L726) vs `.config-panel{z-index:120}` (L1659) on the same element.
4. `.overlay-close-button{z-index:3}` (L810) vs `.mg-close-button{z-index:4}` (L1509).
5. `.mobile-settings-row input, .mobile-settings-row select, .mobile-settings-textarea` (L2253) vs `.mobile-settings-textarea` (L2266) — textarea gets min-height 100, DepartureMono 15 px from the later rule.
6. `.mini-game-button:hover` (L1244) vs `.mobile-root .mini-game-button` (L2928, same specificity, later) → on mobile hover keeps the dark translucent background but still sets opacity 1; mobile transition replaces desktop transition. `.mobile-achievements-trigger-button` never gets the 0.85 opacity → the two mobile trigger buttons differ slightly (V1 quirk).
7. `.amadeus-root` duplicate blocks (merge-safe).

### 12.5 `!important` inventory (33 declarations)

L85, L92–94 (focus resets), L401 (`.sleeping-input` italic), L2332–2333 (mobile canvas size), L2711–2722 (mobile minigame sheet beats inline styles), L2908–2923 (mobile close pill), L2932 (mobile trigger bottom), L2961–2962 (mobile Live2D shift). They intentionally beat inline styles — so V2 must keep the inline styles as plain (non-important) declarations.

---

## 13. Build transformations V1 applied (replicate for parity)

V1's amadeus CSS went through Lightning CSS (Next default targets `chrome 111, edge 111, firefox 111, safari 16.4`). Observable differences between `globals.css` and the shipped `1b90ae59afeb13d0.css`:
- `-webkit-backdrop-filter` added next to each of the 6 `backdrop-filter` declarations (L2350, L2406, L2470, L2599, L2719, L2945). **Required** for Safari 16.4/17 and therefore for macOS WKWebView in Tauri.
- `.amadeus-root` blocks merged; declarations reordered (no semantic change); colors shortened (`#ff9900`→`#f90`, `rgba(...)`→hex-alpha); `transform: scale(1.666667)` → `scale(1.66667)`; `transform-origin: top center` → `top`.
- Kept as-is: `object-fit:stretch`, `image-rendering:high-quality`, the `@import url(...)` at the top, all `!important`s, both media blocks.

Recommendation: run the V2 `amadeus-fonts.css`/`amadeus-app.css` through the **`lightningcss` Rust crate** (xtask, or a Trunk `pre_build` hook) with targets `{ chrome: 111<<16, edge: 111<<16, firefox: 111<<16, safari: (16<<16)|(4<<8) }`, `minify: true`, no bundling (keep the remote `@import`). Output is then byte-for-byte comparable with V1's chunk (useful as a parity test). The Tailwind CLI output already contains its own fallbacks/prefixes.

---

## 14. Desktop WebView compatibility (Tauri 2)

| Feature | WebView2 (Win, Chromium) | WKWebView (macOS) | WebKitGTK (Linux) | Action |
|---|---|---|---|---|
| `backdrop-filter` | ok | needs `-webkit-` before Safari 18 | needs `-webkit-` on older | Lightning CSS prefixing (§13) |
| `100dvh` / `dvh` in `translateY(-14dvh)` | ok | Safari ≥ 15.4 | ok on 2.40+ | none |
| `::-webkit-scrollbar` styling | ok | ok (overlay scrollbars may ignore width) | ok | none |
| `scrollbar-gutter: stable` | ok | may be unsupported | may be unsupported | harmless |
| `color-mix()` (Tailwind selection/placeholder) | ok | ok (16.2+) | ok | fallbacks exist |
| `appearance:none` range/checkbox, `accent-color` | ok | ok | ok | none |
| `env(safe-area-inset-*)` | 0 | 0 | 0 | none |
| Fullscreen API (`requestFullscreen`, used by screen mode; toggles `.window-frame.fullscreen/.windowed`) | ok | element fullscreen may be disabled in WKWebView | ok | desktop spec: map to Tauri window fullscreen |
| Remote fonts / fetch | – | – | – | CSP must allow `https://fonts.googleapis.com` (style-src), `https://fonts.gstatic.com` (font-src), `https://raw.githubusercontent.com` (connect-src) |
| Inline styles | – | – | – | if a CSP is set, `style-src 'unsafe-inline'` (Leptos `style="…"` attributes) |

Font rendering differs per OS exactly as in V1 browsers ("Noto Serif CJK JP" system font present on many Linux installs, MS Mincho local on Windows).

---

## 15. React/Next-specific DOM dependencies

- No `next/font` usage in the app (only the dead page.module.css references `--font-geist-sans`). No selectors target Next-generated elements (`next-route-announcer`, `#__next` does not exist in App Router).
- Reproduce: body classes, `.amadeus-root` wrapper div (Leptos must render it as the top-level element mounted into `<body>`), always-mounted overlays with wrappers, `dangerouslySetInnerHTML` → `inner_html` for `.status-value`, `class=""` / trailing spaces are irrelevant.
- React serializes numeric style values with `px` (e.g. `left: 225px`, `top: 0px`) and unitless for `opacity`/`z-index`; custom properties via `style.setProperty("--fill", …)`.
- `suppressHydrationWarning` → irrelevant in CSR.

---

## 16. Recommended CSS file split and scaffolding (exact)

```
crates/amadeus-ui/
├─ index.html
├─ Trunk.toml
└─ styles/
   ├─ tailwind.css        = app/globals.css L1–69 verbatim, with L1 replaced by:
   │                          @import "tailwindcss" source(none);
   │                          @source "../src/**/*.rs";
   │                          @source "../index.html";
   ├─ amadeus-fonts.css   = §5.5 (globals.css L1–65 with "/resources/" → "resources/")
   ├─ amadeus-app.css     = app/amadeus/app/globals.css L67–2966 VERBATIM (LF). CI check:
   │                          diff <(sed -n '67,2966p' V1/app/amadeus/app/globals.css) styles/amadeus-app.css
   └─ site-pages.css      = §4.5, page-scoped (web only; omit if landing/docs are not ported)
(processed copies → styles/dist/*.css by the lightningcss xtask)
```

`index.html` (order matters):
```html
<!DOCTYPE html>
<html lang="en" class="dark">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
  <meta name="theme-color" content="#000000" />
  <meta http-equiv="Cache-Control" content="no-cache, no-store, must-revalidate" />
  <meta http-equiv="Pragma" content="no-cache" />
  <meta http-equiv="Expires" content="0" />
  <title>Amadeus</title>
  <base data-trunk-public-url />
  <!-- 1 --> <link data-trunk rel="tailwind-css" href="styles/tailwind.css" />
  <!-- 2 --> <link data-trunk rel="css" href="styles/dist/amadeus-fonts.css" />   <!-- contains the Google @import first -->
  <!-- 3 --> <link data-trunk rel="css" href="styles/dist/amadeus-app.css" />
  <!-- 4 --> <link data-trunk rel="css" href="styles/dist/site-pages.css" />      <!-- web build only -->
  <link data-trunk rel="copy-dir" href="<path-to>/resources" />                    <!-- → dist/resources -->
  <link data-trunk rel="rust" />
</head>
<body class="bg-black text-white min-h-screen selection:bg-purple-500/30 selection:text-white"></body>
</html>
```
(Other head metadata — description, application-name, apple-web-app tags, manifest, icons — per the app-shell spec.)

`Trunk.toml` sketch:
```toml
[build]
target = "index.html"
public_url = "/amadeus/"          # desktop: trunk build --public-url /
[tools]
tailwindcss = "4.1.18"
[[hooks]]
stage = "pre_build"
command = "cargo"
command_arguments = ["run", "-p", "xtask", "--", "css"]   # lightningcss pass → styles/dist
```

If landing/docs pages share the SPA: the app CSS (`amadeus-app*.css`, whose unlayered `*{margin:0;padding:0}` would erase every Tailwind spacing utility on those pages) must be **disabled outside `/amadeus/app*`**: on every route change set `link.disabled = !is_app_route` for `<link rel=stylesheet>` whose `href` contains `/amadeus-app-` (Trunk keeps the file stem in the hashed name). V1 only loads it on the app route (fresh load).

---

## 17. Verification checklist

1. Computed-style parity: load V1 and V2 side by side (Chromium, 1920×1080 and 390×844 with touch emulation); compare `getComputedStyle` for every element with a class from §12.1 (position/size/font/color/z-index/line-height).
2. Pixel diffs of: login, boot, chat idle, dialogue typing, menu open, each overlay, config each category, minigame (menu/playing/revealed/result), achievements toast; mobile: login, loading, chat, settings sheet, backlog, achievements, minigame sheet.
3. Network: fonts requested from `{base}resources/fonts/...` (web) / `/resources/fonts/...` (desktop), `Eurostile%20Condensed%20Bold.otf` served 200 with `font/otf`.
4. `document.styleSheets` order: Tailwind → fonts → app; app CSS has no `@layer`.
5. Shipped app CSS contains 6 × `-webkit-backdrop-filter`.
6. Touch device > 1200 px: desktop stage + MENU toggle + SEND button.
7. Live2D canvas inline style present; canvas `clientWidth` = host width.

---

## Rust mapping

| Concern | Crate / module | Recommended items |
|---|---|---|
| Stylesheets | `amadeus-ui/styles/*` + `index.html` | files per §16; no Rust code |
| CSS post-processing | `xtask` (bin, native) | `fn build_css() -> anyhow::Result<()>`: read `styles/amadeus-fonts.css`, `styles/amadeus-app.css`, `styles/site-pages.css`; `lightningcss::stylesheet::StyleSheet::parse` → `minify(MinifyOptions{ targets })` → `to_css(PrinterOptions{ minify: true, targets })`; targets `Browsers{ chrome: Some(111<<16), edge: Some(111<<16), firefox: Some(111<<16), safari: Some((16<<16)|(4<<8)), ..Default::default() }`; write `styles/dist/*.css` |
| Asset URLs | `amadeus_ui::style::assets` | `pub fn public_url() -> &'static str` (from `option_env!("TRUNK_PUBLIC_URL")`/build-time env, or `document.baseURI`), `pub fn asset(rel: &str) -> String` (`"resources/images/Amadeus_BG.png"` → `"{public_url}resources/images/Amadeus_BG.png"`); constants `IMG_AMADEUS_BG`, `IMG_LOGO_V3`, `IMG_LOGIN_BUTTON`, `IMG_MENU_WALLPAPER`, `IMG_MENU_BG`, `IMG_DOWN_ARROW`, `fn menu_icon(name: &str, selected: bool) -> String` |
| Viewport detection | `amadeus_ui::style::viewport` | `pub const MOBILE_MAX_WIDTH: f64 = 900.0; pub const COARSE_MOBILE_MAX_WIDTH: f64 = 1200.0;` `pub fn is_mobile_viewport_snapshot() -> bool` (exact §8.1 logic), `pub fn use_is_mobile_viewport() -> Signal<bool>` (listeners: window resize, visualViewport resize, `matchMedia("(pointer: coarse)")` change; remove on cleanup) |
| Stage scaling | `amadeus_ui::style::stage` | `pub const DESIGN_W: f64 = 1920.0; pub const DESIGN_H: f64 = 1080.0;` `pub struct StageDims{ width: f64, height: f64, scale: f64 }`, `pub fn compute_stage(inner_w: f64, inner_h: f64) -> StageDims`, `pub fn system_panel_transform(tab_active: bool) -> String` (`"translate(150px, -12px) scale(0.6)"` / `"translate(0px, 0px) scale(1)"`), `pub fn menu_panel_style(open: bool, d: &StageDims) -> String` |
| Inline style helpers | `amadeus_ui::style::inline` | `pub enum Fade { Ms300, Ms250 }`, `pub fn overlay_wrapper_style(visible: bool, z: u32, fade: Fade, backlog_extras: bool) -> String` (exact strings §7.9), `pub fn minigame_panel_style(visible: bool, mobile: bool) -> String`, `pub fn slider_fill(value: f64, min: f64, max: f64) -> String` (`"--fill: {js}%"`), `pub fn mic_level_width(level: f64) -> String`, `pub fn js_number(x: f64) -> String` (JS Number→String: `-0`→`"0"`, integers without `.0`, shortest round-trip digits; Rust `{}` already matches for normal values — differences only for `-0` (Rust `"-0"`), infinities (Rust `"inf"` vs JS `"Infinity"`), and exponent form (JS uses `1e+21`-style for |x| ≥ 1e21 and `1e-7`-style for 0 < |x| < 1e-6; Rust never does) — none of which occur in the style values above except `-0`) |
| Class composition | `amadeus_ui::style::cls` | `pub fn cls(base: &str, flags: &[(&str, bool)]) -> String` — e.g. `cls("mg-answer", &[("revealed", r), ("correct", r && i==c), ("wrong", r && i==s && i!=c)])`; `backlog_row_class(role: &str) -> String` = `format!("backlog-row backlog-row-{}", role.to_lowercase())` |
| Fonts | `amadeus_ui::style::fonts` | `pub const MS_MINCHO_URL: &str = "https://raw.githubusercontent.com/vslabs/winfonts/master/msmincho.ttc"; pub const MS_MINCHO_TIMEOUT_MS: u32 = 8000; pub const MS_MINCHO_CHECK: &str = "16px \"MS Mincho\"";` `pub fn spawn_ms_mincho_loader() -> impl FnOnce()` (returns canceller; algorithm §5.4; call once after root mount) |
| Live2D canvas | `amadeus_live2d::player` | when creating the canvas: inline `display:block;width:100%;height:100%;pointer-events:auto` (§7.5.2); configurable resource prefix (default `asset("resources/")`) |
| Route-scoped CSS | `amadeus_ui::style::stylesheets` | `pub fn set_app_css_enabled(enabled: bool)` (web, only if site pages are ported) |
| Desktop | `amadeus-desktop/tauri.conf.json` | `build.frontendDist` = Trunk dist built with `--public-url /`; CSP (§14); window `minWidth`/`minHeight` decision (§8.4) |
| Server | `amadeus-server` | serve `dist/` at `/amadeus/`, resources at `/amadeus/resources/*` **and** legacy `/resources/*`, caching headers §6, MIME `font/otf`/`font/ttf`, percent-decoded paths |

---

## Open questions / risks

1. **Preflight dependency** — if any V2 page loads the app CSS without Tailwind's base layer (e.g. a desktop build that skips Tailwind), line heights, image display and button backgrounds change. Ship the Tailwind output in every build (it is small).
2. **Never layer or scope the app CSS** — wrapping it in `@layer` or prefixing selectors (e.g. for Leptos component scoping/stylers) inverts precedence against Tailwind and changes specificity; the `!important` rules intentionally beat inline styles.
3. **Landing/docs pages in scope?** They are Tailwind-heavy and live in the DSUV Next site. If ported, the app CSS must be disabled on those routes (§16). If not ported, drop `site-pages.css` and the toggle. Decide early (affects `public_url`, router, Tailwind sources).
4. **Font payload** — 73.6 MB of fonts (three 24.5 MB CJK OTFs). Two are never referenced (TC, KR) — bundling them into desktop installers is pure weight; consider omitting unused files from the desktop bundle (keeping the `@font-face` rules is harmless since unused faces never load). Subsetting would change glyph coverage → visual change; out of parity scope.
5. **Google Fonts dependency** — desktop offline renders differently (no 700 weight, different CJK at 300). For offline parity, vendor the css2 response + woff2 slices under `resources/fonts/google/` and replace the `@import` (same order). Needs a decision; licensing OK (OFL).
6. **MS Mincho runtime download** — fetches a proprietary Microsoft font from a third-party GitHub repo on most launches (cache max-age short). Legal/licensing risk and multi-MB traffic; replicate for parity only if accepted. Requires CSP connect-src in Tauri.
7. **JS vs CSS mobile detection mismatch** (§8.3) must be preserved bit-for-bit; "fixing" it changes tablet behavior.
8. **Inline styles are part of the CSS contract** — wrapper z-indexes/transitions, `--fill`, menu transforms, Live2D canvas inline size. Losing any of them silently breaks stacking or layout. Note JS→string number formatting (normalize `-0`).
9. **Always-mounted overlays** — Leptos `<Show>` would unmount panels and change fade behavior, scroll position retention (backlog auto-scroll), key handlers. Keep them mounted and toggle the wrapper style.
10. **Lightning CSS prefixing** for `backdrop-filter` is mandatory for macOS desktop; verify Trunk does not re-minify/strip it (Trunk's own CSS minifier may also use lightningcss — use `data-no-minify` on the processed files if needed).
11. **Base path & relative URLs** — CSS `url()` resolves relative to the stylesheet; if Trunk outputs CSS into a subdirectory, relative font URLs break. Lock the output location and add the network check from §17.3. The V1 Live2D runtime and some caches still use root `/resources/` — keep the alias on the server.
12. **Filename with spaces** (`Eurostile Condensed Bold.otf`) — fine with percent-decoding servers; renaming is safe (only CSS references it) but breaks byte-for-byte diff with V1.
13. **Tauri window width ≤ 900 px** shows the mobile UI; decide `minWidth` (V1 Qt desktop disabled drag-resizing per its changelog).
14. **V1 quirks to decide (keep vs fix)**: OPERATOR/user name injected as HTML in Status; Vertex index-4 filter applied to every ConfigSelectRow (hides "Español"/"한국어"); `.windowed` border clipping 2 px; marquee never turning off; mobile achievements trigger without 0.85 opacity; `glow-on-hover` dead on landing page. Parity says keep; product may say fix.
15. **Version strings** — Status shows `Real Amadeus v1.4N(Build 20260801_001)` in two places; the user wants the Rust edition to be "V2". `.status-value` is `white-space: nowrap` inside an 837 px grid and `.status-credits` is 1820 px wide — check the new string fits.
16. **Tailwind v4 in Trunk** — confirm the Trunk release used supports `rel="tailwind-css"` with Tailwind v4 (`@import "tailwindcss"` syntax, `[tools] tailwindcss = "4.1.18"`); otherwise run the standalone CLI from a hook. Builds need network to download the CLI unless vendored.
17. **WebKit differences** (macOS/Linux desktop): scrollbar styling, `scrollbar-gutter`, Fullscreen API, font smoothing; plan a manual QA pass on WKWebView and WebKitGTK.

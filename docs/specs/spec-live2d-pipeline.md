# Porting spec — `live2d-pipeline`: Cubism sample/app layer + `ReamaCubismPlayer` → `amadeus-live2d`

**Target crate:** `amadeus-live2d` (wasm32 only; `wasm-bindgen` + `web-sys` WebGL; binds the proprietary
`Live2DCubismCore` JS). **Consumer:** `amadeus-ui` (Leptos 0.8 CSR). The Web build and the Tauri 2 desktop build run the
*same* WASM frontend inside a browser/webview, so everything in this spec is client-side only (no backend, no IPC).

**What this spec covers** — everything between "the page wants Kurisu in this `<div>`" and "the renderer receives an
MVP matrix and draws":

* the public runtime API (`create` / `preload` / player setters / `destroy`) and how the host page loads it,
* Core loading + readiness, framework start-up and ref-counting,
* WebGL context creation, canvas sizing, `devicePixelRatio`, resize,
* asset fetching + in-memory caching + preloading, the `LAppModel` load state machine, texture upload
  (premultiplied alpha, `MAX_TEXTURE_SIZE` downscale),
* the **exact per-frame pipeline order** (SDK update scheduler, double core update, parameter driver),
* the projection / view / model matrix setup (desktop vs mobile constants),
* the complete custom **`ReamaCubismPlayer`** parameter driver (emotion targets, motion bursts, noise/drift idle
  motion, blink FSM, sneeze easter egg, sleep nodding, startled wake-up, gaze tracking, breath, vowel lip sync),
* destroy / release / WebGL context-loss behaviour, console output, golden test vectors.

**Out of scope** (owned by sibling specs, referenced only at the interface):

* Renderer, shaders, clipping masks, render targets, `CubismMatrix44`/`CubismModelMatrix`/`CubismViewMatrix`/
  `CubismTargetPoint` internals → `specs/spec-live2d-framework-render.md` (same scratchpad directory).
* Generic framework internals (CubismModel, CubismPhysics, CubismJson, CubismMotion…): only the semantics this layer
  relies on are restated here (§7.6, §7.7).
* The React/Leptos component around the canvas → `specs/spec-lib-media.md` §6 (`Live2DCanvas`), and where the
  emotion / lip-sync / spoken-char values come from → `specs/spec-app-part2.md`, `spec-app-part3.md`.
  §3.4 below restates the component contract and adds two corrections (§3.4.4).

Where V1 behaviour differs from what the code obviously intends it is marked **QUIRK**, and the spec says whether V2
must reproduce it ("**V2: reproduce**") or may fix it ("**V2: fix**"). Default is *reproduce*.

---

## 0. Sources, provenance, conventions

### 0.1 Files read completely

Abbreviations: `B` = `/home/elvelt_admin/Elvelt/dsuv/public/resources/live2d/reama-cubism-runtime.js`
(the deployed runtime; 669,953 bytes, 18,643 lines, md5 `a62f701cffe615e66c50135d5d358330`, mtime 2026‑11‑09 11:08,
**uncommitted working-tree version**); `REF` = `/tmp/claude-0/-home-elvelt-admin-Elvelt-dsuv/1f3badd8-3ee9-4a13-9340-35fac0e7abf1/scratchpad/ref/RealAmadeusWeb`.

| Bundle lines | Source file (esbuild `// path` marker) | Role here |
|---|---|---|
| B 1–22 | esbuild prelude (`"use strict"; (() => {`, `__spreadValues`, `__spreadProps`) | `__spreadValues(a,b)` = `Object.assign`-like copy of own enumerable props; `__spreadProps(a,b)` = add b's props to a |
| B 1007–1127 | `cubismframeworkconfig.ts`, `utils/cubismdebug.ts` | log gating (§11) |
| B 1688–1700 | `CubismJson.arrayBufferToString` | UTF‑8 decode (customized, §0.3 A1) |
| B 2431–2623 | `live2dcubismframework.ts` | `CubismFramework.startUp/initialize/dispose/cleanUp` (§4.5) |
| **B 2625–2664** | `Samples/.../lapppal.ts` | `LAppPal` clock (§7.2) |
| **B 2665–2698** | `Samples/.../lappdefine.ts` | constants (§2.7) |
| **B 2699–2764** | *(user code appended to lappdefine.ts)* | cached fetch helpers (§6.1) |
| **B 2766–2799** | `Samples/.../lappglmanager.ts` | WebGL context (§4.7) |
| B 3038–3422 | `rendering/cubismoffscreenmanager.ts` | per-frame begin/end bookkeeping (no-op for this model) |
| B 3536–4098 | `cubismmodelsettingjson.ts` | model3.json accessors used by the state machine |
| B 4100–4255 | `effect/cubismbreath.ts`, `effect/cubismlook.ts` | updater math (§7.6) |
| B 4256–4416 | `effect/cubismeyeblink.ts` | **not active** for this model (verified) |
| B 7671–8230 | `physics/cubismphysics.ts` | integration semantics only (§7.6.4) |
| B 9353–10748 | `model/cubismmodel.ts` | parameter load/save/set/add/clamp (§7.7) |
| B 11394–11515 | `rendering/cubismshader_webgl.ts` (shader file list, `loadShader`) | shader fetch (§6.4) |
| B 13418–13660, 14126–14310 | `rendering/cubismrenderer_webgl.ts` (`initialize`, `bindTexture`, `loadShaders`, `doDrawModel`, `setRenderState`, `startUp`) | renderer entry points used by this layer |
| B 14312–14419 | `model/cubismmoc.ts` | moc creation / failure |
| **B 14590–14963** | `model/cubismusermodel.ts` | base class of `LAppModel` |
| **B 14965–15398** | `motion/icubismupdater.ts`, `cubismupdatescheduler.ts`, 7 updaters, `iparameterprovider.ts` | update scheduler (§7.6) |
| **B 15399–15664** | `Samples/.../lappwavfilehandler.ts` | dead code in V1 (§0.4) |
| **B 15666–16383** | `Samples/.../lappmodel.ts` | load state machine, per-frame update, draw (§6.3, §7.6) |
| **B 16385–16572** | `Samples/.../lapplive2dmanager.ts` | `onUpdate` = frame orchestration + projection (§7.5, §8) |
| **B 16574–16725** | `Samples/.../lapptexturemanager.ts` | texture upload (§6.5) |
| **B 16727–16996** | `math/cubismviewmatrix.ts` | view matrix (§8.1) |
| **B 16998–17134** | `Samples/.../lappsprite.ts` | dead code in V1 |
| **B 17136–17272** | `Samples/.../touchmanager.ts` | dead code in V1 |
| **B 17274–17474** | `Samples/.../lappview.ts` | view/device matrices, sprite program, `render()` (§4.8, §7.4) |
| **B 17476–17687** | `Samples/.../lappsubdelegate.ts` | canvas/GL owner, resize, frame GL state (§4.6, §5, §7.3) |
| **B 17689–18643** | `scripts/cubism-runtime-entry.ts` | `ReamaCubismPlayer`, `preloadDefaultAssets`, `window.ReamaCubismRuntime` (§3, §9) |

Also read completely:

* `/home/elvelt_admin/Elvelt/dsuv/components/amadeus/Live2DCanvas.tsx` (162 lines, md5 `43bfaff0e3215f6566f9270a3e622044`, uncommitted).
* Model dir `/home/elvelt_admin/Elvelt/dsuv/public/resources/models/AmadeusKurisu5.0/reama5.0/`:
  `Live2D紅莉栖forSDK5.0.model3.json`, `.physics3.json`, `.cdi3.json` (verbatim/summarized in §2).
* `REF/CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Samples/TypeScript/Demo/src/*.ts` (13 files),
  `REF/scripts/cubism-runtime-entry.ts` (older April entry), `REF/scripts/build-cubism-runtime.mjs`.
* Call sites in `components/amadeus/RealAmadeusApp.tsx` (L842–905 idle→SLEEPING, L1120–1190 spoken char,
  L1700–1760 `/dev emotion`, L1863–1870 `<Live2DCanvas …>`, L3974 `preloadLive2D()`), CSS `app/amadeus/app/globals.css`
  L150–158, L323–349, L1216–1224, L2323–2340, L2960–2963.

### 0.2 Build provenance

* V1 builds the runtime with esbuild (`REF/scripts/build-cubism-runtime.mjs`): entry `scripts/cubism-runtime-entry.ts`,
  `bundle: true, format: "iife", platform: "browser", target: "es2017"`, tsconfig path alias
  `@framework/* → CubismSdkForWeb-5-r.5/.../Framework/src/*`, then copies `Framework/Shaders/WebGL/*` to
  `public/resources/live2d/shaders/`. Not minified, comments preserved.
* **The entry source that produced the deployed bundle is not in the repository** (`dsuv/scripts/` has no
  `cubism-runtime-entry.ts`); `REF/scripts/cubism-runtime-entry.ts` is an older (2026‑04‑11) version. The git HEAD
  bundle is also different (the working tree changes 298 lines). **The deployed working-tree bundle `B` is the only
  source of truth.** All behaviour below is taken from `B`; the ref TS was used only to recover type information,
  comments and the customization history.
* The sibling `live2d-framework-render` task rebuilt `B` from the ref TS with the same esbuild options and diffed it
  (`/tmp/claude-0/-home-elvelt-admin-Elvelt-dsuv/1f3badd8-3ee9-4a13-9340-35fac0e7abf1/scratchpad/build-l2d/bundle.diff`,
  939 lines). I re-read every hunk against `B`; the inventory in §0.3‑A is complete.
* The Core: `/home/elvelt_admin/Elvelt/dsuv/public/resources/live2d/live2dcubismcore.min.js` (228,042 bytes,
  md5 `ec55199882d23fd1883ddc2cc1fec569`), "Live2D Cubism SDK Core Version 6.0.1", `csmGetVersion()` = 100663297.
  Kept as-is in V2.

### 0.3 Customization inventory

#### A. Deployed bundle vs. ref TS (changes made after 2026‑04‑11; all live in V1)

| # | Where (B lines) | Change |
|---|---|---|
| A1 | 1690–1692 `CubismJson.arrayBufferToString` | Fast path `if (typeof TextDecoder !== "undefined") return new TextDecoder("utf-8").decode(buffer);` before the SDK's `%xx` + `decodeURIComponent` fallback. (A leading UTF‑8 BOM is stripped by TextDecoder.) Needed for the Japanese file names in model3.json. |
| A2 | 2693–2694 | `MOCConsistencyValidationEnable` and `MotionConsistencyValidationEnable` changed `true → false` (no `hasMocConsistency` check at load). |
| A3 | 2699–2764 | Module-level caches `s_arrayBufferCache`, `s_textCache`, `s_imageCache` + `cachedFetchArrayBuffer`, `cachedFetchText`, `cachedLoadImage` (§6.1). |
| A4 | 11449–11451 `CubismShader_WebGL.loadShader` | `fetch(url)` → `cachedFetchText(url)`. |
| A5 | 15676 `LAppModel.loadAssets` | model3.json via `cachedFetchArrayBuffer` (git HEAD had a `?t=${Date.now()}` cache-buster; gone). |
| A6 | 15699–15712 `LAppModel.setupModel` | Parallel prefetch of moc3, physics3.json and every texture as soon as model3.json is parsed. |
| A7 | 15715–15721, 15735–15739, 15773–15777 | moc3 / expression / physics loads use `cachedFetchArrayBuffer(..).catch(() => { CubismLogError(..); return new ArrayBuffer(0); })`. (Pose, user data and motions still use plain `fetch` with the SDK's `response.ok` handling.) |
| A8 | 16490–16496 `LAppLive2DManager.onUpdate` | **Second core update** `model.getModel().update()` right after the parameter driver. |
| A9 | 16497–16505 | **Mobile vs desktop projection**: `width < height` → `scaleRelative(3.3, 3.3); translateY(-2.1)`, else `scaleRelative(4.1, 4.1); translateY(-2.75)` (ref had only the desktop pair). |
| A10 | 16599–16660 `LAppTextureManager.createTextureFromPngFile` | Cache hit calls back synchronously (ref re-created an `Image` and waited for `load`); image comes from `cachedLoadImage`; returns silently if the GL manager/context is gone; errors via promise `.catch`. |
| A11 | 17857–17871 | `EMOTION_TARGETS.SLEEPING` added. |
| A12 | 17877–18010 | `qBound`, `noise01`, `noiseSigned`, `drift`, `MOTION_BURSTS`, `getEmotionIdleMotion` added. |
| A13 | 18034–18074 | `findExpressionName` added. |
| A14 | 18089–18118 | ~30 new player state fields (bursts, wake-up, sneeze, nod, gaze, blink FSM). |
| A15 | 18131–18166 | `setEmotion` rewritten (trim, no-op on same tag, wake-up, burst trigger); `setSpokenChar`, `setEyeTracking` added. |
| A16 | 18205–18213 | `tryInit` falls back to `host.clientWidth/Height`; **first `tryInit()` is synchronous** (ref waited one rAF). |
| A17 | 18263–18264 | `ParamEyeBallX/Y` handles added. |
| A18 | 18285–18602 | `applyParameters` rewritten: real `performance.now()` delta (cap 0.1 s) instead of fixed 1/60, expression hook, sneeze, sleep nod, wake-up blend, frame-rate-independent smoothing, bursts, blink FSM, gaze, emotion-dependent breath, vowel lip sync. |
| A19 | 18605–18642 | `preloadDefaultAssets()`, `runtime.preload`, and an automatic `void preloadDefaultAssets()` at script evaluation. |

#### B. Ref TS vs. pristine Cubism SDK 5-r.5 sample (customizations already present in April; inferred)

Inference basis: files whose mtime is later than the SDK extraction time (2026‑04‑10 19:19:46) are `lappdefine.ts`,
`lappglmanager.ts`, `lapplive2dmanager.ts`, `lappmodel.ts`, `lappsubdelegate.ts`, `lapptexturemanager.ts`,
`lappview.ts`; plus `[ReamaCubism]` / "Qt" markers and knowledge of the stock sample. Untouched (= stock):
`lapppal.ts`, `lappsprite.ts`, `touchmanager.ts`, `lappwavfilehandler.ts`; `lappdelegate.ts` and `main.ts` are not
bundled (the entry replaces them).

| # | File | Customization |
|---|---|---|
| B1 | lappdefine | `ResourcesPath = "/resources/models/AmadeusKurisu5.0/"`, `ShaderPath = "/resources/live2d/shaders/"`, `BackImageName = ""`, `GearImageName = ""`, `ModelDir = ["reama5.0"]`, new `ModelJsonFileName = "Live2D紅莉栖forSDK5.0.model3.json"`. |
| B2 | lappglmanager | WebGL**1** first, WebGL2 fallback, explicit context attributes, `console.error` instead of `alert`. Verbatim comment: `// WebGL2だとCubism sample shaderの初期化に失敗する環境があるため、` / `// Qt版と同じ安定挙動を優先してWebGL1を先に使う。` |
| B3 | lapplive2dmanager | `ParameterDriver` hook (`_parameterDriver`, `setParameterDriver`, called after `model.update()`), `getMainModel()`, `ModelJsonFileName` override in `changeScene`, upper-body zoom. Verbatim ref comment: `// Qt parity: zoom into upper body (head + chest)` / `// Qt renders Live2D at 3× parent size and clips to show model Y [0.231, 0.707]`. |
| B4 | lappmodel | `[ReamaCubism]` console logs in `loadAssets`/`setupTextures`, `.catch` with `console.error` in `loadAssets`. |
| B5 | lappsubdelegate | `gl.clearColor(0.0, 0.0, 0.0, 0.0)` — transparent canvas (stock: opaque black, alpha 1.0). |
| B6 | lapptexturemanager | `MAX_TEXTURE_SIZE` downscale through a 2D canvas; `[ReamaCubism]` error log. |
| B7 | lappview | `if (BackImageName)` / `if (GearImageName)` guards in `initializeSprite` (stock always loads both images). |

### 0.4 Dead code in V1 (present in the bundle, never reachable with this model/host)

| Code | Why dead | V2 policy |
|---|---|---|
| `TouchManager`, `LAppView.onTouches*`, `transformView*/Screen*`, `LAppSubdelegate.onPoint*/onTouchCancel` | nothing registers pointer/touch listeners (the canvas even gets `pointer-events:none` from the runtime; Live2DCanvas flips it to `auto` but adds no listener) | Do not port, or port as unused API. |
| `LAppSprite`, background/gear sprites | `BackImageName = GearImageName = ""` | Do not port. The sprite **shader program** is still created and bound every frame in V1 (§4.8, §7.4) — visually irrelevant; V2 may skip it. |
| `LAppLive2DManager.onTap/onDrag/nextScene/addModel/getMainModel`, `LAppModel.hitTest`, `setRandomExpression`, `setDragging` targets | no input wiring; model3.json has no `HitAreas` | Optional. Keep `CubismLookUpdater` + `CubismTargetPoint` (they run every frame, adding 0). |
| Motions (`preLoadMotionGroup`, `startMotion`, `startRandomMotion`) | model3.json has no `Motions` → `getMotionGroupCount() == 0`; `startRandomMotion("Idle", 1)` is *called every frame* but returns −1 immediately because `getMotionCount("Idle") == 0` | Port the call + the early return (cheap) or stub. 193 `.motion3.json`/`.anim` files on disk are never loaded. |
| Expressions (`loadCubismExpression` branch, `CubismExpressionUpdater`, `findExpressionName`/`setExpression` hook in the player) | model3.json has no `Expressions` → `_expressions.size == 0` (8 `.exp3.json` files exist on disk but are not referenced) | Port generically (latent path, §9.4) — becomes live if someone adds `Expressions` to model3.json. |
| `CubismEyeBlink` + `CubismEyeBlinkUpdater` | `Groups[EyeBlink].Ids` is `[]` → count 0 → not created | Port generically; inactive. The player has its own blink FSM. |
| `CubismLipSyncUpdater`, `LAppWavFileHandler` | `Groups[LipSync].Ids` is `[]`; no motion sounds → `start()` never called | Optional. |
| Pose, user data | not in model3.json | Port generically (cheap) or stub. |
| `LAppModel.reloadRenderer`, `hasMocConsistencyFromFile`, `releaseMotions/Expressions`, `LAppModel.release`, `LAppLive2DManager.release` (empty) | never called | Optional. |
| `CubismWebGLOffscreenManager` per-frame calls | model has 0 offscreens, no blend modes | Keep the calls (render spec) — no-ops here. |

### 0.5 Numeric conventions

* All JS arithmetic is IEEE‑754 **f64**. Rust: use `f64` for all player/driver/timer math.
* The Core's parameter array (`model.parameters.values`) is a `Float32Array`: every `setParameterValue*` write is
  rounded to **f32** after clamping (`0.515 → 0.514999986`). The player's own state is never read back from the model,
  so it stays f64.
* Matrices are `Float32Array(16)` (f32 storage, f64 arithmetic) — exact rules in the render spec §3.1–3.3.
* `Math.sin`: V8 uses an fdlibm port; Rust `f64::sin` on wasm32 uses compiler-builtins' libm (musl/FreeBSD msun,
  also fdlibm-derived). Expect equality to ~1 ulp; golden tests must compare with a tolerance (recommend
  `|a-b| <= 1e-9 * max(1, |a|)` for driver outputs, `1e-6` for f32 parameter values).
* Golden data in this spec was produced by running the **real deployed bundle + real Core + real model files** in
  Node (§12.3). Script directory: `/tmp/claude-0/-home-elvelt-admin-Elvelt-dsuv/1f3badd8-3ee9-4a13-9340-35fac0e7abf1/scratchpad/pipeline-probe/`
  (`pipeline.cjs`, `funcs.cjs`, `coreready.cjs`, `corelog.cjs`, `viewrelease.cjs`, `trace-A-*.json`). The scratchpad is ephemeral —
  the harness source is reproduced in Appendix A so it can be re-created.

---

## 1. Executive summary (the facts an implementer must not miss)

1. **One model, almost no SDK features.** model3.json lists only Moc, one Texture, Physics, DisplayInfo and two
   *empty* groups (`EyeBlink`, `LipSync` with `Ids: []`). No motions, expressions, pose, hit areas, layout or user
   data. Therefore the SDK part of each frame consists of exactly three updaters, sorted by execution order:
   **Look (400, adds 0) → Breath (500) → Physics (600)** (verified at runtime: `CubismLookUpdater:400,
   CubismBreathUpdater:500, CubismPhysicsUpdater:600`).
2. **Per-frame order** (§7): `loadParameters` (restore defaults) → (no motion) → `saveParameters` → Look → Breath →
   Physics → **core update #1** → **ReamaCubismPlayer.applyParameters overwrites 22 parameters** → **core update #2**
   → projection → draw. The renderer therefore sees the player's values plus the physics hair outputs.
3. **QUIRK (visible, must reproduce): hair physics never sees the player's head/body motion.** Physics reads
   `ParamAngleX/Z` and `ParamBodyAngleX/Z` *before* the driver runs, i.e. the SDK breath oscillation
   (`7.5·sin(2πt/6.5345)`, `5·sin(2πt/5.5345)`, `2·sin(2πt/15.5345)`, `0`), not the emotion-driven angles. Hair sways
   gently on its own rhythm (10-s golden run: HairFront/Side ∈ [−0.033, 0.027], HairBack ∈ [−0.044, 0.016]) and
   does not react to nods/sneezes/look-around.
4. **Two clocks.** SDK updaters use `LAppPal.deltaTime` = Δ`Date.now()`/1000 (ms resolution, **uncapped**). The
   player uses Δ`performance.now()`/1000 **capped at 0.1 s**, and its first call has Δ = 0.
5. **Projection** (§8): portrait canvas (`canvas.width < canvas.height`, device px) → `scaleRelative(3.3,3.3)`,
   `translateY(-2.1)`; otherwise → `scaleRelative(4.1,4.1)`, `translateY(-2.75)`. Final MVP (1920×1045):
   `tr[0]=3.12411451, tr[5]=5.73999977, tr[13]=-2.75` (upper body only).
6. **Canvas** backing store = `clientWidth·devicePixelRatio × clientHeight·devicePixelRatio` (truncated), re-evaluated
   on the next frame after a `ResizeObserver` callback. Desktop stage → 1920×1045 CSS px regardless of window size.
7. **Assets** are fetched once per page through module-level promise caches and are preloaded at script load and by
   `runtime.preload()`; model setup normally completes without network.
8. **Player** (§9): 12 emotion targets + unknown→NORMAL, 13 motion bursts (spring overshoot; special SLEEPING nod and
   SNEEZE curves), noise-based idle drift per emotion, own blink FSM (0.1 s close / 0.1 s open, 2.5–6.0 s interval),
   sneeze easter egg (every 45 s of idle NORMAL, p = 0.15), sleep nods (first after 30 s, then 20–40 s), 2.5 s
   startled wake-up with look-around, gaze from mouse, breath param, vowel-table lip sync.
9. **V1 bugs found** (details in §3.2, §10): `destroy()` always throws `TypeError` (sample `LAppView.release()` calls
   `this._gear.release()` on `null`) → framework ref-count leaks, model never released (Live2DCanvas catches it and
   force-loses the context); the runtime script throws `ReferenceError` if it executes before the Core script, and
   Live2DCanvas now loads both **in parallel**; the Core's WASM is ready only asynchronously after its script runs.
   WebGL context loss is never recovered.

---

## 2. Model and asset facts

### 2.1 `Live2D紅莉栖forSDK5.0.model3.json` (verbatim, 421 bytes, tabs)

```json
{
	"Version": 3,
	"FileReferences": {
		"Moc": "Live2D紅莉栖forSDK5.0.moc3",
		"Textures": [
			"Live2D紅莉栖forSDK5.0.8192/texture_00.png"
		],
		"Physics": "Live2D紅莉栖forSDK5.0.physics3.json",
		"DisplayInfo": "Live2D紅莉栖forSDK5.0.cdi3.json"
	},
	"Groups": [
		{
			"Target": "Parameter",
			"Name": "EyeBlink",
			"Ids": []
		},
		{
			"Target": "Parameter",
			"Name": "LipSync",
			"Ids": []
		}
	]
}
```

Accessor results the state machine sees (CubismModelSettingJson, B 3536–4098): `getModelFileName()` =
`"Live2D紅莉栖forSDK5.0.moc3"`, `getTextureCount()` = 1, `getTextureFileName(0)` =
`"Live2D紅莉栖forSDK5.0.8192/texture_00.png"`, `getPhysicsFileName()` = `"Live2D紅莉栖forSDK5.0.physics3.json"`,
`getPoseFileName()` = `""`, `getUserDataFile()` = `""`, `getExpressionCount()` = 0, `getMotionGroupCount()` = 0,
`getMotionCount(any)` = 0, `getHitAreasCount()` = 0, `getEyeBlinkParameterCount()` = 0 (group exists, Ids empty),
`getLipSyncParameterCount()` = 0, `getLayoutMap(map)` → `false`, map stays empty. `DisplayInfo` is never read by the
runtime.

### 2.2 Parameters (27, from the real Core; all `repeats = 0`, so every write is **clamped** to [min, max])

`CubismModel` constructor sets `_isOverriddenParameterRepeat = true` and `initialize()` fills
`ParameterRepeatData(false, false)` for every index → `isRepeat()` is always false → `getParameterClampValue`
(`CubismMath.clamp`) is used for every set/add.

| idx | Id | cdi3 Name | min | max | default | written by (per frame, in order) |
|---|---|---|---|---|---|---|
| 0 | `ParamAngleX` | 角度 X | -30 | 30 | 0 | Look(+0), Breath(+) → Physics input; **overwritten by driver** (head X) |
| 1 | `ParamAngleY` | 角度 Y | -30 | 30 | 0 | Look(+0), Breath(+); **overwritten by driver** (head Y) |
| 2 | `ParamAngleZ` | 角度 Z | -30 | 30 | 0 | Look(+0), Breath(+) → Physics input; **overwritten by driver** (head Z) |
| 3 | `ParamEyeLOpen` | 左目　開閉 | 0 | 1 | 1 | driver (eyeOpen × blink) |
| 4 | `ParamEyeLSmile` | 左目　笑顔 | 0 | 1 | 0 | driver |
| 5 | `ParamEyeROpen` | 右目　開閉 | 0 | 1 | 1 | driver (0 when target.isWink) |
| 6 | `ParamEyeRSmile` | 右目　笑顔 | 0 | 1 | 0 | driver |
| 7 | `ParamEyeBallX` | 目玉 X | -1 | 1 | 0 | Look(+0); **driver** (smoothed gaze X) |
| 8 | `ParamEyeBallY` | 目玉 Y | -1 | 1 | 0 | Look(+0); **driver** (smoothed gaze Y) |
| 9 | `ParamBrowLForm` | 左眉　変形 | -1 | 1 | 0 | driver |
| 10 | `ParamBrowLY` | 左眉　上下 | -1 | 1 | 0 | driver |
| 11 | `ParamBrowRAngle` | 右眉　角度 | -1 | 1 | 0 | driver |
| 12 | `ParamBrowLX` | 左眉　左右 | -1 | 1 | 0 | nobody (stays default via loadParameters) |
| 13 | `ParamBrowRX` | 右眉　左右 | -1 | 1 | 0 | nobody (stays default) |
| 14 | `ParamBrowLAngle` | 左眉　角度 | -1 | 1 | 0 | driver |
| 15 | `ParamBrowRY` | 右眉　上下 | -1 | 1 | 0 | driver |
| 16 | `ParamBrowRForm` | 右眉　変形 | -1 | 1 | 0 | driver |
| 17 | `ParamMouthForm` | 口　変形 | -1 | 1 | 0 | driver |
| 18 | `ParamMouthOpenY` | 口　開閉 | 0 | 1 | 0 | driver (lip sync) |
| 19 | `ParamCheek` | 照れ | 0 | 1 | 0 | driver |
| 20 | `ParamBodyAngleX` | 体の回転 X | -10 | 10 | 0 | Look(+0), Breath(+) → Physics input; **overwritten by driver** |
| 21 | `ParamBodyAngleY` | 体の回転 Y | -10 | 10 | 0 | driver |
| 22 | `ParamBodyAngleZ` | 体の回転 Z | -10 | 10 | 0 | Physics input (default value only); **overwritten by driver** |
| 23 | `ParamBreath` | 呼吸 | 0 | 1 | 0 | Breath(+); **overwritten by driver** |
| 24 | `ParamHairFront` | 髪揺れ　前 | -1 | 1 | 0 | Physics output (PhysicsSetting1) |
| 25 | `ParamHairSide` | 髪揺れ　横 | -1 | 1 | 0 | Physics output (PhysicsSetting2) |
| 26 | `ParamHairBack` | 髪揺れ　後 | -1 | 1 | 0 | Physics output (PhysicsSetting3) |

Model-level facts (Core probe): MocVersion 5 (`MOC3` header byte `05`), canvas 7000×10000 px, origin (3500, 5000),
PixelsPerUnit 7000 → `getCanvasWidth()` = 1.0, `getCanvasHeight()` = 1.4285714285714286; 49 drawables, 15 parts,
0 offscreens; `isBlendModeEnabled()` = false; masks are used (6 clipping contexts) — renderer details in the render spec.

### 2.3 Physics (`Live2D紅莉栖forSDK5.0.physics3.json`, summary of the full file)

* `Meta.Fps = 60` → fixed physics step 1/60 s with output interpolation (`CubismPhysics.evaluate`, B 7998–8138);
  `MaxDeltaTime = 5` s (if the accumulated remainder exceeds 5 s it is reset to 0). `EffectiveForces`: gravity
  (0, −1), wind (0, 0). `CubismPhysics.create` sets `_physicsRig.gravity.y = 0` after parsing (SDK behaviour);
  `_options.gravity = (0, −1)`, `_options.wind = (0, 0)`.
* 3 settings (`PhysicsSetting1` 前髪揺れ, `PhysicsSetting2` 横髪揺れ, `PhysicsSetting3` 後髪揺れ); **identical inputs**:
  `ParamAngleX` (Weight 60, Type X), `ParamAngleZ` (60, Angle), `ParamBodyAngleX` (40, X), `ParamBodyAngleZ` (40,
  Angle), all `Reflect: false`. Outputs: `ParamHairFront` / `ParamHairSide` / `ParamHairBack` (VertexIndex 1, Scale 1,
  Weight 100, Type Angle). Particles (Position Y / Mobility / Delay / Acceleration / Radius; X is always 0):
  settings 1 and 2: `(0, 1, 1, 1, 0)`, `(15, 0.95, 0.8, 1.5, 15)`; setting 3: `(0, 1, 1, 1, 0)`, `(10, 0.85, 0.9, 1, 10)`,
  `(20, 0.9, 0.9, 1, 10)`, `(28, 0.9, 0.9, 0.8, 8)`. Normalization (Position and Angle): min −10, default 0, max 10.
* Consequence: settings 1 and 2 produce **identical** outputs every frame (same inputs, same particles) — visible in
  the golden trace (`HairFront == HairSide`).
* `CubismPhysics.stabilization()` is **never called** in V1 (no call site in the bundle) → hair starts from rest at
  frame 0 and settles over the first second.

### 2.4 Files on disk that are never loaded

`expressions/` (Angry, Blushing, f01, f02, Normal, Sad, Smile, Surprised `.exp3.json`) and `motions/` (193 files,
`*.motion3.json` + `*.anim`) are not referenced by model3.json. **Do not add them in V2** without a product decision —
adding `Expressions` would activate the latent expression hook (§9.4) and an `CubismExpressionUpdater` at order 300,
changing the face.

### 2.5 Texture

`Live2D紅莉栖forSDK5.0.8192/texture_00.png`: **4096×4096**, 8-bit RGBA PNG, 1,843,918 bytes (working tree; git HEAD
had an 8192×8192, 11,866,666-byte version — the folder name still says 8192). Uploaded with premultiplied alpha and
mipmaps (§6.5): ≈ 85 MB of GPU memory with the mip chain.

### 2.6 Model matrix

`CubismUserModel.loadModel` → `new CubismModelMatrix(1.0, 1.4285714285714286)` → constructor calls `setHeight(2.0)`
→ `scale(1.4, 1.4)` (stored f32 `1.399999976158142`), translation (0, 0). No `Layout` → `setupFromLayout(empty)` is a
no-op. It is never changed afterwards (the portrait `setWidth(2)` branch in §8.2 is not taken because
`getCanvasWidth()` is not `> 1`).

### 2.7 `lappdefine` constants present in the bundle (B 2666–2697)

```js
var CanvasSize = "auto";
var ViewScale = 1;
var ViewMaxScale = 2;
var ViewMinScale = 0.8;
var ViewLogicalLeft = -1;
var ViewLogicalRight = 1;
var ViewLogicalMaxLeft = -2;
var ViewLogicalMaxRight = 2;
var ViewLogicalMaxBottom = -2;
var ViewLogicalMaxTop = 2;
var ResourcesPath = "/resources/models/AmadeusKurisu5.0/";
var ShaderPath = "/resources/live2d/shaders/";
var BackImageName = "";
var GearImageName = "";
var ModelDir = [
  "reama5.0"
];
var ModelDirSize = ModelDir.length;
var ModelJsonFileName = "Live2D紅莉栖forSDK5.0.model3.json";   // "Live2D紅莉栖forSDK5.0.model3.json"
var MotionGroupIdle = "Idle";
var MotionGroupTapBody = "TapBody";
var HitAreaNameHead = "Head";
var HitAreaNameBody = "Body";
var PriorityNone = 0;
var PriorityIdle = 1;
var PriorityNormal = 2;
var PriorityForce = 3;
var MOCConsistencyValidationEnable = false;
var MotionConsistencyValidationEnable = false;
var DebugLogEnable = false;
var DebugTouchLogEnable = false;
var CubismLoggingLevel = 0 /* LogLevel_Verbose */;   // unused: the player starts the framework with LogLevel_Off
```

Tree-shaken from the ref TS (unused): `CanvasNum = 1`, `ViewLogicalBottom = -1`, `ViewLogicalTop = 1`,
`PowerImageName = 'CloseNormal.png'`, `RenderTargetWidth = 1920`, `RenderTargetHeight = 1080`.

Derived URLs (all same-origin, absolute from the web root; the browser percent-encodes the UTF‑8 characters):

| Asset | URL string passed to `fetch`/`Image.src` |
|---|---|
| model3.json | `/resources/models/AmadeusKurisu5.0/reama5.0/Live2D紅莉栖forSDK5.0.model3.json` |
| moc3 | `/resources/models/AmadeusKurisu5.0/reama5.0/Live2D紅莉栖forSDK5.0.moc3` |
| physics3.json | `/resources/models/AmadeusKurisu5.0/reama5.0/Live2D紅莉栖forSDK5.0.physics3.json` |
| texture | `/resources/models/AmadeusKurisu5.0/reama5.0/Live2D紅莉栖forSDK5.0.8192/texture_00.png` |
| shaders (13) | `/resources/live2d/shaders/` + `vertshadersrc.vert`, `vertshadersrcmasked.vert`, `vertshadersrcsetupmask.vert`, `fragshadersrcsetupmask.frag`, `fragshadersrcpremultipliedalpha.frag`, `fragshadersrcmaskpremultipliedalpha.frag`, `fragshadersrcmaskinvertedpremultipliedalpha.frag`, `vertshadersrccopy.vert`, `fragshadersrccopy.frag`, `fragshadersrccolorblend.frag`, `fragshadersrcalphablend.frag`, `vertshadersrcblend.vert`, `fragshadersrcpremultipliedalphablend.frag` |
| Core script (host page) | `/resources/live2d/live2dcubismcore.min.js` |
| Runtime script (host page) | `/resources/live2d/reama-cubism-runtime.js?v=19` (replaced by the WASM in V2) |

Server note (owned by the server spec): V1 serves `/resources/:path*` with
`Cache-Control: public, max-age=86400, stale-while-revalidate=604800` (next.config.ts); the V2 axum static handler and
the Tauri asset resolver must percent-decode UTF‑8 paths (`%E7%B4%85%E8%8E%89%E6%A0%96` = 紅莉栖).

---

## 3. Runtime API and host-page contract

### 3.1 Global API (B 18634–18642)

```js
var runtime = {
  create(host) { return new ReamaCubismPlayer(host); },
  preload: preloadDefaultAssets
};
var runtimeWindow = window;
runtimeWindow.ReamaCubismRuntime = runtime;
void preloadDefaultAssets();   // runs once when the script is evaluated
```

TypeScript view used by the host (Live2DCanvas.tsx):

```ts
type CubismPlayer = {
  setEmotion: (emotion: string) => void;
  setLipSync: (value: number) => void;
  setSpokenChar?: (char: string) => void;
  setEyeTracking?: (x: number, y: number) => void;
  destroy: () => void;
};
type CubismRuntime = { create: (host: HTMLElement) => CubismPlayer; preload?: () => Promise<void>; };
```

* `create(host)` synchronously constructs a player; it may synchronously create the canvas + WebGL context, start
  the framework, begin model loading and run the first frame (§4). Returns the player even if initialization failed
  (failure is only logged).
* `preload()` returns `Promise<void>` that always resolves (every task has `.catch(() => {})`), §6.2.
* Player methods: `setEmotion(string)`, `setLipSync(number)`, `setSpokenChar(string)`, `setEyeTracking(x, y)`,
  `destroy()` (§9.6, §10). Other members (`applyParameters`, `tick`, …) are internal.

### 3.2 Script / Core loading requirements (critical for V2)

1. **The runtime needs the `Live2DCubismCore` namespace at evaluation time.** B 9358–9386 builds the
   `CubismColorBlend`/`CubismAlphaBlend` enums at top level from `Live2DCubismCore.ColorBlendType_*` /
   `AlphaBlendType_*`. Evaluated without the Core → `ReferenceError: Live2DCubismCore is not defined` → IIFE aborts →
   `window.ReamaCubismRuntime` stays `undefined` (verified in Node, `viewrelease.cjs`).
2. **The Core is not callable synchronously after its script runs.** It instantiates an embedded base64 WASM
   asynchronously (Emscripten, `instantiateArrayBuffer` for data URIs). Verified: `Live2DCubismCore.Version.csmGetVersion()`
   throws `TypeError: Cannot read properties of undefined (reading 'apply')` synchronously and in a microtask, and
   succeeds after one macrotask (`setTimeout 0`) in Node. Namespace constants (`MocVersion_*`, `ColorBlendType_*`,
   `AlphaBlendType_*`, `ParameterType_*`) and classes exist synchronously. Public namespace keys:
   `AlignofMoc, AlignofModel, MocVersion_Unknown, MocVersion_30, MocVersion_33, MocVersion_40, MocVersion_42,
   MocVersion_50, MocVersion_53, ParameterType_Normal, ParameterType_BlendShape, ColorBlendType_* (17),
   AlphaBlendType_Over, AlphaBlendType_Atop, AlphaBlendType_Out, AlphaBlendType_ConjointOver,
   AlphaBlendType_DisjointOver, Version, Logging, Moc, Model, CanvasInfo, Parameters, Parts, Drawables, Offscreens,
   Utils, Memory`. No public "ready" promise exists.
3. **V1 relies on timing** for (2): the first Core calls (`csmSetLogFunction`, then `csmGetVersion`, in
   `CubismFramework.startUp`) happen only
   inside `create()`, which runs after both scripts' `onload` + React effects — normally later than the WASM
   instantiation. And since the working-tree Live2DCanvas loads the two scripts **in parallel** with `async=true`,
   (1) is a real race: if the 670 KB runtime finishes before the 228 KB Core, Live2D silently never appears for the
   page lifetime (the memoized load promise has already resolved). **QUIRK — V2: fix** (§3.5).

### 3.3 Startup readiness rule for V2 (normative)

`amadeus-live2d` must expose `async fn ensure_core_ready()` that (a) injects
`<script src="/resources/live2d/live2dcubismcore.min.js" data-core="true" async>` once (reuse an existing tag; same
marker attribute as V1 so a mixed page never loads it twice), (b) awaits its `load` event (reject on `error`), then
(c) polls `Live2DCubismCore.Version.csmGetVersion()` inside `catch`-wrapped JS (or `js_sys::Reflect` +
`Function::call0` in Rust with `Result`) once per `setTimeout(0)`/animation frame until it returns a number (cap e.g.
10 s → error). All Core calls (moc creation, `CubismFramework::start_up`) must happen after this. The WASM player itself
replaces `reama-cubism-runtime.js`, so there is no second script and race (1) disappears.

### 3.4 Host component contract (`components/amadeus/Live2DCanvas.tsx`)

Full React lifecycle is specified in `spec-lib-media.md` §6 — this is the part that defines the player API usage.

#### 3.4.1 Props and how ChatPanel feeds them (RealAmadeusApp.tsx L1865–1870)

```tsx
<Live2DCanvas
  emotion={miniGameOpen ? miniGameEmotion : currentEmotionTag}
  lipSyncValue={miniGameOpen ? miniSpeak : isActivelySpeaking ? 1 : lipSyncValue}
  spokenChar={miniGameOpen ? "" : isActivelySpeaking ? currentSpokenChar : ""}
  eyeTrackingEnabled={eyeTrackingEnabled}            // useAppSettingInt("Config_EyeTracking", 0) === 1
/>
```

Emotion strings seen in practice: `NORMAL, SMILE, ANGRY, SAD, SURPRISED, BLUSH, WINK, DISGUST, SMUG, THINKING,
PANIC, SLEEPING` (ChatPanel sets `SLEEPING` after 60 s idle and resets to `NORMAL` on activity, L842–905), and any
string typed via `/dev emotion <x>` (uppercased; the help text lists `SNEEZE (クシャミ)` as a supported "motion ID").
`currentSpokenChar` is the character the typewriter just revealed (L1155–1156), `""` at pauses/end.

#### 3.4.2 Behaviour (verbatim logic)

* Script URLs: `CORE_SCRIPT_SRC = "/resources/live2d/live2dcubismcore.min.js"`,
  `RUNTIME_SCRIPT_SRC = "/resources/live2d/reama-cubism-runtime.js?v=19"`.
* `loadScript(src, marker)`: if `script[data-${marker}="true"]` exists → resolve if it has `data-loaded="true"`, else
  wait for its `load`/`error` (once). Otherwise create `<script src async data-${marker}="true">`, `onload` sets
  `data-loaded="true"` and resolves, `onerror` rejects `new Error(\`Failed to load ${src}\`)`; append to `<head>`.
  (QUIRK: an existing tag that already failed never settles.)
* `ensureLive2DScriptsLoaded()`: SSR guard; memoized `Promise.all([loadScript(CORE,"core"), loadScript(RUNTIME,"runtime")])`;
  on rejection the memo is cleared and the error rethrown.
* `preloadLive2D()`: `ensure…().then(() => window.ReamaCubismRuntime?.preload?.()).catch(err => console.warn("Live2D preload warning:", err))`;
  called once from the root component mount (RealAmadeusApp L3974) while the user is still on login/boot screens.
* Mount: `await ensure…()` (catch → `console.error("Cubism init failed", e)`, return) → if unmounted return →
  `rt = window.ReamaCubismRuntime`; if missing return silently → `p = rt.create(host)` → `p.setEmotion(emotionRef)`
  (ref = `emotion.toUpperCase()`, no trim), `p.setLipSync(lipSyncRef)`, `p.setSpokenChar?.(spokenCharRef)` → find
  `host.querySelector("canvas")` and set inline `pointerEvents="auto"`, `width="100%"`, `height="100%"`; if no canvas
  yet, retry once after 100 ms. Eye tracking is **not** pushed at creation.
* Prop changes: `setEmotion(emotion.toUpperCase())`, `setLipSync(value)`, `setSpokenChar(spokenChar || "")` when the
  player exists (refs updated otherwise).
* Eye tracking: when enabled, a `window` `mousemove` listener sends
  `x = clamp(((clientX / innerWidth) - 0.5) * 2, -0.7, 0.7)`, `y = -clamp(((clientY / innerHeight) - 0.5) * 2, -0.7, 0.7)`;
  when disabled (and on every toggle to disabled) → `setEyeTracking(0, 0)`.
* Unmount: grab `canvas.getContext("webgl") || canvas.getContext("webgl2")` (returns the existing context) **before**
  destroying, `try { player.destroy() } catch (e) { console.warn("Live2D destroy error:", e) }`, then
  `gl.getExtension("WEBGL_lose_context")?.loseContext()`, then `host.innerHTML = ""`.
* Renders `<div class="live2d-host"></div>`.

#### 3.4.3 Canvas/CSS contract

The runtime creates `<canvas>` with inline `display:block; width:100%; height:100%; pointer-events:none` and appends it
to the host after `host.innerHTML = ""`. CSS: `.live2d-host{position:absolute;inset:0;z-index:0}`,
`.live2d-host canvas{position:absolute;left:50%;top:50%;width:3840px;height:2160px;transform:translate(-50%,-50%);pointer-events:auto}`
— the inline 100% wins, so the canvas exactly covers the host. Desktop: `.live2d-host` fills `.live2d-shift` (abs,
inset 0), which fills `.chat-panel-root` (abs, `inset:35px 0 0 0`) inside the 1920×1080 `.viewport` (scaled with `transform: scale(uiScale)`,
which does not change layout size) → **canvas client size is always 1920×1045 CSS px on desktop**. Mobile:
`.mobile-root .live2d-host{position:fixed;inset:0;z-index:0}`, canvas `inset:0; width/height:100% !important;
transform:none`. The minigame shift (`.live2d-shift.live2d-shifted{transform:translateX(28%)}`, mobile
`translateY(-14dvh) scale(0.85)`) moves the canvas visually without resizing it.

#### 3.4.4 Corrections to `spec-lib-media.md` §6

* §6.6 describes `destroy()` as ending with `host.innerHTML=""; releaseFramework()`. In V1 those steps are never
  reached when a subdelegate exists: `destroy()` throws inside `subdelegate.release()` (§10.2) and the host is emptied by
  Live2DCanvas instead. Externally invisible except for the console warning and the ref-count leak.
* The race in (3.2‑1) is not only "latent": the runtime script cannot evaluate at all before the Core.

### 3.5 V2 replacement (normative)

`amadeus-ui`'s `Live2DCanvas` component calls `amadeus_live2d::Live2dRuntime::create(host_element)` after
`ensure_core_ready().await`; `preload_live2d()` = `ensure_core_ready()` (optional) + `amadeus_live2d::preload()`
(asset caches, §6.2). Keep the same prop semantics, the eye-tracking mapping, the inline canvas styles, the unmount
order (capture context → destroy → `WEBGL_lose_context.loseContext()` → clear host) and the `console.warn` texts.

---

## 4. Initialization sequence (exact order)

### 4.1 `new ReamaCubismPlayer(host)` (B 18076–18130)

Field initialization, in this order (order matters only for the single `Math.random()` draw):

```
subdelegate = null; rafHandle = null; parameterHandles = null;
current = { ...EMOTION_TARGETS.NORMAL };           // 13 numeric fields, no isWink
userTime = 0; mouthOpen = 0; emotion = "NORMAL"; lipSyncValue = 0;
destroyed = false; frameworkAcquired = false; initRetryHandle = null;
wakeUpTimer = 0; wakeUpHeadX = 0; wakeUpGazeX = 0; sneezeEasterEggTimer = 0;
burstProgress = 1.0; burstTimer = 0; activeBurstTag = "";
sleepingNodTimer = 0; nextSleepingNodInterval = 30.0; spokenChar = "";
currentEmotionTag = "NORMAL"; idlePhase = 0;
smoothedBodyX = smoothedBodyY = smoothedBodyZ = 0; smoothedHeadX = smoothedHeadY = smoothedHeadZ = 0;
lastAppliedEmotionTag = ""; eyeTrackingX = 0.0; eyeTrackingY = 0.0; smoothedGazeX = 0.0; smoothedGazeY = 0.0;
blinkState = 0; blinkTimer = 0.0;
blinkInterval = 3.0 + Math.random() * 3.5;          // first RNG draw; source comment says "Random 2.5-6.0s" (wrong: 3.0–6.5)
blinkDuration = 0.1;
tick = () => { … }                                 // §7.1
host = host;
beginInitialize();
// _lastUpdateTime is NOT initialized (undefined) until the first applyParameters()
```

### 4.2 `beginInitialize()` (B 18190–18214)

1. If `window.Live2DCubismCore` is falsy → `console.error("[ReamaCubism] Live2DCubismCore is not loaded.")`; return
   (the player stays inert forever; `destroy()` still works).
2. `canvas = document.createElement("canvas")`; `canvas.style.display = "block"`, `.width = "100%"`, `.height = "100%"`,
   `.pointerEvents = "none"`.
3. `host.innerHTML = ""`; `host.appendChild(canvas)`.
4. `tryInit = () => { if (destroyed) return; w = canvas.clientWidth || host.clientWidth; h = canvas.clientHeight || host.clientHeight;
   if (w > 0 && h > 0) initSubdelegate(canvas); else initRetryHandle = requestAnimationFrame(tryInit); }`
5. **Call `tryInit()` synchronously** (reading `clientWidth` forces layout). Retries every animation frame with no limit
   until the size is non-zero or the player is destroyed.

### 4.3 `initSubdelegate(canvas)` (B 18215–18236)

1. `initRetryHandle = null`.
2. `acquireFramework()`; `frameworkAcquired = true`.
3. `console.log(\`[ReamaCubism] Initializing. Canvas client size: ${canvas.clientWidth}x${canvas.clientHeight}\`)`.
4. `next = new LAppSubdelegate()`; if `!next.initialize(canvas)` →
   `console.error("[ReamaCubism] Failed to initialize Cubism subdelegate.")`, `host.innerHTML = ""`,
   `releaseFramework()`, `frameworkAcquired = false`, return.
5. `gl = next.getGl()`; if truthy: `console.log(\`[ReamaCubism] WebGL ready. Max texture size: ${gl.getParameter(gl.MAX_TEXTURE_SIZE)}, Canvas buffer: ${gl.drawingBufferWidth}x${gl.drawingBufferHeight}\`)`.
6. `next.getLive2DManager().setParameterDriver((model) => this.applyParameters(model))`.
7. `this.subdelegate = next`.
8. `this.tick()` — **first frame runs synchronously inside `create()`** (the model is still loading, so it only clears
   the canvas).

### 4.4 Framework start-up and ref-count (B 18011–18033, B 2479–2564)

```js
var frameworkRefCount = 0;                       // module-level, shared by all players on the page
function acquireFramework() {
  if (frameworkRefCount === 0) {
    const option = new Option();
    option.loggingLevel = 5 /* LogLevel_Off */;
    CubismFramework.startUp(option);
    CubismFramework.initialize();
  }
  frameworkRefCount += 1;
}
function releaseFramework() {
  if (frameworkRefCount <= 0) return;
  frameworkRefCount -= 1;
  if (frameworkRefCount === 0) {
    if (CubismFramework.isInitialized()) CubismFramework.dispose();
    if (CubismFramework.isStarted()) CubismFramework.cleanUp();
  }
}
```

* `CubismFramework.startUp(option)`: if already started → (suppressed) info log, return true. Else `s_option = option`;
  `Live2DCubismCore.Logging.csmSetLogFunction(option.logFunction)` — **`logFunction` is never set → `undefined`**.
  This is the first real Core call (`_em.addFunction(Logging.wrapLogFunction, "vi")` + `ccall("csmSetLogFunction")`):
  the Core now routes its own log messages to `Logging.logFunction(message)` = `undefined(message)`. **QUIRK (latent
  crash):** any Core-side log — emitted only on errors, e.g. loading an invalid moc logs
  `[CSM] [E]csmReviveMocInPlace: "size" is invalid.` — throws `TypeError: Logging.logFunction is not a function` out of
  the Core call (verified, `corelog.cjs`). The Core's version banner `Live2D Cubism SDK Core Version 6.0.1` is printed
  with plain `console.log` on the first moc load regardless. Then `s_isStarted = true`; `csmGetVersion()` for a
  suppressed log. Returns true. **V2: fix** — always register a log function (no-op or forwarding to
  `log::debug!`/`console.debug`); only error paths change (no exception).
* `CubismFramework.initialize(memorySize = 0)`: requires started; if already initialized → (suppressed) warning,
  return. Else `Value.staticInitializeNotForClientCall()`, `s_cubismIdManager = new CubismIdManager()`,
  `Live2DCubismCore.Memory.initializeAmountOfMemory(0)` (Core default heap), `s_isInitialized = true`.
* `dispose()`: `Value.staticReleaseNotForClientCall()`, id manager release, `CubismRenderer.staticRelease()` →
  `CubismShaderManager_WebGL.deleteInstance()` (frees all per-context shader programs), `s_isInitialized = false`.
  `cleanUp()`: `s_isStarted = s_isInitialized = false; s_option = s_cubismIdManager = null`.
* Log level: `CubismFramework.getLoggingLevel()` returns `5` (Off) → every `CubismLog*` call is dropped
  (`CubismDebug.print` returns when `logLevel < 5`; all levels are 0–4). `CSM_ASSERT(expr)` is `console.assert(expr)`
  and does print on failure (none fire in normal operation).
* **V1 reality:** because `destroy()` throws before `releaseFramework()` (§10.2), the ref-count never returns to 0
  after the first successful init; the framework stays started/initialized for the page lifetime and the shader
  manager keeps one entry per (lost) GL context. **V2: fix** — release properly; behaviour is otherwise identical.

### 4.5 `LAppSubdelegate` (B 17477–17539)

Constructor: `_canvas = null; _glManager = new LAppGlManager(); _textureManager = new LAppTextureManager();
_live2dManager = new LAppLive2DManager(); _view = new LAppView(); _frameBuffer = null; _captured = false;`
(`_needResize` and `_resizeObserver` are undefined until set.)

`initialize(canvas)` → returns bool:

1. `if (!_glManager.initialize(canvas)) return false;` (§4.6)
2. `_canvas = canvas`.
3. `CanvasSize === "auto"` → `resizeCanvas()` (§5.1). (Fixed-size branch unused.)
4. `_textureManager.setGlManager(_glManager)`.
5. `gl = _glManager.getGl()`; if `!_frameBuffer` → `_frameBuffer = gl.getParameter(gl.FRAMEBUFFER_BINDING)` → `null`
   (default framebuffer).
6. `gl.enable(gl.BLEND); gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA)`.
7. `_view.initialize(this)` (§8.1).
8. `_live2dManager.setOffscreenSize(canvas.width, canvas.height)` → no models yet → no-op.
9. `_view.initializeSprite()` (§4.7).
10. `_live2dManager.initialize(this)` → `_subdelegate = this; changeScene(0)` (§4.8) → model loading starts.
11. `_resizeObserver = new ResizeObserver((entries, observer) => this.resizeObserverCallback(entries, observer))`;
    `_resizeObserver.observe(_canvas)`.
12. `return true`.

### 4.6 `LAppGlManager.initialize(canvas)` (B 2772–2789)

```js
const contextAttributes = { alpha: true, antialias: true, premultipliedAlpha: true, preserveDrawingBuffer: false };
this._gl = canvas.getContext("webgl", contextAttributes);
if (!this._gl) this._gl = canvas.getContext("webgl2", contextAttributes);
if (!this._gl) { console.error("Cannot initialize WebGL context."); this._gl = null; return false; }
return true;
```

`depth` (default true), `stencil` (default false), `failIfMajorPerformanceCaveat`, `powerPreference` are left at
defaults. `release()` is empty (the context is only freed by Live2DCanvas's `loseContext()`). WebGL1-first decision
and its consequences: render spec §2. **V2: identical attributes, WebGL1 first, WebGL2 fallback.**

### 4.7 `LAppView` construction, `initializeSprite()`, sprite program (B 17279–17392, 17580–17606)

Constructor: `_programId = null; _back = null; _gear = null; _touchManager = new TouchManager();
_deviceToScreen = new CubismMatrix44(); _viewMatrix = new CubismViewMatrix();`

`initializeSprite()`: background/gear branches skipped (empty names). If `_programId == null` →
`_programId = subdelegate.createShader()`:

* compiles vertex source (verbatim, one line):
  `precision mediump float;attribute vec3 position;attribute vec2 uv;varying vec2 vuv;void main(void){   gl_Position = vec4(position, 1.0);   vuv = uv;}`
* fragment source: `precision mediump float;varying vec2 vuv;uniform sampler2D texture;void main(void){   gl_FragColor = texture2D(texture, vuv);}`
* `createProgram`, attach both, **delete both shaders before linking**, `linkProgram`, `useProgram(program)`; no
  status checks; on `createShader` returning null → `LAppPal.printMessage("failed to create vertexShader")` /
  `"failed to create fragmentShader"` (console.log) and returns null.

The program is bound every frame (§7.4) but never draws. **V2: may skip entirely** (no pixels depend on it; the
renderer binds its own programs).

### 4.8 `LAppLive2DManager` (B 16390–16571)

Constructor: `beganMotion` / `finishedMotion` callbacks (print `"Motion Began:"` / `"Motion Finished:"` + object —
dead), `_subdelegate = null`, `_viewMatrix = new CubismMatrix44()` (identity), `_models = []`, `_sceneIndex = 0`,
`_parameterDriver = null`.

`changeScene(index)`: `_sceneIndex = index`; `model = ModelDir[index]` (`"reama5.0"`);
`modelPath = ResourcesPath + model + "/"`; `modelJsonName = \`${ModelDir[index]}.model3.json\`` then overridden by
`ModelJsonFileName` (non-empty); `releaseAllModel()` (= `_models.length = 0`; old models are **not** released);
`instance = new LAppModel(); instance.setSubdelegate(_subdelegate); instance.loadAssets(modelPath, modelJsonName);
_models.push(instance)`.

`setParameterDriver(fn | null)`; `setViewMatrix(m)` copies 16 floats into `_viewMatrix`; `setOffscreenSize(w, h)` →
`model?.setRenderTargetSize(w, h)` for each model (→ renderer `_modelRenderTargetWidth/Height`, only used by 5.3
blend offscreens); `release()` is **empty**.

### 4.9 `LAppModel` construction (B 16342–16381 + `CubismUserModel` B 14906–14931)

CubismUserModel: `_moc = _model = null; _motionManager = new CubismMotionManager()` (event callback → model);
`_expressionManager = new CubismExpressionMotionManager(); _eyeBlink = _breath = _modelMatrix = _pose = null;
_dragManager = new CubismTargetPoint(); _physics = _modelUserData = null; _initialized = _updating = false;
_opacity = 1; _mocConsistency = false; _debugMode = false; _renderer = null`.

LAppModel: `_modelSetting = null; _modelHomeDir = null; _userTimeSeconds = 0; _eyeBlinkIds = []; _lipSyncIds = [];
_motions = new Map(); _expressions = new Map(); _hitArea = []; _userArea = [];
_idParamAngleX/Y/Z, _idParamBodyAngleX = idManager.getId("ParamAngleX"/"ParamAngleY"/"ParamAngleZ"/"ParamBodyAngleX")`
(requires the framework to be initialized — hence `acquireFramework()` first);
`_mocConsistency` stays false, `_motionConsistency` stays undefined (both flags false); `_state = 0 (LoadAssets)`;
`_expressionCount = _textureCount = _motionCount = _allMotionCount = 0`; `_wavFileHandler = new LAppWavFileHandler()`;
`_consistency = false; _look = null; _updateScheduler = new CubismUpdateScheduler(); _motionUpdated = false`.

---

## 5. Canvas sizing, devicePixelRatio, resize

### 5.1 `resizeCanvas()` (B 17628–17633)

```js
this._canvas.width  = this._canvas.clientWidth  * window.devicePixelRatio;
this._canvas.height = this._canvas.clientHeight * window.devicePixelRatio;
const gl = this._glManager.getGl();
gl.viewport(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight);
```

* Assigning a fractional number to `canvas.width/height` truncates toward zero (WebIDL `unsigned long`). Rust:
  `(client as f64 * dpr) as u32` (truncating, saturating). Example: 1045 × 1.25 = 1306.25 → 1306.
* Uses `canvas.clientWidth` only (not the host fallback used by `tryInit`), so in the corner case "canvas 0, host >0"
  the backing store becomes 0×0 until the next resize notification.
* Desktop examples: DPR 1 → 1920×1045; DPR 1.25 → 2400×1306; DPR 2 → 3840×2090 (independent of the window size,
  because the stage is scaled by CSS transform). Mobile 390×844 CSS @DPR 3 → 1170×2532.
* The viewport set here uses `drawingBufferWidth/Height`; every draw later sets `viewport(0, 0, canvas.width,
  canvas.height)` (§7.5 step 9) — they differ only if the browser clamps the drawing buffer (huge canvases). Reproduce.

### 5.2 Resize detection (B 17541–17566)

* `ResizeObserver` on the canvas → `resizeObserverCallback()` sets `_needResize = true` (only when
  `CanvasSize === "auto"`). The callback also fires once right after `observe()` (spec: initial observation for a
  rendered, non-zero element) → one redundant `onResize()` on an early frame.
* Applied lazily at the start of the next `LAppSubdelegate.update()` (§7.3): `onResize()` = `resizeCanvas()`;
  `_view.initialize(this)`; `_live2dManager.setOffscreenSize(canvas.width, canvas.height)`; then `_needResize = false`.
* The projection is recomputed every frame from `canvas.width/height`, so a resize takes effect on the same frame.
* **Not handled:** a `devicePixelRatio` change without a CSS size change (dragging the window to another monitor)
  does not fire `ResizeObserver` → the backing store keeps the old DPR until the next real resize. Browser zoom changes
  CSS sizes and therefore does trigger it. **V2: reproduce** (optionally also listen to
  `matchMedia("(resolution: Xdppx)")` — a documented, invisible-in-practice improvement).
* CSS transforms (menu shrink, minigame shift, desktop `scale(uiScale)`) never change `clientWidth/Height` → no resize.

---

## 6. Asset pipeline

### 6.1 Cached fetch helpers (B 2699–2764, verbatim)

```js
var s_arrayBufferCache = /* @__PURE__ */ new Map();
var s_textCache = /* @__PURE__ */ new Map();
var s_imageCache = /* @__PURE__ */ new Map();
function cachedFetchArrayBuffer(url) {
  let p = s_arrayBufferCache.get(url);
  if (!p) {
    p = fetch(url).then((res) => {
      if (!res.ok) {
        throw new Error(`Failed to fetch ${url}: ${res.status}`);
      }
      return res.arrayBuffer();
    }).catch((err) => {
      s_arrayBufferCache.delete(url);
      throw err;
    });
    s_arrayBufferCache.set(url, p);
  }
  return p;
}
function cachedFetchText(url) {
  let p = s_textCache.get(url);
  if (!p) {
    p = fetch(url).then((res) => {
      if (!res.ok) {
        throw new Error(`Failed to fetch ${url}: ${res.status}`);
      }
      return res.text();
    }).catch((err) => {
      s_textCache.delete(url);
      throw err;
    });
    s_textCache.set(url, p);
  }
  return p;
}
function cachedLoadImage(url) {
  let p = s_imageCache.get(url);
  if (!p) {
    p = new Promise((resolve, reject) => {
      const img = new Image();
      img.decoding = "async";
      img.addEventListener(
        "load",
        () => {
          if (typeof img.decode === "function") {
            img.decode().then(() => resolve(img), () => resolve(img));
          } else {
            resolve(img);
          }
        },
        { once: true, passive: true }
      );
      img.addEventListener(
        "error",
        (err) => {
          s_imageCache.delete(url);
          reject(err);
        },
        { once: true, passive: true }
      );
      img.src = url;
    });
    s_imageCache.set(url, p);
  }
  return p;
}
```

Semantics to reproduce:

* One page-global map per kind, keyed by the exact URL string; the **promise** is cached (concurrent callers share one
  request); every consumer receives the **same** `ArrayBuffer` / string / `HTMLImageElement`.
* Non-2xx → rejects with `Error("Failed to fetch <url>: <status>")`; network error → the fetch rejection. On any
  rejection the entry is deleted (next call retries). Successful entries live for the page lifetime (they survive
  player destroy/re-create — re-mounting Kurisu needs no network).
* `cachedLoadImage`: `new Image()`, `decoding = "async"`, no `crossOrigin` (same-origin), `load` → `img.decode()` if
  available and resolve **regardless** of decode success; `error` → delete + reject with the `Event`.
* Consumers of the shared `ArrayBuffer` never mutate it (CubismJson reads; `Live2DCubismCore.Moc.fromArrayBuffer`
  copies into WASM memory).
* Rust: `thread_local! static CACHE: RefCell<HashMap<String, Shared<LocalBoxFuture<'static, Result<Rc<[u8]>, AssetError>>>>>`
  (and String / `HtmlImageElement` variants); remove the entry when the shared future resolves to `Err`.

### 6.2 `preloadDefaultAssets()` (B 18605–18633, verbatim)

```js
function preloadDefaultAssets() {
  const modelDir = ResourcesPath + ModelDir[0] + "/";
  const modelJsonUrl = modelDir + ModelJsonFileName;
  const shaderPaths = [
    VertShaderSrcPath,
    VertShaderSrcMaskedPath,
    VertShaderSrcSetupMaskPath,
    FragShaderSrcSetupMaskPath,
    FragShaderSrcPremultipliedAlphaPath,
    FragShaderSrcMaskPremultipliedAlphaPath,
    FragShaderSrcMaskInvertedPremultipliedAlphaPath,
    VertShaderSrcCopyPath,
    FragShaderSrcCopyPath,
    FragShaderSrcColorBlendPath,
    FragShaderSrcAlphaBlendPath,
    VertShaderSrcBlendPath,
    FragShaderSrcBlendPath
  ];
  const tasks = [
    cachedFetchArrayBuffer(modelJsonUrl).catch(() => {}),
    cachedFetchArrayBuffer(modelDir + "Live2D\u7D05\u8389\u6816forSDK5.0.moc3").catch(() => {}),
    cachedFetchArrayBuffer(modelDir + "Live2D\u7D05\u8389\u6816forSDK5.0.physics3.json").catch(() => {}),
    cachedLoadImage(modelDir + "Live2D\u7D05\u8389\u6816forSDK5.0.8192/texture_00.png").catch(() => {})
  ];
  for (const sp of shaderPaths) {
    tasks.push(cachedFetchText(ShaderPath + sp).catch(() => {}));
  }
  return Promise.all(tasks).then(() => {});
}
```

Called (a) automatically when the runtime script is evaluated and (b) by `runtime.preload()` (from
`preloadLive2D()`); the second call is a cache hit unless a previous fetch failed. 17 requests in parallel. The file
names are hard-coded (they duplicate model3.json) — **V2: keep the same list** (derive from constants, not by parsing
model3.json, so preloading needs no round trip). V2 entry point: `amadeus_live2d::preload() -> impl Future<Output = ()>`
(never fails). Because the Core is not needed to fetch, preload may start before the Core is ready.

### 6.3 `LAppModel.loadAssets` → `setupModel` state machine (B 15673–15982)

`LoadStep` (numeric values are stored in `_state`; reproduce them for debugging parity):

| Value | Name | Value | Name |
|---|---|---|---|
| 0 | LoadAssets | 12 | WaitLoadUserData |
| 1 | LoadModel | 13 | SetupEyeBlinkIds |
| 2 | WaitLoadModel | 14 | SetupLipSyncIds |
| 3 | LoadExpression | 15 | SetupLook |
| 4 | WaitLoadExpression | 16 | SetupLayout |
| 5 | LoadPhysics | 17 | LoadMotion |
| 6 | WaitLoadPhysics | 18 | WaitLoadMotion |
| 7 | LoadPose | 19 | CompleteInitialize (unused) |
| 8 | WaitLoadPose | 20 | CompleteSetupModel (unused) |
| 9 | SetupEyeBlink | 21 | LoadTexture |
| 10 | SetupBreath | 22 | WaitLoadTexture |
| 11 | LoadUserData | 23 | **CompleteSetup** (only state in which `update()`/`draw()` act) |

`loadAssets(dir, fileName)`:

1. `_modelHomeDir = dir`; `console.log(\`[ReamaCubism] Loading model: ${dir}${fileName}\`)`.
2. `cachedFetchArrayBuffer(dir + fileName)` →
   * ok: `console.log(\`[ReamaCubism] model3.json loaded (${byteLength} bytes)\`)` (421), `setting = new CubismModelSettingJson(buf, len)`,
     `_state = 1`, `setupModel(setting)`.
   * error: `console.error(\`[ReamaCubism] Failed to load model: ${dir}${fileName}\`, error)` + suppressed
     `CubismLogError`; the model stays at state 0 forever (nothing drawn, no retry).

`setupModel(setting)` — written as a chain of closures; for **this model** the exact sequence is:

| # | Step | Effect |
|---|---|---|
| 1 | `_updating = true; _initialized = false; _modelSetting = setting` | |
| 2 | prefetch (A6) | `cachedFetchArrayBuffer(dir+moc).catch(()=>{})`, `cachedFetchArrayBuffer(dir+physics).catch(()=>{})`, `cachedLoadImage(dir+tex[i]).catch(()=>{})` for each texture with a non-empty name |
| 3 | moc | `cachedFetchArrayBuffer(dir+moc).catch(()=>{ CubismLogError(..); return new ArrayBuffer(0); }).then(buf => { loadModel(buf, _mocConsistency=false); _state = 3; loadCubismExpression(); })`; synchronously `_state = 2` |
| 4 | `loadModel` (CubismUserModel) | `CubismMoc.create(buf, false)` (`Live2DCubismCore.Moc.fromArrayBuffer`; null → suppressed error, return) → `createModel()` (`Model.fromMoc` + `CubismModel.initialize()`) → `_model.saveParameters()` (defaults) → `_modelMatrix = new CubismModelMatrix(canvasW, canvasH)` |
| 5 | expressions | count 0 → `_state = 5`, `loadCubismPhysics()` |
| 6 | physics | `cachedFetchArrayBuffer(dir+physics).catch(→ empty).then(buf => { loadPhysics(buf, len); if (_physics) addUpdatable(new CubismPhysicsUpdater(_physics)); _state = 7; loadCubismPose(); })`; sync `_state = 6` |
| 7 | pose | none → `_state = 9`, `setupEyeBlink()` |
| 8 | eye blink | `getEyeBlinkParameterCount()` = 0 → nothing; `_state = 10`, `setupBreath()` |
| 9 | breath | `_breath = CubismBreath.create()`, parameters (§7.6.2), `addUpdatable(new CubismBreathUpdater(_breath))`; `_state = 11` |
| 10 | user data | none → `_state = 13` |
| 11 | eye-blink ids | `_eyeBlinkIds.length = 0` → `_state = 14` |
| 12 | lip-sync ids | `_lipSyncIds.length = 0` → no `CubismLipSyncUpdater`; `_state = 15` |
| 13 | look | `_look = CubismLook.create()`, parameters (§7.6.1), `addUpdatable(new CubismLookUpdater(_look, _dragManager))` |
| 14 | finalize | `_updateScheduler.sortUpdatableList()` (stable ascending execution order → Look 400, Breath 500, Physics 600); `_state = 16` |
| 15 | layout | if `_modelSetting == null || _modelMatrix == null` → suppressed error and **stop** (state stuck at 16). Else `getLayoutMap` (empty) → `setupFromLayout` (no-op); `_state = 17` |
| 16 | motions | `_state = 18`; `_model.saveParameters()` (still defaults); `_allMotionCount = _motionCount = 0`; group count 0 → `_state = 21`; `_motionManager.stopAllMotions()`; `_updating = false; _initialized = true`; `createRenderer(canvas.width, canvas.height)`; `setupTextures()` (§6.5, ends with `_state = 22`); `getRenderer().startUp(gl)`; `getRenderer().loadShaders(ShaderPath)` |
| 17 | texture loaded (async) | `bindTexture(0, tex)`, `_textureCount++`; when `>= textureCount` → `console.log("[ReamaCubism] All textures loaded. Model setup complete!")`, `_state = 23` |

Steps 6 (from the physics `.then`) through 16 run synchronously in one microtask.

Generic branches (port for completeness; inactive here): expressions → load each `File` via
`cachedFetchArrayBuffer(..).catch(→ empty)`, `loadExpression` (`CubismExpressionMotion.create`), replace an existing
entry with the same name, `_expressionCount++`; when all loaded → add `CubismExpressionUpdater` (order 300), `_state = 5`.
Pose/user data → plain `fetch` (`response.ok` → arrayBuffer; `status >= 400` → suppressed error + empty buffer; any
other non-ok status → `undefined` → crash on `.byteLength`), then `CubismPoseUpdater` (800). Eye blink → `CubismEyeBlink`
+ `CubismEyeBlinkUpdater(() => _motionUpdated, eyeBlink)` (200). Lip sync → `CubismLipSyncUpdater(ids, _wavFileHandler)`
(700). Motions → `preLoadMotionGroup` for each group; `_allMotionCount--` on parse failure; when
`_motionCount >= _allMotionCount` do the same tail as step 16.

**QUIRK (V2: guard):** `setupTextures` sets `_state = 22` *after* issuing the texture loads; if a load callback ran
synchronously (texture-manager cache hit), the state would end at 22 and the model would never draw. Unreachable in V1
(each subdelegate has a fresh texture manager; `cachedLoadImage` is always async). V2: set `WaitLoadTexture` before
issuing loads, transition to `CompleteSetup` when the count is reached.

### 6.4 Renderer creation and shader loading (integration level; internals in the render spec)

* `createRenderer(w, h, maskBufferCount = 1)`: delete an existing renderer; `new CubismRenderer_WebGL(w, h)`;
  `initialize(model, 1)` (clipping manager because `isUsingMasking()`; no GL calls yet).
* `setIsPremultipliedAlpha(true)` (called inside the texture loop).
* `startUp(gl)`: stores `gl`, shader manager context, anisotropy extension probe
  (`EXT_texture_filter_anisotropic` / `WEBKIT_…` / `MOZ_…`), creates the 256×256 mask render target(s), binds
  framebuffer `_currentFbo`.
* `loadShaders(ShaderPath)`: if the per-context shader set is empty or not loaded → `setShaderPath(ShaderPath)` +
  `generateShaders()` → async `loadShaders()` fetches the 13 files through `cachedFetchText` (individual failures →
  `console.error(\`Error loading ${path} shader:\`, error)` and empty source) → `registerShader()` +
  `registerBlendShader()` → `_isShaderLoaded = true` (overall failure → `console.error("Failed to load shaders:", error)`).
  `doDrawModel` calls `loadShaders` again every frame (no-op once loading/loaded) and every draw call returns early while
  `_isShaderLoaded` is false → the model appears only when **both** the texture (state 23) and the shaders are ready.

### 6.5 Texture upload — `LAppModel.setupTextures` + `LAppTextureManager.createTextureFromPngFile` (B 15986–16012, 16598–16661)

`setupTextures()` (only if `_state == 21`): `usePremultiply = true`; `console.log(\`[ReamaCubism] Loading ${textureCount} texture(s)...\`)`;
for each index: empty name → `console.log("getTextureFileName null")`, skip; `path = _modelHomeDir + name`;
`console.log(\`[ReamaCubism] Loading texture ${i}: ${path}\`)`; `createTextureFromPngFile(path, true, onLoad)`;
`getRenderer().setIsPremultipliedAlpha(true)`. After the loop `_state = 22`.
`onLoad(info)`: `console.log(\`[ReamaCubism] Texture ${i} loaded: ${info.width}x${info.height}\`)`,
`getRenderer().bindTexture(i, info.id)`, `_textureCount++`, completion as in §6.3 step 17.

`createTextureFromPngFile(fileName, usePremultiply, callback)`:

1. Linear search in `_textures` for `fileName == f && usePremultply == p` → `callback(entry)` **synchronously**, return.
2. `cachedLoadImage(fileName).then(img => { … }).catch(e => console.error(\`[ReamaCubism] Failed to load texture image: ${fileName}\`, e))`.
   The `.catch` also swallows exceptions thrown inside the `then` body, including inside `callback`.
3. In `then`: if `!_glManager || !_glManager.getGl()` → return silently (no callback).
4. `tex = gl.createTexture(); gl.bindTexture(TEXTURE_2D, tex);`
   `texParameteri(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR_MIPMAP_LINEAR); texParameteri(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR);`
   (wrap modes left at REPEAT; `UNPACK_FLIP_Y_WEBGL` left false).
5. `if (usePremultiply) gl.pixelStorei(UNPACK_PREMULTIPLY_ALPHA_WEBGL, 1)` — **never reset** (stays on for the
   context; harmless, later uploads have null data).
6. `max = gl.getParameter(MAX_TEXTURE_SIZE)`. If `img.width > max || img.height > max`:
   `console.warn(\`[ReamaCubism] Texture ${fileName} (${img.width}x${img.height}) exceeds MAX_TEXTURE_SIZE (${max}). Downscaling...\`)`;
   `scale = max / Math.max(w, h)`; offscreen `<canvas>` of `Math.floor(w*scale) × Math.floor(h*scale)`;
   `ctx = getContext("2d")`; if ctx: `ctx.drawImage(img, 0, 0, ow, oh)` (default smoothing) and
   `texImage2D(TEXTURE_2D, 0, RGBA, RGBA, UNSIGNED_BYTE, offscreen)`; **if ctx is null nothing is uploaded** (QUIRK).
   Else `texImage2D(TEXTURE_2D, 0, RGBA, RGBA, UNSIGNED_BYTE, img)`.
7. `generateMipmap(TEXTURE_2D); bindTexture(TEXTURE_2D, null)`.
8. `info = { fileName, width: img.width, height: img.height /* ORIGINAL size even if downscaled */, id: tex, img,
   usePremultply }`; push to `_textures`; `callback(info)`.

Rust (`web-sys`): `tex_image_2d_with_u32_and_u32_and_html_image_element` /
`…_and_html_canvas_element`, `pixel_storei(UNPACK_PREMULTIPLY_ALPHA_WEBGL, 1)`. With the 4096² texture the downscale
path only triggers on GPUs with `MAX_TEXTURE_SIZE < 4096` (rare); keep it.

### 6.6 Failure modes (V1 behaviour → V2 must match)

| Failure | V1 result |
|---|---|
| Core script not loaded when `create()` runs | `console.error("[ReamaCubism] Live2DCubismCore is not loaded.")`, inert player |
| WebGL unavailable | `console.error("Cannot initialize WebGL context.")` + `"[ReamaCubism] Failed to initialize Cubism subdelegate."`, host emptied, framework released |
| model3.json 404 / network | `console.error("[ReamaCubism] Failed to load model: …", err)`; blank canvas forever; frames keep running |
| moc3 fails / invalid | empty buffer → `Live2DCubismCore.Moc.fromArrayBuffer` logs `[CSM] [E]csmReviveMocInPlace: "size" is invalid.` through the undefined log function → `TypeError: Logging.logFunction is not a function` escapes `loadModel` → the `.then` aborts (unhandled promise rejection in the console), state stays **2**; blank. (With a log function registered — V2 — `CubismMoc.create` returns null, `_model` stays null and the SDK chain runs on to SetupLayout and stops at state 16; equally blank. V2 may simply stop loading and log once.) |
| physics fails | `loadPhysics(empty, 0)` → no physics → no updater; model renders without hair sway |
| texture fails | `console.error("[ReamaCubism] Failed to load texture image: …", e)`; state stays 22; blank |
| shader file fails | `console.error("Error loading … shader:", e)`, empty source → compile errors logged by the render layer; draws skipped/broken |

### 6.7 Typical load timeline (all caches warm thanks to `preloadLive2D()`)

`create()` → [sync] canvas, context, framework, `loadAssets` issued, first frame (clear only) → [microtasks] model3.json
promise resolves → moc promise resolves → physics promise resolves → steps 6–16 → [task] image decode already done →
texture `then` → state 23 → [async] shader sources already cached → `_isShaderLoaded = true` within a few microtasks →
model visible on the next animation frame (typically frame 2–3 after mount).

---

## 7. Per-frame pipeline (exact order)

### 7.1 `tick` (B 18120–18127)

```js
this.tick = () => {
  if (this.destroyed || !this.subdelegate) return;
  LAppPal.updateTime();
  this.subdelegate.update();
  this.rafHandle = requestAnimationFrame(this.tick);
};
```

Driven by `requestAnimationFrame` (no FPS cap: 60/120/144 Hz follow the display; paused in background tabs). **No
try/catch:** an exception anywhere in `subdelegate.update()` happens before the rAF is re-armed and stops the loop
forever (QUIRK; V2: reproduce the "stop on fatal error" semantics but log the error — see Open questions).

### 7.2 Clock 1 — `LAppPal` (B 2626–2663, verbatim)

```js
var LAppPal = class {
  static loadFileAsBytes(filePath, callback) {
    fetch(filePath).then((response) => response.arrayBuffer()).then((arrayBuffer) => callback(arrayBuffer, arrayBuffer.byteLength));
  }
  static getDeltaTime() { return this.deltaTime; }
  static updateTime() {
    this.currentFrame = Date.now();
    this.deltaTime = (this.currentFrame - this.lastFrame) / 1e3;
    this.lastFrame = this.currentFrame;
  }
  static printMessage(message) { console.log(message); }
};
LAppPal.lastUpdate = Date.now();   // unused
LAppPal.currentFrame = 0;
LAppPal.lastFrame = 0;
LAppPal.deltaTime = 0;
```

* Unit: seconds, from integer-millisecond `Date.now()` → typical sequence at 60 Hz: 0.016, 0.017, 0.017, 0.016, …
* Uncapped. Used only by `LAppModel.update()` (scheduler: Look/Breath/Physics).
* Page-global static. The very first `updateTime()` of the page yields `Date.now()/1000` (≈1.76·10⁹ s) because
  `lastFrame` starts at 0 — harmless because no model can be `CompleteSetup` on the first tick (loading is async).
* **QUIRK:** two simultaneously running players (e.g. a remount overlapping the old instance) each call
  `updateTime()`, so each sees roughly half the real delta. **V2: fix** — one `FrameClock` per player, initialized
  with `last_frame_ms = Date.now()` at player creation (observable behaviour of a single player is unchanged).

### 7.3 `LAppSubdelegate.update()` (B 17559–17576, verbatim)

```js
update() {
  if (this._glManager.getGl().isContextLost()) { return; }
  if (this._needResize) { this.onResize(); this._needResize = false; }
  const gl = this._glManager.getGl();
  gl.clearColor(0, 0, 0, 0);                    // transparent (customization B5)
  gl.enable(gl.DEPTH_TEST);
  gl.depthFunc(gl.LEQUAL);
  gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
  gl.clearDepth(1);                             // after clear (stock sample order) – affects the NEXT clear only
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  this._view.render();
}
```

On a lost context the frame is skipped but the rAF loop keeps running (§10.3). The renderer's `preDraw()` disables
`DEPTH_TEST` before drawing (this model uses masks, so `preDraw` always runs).

### 7.4 `LAppView.render()` (B 17335–17349, verbatim)

```js
render() {
  this._subdelegate.getGlManager().getGl().useProgram(this._programId);   // sprite program, draws nothing
  if (this._back) { this._back.render(this._programId); }
  if (this._gear) { this._gear.render(this._programId); }
  this._subdelegate.getGlManager().getGl().flush();                        // before the model is drawn (stock)
  const lapplive2dmanager = this._subdelegate.getLive2DManager();
  if (lapplive2dmanager != null) {
    lapplive2dmanager.setViewMatrix(this._viewMatrix);                    // copies 16 floats (identity in V1)
    lapplive2dmanager.onUpdate();
  }
}
```

### 7.5 `LAppLive2DManager.onUpdate()` (B 16465–16509, verbatim)

```js
onUpdate() {
  const gl = this._subdelegate.getGl();
  CubismWebGLOffscreenManager.getInstance().beginFrameProcess(gl);
  const { width, height } = this._subdelegate.getCanvas();       // backing-store size in device pixels
  const projection = new CubismMatrix44();                        // identity
  const model = this._models[0];
  if (!model) {
    CubismWebGLOffscreenManager.getInstance().endFrameProcess(gl);
    CubismWebGLOffscreenManager.getInstance().releaseStaleRenderTextures(gl);
    return;
  }
  if (model.getModel()) {
    if (model.getModel().getCanvasWidth() > 1 && width < height) {
      model.getModelMatrix().setWidth(2);
      projection.scale(1, width / height);
    } else {
      projection.scale(height / width, 1);
    }
    if (this._viewMatrix != null) {
      projection.multiplyByMatrix(this._viewMatrix);
    }
  }
  model.update();
  if (this._parameterDriver) {
    this._parameterDriver(model);
    // Re-compute vertex positions after our parameter overrides.
    // Without this, the Cubism core already finalized geometry inside
    // model.update() above, making our setParameterValueById calls invisible.
    if (model.getModel()) {
      model.getModel().update();
    }
  }
  if (width < height) {
    // Mobile: slightly smaller and shifted lower
    projection.scaleRelative(3.3, 3.3);
    projection.translateY(-2.1);
  } else {
    // Desktop
    projection.scaleRelative(4.1, 4.1);
    projection.translateY(-2.75);
  }
  model.draw(projection);
  CubismWebGLOffscreenManager.getInstance().endFrameProcess(gl);
  CubismWebGLOffscreenManager.getInstance().releaseStaleRenderTextures(gl);
}
```

`LAppModel.draw(matrix)` (B 16296–16305): `if (_model == null) return; if (_state == 23) { matrix.multiplyByMatrix(_modelMatrix);
getRenderer().setMvpMatrix(matrix); doDraw(); }`. `doDraw()` (B 16283–16292): `if (_model == null) return;
viewport = [0, 0, canvas.width, canvas.height]; getRenderer().setRenderState(subdelegate.getFrameBuffer() /*null*/,
viewport); getRenderer().drawModel(ShaderPath);`. `setRenderState` binds the framebuffer, sets the viewport and updates
`_modelRenderTargetWidth/Height` if they differ.

### 7.6 `LAppModel.update()` and the update scheduler (B 16028–16048, 15048–15384)

```js
update() {
  if (this._state != 23 /* CompleteSetup */) return;
  const deltaTimeSeconds = LAppPal.getDeltaTime();
  this._userTimeSeconds += deltaTimeSeconds;                 // unused elsewhere
  this._model.loadParameters();                              // restore saved (= default) values
  this._motionUpdated = false;
  if (this._motionManager.isFinished()) {                    // always true (empty queue)
    this.startRandomMotion(MotionGroupIdle, PriorityIdle);   // getMotionCount("Idle") == 0 → returns -1 immediately
  } else {
    this._motionUpdated = this._motionManager.updateMotion(this._model, deltaTimeSeconds);
  }
  this._model.saveParameters();                              // saves the same defaults again
  this._updateScheduler.onLateUpdate(this._model, deltaTimeSeconds);
  this._model.update();                                      // core update #1 (+ resetDynamicFlags)
}
```

`CubismUpdateScheduler.onLateUpdate(model, dt)`: return if `!model`; if `_needsSort` → `sortUpdatableList()`
(`Array.sort` with `left.getExecutionOrder() - right.getExecutionOrder()`, nulls last); call
`updater.onLateUpdate(model, dt)` in list order. `addUpdatableList` ignores null/duplicates, registers the scheduler as
change listener, sets `_needsSort`. `setExecutionOrder` on an updater re-flags the sort. Default orders:
EyeBlink 200, Expression 300, Drag/Look 400, Breath 500, Physics 600, LipSync 700, Pose 800.

**Registered for this model: Look (400) → Breath (500) → Physics (600).** Each updater returns early if `!model`.

#### 7.6.1 Look (order 400; B 15223–15231, 4217–4225, 15909–15936)

```js
this._dragManager.update(deltaTimeSeconds);                      // CubismTargetPoint, render spec §3.7
const dragX = this._dragManager.getX(), dragY = this._dragManager.getY();
for (data of lookParameters)
  model.addParameterValueById(data.parameterId, data.factorX * dragX + data.factorY * dragY + data.factorXY * dragX * dragY);
```

Look parameters (`new LookParameterData(id, factorX, factorY, factorXY)`), in order:
`ParamAngleX (30, 0, 0)`, `ParamAngleY (0, 30, 0)`, `ParamAngleZ (0, 0, -30)`, `ParamBodyAngleX (10, 0, 0)`,
`ParamEyeBallX (1, 0, 0)`, `ParamEyeBallY (0, 1, 0)`. The drag target is never set (no input) → face stays (0, 0) →
adds 0 (value re-clamped and re-stored, no change). Keep it: cheap, and it keeps `CubismTargetPoint` time advancing.

#### 7.6.2 Breath (order 500; B 4136–4147, 15832–15857)

```js
this._currentTime += deltaTimeSeconds;           // starts at 0 when the breath object is created
const t = this._currentTime * 2 * Math.PI;
for (data of breathParameters)
  model.addParameterValueById(data.parameterId, data.offset + data.peak * Math.sin(t / data.cycle), data.weight);
// addParameterValueById(id, v, w) = setParameterValueByIndex(i, getValue(i) + v * w)   (then clamped)
```

`new BreathParameterData(id, offset, peak, cycle, weight)`, in order:

| Parameter | offset | peak | cycle (s) | weight | Net added to the default |
|---|---|---|---|---|---|
| `ParamAngleX` | 0 | 15 | 6.5345 | 0.5 | `7.5·sin(2πt/6.5345)` |
| `ParamAngleY` | 0 | 8 | 3.5345 | 0.5 | `4·sin(2πt/3.5345)` |
| `ParamAngleZ` | 0 | 10 | 5.5345 | 0.5 | `5·sin(2πt/5.5345)` |
| `ParamBodyAngleX` | 0 | 4 | 15.5345 | 0.5 | `2·sin(2πt/15.5345)` |
| `ParamBreath` | 0.5 | 0.5 | 3.2345 | 1 | `0.5 + 0.5·sin(2πt/3.2345)` |

`t` here is the breath object's accumulated **Clock 1** time. Golden check (first frame after setup, Δ = 0.016 s):
`ParamAngleX = 0.115380287`, `ParamAngleY = 0.113755733`, `ParamAngleZ = 0.0908170864`,
`ParamBodyAngleX = 0.0129428385`, `ParamBreath = 0.515537918` (§12.2).

#### 7.6.3 Not registered here (generic)

`CubismEyeBlinkUpdater` (200; only when `!motionUpdated()`), `CubismExpressionUpdater` (300;
`expressionManager.updateMotion`), `CubismLipSyncUpdater` (700; `audioProvider.update(dt)` then
`addParameterValueById(id, rms)` for each lip-sync id), `CubismPoseUpdater` (800). Port them with the framework; they
stay inactive for this model.

#### 7.6.4 Physics (order 600; `CubismPhysics.evaluate`, B 7998–8178) — integration semantics

* `dt <= 0` → no-op. `_currentRemainTime += dt`; if `> 5` → `0`.
* Parameter caches (`Float32Array(paramCount)`): `_parameterInputCaches` is initialized from the current values on the
  first call.
* Fixed step `1/Fps = 1/60`. While `remain >= step`: copy current outputs to previous; `inputWeight = step / remain`;
  `cache[j] = inputCache[j]·(1−inputWeight) + values[j]·inputWeight` for **every** parameter (inputCache := cache);
  evaluate each setting from the cached inputs (normalized position/angle, `updateParticles` with air resistance 5,
  movement threshold `0.001 × normalizationPosition.maximum`), store outputs; `remain -= step`.
* Finally `interpolate(model, remain / step)`: for every output,
  `updateOutputParameterValue(values.slice(dest), min, max, prev·(1−α) + cur·α, output)` written **directly into the
  Core parameter array** (`model.getModel().parameters.values`).
* Inputs are whatever is in the parameter array at this moment = defaults + Look(0) + Breath — **not** the player's
  values (§7.8 b).

### 7.7 CubismModel parameter semantics this layer relies on (B 9436–10031, 10495–10506)

* `update()` = `this._model.update(); this._model.drawables.resetDynamicFlags();` (Core).
* `saveParameters()`: copy `parameters.values[i]` (f32) into the JS array `_savedParameters` (grows as needed).
  `loadParameters()`: copy back `min(paramCount, saved.length)` entries. Saves happen at `loadModel`, at
  `loadCubismMotion`, and every frame after the (absent) motion → **saved = model defaults forever**.
* `getParameterIndex(id)`: linear search over the interned ids; unknown ids get a virtual index
  `paramCount + n` with a JS-side value (never reaches the Core). All 22 player ids exist in this model.
* `setParameterValueByIndex(i, value, weight = 1)`: virtual index → store (weighted); else clamp (`repeat` never
  applies, §2.2) then `values[i] = weight == 1 ? value : values[i]·(1−weight) + value·weight` (f32 store).
* `addParameterValueByIndex(i, v, w = 1)` = `setParameterValueByIndex(i, getValue(i) + v·w)` (weight 1 for the set).
* `CubismMath.clamp(val, min, max)`: `val < min → min; max < val → max; else val` (NaN passes through).

### 7.8 Consequences of the order (all must be reproduced)

a. **Every frame starts from the defaults** (`loadParameters`), so the driver writes all of its 22 parameters every
   frame. `ParamBrowLX/RX` stay at 0; the hair params come from physics only.
b. **Physics/breath decoupling (QUIRK, visible).** At Physics time the four input parameters hold
   `AngleX = 7.5·sin(2πt/6.5345)`, `AngleZ = 5·sin(2πt/5.5345)`, `BodyAngleX = 2·sin(2πt/15.5345)`, `BodyAngleZ = 0`
   (Clock-1 time). The visible angles are then replaced by the driver; the breath-driven values are never rendered
   but fully determine hair motion (golden 10-s run: HairFront/Side ∈ [−0.033, 0.027], HairBack ∈ [−0.044, 0.016],
   while the rendered ParamAngleX spans −19.3…24.3). **Do not "fix"
   by moving the driver before physics** — it would change how the hair moves (product decision, see Open questions).
c. **Loading window.** Once the moc is loaded (`getModel()` non-null, state 3…22) but before `CompleteSetup`,
   `model.update()` returns early yet the driver still runs every frame (player timers, blink FSM and smoothing advance;
   `_lastUpdateTime` is initialized on the first such call) and core update #2 runs; nothing is drawn. Because
   `loadParameters` does not run in that window, the driver's values persist between frames (harmless).
d. **Dynamic flags.** Each `CubismModel.update()` resets the Core's dynamic flags; the renderer sees the flags
   produced by update #2 (relative to the update #1 state). The render spec measured `IsVisible|VertexPositionsDidChange`
   (33) for all drawables every frame.
e. **Cost:** two Core updates per frame (the first one's geometry is discarded). Keep both (risk-free parity).
f. **The view matrix is identity** for the whole session (§8.1), so `multiplyByMatrix(viewMatrix)` is a no-op.

### 7.9 Whole frame as pseudocode (normative for V2)

```text
on animation frame:
  if destroyed or no subdelegate: return
  clock1.update(Date.now())                                  # dt1 = Δms/1000, uncapped
  if gl.is_context_lost(): goto SCHEDULE                     # skip all GL work, loop keeps running
  if need_resize: resize_canvas(); view.initialize(); manager.set_offscreen_size(w,h); need_resize=false
  gl: clearColor(0,0,0,0); enable DEPTH_TEST; depthFunc LEQUAL; clear(COLOR|DEPTH); clearDepth(1); enable BLEND; blendFunc(SRC_ALPHA, ONE_MINUS_SRC_ALPHA)
  (optional) useProgram(sprite); gl.flush()
  manager.view_matrix := view.view_matrix                     # identity
  offscreen.begin_frame(gl)
  (w, h) := canvas.width, canvas.height
  P := identity
  if model.core exists:
      if model.canvas_width > 1 and w < h: model_matrix.set_width(2); P.scale(1, w/h)   # never for this model
      else: P.scale(h/w, 1)
      P := view_matrix · P                                    # multiplyByMatrix
  if model.state == CompleteSetup:
      core.load_parameters(); startRandomMotion("Idle") /*no-op*/; core.save_parameters()
      look.update(dt1); breath.update(dt1); physics.evaluate(dt1)
      core.update()                                           # #1
  if driver set:
      player.apply_parameters(model)                         # uses clock 2 (performance.now, cap 0.1 s); returns early if core missing
      if model.core exists: core.update()                     # #2
  if w < h: P.scale_relative(3.3, 3.3); P.translate_y(-2.1) else: P.scale_relative(4.1, 4.1); P.translate_y(-2.75)
  if model.core exists and state == CompleteSetup:
      P := model_matrix · P; renderer.set_mvp(P); renderer.set_render_state(null, [0,0,w,h]); renderer.draw_model()
  offscreen.end_frame(gl); offscreen.release_stale(gl)
SCHEDULE:
  schedule next frame          # only if no exception escaped (V1 has no try/catch)
```

(In V1 `tick` re-arms the rAF after `subdelegate.update()` returns, including when the context is lost.)

---

## 8. Projection, view and model matrices

Matrix conventions (row-vector, `multiplyByMatrix(m)` = `this ← m · this`, `scale()`/`translateY()` absolute,
`scaleRelative()` = `S · this`, f32 storage): render spec §3.2–3.5. Restated only as needed.

### 8.1 `LAppView.initialize(subdelegate)` (B 17290–17317, verbatim) — run at init and on every resize

```js
initialize(subdelegate) {
  this._subdelegate = subdelegate;
  const { width, height } = subdelegate.getCanvas();
  const ratio = width / height;
  const left = -ratio;
  const right = ratio;
  const bottom = ViewLogicalLeft;     // -1   (sic: uses Left/Right constants for bottom/top)
  const top = ViewLogicalRight;       //  1
  this._viewMatrix.setScreenRect(left, right, bottom, top);
  this._viewMatrix.scale(ViewScale, ViewScale);   // absolute 1,1 → resets zoom, translation untouched (always 0)
  this._deviceToScreen.loadIdentity();
  if (width > height) {
    const screenW = Math.abs(right - left);
    this._deviceToScreen.scaleRelative(screenW / width, -screenW / width);
  } else {
    const screenH = Math.abs(top - bottom);
    this._deviceToScreen.scaleRelative(screenH / height, -screenH / height);
  }
  this._deviceToScreen.translateRelative(-width * 0.5, -height * 0.5);
  this._viewMatrix.setMaxScale(ViewMaxScale);     // 2
  this._viewMatrix.setMinScale(ViewMinScale);     // 0.8
  this._viewMatrix.setMaxScreenRect(ViewLogicalMaxLeft, ViewLogicalMaxRight, ViewLogicalMaxBottom, ViewLogicalMaxTop); // -2,2,-2,2
}
```

* `CubismViewMatrix` (B 16728–16992) = `CubismMatrix44` + f64 fields `_screenLeft/_screenRight/_screenTop/_screenBottom`,
  `_maxLeft/_maxRight/_maxTop/_maxBottom`, `_maxScale`, `_minScale` (all 0 initially); `setScreenRect(l, r, b, t)`,
  `setMaxScreenRect(l, r, b, t)`, `setMaxScale/setMinScale`, getters, `isMaxScale()` = `tr[0] >= _maxScale`,
  `isMinScale()` = `tr[0] <= _minScale`; `adjustTranslate(x, y)` clamps `x`/`y` against the screen/max rectangles
  (QUIRK: assumes scale 1) then left-multiplies a translation; `adjustScale(cx, cy, s)` clamps `s·tr[0]` to
  [min, max] (only when `tr[0] > 0`) then left-multiplies `T(−cx,−cy)`, `S(s)`, `T(cx,cy)` in that order. Exact
  formulas and f32 rules: render spec §3.5.
* `adjustTranslate` / `adjustScale` are never called → **the view matrix is identity forever**; `_deviceToScreen` is only
  used by the dead touch transforms. With a 0×0 canvas `ratio` is NaN (harmless; no draw is visible).
* V2: implement `CubismViewMatrix` per render spec §3.5 and call `initialize` exactly like this (cheap), or hard-wire
  identity — both are pixel-identical.

### 8.2 Projection (from §7.5)

For this model `getCanvasWidth() = 1.0` (not `> 1`) so the "wide model in portrait window" branch never runs.
With `w = canvas.width`, `h = canvas.height` (device px):

```
P = I
P.scale(h / w, 1)              # tr[0] = f32(h/w), tr[5] = 1
P = View · P                   # identity
if w < h:  P = S(3.3) · P ; tr[13] = -2.1        ("Mobile")
else:      P = S(4.1) · P ; tr[13] = -2.75       ("Desktop"; also used for square canvases)
MVP = ModelMatrix · P          # ModelMatrix = diag(1.4 (f32 1.39999998), 1.4, 1, 1), translation 0
```

Result: `tr[0] = k·1.4·h/w`, `tr[5] = k·1.4`, `tr[12] = 0`, `tr[13] = c`, all other entries identity, with
`(k, c) = (3.3, −2.1)` portrait, `(4.1, −2.75)` otherwise. NDC: `x = tr[0]·x_model`, `y = tr[5]·y_model + c`.
The "Mobile"/"Desktop" choice depends only on the canvas aspect, not on the device or the app's mobile layout.
QUIRK (unreachable here, reproduce generically): the portrait branch's `setWidth(2)` permanently mutates the model
matrix.

### 8.3 MVP golden values (real bundle, `pipeline.cjs`; f32 values printed to 9 significant digits)

| canvas (device px) | branch | tr[0] | tr[5] | tr[12] | tr[13] |
|---|---|---|---|---|---|
| 1920×1045 (desktop stage, DPR 1) | Desktop | 3.12411451 | 5.73999977 | 0 | −2.75 |
| 2880×1567 (DPR 1.5) | Desktop | 3.12311816 | 5.73999977 | 0 | −2.75 |
| 1280×720 | Desktop | 3.22874975 | 5.73999977 | 0 | −2.75 |
| 1000×1000 | Desktop | 5.73999977 | 5.73999977 | 0 | −2.75 |
| 1170×2532 (390×844 @3) | Mobile | 9.99815369 | 4.61999989 | 0 | −2.0999999 |
| 750×1334 (375×667 @2) | Mobile | 8.21743965 | 4.61999989 | 0 | −2.0999999 |

### 8.4 Visible model region

Model space y ∈ [−0.714, 0.714] (canvas height 1.42857 units, origin centred). Visible NDC y ∈ [−1, 1] ↔
Desktop y_model ∈ [(−1+2.75)/5.74, (1+2.75)/5.74] = [0.3049, 0.6533]; Mobile [(−1+2.1)/4.62, (1+2.1)/4.62] =
[0.2381, 0.6710] — head and upper chest only. (The ref comment's Qt target was y ∈ [0.231, 0.707].)

---

## 9. `ReamaCubismPlayer` — the custom parameter driver (B 17689–18604)

Everything in this section is pure logic on top of 22 parameter writes; it has no WebGL dependency and should live in
a **native-testable, web-free Rust module** (`amadeus_live2d::player::driver`, see Rust mapping).

### 9.1 `EMOTION_TARGETS` (B 17690–17872, verbatim — copy exactly; key order matters only for iteration)

```js
var EMOTION_TARGETS = {
  NORMAL: {
    browY: 0,
    browForm: 0.5,
    browAngle: 0,
    eyeOpen: 1,
    eyeSmile: 0,
    mouthForm: 0,
    bodyAngleX: 0,
    bodyAngleY: 0,
    bodyAngleZ: 0,
    headAngleX: 0,
    headAngleY: 0,
    headAngleZ: 0,
    cheek: 0
  },
  SMILE: {
    browY: 0.4,
    browForm: 0.8,
    browAngle: 0.2,
    eyeOpen: 0.6,
    eyeSmile: 1,
    mouthForm: 1,
    bodyAngleX: 3,
    bodyAngleY: 0,
    bodyAngleZ: -2,
    headAngleX: 5,
    headAngleY: 3,
    headAngleZ: -3,
    cheek: 0.3
  },
  ANGRY: {
    browY: -0.6,
    browForm: -0.8,
    browAngle: -0.8,
    eyeOpen: 0.5,
    eyeSmile: 0,
    mouthForm: -0.8,
    bodyAngleX: -4,
    bodyAngleY: 0,
    bodyAngleZ: 0,
    headAngleX: -5,
    headAngleY: -5,
    headAngleZ: 0,
    cheek: 0
  },
  SAD: {
    browY: -0.5,
    browForm: -0.6,
    browAngle: 0.6,
    eyeOpen: 0.4,
    eyeSmile: 0,
    mouthForm: -0.5,
    bodyAngleX: -3,
    bodyAngleY: -3,
    bodyAngleZ: -3,
    headAngleX: -8,
    headAngleY: -7,
    headAngleZ: -5,
    cheek: 0
  },
  SURPRISED: {
    browY: 0.8,
    browForm: 0.5,
    browAngle: 0,
    eyeOpen: 1.25,
    eyeSmile: 0,
    mouthForm: -0.4,
    bodyAngleX: -2,
    bodyAngleY: 5,
    bodyAngleZ: 2,
    headAngleX: 2,
    headAngleY: 8,
    headAngleZ: 0,
    cheek: 0
  },
  BLUSH: {
    browY: 0.2,
    browForm: 0.4,
    browAngle: 0.3,
    eyeOpen: 0.6,
    eyeSmile: 0.7,
    mouthForm: 0.3,
    bodyAngleX: 8,
    bodyAngleY: -3,
    bodyAngleZ: -3,
    headAngleX: 8,
    headAngleY: -8,
    headAngleZ: -7,
    cheek: 1
  },
  WINK: {
    browY: 0.5,
    browForm: 0.8,
    browAngle: 0.2,
    eyeOpen: 1,
    eyeSmile: 0.7,
    mouthForm: 0.5,
    bodyAngleX: 2,
    bodyAngleY: 0,
    bodyAngleZ: -5,
    headAngleX: 5,
    headAngleY: 2,
    headAngleZ: -5,
    cheek: 0,
    isWink: true
  },
  DISGUST: {
    browY: -0.3,
    browForm: -0.9,
    browAngle: -0.5,
    eyeOpen: 0.4,
    eyeSmile: 0,
    mouthForm: -0.9,
    bodyAngleX: 5,
    bodyAngleY: 4,
    bodyAngleZ: 2,
    headAngleX: 5,
    headAngleY: -4,
    headAngleZ: 3,
    cheek: 0
  },
  SMUG: {
    browY: 0.3,
    browForm: 0.7,
    browAngle: 0.4,
    eyeOpen: 0.6,
    eyeSmile: 0.8,
    mouthForm: 0.6,
    bodyAngleX: 4,
    bodyAngleY: 0,
    bodyAngleZ: -2,
    headAngleX: 10,
    headAngleY: 5,
    headAngleZ: -2,
    cheek: 0
  },
  THINKING: {
    browY: -0.2,
    browForm: -0.3,
    browAngle: 0.2,
    eyeOpen: 0.8,
    eyeSmile: 0,
    mouthForm: -0.2,
    bodyAngleX: -2,
    bodyAngleY: 5,
    bodyAngleZ: 5,
    headAngleX: 5,
    headAngleY: -8,
    headAngleZ: 5,
    cheek: 0
  },
  PANIC: {
    browY: 0.6,
    browForm: -0.5,
    browAngle: -0.3,
    eyeOpen: 1.2,
    eyeSmile: 0,
    mouthForm: -0.5,
    bodyAngleX: -3,
    bodyAngleY: 0,
    bodyAngleZ: 0,
    headAngleX: -2,
    headAngleY: 0,
    headAngleZ: 0,
    cheek: 0.6
  },
  SLEEPING: {
    browY: -0.3,
    browForm: 0.2,
    browAngle: -0.2,
    eyeOpen: 0.0,
    eyeSmile: 0.0,
    mouthForm: -0.1,
    bodyAngleX: -1.0,
    bodyAngleY: -2.0,
    bodyAngleZ: -2.0,
    headAngleX: -12.0,
    headAngleY: -10.0,
    headAngleZ: -8.0,
    cheek: 0.0
  }
};
```

Same data as a table (`isWink` only on WINK). Unknown tags → `EMOTION_TARGETS.NORMAL` (lookup is by the exact upper-case
key; `"SNEEZE"` is **not** a key → NORMAL face):

| tag | browY | browForm | browAngle | eyeOpen | eyeSmile | mouthForm | bodyX | bodyY | bodyZ | headX | headY | headZ | cheek |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| NORMAL | 0 | 0.5 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| SMILE | 0.4 | 0.8 | 0.2 | 0.6 | 1 | 1 | 3 | 0 | −2 | 5 | 3 | −3 | 0.3 |
| ANGRY | −0.6 | −0.8 | −0.8 | 0.5 | 0 | −0.8 | −4 | 0 | 0 | −5 | −5 | 0 | 0 |
| SAD | −0.5 | −0.6 | 0.6 | 0.4 | 0 | −0.5 | −3 | −3 | −3 | −8 | −7 | −5 | 0 |
| SURPRISED | 0.8 | 0.5 | 0 | 1.25 | 0 | −0.4 | −2 | 5 | 2 | 2 | 8 | 0 | 0 |
| BLUSH | 0.2 | 0.4 | 0.3 | 0.6 | 0.7 | 0.3 | 8 | −3 | −3 | 8 | −8 | −7 | 1 |
| WINK (isWink) | 0.5 | 0.8 | 0.2 | 1 | 0.7 | 0.5 | 2 | 0 | −5 | 5 | 2 | −5 | 0 |
| DISGUST | −0.3 | −0.9 | −0.5 | 0.4 | 0 | −0.9 | 5 | 4 | 2 | 5 | −4 | 3 | 0 |
| SMUG | 0.3 | 0.7 | 0.4 | 0.6 | 0.8 | 0.6 | 4 | 0 | −2 | 10 | 5 | −2 | 0 |
| THINKING | −0.2 | −0.3 | 0.2 | 0.8 | 0 | −0.2 | −2 | 5 | 5 | 5 | −8 | 5 | 0 |
| PANIC | 0.6 | −0.5 | −0.3 | 1.2 | 0 | −0.5 | −3 | 0 | 0 | −2 | 0 | 0 | 0.6 |
| SLEEPING | −0.3 | 0.2 | −0.2 | 0.0 | 0.0 | −0.1 | −1.0 | −2.0 | −2.0 | −12.0 | −10.0 | −8.0 | 0.0 |

(`bodyX/Y/Z` = `bodyAngleX/Y/Z`, `headX/Y/Z` = `headAngleX/Y/Z`.) Values above the parameter range (eyeOpen 1.25 /
1.2) are clamped by the model at write time; the player keeps the unclamped value internally.

### 9.2 Math helpers (B 17873–17894, verbatim)

```js
var frameworkRefCount = 0;
function lerp(from, to, amount) {
  return from + (to - from) * amount;
}
function qBound(minV, value, maxV) {
  return Math.max(minV, Math.min(value, maxV));
}
function noise01(x, seed) {
  const s1 = Math.sin(x * 1.37 + seed * 0.71);
  const s2 = Math.sin(x * 2.11 + seed * 1.13);
  const s3 = Math.sin(x * 0.73 + seed * 2.03);
  return qBound(0.0, 0.5 + s1 * 0.28 + s2 * 0.16 + s3 * 0.06, 1.0);
}
function noiseSigned(x, seed) {
  return (noise01(x, seed) - 0.5) * 2.0;
}
function drift(phase, speed, seed, amplitude) {
  const slow = noiseSigned(phase * speed * 0.3, seed);
  const medium = noiseSigned(phase * speed * 0.8, seed + 50.0);
  const micro = noiseSigned(phase * speed * 2.5, seed + 100.0);
  return (slow * 0.5 + medium * 0.35 + micro * 0.15) * amplitude;
}
```

`noise01` is a deterministic sum of three sines (not random); `noiseSigned ∈ [−1, 1]`; `drift` mixes three octaves
(0.3×, 0.8×, 2.5× speed; seeds +0, +50, +100; weights 0.5/0.35/0.15). Note `qBound(min, v, max)` argument order.

### 9.3 `MOTION_BURSTS` (B 17895–17909, verbatim)

```js
var MOTION_BURSTS = {
  NORMAL: { bodyX: 0, bodyY: 1, bodyZ: 0, headX: 0, headY: 1, headZ: 0, duration: 0.4, intensity: 0.5 },
  SMILE: { bodyX: 4, bodyY: 2, bodyZ: -2, headX: 5, headY: 3, headZ: -3, duration: 0.5, intensity: 0.8 },
  ANGRY: { bodyX: -5, bodyY: -3, bodyZ: 0, headX: -5, headY: -5, headZ: 0, duration: 0.5, intensity: 1.2 },
  SAD: { bodyX: -2, bodyY: -3, bodyZ: -2, headX: -3, headY: -4, headZ: -2, duration: 0.8, intensity: 0.7 },
  SURPRISED: { bodyX: -2, bodyY: 5, bodyZ: 2, headX: 0, headY: 8, headZ: 0, duration: 0.4, intensity: 1.5 },
  BLUSH: { bodyX: 8, bodyY: -2, bodyZ: -3, headX: 12, headY: -3, headZ: -5, duration: 0.6, intensity: 1.0 },
  WINK: { bodyX: 5, bodyY: 2, bodyZ: -2, headX: 6, headY: 3, headZ: -4, duration: 0.4, intensity: 0.9 },
  DISGUST: { bodyX: 5, bodyY: 4, bodyZ: 2, headX: 7, headY: -3, headZ: 3, duration: 0.5, intensity: 1.1 },
  SMUG: { bodyX: 2, bodyY: 2, bodyZ: -1, headX: 5, headY: 3, headZ: -1, duration: 0.7, intensity: 0.6 },
  THINKING: { bodyX: 0, bodyY: 0, bodyZ: 0, headX: 2, headY: -3, headZ: 1, duration: 0.8, intensity: 0.3 },
  PANIC: { bodyX: 0, bodyY: 0, bodyZ: 0, headX: 0, headY: 0, headZ: 0, duration: 0.2, intensity: 2.0 },
  SLEEPING: { bodyX: -1, bodyY: -2, bodyZ: -1, headX: -2, headY: -3, headZ: -2, duration: 4.5, intensity: 0.3 },
  SNEEZE: { bodyX: 0, bodyY: 0, bodyZ: 0, headX: 0, headY: 0, headZ: 0, duration: 2.2, intensity: 1.0 }
};
```

Lookup: `MOTION_BURSTS[activeBurstTag] || MOTION_BURSTS.NORMAL` (unknown tags play the NORMAL burst). `SLEEPING` and
`SNEEZE` entries are only used for their `duration` (4.5 s, 2.2 s): those two tags have dedicated curves (§9.7 step 7).
The `intensity`/axis values of SLEEPING and SNEEZE are unused.

### 9.4 Idle motion per emotion (B 17910–18010, verbatim)

```js
function getEmotionIdleMotion(tag, phase) {
  let bodyX = 0, bodyY = 0, bodyZ = 0;
  let headX = 0, headY = 0, headZ = 0;
  const t = tag.toUpperCase();
  if (t === "NORMAL") {
    bodyX = drift(phase, 0.6, 0.0, 1.8);
    bodyY = drift(phase, 0.4, 10.0, 0.8) + Math.sin(phase * 0.8) * 0.3;
    bodyZ = drift(phase, 0.35, 20.0, 0.6);
    headX = drift(phase, 0.5, 30.0, 3.0);
    headY = drift(phase, 0.4, 40.0, 2.0);
    headZ = drift(phase, 0.3, 50.0, 1.0);
  } else if (t === "SMILE") {
    bodyX = drift(phase, 1.2, 5.0, 3.0);
    bodyY = drift(phase, 1.0, 15.0, 1.5);
    bodyZ = drift(phase, 0.7, 25.0, 1.5);
    headX = drift(phase, 1.0, 35.0, 4.0);
    headY = drift(phase, 0.8, 45.0, 2.5);
    headZ = drift(phase, 0.9, 55.0, 2.0);
  } else if (t === "ANGRY") {
    const tension = noiseSigned(phase * 3.0, 7.0);
    bodyX = drift(phase, 1.5, 8.0, 2.5) + tension * 1.5;
    bodyY = drift(phase, 0.5, 18.0, 0.8);
    bodyZ = drift(phase, 2.0, 28.0, 1.2);
    headX = drift(phase, 1.8, 38.0, 3.0) + tension * 1.0;
    headY = drift(phase, 1.2, 48.0, 2.0);
    headZ = drift(phase, 0.8, 58.0, 1.0);
  } else if (t === "SAD") {
    bodyX = drift(phase, 0.4, 3.0, 2.5);
    bodyY = drift(phase, 0.3, 13.0, 1.5);
    bodyZ = drift(phase, 0.35, 23.0, 1.8);
    headX = drift(phase, 0.3, 33.0, 3.5);
    headY = drift(phase, 0.25, 43.0, 2.5);
    headZ = drift(phase, 0.3, 53.0, 1.5);
  } else if (t === "SURPRISED") {
    bodyX = drift(phase, 1.8, 6.0, 3.0);
    bodyY = drift(phase, 1.5, 16.0, 1.5) + Math.abs(Math.sin(phase * 1.5)) * 0.8;
    bodyZ = drift(phase, 1.0, 26.0, 1.5);
    headX = drift(phase, 2.0, 36.0, 5.0);
    headY = drift(phase, 1.8, 46.0, 4.5);
    headZ = drift(phase, 1.0, 56.0, 1.5);
  } else if (t === "BLUSH") {
    const fidget = noiseSigned(phase * 4.0, 99.0) * 0.75;
    bodyX = drift(phase, 0.8, 9.0, 3.0) + 1.0;
    bodyY = drift(phase, 0.6, 19.0, 1.0);
    bodyZ = drift(phase, 0.9, 29.0, 2.0);
    headX = drift(phase, 0.6, 39.0, 4.0) + 2.0 + fidget;
    headY = drift(phase, 0.5, 49.0, 3.0);
    headZ = drift(phase, 0.7, 59.0, 2.5);
  } else if (t === "WINK") {
    bodyX = drift(phase, 1.3, 4.0, 3.5);
    bodyY = drift(phase, 0.8, 14.0, 1.2);
    bodyZ = drift(phase, 0.9, 24.0, 2.0);
    headX = drift(phase, 1.0, 34.0, 4.5);
    headY = drift(phase, 0.7, 44.0, 2.5);
    headZ = drift(phase, 1.1, 54.0, 2.5);
  } else if (t === "DISGUST") {
    bodyX = drift(phase, 0.7, 2.0, 2.5) + 1.0;
    bodyY = drift(phase, 0.9, 12.0, 1.5);
    bodyZ = drift(phase, 0.5, 22.0, 1.0);
    headX = drift(phase, 0.8, 32.0, 3.5) + 1.5;
    headY = drift(phase, 0.6, 42.0, 2.5);
    headZ = drift(phase, 0.5, 52.0, 1.5);
  } else if (t === "SMUG") {
    bodyX = drift(phase, 0.5, 4.0, 2.0);
    bodyY = drift(phase, 0.4, 14.0, 1.0);
    bodyZ = drift(phase, 0.4, 24.0, 1.5);
    headX = drift(phase, 0.6, 34.0, 3.0) + 3.0;
    headY = drift(phase, 0.5, 44.0, 2.0);
    headZ = drift(phase, 0.5, 54.0, 1.0);
  } else if (t === "THINKING") {
    bodyX = drift(phase, 0.2, 5.0, 1.0);
    bodyY = drift(phase, 0.2, 15.0, 0.5);
    bodyZ = drift(phase, 0.2, 25.0, 0.5);
    headX = drift(phase, 0.2, 35.0, 1.0);
    headY = drift(phase, 0.2, 45.0, 1.0);
    headZ = drift(phase, 0.2, 55.0, 0.5);
  } else if (t === "PANIC") {
    const panic = noiseSigned(phase * 15.0, 999.0) * 2.0;
    bodyX = drift(phase, 2.0, 6.0, 2.0) + panic;
    bodyY = drift(phase, 2.0, 16.0, 2.0);
    bodyZ = drift(phase, 2.0, 26.0, 2.0);
    headX = drift(phase, 3.0, 36.0, 3.0) + panic;
    headY = panic * 1.5;
    headZ = panic * 0.5;
  } else if (t === "SLEEPING") {
    bodyX = drift(phase * 0.3, 0.15, 0.0, 0.5);
    bodyY = drift(phase * 0.3, 0.5, 10.0, 0.6) + Math.sin(phase * 0.8) * 0.6;
    bodyZ = drift(phase * 0.3, 0.1, 20.0, 0.3);
    headX = drift(phase * 0.3, 0.2, 30.0, 1.0);
    headY = drift(phase * 0.3, 0.15, 40.0, 0.5) + Math.sin(phase * 0.8) * 0.3;
    headZ = drift(phase * 0.3, 0.2, 50.0, 1.5);
  } else {
    bodyX = Math.sin(phase * 0.5) * 1.2;
    bodyY = Math.sin(phase * 0.3) * 0.6;
    bodyZ = Math.sin(phase * 0.4) * 0.8;
    headX = Math.sin(phase * 0.8) * 2.2;
    headY = Math.sin(phase * 0.6) * 1.5;
    headZ = Math.sin(phase * 0.7) * 0.9;
  }
  return { bodyX, bodyY, bodyZ, headX, headY, headZ };
}
```

Note: `tag.toUpperCase()` again; the fallback branch (any other tag, e.g. `"SNEEZE"`, `"FOO"`) uses plain sines of the
raw phase; `SLEEPING` slows the drift phase by 0.3 and adds a 0.8 rad/s sine "breathing" bob.

### 9.5 Expression hook (latent; B 18034–18074 verbatim)

```js
function findExpressionName(model, tag) {
  if (!model || !model._expressions || model._expressions.size === 0) return null;
  const needle = tag.toLowerCase();
  let preferredKeys = [];
  if (needle === "sad") preferredKeys = ["sad"];
  else if (needle === "smile") preferredKeys = ["smile"];
  else if (needle === "angry") preferredKeys = ["angry"];
  else if (needle === "surprised") preferredKeys = ["surprised"];
  else if (needle === "blush") preferredKeys = ["blush"];
  else if (needle === "wink") preferredKeys = ["wink"];
  else if (needle === "disgust") preferredKeys = ["disgust"];
  else if (needle === "smug") preferredKeys = ["smug"];
  else if (needle === "thinking") preferredKeys = ["thinking"];
  else if (needle === "panic") preferredKeys = ["surprised"];
  else preferredKeys = [needle];

  let selectedName = null;
  for (const key of preferredKeys) {
    for (const name of model._expressions.keys()) {
      if (name.toLowerCase() === key) {
        selectedName = name;
        break;
      }
    }
    if (selectedName) break;
  }

  if (!selectedName) {
    for (const name of model._expressions.keys()) {
      if (name.toLowerCase().includes(needle)) {
        selectedName = name;
        break;
      }
    }
  }

  if (!selectedName) {
    selectedName = model._expressions.keys().next().value;
  }
  return selectedName;
}
```

Used in `applyParameters` only if `model._expressions.size > 0` (never for this model, §2.4):
`if (lastAppliedEmotionTag !== currentEmotionTag) { name = findExpressionName(model, currentEmotionTag); if (name) {
model.setExpression(name); lastAppliedEmotionTag = currentEmotionTag; } }`. `setExpression(name)` starts the expression
motion through `_expressionManager.startMotion(motion, false)`. If expressions were ever added: `"PANIC"` maps to
`surprised`, a tag with no match falls back to the first key **containing** the lowercase tag, then to the first
expression in insertion order (so *any* tag selects *some* expression). V2: port verbatim; it stays dormant.

### 9.6 Public setters (B 18131–18166, verbatim)

```js
  setEmotion(emotion) {
    const normalizedTag = emotion.trim().toUpperCase();
    if (normalizedTag === this.emotion) {
      return;
    }
    const isWakingUp = (this.emotion === "SLEEPING" && normalizedTag !== "SLEEPING");
    this.emotion = normalizedTag;

    if (isWakingUp) {
      this.wakeUpTimer = 2.5; // Startled wake-up duration (2.5 seconds)
      this.activeBurstTag = "SURPRISED";
      this.burstTimer = 0;
      this.burstProgress = 0.0;
      this.idlePhase = 0.0;
      this.currentEmotionTag = "SURPRISED";
    } else {
      this.wakeUpTimer = 0;
      if (normalizedTag !== this.currentEmotionTag) {
        this.activeBurstTag = normalizedTag;
        this.burstTimer = 0;
        this.burstProgress = 0.0;
        this.idlePhase = 0.0;
        this.currentEmotionTag = normalizedTag;
      }
    }
  }
  setSpokenChar(spokenChar) {
    this.spokenChar = spokenChar || "";
  }
  setLipSync(value) {
    this.lipSyncValue = Math.max(0, Math.min(1, value));
  }
  setEyeTracking(x, y) {
    this.eyeTrackingX = Math.max(-1.0, Math.min(1.0, x));
    this.eyeTrackingY = Math.max(-1.0, Math.min(1.0, y));
  }
```

Semantics:

* `setEmotion(e)`: `normalized = e.trim().toUpperCase()` (JS Unicode-aware; Rust: `str::trim()` + `to_uppercase()`;
  differences only for U+FEFF/U+0085 and exotic casing — acceptable). Same as current `emotion` → **no-op** (no burst,
  no idle-phase reset). Otherwise `emotion = normalized` and:
  * waking up (previous `emotion === "SLEEPING"` and new ≠ SLEEPING): `wakeUpTimer = 2.5`, `activeBurstTag =
    "SURPRISED"`, `burstTimer = 0`, `burstProgress = 0`, `idlePhase = 0`, `currentEmotionTag = "SURPRISED"`.
  * else: `wakeUpTimer = 0` (cancels a running wake-up immediately) and, only if `normalized !== currentEmotionTag`:
    `activeBurstTag = normalized`, `burstTimer = 0`, `burstProgress = 0`, `idlePhase = 0`, `currentEmotionTag = normalized`.
    (So switching to the emotion that `currentEmotionTag` already holds — e.g. `"SURPRISED"` during a wake-up — does not
    restart a burst.)
  * Calls before the model has loaded take effect immediately (pure state).
* `setSpokenChar(c)`: `spokenChar = c || ""`.
* `setLipSync(v)`: `lipSyncValue = Math.max(0, Math.min(1, v))` (NaN stays NaN in JS → "not speaking" and blocks the
  sneeze timer; V2: map NaN to 0 *and* treat it as "not speaking" — the sneeze difference is negligible, document it).
* `setEyeTracking(x, y)`: clamp both to [−1, 1] (NaN → NaN, later read as `|| 0` → 0; V2: NaN → 0).

### 9.7 `applyParameters(model)` — the complete algorithm (B 18275–18603; verbatim code in 9.8)

Called once per frame by `LAppLive2DManager.onUpdate` **after** `model.update()` and **before** core update #2.
Order of operations (each step uses the values produced by the previous ones):

1. **Guards.** Return if `destroyed`; return if `model.getModel()` is null (before the moc is loaded).
   `ids = resolveParameterHandles()` (interned once, §9.9).
2. **Target.** `target = EMOTION_TARGETS[emotion] ?? EMOTION_TARGETS.NORMAL` (may be replaced in step 5).
3. **Clock 2.** `now = performance.now() / 1000`; `if (!_lastUpdateTime) _lastUpdateTime = now` (falsy check, so the
   first call yields Δ = 0); `dt = min(now − _lastUpdateTime, 0.1)`; `_lastUpdateTime = now`; `userTime += dt`;
   `idlePhase += dt`. No lower bound (performance.now is monotonic).
4. **Expression hook** (§9.5; inactive).
5. **Timers.**
   * 5a **Sneeze easter egg:** if `emotion === "NORMAL" && burstProgress >= 1 && wakeUpTimer <= 0 && lipSyncValue <= 0.01`:
     `sneezeEasterEggTimer += dt`; if `>= 45.0` → reset to 0 and, if `Math.random() < 0.15`, start a burst:
     `activeBurstTag = "SNEEZE"; burstTimer = 0; burstProgress = 0` (emotion and `currentEmotionTag` unchanged,
     `idlePhase` not reset). Otherwise (any condition false) `sneezeEasterEggTimer = 0`.
   * 5b **Sleep nodding:** if `emotion === "SLEEPING"`: `sleepingNodTimer += dt`; if `>= nextSleepingNodInterval`:
     `sleepingNodTimer = 0; nextSleepingNodInterval = 20.0 + Math.random() * 20.0; activeBurstTag = "SLEEPING";
     burstProgress = 0; burstTimer = 0`. Else `sleepingNodTimer = 0`. (`nextSleepingNodInterval` starts at 30.0 and is
     **not** reset when sleep ends — the first periodic nod of a later sleep uses the last drawn interval.)
   * 5c **Startled wake-up:** if `wakeUpTimer > 0`: `wakeUpTimer -= dt`;
     * if now `<= 0`: `wakeUpTimer = 0; wakeUpHeadX = 0; wakeUpGazeX = 0; currentEmotionTag = emotion` (target stays the
       plain target this frame; no new burst, `idlePhase` not reset);
     * else: `blend = 1; if (wakeUpTimer < 1.7) blend = clamp(wakeUpTimer / 1.7, 0, 1)`;
       `target = { each of the 13 numeric fields: lerp(target[f], EMOTION_TARGETS.SURPRISED[f], blend) }` — **without
       `isWink`** (QUIRK: waking into WINK keeps the right eye open until the wake-up ends);
       `elapsed = 2.5 − wakeUpTimer`; if `0.4 <= elapsed < 1.4`: `wave = sin((elapsed − 0.4) · 2π)`,
       `wakeUpHeadX = 15·wave`, `wakeUpGazeX = 0.7·wave` (look right, then left, one full period in 1 s); else both 0.
     * if `wakeUpTimer <= 0` at entry: `wakeUpHeadX = wakeUpGazeX = 0`.
6. **Emotion smoothing.** `s = min(dt · 3.0, 1.0)` (source comment: `_emotionLerpSpeed = 3.0 in C++`);
   `current = { ...current, f: lerp(current.f, target.f, s) }` for the 13 fields (browY, browForm, browAngle, eyeOpen,
   eyeSmile, mouthForm, bodyAngleX/Y/Z, headAngleX/Y/Z, cheek). `current` never gets `isWink`.
7. **Bursts.** Locals `burstBodyX/Y/Z = burstHeadX/Y/Z = 0`, `sneezeMouthOpen = 0`, `sneezeEyeOpen = 1`;
   `isSneezeActive = burstProgress < 1 && activeBurstTag === "SNEEZE"` (**evaluated before** this frame's progress
   update). If `burstProgress < 1`:
   * `burstTimer += dt`; `b = MOTION_BURSTS[activeBurstTag] || MOTION_BURSTS.NORMAL`;
     `burstProgress = clamp(burstTimer / b.duration, 0, 1)`; `t = burstProgress`.
   * **SLEEPING (nod, 4.5 s, three nods):**
     `nod = t < 0.33 ? sin(t/0.33·π)·−10 : t < 0.66 ? sin((t−0.33)/0.33·π)·−10 : sin((t−0.66)/0.34·π)·−18`;
     `burstHeadY = nod`; `burstBodyY = t·−1.5 + nod·0.15`.
   * **SNEEZE (2.2 s):**
     * `t < 0.5`: `u = t/0.5`; `headY = 12u`, `headX = −3u`, `bodyY = 1.5u`, `sneezeEyeOpen = 1 − 0.9u`.
     * `0.5 <= t < 0.65`: `u = (t−0.5)/0.15`; `headY = 12 − 32u`, `headX = −3 + 6u`, `bodyY = 1.5 − 5u`,
       `sneezeEyeOpen = 0.1·(1−u)`, `sneezeMouthOpen = 0.9u`.
     * `t >= 0.65`: `u = (t−0.65)/0.35`; `headY = −20 + 20u`, `headX = 3 − 3u`, `bodyY = −3.5 + 3.5u`,
       `sneezeEyeOpen = u`, `sneezeMouthOpen = 0.9·(1−u)`.
     (Phases in seconds: wind-up 0–1.1, "achoo" 1.1–1.43, recovery 1.43–2.2.)
   * **Any other tag (spring overshoot):** `spring = sin(t·π·2.5)·(1−t)²`; `k = b.intensity·spring`;
     `burstBodyX = b.bodyX·k`, `burstBodyY = b.bodyY·k`, `burstBodyZ = b.bodyZ·k`, `burstHeadX = b.headX·k`,
     `burstHeadY = b.headY·k`, `burstHeadZ = b.headZ·k`. (spring peaks 0.64 at t = 0.2, crosses 0 at t = 0.4, minimum
     ≈ −0.177 at t = 0.5, 0 at t = 0.8 and 1.)
   * `activeBurstTag` is never cleared after completion (it just has `burstProgress == 1`).
8. **Blink FSM** ("matching QT C++ UpdateBlink"). `blinkTimer += dt` first; `blinkValue = 1`;
   * state 0 (open): if `blinkTimer >= blinkInterval` → state 1, `blinkTimer = 0` (value stays 1 this frame);
   * state 1 (closing): `blinkValue = 1 − min(blinkTimer/0.1, 1)`; if `blinkTimer >= 0.1` → state 2, `blinkTimer = 0`;
   * state 2 (opening): `blinkValue = min(blinkTimer/0.1, 1)`; if `blinkTimer >= 0.1` → state 0, `blinkTimer = 0`,
     `blinkInterval = 2.5 + Math.random() * 3.5`.
   (`blinkDuration = 0.1` for both phases; no "closed" hold; the FSM runs in every emotion including SLEEPING.)
9. **Eye openness.**
   `nodEyeOpen = 1`; if `emotion === "SLEEPING" && burstProgress < 1` (**after** step 7's update):
   `u = t<0.33 ? t/0.33 : t<0.66 ? (t−0.33)/0.33 : (t−0.66)/0.34` with `t = burstProgress`; `nodEyeOpen = 0.18·sin(u·π)`
   (eyes flutter slightly open during each nod). `baseEyeOpen = current.eyeOpen`; if the SLEEPING-nod condition holds
   → `baseEyeOpen = nodEyeOpen`; else if `isSneezeActive` → `baseEyeOpen = sneezeEyeOpen`.
   `eyeOpen = max(0, baseEyeOpen · blinkValue)`.
10. **Face writes** (in this order): `ParamBrowLY = ParamBrowRY = current.browY`; `ParamBrowLForm = ParamBrowRForm =
    current.browForm`; `ParamBrowLAngle = ParamBrowRAngle = current.browAngle`; `ParamEyeLSmile = ParamEyeRSmile =
    current.eyeSmile`; `ParamMouthForm = current.mouthForm`; `ParamCheek = current.cheek`; `ParamEyeLOpen = eyeOpen`;
    `ParamEyeROpen = target.isWink ? 0 : eyeOpen` (`target` as possibly replaced in 5c).
11. **Idle + burst smoothing.** `idle = getEmotionIdleMotion(currentEmotionTag, idlePhase)`; `m = clamp(dt·6, 0, 1)`;
    `smoothedBodyX = lerp(smoothedBodyX, burstBodyX + idle.bodyX, m)`, same for BodyY/BodyZ, `smoothedHeadX =
    lerp(smoothedHeadX, burstHeadX + idle.headX + wakeUpHeadX, m)`, `smoothedHeadY/Z` analogous (no wake term).
12. **Gaze.** `disableGaze = wakeUpTimer > 0 || emotion === "SLEEPING" || currentEmotionTag === "SLEEPING"`;
    `targetGazeX = wakeUpTimer > 0 ? wakeUpGazeX : (disableGaze ? 0 : (eyeTrackingX || 0))`;
    `targetGazeY = disableGaze ? 0 : (eyeTrackingY || 0)`; `g = clamp(dt·5, 0, 1)`;
    `smoothedGazeX = lerp(smoothedGazeX, targetGazeX, g)`, `smoothedGazeY = lerp(smoothedGazeY, targetGazeY, g)`.
13. **Body/head/eyeball writes:**
    `ParamBodyAngleX = current.bodyAngleX + smoothedBodyX + smoothedGazeX·5`;
    `ParamBodyAngleY = current.bodyAngleY + smoothedBodyY + smoothedGazeY·5`;
    `ParamBodyAngleZ = current.bodyAngleZ + smoothedBodyZ`;
    `ParamAngleX = current.headAngleX + smoothedHeadX + smoothedGazeX·25`;
    `ParamAngleY = current.headAngleY + smoothedHeadY + smoothedGazeY·25`;
    `ParamAngleZ = current.headAngleZ + smoothedHeadZ`;
    `ParamEyeBallX = smoothedGazeX`; `ParamEyeBallY = smoothedGazeY`.
14. **Breath write:** `speed = emotion === "SLEEPING" ? 0.8 : 1.2`; `ParamBreath = (sin(userTime·speed) + 1)·0.5`
    (phase jumps when the speed switches — reproduce).
15. **Mouth:**
    * if `isSneezeActive`: `mouthOpen = lerp(mouthOpen, sneezeMouthOpen, 16·dt)`;
    * else if `lipSyncValue > 0.01`: `target = 0.5`; if `spokenChar` non-empty → `target = getVowelMouthOpening(spokenChar)`;
      else `target = |sin(userTime·12)|·0.5 + |sin(userTime·7.3)|·0.3 + noise01(userTime·8, 5)·0.2`;
      `final = clamp(target · clamp(lipSyncValue, 0, 1), 0, 1)`; `mouthOpen = lerp(mouthOpen, final, 14·dt)`;
    * else `mouthOpen = lerp(mouthOpen, 0, 10·dt)`.
    `ParamMouthOpenY = mouthOpen`.
    **QUIRK (reproduce):** the factors `16·dt`, `14·dt`, `10·dt` are **not clamped**; with the 0.1 s cap they reach
    1.6/1.4/1.0, so at low frame rates `mouthOpen` overshoots (internal state may leave [0, 1]; the model clamps the
    written value).

All writes go through `setParam(model, id, value)` = `model.getModel()?.setParameterValueById(id, value)` (weight 1,
clamped, f32). Parameters the driver does not write this frame keep the values from the SDK stage.

### 9.8 Verbatim `applyParameters` (B 18275–18603) incl. `getVowelMouthOpening`

```js
  applyParameters(model) {
    var _a;
    if (this.destroyed) {
      return;
    }
    const cubismModel = model.getModel();
    if (!cubismModel) {
      return;
    }
    const ids = this.resolveParameterHandles();
    let target = (_a = EMOTION_TARGETS[this.emotion]) != null ? _a : EMOTION_TARGETS.NORMAL;
    const now = performance.now() / 1000.0;
    if (!this._lastUpdateTime) this._lastUpdateTime = now;
    const deltaSeconds = Math.min(now - this._lastUpdateTime, 0.1); // cap at 100ms
    this._lastUpdateTime = now;
    this.userTime += deltaSeconds;
    this.idlePhase += deltaSeconds;

    // Set initial/new expression once model is loaded
    if (model && model._expressions && model._expressions.size > 0) {
      if (this.lastAppliedEmotionTag !== this.currentEmotionTag) {
        const selectedName = findExpressionName(model, this.currentEmotionTag);
        if (selectedName) {
          model.setExpression(selectedName);
          this.lastAppliedEmotionTag = this.currentEmotionTag;
        }
      }
    }

    // 1. Sneeze Easter Egg (idle normal state, not speaking, not waking up, not sleeping)
    if (this.emotion === "NORMAL" && this.burstProgress >= 1.0 && this.wakeUpTimer <= 0.0 && this.lipSyncValue <= 0.01) {
      this.sneezeEasterEggTimer += deltaSeconds;
      if (this.sneezeEasterEggTimer >= 45.0) {
        this.sneezeEasterEggTimer = 0.0;
        if (Math.random() < 0.15) {
          this.activeBurstTag = "SNEEZE";
          this.burstTimer = 0.0;
          this.burstProgress = 0.0;
        }
      }
    } else {
      this.sneezeEasterEggTimer = 0.0;
    }

    // 2. Periodic sleep nodding
    if (this.emotion === "SLEEPING") {
      this.sleepingNodTimer += deltaSeconds;
      if (this.sleepingNodTimer >= this.nextSleepingNodInterval) {
        this.sleepingNodTimer = 0.0;
        this.nextSleepingNodInterval = 20.0 + Math.random() * 20.0; // 20 to 40 seconds
        this.activeBurstTag = "SLEEPING";
        this.burstProgress = 0.0;
        this.burstTimer = 0.0;
      }
    } else {
      this.sleepingNodTimer = 0.0;
    }

    // 3. Startled wake-up transition blend
    if (this.wakeUpTimer > 0.0) {
      this.wakeUpTimer -= deltaSeconds;
      if (this.wakeUpTimer <= 0.0) {
        this.wakeUpTimer = 0.0;
        this.wakeUpHeadX = 0.0;
        this.wakeUpGazeX = 0.0;
        this.currentEmotionTag = this.emotion; // Restore emotion tag when wakeUp timer expires
      } else {
        const targetBase = target;
        const targetSurprised = EMOTION_TARGETS.SURPRISED;
        let blend = 1.0;
        if (this.wakeUpTimer < 1.7) {
          blend = Math.max(0.0, Math.min(1.0, this.wakeUpTimer / 1.7));
        }

        target = {
          browY: lerp(targetBase.browY, targetSurprised.browY, blend),
          browForm: lerp(targetBase.browForm, targetSurprised.browForm, blend),
          browAngle: lerp(targetBase.browAngle, targetSurprised.browAngle, blend),
          eyeOpen: lerp(targetBase.eyeOpen, targetSurprised.eyeOpen, blend),
          eyeSmile: lerp(targetBase.eyeSmile, targetSurprised.eyeSmile, blend),
          mouthForm: lerp(targetBase.mouthForm, targetSurprised.mouthForm, blend),
          cheek: lerp(targetBase.cheek, targetSurprised.cheek, blend),
          bodyAngleX: lerp(targetBase.bodyAngleX, targetSurprised.bodyAngleX, blend),
          bodyAngleY: lerp(targetBase.bodyAngleY, targetSurprised.bodyAngleY, blend),
          bodyAngleZ: lerp(targetBase.bodyAngleZ, targetSurprised.bodyAngleZ, blend),
          headAngleX: lerp(targetBase.headAngleX, targetSurprised.headAngleX, blend),
          headAngleY: lerp(targetBase.headAngleY, targetSurprised.headAngleY, blend),
          headAngleZ: lerp(targetBase.headAngleZ, targetSurprised.headAngleZ, blend),
        };

        // Startled look around (look right, then look left)
        const elapsed = 2.5 - this.wakeUpTimer;
        if (elapsed >= 0.4 && elapsed < 1.4) {
          const wave = Math.sin((elapsed - 0.4) * (2.0 * Math.PI / 1.0));
          this.wakeUpHeadX = wave * 15.0; // Look right (+15), then look left (-15)
          this.wakeUpGazeX = wave * 0.7;  // Eyeball look right (+0.7), then left (-0.7)
        } else {
          this.wakeUpHeadX = 0.0;
          this.wakeUpGazeX = 0.0;
        }
      }
    } else {
      this.wakeUpHeadX = 0.0;
      this.wakeUpGazeX = 0.0;
    }

    // Smooth emotion blend
    const smooth = Math.min(deltaSeconds * 3.0, 1.0); // _emotionLerpSpeed = 3.0 in C++
    this.current = __spreadProps(__spreadValues({}, this.current), {
      browY: lerp(this.current.browY, target.browY, smooth),
      browForm: lerp(this.current.browForm, target.browForm, smooth),
      browAngle: lerp(this.current.browAngle, target.browAngle, smooth),
      eyeOpen: lerp(this.current.eyeOpen, target.eyeOpen, smooth),
      eyeSmile: lerp(this.current.eyeSmile, target.eyeSmile, smooth),
      mouthForm: lerp(this.current.mouthForm, target.mouthForm, smooth),
      bodyAngleX: lerp(this.current.bodyAngleX, target.bodyAngleX, smooth),
      bodyAngleY: lerp(this.current.bodyAngleY, target.bodyAngleY, smooth),
      bodyAngleZ: lerp(this.current.bodyAngleZ, target.bodyAngleZ, smooth),
      headAngleX: lerp(this.current.headAngleX, target.headAngleX, smooth),
      headAngleY: lerp(this.current.headAngleY, target.headAngleY, smooth),
      headAngleZ: lerp(this.current.headAngleZ, target.headAngleZ, smooth),
      cheek: lerp(this.current.cheek, target.cheek, smooth)
    });

    // 4. Handle Bursts (Nodding / Sneeze / Transitions)
    let burstBodyX = 0.0, burstBodyY = 0.0, burstBodyZ = 0.0;
    let burstHeadX = 0.0, burstHeadY = 0.0, burstHeadZ = 0.0;
    let sneezeMouthOpen = 0.0;
    let sneezeEyeOpen = 1.0;
    const isSneezeActive = (this.burstProgress < 1.0 && this.activeBurstTag === "SNEEZE");

    if (this.burstProgress < 1.0) {
      this.burstTimer += deltaSeconds;
      const activeBurst = MOTION_BURSTS[this.activeBurstTag] || MOTION_BURSTS.NORMAL;
      const duration = activeBurst.duration;
      this.burstProgress = Math.max(0.0, Math.min(1.0, this.burstTimer / duration));
      const t01 = this.burstProgress;

      if (this.activeBurstTag === "SLEEPING") {
        let nodCurve = 0.0;
        if (t01 < 0.33) {
          nodCurve = Math.sin((t01 / 0.33) * Math.PI) * -10.0;
        } else if (t01 < 0.66) {
          nodCurve = Math.sin(((t01 - 0.33) / 0.33) * Math.PI) * -10.0;
        } else {
          nodCurve = Math.sin(((t01 - 0.66) / 0.34) * Math.PI) * -18.0;
        }
        burstHeadY = nodCurve;
        burstBodyY = t01 * -1.5 + (nodCurve * 0.15);
      } else if (this.activeBurstTag === "SNEEZE") {
        if (t01 < 0.5) {
          let localT = t01 / 0.5;
          burstHeadY = localT * 12.0;
          burstHeadX = localT * -3.0;
          burstBodyY = localT * 1.5;
          sneezeEyeOpen = 1.0 - (localT * 0.9);
        } else if (t01 < 0.65) {
          let localT = (t01 - 0.5) / 0.15;
          burstHeadY = 12.0 - (localT * 32.0);
          burstHeadX = -3.0 + (localT * 6.0);
          burstBodyY = 1.5 - (localT * 5.0);
          sneezeEyeOpen = 0.1 * (1.0 - localT);
          sneezeMouthOpen = localT * 0.9;
        } else {
          let localT = (t01 - 0.65) / 0.35;
          burstHeadY = -20.0 + (localT * 20.0);
          burstHeadX = 3.0 - (localT * 3.0);
          burstBodyY = -3.5 + (localT * 3.5);
          sneezeEyeOpen = 0.0 + (localT * 1.0);
          sneezeMouthOpen = 0.9 * (1.0 - localT);
        }
      } else {
        // General spring-physics overshoot animation
        const spring = Math.sin(t01 * Math.PI * 2.5) * (1.0 - t01) * (1.0 - t01);
        const intensity = activeBurst.intensity * spring;
        burstBodyX = activeBurst.bodyX * intensity;
        burstBodyY = activeBurst.bodyY * intensity;
        burstBodyZ = activeBurst.bodyZ * intensity;
        burstHeadX = activeBurst.headX * intensity;
        burstHeadY = activeBurst.headY * intensity;
        burstHeadZ = activeBurst.headZ * intensity;
      }
    }

    // Blink & Eyes
    // Blink FSM (matching QT C++ UpdateBlink)
    this.blinkTimer += deltaSeconds;
    let blinkValue = 1.0; // fully open
    if (this.blinkState === 0) { // Open
      if (this.blinkTimer >= this.blinkInterval) {
        this.blinkState = 1; // start closing
        this.blinkTimer = 0.0;
      }
    } else if (this.blinkState === 1) { // Closing
      const t = Math.min(this.blinkTimer / this.blinkDuration, 1.0);
      blinkValue = 1.0 - t;
      if (this.blinkTimer >= this.blinkDuration) {
        this.blinkState = 2; // start opening
        this.blinkTimer = 0.0;
      }
    } else if (this.blinkState === 2) { // Opening
      const t = Math.min(this.blinkTimer / this.blinkDuration, 1.0);
      blinkValue = t;
      if (this.blinkTimer >= this.blinkDuration) {
        this.blinkState = 0; // back to open
        this.blinkTimer = 0.0;
        this.blinkInterval = 2.5 + Math.random() * 3.5;
      }
    }
    const blink = blinkValue;
    let nodEyeOpen = 1.0;
    if (this.emotion === "SLEEPING" && this.burstProgress < 1.0) {
      const t01 = this.burstProgress;
      let localT = 0.0;
      if (t01 < 0.33) {
        localT = t01 / 0.33;
      } else if (t01 < 0.66) {
        localT = (t01 - 0.33) / 0.33;
      } else {
        localT = (t01 - 0.66) / 0.34;
      }
      nodEyeOpen = 0.18 * Math.sin(localT * Math.PI);
    }
    let baseEyeOpen = this.current.eyeOpen;
    if (this.emotion === "SLEEPING" && this.burstProgress < 1.0) {
      baseEyeOpen = nodEyeOpen;
    } else if (isSneezeActive) {
      baseEyeOpen = sneezeEyeOpen;
    }
    const eyeOpen = Math.max(0.0, baseEyeOpen * blink);

    // Set parameter values
    this.setParam(model, ids.browLY, this.current.browY);
    this.setParam(model, ids.browRY, this.current.browY);
    this.setParam(model, ids.browLForm, this.current.browForm);
    this.setParam(model, ids.browRForm, this.current.browForm);
    this.setParam(model, ids.browLAngle, this.current.browAngle);
    this.setParam(model, ids.browRAngle, this.current.browAngle);
    this.setParam(model, ids.eyeLSmile, this.current.eyeSmile);
    this.setParam(model, ids.eyeRSmile, this.current.eyeSmile);
    this.setParam(model, ids.mouthForm, this.current.mouthForm);
    this.setParam(model, ids.cheek, this.current.cheek);
    this.setParam(model, ids.eyeLOpen, eyeOpen);
    this.setParam(model, ids.eyeROpen, target.isWink ? 0 : eyeOpen);

    // 5. Idle movement (breathing & drift) with smoothing
    const idleMotion = getEmotionIdleMotion(this.currentEmotionTag, this.idlePhase);
    const motionSmoothT = Math.max(0.0, Math.min(deltaSeconds * 6.0, 1.0));
    this.smoothedBodyX = lerp(this.smoothedBodyX, burstBodyX + idleMotion.bodyX, motionSmoothT);
    this.smoothedBodyY = lerp(this.smoothedBodyY, burstBodyY + idleMotion.bodyY, motionSmoothT);
    this.smoothedBodyZ = lerp(this.smoothedBodyZ, burstBodyZ + idleMotion.bodyZ, motionSmoothT);
    this.smoothedHeadX = lerp(this.smoothedHeadX, burstHeadX + idleMotion.headX + this.wakeUpHeadX, motionSmoothT);
    this.smoothedHeadY = lerp(this.smoothedHeadY, burstHeadY + idleMotion.headY, motionSmoothT);
    this.smoothedHeadZ = lerp(this.smoothedHeadZ, burstHeadZ + idleMotion.headZ, motionSmoothT);
    // Gaze tracking (eyeball look) — compute smoothed gaze before body/head params (QT parity)
    const disableGaze = (this.wakeUpTimer > 0.0) || (this.emotion === "SLEEPING") || (this.currentEmotionTag === "SLEEPING");
    const targetGazeX = this.wakeUpTimer > 0.0 ? this.wakeUpGazeX : (disableGaze ? 0.0 : (this.eyeTrackingX || 0.0));
    const targetGazeY = disableGaze ? 0.0 : (this.eyeTrackingY || 0.0);

    const gazeSmoothT = Math.max(0.0, Math.min(1.0, deltaSeconds * 5.0));
    this.smoothedGazeX = lerp(this.smoothedGazeX, targetGazeX, gazeSmoothT);
    this.smoothedGazeY = lerp(this.smoothedGazeY, targetGazeY, gazeSmoothT);

    this.setParam(model, ids.bodyAngleX, this.current.bodyAngleX + this.smoothedBodyX + (this.smoothedGazeX * 5.0));
    this.setParam(model, ids.bodyAngleY, this.current.bodyAngleY + this.smoothedBodyY + (this.smoothedGazeY * 5.0));
    this.setParam(model, ids.bodyAngleZ, this.current.bodyAngleZ + this.smoothedBodyZ);
    this.setParam(model, ids.angleX, this.current.headAngleX + this.smoothedHeadX + (this.smoothedGazeX * 25.0));
    this.setParam(model, ids.angleY, this.current.headAngleY + this.smoothedHeadY + (this.smoothedGazeY * 25.0));
    this.setParam(model, ids.angleZ, this.current.headAngleZ + this.smoothedHeadZ);

    this.setParam(model, ids.eyeBallX, this.smoothedGazeX);
    this.setParam(model, ids.eyeBallY, this.smoothedGazeY);

    // Breath cycles
    const breathCycleSpeed = this.emotion === "SLEEPING" ? 0.8 : 1.2;
    this.setParam(model, ids.breath, (Math.sin(this.userTime * breathCycleSpeed) + 1) * 0.5);

    // 6. Lip sync & Vowel mouth opening
    if (isSneezeActive) {
      this.mouthOpen = lerp(this.mouthOpen, sneezeMouthOpen, 16.0 * deltaSeconds);
    } else if (this.lipSyncValue > 0.01) {
      let targetMouth = 0.5;
      if (this.spokenChar) {
        targetMouth = getVowelMouthOpening(this.spokenChar);
      } else {
        const syllable = Math.abs(Math.sin(this.userTime * 12));
        const subBeat = Math.abs(Math.sin(this.userTime * 7.3));
        const flutter = noise01(this.userTime * 8.0, 5.0);
        targetMouth = syllable * 0.5 + subBeat * 0.3 + flutter * 0.2;
      }
      const finalMouth = Math.max(0.0, Math.min(1.0, targetMouth * Math.max(0.0, Math.min(1.0, this.lipSyncValue))));
      this.mouthOpen = lerp(this.mouthOpen, finalMouth, 14.0 * deltaSeconds);
    } else {
      this.mouthOpen = lerp(this.mouthOpen, 0.0, 10.0 * deltaSeconds);
    }
    this.setParam(model, ids.mouthOpenY, this.mouthOpen);

    // Helper function definitions
    function getVowelMouthOpening(chr) {
      if (!chr) return 0.0;
      const c = chr.charAt(0);

      if (c === 'a' || c === 'A') return 0.9;
      if (c === 'o' || c === 'O') return 0.7;
      if (c === 'e' || c === 'E') return 0.6;
      if (c === 'u' || c === 'U') return 0.4;
      if (c === 'i' || c === 'I' || c === 'í' || c === 'Í') return 0.2;

      const aVowels = "あかさたなはまやらわがざだばぱぁゃァカサタナハマヤラワガザダバパァャ";
      if (aVowels.includes(c)) return 0.9;

      const oVowels = "おこそとのほもよろをごぞどぼぽぉょォコソトノホモヨロヲゴゾドボポォョ";
      if (oVowels.includes(c)) return 0.7;

      const eVowels = "えけせてねへめれげぜでべぺぇェケセテネヘメレゲゼデベペェ";
      if (eVowels.includes(c)) return 0.6;

      const uVowels = "うくすつぬふむゆるぐずづぶぷぅゅゥクスツヌフムユルグズヅブプゥュ";
      if (uVowels.includes(c)) return 0.4;

      const iVowels = "いきしちにひみりぎじぢびぴぃィキシチニヒミリギジヂビピィ";
      if (iVowels.includes(c)) return 0.2;

      const closedChars = "んン。、？！.,?! 　\n\r\t-_";
      if (closedChars.includes(c)) return 0.0;

      return 0.5;
    }
  }
```

### 9.9 Parameter handles and final formulas

`resolveParameterHandles()` (B 18237–18267) interns these 22 ids once (`CubismFramework.getIdManager().getId(…)`):

| handle | Id | final value written each frame |
|---|---|---|
| browLY, browRY | `ParamBrowLY`, `ParamBrowRY` | `current.browY` |
| browLForm, browRForm | `ParamBrowLForm`, `ParamBrowRForm` | `current.browForm` |
| browLAngle, browRAngle | `ParamBrowLAngle`, `ParamBrowRAngle` | `current.browAngle` |
| eyeLSmile, eyeRSmile | `ParamEyeLSmile`, `ParamEyeRSmile` | `current.eyeSmile` |
| mouthForm | `ParamMouthForm` | `current.mouthForm` |
| cheek | `ParamCheek` | `current.cheek` |
| eyeLOpen | `ParamEyeLOpen` | `max(0, baseEyeOpen·blink)` |
| eyeROpen | `ParamEyeROpen` | `target.isWink ? 0 : eyeLOpen value` |
| bodyAngleX | `ParamBodyAngleX` | `current.bodyAngleX + smoothedBodyX + 5·smoothedGazeX` |
| bodyAngleY | `ParamBodyAngleY` | `current.bodyAngleY + smoothedBodyY + 5·smoothedGazeY` |
| bodyAngleZ | `ParamBodyAngleZ` | `current.bodyAngleZ + smoothedBodyZ` |
| angleX | `ParamAngleX` | `current.headAngleX + smoothedHeadX + 25·smoothedGazeX` |
| angleY | `ParamAngleY` | `current.headAngleY + smoothedHeadY + 25·smoothedGazeY` |
| angleZ | `ParamAngleZ` | `current.headAngleZ + smoothedHeadZ` |
| eyeBallX, eyeBallY | `ParamEyeBallX`, `ParamEyeBallY` | `smoothedGazeX`, `smoothedGazeY` |
| breath | `ParamBreath` | `(sin(userTime·(SLEEPING ? 0.8 : 1.2)) + 1)/2` |
| mouthOpenY | `ParamMouthOpenY` | `mouthOpen` |

All are clamped to the ranges in §2.2 on write (e.g. head X saturates at ±30, body at ±10).

### 9.10 Vowel table (`getVowelMouthOpening`, first UTF-16 code unit of `spokenChar`)

Order of checks (first match wins): `''`/falsy → 0.0; `a A` → 0.9; `o O` → 0.7; `e E` → 0.6; `u U` → 0.4;
`i I í Í` → 0.2; then membership in these strings (verbatim):

* `aVowels` → **0.9** — source literal `"あかさたなはまやらわがざだばぱぁゃァカサタナハマヤラワガザダバパァャ"` (34 chars): U+3042 U+304B U+3055 U+305F U+306A U+306F U+307E U+3084 U+3089 U+308F U+304C U+3056 U+3060 U+3070 U+3071 U+3041 U+3083 U+30A1 U+30AB U+30B5 U+30BF U+30CA U+30CF U+30DE U+30E4 U+30E9 U+30EF U+30AC U+30B6 U+30C0 U+30D0 U+30D1 U+30A1 U+30E3
* `oVowels` → **0.7** — source literal `"おこそとのほもよろをごぞどぼぽぉょォコソトノホモヨロヲゴゾドボポォョ"` (34 chars): U+304A U+3053 U+305D U+3068 U+306E U+307B U+3082 U+3088 U+308D U+3092 U+3054 U+305E U+3069 U+307C U+307D U+3049 U+3087 U+30A9 U+30B3 U+30BD U+30C8 U+30CE U+30DB U+30E2 U+30E8 U+30ED U+30F2 U+30B4 U+30BE U+30C9 U+30DC U+30DD U+30A9 U+30E7
* `eVowels` → **0.6** — source literal `"えけせてねへめれげぜでべぺぇェケセテネヘメレゲゼデベペェ"` (28 chars): U+3048 U+3051 U+305B U+3066 U+306D U+3078 U+3081 U+308C U+3052 U+305C U+3067 U+3079 U+307A U+3047 U+30A7 U+30B1 U+30BB U+30C6 U+30CD U+30D8 U+30E1 U+30EC U+30B2 U+30BC U+30C7 U+30D9 U+30DA U+30A7
* `uVowels` → **0.4** — source literal `"うくすつぬふむゆるぐずづぶぷぅゅゥクスツヌフムユルグズヅブプゥュ"` (32 chars): U+3046 U+304F U+3059 U+3064 U+306C U+3075 U+3080 U+3086 U+308B U+3050 U+305A U+3065 U+3076 U+3077 U+3045 U+3085 U+30A5 U+30AF U+30B9 U+30C4 U+30CC U+30D5 U+30E0 U+30E6 U+30EB U+30B0 U+30BA U+30C5 U+30D6 U+30D7 U+30A5 U+30E5
* `iVowels` → **0.2** — source literal `"いきしちにひみりぎじぢびぴぃィキシチニヒミリギジヂビピィ"` (28 chars): U+3044 U+304D U+3057 U+3061 U+306B U+3072 U+307F U+308A U+304E U+3058 U+3062 U+3073 U+3074 U+3043 U+30A3 U+30AD U+30B7 U+30C1 U+30CB U+30D2 U+30DF U+30EA U+30AE U+30B8 U+30C2 U+30D3 U+30D4 U+30A3
* `closedChars` → **0.0** — source literal `"んン。、？！.,?! 　\n\r\t-_"` (17 chars): U+3093 U+30F3 U+3002 U+3001 U+FF1F U+FF01 U+002E U+002C U+003F U+0021 U+0020 U+3000 U+000A U+000D U+0009 U+002D U+005F

Anything else → 0.5. Notable consequences (from the golden run, §12.1): **full-size katakana vowels ア イ ウ エ オ are
missing** (only the small ァ ィ ゥ ェ ォ are listed) → 0.5. Each small kana occurs **twice** in its string
(e.g. U+30A1 ァ at positions 18 and 33 of `aVowels`), so the author almost certainly typed the small form where the
full-size vowel was intended — a V1 typo; **V2: reproduce** (fixing it changes mouth shapes for katakana text → product
decision); `ん/ン`, `。、？！.,?!`, ASCII space, U+3000, `\n \r \t - _`
→ 0.0; `っ/ッ`, `ー`, kanji, `é`, digits, emoji (charAt(0) is a lone surrogate) → 0.5; multi-char strings use only the
first unit (`"おはよう"` → 0.7, `" x"` → 0.0). Rust: `s.chars().next()` gives identical results (no astral char is in any
table).

### 9.11 Timers, random ranges and rates (summary)

| Item | Value |
|---|---|
| Driver Δt | `performance.now()` delta, first call 0, capped at **0.1 s** |
| Emotion smoothing | `min(3·dt, 1)` per frame |
| Idle/burst body-head smoothing | `clamp(6·dt, 0, 1)` |
| Gaze smoothing | `clamp(5·dt, 0, 1)` |
| Mouth | speaking `14·dt`, closing `10·dt`, sneeze `16·dt` (unclamped) |
| Lip-sync threshold | speaking iff `lipSyncValue > 0.01` |
| Blink | first interval `3.0 + r·3.5` ∈ [3.0, 6.5) s; later `2.5 + r·3.5` ∈ [2.5, 6.0) s; close 0.1 s; open 0.1 s |
| Sneeze | needs NORMAL + no burst + no wake-up + not speaking; check every **45.0 s** of continuous eligibility; `r < 0.15` |
| Sleep nod | first after **30.0 s** of SLEEPING (initial value, persists), then `20 + r·20` ∈ [20, 40) s; nod burst 4.5 s; also one nod burst immediately on entering SLEEPING (via `setEmotion`) |
| Wake-up | 2.5 s total; full SURPRISED target for 0.8 s, then 1.7 s linear blend back; look-around for elapsed ∈ [0.4, 1.4) s, ±15° head X, ±0.7 gaze X; SURPRISED spring burst (0.4 s) at start; gaze/eye-tracking disabled during wake-up |
| Burst durations | NORMAL 0.4, SMILE 0.5, ANGRY 0.5, SAD 0.8, SURPRISED 0.4, BLUSH 0.6, WINK 0.4, DISGUST 0.5, SMUG 0.7, THINKING 0.8, PANIC 0.2, SLEEPING 4.5, SNEEZE 2.2 (s) |
| Breath | ω = 1.2 rad/s (0.8 when `emotion === "SLEEPING"`), on `userTime` |
| RNG | `Math.random()`: constructor (blink), each blink completion, each 45-s sneeze check, each periodic nod |

### 9.12 Behaviour timelines (for QA)

* **Emotion change X→Y (neither SLEEPING):** Y burst (spring overshoot, Y's axes, Y's duration) starts at once; idle
  drift restarts from phase 0 with Y's parameters (identical trajectory every time Y is entered); face eases toward Y
  with time constant ≈ 1/3 s; gaze keeps following the mouse.
* **Idle 60 s → SLEEPING** (ChatPanel): burst tag `SLEEPING` → three nods over 4.5 s with eyes fluttering to ≤ 0.18;
  face eases to the SLEEPING target (eyes → 0, head down/tilted); gaze returns to 0; breath slows to 0.8 rad/s; drift
  slows (×0.3 phase) plus a bob; periodic nod after 30 s, then every 20–40 s.
* **Activity while SLEEPING → NORMAL:** 2.5-s startled wake-up: SURPRISED spring burst, wide eyes (target eyeOpen
  1.25 → clamped 1) for 0.8 s then 1.7 s blend to NORMAL; at 0.4–1.4 s the head swings right then left (±15°) with the
  eyeballs (±0.7); afterwards `currentEmotionTag = NORMAL`, gaze tracking resumes. Idle drift during the wake-up uses
  the SURPRISED parameters.
* **Sneeze** (NORMAL idle ≥ 45 s, 15 % per check, or `/dev emotion sneeze`): 1.1 s wind-up (head up +12, eyes closing
  to 0.1), 0.33 s sneeze (head down to −20, mouth to 0.9, eyes shut), 0.77 s recovery. With `/dev emotion sneeze` the
  emotion stays `"SNEEZE"` afterwards: NORMAL face targets, fallback sine idle motion, no further sneezes.
* **Speaking:** `lipSyncValue = 1` from ChatPanel while typing; mouth follows the vowel of each revealed character
  (0.9/0.7/0.6/0.4/0.2/0/0.5) with rate 14/s; `spokenChar = ""` at pauses → procedural babble if `lipSyncValue`
  stays > 0.01, else closes at 10/s.

### 9.13 Edge cases (reproduce)

* `setEmotion` while waking up with any tag ≠ current → wake-up cancelled at once (`wakeUpTimer = 0`), look-around
  stops next frame.
* Re-entering SLEEPING during a wake-up: `currentEmotionTag` is `"SURPRISED"` ≠ `"SLEEPING"` → nod burst restarts.
* `setEmotion("SURPRISED")` during a wake-up: wake-up cancelled, no new burst (`currentEmotionTag` already SURPRISED).
* Unknown tag `"FOO"`: NORMAL face, NORMAL burst (bodyY/headY +1·0.5·spring), fallback sine idle; gaze enabled.
* The driver keeps its own state across model reloads; it is created once per player.
* First driver call: Δ = 0 → `current` stays NORMAL, `userTime = 0` → breath 0.5 (golden frame 0).

---

## 10. Destroy, release, WebGL context loss

### 10.1 `destroy()` as written (B 18167–18187, verbatim)

```js
destroy() {
  this.destroyed = true;
  if (this.initRetryHandle !== null) { cancelAnimationFrame(this.initRetryHandle); this.initRetryHandle = null; }
  if (this.rafHandle !== null) { cancelAnimationFrame(this.rafHandle); this.rafHandle = null; }
  if (this.subdelegate) {
    this.subdelegate.getLive2DManager().setParameterDriver(null);
    this.subdelegate.release();
    this.subdelegate = null;
  }
  this.host.innerHTML = "";
  if (this.frameworkAcquired) { releaseFramework(); this.frameworkAcquired = false; }
}
```

### 10.2 What actually happens in V1 (QUIRK — V2: fix)

`LAppSubdelegate.release()` (B 17493–17505):

```js
release() {
  this._resizeObserver.unobserve(this._canvas);
  this._resizeObserver.disconnect();
  this._resizeObserver = null;
  this._live2dManager.release();      // empty function
  this._live2dManager = null;
  this._view.release();               // ← throws
  this._view = null;
  this._textureManager.release();
  this._textureManager = null;
  this._glManager.release();
  this._glManager = null;
}
```

`LAppView.release()` (B 17321–17331) runs `this._viewMatrix = null; this._touchManager = null; this._deviceToScreen = null;
this._gear.release();` — `_gear` is `null` (no gear image) → **`TypeError: Cannot read properties of null (reading
'release')`** (verified in Node, `viewrelease.cjs`). Therefore, whenever a subdelegate exists:

1. done: `destroyed = true`, both rAFs cancelled, driver detached, ResizeObserver disconnected, `_live2dManager = null`;
2. **not done:** view/texture-manager/GL-manager release, `this.subdelegate = null`, `host.innerHTML = ""`,
   `releaseFramework()`. The `LAppModel` (Core model + moc, renderer, GL textures) is never released in any path
   (`LAppLive2DManager.release()` is empty, `releaseAllModel()` only truncates the array).
3. The exception propagates to Live2DCanvas → `console.warn("Live2D destroy error:", e)` → `loseContext()` frees the GPU
   side → `host.innerHTML = ""` removes the canvas.
4. Still-pending async loads continue on the dead objects (they hit the lost context, whose calls are no-ops) and the
   page keeps the Core heap allocations — a small leak per remount.

The init-failure path (`initSubdelegate` with WebGL unavailable) does release the framework correctly.

### 10.3 WebGL context loss (V1)

* `LAppSubdelegate.update()` returns early while `gl.isContextLost()`; the rAF loop keeps running.
* No `webglcontextlost` / `webglcontextrestored` listeners and no `preventDefault()` → per the WebGL spec the browser
  never restores the context → the model stays invisible until the component remounts. `LAppModel.reloadRenderer()`
  exists but is never called.
* Live2DCanvas deliberately loses the context on unmount (`WEBGL_lose_context`), because browsers cap live contexts
  (~16) and the runtime never frees its own.

### 10.4 Required V2 behaviour (normative)

`PlayerHandle::destroy()` must never panic/throw and must, in order: set `destroyed`; cancel the init-retry and frame
rAF callbacks (drop their `Closure`s); detach the driver; disconnect the `ResizeObserver`; release the model (renderer:
GL programs/buffers/FBOs/mask textures; texture manager: `deleteTexture`; Core: model and moc `_release`); release the
sprite program if created; `host.set_inner_html("")`; `release_framework()` (ref-count, §4.4). Every pending async
continuation (fetch/decode/shader compile) must check a generation token or `Weak` upgrade and become a no-op after
destroy. The UI component still calls `WEBGL_lose_context.loseContext()` afterwards (keep it — it is the only thing
that frees the context immediately). Context loss: reproduce "skip frames, never recover" by default; an opt-in restore
path (preventDefault on `webglcontextlost`, rebuild renderer/textures/shaders on `webglcontextrestored`) is listed under
Open questions.

---

## 11. Console output (verbatim strings; keep them identical in V2 for support/debugging parity)

| Level | Message (template) | When |
|---|---|---|
| error | `[ReamaCubism] Live2DCubismCore is not loaded.` | `create()` without Core namespace |
| log | `` `[ReamaCubism] Initializing. Canvas client size: ${w}x${h}` `` | before subdelegate init |
| error | `Cannot initialize WebGL context.` | both `getContext` calls failed |
| error | `[ReamaCubism] Failed to initialize Cubism subdelegate.` | subdelegate init failed |
| log | `` `[ReamaCubism] WebGL ready. Max texture size: ${max}, Canvas buffer: ${dbw}x${dbh}` `` | after init |
| log | `` `[ReamaCubism] Loading model: ${dir}${file}` `` | `loadAssets` |
| log | `` `[ReamaCubism] model3.json loaded (${bytes} bytes)` `` | model3.json fetched (421) |
| error | `` `[ReamaCubism] Failed to load model: ${dir}${file}` `` + error object | model3.json failed |
| log | `Model data does not exist.` | model3.json without `Moc` (LAppPal.printMessage) |
| log | `` `[ReamaCubism] Loading ${n} texture(s)...` `` | `setupTextures` |
| log | `getTextureFileName null` | empty texture name |
| log | `` `[ReamaCubism] Loading texture ${i}: ${path}` `` | per texture |
| warn | `` `[ReamaCubism] Texture ${file} (${w}x${h}) exceeds MAX_TEXTURE_SIZE (${max}). Downscaling...` `` | downscale path |
| log | `` `[ReamaCubism] Texture ${i} loaded: ${w}x${h}` `` | per texture (original size) |
| log | `[ReamaCubism] All textures loaded. Model setup complete!` | state 23 |
| error | `` `[ReamaCubism] Failed to load texture image: ${file}` `` + error | image load/decode/upload failure |
| error | `` `Error loading ${path} shader:` `` + error / `Failed to load shaders:` + error | shader fetch (framework) |
| log | `Live2D Cubism SDK Core Version 6.0.1` | printed by the Core itself (`console.log`) on the first moc load (not by this code) |
| error (uncaught) | `Uncaught (in promise) TypeError: Logging.logFunction is not a function` | V1 only: any Core error log (e.g. invalid/empty moc), §4.4 |
| error | `Cubism init failed` + error | Live2DCanvas: script load failed |
| warn | `Live2D destroy error:` + error | Live2DCanvas: destroy threw (always, in V1) |
| warn | `Live2D preload warning:` + error | Live2DCanvas: preload failed |

Framework logs (`[CSM][E]…`, `[CSM][I]…`) are all suppressed (log level Off, no log function). Dead-path messages:
`failed to create vertexShader`, `failed to create fragmentShader`, `view notfound`, `Motion Began:`, `Motion Finished:`,
`[APP]…` debug lines.

---

## 12. Golden vectors

### 12.1 Pure helper functions (real bundle code, `funcs.cjs`; JS f64, 17 significant digits)

```text
noise01(x, seed):
  noise01(0, 0) = 0.5
  noise01(0, 5) = 0.2543259229059099
  noise01(1, 0) = 0.9516852880334032
  noise01(1, 5) = 0.32570687637959156
  noise01(2.5, 5) = 0.4855703273270565
  noise01(10, 7) = 0.3703889319567717
  noise01(123.456, 99) = 0.8847108300062086
noiseSigned(x, seed):
  noiseSigned(0, 0) = 0
  noiseSigned(3, 7) = 0.43100986590328305
  noiseSigned(60, 999) = -0.5200446589817017
drift(phase, speed, seed, amp):
  drift(0, 0.6, 0, 1.8) = -0.06726709451437167
  drift(1, 0.6, 0, 1.8) = 0.03175198016194417
  drift(5, 1.2, 5, 3) = 0.37149138218976907
  drift(10, 0.2, 55, 0.5) = 0.2281149899126924
  drift(2, 3, 36, 3) = -0.27675290041749157
getEmotionIdleMotion(tag, phase):
  NORMAL @0: bodyX=-0.06726709451437167 bodyY=-0.14215916325768027 bodyZ=-0.03751219675140892 headX=0.7859467673535369 headY=0.7337209251261717 headZ=0.0011149235125782544
  NORMAL @1.5: bodyX=0.05793305008170467 bodyY=0.2620167478067107 bodyZ=-0.04125808739688172 headX=0.7079168975404062 headY=0.7749784642563047 headZ=0.18252378932914476
  NORMAL @7.25: bodyX=0.8115503375918866 bodyY=0.31322507988182025 bodyZ=0.08175853154868364 headX=-1.524367342710756 headY=-0.7739777598481384 headZ=-0.2542097587379118
  SMILE @0: bodyX=-0.8099485941293798 bodyY=-0.6563814747614982 bodyZ=-0.09003335848089042 headX=0.9984734413599257 headY=0.2558229226801568 headZ=-0.322723026291861
  SMILE @1.5: bodyX=-0.7142229627879784 bodyY=-0.8517052611290614 bodyZ=-0.7916285738486148 headX=-0.20798043700309776 headY=0.9573582661345947 headZ=1.1355544902042232
  SMILE @7.25: bodyX=-0.25378341501102714 bodyY=0.06780148316840841 bodyZ=0.3103867617589693 headX=0.8887202901988618 headY=-1.0675516384536625 headZ=-0.5686093081914065
  ANGRY @0: bodyX=-0.3978478000695263 bodyY=0.17605170945397042 bodyZ=0.25344802517415266 headX=-0.2783002565048631 headY=-0.3025669215203576 headZ=0.0907053500145394
  ANGRY @1.5: bodyX=-1.6823429259946325 bodyY=0.005585788970964279 bodyZ=0.637903096374237 headX=-0.11644157853129966 headY=-0.2320390675928608 headZ=-0.29742565374858815
  ANGRY @7.25: bodyX=-0.7211968743069005 bodyY=0.5023525301754765 bodyZ=0.43767392608399924 headX=0.9462321383900483 headY=-0.38509362347316856 headZ=0.11288985640997061
  SAD @0: bodyX=0.3156682165948016 bodyY=0.7062053788158595 bodyZ=0.36708047890639744 headX=-1.247787775854888 headY=-1.205656142986785 headZ=-0.076106378406259
  SAD @1.5: bodyX=0.33476992339908773 bodyY=0.5418986555864302 bodyZ=0.4532967505283254 headX=-0.5649857237490253 headY=-1.1404684570690156 headZ=-0.43948032679598686
  SAD @7.25: bodyX=0.13740071344732258 bodyY=-0.327377495115599 bodyZ=-0.6797224573592219 headX=-1.0725093696036438 headY=-0.15990761646726176 headZ=0.39229812020823107
  SURPRISED @0: bodyX=0.21443270773914905 bodyY=-0.442323692575608 bodyZ=-0.6043964476078678 headX=-0.2230201409930268 headY=1.792339988998879 headZ=0.5615875585436426
  SURPRISED @1.5: bodyX=-0.8044333982022521 bodyY=0.5752411290033272 bodyZ=-0.28641467696109546 headX=1.8135560432881492 headY=1.4505433256168725 headZ=0.9504039814643551
  SURPRISED @7.25: bodyX=0.013527203097700842 bodyY=0.8045357483973536 bodyZ=0.015408866836965789 headX=1.92899122439607 headY=0.5272813638534124 headZ=-0.2808598582079118
  BLUSH @0: bodyX=0.1837867293235439 bodyY=0.3234973303317039 bodyZ=0.9631698916402144 headX=2.4454043698854404 headY=-0.8911598453367018 headZ=-0.5039369983107402
  BLUSH @1.5: bodyX=0.3428501337853447 bodyY=0.18717036535552395 bodyZ=1.1606234361785648 headX=3.4240690605785344 headY=0.06561584858542434 headZ=-1.1642356301057601
  BLUSH @7.25: bodyX=1.798603282228162 bodyY=-0.18909340377241912 bodyZ=-0.6489556706656359 headX=0.7724002135904049 headY=-1.5006206125879265 headZ=0.3948840197710501
  WINK @0: bodyX=-0.47028179375536305 bodyY=-0.10527529678735047 bodyZ=0.20826637562566275 headX=0.39584527921772095 headY=-0.4743303421210224 headZ=-0.7777930815339467
  WINK @1.5: bodyX=-0.13608725257896243 bodyY=-0.5473674951135944 bodyZ=-1.075715858256133 headX=-0.4188105596410414 headY=-0.13805774714041455 headZ=0.9389305476363968
  WINK @7.25: bodyX=-0.6333208018918618 bodyY=0.3699492388051008 bodyZ=0.26405243858159066 headX=0.39794821337387787 headY=1.2546405100288396 headZ=-0.5502631617316069
  DISGUST @0: bodyX=1.7147576245624054 bodyY=0.6044522326065389 bodyZ=0.0832637641670154 headX=0.39692517324782517 headY=-0.7800030031050229 headZ=0.07699497496568174
  DISGUST @1.5: bodyX=1.8133586133435284 bodyY=0.8033097238034037 bodyZ=0.3828258485096913 headX=0.21596505799063603 headY=-1.4908505980580373 headZ=-0.503076587188339
  DISGUST @7.25: bodyX=-0.18332059268790468 bodyY=-0.40240908873540127 bodyZ=-0.42875585952833317 headX=3.052177595539438 headY=0.4236655155554581 headZ=0.6345972844798702
  SMUG @0: bodyX=-0.26873245357449316 bodyY=-0.08772941398945873 bodyZ=0.15619978171924706 headX=3.263896852811814 headY=-0.3794642736968179 headZ=-0.31111723261357865
  SMUG @1.5: bodyX=0.25554763318056806 bodyY=-0.17419174780669577 bodyZ=-0.1963848481198332 headX=2.5950304599174734 headY=-0.2550613933008704 headZ=-0.18734456603007088
  SMUG @7.25: bodyX=-1.2687815573358305 bodyY=-0.5839836786625833 bodyZ=-0.6942363525216574 headX=4.280944225823598 headY=1.2303660179154061 headZ=0.2920884152182681
  THINKING @0: bodyX=-0.26998286470979327 bodyY=-0.21879382492049942 bodyZ=-0.03001111949363014 headX=0.24961836033998142 headY=0.10232916907206271 headZ=-0.08068075657296525
  THINKING @1.5: bodyX=-0.06575156372834279 bodyY=-0.19759769908182773 bodyZ=-0.11431884345989383 headX=0.08915578906473444 headY=0.13920456986554672 headZ=-0.008075738856790606
  THINKING @7.25: bodyX=-0.14402570515982097 bodyY=-0.29350193057659907 bodyZ=-0.28105874414410226 headX=-0.05682115070094712 headY=0.48555639196503725 headZ=0.3037126931223746
  PANIC @0: bodyX=-1.3777934513192347 bodyY=-0.589764923434144 bodyZ=-0.8058619301438237 headX=-1.6545606744078167 headY=-2.281122884718001 headZ=-0.7603742949060004
  PANIC @1.5: bodyX=-0.9729936814927411 bodyY=0.05330065831221799 bodyZ=0.5359966422347664 headX=-0.24703736716089236 headY=-0.4212811577051715 headZ=-0.1404270525683905
  PANIC @7.25: bodyX=-0.6878616075565628 bodyY=-0.7221375302601136 bodyZ=-0.032858945759644556 headX=0.1421418335771947 headY=0.12020478249520328 headZ=0.040068260831734426
  SLEEPING @0: bodyX=-0.018685304031769906 bodyY=-0.10661937244326021 bodyZ=-0.01875609837570446 headX=0.2619822557845123 headY=0.18343023128154293 headZ=0.0016723852688673817
  SLEEPING @1.5: bodyX=-0.004174520205015915 bodyY=0.46639830928411136 bodyZ=-0.026185756456255857 headX=0.22976142921461448 headY=0.47429103756730995 headZ=0.08888313236720066
  SLEEPING @7.25: bodyX=0.023966273248061658 bodyY=-0.15043755148617963 bodyZ=-0.04312248275447303 headX=0.1667887591051327 headY=0.07187770080231518 headZ=0.27452421606819793
  sneeze @0: bodyX=0 bodyY=0 bodyZ=0 headX=0 headY=0 headZ=0
  sneeze @1.5: bodyX=0.817966512028001 bodyY=0.2609793204667381 bodyZ=0.4517139787160284 headX=2.0504859891278984 headY=1.174990364441225 headZ=0.7806809030346152
  sneeze @7.25: bodyX=-0.5577585456379307 bodyY=0.49377285436591567 bodyZ=0.1913994633711856 headX=-1.0221247947102645 headY=-1.4025788663376737 headZ=-0.8414764556304017
  UNKNOWN @0: bodyX=0 bodyY=0 bodyZ=0 headX=0 headY=0 headZ=0
  UNKNOWN @1.5: bodyX=0.817966512028001 bodyY=0.2609793204667381 bodyZ=0.4517139787160284 headX=2.0504859891278984 headY=1.174990364441225 headZ=0.7806809030346152
  UNKNOWN @7.25: bodyX=-0.5577585456379307 bodyY=0.49377285436591567 bodyZ=0.1913994633711856 headX=-1.0221247947102645 headY=-1.4025788663376737 headZ=-0.8414764556304017
getVowelMouthOpening(chr):
  "a" (U+0061) -> 0.9
  "A" (U+0041) -> 0.9
  "o" (U+006F) -> 0.7
  "O" (U+004F) -> 0.7
  "e" (U+0065) -> 0.6
  "E" (U+0045) -> 0.6
  "u" (U+0075) -> 0.4
  "U" (U+0055) -> 0.4
  "i" (U+0069) -> 0.2
  "I" (U+0049) -> 0.2
  "í" (U+00ED) -> 0.2
  "Í" (U+00CD) -> 0.2
  "é" (U+00E9) -> 0.5
  "x" (U+0078) -> 0.5
  "1" (U+0031) -> 0.5
  "あ" (U+3042) -> 0.9
  "か" (U+304B) -> 0.9
  "ア" (U+30A2) -> 0.5
  "カ" (U+30AB) -> 0.9
  "ぁ" (U+3041) -> 0.9
  "ゃ" (U+3083) -> 0.9
  "ャ" (U+30E3) -> 0.9
  "お" (U+304A) -> 0.7
  "オ" (U+30AA) -> 0.5
  "ょ" (U+3087) -> 0.7
  "ョ" (U+30E7) -> 0.7
  "を" (U+3092) -> 0.7
  "ヲ" (U+30F2) -> 0.7
  "え" (U+3048) -> 0.6
  "エ" (U+30A8) -> 0.5
  "う" (U+3046) -> 0.4
  "ウ" (U+30A6) -> 0.5
  "ゅ" (U+3085) -> 0.4
  "ュ" (U+30E5) -> 0.4
  "い" (U+3044) -> 0.2
  "イ" (U+30A4) -> 0.5
  "ん" (U+3093) -> 0
  "ン" (U+30F3) -> 0
  "っ" (U+3063) -> 0.5
  "ッ" (U+30C3) -> 0.5
  "ー" (U+30FC) -> 0.5
  "ヴ" (U+30F4) -> 0.5
  "ゎ" (U+308E) -> 0.5
  "ゐ" (U+3090) -> 0.5
  "ゑ" (U+3091) -> 0.5
  "。" (U+3002) -> 0
  "、" (U+3001) -> 0
  "？" (U+FF1F) -> 0
  "！" (U+FF01) -> 0
  "." (U+002E) -> 0
  "," (U+002C) -> 0
  "?" (U+003F) -> 0
  "!" (U+0021) -> 0
  " " (U+0020) -> 0
  "　" (U+3000) -> 0
  "\n" (U+000A) -> 0
  "\r" (U+000D) -> 0
  "\t" (U+0009) -> 0
  "-" (U+002D) -> 0
  "_" (U+005F) -> 0
  "紅" (U+7D05) -> 0.5
  "😀" (U+1F600) -> 0.5
  "ab" (U+0061 U+0062) -> 0.9
  "おはよう" (U+304A U+306F U+3088 U+3046) -> 0.7
  " x" (U+0020 U+0078) -> 0
  "" (empty) -> 0
burst spring factor sin(t*PI*2.5)*(1-t)^2:
  t=0: 0
  t=0.1: 0.5727564927611035
  t=0.2: 0.6400000000000001
  t=0.3: 0.3464823227814083
  t=0.4: 4.408728476930471e-17
  t=0.5: -0.17677669529663687
  t=0.6: -0.16000000000000003
  t=0.8: -9.797174393178821e-18
  t=1: 0
```

### 12.2 Full pipeline trace (real bundle + real Core + real model, `pipeline.cjs A 1920 1045`)

Setup: canvas 1920×1045 (Desktop branch), `Math.random` = mulberry32(20261116) (first draw → initial blink interval
3.871642), clock 2 advances exactly 1000/60 ms per frame (first driver call Δ = 0), clock 1 is
`floor(T0 + (f+1)·1000/60)` ms → `palDt` = 0.016, 0.017, 0.017, … Script: f0 `setEmotion("NORMAL")` (no-op),
f60 `"smile"`, f120 `setLipSync(1)` + `setSpokenChar("あ")`, f150 `"ン"`, f180 `setLipSync(0)` + `""`,
f200 `setEyeTracking(0.7, -0.35)`, f240 `"WINK"`, f300 `"SLEEPING"`, f420 `"NORMAL"` (wake-up). Blink intervals drawn:
3.871642, 2.860994, 4.204358. Values are the f32 parameter values after core update #2 (what the renderer sees):

| f | palDt | emotion / currentEmotionTag / activeBurstTag | burstProgress | blinkState | AngleX | AngleY | AngleZ | BodyAngleX | BodyAngleY | BodyAngleZ | EyeLOpen | EyeROpen | EyeLSmile | BrowLY | BrowLForm | MouthForm | Cheek | EyeBallX | EyeBallY | MouthOpenY | Breath | HairFront | HairBack |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 0.016 | NORMAL / NORMAL / "" | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 1 | 0 | 0 | 0.5 | 0 | 0 | 0 | 0 | 0 | 0.5 | 0 | 0 |
| 1 | 0.017 | NORMAL / NORMAL / "" | 1 | 0 | 0.07770973 | 0.07385865 | 0.0004349184 | -0.005909868 | -0.01384519 | -0.003954306 | 1 | 1 | 0 | 0 | 0.5 | 0 | 0 | 0 | 0 | 0 | 0.5099993 | -0.002836195 | -0.004254298 |
| 2 | 0.017 | NORMAL / NORMAL / "" | 1 | 0 | 0.146758 | 0.1408094 | 0.001150433 | -0.01042509 | -0.02592998 | -0.007713359 | 1 | 1 | 0 | 0 | 0.5 | 0 | 0 | 0 | 0 | 0 | 0.5199947 | -0.004389224 | -0.006583858 |
| 60 | 0.016 | SMILE / SMILE / SMILE | 0.033333 | 0 | 0.9037787 | 0.9895496 | -0.1100395 | 0.1990464 | 0.05210602 | -0.2249014 | 0.98 | 0.98 | 0.05 | 0.02 | 0.515 | 0.05 | 0.015 | 0 | 0 | 0 | 0.9660196 | 0.01407608 | 0.002281097 |
| 61 | 0.017 | SMILE / SMILE / SMILE | 0.066667 | 0 | 1.346243 | 1.180309 | -0.3912216 | 0.4022569 | 0.05091958 | -0.3879172 | 0.961 | 0.961 | 0.0975 | 0.039 | 0.52925 | 0.0975 | 0.02925 | 0 | 0 | 0 | 0.9695497 | 0.01313314 | 0.00256783 |
| 120 | 0.016 | SMILE / SMILE / SMILE | 1 | 0 | 4.307473 | 3.182608 | -2.404797 | 2.677882 | -0.6966033 | -2.456149 | 0.6175065 | 0.6175065 | 0.9562337 | 0.3824935 | 0.7868701 | 0.9562337 | 0.2868701 | 0 | 0 | 0.21 | 0.8377316 | 0.01287746 | 0.01588888 |
| 121 | 0.017 | SMILE / SMILE / SMILE | 1 | 0 | 4.320599 | 3.195549 | -2.395696 | 2.670685 | -0.7080429 | -2.467079 | 0.6166312 | 0.6166312 | 0.958422 | 0.3833688 | 0.7875266 | 0.958422 | 0.2875266 | 0 | 0 | 0.371 | 0.8302906 | 0.01268413 | 0.01596875 |
| 150 | 0.016 | SMILE / SMILE / SMILE | 1 | 0 | 4.689624 | 3.759701 | -1.977144 | 2.391295 | -0.8761662 | -2.720556 | 0.6037576 | 0.6037576 | 0.9906061 | 0.3962424 | 0.7971818 | 0.9906061 | 0.2971818 | 0 | 0 | 0.6897618 | 0.57056 | 0.007042993 | 0.01523749 |
| 240 | 0.016 | WINK / WINK / WINK | 0.041667 | 2 | 22.87531 | -4.753987 | -2.57485 | 4.984192 | -2.092447 | -2.46343 | 0 | 0 | 0.9849071 | 0.4049628 | 0.7999721 | 0.9749071 | 0.2849721 | 0.6802409 | -0.3401205 | 4.594e-9 | 0.001917696 | -0.0141224 | -0.006448436 |
| 241 | 0.017 | WINK / WINK / WINK | 0.083333 | 2 | 23.15203 | -4.809438 | -2.987102 | 5.263723 | -1.97713 | -2.646092 | 0.1065059 | 0 | 0.9706618 | 0.4097147 | 0.7999735 | 0.9511617 | 0.2707235 | 0.6818875 | -0.3409438 | 3.829e-9 | 0.00289224 | -0.01432415 | -0.00647101 |
| 300 | 0.016 | SLEEPING / SLEEPING / SLEEPING | 0.003704 | 0 | 19.77144 | -6.85112 | -5.289271 | 5.297485 | -1.921094 | -5.102048 | 0.006345337 | 0.006345337 | 0.6781256 | 0.4556217 | 0.7699987 | 0.4918789 | 0.01312861 | 0.6415599 | -0.3207799 | 8.154e-14 | 0.1215988 | -0.008908814 | -0.008893779 |
| 301 | 0.017 | SLEEPING / SLEEPING / SLEEPING | 0.007407 | 0 | 17.69524 | -6.778768 | -5.413106 | 4.864054 | -1.881475 | -4.929671 | 0.01268279 | 0.01268279 | 0.6442193 | 0.4178406 | 0.7414988 | 0.4622849 | 0.01247218 | 0.5880966 | -0.2940483 | 6.795e-14 | 0.1172749 | -0.00868976 | -0.008882904 |
| 420 | 0.016 | NORMAL / SURPRISED / SURPRISED | 0.041667 | 1 | -11.09818 | -14.32478 | -7.438851 | -1.106078 | -2.496063 | -1.803549 | 0.05373263 | 0.05373263 | 0.001439272 | -0.2433963 | 0.2162098 | -0.1137438 | 0.00002786451 | 0.00001873311 | -0.000009366556 | 2.568e-23 | 0.9272994 | 0.01441745 | 0.00496511 |
| 421 | 0.017 | NORMAL / SURPRISED / SURPRISED | 0.083333 | 1 | -10.49587 | -12.15758 | -7.016262 | -1.268337 | -1.73235 | -1.522254 | 0.08250346 | 0.08250346 | 0.001367308 | -0.1912264 | 0.2303993 | -0.1280566 | 0.00002647128 | 0.00001717202 | -0.000008586009 | 2.140e-23 | 0.9220214 | 0.01435228 | 0.005064335 |
| 599 | 0.017 | NORMAL / NORMAL / SURPRISED | 1 | 0 | 16.29086 | -7.405718 | 0.04960424 | 3.939641 | -0.9823619 | 0.1728895 | 1 | 1 | 1.481e-7 | 0.03170709 | 0.4999708 | -0.01587778 | 2.868e-9 | 0.6484663 | -0.3242717 | 1.723e-37 | 0.2233292 | -0.0101566 | 0.003517418 |

Parameter values **after `model.update()`** (load + Look + Breath + Physics, i.e. what physics saw and wrote, before
the driver):

| f | AngleX | AngleY | AngleZ | BodyAngleX | BodyAngleZ | Breath | HairFront | HairSide | HairBack |
|---|---|---|---|---|---|---|---|---|---|
| 0 | 0.1153803 | 0.1137557 | 0.09081709 | 0.01294284 | 0 | 0.5155379 | 0 | 0 | 0 |
| 1 | 0.2379413 | 0.2345183 | 0.1872767 | 0.026694 | 0 | 0.5320302 | -0.002836195 | -0.002836195 | -0.004254298 |
| 2 | 0.3604387 | 0.3550667 | 0.2836666 | 0.0404439 | 0 | 0.5484875 | -0.004389224 | -0.004389224 | -0.006583858 |
| 3 | 0.475643 | 0.4682297 | 0.3742906 | 0.05338324 | 0 | 0.5639287 | -0.005824573 | -0.005824573 | -0.008736911 |
| 30 | 3.570357 | 3.175802 | 2.764341 | 0.4143858 | 0 | 0.921371 | -0.004982973 | -0.004982973 | -0.0179569 |
| 60 | 6.215855 | 3.88976 | 4.57082 | 0.7989388 | 0 | 0.9599763 | 0.01407608 | 0.01407608 | 0.002281097 |
| 120 | 6.998758 | -1.711702 | 3.765877 | 1.456006 | 0 | 0.1502875 | 0.01287746 | 0.01287746 | 0.01588888 |
| 240 | -4.945122 | 3.020775 | -4.9415 | 1.997134 | 0 | 0.9993061 | -0.0141224 | -0.0141224 | -0.006448436 |
| 300 | -7.454095 | 1.945619 | -2.776155 | 1.79387 | 0 | 0.3431609 | -0.008908814 | -0.008908814 | -0.008893779 |
| 420 | 3.349634 | -0.3763094 | 4.969165 | 0.5984026 | 0 | 0.936806 | 0.01441745 | 0.01441745 | 0.00496511 |
| 599 | -1.421047 | -3.514215 | -4.684422 | -1.570466 | 0 | 0.7723229 | -0.0101566 | -0.0101566 | 0.003517418 |

The full 600-frame trace (all 27 parameters before/after the driver, MVP, player state) is
`/tmp/claude-0/-home-elvelt-admin-Elvelt-dsuv/1f3badd8-3ee9-4a13-9340-35fac0e7abf1/scratchpad/pipeline-probe/trace-A-1920x1045.json`
— copy it into `amadeus-live2d/tests/golden/` (or regenerate with Appendix A). Use it as an integration test: drive
the Rust pipeline with the same Δ sequences and RNG draws, compare all 27 parameters per frame (tolerance §0.5).
An independent, driver-only golden set produced by another task also exists at
`/tmp/claude-0/-home-elvelt-admin-Elvelt-dsuv/1f3badd8-3ee9-4a13-9340-35fac0e7abf1/scratchpad/harness/golden.json`
(`run.js`; noise/drift/idle/vowel tables and three driver scenarios) — consistent with this spec.

### 12.3 How the golden data was produced

`pipeline.cjs` loads the Core and a copy of the deployed bundle whose last line additionally exports internals
(`window.__internals = { LAppModel, LAppLive2DManager, LAppPal, CubismFramework, ReamaCubismPlayer, … }`) into a Node
`vm` context with stubs (`fetch` → files under `public/`, fake `Image` 4096², controllable `performance.now`/`Date.now`,
seeded `Math.random`, no-op rAF). It replaces `LAppModel.prototype.createRenderer` with a fake renderer that records the
MVP, gives the model a fake subdelegate (`getCanvas() → {width, height}`), runs the **real** `LAppLive2DManager.onUpdate()`
each frame and samples the Core parameter array after `CubismModel.update()` call #1 and at the end of the frame.
Source in Appendix A (§13).

---

## 13. Appendix A — golden harness source (re-create in any scratch directory; Node ≥ 18)

Step 1 — make a copy of the deployed bundle that exports internals (the project file is not modified):

```python
s = open('reama-cubism-runtime.js', encoding='utf-8').read()
old = "  void preloadDefaultAssets();\n})();"
assert s.count(old) == 1
s = s.replace(old, "  void preloadDefaultAssets();\n  window.__internals = { LAppModel, LAppLive2DManager, LAppPal, CubismFramework, ReamaCubismPlayer, CubismMatrix44, CubismWebGLOffscreenManager, acquireFramework, releaseFramework, CubismUserModel, LAppView, CubismViewMatrix, EMOTION_TARGETS, MOTION_BURSTS, noise01, drift, getEmotionIdleMotion, s_arrayBufferCache, s_textCache, s_imageCache };\n})();")
open('bundle-exposed.js', 'w', encoding='utf-8').write(s)
```

Step 2 — `pipeline.cjs` (usage: `node pipeline.cjs A 1920 1045` → `trace-A-1920x1045.json`):

```js
// Golden trace of the V1 per-frame pipeline (LAppLive2DManager.onUpdate) using the REAL deployed bundle
// (copy with internals exposed) + REAL Live2DCubismCore + REAL model files. No WebGL: renderer is faked.
"use strict";
const fs = require("fs"); const vm = require("vm"); const path = require("path");
const PUB = "/home/elvelt_admin/Elvelt/dsuv/public";
const coreCode = fs.readFileSync(PUB + "/resources/live2d/live2dcubismcore.min.js", "utf8");
const bundleCode = fs.readFileSync(path.join(__dirname, "bundle-exposed.js"), "utf8");
const scenario = process.argv[2] || "A";
const W = +(process.argv[3] || 1920), H = +(process.argv[4] || 1045);

function mulberry32(a) { return function () { a |= 0; a = (a + 0x6D2B79F5) | 0; let t = Math.imul(a ^ (a >>> 15), 1 | a); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; }
const SMath = Object.create(Math); SMath.random = mulberry32(20261116);
let simMs = 1000000;               // performance.now() value
let dateMs = 1763000000000;        // Date.now() value (integer)
class FakeDate extends Date { static now() { return dateMs; } }
const logs = [];
const fakeConsole = { log: (...a) => logs.push(["log", a.map(String).join(" ")]), warn: (...a) => logs.push(["warn", a.map(String).join(" ")]), error: (...a) => logs.push(["error", a.map(String).join(" ")]), assert: (c) => { if (!c) logs.push(["assert", "Assertion failed"]); } };
function fakeFetch(url) {
  const p = PUB + url.split("?")[0];
  return new Promise((res) => setTimeout(() => {
    if (!fs.existsSync(p)) { res({ ok: false, status: 404, arrayBuffer: async () => new ArrayBuffer(0), text: async () => "" }); return; }
    const b = fs.readFileSync(p);
    res({ ok: true, status: 200, arrayBuffer: async () => b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength), text: async () => b.toString("utf8") });
  }, 0));
}
class FakeImage { constructor() { this.width = 0; this.height = 0; this._l = {}; } addEventListener(t, f) { this._l[t] = f; } decode() { return Promise.resolve(); } set src(v) { this._src = v; setTimeout(() => { this.width = 4096; this.height = 4096; this._l.load && this._l.load(); }, 0); } get src() { return this._src; } }
const sandbox = { console: fakeConsole, setTimeout, clearTimeout, Uint8Array, Int32Array, Float32Array, Uint16Array, Float64Array, ArrayBuffer, DataView, WebAssembly, Math: SMath, Date: FakeDate, Promise, TextDecoder, TextEncoder, Map, Set, JSON, Number, Object, Array, String, Error, TypeError, Symbol };
sandbox.document = { currentScript: { src: "http://localhost/resources/live2d/live2dcubismcore.min.js" }, createElement: (t) => ({ tagName: t, style: {}, clientWidth: 0, clientHeight: 0, getContext: () => null }) };
sandbox.location = { href: "http://localhost/" }; sandbox.navigator = { userAgent: "node" };
sandbox.atob = (b) => Buffer.from(b, "base64").toString("binary"); sandbox.btoa = (s) => Buffer.from(s, "binary").toString("base64");
sandbox.fetch = fakeFetch; sandbox.Image = FakeImage;
sandbox.performance = { now: () => simMs };
sandbox.requestAnimationFrame = () => 1; sandbox.cancelAnimationFrame = () => {};
sandbox.ResizeObserver = class { observe() {} unobserve() {} disconnect() {} };
sandbox.window = sandbox; sandbox.self = sandbox; sandbox.globalThis = sandbox;
vm.createContext(sandbox);
vm.runInContext(coreCode, sandbox);
vm.runInContext(bundleCode, sandbox);
const I = sandbox.__internals;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

(async () => {
  for (let i = 0; i < 100; i++) { try { sandbox.Live2DCubismCore.Version.csmGetVersion(); break; } catch (e) { await sleep(10); } }
  I.acquireFramework();
  // fake renderer + subdelegate
  let lastMvp = null; let drawCalls = 0;
  const fakeRenderer = { startUp() {}, loadShaders() {}, setIsPremultipliedAlpha() {}, bindTexture() {}, setRenderTargetSize() {}, release() {},
    setMvpMatrix(m) { lastMvp = Array.from(m.getArray()); }, setRenderState() {}, drawModel() { drawCalls++; } };
  I.LAppModel.prototype.createRenderer = function () { this._renderer = fakeRenderer; };
  const canvas = { width: W, height: H };
  const gl = { isContextLost: () => false };
  const fakeSub = { getCanvas: () => canvas, getGl: () => gl, getGlManager: () => ({ getGl: () => gl }), getFrameBuffer: () => null,
    getTextureManager: () => ({ createTextureFromPngFile(p, pm, cb) { setTimeout(() => cb({ id: {}, width: 4096, height: 4096 }), 0); } }) };
  const mgr = new I.LAppLive2DManager();
  mgr.initialize(fakeSub);
  const host = { innerHTML: "", clientWidth: 0, clientHeight: 0, appendChild() {} };
  const player = new I.ReamaCubismPlayer(host);
  mgr.setParameterDriver((m) => player.applyParameters(m));
  const model = mgr.getMainModel();
  for (let i = 0; i < 200 && model._state !== 23; i++) await sleep(5);
  if (model._state !== 23) { console.error("model not ready, state", model._state); process.exit(1); }
  const cm = model.getModel();
  const n = cm.getParameterCount();
  const ids = []; for (let i = 0; i < n; i++) ids.push(cm._model.parameters.ids[i]);
  const info = ids.map((id, i) => ({ id, min: cm._model.parameters.minimumValues[i], max: cm._model.parameters.maximumValues[i], def: cm._model.parameters.defaultValues[i], repeat: cm._model.parameters.repeats ? cm._model.parameters.repeats[i] : null }));
  const updaters = model._updateScheduler._cubismUpdatableList.map((u) => u.constructor.name + ":" + u.getExecutionOrder());
  const snap = () => Array.from(cm._model.parameters.values).map((v) => +v.toPrecision(9));
  // prime timing: LAppPal lastFrame
  I.LAppPal.updateTime();
  const script = {
    A: { 0: (p) => { p.setEmotion("NORMAL"); }, 60: (p) => p.setEmotion("smile"), 120: (p) => { p.setLipSync(1); p.setSpokenChar("あ"); }, 150: (p) => p.setSpokenChar("ン"), 180: (p) => { p.setLipSync(0); p.setSpokenChar(""); }, 200: (p) => p.setEyeTracking(0.7, -0.35), 240: (p) => p.setEmotion("WINK"), 300: (p) => p.setEmotion("SLEEPING"), 420: (p) => p.setEmotion("NORMAL") },
  }[scenario];
  const frames = [];
  const N = 600;
  for (let f = 0; f < N; f++) {
    simMs += 1000 / 60; dateMs = Math.floor(1763000000000 + (f + 1) * 1000 / 60);
    if (script[f]) script[f](player);
    I.LAppPal.updateTime();
    const palDt = I.LAppPal.getDeltaTime();
    // replicate LAppLive2DManager.onUpdate but intercept stages
    let afterModelUpdate = null;
    const origUpdate = cm.update.bind(cm);
    let callNo = 0;
    cm.update = function () { callNo++; if (callNo === 1) afterModelUpdate = snap(); return origUpdate(); };
    mgr.onUpdate();
    cm.update = origUpdate;
    frames.push({ f, palDt, playerT: +player.userTime.toFixed(9), afterModelUpdate, final: snap(), mvp: lastMvp.map((v) => +v.toPrecision(9)), st: { emotion: player.emotion, cur: player.currentEmotionTag, burst: player.activeBurstTag, prog: +player.burstProgress.toFixed(6), blinkState: player.blinkState, blinkInterval: +player.blinkInterval.toFixed(6), wake: +player.wakeUpTimer.toFixed(6) } });
  }
  const out = { W, H, coreVersion: sandbox.Live2DCubismCore.Version.csmGetVersion(), params: info, updaters, frameworkLogs: logs, drawCalls, frames };
  fs.writeFileSync(path.join(__dirname, `trace-${scenario}-${W}x${H}.json`), JSON.stringify(out));
  console.log("ok", "updaters=", updaters.join(","), "drawCalls=", drawCalls);
  console.log(JSON.stringify(info));
  console.log("logs:", JSON.stringify(logs.slice(0, 20)));
})().catch((e) => { console.error("ERR", e && e.stack || e); process.exit(1); });
```

Step 3 — `funcs.cjs` (helper-function vectors of §12.1; reads the deployed bundle directly by line range):

```js
"use strict";
const fs = require("fs");
const src = fs.readFileSync("/home/elvelt_admin/Elvelt/dsuv/public/resources/live2d/reama-cubism-runtime.js", "utf8").split("\n");
const seg = src.slice(17873, 18010).join("\n"); // frameworkRefCount..getEmotionIdleMotion (0-based slice of L17874..L18010)
const f = new Function(seg + "\nreturn { lerp, qBound, noise01, noiseSigned, drift, MOTION_BURSTS, getEmotionIdleMotion };")();
const fmt = (x) => Number(x.toPrecision(17));
const out = [];
out.push("noise01(x, seed):");
for (const [x, s] of [[0,0],[0,5],[1,0],[1,5],[2.5,5],[10,7],[123.456,99]]) out.push(`  noise01(${x}, ${s}) = ${fmt(f.noise01(x, s))}`);
out.push("noiseSigned(x, seed):");
for (const [x, s] of [[0,0],[3,7],[60,999]]) out.push(`  noiseSigned(${x}, ${s}) = ${fmt(f.noiseSigned(x, s))}`);
out.push("drift(phase, speed, seed, amp):");
for (const a of [[0,0.6,0,1.8],[1,0.6,0,1.8],[5,1.2,5,3],[10,0.2,55,0.5],[2,3,36,3]]) out.push(`  drift(${a.join(", ")}) = ${fmt(f.drift(...a))}`);
out.push("getEmotionIdleMotion(tag, phase):");
for (const tag of ["NORMAL","SMILE","ANGRY","SAD","SURPRISED","BLUSH","WINK","DISGUST","SMUG","THINKING","PANIC","SLEEPING","sneeze","UNKNOWN"]) {
  for (const ph of [0, 1.5, 7.25]) { const m = f.getEmotionIdleMotion(tag, ph); out.push(`  ${tag} @${ph}: ` + ["bodyX","bodyY","bodyZ","headX","headY","headZ"].map(k => k + "=" + fmt(m[k])).join(" ")); }
}
const all = src.join("\n");
const m = all.match(/function getVowelMouthOpening\(chr\) \{[\s\S]*?\n        return 0\.5;\n      \}/);
const gv = new Function(m[0] + "\nreturn getVowelMouthOpening;")();
out.push("getVowelMouthOpening(chr):");
const chars = ["a","A","o","O","e","E","u","U","i","I","í","Í","é","x","1","あ","か","ア","カ","ぁ","ゃ","ャ","お","オ","ょ","ョ","を","ヲ","え","エ","う","ウ","ゅ","ュ","い","イ","ん","ン","っ","ッ","ー","ヴ","ゎ","ゐ","ゑ","。","、","？","！",".",",","?","!"," ","　","\n","\r","\t","-","_","紅","😀","ab","おはよう"," x",""];
for (const c of chars) out.push(`  ${JSON.stringify(c)} (${Array.from(c).map(ch => "U+" + ch.codePointAt(0).toString(16).toUpperCase().padStart(4, "0")).join(" ") || "empty"}) -> ${gv(c)}`);
// spring curve samples
out.push("burst spring factor sin(t*PI*2.5)*(1-t)^2:");
for (const t of [0,0.1,0.2,0.3,0.4,0.5,0.6,0.8,1]) out.push(`  t=${t}: ${fmt(Math.sin(t*Math.PI*2.5)*(1-t)*(1-t))}`);
console.log(out.join("\n"));
```

Step 4 — `coreready.cjs` (demonstrates §3.2-2: Core not callable synchronously):

```js
const fs = require('fs'); const vm = require('vm');
const code = fs.readFileSync(process.argv[2], 'utf8');
const sandbox = { console, setTimeout, clearTimeout, Uint8Array, Int32Array, Float32Array, Uint16Array, ArrayBuffer, WebAssembly, Math, Date, Promise, TextDecoder };
sandbox.document = { currentScript: { src: 'http://localhost/live2dcubismcore.min.js' }, createElement: () => ({}) };
sandbox.location = { href: 'http://localhost/' }; sandbox.navigator = { userAgent: 'node' };
sandbox.atob = (b) => Buffer.from(b, 'base64').toString('binary'); sandbox.btoa = (s) => Buffer.from(s, 'binary').toString('base64');
sandbox.window = sandbox; sandbox.self = sandbox; sandbox.globalThis = sandbox;
vm.createContext(sandbox);
vm.runInContext(code + '\n;globalThis.__core = Live2DCubismCore;', sandbox);
const C = sandbox.__core;
console.log('namespace keys:', Object.keys(C).join(','));
try { console.log('sync csmGetVersion =', C.Version.csmGetVersion()); } catch (e) { console.log('sync call threw:', String(e).slice(0, 200)); }
Promise.resolve().then(() => { try { console.log('microtask csmGetVersion =', C.Version.csmGetVersion()); } catch (e) { console.log('microtask threw:', String(e).slice(0,200)); } });
setTimeout(() => { try { console.log('after setTimeout0 csmGetVersion =', C.Version.csmGetVersion()); } catch (e) { console.log('timeout0 threw:', String(e).slice(0,200)); } }, 0);
setTimeout(() => { try { console.log('after 200ms csmGetVersion =', C.Version.csmGetVersion()); } catch (e) { console.log('200ms threw:', String(e).slice(0,200)); } }, 200);
```

Expected output: `sync call threw: TypeError: Cannot read properties of undefined (reading 'apply')`, the same for the microtask, then `after setTimeout0 csmGetVersion = 100663297`.

Step 5 — `corelog.cjs` (demonstrates the §4.4 log-function quirk; `node corelog.cjs undef` → `THREW: TypeError: Logging.logFunction is not a function` when an empty moc is loaded; `fn` → the handler receives `[CSM] [E]csmReviveMocInPlace: "size" is invalid.`):

```js
const fs = require('fs'); const vm = require('vm');
const code = fs.readFileSync('/home/elvelt_admin/Elvelt/dsuv/public/resources/live2d/live2dcubismcore.min.js', 'utf8');
const logs = [];
const sandbox = { console: { log: (...a) => logs.push('console.log: ' + a.join(' ')), warn: (...a) => logs.push('console.warn: ' + a.join(' ')), error: (...a) => logs.push('console.error: ' + a.join(' ')) }, setTimeout, clearTimeout, Uint8Array, Int32Array, Float32Array, Uint16Array, ArrayBuffer, WebAssembly, Math, Date, Promise, TextDecoder };
sandbox.document = { currentScript: { src: 'http://localhost/x.js' }, createElement: () => ({}) };
sandbox.location = { href: 'http://localhost/' }; sandbox.navigator = { userAgent: 'node' };
sandbox.atob = (b) => Buffer.from(b, 'base64').toString('binary'); sandbox.btoa = (s) => Buffer.from(s, 'binary').toString('base64');
sandbox.window = sandbox; sandbox.self = sandbox; sandbox.globalThis = sandbox;
vm.createContext(sandbox);
vm.runInContext(code + '\n;globalThis.__core = Live2DCubismCore;', sandbox);
const C = sandbox.__core;
setTimeout(() => {
  const variant = process.argv[2];
  try {
    if (variant === 'undef') C.Logging.csmSetLogFunction(undefined);
    if (variant === 'fn') C.Logging.csmSetLogFunction((m) => logs.push('handler: ' + m));
    logs.push('after setLogFunction');
    const b = fs.readFileSync('/home/elvelt_admin/Elvelt/dsuv/public/resources/models/AmadeusKurisu5.0/reama5.0/Live2D紅莉栖forSDK5.0.moc3');
    const moc = C.Moc.fromArrayBuffer(b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength));
    logs.push('moc ok ' + !!moc);
    const bad = C.Moc.fromArrayBuffer(new ArrayBuffer(0));
    logs.push('empty moc -> ' + bad);
  } catch (e) { logs.push('THREW: ' + e); }
  console.log(variant, JSON.stringify(logs, null, 1));
}, 50);
```

Step 6 — `viewrelease.cjs` (demonstrates §10.2 and §3.2-1):

```js
const fs = require("fs"); const vm = require("vm"); const path = require("path");
const PUB = "/home/elvelt_admin/Elvelt/dsuv/public";
const sandbox = { console: { log(){}, warn(){}, error(){}, assert(){} }, setTimeout, clearTimeout, Uint8Array, Int32Array, Float32Array, Uint16Array, ArrayBuffer, DataView, WebAssembly, Math, Date, Promise, TextDecoder, TextEncoder };
sandbox.document = { currentScript: { src: "http://localhost/x.js" }, createElement: () => ({ style: {} }) };
sandbox.location = { href: "http://localhost/" }; sandbox.navigator = { userAgent: "node" };
sandbox.atob = (b) => Buffer.from(b, "base64").toString("binary"); sandbox.btoa = (s) => Buffer.from(s, "binary").toString("base64");
sandbox.fetch = () => new Promise(() => {}); sandbox.Image = class { addEventListener() {} set src(v) {} };
sandbox.performance = { now: () => 0 }; sandbox.requestAnimationFrame = () => 1; sandbox.cancelAnimationFrame = () => {};
sandbox.window = sandbox; sandbox.self = sandbox; sandbox.globalThis = sandbox;
vm.createContext(sandbox);
vm.runInContext(fs.readFileSync(PUB + "/resources/live2d/live2dcubismcore.min.js", "utf8"), sandbox);
vm.runInContext(fs.readFileSync(path.join(__dirname, "bundle-exposed.js"), "utf8"), sandbox);
const I = sandbox.__internals;
const v = new I.LAppView();
try { v.release(); console.log("release OK"); } catch (e) { console.log("LAppView.release() threw:", String(e)); }
// also: does the runtime script throw if Core namespace is missing?
const sb2 = { console: { log(){}, warn(){}, error(){} }, Map, Promise, Math, Date, Float32Array, Uint16Array, Uint8Array, Int32Array, ArrayBuffer };
sb2.window = sb2; sb2.fetch = () => new Promise(() => {}); sb2.Image = class { addEventListener() {} set src(v) {} }; sb2.document = { createElement: () => ({ style: {} }) };
vm.createContext(sb2);
try { vm.runInContext(fs.readFileSync(PUB + "/resources/live2d/reama-cubism-runtime.js", "utf8"), sb2); console.log("runtime loaded without Core; ReamaCubismRuntime =", typeof sb2.ReamaCubismRuntime); } catch (e) { console.log("runtime without Core threw:", String(e).slice(0, 120), "| ReamaCubismRuntime =", typeof sb2.ReamaCubismRuntime); }
```

Expected output: `LAppView.release() threw: TypeError: Cannot read properties of null (reading 'release')` and `runtime without Core threw: ReferenceError: Live2DCubismCore is not defined | ReamaCubismRuntime = undefined`.

---

## 14. Appendix B — dead / latent sample-layer functions (reference for an optional generic port)

None of these run in V1 with this model/host (§0.4). They are summarized so a generic port does not need the JS.

**B.1 `LAppModel.startMotion(group, no, priority, onFinished?, onBegan?)` (B 16057–16119).** If `priority ==
PriorityForce (3)` → `motionManager.setReservePriority(3)`; else if `!motionManager.reserveMotion(priority)` → return
`-1` (`InvalidMotionQueueEntryHandleValue`; debug message `"[APP]can't start motion."`). `name = \`${group}_${no}\``;
`motion = _motions.get(name)`; if missing: starts an async `fetch` + `loadMotion(...)` but checks `if (motion)`
**synchronously** (still undefined) → suppressed error `"Can't start motion {0} ."`, `setReservePriority(PriorityNone)`,
return −1 (stock SDK quirk: a non-preloaded motion never starts). If present: set began/finished handlers. If the motion
has a `Sound`, `_wavFileHandler.start(homeDir + sound)`. Return `motionManager.startMotionPriority(motion, autoDelete,
priority)`.

**B.2 `startRandomMotion(group, priority, …)` (B 16127–16141).** `count = getMotionCount(group)`; `0` → return −1
(this is what runs every frame with `"Idle"`). Else `no = floor(Math.random() * count)` → `startMotion`.

**B.3 `preLoadMotionGroup(group)` (B 16210–16267).** For each motion `i`: plain `fetch(homeDir + file)` (`ok` →
arrayBuffer; `status >= 400` → suppressed error + empty buffer) → `loadMotion(buf, len, name, null, null, setting,
group, i, _motionConsistency)` (applies `FadeInTime`/`FadeOutTime` when `>= 0`) → on success
`setEffectIds(_eyeBlinkIds, _lipSyncIds)`, replace any existing entry, `_motionCount++`; on failure `_allMotionCount--`;
when `_motionCount >= _allMotionCount` → same tail as §6.3 step 16 (state 21, stop motions, renderer, textures,
startUp, shaders).

**B.4 `setExpression(id)` / `setRandomExpression()` (B 16147–16176).** `motion = _expressions.get(id)`; non-null →
`_expressionManager.startMotion(motion, false)`; else debug message `"[APP]expression[${id}] is null"`.
Random: `no = floor(Math.random() * size)`, pick the `no`-th entry in insertion order.

**B.5 `hitTest(name, x, y)` / `CubismUserModel.isHit(drawableId, x, y)` (B 16191–16203, 14800–14830).** Returns false if
`_opacity < 1`; finds the hit area by name and tests the point (converted with `modelMatrix.invertTransformX/Y`)
against the axis-aligned bounds of the drawable's vertex positions.

**B.6 `motionEventFired(value)`** → suppressed info log `"{0} is fired on LAppModel!!"`.

**B.7 `LAppLive2DManager`**: `onDrag(x, y)` → `models[0].setDragging(x, y)` (sets the `CubismTargetPoint` target;
would make the Look updater add `30x` to AngleX, `30y` to AngleY, `−30xy` to AngleZ, `10x` to BodyAngleX, `x`/`y` to
EyeBallX/Y; all six are overwritten later by the driver, so a wired drag would only be visible through the hair
physics, which reads AngleX/AngleZ/BodyAngleX before the driver);
`onTap(x, y)` → head hit → `setRandomExpression()`, body hit → `startRandomMotion("TapBody", 2, finishedMotion,
beganMotion)`; `nextScene()` → `changeScene((index + 1) % ModelDirSize)`; `addModel(i = 0)`; `getMainModel()` →
`models[0] ?? null`.

**B.8 `LAppView` touch path.** `onTouchesBegan(px, py)` → `touchManager.touchesBegan(px·dpr, py·dpr)`;
`onTouchesMoved(px, py)` → computes `viewX/Y = transformView(touchManager.getX/Y())` from the **previous** touch point,
then `touchesMoved(px·dpr, py·dpr)`, then `manager.onDrag(viewX, viewY)` (stock one-event lag);
`onTouchesEnded(px, py)` → `onDrag(0, 0)`, `onTap(transformViewX(px·dpr), transformViewY(py·dpr))`, gear hit →
`nextScene()`. `transformViewX(d) = viewMatrix.invertTransformX(deviceToScreen.transformX(d))` (Y analogous);
`transformScreenX/Y` = `deviceToScreen.transformX/Y`.

**B.9 `TouchManager`.** Fields `_startX/Y, _lastX/Y, _lastX1/Y1, _lastX2/Y2, _lastTouchDistance, _deltaX/Y, _scale = 1,
_touchSingle, _flipAvailable`. `touchesBegan(x, y)`: last = start = (x, y), distance −1, flick available, single.
`touchesMoved(x, y)`: last = (x, y), distance −1, single. `getFlickDistance()` = distance(start, last).
`calculateMovingAmount(v1, v2)`: opposite signs → 0, else `sign(v1)·min(|v1|, |v2|)`.

**B.10 `LAppSubdelegate` pointer handlers.** `onPointBegan(pageX, pageY)`: no view → `"view notfound"`;
`_captured = true`; local = `page − canvas.offsetLeft/Top`; `view.onTouchesBegan(local)`. `onPointMoved`: only when
captured. `onPointEnded` / `onTouchCancel`: `_captured = false`, `view.onTouchesEnded(local)`. `isContextLost()` →
`gl.isContextLost()`.

**B.11 `LAppSprite`.** Rect from centre/size (`left = x − w/2, right = x + w/2, up = y + h/2, down = y − h/2`). First
`render(program)`: attribute `position`/`uv`, uniform `texture = 0`, UVs `[1,0, 0,0, 0,1, 1,1]`, positions
`((rect − max/2) / (max/2))` for (right,up), (left,up), (left,down), (right,down), indices `[0,1,2, 3,2,0]`; every
render re-uploads the buffers and draws 6 indices. `isHit(px, py)`: `y = canvas.height − py`, inside the rect.

**B.12 `LAppWavFileHandler` (extends `IParameterProvider`).** `start(path)`: reset offset/time/rms, `loadWavFile`.
`loadWavFile`: plain `fetch` → parse `RIFF`, skip size, `WAVE`, `fmt `, fmt size, format must be 1 (linear PCM, else
error `"File is not linear PCM."`), channels (u16), sampling rate (u32), skip byte rate (u32) and block align (u16),
bits per sample (u16), skip `fmtSize − 16` extra bytes, then scan chunks `while (!sig("data") && off < size)
off += u32 + 4` (sic), `"Cannot find \"data\" Chunk."` if not found, `samplesPerChannel = dataSize·8/(bits·channels)`,
read interleaved samples (`8`: `(u8 − 128) << 24`; `16`: `u16 << 16`; `24`: `u24 << 8`; other: 0; all `/ 2147483647`).
Errors → `console.log(e)`, resolves false. `update(dt)`: no data or finished → `lastRms = 0`, false; `t += dt ?? 1/60`;
`goal = min(floor(t·rate), samplesPerChannel)`; `rms = sqrt(Σ pcm² / (channels·(goal − offset)))` (0/0 → NaN when no
new samples, stock quirk); `offset = goal`; true. `getParameter()` = `getRms()` = `lastRms`.

**B.13 Other.** `LAppModel.reloadRenderer()` = `deleteRenderer(); createRenderer(canvas.width, canvas.height);
setupTextures()` (note: `setupTextures` only acts in state 21, so it would not reload textures in state 23).
`hasMocConsistencyFromFile()` = plain fetch + `CubismMoc.hasMocConsistency`. `LAppModel.release()` = `CubismLook.delete`,
`_updateScheduler.release()` (removes listeners, clears the list), `CubismUserModel.release()` (motion/expression
managers, `moc.deleteModel(model)` + `moc.release()`, pose/eyeblink/breath/physics/userdata deletes, `deleteRenderer()`).
`LAppTextureManager.release()` deletes all GL textures and sets `_textures = null`; `releaseTextures()`,
`releaseTextureByTexture(tex)`, `releaseTextureByFilePath(name)` delete matching entries.

---

## Rust mapping

### R.1 Crate layout (`RealAmadeusRust/crates/amadeus-live2d`)

```
amadeus-live2d/
  src/
    lib.rs                    pub use player::{Live2dRuntime, PlayerHandle}; pub use app::assets::preload;
                              pub use core::ensure_core_ready;
    core/                     wasm-bindgen bindings to Live2DCubismCore (owned with the framework spec)
    framework/                CubismModel, physics, breath, look, eye_blink, update_scheduler, model_setting_json,
                              id_manager, math (matrix44, model/view matrix, target_point), renderer (render spec)
    app/
      define.rs               §2.7 constants
      clock.rs                FrameClock (Clock 1)          ← LAppPal
      assets.rs               AssetCache, preload()          ← cachedFetch*/cachedLoadImage/preloadDefaultAssets
      gl_context.rs           GlContext::create(canvas)      ← LAppGlManager
      texture_manager.rs      TextureManager                 ← LAppTextureManager
      app_model.rs            AppModel, LoadStep             ← LAppModel
      manager.rs              Live2dManager                  ← LAppLive2DManager (projection + driver hook)
      view.rs                 AppView                        ← LAppView (view/device matrices; sprite program optional)
      subdelegate.rs          Subdelegate                    ← LAppSubdelegate (canvas, resize, frame GL state)
    player/
      mod.rs                  ReamaPlayer + PlayerHandle     ← ReamaCubismPlayer (DOM, rAF, lifecycle)
      driver.rs               EmotionDriver (pure, no web-sys) ← applyParameters logic
      tables.rs               EMOTION_TARGETS, MOTION_BURSTS, vowel strings
      noise.rs                lerp, q_bound, noise01, noise_signed, drift, idle_motion
      params.rs               PlayerParam enum (22 ids) + ParamFrame
  tests/                      native unit tests for player::{driver,noise,tables}; golden JSON
```

Make `player::{driver,noise,tables,params}` and `framework::math` compile on the host target (no `web-sys` imports;
gate DOM/GL modules with `#[cfg(target_arch = "wasm32")]`) so `cargo test -p amadeus-live2d` runs natively; use
`wasm-bindgen-test` only for GL/DOM code.

### R.2 Types and signatures (recommended)

```rust
// app/define.rs
pub const RESOURCES_PATH: &str = "/resources/models/AmadeusKurisu5.0/";
pub const SHADER_PATH: &str = "/resources/live2d/shaders/";
pub const MODEL_DIR: &str = "reama5.0";
pub const MODEL_JSON_FILE_NAME: &str = "Live2D紅莉栖forSDK5.0.model3.json";
pub const PRELOAD_BINARIES: [&str; 3] = [MODEL_JSON_FILE_NAME, "Live2D紅莉栖forSDK5.0.moc3", "Live2D紅莉栖forSDK5.0.physics3.json"]; // fetch_bytes(RESOURCES_PATH + MODEL_DIR + "/" + f)
pub const PRELOAD_IMAGE: &str = "Live2D紅莉栖forSDK5.0.8192/texture_00.png";                                                         // load_image(...)
pub const SHADER_FILES: [&str; 13] = ["vertshadersrc.vert", "vertshadersrcmasked.vert", "vertshadersrcsetupmask.vert",
  "fragshadersrcsetupmask.frag", "fragshadersrcpremultipliedalpha.frag", "fragshadersrcmaskpremultipliedalpha.frag",
  "fragshadersrcmaskinvertedpremultipliedalpha.frag", "vertshadersrccopy.vert", "fragshadersrccopy.frag",
  "fragshadersrccolorblend.frag", "fragshadersrcalphablend.frag", "vertshadersrcblend.vert",
  "fragshadersrcpremultipliedalphablend.frag"];                                                                  // fetch_text(SHADER_PATH + f)
pub const VIEW_SCALE: f64 = 1.0; pub const VIEW_MAX_SCALE: f64 = 2.0; pub const VIEW_MIN_SCALE: f64 = 0.8;
pub const VIEW_LOGICAL_LEFT: f64 = -1.0; pub const VIEW_LOGICAL_RIGHT: f64 = 1.0;
pub const VIEW_LOGICAL_MAX: (f64, f64, f64, f64) = (-2.0, 2.0, -2.0, 2.0); // left, right, bottom, top
pub const PROJ_DESKTOP: (f32, f32) = (4.1, -2.75);  pub const PROJ_MOBILE: (f32, f32) = (3.3, -2.1);
pub const CONTEXT_ATTRS: (bool, bool, bool, bool) = (true, true, true, false); // alpha, antialias, premultipliedAlpha, preserveDrawingBuffer

// app/clock.rs
pub struct FrameClock { last_frame_ms: f64, delta_s: f64 }
impl FrameClock { pub fn new(now_ms: f64) -> Self; pub fn update(&mut self, now_ms: f64); pub fn delta(&self) -> f64; }

// app/assets.rs
pub async fn fetch_bytes(url: &str) -> Result<Rc<[u8]>, AssetError>;      // cached, shared, evict on Err
pub async fn fetch_text(url: &str) -> Result<Rc<str>, AssetError>;
pub async fn load_image(url: &str) -> Result<HtmlImageElement, AssetError>; // decoding="async", decode() ignored errors
pub async fn preload();                                                     // never fails; 17 tasks joined
// AssetError::Http { url, status } => Display "Failed to fetch {url}: {status}"

// app/app_model.rs
#[repr(u8)] pub enum LoadStep { LoadAssets = 0, LoadModel, WaitLoadModel, LoadExpression, WaitLoadExpression,
  LoadPhysics, WaitLoadPhysics, LoadPose, WaitLoadPose, SetupEyeBlink, SetupBreath, LoadUserData, WaitLoadUserData,
  SetupEyeBlinkIds, SetupLipSyncIds, SetupLook, SetupLayout, LoadMotion, WaitLoadMotion, CompleteInitialize,
  CompleteSetupModel, LoadTexture, WaitLoadTexture, CompleteSetup /* = 23 */ }
pub struct AppModel { state: LoadStep, user: CubismUserModel, scheduler: UpdateScheduler, /* … */ }
impl AppModel {
  pub async fn load(self: &Rc<RefCell<Self>>, dir: &str, file: &str, ctx: &AppContext, gen: Generation);
  pub fn update(&mut self, dt1: f64);                 // §7.6
  pub fn core(&self) -> Option<&CubismModel>; pub fn core_mut(&mut self) -> Option<&mut CubismModel>;
  pub fn draw(&mut self, projection: &mut Matrix44, canvas_w: u32, canvas_h: u32);
}

// app/manager.rs
pub type ParameterDriver = Box<dyn FnMut(&mut AppModel)>;
impl Live2dManager { pub fn on_update(&mut self, gl: &Gl, canvas_w: u32, canvas_h: u32, dt1: f64); /* §7.5 */ }
pub fn projection_for(canvas_w: u32, canvas_h: u32, view: &Matrix44) -> Matrix44;   // §8.2 incl. f32 rules

// player/driver.rs  (pure)
pub struct DriverInput<'a> { pub dt: f64 /* already capped */, pub rng: &'a mut dyn FnMut() -> f64 }
pub struct EmotionDriver { /* every field of §4.1 except DOM ones */ }
impl EmotionDriver {
  pub fn new(rng: &mut dyn FnMut() -> f64) -> Self;          // draws the initial blink interval
  pub fn set_emotion(&mut self, e: &str); pub fn set_spoken_char(&mut self, c: &str);
  pub fn set_lip_sync(&mut self, v: f64); pub fn set_eye_tracking(&mut self, x: f64, y: f64);
  pub fn step(&mut self, input: DriverInput) -> ParamFrame;  // §9.7 steps 2–15, returns 22 (id, f64) writes in order
}
pub struct PlayerClock { last: Option<f64> }                 // Clock 2: first call → dt 0; dt = min(Δ, 0.1)
// (JS uses `if (!_lastUpdateTime)`: a stored value of exactly 0 would re-initialize — only if performance.now() is 0; ignore)
// player/mod.rs
pub struct Live2dRuntime;
impl Live2dRuntime { pub fn create(host: &HtmlElement) -> PlayerHandle; }
pub struct PlayerHandle(Rc<RefCell<ReamaPlayer>>);
impl PlayerHandle { pub fn set_emotion(&self, e: &str); pub fn set_lip_sync(&self, v: f64);
  pub fn set_spoken_char(&self, c: &str); pub fn set_eye_tracking(&self, x: f64, y: f64); pub fn destroy(&self); }
```

`ParamFrame` should preserve the V1 write order (§9.7 steps 10, 13–15) and be applied with
`CubismModel::set_parameter_value_by_id(id, value as f64, 1.0)` (f32 store after clamp).

### R.3 JS → Rust correspondence

| V1 (JS) | V2 (Rust) |
|---|---|
| `window.ReamaCubismRuntime.create(host)` | `Live2dRuntime::create(&HtmlElement) -> PlayerHandle` |
| `runtime.preload()` / auto-preload at script eval | `amadeus_live2d::preload()` called by `amadeus-ui::preload_live2d()` at root mount |
| Core `<script>` + implicit readiness | `ensure_core_ready().await` (§3.3) |
| `acquireFramework` / `releaseFramework` | `framework::acquire()` / `release()` (thread_local ref-count) |
| `LAppPal` (static) | `FrameClock` per player (§7.2 fix) |
| `performance.now()` delta in `applyParameters` | `PlayerClock` (Clock 2) |
| `Math.random()` | `js_sys::Math::random` injected as `&mut dyn FnMut() -> f64` (tests: mulberry32) |
| `requestAnimationFrame(tick)` | `web_sys::Window::request_animation_frame` with an `Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>` self-rescheduling loop; store the handle for `cancel_animation_frame` |
| `ResizeObserver` | `web_sys::ResizeObserver` + `Closure` kept alive in `Subdelegate` |
| `new Image(); decode()` | `HtmlImageElement::new()`, `set_decoding("async")`, `decode()` → `JsFuture` |
| `fetch(url)` | `web_sys::Window::fetch_with_str` → `Response::array_buffer/text` (`JsFuture`) |
| `canvas.getContext("webgl", attrs)` | `get_context_with_context_options("webgl", &attrs_obj)` then `"webgl2"` |
| `Float32Array` parameter writes | Core binding `Float32Array::set_index` (f32) |
| `CubismLog*` (suppressed) | `log` crate at `trace` level or compiled out |
| `Live2DCubismCore.Logging.csmSetLogFunction(undefined)` | register a real `Closure<dyn FnMut(String)>` (no-op or `log::debug!`), keep it alive for the page lifetime (§4.4) |

### R.4 Leptos component (`amadeus-ui/src/components/live2d_canvas.rs`)

```rust
#[component]
pub fn Live2DCanvas(emotion: Signal<String>, lip_sync_value: Signal<f64>, spoken_char: Signal<String>,
                    eye_tracking_enabled: Signal<bool>) -> impl IntoView {
  // NodeRef<Div> class="live2d-host"; on mount (Effect after mount): spawn_local { ensure_core_ready().await?;
  //   if disposed {return}; let p = Live2dRuntime::create(&host); p.set_emotion(&emotion.get_untracked().to_uppercase());
  //   p.set_lip_sync(..); p.set_spoken_char(..); apply inline canvas styles (pointer-events:auto, width/height 100%) }
  // Effects: emotion → set_emotion(upper), lip → set_lip_sync, spoken → set_spoken_char,
  //   eye tracking: window mousemove listener (§3.4.2 mapping) / set_eye_tracking(0,0) when disabled
  // on_cleanup: capture canvas context, p.destroy(), WEBGL_lose_context.loseContext(), host.set_inner_html("")
}
```

### R.5 web-sys features needed

`Window, Document, Element, HtmlElement, HtmlCanvasElement, HtmlImageElement, CssStyleDeclaration,
WebGlRenderingContext, WebGl2RenderingContext, WebGlTexture, WebGlProgram, WebGlShader, WebGlBuffer,
WebGlFramebuffer, WebGlUniformLocation, WebGlContextAttributes, WebglLoseContext, CanvasRenderingContext2d,
ResizeObserver, ResizeObserverEntry, Performance, Response, Request, MouseEvent, HtmlScriptElement, Event`.

### R.6 Test plan

1. `noise.rs` / `tables.rs`: §12.1 values (tolerance 1e-12) and the vowel table.
2. `driver.rs`: replay `trace-A-1920x1045.json` player-only (inputs: Δ = 1/60, scripted setters, the three RNG draws)
   → compare the 22 driver parameters (tolerance 1e-6 after f32 rounding).
3. `manager.rs`: MVP table §8.3.
4. wasm integration (headless Chrome via `wasm-pack test --chrome`): full pipeline vs. the trace's `afterModelUpdate`
   and `final` arrays (requires the framework/physics port).
5. Manual QA: §9.12 timelines; console strings §11; remount 20× without context exhaustion.

---

## Open questions / risks

1. **Physics decoupled from the visible head motion (§7.8 b).** Faithful port keeps hair swaying on the hidden SDK
   breath rhythm, ignoring nods/sneezes/look-around. Probably unintended in V1, but changing it alters the look. Keep
   V1 behaviour; ask the owner before "fixing" (a fix = run the driver before the scheduler, or feed driver angles to
   physics).
2. **Core readiness / script order.** V1 can silently fail to show Kurisu (runtime evaluated before Core; Core WASM
   not yet instantiated). V2 must implement `ensure_core_ready()` with polling (§3.3). Risk if the Core ever exposes a
   different init (keep the poll generic).
3. **destroy() TypeError and leaks (§10.2).** V2 fixes them; no visible change, but the framework ref-count will now
   really reach 0 → `CubismFramework::dispose()` runs on each unmount and `start_up/initialize` on the next mount.
   Make sure dispose/initialize are cheap and the id manager re-interns correctly (parameter handles are resolved per
   player, after acquire — OK).
4. **Async continuations after destroy.** Rust futures holding `Rc`s keep the model alive and may touch a released GL
   context; use generation tokens / `Weak`.
5. **Two clocks.** Reproduced as specified; unifying them would change breath/physics timing very slightly (ms
   quantization) — acceptable but not required. Do not apply the 0.1 s cap to Clock 1 (physics has its own 5-s guard;
   breath must jump in phase after a background tab, as in V1).
6. **Floating-point parity.** `sin` implementations may differ by 1 ulp; noise/drift are smooth so differences stay
   ~1e-15. Golden comparisons need tolerances.
7. **Context loss never recovers** (V1). Optional V2 improvement: handle `webglcontextlost`/`restored`
   (rebuild renderer + re-upload texture from the cached image). Requires product decision; default = reproduce.
8. **Unclamped mouth lerp factors** (§9.7 step 15) make the mouth overshoot below ~10 fps; reproduce for parity.
9. **Blink comment mismatch**: first interval is 3.0–6.5 s (code) though the comment says 2.5–6.0 s; code wins.
10. **Katakana vowel gaps** in the vowel table (ア/イ/ウ/エ/オ → 0.5; the small forms are listed twice instead) —
    almost certainly a typo in V1; reproduce unless the owner approves the fix.
11. **Expressions folder unused.** Adding `Expressions` to model3.json would wake the latent expression hook and change
    faces; keep model3.json byte-identical in V2 assets.
12. **Non-ASCII asset paths** (`紅莉栖`): the axum static handler and the Tauri asset protocol must percent-decode
    UTF-8; test on Windows (WebView2), macOS (WKWebView) and Linux (WebKitGTK). Renaming assets to ASCII is possible
    but then model3.json and the preload list must change together.
13. **Linux WebKitGTK WebGL.** Some GPU/driver combos render a blank canvas in WebKitGTK (DMA-BUF renderer issues);
    the desktop shell may need `WEBKIT_DISABLE_DMABUF_RENDERER=1` / compositing fallbacks. Verify WebGL1 + 4096²
    texture + `antialias:true` on all three desktop targets.
14. **HiDPI cost.** Desktop canvas is 1920·DPR × 1045·DPR regardless of window size (3840×2090 at DPR 2) and draws
    every display refresh; acceptable in V1, watch weak iGPUs (no change recommended for parity).
15. **`devicePixelRatio` changes** without resize are ignored (V1); optional improvement.
16. **Multiple players** (remount overlap) shared `LAppPal` in V1; V2 per-player clock changes only that pathological
    case.
17. **Model-ready ordering** (§6.3 QUIRK): ensure `WaitLoadTexture` is set before texture callbacks can fire.
18. **Shader loading** is a separate readiness gate (§6.4): frames before `_isShaderLoaded` draw nothing; the V2
    renderer must keep that behaviour rather than drawing with half-initialized programs.
19. **Core log function.** V1 registers `undefined` as the Core log handler, so any Core error log throws
    (§4.4). V2 registers a real handler; error paths therefore behave slightly differently (no uncaught exception).
20. **Old `?v=19` cache-busting** disappears in V2 (Trunk hashes the WASM). Live2D assets under `/resources` keep the
    1-day cache headers (server spec).

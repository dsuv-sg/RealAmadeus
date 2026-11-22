# Porting spec: `live2d-framework-model` (Cubism Core FFI + Cubism Web Framework model/physics/effects/motion layer → `amadeus-live2d`)

Target crate: `amadeus-live2d` (wasm32 only; `wasm-bindgen`, `js-sys`, `web-sys`, `serde_json`). This spec covers:

* the **Core FFI**: every member of the proprietary `Live2DCubismCore` JS namespace that V2 must bind (the
  `live2dcubismcore.min.js` file is kept as-is), its asynchronous initialisation, its error/log behaviour and the
  typed-array-view semantics;
* the framework **model layer**: `CubismFramework` bootstrap, ID manager, logging, the JSON layer, `CubismModelSettingJson`
  (model3.json), `CubismMoc`, `CubismModel`, `CubismModelMultiplyAndScreenColor`, `CubismModelUserData`, `CubismUserModel`;
* **physics** (`CubismPhysics` + json + internal), **effects** (breath, eye blink, pose, look), the **motion system**
  (motion3.json, curve evaluation, `CubismMotion`, queue entry/manager, motion manager with priorities), the
  **expression system** (exp3.json, expression motion + manager), the **update scheduler** and every updater,
  `CubismMath` / `CubismVector2`;
* the parts of the V1 sample/entry glue that decide **which** of those features run and **in which order**
  (`LAppModel`, `LAppLive2DManager.onUpdate`, `lappdefine`), and a full inventory of the real model assets.

Companion spec (already written by another agent): `spec-live2d-framework-render.md` in the same folder — it owns the
renderer, shaders, clipping, `CubismMatrix44`, `CubismModelMatrix`, `CubismViewMatrix`, `CubismTargetPoint`, `csmRect`,
the GL context and the MVP. Where this spec needs one of those, it references the render spec section instead of
repeating it. The ReamaCubismPlayer (emotion driver) belongs to the player spec; only its framework-facing calls are
listed here.

Convention: behaviour that differs from what the code "obviously intends" is marked **QUIRK**. Unless a deviation is
explicitly declared safe, the Rust port reproduces QUIRKs. Everything below was read line-by-line from the sources in
section 0, and every model-specific number was **measured** by running the real deployed code with the real Core
(section 18).

---

## 0. Sources and provenance

### 0.1 Files read completely

Base dir `FW/` = `/tmp/claude-0/-home-elvelt-admin-Elvelt-dsuv/1f3badd8-3ee9-4a13-9340-35fac0e7abf1/scratchpad/ref/RealAmadeusWeb/CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/`,
`CORE/` = `.../CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Core/`.

| File | Lines | Content |
|---|---|---|
| `CORE/live2dcubismcore.d.ts` | 463 | public TS declaration of the Core namespace |
| `CORE/live2dcubismcore.js` | 829 | un-minified Core: JS wrapper lines 1–827, emscripten module (wasm embedded as base64 data URI) line 828 |
| `CORE/README.md`, `RedistributableFiles.txt` | – | `live2dcubismcore.d.ts/.js/.min.js` are redistributable under the Live2D Proprietary Software License |
| `FW/live2dcubismframework.ts` | 288 | `CubismFramework`, `Option`, `LogLevel`, `Constant`, `strtod`, `csmDelete` |
| `FW/cubismframeworkconfig.ts` | 32 | `CSM_LOG_LEVEL_*`, `CSM_LOG_LEVEL = VERBOSE` |
| `FW/cubismdefaultparameterid.ts` | 118 | standard parameter/part ID strings |
| `FW/icubismmodelsetting.ts` | 202 | abstract model-setting interface (25 abstract methods) |
| `FW/cubismmodelsettingjson.ts` | 790 | model3.json accessor |
| `FW/id/cubismid.ts`, `FW/id/cubismidmanager.ts` | 85, 114 | interned IDs |
| `FW/utils/cubismjson.ts`, `cubismjsonextension.ts` | 1244, 99 | JSON value tree (uses `JSON.parse`) |
| `FW/utils/cubismdebug.ts`, `cubismarrayutils.ts` | 162, 42 | logging, `updateSize` |
| `FW/model/cubismmoc.ts` | 155 | `CubismMoc` |
| `FW/model/cubismmodel.ts` | 1930 | `CubismModel` |
| `FW/model/cubismusermodel.ts` | 521 | `CubismUserModel` |
| `FW/model/cubismmodelmultiplyandscreencolor.ts` | 1066 | multiply/screen colour overrides |
| `FW/model/cubismmodeluserdata.ts`, `cubismmodeluserdatajson.ts` | 141, 117 | userdata3.json |
| `FW/physics/cubismphysicsinternal.ts`, `cubismphysicsjson.ts`, `cubismphysics.ts` | 251, 658, 1358 | physics |
| `FW/effect/cubismbreath.ts`, `cubismeyeblink.ts`, `cubismpose.ts`, `cubismlook.ts` | 123, 234, 396, 120 | effects |
| `FW/motion/acubismmotion.ts` | 443 | abstract motion (fade, callbacks) |
| `FW/motion/cubismmotioninternal.ts`, `cubismmotionjson.ts`, `cubismmotion.ts` | 155, 463, 1235 | motion data, motion3.json, `CubismMotion` |
| `FW/motion/cubismmotionqueueentry.ts`, `cubismmotionqueuemanager.ts`, `cubismmotionmanager.ts` | 262, 329, 126 | playback queue |
| `FW/motion/cubismexpressionmotion.ts`, `cubismexpressionmotionmanager.ts` | 365, 282 | expressions |
| `FW/motion/icubismupdater.ts`, `cubismupdatescheduler.ts` | 126, 180 | scheduler |
| `FW/motion/cubism{breath,eyeblink,expression,look,physics,pose,lipsync}updater.ts`, `iparameterprovider.ts` | 60–104 each, 41 | updaters |
| `FW/math/cubismmath.ts`, `cubismvector2.ts` | 376, 172 | math |
| consulted: `FW/rendering/cubismrenderer.ts` (`CubismTextureColor`, `CubismBlendMode`), `FW/math/cubismmodelmatrix.ts` (layout keys) | | |

Deployed V1 bundle: `/home/elvelt_admin/Elvelt/dsuv/public/resources/live2d/reama-cubism-runtime.js` (669,953 bytes,
18,643 lines, esbuild IIFE, comments kept, one `// <path>` marker per source file). Ranges used here (in addition to
the render spec table 0.3): `lapppal.ts` 2625–2664, `lappdefine.ts` 2665–2765, `effect/*` 4100–4685,
`motion/acubismmotion…cubismmotionmanager` 4958–7264, `physics/*` 7265–8528,
`model/cubismmodelmultiplyandscreencolor.ts` 8529–9352, `model/cubismmodel.ts` 9353–10748, `model/cubismmoc.ts`
14312–14419, userdata 14420–14589, `cubismusermodel.ts` 14590–14964, updaters 14965–15398, `lappmodel.ts`
15666–16384, `lapplive2dmanager.ts` 16385–16573, `scripts/cubism-runtime-entry.ts` 17689–18643.

Core shipped by V1: `/home/elvelt_admin/Elvelt/dsuv/public/resources/live2d/live2dcubismcore.min.js` — 228,042 bytes,
md5 `ec55199882d23fd1883ddc2cc1fec569`, **byte-identical** to `CORE/live2dcubismcore.min.js`. Runtime banner:
`Live2D Cubism SDK Core Version 6.0.1`; `Version.csmGetVersion()` = `100663297` (`0x06000001`).

### 0.2 Deployed code vs. ref TS

The render-spec agent rebuilt the bundle from the ref TS with the same esbuild options and diffed it
(`/tmp/claude-0/-home-elvelt-admin-Elvelt-dsuv/1f3badd8-3ee9-4a13-9340-35fac0e7abf1/scratchpad/build-l2d/bundle.diff`).
For every framework file in this spec the deployed code equals the ref TS **except**:

1. `CubismJson.arrayBufferToString` (bundle 1689–1700) gained a first branch
   `if (typeof TextDecoder !== "undefined") return new TextDecoder("utf-8").decode(buffer);` (the pristine version
   percent-encodes every byte and calls `decodeURIComponent`). → JSON text is decoded as UTF-8 with BOM stripping and
   U+FFFD replacement (section 4.5).
2. `lappdefine.ts`: `MOCConsistencyValidationEnable = false`, `MotionConsistencyValidationEnable = false` (pristine:
   `true`) and added `cachedFetchArrayBuffer` / `cachedFetchText` / `cachedLoadImage` helpers.
3. `lappmodel.ts` (asset prefetch + cached fetch), `lapplive2dmanager.ts` (projection, parameter driver), entry script —
   sample layer, described where relevant (sections 1.2, 10).

### 0.3 Ground truth

All model facts and all golden vectors in this spec were produced by
`/tmp/claude-0/-home-elvelt-admin-Elvelt-dsuv/1f3badd8-3ee9-4a13-9340-35fac0e7abf1/scratchpad/l2d-model-probe/harness.cjs`,
which runs the **deployed** bundle (framework classes exported from inside the IIFE) plus the **shipped** Core in a
Node 22 `vm` sandbox. Outputs: `.../scratchpad/l2d-model-probe/golden/` (section 18). Extra probes in the same folder:
`motions_summary.py` (motion inventory), `logcheck2.cjs` (Core logging/bad-moc behaviour), `corewrite.cjs` (Core
write-back behaviour).

---

## 1. Executive summary — what THIS model actually uses

### 1.1 Feature matrix (measured, see `golden/model_info.json`)

Model: `/resources/models/AmadeusKurisu5.0/reama5.0/Live2D紅莉栖forSDK5.0.model3.json` (421 bytes, verbatim in 17.1).

| Framework feature | V1 status for this model | Evidence |
|---|---|---|
| Core 6.0.1 `Moc`/`Model` | **used** | moc3 header `4D 4F 43 33 05 00 00 00` = `"MOC3"` + version byte **5** → `csmGetMocVersion` = 5 = `MocVersion_50` (moc 5.0.00–5.2.03); `csmGetLatestMocVersion` = 6 |
| `CubismModelSettingJson` | **used**: `Moc`, `Textures` (1), `Physics`, `Groups` | no `Motions`, `Expressions`, `Pose`, `UserData`, `HitAreas`, `Layout` keys; `DisplayInfo` (cdi3) is ignored by the framework |
| `CubismModel` parameters | **used**: 27 params, all type Normal(0), `repeats` all 0 | table 17.3 |
| parts | 15 parts, opacity 1, two nested (`Part12`→`Part11`, `Part10`→`Part9`) | 17.4 |
| drawables / masks | 49 drawables, masks used, 0 offscreens, `isBlendModeEnabled()` = false | render spec 1 |
| **Physics** | **used** — 3 settings, 12 inputs, 3 outputs (all `Angle`), 8 vertices, `Fps` 60 | 13.12 |
| **Breath** | **used** — 5 parameters (table 12.1.3) | `LAppModel.setupBreath` (bundle 15832–15860) |
| Look + `CubismTargetPoint` | instantiated and run **every frame**, but the drag target stays (0,0) forever → adds `0.0` to 6 params (only effect: clamp + f32 re-store, i.e. none) | bundle 15909–15936; no `addEventListener`/pointer handler exists anywhere in the bundle (`LAppSubdelegate.onTouchBegan/Moved/Ended` are defined but never attached), so `onDrag` is unreachable even though `Live2DCanvas.tsx` sets the canvas back to `pointer-events: auto`; the player's eye tracking goes through its own driver (`setEyeTracking`), not `CubismTargetPoint` |
| Eye blink | **not created**: `Groups[EyeBlink].Ids` = `[]` → `getEyeBlinkParameterCount()` = 0 → `if (count > 0)` false | bundle 15820–15831 |
| Lip-sync updater | **not created**: `Groups[LipSync].Ids` = `[]` | bundle 15893–15908 |
| Expressions | **not loaded** (`getExpressionCount()` = 0). The 8 files in `expressions/` are unreferenced **and** use Cubism-2 IDs (`PARAM_EYE_L_OPEN`, `PARAM_TERE`, …) that do not exist in the moc → they could never affect this model | 17.5 |
| Motions | **not loaded** (`getMotionGroupCount()` = 0). Every frame `motionManager.isFinished()` is true → `startRandomMotion("Idle", 1)` → `getMotionCount("Idle")` = 0 → returns `-1` without doing anything. The 93 `*.motion3.json` in `motions/` are unreferenced (most use IDs absent from the moc) | 17.6 |
| Pose / user data / hit areas / layout | none | `getPoseFileName()` = `""`, `getUserDataFile()` = `""`, `getHitAreasCount()` = 0, `getLayoutMap()` = false |
| Multiply/screen overrides, culling overrides, parameter repeat | never touched (defaults: off / SDK-override-with-repeat-false) | |
| moc/motion consistency validation | **disabled** in V1 (`lappdefine`) | 0.2 |
| `CubismPhysics.stabilization`, `setOptions`, `CubismMotion.setLoop`, `setMotionBehavior`, `setOffsetTime`, `stopAllMotions` with entries, `setRandomExpression`, `hitTest` | never called (grep of bundle: definitions only; `stopAllMotions()` only on an empty queue) | |

### 1.2 The exact V1 per-frame pipeline (framework-relevant part)

`ReamaCubismPlayer.tick` (rAF) → `LAppPal.updateTime()` → `LAppSubdelegate.update()` (resize check, `gl.clear`) →
`LAppView.render()` → `LAppLive2DManager.onUpdate()`:

```
// LAppPal.updateTime (bundle 2647-2651):
currentFrame = Date.now(); deltaTime = (currentFrame - lastFrame) / 1000; lastFrame = currentFrame
//   -> dt is always an integer number of milliseconds / 1000 (0, 0.016, 0.017, 0.033, ...).
//   -> the very first call yields Date.now()/1000 (~1.7e9 s) because lastFrame starts at 0; the model is
//      never in CompleteSetup on that frame, so it is never used.

LAppLive2DManager.onUpdate():                       (bundle 16465-16509)
  ...projection setup (render spec 3.9)...
  model.update():                                   // LAppModel.update (bundle 16028-16048)
    if (_state != CompleteSetup(23)) return
    dt = LAppPal.getDeltaTime(); _userTimeSeconds += dt
    cubismModel.loadParameters()                    // restore snapshot
    _motionUpdated = false
    if (motionManager.isFinished()) startRandomMotion("Idle", PriorityIdle = 1)   // no-op for this model
    else _motionUpdated = motionManager.updateMotion(cubismModel, dt)
    cubismModel.saveParameters()                    // snapshot
    updateScheduler.onLateUpdate(cubismModel, dt)   // sorted by execution order:
        //   EyeBlink 200 (absent) · Expression 300 (absent) · Look 400 · Breath 500 · Physics 600 ·
        //   LipSync 700 (absent) · Pose 800 (absent)
    cubismModel.update()                            // Core update + drawables.resetDynamicFlags()
  if (_parameterDriver):                            // ReamaCubismPlayer.applyParameters (player spec)
    _parameterDriver(model)                         //   22x cubismModel.setParameterValueById(id, value)  (weight 1, clamped)
    model.getModel().update()                       // CubismModel.update() again: Core update + resetDynamicFlags
  draw (render spec)
```

Consequences that the Rust port must reproduce:

1. The snapshot written by `saveParameters()` is **always the moc default values** for this model (no motion ever
   writes before the snapshot). Every frame therefore starts from the defaults; the driver's values never persist to
   the next frame and **never reach breath or physics**. Physics inputs are `default + breath (+ look 0)`; physics
   outputs (`ParamHairFront/Side/Back`) are not overridden by the driver. This is pinned by
   `golden/pipeline_dt60.jsonl` etc.
2. Physics runs at a fixed 60 Hz (`Fps: 60`) with output interpolation; at exactly 60 fps the displayed hair value is the
   **previous** physics step (one-frame lag, alpha = 0) — section 13.7.
3. A frame with `dt == 0` (two rAF callbacks in the same millisecond) makes physics return early without
   interpolating, so the hair parameters keep their just-restored default (0) for that frame (**QUIRK**, captured in
   `golden/pipeline_jitter.jsonl` frame 12).
4. A stall longer than 5 s (hidden tab) resets the physics accumulator (`MaxDeltaTime`), section 13.6.

### 1.3 Porting tiers

| Tier | Scope | Rationale |
|---|---|---|
| **A — parity-critical** | Core FFI (3), framework bootstrap + IdManager + JSON layer (4), model3.json subset (5), `CubismMoc` (6), `CubismModel` (7), scheduler + Look/Breath/Physics updaters (11), Breath (12.1), Look (12.4) + `CubismTargetPoint` (render spec 3.7), **Physics complete** (13), math (16) | everything that runs in V1 every frame |
| **B — implement, cheap, covered by goldens** | motion system (14), expression system (15), eye blink (12.2), pose (12.3), multiply/screen overrides (8), user data (9), physics `stabilization`, parameter repeat, `CubismUserModel` helpers (10) | not exercised by this model today, but the model folder ships 93 motions + 8 expressions and the user will expect V2 to be able to use them once referenced in model3.json; goldens exist for all of them |
| **C — skip or feature-gate** | moc consistency check (disabled in V1; keep a stub calling `hasMocConsistency`), motion3 consistency check, the hand-written `CubismJson` parser + `strtod` (dead code — the JSON.parse path is always used), `bezierEvaluateBinarySearch` (dead code), `CubismLipSyncUpdater` + `LAppWavFileHandler` (V1 lip sync is done by the driver writing `ParamMouthOpenY`), hit testing (pointer disabled), Cubism 5.3 offscreen/blend accessors beyond "count = 0" | dead or unreachable in V1 |

---

## 2. Numeric model (critical for golden tests)

* JS `number` = IEEE-754 binary64. Every arithmetic expression in the framework is evaluated in **f64**.
* Core arrays are `Float32Array` views into the emscripten heap: `parameters.values/minimumValues/maximumValues/
  defaultValues`, `parts.opacities`, all drawable float arrays. **Every store into them rounds to f32**
  (round-half-to-even, same as Rust `f64 as f32`); every read widens to f64.
* `CubismModel._savedParameters` is a plain JS array, filled from the `Float32Array` → values are f32-exact doubles;
  `loadParameters` writes them back unchanged.
* Values of **non-existent** parameters/parts (`_notExistParameterValues`, `_notExistPartOpacities`, `Map<number,
  number>`) are full **f64**, never clamped, and start as JS `null` (section 7.4).
* Physics: `_parameterCaches` and `_parameterInputCaches` are `Float32Array` (f32 rounding on every store);
  `_currentRigOutputs[*].outputs` / `_previousRigOutputs` are JS arrays (f64); particle vectors are f64; the
  `outParameterCaches`/`outParameterValues` temporaries are `Float32Array` slices (f32).
* Motion/expression math is f64; only the final write into the parameter array rounds to f32.
* `Math.sin/cos/atan2/acos/exp/log/sqrt` are V8's fdlibm ports. Rust `f64::sin` etc. (libm) can differ by 1 ulp for some
  inputs; golden comparisons must use a tolerance (recommend: parameters compared as f32 with `|Δ| <= 2 ulp(f32)` or
  `1e-6` absolute, physics rig outputs `1e-9` relative). If bit-exactness is ever required, use the `libm` crate (a
  musl/fdlibm port) for `sin/cos/atan2/acos/exp/log` — V8 also uses fdlibm-derived code, so results usually coincide.
* `Math.pow(x, 0.5)` (used by `CubismVector2.normalize`) equals `Math.sqrt(x)` for `x >= +0` in V8 (fdlibm special
  case) → Rust: `x.sqrt()`.

Rule for Rust: keep every JS `number` as `f64`; store `as f32` exactly where JS stores into a `Float32Array`; the list
of such store points is given per algorithm.

---

## 3. Core FFI — the `Live2DCubismCore` JS API to bind

### 3.1 Loading and readiness (**must design explicitly**)

* `live2dcubismcore.min.js` is a classic script that defines the global `var Live2DCubismCore` (a namespace object) and,
  at the end, `var _em = _em_module();`. The wasm binary is embedded as a base64 `data:` URI and compiled with the
  **asynchronous** path (`WebAssembly.instantiate` on the decoded bytes; `instantiateStreaming` is skipped for data
  URIs). The namespace exists immediately, but every function that reaches `_em.ccall` / `_em.addFunction` /
  `_em.HEAP*` throws a `TypeError` until the runtime is initialised (a few promise ticks after script execution).
  Nothing in the namespace exposes a ready promise (the emscripten module's `.then` is private in `_em`).
* V1 has no readiness check: it relies on the wasm being ready by the time the React component has laid out. Also,
  `Live2DCanvas.tsx` loads the Core and the runtime with **two parallel `async` scripts** (no ordering guarantee) while
  the runtime evaluates `Live2DCubismCore.ColorBlendType_*` **at script evaluation time** (enum `CubismColorBlend`,
  bundle 9356–9377) → if the runtime script finishes first it throws `ReferenceError: Live2DCubismCore is not defined`
  and `window.ReamaCubismRuntime` is never set (silent "no avatar"). **Latent V1 race; do not reproduce.**
* **V2 requirement:**
  1. Load `live2dcubismcore.min.js` (unchanged, served from `{base}/resources/live2d/`) before any wasm code touches the
     namespace (e.g. inject the `<script>` from Rust and await its `load` event, or list it in `index.html` before the
     Trunk wasm bootstrap; on desktop the same file is bundled as a Tauri asset).
  2. Await readiness by polling: `try { Live2DCubismCore.Version.csmGetVersion(); ready } catch { retry next
     setTimeout(…, 16) }`, max ~5 s, then fail with a clear error.
  3. Never read constants from the JS namespace at startup; they are compiled into Rust (3.2).

### 3.2 Constants (exact values, from `live2dcubismcore.js` 195–262)

| Name | Value | | Name | Value |
|---|---|---|---|---|
| `AlignofMoc` | 64 | | `ColorBlendType_Normal` | 0 |
| `AlignofModel` | 16 | | `ColorBlendType_AddCompatible` | 1 |
| `MocVersion_Unknown` | 0 | | `ColorBlendType_MultiplyCompatible` | 2 |
| `MocVersion_30` (3.0.00–3.2.07) | 1 | | `ColorBlendType_Add` | 3 |
| `MocVersion_33` (3.3.00–3.3.03) | 2 | | `ColorBlendType_AddGlow` | 4 |
| `MocVersion_40` (4.0.00–4.1.05) | 3 | | `ColorBlendType_Darken` | 5 |
| `MocVersion_42` (4.2.00–4.2.04) | 4 | | `ColorBlendType_Multiply` | 6 |
| `MocVersion_50` (5.0.00–5.2.03) | 5 | | `ColorBlendType_ColorBurn` | 7 |
| `MocVersion_53` (5.3.00–) | 6 | | `ColorBlendType_LinearBurn` | 8 |
| `ParameterType_Normal` | 0 | | `ColorBlendType_Lighten` | 9 |
| `ParameterType_BlendShape` | 1 | | `ColorBlendType_Screen` | 10 |
| `AlphaBlendType_Over` | 0 | | `ColorBlendType_ColorDodge` | 11 |
| `AlphaBlendType_Atop` | 1 | | `ColorBlendType_Overlay` | 12 |
| `AlphaBlendType_Out` | 2 | | `ColorBlendType_SoftLight` | 13 |
| `AlphaBlendType_ConjointOver` | 3 | | `ColorBlendType_HardLight` | 14 |
| `AlphaBlendType_DisjointOver` | 4 | | `ColorBlendType_LinearLight` | 15 |
| | | | `ColorBlendType_Hue` | 16 |
| | | | `ColorBlendType_Color` | 17 |

Bit helpers (`Live2DCubismCore.Utils`, 700–802; each is `(bitfield & (1 << n)) == (1 << n)`):

| Constant flags (`drawables.constantFlags`, `offscreens.constantFlags`) | mask | | Dynamic flags (`drawables.dynamicFlags`) | mask |
|---|---|---|---|---|
| `hasBlendAdditiveBit` | 1 | | `hasIsVisibleBit` | 1 |
| `hasBlendMultiplicativeBit` | 2 | | `hasVisibilityDidChangeBit` | 2 |
| `hasIsDoubleSidedBit` | 4 | | `hasOpacityDidChangeBit` | 4 |
| `hasIsInvertedMaskBit` | 8 | | `hasDrawOrderDidChangeBit` | 8 |
| | | | `hasRenderOrderDidChangeBit` | 16 |
| | | | `hasVertexPositionsDidChangeBit` | 32 |
| | | | `hasBlendColorDidChangeBit` | 64 |

Implement these as pure Rust bit tests (`const`s), never via JS calls.

### 3.3 Classes and members (every member V2 may touch)

All "views" below are **typed-array views into the Core's heap**, created **once** when the wrapper object is constructed
(they are not refreshed). Lengths are fixed at construction.

**`Version`** (static only)
* `csmGetVersion(): number` — `0xMMmmpppp` (major 8 bits, minor 8 bits, patch 16 bits). Core 6.0.1 → `100663297`.
* `csmGetLatestMocVersion(): number` — 6 for this Core.
* `csmGetMocVersion(mocBytes: ArrayBuffer): number` — copies the bytes into a fresh heap block (`csmMallocMoc`), reads the
  version, frees. Returns 0 if allocation fails. Overload `csmGetMocVersion(moc: Moc, mocBytes)` uses the revived moc
  pointer and `mocBytes.byteLength`; the framework only uses the single-argument form. Measured: this moc → 5; header byte
  patched to 99 → 99.

**`Logging`** (static only)
* `csmSetLogFunction(handler)` — stores `handler` in `Logging.logFunction`, then `_em.addFunction(Logging.wrapLogFunction,
  'vi')` and passes the table index to C. **QUIRK/RISK (verified):** the wasm function table cannot grow; the **first** call
  succeeds, **any second call throws the string** `"Unable to grow wasm table. Use a higher value for
  RESERVED_FUNCTION_POINTERS or set ALLOW_TABLE_GROWTH."` (after printing it via `console.log` and `console.warn`).
  V1 calls it from `CubismFramework.startUp(option)`; after `releaseFramework()` → `CubismFramework.cleanUp()`, a remount of
  the avatar component calls `startUp` again → throws (latent V1 bug on remount).
* **QUIRK/RISK (verified):** `wrapLogFunction` calls `Logging.logFunction(messageStr)` unconditionally. V1's `Option` has
  no `logFunction`, so `startUp` registers `undefined`; afterwards any Core log (bad moc, truncated moc, inconsistent
  moc) throws `TypeError: Logging.logFunction is not a function` out of `Moc.fromArrayBuffer` /
  `Moc.prototype.hasMocConsistency`. With a valid moc nothing is logged, so V1 is unaffected in practice.
* Core log messages observed (verbatim, including trailing `\n`):
  `[CSM] [E]csmReviveMocInPlace is failed. The Core unsupport later than moc3 ver:[6]. This moc3 ver is [99].\n`,
  `[CSM] [E]csmReviveMocInPlace: "size" is invalid.\n`, `[CSM] [E]csmHasMocConsistency: "size" is invalid.\n`.
* The banner `Live2D Cubism SDK Core Version 6.0.1` is printed by the Core through emscripten stdout (`console.log`) on
  every moc revive, **not** through the log callback.
* `csmGetLogFunction()` returns the stored JS handler (or `undefined`).
* **V2 rule:** call `csmSetLogFunction` **at most once per page lifetime**, always with a real function (forward to the
  Rust `log` facade); guard with a process-wide `OnceCell`.

**`Moc`**
* `static fromArrayBuffer(buffer: ArrayBuffer): Moc | null` — `null` if `buffer` is falsy; otherwise `new Moc(buffer)`:
  `memory = csmMallocMoc(byteLength)` (64-byte aligned); if 0 → returns an object without `_ptr` → `fromArrayBuffer`
  returns `null`; copies bytes into the heap; `_ptr = csmReviveMocInPlace(memory, byteLength)`; on failure frees memory
  and `_ptr` stays falsy → `null`. Measured: wrong version byte → `null`; truncated file → `null`.
* `hasMocConsistency(mocBytes: ArrayBuffer): number` — **instance** method but used statically by the framework as
  `Live2DCubismCore.Moc.prototype.hasMocConsistency(bytes)`; copies to a temp heap block, returns `1` if consistent else
  `0`, frees. Returns `undefined` if the temp allocation fails. This moc → `1`.
* `_release()` — `csmFree(_ptr)`, `_ptr = 0`.
* `_ptr: number` — heap address.

**`Model`**
* `static fromMoc(moc: Moc): Model | null` — `_ptr = csmMallocModelAndInitialize(moc._ptr)`; on failure `null`. On
  success constructs `parameters`, `parts`, `drawables`, `offscreens`, `canvasinfo` and
  `renderOrders = Int32Array(HEAP32, csmGetRenderOrders, drawableCount + offscreenCount)`.
* `getRenderOrders(): Int32Array` — the cached view. `renderOrders[objectIndex]` = drawing position (drawables first,
  then offscreens). This model: length 49, first values `48,47,46,45,44,43,42,41,40,38`.
* `update(): void` — `csmUpdateModel(_ptr)`. **Measured write-back:** the update **clamps parameter values in place** to
  `[min, max]` (measured on this model's non-repeat parameters; behaviour for `repeats != 0` not measured) (e.g. `ParamAngleX` 100 → 30, `ParamEyeLOpen` −5 → 0; `NaN` stays `NaN`) and **clamps part opacities
  to `[0, 1]`** (3 → 1, −1 → 0). It recomputes all drawable outputs and dynamic flags.
* `release(): void` — `csmFree(_ptr)`, `_ptr = 0`.
* `_ptr: number`.

**`CanvasInfo`** (`model.canvasinfo`, read once at construction via `csmReadCanvasInfo` into temporary heap blocks):
`CanvasWidth`, `CanvasHeight`, `CanvasOriginX`, `CanvasOriginY` (pixels), `PixelsPerUnit`. This model: 7000, 10000, 3500,
5000, 7000.

**`Parameters`** (`model.parameters`)
| Member | Type | Length | Notes |
|---|---|---|---|
| `count` | number | – | 27 |
| `ids` | `string[]` (JS strings decoded with `UTF8ToString`) | count | |
| `minimumValues`, `maximumValues`, `defaultValues` | `Float32Array` | count | read-only by convention |
| `values` | `Float32Array` | count | **read/write**; the model state |
| `types` | `Int32Array` | count | 0 Normal / 1 BlendShape |
| `repeats` | `Int32Array` | count | non-zero = editor "repeat" flag |
| `keyCounts` | `Int32Array` | count | |
| `keyValues` | `Float32Array[]` | count, each `keyCounts[i]` | |

**`Parts`** (`model.parts`): `count`, `ids: string[]`, `opacities: Float32Array` (read/write; clamped by `update`),
`parentIndices: Int32Array` (−1 none), `offscreenIndices: Int32Array` (−1 none; all −1 here).

**`Drawables`** (`model.drawables`): `count`, `ids`, `constantFlags: Uint8Array`, `dynamicFlags: Uint8Array`,
`textureIndices`, `drawOrders`, `opacities: Float32Array`, `maskCounts: Int32Array`, `masks: Int32Array[]` (each
`maskCounts[i]` long), `vertexCounts`, `vertexPositions: Float32Array[]` (each `2*vertexCounts[i]`), `vertexUvs:
Float32Array[]` (each `2*vertexCounts[i]`), `indexCounts`, `indices: Uint16Array[]` (each `indexCounts[i]`),
`multiplyColors: Float32Array` (`4*count`, RGBA), `screenColors: Float32Array` (`4*count`), `parentPartIndices:
Int32Array`, `blendModes: Int32Array` (**`2*count`** long — the framework reads `blendModes[i]`, colour blend in bits
0–7, alpha blend in bits 8–15), `resetDynamicFlags(): void` (`csmResetDrawableDynamicFlags`). Renderer usage: render spec 4.

**`Offscreens`** (`model.offscreens`): `count` (0 here), `blendModes: Int32Array` (`2*count`), `opacities`,
`ownerIndices`, `multiplyColors`/`screenColors` (`4*count`), `maskCounts`, `masks: Int32Array[]`, `constantFlags`.

**`Memory`**: `static initializeAmountOfMemory(size)` — calls `csmInitializeAmountOfMemory(size)` **only if `size >
16777216`**; otherwise no-op (default heap 16 MiB). The framework calls it from `CubismFramework.initialize(memorySize)`
(V1 passes 0 → no-op).

### 3.4 Heap growth and detached views (**RISK**)

The emscripten heap may grow (`_emscripten_resize_heap`: starting from `max(current, 16 MiB)`, the target size doubles
while `<= 536870912` bytes, then moves a quarter of the way towards 2 GiB (`min((3c + 2147483648)/4, 2147418112)`);
requests above 2147418112 fail). A growth replaces the `ArrayBuffer`; **every view held by `Parameters/Parts/Drawables/…` becomes
detached (length 0)** and the wrapper never refreshes them, so writes silently go nowhere. Measured: creating 200
additional models of this moc did not grow the heap (views stayed valid), so a single model is safe. V2 rules:

1. Create at most one `Moc`/`Model` per avatar; release them on destroy (`Model.release()`, `Moc._release()`).
2. After creating the model, record the expected lengths; on every `core_update()` check `values.length() == count`
   (cheap) and, if 0, treat the model as lost (log, recreate `Moc` + `Model` from the cached moc bytes, re-run setup).
3. Optionally call `Memory.initializeAmountOfMemory(32 * 1024 * 1024)` once, before the first `Moc` (exactly what
   `CubismFramework.initialize(memorySize)` would do) if more than one model is ever shown.

### 3.5 Recommended `wasm-bindgen` declarations (Rust)

```rust
// amadeus-live2d/src/core_ffi.rs  (wasm32 only)
use js_sys::{ArrayBuffer, Float32Array, Int32Array, Uint16Array, Uint8Array, Array, Function};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Live2DCubismCore, js_name = Version)]
    pub type CoreVersion;
    #[wasm_bindgen(static_method_of = CoreVersion, js_namespace = Live2DCubismCore, js_name = csmGetVersion, catch)]
    pub fn csm_get_version() -> Result<u32, JsValue>;            // Err until wasm ready
    #[wasm_bindgen(static_method_of = CoreVersion, js_namespace = Live2DCubismCore, js_name = csmGetLatestMocVersion)]
    pub fn csm_get_latest_moc_version() -> u32;
    #[wasm_bindgen(static_method_of = CoreVersion, js_namespace = Live2DCubismCore, js_name = csmGetMocVersion)]
    pub fn csm_get_moc_version(moc_bytes: &ArrayBuffer) -> u32;

    #[wasm_bindgen(js_namespace = Live2DCubismCore, js_name = Logging)]
    pub type CoreLogging;
    #[wasm_bindgen(static_method_of = CoreLogging, js_namespace = Live2DCubismCore, js_name = csmSetLogFunction, catch)]
    pub fn csm_set_log_function(handler: &Function) -> Result<(), JsValue>;   // call ONCE per page

    #[wasm_bindgen(js_namespace = Live2DCubismCore)]
    pub type Moc;
    #[wasm_bindgen(static_method_of = Moc, js_namespace = Live2DCubismCore, js_name = fromArrayBuffer, catch)]
    pub fn from_array_buffer(buffer: &ArrayBuffer) -> Result<Option<Moc>, JsValue>; // Ok(None) = invalid moc
    #[wasm_bindgen(method, js_name = _release)]
    pub fn release(this: &Moc);

    #[wasm_bindgen(js_namespace = Live2DCubismCore)]
    pub type Model;
    #[wasm_bindgen(static_method_of = Model, js_namespace = Live2DCubismCore, js_name = fromMoc, catch)]
    pub fn from_moc(moc: &Moc) -> Result<Option<Model>, JsValue>;
    #[wasm_bindgen(method)] pub fn update(this: &Model);
    #[wasm_bindgen(method)] pub fn release(this: &Model);
    #[wasm_bindgen(method, js_name = getRenderOrders)] pub fn get_render_orders(this: &Model) -> Int32Array;
    #[wasm_bindgen(method, getter)] pub fn parameters(this: &Model) -> CoreParameters;
    #[wasm_bindgen(method, getter)] pub fn parts(this: &Model) -> CoreParts;
    #[wasm_bindgen(method, getter)] pub fn drawables(this: &Model) -> CoreDrawables;
    #[wasm_bindgen(method, getter)] pub fn offscreens(this: &Model) -> CoreOffscreens;
    #[wasm_bindgen(method, getter)] pub fn canvasinfo(this: &Model) -> CoreCanvasInfo;

    pub type CoreCanvasInfo;   // getters: CanvasWidth, CanvasHeight, CanvasOriginX, CanvasOriginY, PixelsPerUnit (f32 widened, use f64)
    pub type CoreParameters;   // getters: count (u32), ids (Array of JsString), minimumValues/maximumValues/defaultValues/values (Float32Array),
                               //          types/repeats/keyCounts (Int32Array), keyValues (Array of Float32Array)
    pub type CoreParts;        // count, ids, opacities (Float32Array), parentIndices, offscreenIndices (Int32Array)
    pub type CoreDrawables;    // count, ids, constantFlags/dynamicFlags (Uint8Array), textureIndices, drawOrders, maskCounts,
                               // vertexCounts, indexCounts, parentPartIndices, blendModes (Int32Array), opacities, multiplyColors,
                               // screenColors (Float32Array), masks/vertexPositions/vertexUvs/indices (Array of typed arrays),
                               // method resetDynamicFlags()
    pub type CoreOffscreens;   // count, blendModes, opacities, ownerIndices, multiplyColors, screenColors, maskCounts, masks, constantFlags

    #[wasm_bindgen(js_namespace = Live2DCubismCore, js_name = Memory)]
    pub type CoreMemory;
    #[wasm_bindgen(static_method_of = CoreMemory, js_namespace = Live2DCubismCore, js_name = initializeAmountOfMemory)]
    pub fn initialize_amount_of_memory(size: u32);
}
```

(Getter declarations follow the same pattern: `#[wasm_bindgen(method, getter, js_name = minimumValues)] pub fn
minimum_values(this: &CoreParameters) -> Float32Array;`.) (`static_method_of = CoreVersion` uses the JS name given by the type's `js_name`, i.e. it emits
`Live2DCubismCore.Version.csmGetVersion()`.) If a `js_namespace` + `js_name` combination does not resolve with the
pinned wasm-bindgen version, fall back to `js_sys::Reflect::get(&js_sys::global(), "Live2DCubismCore")` + dynamic
`Function::call` for the ~10 static calls; the instance members work with plain `method, getter` bindings.

### 3.6 Data flow between Rust and the Core (recommended design)

Because all framework math runs in Rust, keep **Rust-side shadows** of the two writable Core inputs and synchronise in
bulk:

```
struct CoreModelHandle { model: Model, values_view: Float32Array, part_opacities_view: Float32Array,
                         drawables: CoreDrawables, ... }
// in CubismModel (Rust):  param_values: Vec<f32>, part_opacities: Vec<f32>  (authoritative between core updates)
fn core_update(&mut self) {
    self.handle.values_view.copy_from(&self.param_values);          // 1 JS call
    self.handle.part_opacities_view.copy_from(&self.part_opacities); // 1 JS call
    self.handle.model.update();
    self.handle.values_view.copy_to(&mut self.param_values);        // Core clamps in place (3.3) -> pull back
    self.handle.part_opacities_view.copy_to(&mut self.part_opacities);
    self.handle.drawables.reset_dynamic_flags();                     // CubismModel.update() semantics
}
```

This is behaviour-identical to V1 (where the framework reads/writes the views directly) because nothing reads the views
between the framework writes and `update()` except the framework itself. Drawable **outputs** are read by the renderer
through the views (zero-copy `Float32Array` handed to `gl.bufferData`), see render spec 4 / 15.

---

## 4. Framework bootstrap, IDs, logging, JSON layer

### 4.1 `CubismFramework` (`FW/live2dcubismframework.ts`)

Module state: `s_isStarted = false`, `s_isInitialized = false`, `s_option = null`, `s_cubismIdManager = null`.

| Function | Exact behaviour |
|---|---|
| `startUp(option = null): boolean` | if `s_isStarted` → `CubismLogInfo('CubismFramework.startUp() is already done.')`, return `s_isStarted` (true). `s_option = option`; if `option != null` → `Live2DCubismCore.Logging.csmSetLogFunction(option.logFunction)` (3.3 risks). `s_isStarted = true`. `version = csmGetVersion()`; `major = (version & 0xff000000) >> 24`, `minor = (version & 0x00ff0000) >> 16`, `patch = version & 0x0000ffff`; `CubismLogInfo('Live2D Cubism Core version: {0}.{1}.{2} ({3})', ('00'+major).slice(-2), ('00'+minor).slice(-2), ('0000'+patch).slice(-4), version)` → for 6.0.1: `Live2D Cubism Core version: 06.00.0001 (100663297)`; `CubismLogInfo('CubismFramework.startUp() is complete.')`; return true |
| `cleanUp()` | `s_isStarted = false; s_isInitialized = false; s_option = null; s_cubismIdManager = null` |
| `initialize(memorySize = 0)` | `CSM_ASSERT(s_isStarted)`; if not started → `CubismLogWarning('CubismFramework is not started.')`, return. If initialized → `CubismLogWarning('CubismFramework.initialize() skipped, already initialized.')`, return. `Value.staticInitializeNotForClientCall()` (creates shared `JsonBoolean.trueValue/falseValue`, `Value.errorValue = new JsonError('ERROR', true)`, `Value.nullValue = new JsonNullvalue()`, `dummyKeys = []`); `s_cubismIdManager = new CubismIdManager()`; `Live2DCubismCore.Memory.initializeAmountOfMemory(memorySize)` (no-op unless > 16777216); `s_isInitialized = true`; `CubismLogInfo('CubismFramework.initialize() is complete.')` |
| `dispose()` | `CSM_ASSERT(s_isStarted)`; not started → warn `'CubismFramework is not started.'`; not initialized → `CubismLogWarning('CubismFramework.dispose() skipped, not initialized.')`, return. `Value.staticReleaseNotForClientCall()`; `s_cubismIdManager.release(); s_cubismIdManager = null`; `CubismRenderer.staticRelease()` (render spec); `s_isInitialized = false`; `CubismLogInfo('CubismFramework.dispose() is complete.')` |
| `isStarted()`, `isInitialized()`, `getIdManager()` | getters |
| `coreLogFunction(message)` | if `!Logging.csmGetLogFunction()` return; else call it with `message` |
| `getLoggingLevel()` | `s_option != null ? s_option.loggingLevel : LogLevel_Off` |

`Option { logFunction, loggingLevel }`; `LogLevel { Verbose 0, Debug 1, Info 2, Warning 3, Error 4, Off 5 }`;
`Constant = { vertexOffset: 0, vertexStep: 2 }`; `csmDelete(x)` is a no-op.

V1 usage (entry, bundle 18011–18033): reference-counted `acquireFramework()` → on first use
`option = new Option(); option.loggingLevel = 5 /* Off */; startUp(option); initialize();` (no `logFunction`!);
`releaseFramework()` → when the count reaches 0: `dispose()` if initialized, `cleanUp()` if started.

**V2:** `amadeus_live2d::framework::start()` = readiness wait (3.1) + one-shot log binding + `IdManager` creation;
no global dispose (the Core and the interner live for the page lifetime). Model objects own their Core handles.

### 4.2 IDs (`CubismId`, `CubismIdManager`)

* `CubismIdManager` keeps a list of `CubismId`; `registerId(id: string)`: linear search by string; return the existing
  object or create, push and return a new one. `getId(id) = registerId(id)`; `isExist(id)` = found. (Non-string input
  recurses forever — unreachable.) `release()` clears the list.
* `CubismId { _id }`: `getString()`, `isEqual(c)` / `isNotEqual(c)` accept a string or a `CubismId` (string compare).
* Everywhere else the framework compares handles **by object identity** (`==` between `CubismId` objects), which equals
  string equality because the manager interns.
* **Rust:** `#[derive(Copy, Clone, Eq, PartialEq, Hash)] pub struct CubismId(u32)` + a page-global interner
  (`IdManager { by_name: HashMap<Box<str>, u32>, names: Vec<Box<str>> }`, never cleared) with `CubismId::new(&str)` and
  `id.as_str()`. Standard IDs (`CubismDefaultParameterId`, table 4.6) as `const` strings.

### 4.3 Logging (`FW/utils/cubismdebug.ts`, `cubismframeworkconfig.ts`)

`CSM_LOG_LEVEL = CSM_LOG_LEVEL_VERBOSE (0)`, so all five helpers exist:
`CubismLogVerbose/Debug/Info/Warning/Error(fmt, ...args)` → `CubismLogPrintIn(level, '[V]'|'[D]'|'[I]'|'[W]'|'[E]' + fmt,
args)` → `CubismLogPrint(level, fmt + '\n', args)` → `CubismDebug.print(level, '[CSM]' + fmt, args)`. Final text:
`[CSM][I]<message>\n` (no space). `CubismDebug.print`: if `level < CubismFramework.getLoggingLevel()` → return; replace
every `{N}` with `args[N]` (regex `/\{(\d+)\}/g`; missing args print `undefined`); call
`CubismFramework.coreLogFunction(buffer)`. `dumpBytes` prints bytes 16 per line (unused).

In V1 the level is Off (5) → **every framework log is suppressed**. Output that bypasses `CubismDebug` (visible in V1's
console when triggered):
* `CSM_ASSERT(expr)` = `console.assert(expr)` ("Assertion failed" when falsy). Sites: `CubismFramework.initialize/
  dispose`, every `CubismModel` index-range check, `CubismMoc.release` (`_modelCount == 0`), `updateFadeWeight`
  (`0 <= w <= 1`), `CSM_ASSERT(0)` in the default branch of motion segment parsing/consistency.
* `CubismMath.mod`: ``console.warn(`divided: ${dividend}, divisor: ${divisor} mod() returns 'NaN'.`)``
* `CubismExpressionMotionManager.getFadeWeight`: `console.warn('Failed to get the fade weight value. The element at that
  index does not exist.')`; `setFadeWeight`: `console.warn('Failed to set the fade weight value. The element at that index
  does not exist.')`.

**Rust:** `log` crate, target `"cubism"`, same message texts (without the `[CSM][X]` prefix, which `log` levels replace);
default max level Off for parity, `debug` builds may enable Warn.

### 4.4 JSON value semantics (`CubismJson`, `CubismJsonExtension`) → `serde_json`

**Parsing** (`CubismJson.create(buffer, size)` → `parseBytes(buffer, size, CubismJsonExtension.parseJsonObject)`):
1. Decode the **whole** buffer (`size` ignored) with `new TextDecoder("utf-8").decode(buffer)` (deployed variant):
   a leading UTF-8 BOM is stripped, invalid byte sequences become U+FFFD.
2. `JSON.parse(text)` — a syntax error **throws** (`SyntaxError`) out of `create`, out of every constructor that uses it
   (`CubismModelSettingJson`, `CubismPhysicsJson`, `CubismMotionJson`, `CubismModelUserDataJson`, `CubismPose.create`,
   `CubismExpressionMotion.create`). `create` therefore never returns `null` in practice.
3. `parseJsonObject(obj, new JsonMap())`: `Object.keys(obj).forEach(key => …)`: boolean → new `JsonBoolean`, string →
   `JsonString`, number → `JsonFloat`, `Array` → `JsonArray` (recursive, element-wise with the same rules), other object →
   `JsonMap` (recursive), `null` → new `JsonNullvalue`. Root array → map keyed `"0"`, `"1"`, …; root primitive → empty
   map; root `null` → `TypeError` from `Object.keys(null)`.
4. **Key order** of every `JsonMap` = `Object.keys` order: integer-like keys (canonical numeric strings `0`…`4294967294`)
   first in ascending numeric order, then all other keys in document order. Matters for `getMotionGroupName(i)` and
   `getLayoutMap` iteration. **Rust:** `serde_json` with feature `preserve_order`, plus a helper `js_key_order(map)`
   that yields integer-like keys first (ascending) then the rest in order.

**Accessor semantics** (needed to reproduce defaults on missing/wrong-typed fields):

| Call on … | Map | Array | String | Float | Bool | Null | Error |
|---|---|---|---|---|---|---|---|
| `getValueByString(k)` | value or **Null** | **Error** ("Error: type mismatch") | Null | Null | Null | Null | Null |
| `getValueByIndex(i)` | Error ("Error: type mismatch") | element; out of range → Error ("Error: index out of bounds") | Error | Error | Error | Error | Error |
| `toFloat(d = 0)` | d | d | d (**no coercion of numeric strings**) | value | d | d | d |
| `toInt(d = 0)` | d | d | d | `parseInt(value.toString())` (truncates toward 0; `1e21` → 1; NaN → NaN) | d | d | d |
| `toBoolean(d = false)` | d | d | d | d | value | d | d |
| `getRawString()` / `getString()` | debug dump | debug dump | the string | **QUIRK** returns `"\0"` and sets its own value to `NaN` | `"true"`/`"false"` | current text of the shared null value (`"NullValue"` initially, afterwards the last error text written by `setErrorNotForClientCall`, e.g. `"Error: type mismatch"`) | its message (`"ERROR"`, `"Error: type mismatch"`, `"Error: index out of bounds"`) |
| `getSize()` | **QUIRK** `this._keys.length` → `TypeError` unless `getKeys()` ran first (never hit by the framework) | length | 0 | 0 | 0 | 0 | 0 |
| `getVector()` | `[]` | elements | `[]` | `[]` | `[]` | `[]` | `[]` |
| `getMap()` | entries | `undefined` | `undefined` | … | … | `undefined` | `undefined` |
| `getKeys()` | keys (cached) | `[]` (shared) | `[]` | … | … | `[]` | `[]` |
| `isNull()` / `isError()` | false/false | false/false | false/false | false/false | false/false | **true**/false | false/**true** |

`Value.nullValue` / `Value.errorValue` are **shared singletons** whose text is overwritten by
`setErrorNotForClientCall` (`JsonError.setErrorNotForClientCall` returns `this`; `JsonNullvalue.setErrorNotForClientCall`
sets the text and returns the null singleton). The text is only observable through `getRawString()` on a missing value
(e.g. a missing `Name` would yield the expression name `"NullValue"` or `"Error: type mismatch"`). **Rust:** expose
`Option<&str>`/`Option<f64>` accessors and document these placeholders as the V1 result for malformed files; well-formed
files never hit them.

**Dead code (do not port):** the hand-written `parseValue/parseString/parseObject/parseArray` parser and
`strtod(s, endPtr)` (only reachable when no parse callback is given — never). (`strtod` scans prefixes with `Number()`,
skipping `e`, `-`, `E`; returns `parseFloat(s)`.)

### 4.5 `updateSize` and "plain object" placeholders (`FW/utils/cubismarrayutils.ts`)

`updateSize(arr, newSize, value = null, callPlacementNew = null)`: grow → fill new slots; when `callPlacementNew` is true
and `value` is a class, each new slot is `JSON.parse(JSON.stringify(new value()))` — a **plain object** with only the
fields whose constructor values are JSON-serialisable (fields left `undefined` disappear; nested `CubismVector2` become
`{x, y}` without methods). Shrink → `arr.length = newSize`. Physics and motion parsing allocate their arrays this way
(consequences in 13.2 and 14.2).

### 4.6 Standard IDs (`CubismDefaultParameterId`, verbatim)

Parts: `HitAreaPrefix 'HitArea'`, `HitAreaHead 'Head'`, `HitAreaBody 'Body'`, `PartsIdCore 'Parts01Core'`,
`PartsArmPrefix 'Parts01Arm_'`, `PartsArmLPrefix 'Parts01ArmL_'`, `PartsArmRPrefix 'Parts01ArmR_'`.
Parameters (value = key): `ParamAngleX ParamAngleY ParamAngleZ ParamEyeLOpen ParamEyeLSmile ParamEyeROpen ParamEyeRSmile
ParamEyeBallX ParamEyeBallY ParamEyeBallForm ParamBrowLY ParamBrowRY ParamBrowLX ParamBrowRX ParamBrowLAngle
ParamBrowRAngle ParamBrowLForm ParamBrowRForm ParamMouthForm ParamMouthOpenY ParamCheek ParamBodyAngleX ParamBodyAngleY
ParamBodyAngleZ ParamBreath ParamArmLA ParamArmRA ParamArmLB ParamArmRB ParamHandL ParamHandR ParamHairFront ParamHairSide
ParamHairBack ParamHairFluffy ParamShoulderY ParamBustX ParamBustY ParamBaseX ParamBaseY`, and `ParamNONE 'NONE:'`.

---

## 5. `CubismModelSettingJson` (model3.json)

Constructor `(buffer, size)`: `_json = CubismJson.create(buffer, size)` (throws on invalid JSON); then caches
`_jsonValue[FrequestNode]` in this order: `0 Groups` = `root.Groups`, `1 Moc` = `root.FileReferences.Moc`, `2 Motions`,
`3 Expressions`, `4 Textures`, `5 Physics`, `6 Pose` (all under `FileReferences`), `7 HitAreas` = `root.HitAreas`.
"exists" below means `!isNull() && !isError()` of the cached node.

| Accessor | Behaviour | This model |
|---|---|---|
| `getModelFileName()` | exists(Moc) ? `Moc.getRawString()` : `''` | `Live2D紅莉栖forSDK5.0.moc3` |
| `getTextureCount()` | exists ? `Textures.getSize()` : 0 | 1 |
| `getTextureDirectory()` | **no existence check**: `Textures[0].getRawString().split('/')`, join all but the last element with `'/'` | `Live2D紅莉栖forSDK5.0.8192` |
| `getTextureFileName(i)` | `Textures[i].getRawString()` (no check; out of range → `"Error: index out of bounds"`) | `Live2D紅莉栖forSDK5.0.8192/texture_00.png` |
| `getHitAreasCount()` | exists ? size : 0 | 0 |
| `getHitAreaId(i)` / `getHitAreaName(i)` | `getId(HitAreas[i].Id raw)` / `HitAreas[i].Name raw` | – |
| `getPhysicsFileName()` | exists ? raw : `''` | `Live2D紅莉栖forSDK5.0.physics3.json` |
| `getPoseFileName()` | exists ? raw : `''` | `''` |
| `getExpressionCount()` | exists ? size : 0 | 0 |
| `getExpressionName(i)` / `getExpressionFileName(i)` | `Expressions[i].Name raw` / `Expressions[i].File raw` | – |
| `getMotionGroupCount()` | exists(Motions) ? `Motions.getKeys().length` : 0 | 0 |
| `getMotionGroupName(i)` | exists ? `Motions.getKeys()[i]` (JS key order 4.4) : `null` | – |
| `getMotionCount(group)` | `Motions[group]` exists ? its size : 0 | `Idle` → 0 |
| `getMotionFileName(group, i)` | group exists ? `Motions[group][i].File raw` : `''` | – |
| `getMotionSoundFileName(group, i)` | `Motions[group][i].Sound` exists ? raw : `''` | – |
| `getMotionFadeInTimeValue(group, i)` | `…FadeInTime` exists ? `toFloat()` : **−1.0** | −1 |
| `getMotionFadeOutTimeValue(group, i)` | `…FadeOutTime` exists ? `toFloat()` : −1.0 | – |
| `getUserDataFile()` | `root.FileReferences.UserData` exists ? raw : `''` | `''` |
| `getLayoutMap(out: Map)` | `root.Layout.getMap()`; `undefined` → false; else `out.set(key, value.toFloat())` for every entry (JS key order), return true iff ≥ 1 entry | false |
| `getEyeBlinkParameterCount()` | if `!isExistEyeBlinkParameters()` → 0; loop `Groups[i]` (skip null/error entries); first entry with `Name raw == 'EyeBlink'` → `Ids.getVector().length`, break | **0** (group exists, `Ids: []`) |
| `getEyeBlinkParameterId(i)` | not exist → `null`; first `EyeBlink` group → `getId(Ids[i] raw)` | – |
| `getLipSyncParameterCount()` / `getLipSyncParameterId(i)` | same with `'LipSync'` | **0** |
| `isExistEyeBlinkParameters()` / `isExistLipSyncParameters()` | Groups null/error → false; true if any `Groups[i].Name raw` equals the name (`Target` is not checked) | true / true |

Key strings (verbatim): `Version`, `FileReferences`, `Groups`, `Layout`, `HitAreas`, `Moc`, `Textures`, `Physics`, `Pose`,
`Expressions`, `Motions`, `UserData`, `Name`, `File`, `Id`, `Ids`, `Target`, `Idle`, `TapBody`, `PinchIn`, `PinchOut`,
`Shake`, `FlickHead`, `Parameter`, `Sound`, `FadeInTime`, `FadeOutTime`, `CenterX`, `CenterY`, `X`, `Y`, `Width`, `Height`,
`LipSync`, `EyeBlink`, `init_param`, `init_parts_visible`, `val` (the last eleven are declared but unused).

**QUIRK (layout):** `CubismModelMatrix.setupFromLayout` (render spec 3.4) only recognises lowercase keys `width`,
`height`, `x`, `y`, `center_x`, `center_y`, `top`, `bottom`, `left`, `right`, while model3.json files written by the
Cubism Editor use `Width`, `CenterX`, … → such `Layout` entries are silently ignored. No effect for this model.

**Rust:** `ModelSetting` struct deserialised once (serde, all fields optional) plus methods with the exact defaults above;
keep raw `serde_json::Value` for `Motions` (needs key order) and `Groups`.

---

## 6. `CubismMoc`

| Member | Behaviour |
|---|---|
| `static create(mocBytes, shouldCheckMocConsistency)` | if checking: `CubismMoc.hasMocConsistency(mocBytes)` false → `CubismLogError('Inconsistent MOC3.')`, return `null`. `moc = Live2DCubismCore.Moc.fromArrayBuffer(mocBytes)`; if non-null → `new CubismMoc(moc)` with `_mocVersion = Version.csmGetMocVersion(mocBytes)`; return it (or `null`) |
| `createModel()` | `Model.fromMoc(_moc)`; success → `new CubismModel(model)`, `initialize()`, `++_modelCount`; return model or `null` |
| `deleteModel(model)` | if non-null: `model.release()`, `--_modelCount` |
| `release()` | `CSM_ASSERT(_modelCount == 0)`; `_moc._release()`; `_moc = null` |
| `static delete(moc)` | `moc._moc._release(); moc._moc = null` |
| `getLatestMocVersion()` | `Version.csmGetLatestMocVersion()` (6) |
| `getMocVersion()` | `_mocVersion` (5) |
| `static getMocVersionFromBuffer(bytes)` | `Version.csmGetMocVersion(bytes)` |
| `static hasMocConsistency(bytes)` | `Moc.prototype.hasMocConsistency(bytes) === 1` |

V1 passes `shouldCheckMocConsistency = false`. If the moc fetch fails, V1 substitutes `new ArrayBuffer(0)` and
`Moc.fromArrayBuffer` returns `null` (or throws `TypeError` because of the `undefined` log function, 3.3) → model never
appears. **Rust:** `CubismMoc::create(bytes: &[u8], check: bool) -> Result<CubismMoc, ModelError>`; copy bytes into a
`js_sys::Uint8Array` → `.buffer()`; map `None` to `ModelError::InvalidMoc { version: csm_get_moc_version(...) ,
latest: 6 }` with message `Inconsistent MOC3.` / `Failed to CubismMoc.create().`.

---

## 7. `CubismModel`

### 7.1 Fields and constructor (`new CubismModel(coreModel)`)

`_model = coreModel`; `_parameterValues/_parameterMaximumValues/_parameterMinimumValues/_partOpacities/
_offscreenOpacities = null` (filled by `initialize`); `_savedParameters = []`; `_parameterIds = []`, `_drawableIds = []`,
`_partIds = []`; **`_isOverriddenParameterRepeat = true`**; `_isOverriddenCullings = false`; `_modelOpacity = 1.0`;
`_overrideMultiplyAndScreenColor = new CubismModelMultiplyAndScreenColor(this)`; `_isBlendModeEnabled = false`;
colour caches `null`; `_userParameterRepeatDataList = []`, `_userDrawableCullings = []`, `_userOffscreenCullings = []`,
`_partsHierarchy = []`; `_notExistPartId = Map<CubismId, number>`, `_notExistParameterId = Map<CubismId, number>`,
`_notExistParameterValues = Map<number, number>`, `_notExistPartOpacities = Map<number, number>`;
`_drawableColorBlends = Array(drawables.count).fill(-1)`, `_drawableAlphaBlends` same,
`_offscreenColorBlends/_offscreenAlphaBlends = Array(offscreens.count).fill(-1)`.

### 7.2 `initialize()` (called once by `CubismMoc.createModel`)

1. Cache views: `_parameterValues = parameters.values`, `_partOpacities = parts.opacities`, `_offscreenOpacities =
   offscreens.opacities`, `_parameterMaximumValues`, `_parameterMinimumValues`.
2. For each parameter `i`: `_parameterIds[i] = getId(parameters.ids[i])`,
   `_userParameterRepeatDataList[i] = new ParameterRepeatData(false, false)` (`{isOverridden, isParameterRepeated}`).
3. For each part: `_partIds[i] = getId(parts.ids[i])`.
4. Culling: `_userDrawableCullings.length = drawableCount`; **one** `CullingData(false, false)` object stored in every
   slot; same for offscreens with a second shared object (**QUIRK**: per-index setters change all slots, 7.7).
5. `_drawableIds.push(getId(drawables.ids[i]))` for each drawable.
6. Blend-mode detection (render spec 4.3): offscreens > 0 → true; else true iff some drawable has
   `!(colorBlend == Normal && alphaBlend == Over) && colorBlend != AddCompatible && colorBlend != MultiplyCompatible`
   (this model: false).
7. `setupPartsHierarchy()` (7.8).
8. `_overrideMultiplyAndScreenColor.initialize(partCount, drawableCount, offscreenCount)` (section 8).

### 7.3 `update()`

`_model.update(); _model.drawables.resetDynamicFlags();` (Core write-back: 3.3; flag semantics: render spec 4.2).
Rust: `core_update()` (3.6).

### 7.4 Parameters

Index resolution — `getParameterIndex(id)`:
```
for i in 0..parameters.count: if id == _parameterIds[i] return i
if _notExistParameterId.has(id) return _notExistParameterId.get(id)
index = parameters.count + _notExistParameterId.size
_notExistParameterId.set(id, index); _notExistParameterValues.set(index, null)
return index                       // NEVER -1: unknown IDs get virtual slots >= count
```

| Method | Exact behaviour |
|---|---|
| `getParameterCount()` | `parameters.count` (27) |
| `getParameterType(i)`, `getParameterMaximumValue(i)`, `getParameterMinimumValue(i)`, `getParameterDefaultValue(i)` | direct Core reads (no range check) |
| `getParameterId(i)` | `getId(parameters.ids[i])` |
| `getParameterValueByIndex(i)` | virtual slot → stored value (initially JS `null`); else `CSM_ASSERT(0 <= i < count)`; `_parameterValues[i]` |
| `getParameterValueById(id)` | `getParameterValueByIndex(getParameterIndex(id))` |
| `setParameterValueByIndex(i, value, weight = 1.0)` | virtual slot → `set(i, weight == 1 ? value : stored * (1 - weight) + value * weight)` (f64, no clamp), return. Else: `value = isRepeat(i) ? getParameterRepeatValue(i, value) : getParameterClampValue(i, value)`; `_parameterValues[i] = (weight == 1) ? value : _parameterValues[i] * (1 - weight) + value * weight` (**clamp happens before blending**; f32 store) |
| `setParameterValueById(id, value, weight = 1.0)` | via `getParameterIndex` |
| `addParameterValueByIndex(i, value, weight = 1.0)` | `setParameterValueByIndex(i, getParameterValueByIndex(i) + value * weight)` (inner weight 1) |
| `addParameterValueById(id, value, weight = 1.0)` | via index |
| `multiplyParameterValueByIndex(i, value, weight = 1.0)` / `ById` | `setParameterValueByIndex(i, getParameterValueByIndex(i) * (1.0 + (value - 1.0) * weight))` |
| `isRepeat(i)` | virtual → false; `CSM_ASSERT`; if `_isOverriddenParameterRepeat \|\| _userParameterRepeatDataList[i].isOverridden` → `_userParameterRepeatDataList[i].isParameterRepeated`; else `parameters.repeats[i] != 0` |
| `getParameterRepeatValue(i, value)` | virtual → value. `max, min, size = max - min`; `if (max < value) { o = CubismMath.mod(value - max, size); value = isNaN(o) ? max : min + o }`; `if (value < min) { o = mod(min - value, size); value = isNaN(o) ? min : max - o }`; return value |
| `getParameterClampValue(i, value)` | virtual → value; `CubismMath.clamp(value, min, max)` (`<min → min`, `max< → max`, NaN passes) |
| `getParameterRepeats(i)` | `parameters.repeats[i] != 0` |
| `get/setOverrideFlagForModelParameterRepeat()` | `_isOverriddenParameterRepeat` (default **true**) |
| `get/setOverrideFlagForParameterRepeat(i[, v])`, `get/setRepeatFlagForParameterRepeat(i[, v])` | per-index `isOverridden` / `isParameterRepeated` |
| `saveParameters()` | for `i < count`: `i < saved.length ? saved[i] = values[i] : saved.push(values[i])` (virtual slots are **not** saved) |
| `loadParameters()` | `n = min(count, saved.length)`; `values[i] = saved[i]` (virtual slots are **not** restored) |

**QUIRK (repeat):** because `_isOverriddenParameterRepeat` defaults to true and every per-index flag is false, the
editor's repeat flag is ignored and every real parameter is **clamped** unless the app calls
`setOverrideFlagForModelParameterRepeat(false)`. V1 never does. (This model has no repeat parameters anyway.)

**Virtual-slot (`null`) semantics:** the stored `null` behaves as `0` in every arithmetic and relational expression the
framework performs (`null * x`, `null + x`, `null > 0.001`), so Rust can store `Option<f64>` and treat `None` as `0.0`
everywhere except the public getter (which can return `Option<f64>`). Virtual values are f64, unclamped, never saved or
restored, and never seen by the Core.

### 7.5 Parts

| Method | Behaviour |
|---|---|
| `getPartIndex(id)` | linear search `_partIds`; else virtual slot exactly like parameters (`parts.count + _notExistPartId.size`, value `null`) |
| `getPartId(i)` | `getId(parts.ids[i])` |
| `getPartCount()` | `parts.count` (15) |
| `setPartOpacityByIndex(i, opacity)` | virtual → set and return; `CSM_ASSERT`; `_partOpacities[i] = opacity` (f32, **no clamp** — the Core clamps on `update()`) |
| `setPartOpacityById(id, opacity)` | `index = getPartIndex(id)`; `if (index < 0) return` (never); by index |
| `getPartOpacityByIndex(i)` | virtual → stored (`null`); else `_partOpacities[i]` |
| `getPartOpacityById(id)` | index < 0 → 0 (never); by index |
| `getPartOffscreenIndices()`, `getPartParentPartIndices()` | Core views |

Part opacities are never touched by V1 (no pose, no part-opacity curves applied to parts — see 14.4).

### 7.6 Drawables, colours, blend modes, masks

Full accessor table and semantics: render spec 4.1 (identical behaviour; do not duplicate the implementation). Additional
details owned here:
* `getDrawableIndex(id)`: linear search `_drawableIds` by identity → index or **−1** (no virtual slots).
* `getDrawableId(i)`: `getId(drawables.ids[i])`.
* `getDrawableMultiplyColor(i)` / `getDrawableScreenColor(i)` / `getOffscreenMultiplyColor(o)` /
  `getOffscreenScreenColor(o)`: lazily create an array filled with **one shared** `CubismTextureColor`, overwrite its
  `r,g,b,a` from `multiplyColors/screenColors[4i..4i+4]` and return it (**QUIRK**, harmless: return by value in Rust).
  `CubismTextureColor(r = 1, g = 1, b = 1, a = 1)` (f64 fields).
* `getDrawableColorBlend(i) = blendModes[i] & 0xff`, `getDrawableAlphaBlend(i) = (blendModes[i] >> 8) & 0xff`, cached with
  sentinel −1. `getDrawableBlendMode(i)` (legacy): additive bit → `CubismBlendMode_Additive (1)`, else multiplicative bit →
  `Multiplicative (2)`, else `Normal (0)`.
* `getDrawableDynamicFlag{IsVisible, VisibilityDidChange, OpacityDidChange, RenderOrderDidChange, BlendColorDidChange,
  VertexPositionsDidChange}(i)`: bit tests on `dynamicFlags[i]`.
* `isUsingMasking()`: any `maskCounts[d] > 0` (true here); `isUsingMaskingForOffscreen()`: any offscreen
  `maskCounts > 0` (false).
* Enums: `CubismColorBlend { None = -1, Normal 0, AddCompatible 1, MultiplyCompatible 2, Add 3, AddGlow 4, Darken 5,
  Multiply 6, ColorBurn 7, LinearBurn 8, Lighten 9, Screen 10, ColorDodge 11, Overlay 12, SoftLight 13, HardLight 14,
  LinearLight 15, Hue 16, Color 17 }`, `CubismAlphaBlend { None = -1, Over 0, Atop 1, Out 2, ConjointOver 3,
  DisjointOver 4 }`, `CubismModelObjectType { Drawable 0, Parts 1 }`, `NoParentIndex = -1`, `NoOffscreenIndex = -1`.

### 7.7 Culling overrides

* `getDrawableCulling(i)`: `if (_isOverriddenCullings || _userDrawableCullings[i].isOverridden) return
  _userDrawableCullings[i].isCulling; return !hasIsDoubleSidedBit(constantFlags[i])`. Offscreen twin uses
  `_userOffscreenCullings` and `offscreens.constantFlags`.
* `setDrawableCulling(i, v)`, `setOverrideFlagForDrawableCullings(i, v)`, `setOffscreenCulling(o, v)`: write the
  **shared** object → affect **every** drawable (resp. offscreen) (**QUIRK**). `get/setOverrideFlagForModelCullings`.
  `DrawableCullingData` is a deprecated alias (`isOverwritten` getter).
* Rust: one `CullingData` for drawables and one for offscreens (faithful and simpler).

### 7.8 Parts hierarchy (`setupPartsHierarchy`, `getPartChildDrawObjects`)

1. `_partsHierarchy = [CubismModelPartInfo{ objects: [], childDrawObjects: {drawableIndices: [], offscreenIndices: []} }; partCount]`.
2. For each part `i` (ascending) with `parentIndices[i] != -1`: push `{objectIndex: i, objectType: Parts}` onto the
   parent's `objects`.
3. For each drawable `i` (ascending) with parent part `!= -1`: push `{i, Drawable}` onto that part's `objects`.
4. For each part `i`: `getPartChildDrawObjects(i)`: no objects → return the empty lists; lists already non-empty → return
   them; else for each object in order: Parts → recurse, append the child's `drawableIndices` and `offscreenIndices`, then
   append `parts.offscreenIndices[child]` if `!= -1`; Drawable → append its index.

This model (from `golden/model_info.json`): part 4 (`Part11`) objects `[Part 13, D26, D27, D43, D44]` → child drawables
`[28,29,30,31,32,45,26,27,43,44]`; part 5 (`Part9`) → `[19,20,21,22,23,46,24,25,47,48]`. Used only by multiply/screen
part propagation and offscreens.

### 7.9 Misc

* `getModelOapcity()` / `setModelOapcity(v)` (typos are the real names): `_modelOpacity`, default 1.0; written by
  `CubismMotion` when a `Model` curve with ID `Opacity` exists; **nothing reads it** in this SDK build (no renderer call
  site) → motion opacity curves have no visible effect. Rust: keep the field.
* `getPixelsPerUnit()`, `getCanvasWidth()`, `getCanvasHeight()`: render spec 4.1 (this model 7000, 1.0,
  1.4285714285714286).
* `getModel()` returns the Core model; `release()` releases the Core model and nulls caches.

---

## 8. `CubismModelMultiplyAndScreenColor` (colour overrides; Tier B — never enabled by V1)

* `ColorData { isOverridden = false, color: CubismTextureColor = (1,1,1,1) }`.
* Constructor: model-wide flags `_isOverriddenModelMultiplyColors = false`, `_isOverriddenModelScreenColors = false`; six
  empty arrays.
* `initialize(partCount, drawableCount, offscreenCount)`: for every part, drawable and offscreen create **distinct**
  `ColorData(false, (1,1,1,1))` (multiply) and `ColorData(false, (0,0,0,1))` (screen).
* Index validation helpers `isValidPartIndex / isValidDrawableIndex / isValidOffscreenIndex(index, fnName)`: if
  `index < 0 || index >= count` → `CubismLogWarning(`${fnName}: index is out of range. index=${index}, valid range=[0,
  ${count - 1}].`)`, return false. Every public method validates first and passes its own name (e.g.
  `'setPartMultiplyColorEnabled'`, `'getDrawableScreenColor'`).

| Method | Behaviour (invalid index → warning + listed default) |
|---|---|
| `set/getMultiplyColorEnabled(v)`, `set/getScreenColorEnabled(v)` | model-wide flags |
| `setPartMultiplyColorEnabled(p, v)` / `setPartScreenColorEnabled(p, v)` | `setPartColorEnabled(p, v, partMul, drawMul, offMul)` (resp. screen arrays) |
| `getPartMultiplyColorEnabled(p)` / `getPartScreenColorEnabled(p)` | `parts[p].isOverridden` (invalid → false) |
| `setPartMultiplyColorByTextureColor(p, c)` / `…ByRGBA(p, r, g, b, a = 1.0)` | `setPartColor(p, r, g, b, a, partMul, drawMul, offMul)`; screen twins likewise |
| `getPartMultiplyColor(p)` / `getPartScreenColor(p)` | `parts[p].color` (invalid → new (1,1,1,1) / (0,0,0,1)) |
| `setDrawableMultiplyColorEnabled(d, v)` / screen | `drawMul[d].isOverridden = v` |
| `getDrawableMultiplyColorEnabled(d)` / screen | flag (invalid → false) |
| `setDrawableMultiplyColorByTextureColor(d, c)` / `…ByRGBA(d, r, g, b, a = 1.0)` / screen | write the colour fields |
| `getDrawableMultiplyColor(d)` | invalid → (1,1,1,1); `if (modelWideMultiply \|\| drawMul[d].isOverridden) return drawMul[d].color; return model.getDrawableMultiplyColor(d)` (Core) |
| `getDrawableScreenColor(d)` | same with screen arrays, default (0,0,0,1) |
| Offscreen family (`set/getOffscreen{Multiply,Screen}Color{Enabled,ByTextureColor,ByRGBA}`, `getOffscreen{Multiply,Screen}Color`) | identical pattern over offscreen arrays and `model.getOffscreen*Color` |

Propagation (private):
```
setPartColor(p, r,g,b,a, parts, draws, offs):
  parts[p].color = (r,g,b,a)
  if parts[p].isOverridden:
     o = model.getPartOffscreenIndices()[p]
     if o == -1:
        for obj in partsHierarchy[p].objects (in order):
           Drawable -> draws[obj.index].color = (r,g,b,a)
           Part     -> setPartColor(obj.index, r,g,b,a, ...)      // recursion only recolours overridden children's subtrees
     else: offs[o].color = (r,g,b,a)

setPartColorEnabled(p, v, parts, draws, offs):
  parts[p].isOverridden = v
  o = partOffscreenIndices[p]
  if o == -1:
     for obj in partsHierarchy[p].objects:
        Drawable -> draws[d].isOverridden = v; if v: draws[d].color = parts[p].color
        Part     -> if v: parts[child].color = parts[p].color;  setPartColorEnabled(child, v, ...)
  else: offs[o].isOverridden = v; if v: offs[o].color = parts[p].color
```

---

## 9. `CubismModelUserData` (userdata3.json; Tier B — this model has none)

* JSON keys: `Meta`, `UserDataCount`, `TotalUserDataSize`, `UserData`, `Target`, `Id`, `Value`; constant `ArtMesh = 'ArtMesh'`.
* `create(buffer, size)` → `parseUserData`: `json = new CubismModelUserDataJson(buffer, size)`; `typeOfArtMesh =
  getId('ArtMesh')`; `nodeCount = root.Meta.UserDataCount.toInt()`; for `i < nodeCount`: node `{ targetId =
  getId(UserData[i].Id raw), targetType = getId(UserData[i].Target raw), value = UserData[i].Value raw }`; append to
  `_userDataNodes`; if `targetType == ArtMesh` also append to `_artMeshUserDataNode`.
* `getArtMeshUserDatas()` returns the ArtMesh list. `getTotalUserDataSize()` exists but is unused.

---

## 10. `CubismUserModel` and the V1 `LAppModel` glue

### 10.1 `CubismUserModel` (base)

Constructor: `_moc = _model = null`; `_motionManager = new CubismMotionManager()` with
`setEventCallback(CubismUserModel.cubismDefaultMotionEventCallback, this)`; `_expressionManager = new
CubismExpressionMotionManager()`; `_dragManager = new CubismTargetPoint()`; `_eyeBlink = _breath = _modelMatrix = _pose
= _physics = _modelUserData = null`; `_initialized = false`; `_updating = false`; `_opacity = 1.0`; `_mocConsistency =
false`; `_debugMode = false`; `_renderer = null`. (`_motionConsistency` is declared but **not** initialised →
`undefined` → default parameter `false` when passed on.)

| Method | Behaviour |
|---|---|
| `loadModel(buffer, shouldCheckMocConsistency = false)` | `_moc = CubismMoc.create(buffer, check)`; null → `CubismLogError('Failed to CubismMoc.create().')`, return. `_model = _moc.createModel()`; null → `CubismLogError('Failed to CreateModel().')`, return. `_model.saveParameters()`; `_modelMatrix = new CubismModelMatrix(_model.getCanvasWidth(), _model.getCanvasHeight())` (render spec 3.4) |
| `loadMotion(buffer, size, name, onFinished?, onBegan?, modelSetting?, group?, index?, shouldCheck = false)` | `buffer == null \|\| size == 0` → `CubismLogError('Failed to loadMotion().')`, null. `motion = CubismMotion.create(buffer, size, onFinished, onBegan, shouldCheck)`; null → ``CubismLogError(`Failed to create motion from buffer in LoadMotion()`)``, null. If `modelSetting`: `fi = getMotionFadeInTimeValue(group, index)`; `fi >= 0` → `setFadeInTime(fi)`; same for fade-out. Return motion. (`name` unused) |
| `loadExpression(buffer, size, name)` | null/0 → `CubismLogError('Failed to loadExpression().')`, null; `CubismExpressionMotion.create(buffer, size)` |
| `loadPose(buffer, size)` | null/0 → `CubismLogError('Failed to loadPose().')`; `_pose = CubismPose.create(buffer, size)` |
| `loadUserData(buffer, size)` | `'Failed to loadUserData().'`; `_modelUserData = CubismModelUserData.create(...)` |
| `loadPhysics(buffer, size)` | `'Failed to loadPhysics().'`; `_physics = CubismPhysics.create(buffer, size)` |
| `isHit(drawableId, x, y)` | `d = _model.getDrawableIndex(id)`; `d < 0` → false. Bounding box over vertices: start `left = right = v[0]`, `top = bottom = v[1]`; for `j = 1..count-1`: `x = v[2j]`, `y = v[2j+1]`; `left = min`, `right = max`, `top = min(y)`, `bottom = max(y)`. `tx = _modelMatrix.invertTransformX(x)`, `ty = invertTransformY(y)`; return `left <= tx && tx <= right && top <= ty && ty <= bottom` |
| `setDragging(x, y)` | `_dragManager.set(x, y)` |
| `getModelMatrix()`, `getModel()`, `setOpacity(a)`/`getOpacity()` (`_opacity`, only used by `hitTest`), `isInitialized/setInitialized`, `isUpdating/setUpdating`, `getMocVersionFromBuffer(bytes)` | trivial |
| `createRenderer(w, h, maskBufferCount = 1)` / `deleteRenderer()` / `getRenderer()` / `setRenderTargetSize(w, h)` | render spec |
| `motionEventFired(value)` | `CubismLogInfo('{0}', value)` (LAppModel overrides: `CubismLogInfo('{0} is fired on LAppModel!!', value)`) |
| `static cubismDefaultMotionEventCallback(caller, value, customData)` | `customData?.motionEventFired(value)` |
| `release()` | release managers, `_moc.deleteModel(_model)`, `_moc.release()`, drop everything, `deleteRenderer()` |

### 10.2 V1 `LAppModel` model setup (bundle 15673–15981) — what is created for this model

`loadAssets(dir = '/resources/models/AmadeusKurisu5.0/reama5.0/', file = 'Live2D紅莉栖forSDK5.0.model3.json')`:
log `[ReamaCubism] Loading model: ${dir}${file}`; cached fetch → log `[ReamaCubism] model3.json loaded (${bytes} bytes)`
→ `new CubismModelSettingJson(ab, ab.byteLength)` → `_state = 1 (LoadModel)` → `setupModel(setting)`; fetch/parse failure
→ `console.error('[ReamaCubism] Failed to load model: ' + url, error)` + CubismLogError.

`setupModel` state machine (each step runs after the previous async step resolves):

| State | Step | This model |
|---|---|---|
| — | prefetch moc, physics, textures (cache warm-up) | |
| 2 WaitLoadModel → 3 | fetch moc → `loadModel(ab, _mocConsistency = false)` | creates `CubismMoc` + `CubismModel` + model matrix |
| 3 LoadExpression / 4 Wait | for each expression: fetch, `loadExpression`, `_expressions.set(name, motion)` (insertion order = **fetch completion order**); when all done → add `CubismExpressionUpdater` | **skipped** (count 0) |
| 5 / 6 | physics: fetch → `loadPhysics` → if non-null add `CubismPhysicsUpdater` | **physics created** |
| 7 / 8 | pose: plain `fetch` (not cached) → `loadPose` → add `CubismPoseUpdater` | skipped |
| 9 | eye blink: if `getEyeBlinkParameterCount() > 0` → `CubismEyeBlink.create(setting)` + `CubismEyeBlinkUpdater(() => this._motionUpdated, eyeBlink)` | **skipped** (0) |
| 10 | breath: always; parameters table 12.1.3; add `CubismBreathUpdater` | **created** |
| 11 / 12 | user data | skipped |
| 13 | `_eyeBlinkIds` = eye-blink parameter IDs from setting | `[]` |
| 14 | `_lipSyncIds` = lip-sync IDs; if non-empty add `CubismLipSyncUpdater(_lipSyncIds, _wavFileHandler)` | `[]`, skipped |
| 15 | look: always; table 12.4.2; add `CubismLookUpdater(look, _dragManager)` | **created** |
| — | `finalizeUpdaters`: `_updateScheduler.sortUpdatableList()` | order Look 400, Breath 500, Physics 600 |
| 16 | layout: `getLayoutMap` → `_modelMatrix.setupFromLayout(map)` (error `'Failed to setupLayout().'` if setting/matrix null) | empty map |
| 17 / 18 | motions: `_model.saveParameters()`; count groups; `preLoadMotionGroup(group)` for each | 0 groups → state 21 immediately: `_motionManager.stopAllMotions()`, `_updating = false`, `_initialized = true`, create renderer, textures, shaders |
| 21 → 22 → 23 | textures loaded → `_state = 23 (CompleteSetup)`; logs `[ReamaCubism] Loading ${n} texture(s)...`, `[ReamaCubism] Loading texture ${i}: ${path}`, `[ReamaCubism] Texture ${i} loaded: ${w}x${h}`, `[ReamaCubism] All textures loaded. Model setup complete!` | |

`preLoadMotionGroup(group)` (for completeness): for each index: plain `fetch` → `loadMotion(ab, len, name =
`${group}_${i}`, null, null, setting, group, i, _motionConsistency)`; success → `setEffectIds(_eyeBlinkIds,
_lipSyncIds)`, replace/insert `_motions[name]`, `_motionCount++`; failure → `_allMotionCount--`; when
`_motionCount >= _allMotionCount` → same completion block as above.

`startMotion(group, no, priority, onFinished?, onBegan?)` (bundle 16057–16119):
```
if priority == PriorityForce(3): motionManager.setReservePriority(priority)
else if !motionManager.reserveMotion(priority): (debug log "[APP]can't start motion.") return -1
name = `${group}_${no}`; motion = _motions.get(name); autoDelete = false
if motion == null:
    fetch(...).then(ab => motion = loadMotion(...))        // async!
    if motion: ...                                          // QUIRK: always false here (fetch not resolved yet)
    else: CubismLogError("Can't start motion {0} .", file); setReservePriority(PriorityNone 0); return -1
else: motion.setBeganMotionHandler(onBegan); motion.setFinishedMotionHandler(onFinished)
voice = getMotionSoundFileName(group, no); if voice != "": _wavFileHandler.start(dir + voice)
return motionManager.startMotionPriority(motion, autoDelete, priority)
```
**QUIRK:** a motion that was not preloaded can never start (the async load result is checked synchronously). Since all
groups are preloaded before CompleteSetup, this only matters if the setting changes. V2: start only preloaded motions
(same observable behaviour).

`startRandomMotion(group, priority, …)`: `getMotionCount(group) == 0` → −1; `no = floor(Math.random() * count)`;
`startMotion(group, no, …)`.

`setExpression(name)`: `motion = _expressions.get(name)`; non-null → `_expressionManager.startMotion(motion, false)`.
`setRandomExpression()`: random index over `_expressions` insertion order. `hitTest(name, x, y)`: `_opacity < 1` → false;
first hit area with that name → `isHit(id, x, y)`. Priorities (lappdefine): `PriorityNone 0`, `PriorityIdle 1`,
`PriorityNormal 2`, `PriorityForce 3`; groups `MotionGroupIdle = 'Idle'`, `MotionGroupTapBody = 'TapBody'`; hit areas
`'Head'`, `'Body'`.

The per-frame `update()` is in 1.2.

---

## 11. Update scheduler and updaters (`FW/motion/icubismupdater.ts`, `cubismupdatescheduler.ts`, `*updater.ts`)

### 11.1 Execution orders (`CubismUpdateOrder`)

`EyeBlink = 200`, `Expression = 300`, `Drag = 400` (look), `Breath = 500`, `Physics = 600`, `LipSync = 700`, `Pose = 800`,
`Max = Number.MAX_SAFE_INTEGER (9007199254740991)`. Each updater's constructor takes an optional `executionOrder`
(`??` default = its constant).

### 11.2 `ICubismUpdater`

* `static sortFunction(l, r)`: null/undefined elements sort last; else `l.getExecutionOrder() - r.getExecutionOrder()`.
* `getExecutionOrder()`; `setExecutionOrder(o)`: if changed → notify listeners (`onUpdaterChanged(this)`).
* `addChangeListener(l)` (no duplicates), `removeChangeListener(l)`.

### 11.3 `CubismUpdateScheduler`

* `addUpdatableList(u)`: ignore null and duplicates; push; register itself as change listener; `_needsSort = true`.
* `removeUpdatableList(u)`: splice + unregister; returns found.
* `sortUpdatableList()`: `Array.prototype.sort(sortFunction)` — **stable** (V8 TimSort) → equal orders keep insertion
  order; `_needsSort = false`.
* `onLateUpdate(model, dt)`: `!model` → return; sort if needed; call `updater.onLateUpdate(model, dt)` for each non-null
  entry in order.
* `getUpdatableCount()`, `getUpdatable(i)` (null if out of range), `hasUpdatable(u)`, `clearUpdatableList()`,
  `release()`, `onUpdaterChanged() → _needsSort = true`.

### 11.4 Updaters (`onLateUpdate(model, dt)`; all return early if `!model`)

| Updater | Body |
|---|---|
| `CubismEyeBlinkUpdater(motionUpdated: () => bool, eyeBlink)` | `if (!motionUpdated()) eyeBlink.updateParameters(model, dt)` (blink only while no main motion updated this frame) |
| `CubismExpressionUpdater(expressionManager)` | `expressionManager.updateMotion(model, dt)` |
| `CubismLookUpdater(look, dragManager)` | `dragManager.update(dt)`; `look.updateParameters(model, dragManager.getX(), dragManager.getY())` |
| `CubismBreathUpdater(breath)` | `breath.updateParameters(model, dt)` |
| `CubismPhysicsUpdater(physics)` | `physics.evaluate(model, dt)` |
| `CubismPoseUpdater(pose)` | `pose.updateParameters(model, dt)` |
| `CubismLipSyncUpdater(ids, provider)` | copies `ids`; if provider: `if (provider.update(dt)) { v = provider.getParameter(); for id in ids: model.addParameterValueById(id, v) }`; `set/getAudioProvider` |

`IParameterProvider { update(dt?): boolean; getParameter(): number }` (abstract; V1's implementation is
`LAppWavFileHandler`, unused here).

**Rust:** avoid trait objects holding shared references; the user model owns the components and the scheduler stores
`Vec<(i64 /*order*/, UpdaterKind)>` with `enum UpdaterKind { EyeBlink, Expression, Look, Breath, Physics, LipSync, Pose,
Custom(Box<dyn FnMut(&mut CubismModel, f64)>) }`, sorted with `sort_by_key` (stable), dispatched by `match`.

---

## 12. Effects

### 12.1 Breath (`CubismBreath`, **used**)

12.1.1 Data: `BreathParameterData { parameterId = null, offset = 0, peak = 0, cycle = 0, weight = 0 }` (constructor args
default to these when `undefined`). `CubismBreath { _breathParameters, _currentTime = 0.0 }`, `setParameters(list)`,
`getParameters()`.

12.1.2 `updateParameters(model, dt)`:
```
_currentTime += dt
t = _currentTime * 2.0 * Math.PI
for data in _breathParameters (in order):
    model.addParameterValueById(data.parameterId, data.offset + data.peak * Math.sin(t / data.cycle), data.weight)
    //  => set(id, get(id) + (offset + peak*sin(t/cycle)) * weight)   (clamped, f32 store)
```

12.1.3 V1 parameters (bundle 15834–15854, in this order):

| # | Parameter | offset | peak | cycle [s] | weight |
|---|---|---|---|---|---|
| 1 | `ParamAngleX` | 0 | 15 | 6.5345 | 0.5 |
| 2 | `ParamAngleY` | 0 | 8 | 3.5345 | 0.5 |
| 3 | `ParamAngleZ` | 0 | 10 | 5.5345 | 0.5 |
| 4 | `ParamBodyAngleX` | 0 | 4 | 15.5345 | 0.5 |
| 5 | `ParamBreath` | 0.5 | 0.5 | 3.2345 | 1 |

Check value: first frame at dt = 1/60 → `ParamAngleX = f32(0 + 15·sin(2π·(1/60)/6.5345)·0.5) = 0.12018739432096481`,
`ParamBreath = 0.5161851048469543` (`golden/pipeline_dt60.jsonl` frame 0).

### 12.2 Eye blink (`CubismEyeBlink`, Tier B — not created for this model)

* `EyeState { First 0, Interval 1, Closing 2, Closed 3, Opening 4 }`; `CloseIfZero = true` (static).
* Constructor(setting): `_blinkingState = First`, `_nextBlinkingTime = 0`, `_stateStartTimeSeconds = 0`,
  `_blinkingIntervalSeconds = 4.0`, `_closingSeconds = 0.1`, `_closedSeconds = 0.05`, `_openingSeconds = 0.15`,
  `_userTimeSeconds = 0`, `_parameterIds = []`; if setting non-null → the eye-blink IDs from model3.json.
* Setters: `setBlinkingInterval(s)`, `setBlinkingSetting(closing, closed, opening)`, `setParameterIds(ids)`,
  `getParameterIds()`.
* `determinNextBlinkingTiming()` (sic) `= _userTimeSeconds + Math.random() * (2.0 * _blinkingIntervalSeconds - 1.0)`
  (default: uniform `[0, 7)` s after now).
* `updateParameters(model, dt)`:
```
_userTimeSeconds += dt
switch state:
  Closing:  t = (_userTimeSeconds - _stateStartTimeSeconds) / _closingSeconds
            if t >= 1: t = 1; state = Closed; _stateStartTimeSeconds = _userTimeSeconds
            value = 1 - t
  Closed:   t = (now - start) / _closedSeconds
            if t >= 1: state = Opening; start = now
            value = 0
  Opening:  t = (now - start) / _openingSeconds
            if t >= 1: t = 1; state = Interval; _nextBlinkingTime = determinNextBlinkingTiming()
            value = t
  Interval: if _nextBlinkingTime < now: state = Closing; start = now
            value = 1
  First/default: state = Interval; _nextBlinkingTime = determinNextBlinkingTiming(); value = 1
if !CloseIfZero: value = -value
for id in _parameterIds: model.setParameterValueById(id, value)          // overwrite (weight 1), clamped
```
Golden with a fixed random sequence: `golden/eyeblink.jsonl` (RNG injected; Rust must take an `Rng` trait object so
tests can feed `[0.3, 0.7, 0.1, 0.9, 0.5, 0.05, 0.95, 0.42]`).

### 12.3 Pose (`CubismPose`, Tier B — this model has no pose3.json)

Constants: `Epsilon = 0.001`, `DefaultFadeInSeconds = 0.5`; keys `FadeInTime`, `Link`, `Groups`, `Id`.

`create(buffer, size)`: parse (throws on bad JSON); `ret = new CubismPose()` (`_fadeTimeSeconds = 0.5`, `_lastModel =
null`, `_partGroups = []`, `_partGroupCounts = []`); if `root.FadeInTime` is not null: `_fadeTimeSeconds =
toFloat(0.5)`, and if `< 0` → 0.5. For each `Groups[g]` (array of arrays): for each entry: `PartData { partId =
getId(entry.Id raw), parameterIndex = 0, partIndex = 0, link = [] }`; if `entry.Link` not null: for each link string
(`getString()`): push `PartData{partId: getId(str)}`; push `partData.clone()` to `_partGroups`; `_partGroupCounts[g] =
number of entries`.

`PartData.initialize(model)`: `parameterIndex = model.getParameterIndex(partId)` (**a parameter with the part's ID** —
normally a virtual slot); `partIndex = model.getPartIndex(partId)`; `model.setParameterValueByIndex(parameterIndex, 1)`.

`reset(model)`: for each group with `beginIndex`: for `j` in group: `_partGroups[j].initialize(model)`; if
`partIndex < 0` continue (never); `setPartOpacityByIndex(partIndex, j == beginIndex ? 1 : 0)`;
`setParameterValueByIndex(paramIndex, j == beginIndex ? 1 : 0)`; initialise every link's indices.

`updateParameters(model, dt)`: `if (model != _lastModel) reset(model)`; `_lastModel = model`; `dt < 0 → 0`; for each
group `doFade(model, dt, beginIndex, count)`; then `copyPartOpacities(model)`.

`doFade(model, dt, begin, count)`:
```
visiblePartIndex = -1; newOpacity = 1.0; phi = 0.5; backOpacityThreshold = 0.15
for i in begin..begin+count:
    if model.getParameterValueByIndex(_partGroups[i].parameterIndex) > Epsilon:
        if visiblePartIndex >= 0: break
        visiblePartIndex = i
        if _fadeTimeSeconds == 0: newOpacity = 1.0; continue
        newOpacity = model.getPartOpacityByIndex(_partGroups[i].partIndex) + dt / _fadeTimeSeconds
        if newOpacity > 1.0: newOpacity = 1.0
if visiblePartIndex < 0: visiblePartIndex = 0; newOpacity = 1.0
for i in begin..begin+count:
    p = _partGroups[i].partIndex
    if i == visiblePartIndex: model.setPartOpacityByIndex(p, newOpacity)
    else:
        opacity = model.getPartOpacityByIndex(p)
        a1 = newOpacity < phi ? newOpacity * (phi - 1) / phi + 1.0 : (1 - newOpacity) * phi / (1.0 - phi)
        backOpacity = (1.0 - a1) * (1.0 - newOpacity)
        if backOpacity > backOpacityThreshold: a1 = 1.0 - backOpacityThreshold / (1.0 - newOpacity)
        if opacity > a1: opacity = a1
        model.setPartOpacityByIndex(p, opacity)
```
`copyPartOpacities(model)`: for each part data with links: `opacity = getPartOpacityByIndex(partIndex)`; for each link
with `partIndex >= 0` → `setPartOpacityByIndex(link.partIndex, opacity)`.

### 12.4 Look (`CubismLook`, **run every frame, no-op for V1**)

12.4.1 `LookParameterData { parameterId = null, factorX = 0, factorY = 0, factorXY = 0 }`; `CubismLook {
_lookParameters = [] }`, `setParameters`, `getParameters`; `updateParameters(model, dragX, dragY)`: for each data:
`model.addParameterValueById(id, factorX * dragX + factorY * dragY + factorXY * dragX * dragY)` (weight 1).

12.4.2 V1 parameters (bundle 15911–15932):

| Parameter | factorX | factorY | factorXY |
|---|---|---|---|
| `ParamAngleX` | 30 | 0 | 0 |
| `ParamAngleY` | 0 | 30 | 0 |
| `ParamAngleZ` | 0 | 0 | −30 |
| `ParamBodyAngleX` | 10 | 0 | 0 |
| `ParamEyeBallX` | 1 | 0 | 0 |
| `ParamEyeBallY` | 0 | 1 | 0 |

The drag point is `CubismUserModel._dragManager` (`CubismTargetPoint`, algorithm in render spec 3.7), updated by the
look updater **before** `look.updateParameters`. V1 never sets a target, so `getX() = getY() = 0` and the adds are
`+0.0`. `golden/pipeline_drag.jsonl` exercises a non-zero target `(0.8, −0.5)` for the generic implementation.

---

## 13. Physics (`FW/physics/*`, **used — parity-critical**)

### 13.1 Constants, enums, data

* Tags: `PhysicsTypeTagX = 'X'`, `PhysicsTypeTagY = 'Y'`, `PhysicsTypeTagAngle = 'Angle'`.
* `AirResistance = 5.0`, `MaximumWeight = 100.0`, `MovementThreshold = 0.001`, `MaxDeltaTime = 5.0`.
* `CubismPhysicsTargetType { Parameter = 0 }`; `CubismPhysicsSource { X = 0, Y = 1, Angle = 2 }`.
* `CubismPhysicsNormalization { minimum, maximum, defalut }` (sic, field name `defalut`).
* `CubismPhysicsParticle { initialPosition, mobility, delay, acceleration, radius, position, lastPosition, lastGravity,
  force, velocity }` (vectors f64).
* `CubismPhysicsSubRig { inputCount, outputCount, particleCount, baseInputIndex, baseOutputIndex, baseParticleIndex,
  normalizationPosition, normalizationAngle }`.
* `CubismPhysicsInput { source: {id, targetType}, sourceParameterIndex, weight, type, reflect,
  getNormalizedParameterValue }`.
* `CubismPhysicsOutput { destination: {id, targetType}, destinationParameterIndex, vertexIndex, translationScale,
  angleScale, weight, type, reflect, valueBelowMinimum, valueExceededMaximum, getValue, getScale }`.
* `CubismPhysicsRig { subRigCount, settings[], inputs[], outputs[], particles[], gravity, wind, fps = 0.0 }`.
* `Options { gravity, wind }`; `PhysicsOutput { outputs: number[] }`.
* `CubismPhysics` state: `_physicsRig`, `_options` (constructor: gravity `(0, -1)`, wind `(0, 0)`),
  `_currentRigOutputs: PhysicsOutput[]`, `_previousRigOutputs: PhysicsOutput[]`, `_currentRemainTime = 0.0`,
  `_parameterCaches = null`, `_parameterInputCaches = null`. `setOptions(o)`, `getOption()`.

### 13.2 physics3.json accessors (`CubismPhysicsJson`) — paths and defaults

Missing values use the 4.4 defaults (`toFloat` → 0, `toInt` → 0, `toBoolean` → false, raw string → placeholder).

| Getter | Path | Conversion |
|---|---|---|
| `getGravity()` / `getWind()` | `Meta.EffectiveForces.Gravity.{X,Y}` / `Meta.EffectiveForces.Wind.{X,Y}` | `toFloat()` |
| `getFps()` | `Meta.Fps` | `toFloat(0.0)` |
| `getSubRigCount()` | `Meta.PhysicsSettingCount` | `toInt()` |
| `getTotalInputCount()` / `getTotalOutputCount()` / `getVertexCount()` | `Meta.TotalInputCount` / `Meta.TotalOutputCount` / `Meta.VertexCount` | `toInt()` |
| `getNormalizationPosition{Minimum,Maximum,Default}Value(i)` | `PhysicsSettings[i].Normalization.Position.{Minimum,Maximum,Default}` | `toFloat()` |
| `getNormalizationAngle{…}Value(i)` | `PhysicsSettings[i].Normalization.Angle.{…}` | `toFloat()` |
| `getInputCount(i)` | `PhysicsSettings[i].Input` | `getVector().length` |
| `getInputWeight/Reflect/Type(i, j)` | `…Input[j].Weight` / `.Reflect` / `.Type` | float / bool / raw string |
| `getInputSourceId(i, j)` | `…Input[j].Source.Id` | `getId(raw)` |
| `getOutputCount(i)` | `PhysicsSettings[i].Output` | length |
| `getOutputVertexIndex(i, j)` | `…Output[j].VertexIndex` | `toInt()` |
| `getOutputAngleScale(i, j)` | `…Output[j].Scale` | `toFloat()` |
| `getOutputWeight/Type/Reflect(i, j)`, `getOutputDestinationId(i, j)` | `…Output[j].Weight/Type/Reflect`, `.Destination.Id` | |
| `getParticleCount(i)` | `PhysicsSettings[i].Vertices` | length |
| `getParticleMobility/Delay/Acceleration/Radius(i, j)` | `…Vertices[j].Mobility/Delay/Acceleration/Radius` | `toFloat()` |
| `getParticlePosition(i, j)` | `…Vertices[j].Position.{X,Y}` | `toFloat()` |

Ignored keys: `Version`, `Meta.PhysicsDictionary`, `PhysicsSettings[i].Id`, `Source.Target`, `Destination.Target`.

### 13.3 `create` / `parse`

`CubismPhysics.create(buffer, size)`: `ret = new CubismPhysics(); ret.parse(buffer, size); ret._physicsRig.gravity.y = 0;`
return ret. (The rig's gravity/wind are **never used** by the simulation — `_options` are.)

`parse`:
1. `rig.gravity = json.getGravity()`, `rig.wind = json.getWind()`, `rig.subRigCount = getSubRigCount()`,
   `rig.fps = getFps()`.
2. Allocate `settings[subRigCount]`, `inputs[TotalInputCount]`, `outputs[TotalOutputCount]`, `particles[VertexCount]` with
   `updateSize(..., Class, true)` → **plain objects** (4.5): settings `{normalizationPosition: {}, normalizationAngle: {}}`,
   inputs `{source: {}}`, outputs `{destination: {}, translationScale: {x: 0, y: 0}}`, particles with six `{x: 0, y: 0}`.
   **QUIRK:** `translationScale` is never assigned afterwards, so it stays `(0, 0)` forever (identical in the native SDK),
   and `valueBelowMinimum/valueExceededMaximum` stay `undefined`.
   **QUIRK/RISK:** if the Meta totals are smaller than the actual arrays, `inputs[k]` is `undefined` and parsing throws →
   in V1 the physics promise rejects and setup never reaches CompleteSetup (no avatar). Rust: size arrays from the actual
   JSON arrays and log a warning when Meta disagrees (declared safe deviation; this model's Meta is consistent).
3. `_currentRigOutputs`/`_previousRigOutputs` cleared, then one `PhysicsOutput` per setting.
4. For each setting `i` (running `inputIndex`, `outputIndex`, `particleIndex` from 0):
   * normalization min/max/default (position, angle);
   * `inputCount`, `baseInputIndex = inputIndex`; for each input `j`: `sourceParameterIndex = -1`, `weight`, `reflect`;
     `Type == 'X'` → type X + getter `getInputTranslationXFromNormalizedParameterValue`; `'Y'` → Y + `…Y…`; `'Angle'` →
     Angle + `getInputAngleFromNormalizedParameterValue`; any other string leaves type/getter `undefined` (evaluating such
     an input throws `TypeError`; Rust: skip the input with a warning); `source.targetType = Parameter`, `source.id`;
   * `outputCount`, `baseOutputIndex`; current/previous `outputs` arrays of length `outputCount` filled with `0.0`; for each
     output `j`: `destinationParameterIndex = -1`, `vertexIndex`, `angleScale = Scale`, `weight`, destination id; type
     X/Y/Angle with `getValue` = `getOutputTranslationX` / `getOutputTranslationY` / `getOutputAngle` and `getScale` =
     `getOutputScaleTranslationX` / `…Y` / `getOutputScaleAngle`; `reflect`;
   * `particleCount`, `baseParticleIndex`; per particle `mobility`, `delay`, `acceleration`, `radius`, `position` (from
     JSON).
5. `initialize()`.

### 13.4 `initialize()`

For each setting, `strand = particles[baseParticleIndex..]`:
* `strand[0]`: `initialPosition = (0, 0)`, `lastPosition = (0, 0)`, `lastGravity = (0, -1)` then `lastGravity.y *= -1`
  → `(0, 1)`, `velocity = (0, 0)`, `force = (0, 0)`. (`strand[0].position` keeps the JSON value; it is overwritten by the
  first update.)
* `i = 1..particleCount-1`: `initialPosition = strand[i-1].initialPosition + (0, strand[i].radius)`;
  `position = lastPosition = initialPosition`; `lastGravity = (0, 1)`; `velocity = force = (0, 0)`. (JSON positions of
  `i >= 1` are ignored.)

This model after `initialize` (`golden/physics_rig.jsonl`): settings 1 and 2: `(0,0)`, `(0,15)`; setting 3: `(0,0)`,
`(0,10)`, `(0,20)`, `(0,28)`.

### 13.5 Input normalisation

```
normalizeParameterValue(value, pMin, pMax, pDefault /*UNUSED*/, nMin, nMax, nDefault, isInverted):
    result = 0.0
    maxValue = CubismMath.max(pMax, pMin); if (maxValue < value) value = maxValue
    minValue = CubismMath.min(pMax, pMin); if (minValue > value) value = minValue
    minNorm = CubismMath.min(nMin, nMax); maxNorm = CubismMath.max(nMin, nMax); middleNorm = nDefault
    middleValue = getDefaultValue(minValue, maxValue)        // = min + |max - min| / 2  (midpoint of the RANGE, not pDefault)
    paramValue = value - middleValue
    switch sign(paramValue):                                 // sign: >0 → 1, <0 → -1, else 0 (NaN → 0)
      case 1:  nLength = maxNorm - middleNorm; pLength = maxValue - middleValue
               if (pLength != 0.0) result = paramValue * (nLength / pLength) + middleNorm
      case -1: nLength = minNorm - middleNorm; pLength = minValue - middleValue
               if (pLength != 0.0) result = paramValue * (nLength / pLength) + middleNorm
      case 0:  result = middleNorm
    return isInverted ? result : result * -1.0              // NOT reflected ⇒ NEGATED
```
`getRangeValue(a, b) = |max(a,b) − min(a,b)|`, `getDefaultValue(a, b) = min(a,b) + getRangeValue(a,b) / 2.0`.
**QUIRK:** the parameter default is ignored (midpoint used).

Input getters (weight = `input.weight / 100`):
* X: `totalTranslation.x += normalize(v, min, max, def, normPos.minimum, normPos.maximum, normPos.defalut, reflect) * weight`
* Y: `totalTranslation.y += normalize(… normPos …) * weight`
* Angle: `totalAngle += normalize(v, min, max, def, normAngle.minimum, normAngle.maximum, normAngle.defalut, reflect) * weight`

Virtual (non-existent) source parameters: `model.getParameterIndex` returns an index ≥ count, all reads are `undefined`,
`middleValue` becomes `NaN`, `sign(NaN) = 0` → the input contributes `(reflect ? nDefault : −nDefault) * weight`. Rust:
model the slot as `ParamSlot::{Real(usize), Virtual(usize)}` and return that constant for `Virtual`.

### 13.6 `evaluate(model, deltaTimeSeconds)` — the per-frame entry point

```
if (0.0 >= dt) return                                  // dt <= 0 → no simulation AND no interpolate (QUIRK, 1.2 #3)
values, mins, maxs, defs = Core views of the model's parameters
_currentRemainTime += dt
if (_currentRemainTime > MaxDeltaTime) _currentRemainTime = 0.0          // stall > 5 s: drop accumulated time
if (_parameterCaches?.length ?? 0) < paramCount: _parameterCaches = new Float32Array(paramCount)          // zeros
if (_parameterInputCaches?.length ?? 0) < paramCount:
    _parameterInputCaches = new Float32Array(paramCount); copy values → _parameterInputCaches      // first frame only
physicsDeltaTime = (rig.fps > 0.0) ? 1.0 / rig.fps : dt
while (_currentRemainTime >= physicsDeltaTime):
    for each setting s, i < s.outputCount: previous[s].outputs[i] = current[s].outputs[i]
    inputWeight = physicsDeltaTime / _currentRemainTime
    for j < paramCount:
        _parameterCaches[j] = f32(_parameterInputCaches[j] * (1.0 - inputWeight) + values[j] * inputWeight)
        _parameterInputCaches[j] = _parameterCaches[j]
    for each setting s (index si):
        totalAngle = 0.0; totalTranslation = (0.0, 0.0)
        for each input in s (inputs[s.baseInputIndex .. + s.inputCount]):
            if input.sourceParameterIndex == -1: input.sourceParameterIndex = model.getParameterIndex(input.source.id)  // cached forever
            k = input.sourceParameterIndex
            input.getter(totalTranslation, totalAngle, _parameterCaches[k], mins[k], maxs[k], defs[k],
                         s.normalizationPosition, s.normalizationAngle, input.reflect, input.weight / 100)
        radAngle = degreesToRadian(-totalAngle)
        totalTranslation.x = totalTranslation.x * cos(radAngle) - totalTranslation.y * sin(radAngle)
        totalTranslation.y = totalTranslation.x * sin(radAngle) + totalTranslation.y * cos(radAngle)   // QUIRK: uses the NEW x
        updateParticles(particles[s.baseParticleIndex ..], s.particleCount, totalTranslation, totalAngle,
                        _options.wind, MovementThreshold * s.normalizationPosition.maximum, physicsDeltaTime, AirResistance)
        for each output (index i) in s:
            p = output.vertexIndex
            if output.destinationParameterIndex == -1: output.destinationParameterIndex = model.getParameterIndex(output.destination.id)
            if (p < 1 || p >= s.particleCount) continue
            translation = (particles[p].position.x - particles[p-1].position.x, particles[p].position.y - particles[p-1].position.y)
            outputValue = output.getValue(translation, particles(of s), p, output.reflect, _options.gravity)
            current[si].outputs[i] = outputValue                                       // f64
            d = output.destinationParameterIndex
            tmp = _parameterCaches.slice(d)                                            // Float32Array copy
            updateOutputParameterValue(tmp, mins[d], maxs[d], outputValue, output)     // writes tmp[0]
            _parameterCaches[d .. end] = tmp                                           // net: _parameterCaches[d] = tmp[0]
    _currentRemainTime -= physicsDeltaTime
alpha = _currentRemainTime / physicsDeltaTime
interpolate(model, alpha)
```
Notes:
* `_parameterCaches` propagates values **between settings within one step** (a later setting reading an earlier setting's
  output parameter sees the freshly simulated value). Not used by this model.
* The input-cache interpolation means the inputs fed to each step are a linear blend between the previous step's inputs
  and the current frame's parameter values.
* Virtual destination slots: `slice(d)` is empty → no effect anywhere.

### 13.7 `interpolate(model, weight)` and `updateOutputParameterValue`

```
interpolate(model, weight):
    for each setting s (index si):
        for each output (index i) in s:
            if output.destinationParameterIndex == -1: continue          // no step has ever run
            d = output.destinationParameterIndex
            tmp = values.slice(d)                                        // Float32Array copy of the Core tail
            updateOutputParameterValue(tmp, mins[d], maxs[d],
                previous[si].outputs[i] * (1 - weight) + current[si].outputs[i] * weight, output)
            values[d .. end] = tmp                                        // net: values[d] = f32(tmp[0])

updateOutputParameterValue(paramArr, pMin, pMax, translation, output):
    outputScale = output.getScale(output.translationScale, output.angleScale)
        //  X → translationScale.x (= 0 ALWAYS), Y → translationScale.y (= 0), Angle → angleScale ("Scale")
        //  (implemented as JSON.parse(JSON.stringify(v)): identity for finite numbers)
    value = translation * outputScale
    if (value < pMin) { if (value < output.valueBelowMinimum) output.valueBelowMinimum = value; value = pMin }   // stat never updates (undefined)
    else if (value > pMax) { if (value > output.valueExceededMaximum) output.valueExceededMaximum = value; value = pMax }
    weight = output.weight / 100
    if (weight >= 1.0) paramArr[0] = value
    else paramArr[0] = paramArr[0] * (1.0 - weight) + value * weight          // f32 read, f64 math, f32 store
```
**QUIRK:** X/Y outputs always write `clamp(0)` blended by weight (translation scale never parsed). This model only has
Angle outputs.

Timing consequences (measured, `golden/pipeline_*.jsonl`):
* dt = 1/60 exactly: `_currentRemainTime` becomes exactly 0 after every frame → one step per frame, `alpha = 0` → the
  parameter gets the **previous** step's output (frame N shows step N−1). Frame 0 shows 0 (previous = initial 0).
* dt = 1/144: steps happen on 2 of every 5 frames; `alpha = remain·60` ∈ [0,1) blends previous/current.
* V1 dt pattern (16/17 ms): `remain` hovers around 0.016; when `remain < 1/60` there is no step that frame and the output
  is a blend with `alpha ≈ 0.96–0.99`.
* First frame with `dt < 1/60` (e.g. 0.016): no step, `destinationParameterIndex` still −1 → interpolate skips → hair
  stays at the restored default 0.
* `dt == 0` → early return → hair stays at the restored default for that frame.
* `dt = 6.0` → `remain = 6.016…` > 5 → reset to 0 → no step → interpolate with `alpha = 0` (previous outputs).

### 13.8 `updateParticles(strand, count, totalTranslation, totalAngle, wind, threshold, dt, airResistance)`

```
strand[0].position = copy(totalTranslation)
totalRadian = degreesToRadian(totalAngle)
currentGravity = radianToDirection(totalRadian)          // (sin(totalRadian), cos(totalRadian))
currentGravity.normalize()                                // x /= sqrt(x²+y²), y /= …
for i in 1 .. count-1:
    p = strand[i]; q = strand[i-1]
    p.force = currentGravity * p.acceleration + wind
    p.lastPosition = copy(p.position)
    delay = p.delay * dt * 30.0
    direction = p.position - q.position
    radian = directionToRadian(p.lastGravity, currentGravity) / airResistance
    direction.x = cos(radian) * direction.x - direction.y * sin(radian)
    direction.y = sin(radian) * direction.x + direction.y * cos(radian)    // QUIRK: NEW direction.x
    p.position = q.position + direction
    velocity = p.velocity * delay
    force = p.force * delay * delay
    p.position = p.position + velocity + force
    newDirection = p.position - q.position
    newDirection.normalize()                               // zero length → NaN (propagates)
    p.position = q.position + newDirection * p.radius
    if |p.position.x| < threshold: p.position.x = 0.0
    if delay != 0.0: p.velocity = (p.position - p.lastPosition) / delay * p.mobility   // component-wise
    p.force = (0.0, 0.0)
    p.lastGravity = copy(currentGravity)
```
`threshold = 0.001 * normalizationPosition.maximum` (= 0.01 here). All vector ops are component-wise and allocate new
vectors (no aliasing issues in the TS); evaluate them in exactly this order in Rust.

### 13.9 Output value getters

* `getOutputTranslationX(translation, particles, i, inverted, parentGravity)` = `inverted ? -translation.x :
  translation.x`; `getOutputTranslationY` likewise with `y`.
* `getOutputAngle(translation, particles, i, inverted, parentGravity)`:
  `parentGravity = (i >= 2) ? particles[i-1].position − particles[i-2].position : parentGravity * −1.0` (default options
  → `(−0, 1)`); `v = directionToRadian(parentGravity, translation)`; `inverted ? −v : v`. Result is in **radians**
  (then multiplied by `Scale`).
* `directionToRadian(from, to)`: `q1 = atan2(to.y, to.x)`, `q2 = atan2(from.y, from.x)`, `ret = q1 − q2`;
  `while (ret < −π) ret += 2π; while (ret > π) ret −= 2π`.

### 13.10 `stabilization(model)` (Tier B; never called by V1)

```
values/mins/maxs/defs views
(re)allocate _parameterCaches / _parameterInputCaches if shorter than paramCount (no copy here)
for j < paramCount: _parameterCaches[j] = values[j]; _parameterInputCaches[j] = values[j]
for each setting s (index si):
    totalAngle = 0; totalTranslation = (0, 0)
    for each input: resolve index k (cache); getter(totalTranslation, totalAngle, values[k], mins[k], maxs[k], defs[k], …, weight/100)
                    _parameterCaches[k] = values[k]
    rotate totalTranslation exactly as in evaluate (same QUIRK)
    updateParticlesForStabilization(particles, count, totalTranslation, totalAngle, _options.wind, 0.001 * normPos.maximum)
    for each output (index i): resolve d (before the range check); if (p < 1 || p >= count) continue
        translation = particles[p].position − particles[p-1].position
        v = output.getValue(translation, particles, p, reflect, _options.gravity)
        current[si].outputs[i] = v; previous[si].outputs[i] = v
        tmp = values.slice(d); updateOutputParameterValue(tmp, mins[d], maxs[d], v, output)
        for offset = d .. _parameterCaches.length-1: values[offset] = _parameterCaches[offset] = tmp[offset - d]

updateParticlesForStabilization(strand, count, totalTranslation, totalAngle, wind, threshold):
    strand[0].position = copy(totalTranslation)
    currentGravity = normalize(radianToDirection(degreesToRadian(totalAngle)))
    for i in 1 .. count-1:
        p.force = currentGravity * p.acceleration + wind
        p.lastPosition = copy(p.position)
        p.velocity = (0, 0)
        force = p.force; force.normalize()                 // normalises p.force in place (same object); zero → NaN
        force = force * p.radius
        p.position = strand[i-1].position + force
        if |p.position.x| < threshold: p.position.x = 0
        p.force = (0, 0); p.lastGravity = copy(currentGravity)
```

### 13.11 Other members

`delete(physics)` → `release()` (drops the rig); `getOption()`, `setOptions()`. `CubismPhysics.release()` sets
`_physicsRig = null`.

### 13.12 This model's rig (`Live2D紅莉栖forSDK5.0.physics3.json`, 5,062 bytes)

Meta: `PhysicsSettingCount 3`, `TotalInputCount 12`, `TotalOutputCount 3`, `VertexCount 8`, **`Fps 60`**,
`EffectiveForces.Gravity (0, −1)`, `Wind (0, 0)` (unused), dictionary names: `PhysicsSetting1` = `前髪揺れ` (front hair
sway), `PhysicsSetting2` = `横髪揺れ` (side hair sway), `PhysicsSetting3` = `後髪揺れ` (back hair sway).

All three settings have the same inputs and normalisation:

| Input | Source | Type | Weight | Reflect | Source range |
|---|---|---|---|---|---|
| 1 | `ParamAngleX` | X | 60 | false | [−30, 30] |
| 2 | `ParamAngleZ` | Angle | 60 | false | [−30, 30] |
| 3 | `ParamBodyAngleX` | X | 40 | false | [−10, 10] |
| 4 | `ParamBodyAngleZ` | Angle | 40 | false | [−10, 10] |

Normalisation (each setting): Position `{Minimum −10, Default 0, Maximum 10}`, Angle `{Minimum −10, Default 0,
Maximum 10}` → threshold 0.01.

| Setting | Output | Type | VertexIndex | Scale | Weight | Particles (Mobility, Delay, Acceleration, Radius) |
|---|---|---|---|---|---|---|
| 1 | `ParamHairFront` [−1, 1] | Angle | 1 | 1 | 100 | (1, 1, 1, 0), (0.95, 0.8, 1.5, 15) |
| 2 | `ParamHairSide` [−1, 1] | Angle | 1 | 1 | 100 | (1, 1, 1, 0), (0.95, 0.8, 1.5, 15) |
| 3 | `ParamHairBack` [−1, 1] | Angle | 1 | 1 | 100 | (1, 1, 1, 0), (0.85, 0.9, 1, 10), (0.9, 0.9, 1, 10), (0.9, 0.9, 0.8, 8) |

Settings 1 and 2 are identical except for the destination → `ParamHairFront == ParamHairSide` every frame (verified).
Setting 3 simulates 4 particles but outputs only the angle of segment 0→1. In V1 the inputs are only the breath values
(12.1.3), so hair sways by about ±0.02 (parameter units) — e.g. 60-fps frame 59: front `0.015059679746627808`, back
`0.002047800924628973`.

### 13.13 Rust shape

```rust
pub struct CubismPhysics {
    rig: PhysicsRig,                       // settings/inputs/outputs/particles as Vec<…>, fps: f64
    options: PhysicsOptions,               // gravity (0,-1), wind (0,0)
    current_rig_outputs: Vec<Vec<f64>>,    // per setting
    previous_rig_outputs: Vec<Vec<f64>>,
    current_remain_time: f64,
    parameter_caches: Option<Vec<f32>>,    // None == JS null
    parameter_input_caches: Option<Vec<f32>>,
}
struct PhysicsInput  { source_id: CubismId, source_slot: Option<ParamSlot>, weight: f64, kind: PhysicsSource, reflect: bool }
struct PhysicsOutput { dest_id: CubismId, dest_slot: Option<ParamSlot>, vertex_index: i64, angle_scale: f64,
                       translation_scale: Vec2 /* always (0,0) */, weight: f64, kind: PhysicsSource, reflect: bool }
impl CubismPhysics {
    pub fn from_json(bytes: &[u8]) -> Result<Self, PhysicsError>;
    pub fn evaluate(&mut self, model: &mut CubismModel, dt: f64);
    pub fn interpolate(&mut self, model: &mut CubismModel, weight: f64);
    pub fn stabilization(&mut self, model: &mut CubismModel);
    pub fn set_options(&mut self, o: PhysicsOptions); pub fn options(&self) -> &PhysicsOptions;
}
```
`model` access: `param_values_mut() -> &mut [f32]`, `param_min/max/default(i) -> f32`, `parameter_index(id) -> ParamSlot`.
Compare against `golden/pipeline_dt60.jsonl`, `pipeline_dt144.jsonl`, `pipeline_dt30.jsonl`, `pipeline_jitter.jsonl`,
`pipeline_drag.jsonl` (fields `cur`/`prev` = rig outputs, `remain`, `params`).

---

## 14. Motion system (Tier B — no motions are referenced by this model's model3.json)

### 14.1 Data model (`cubismmotioninternal.ts`)

* `CubismMotionCurveTarget { Model = 0, Parameter = 1, PartOpacity = 2 }`.
* `CubismMotionSegmentType { Linear = 0, Bezier = 1, Stepped = 2, InverseStepped = 3 }`.
* `CubismMotionPoint { time = 0.0, value = 0.0 }` (f64).
* `CubismMotionSegment { evaluate = null, basePointIndex = 0, segmentType = 0 }`.
* `CubismMotionCurve { type = Model, id, segmentCount = 0, baseSegmentIndex = 0, fadeInTime = 0.0, fadeOutTime = 0.0 }`.
* `CubismMotionEvent { fireTime = 0.0, value }`.
* `CubismMotionData { duration = 0, loop = false, curveCount = 0, eventCount = 0, fps = 0, curves[], segments[],
  points[], events[] }`.

### 14.2 motion3.json (`CubismMotionJson`)

Keys: `Meta`, `Duration`, `Loop`, `AreBeziersRestricted`, `CurveCount`, `Fps`, `TotalSegmentCount`, `TotalPointCount`,
`Curves`, `Target`, `Id`, `FadeInTime`, `FadeOutTime`, `Segments`, `UserData`, `UserDataCount`, `TotalUserDataSize`,
`Time`, `Value`.

| Getter | Path → conversion |
|---|---|
| `getMotionDuration()` | `Meta.Duration` → `toFloat()` |
| `isMotionLoop()` | `Meta.Loop` → `toBoolean()` (**parsed but never used**, see 14.3) |
| `getEvaluationOptionFlag(AreBeziersRistricted = 0)` | `Meta.AreBeziersRestricted` → `toBoolean()`; any other flag → false |
| `getMotionCurveCount()` / `getMotionTotalSegmentCount()` / `getMotionTotalPointCount()` | `Meta.CurveCount` / `TotalSegmentCount` / `TotalPointCount` → `toInt()` |
| `getMotionFps()` | `Meta.Fps` → `toFloat()` |
| `isExistMotionFadeInTime()` / `isExistMotionFadeOutTime()` | `!Meta.FadeInTime.isNull()` / `!Meta.FadeOutTime.isNull()` |
| `getMotionFadeInTime()` / `getMotionFadeOutTime()` | `toFloat()` |
| `getMotionCurveTarget(c)` | `Curves[c].Target` raw |
| `getMotionCurveId(c)` | `getId(Curves[c].Id raw)` |
| `isExistMotionCurveFadeInTime(c)` / `…FadeOutTime(c)` | `!Curves[c].FadeInTime.isNull()` / … |
| `getMotionCurveFadeInTime(c)` / `…FadeOutTime(c)` | `toFloat()` |
| `getMotionCurveSegmentCount(c)` | `Curves[c].Segments.getVector().length` (number of **floats**) |
| `getMotionCurveSegment(c, k)` | `Curves[c].Segments[k].toFloat()` (out of range → error value → 0) |
| `getEventCount()` / `getTotalEventValueSize()` | `Meta.UserDataCount` / `Meta.TotalUserDataSize` → `toInt()` |
| `getEventTime(u)` / `getEventValue(u)` | `UserData[u].Time` → float / `UserData[u].Value` raw |

`Segments` encoding (flat float array): `[t0, v0]` then repeated `[type, …]` with `type 0 Linear: t, v`;
`1 Bezier: c1t, c1v, c2t, c2v, t, v`; `2 Stepped: t, v`; `3 InverseStepped: t, v`.

`hasConsistency()` (Tier C; only when `shouldCheckMotionConsistency`): false if `!_json || !root`; recount curves
(`Curves.length`), segments and points with the walk of 14.3 (first point +1 and position += 2; Linear/Stepped/
InverseStepped +1 point, +3; Bezier +3 points, +7; unknown → `CSM_ASSERT(0)`, **no advance → infinite loop**), then
compare with Meta and log (each mismatch → result false): `'The number of curves does not match the metadata.'`,
`'The number of segment does not match the metadata.'`, `'The number of point does not match the metadata.'`.

### 14.3 `CubismMotion.create` / `parse`

`create(buffer, size, onFinished?, onBegan?, shouldCheck = false)`: `ret = new CubismMotion(); ret.parse(...)`; if
`ret._motionData` → `_sourceFrameRate = fps`, `_loopDurationSeconds = duration`, store callbacks, return ret; else
`null`. **`_isLoop` is not taken from `Meta.Loop`** (comment: the Editor does not export looping motions) → every motion
is non-looping unless `setLoop(true)`.

Constructor defaults (`CubismMotion` over `ACubismMotion`): `_fadeInSeconds = -1`, `_fadeOutSeconds = -1`,
`_weight = 1`, `_offsetSeconds = 0`, `_isLoop = false`, `_isLoopFadeIn = true`, `_previousLoopState = false`,
`_firedEventValues = []`, `_sourceFrameRate = 30`, `_loopDurationSeconds = -1`, `_lastWeight = 0`,
`_motionData = null`, cached model-curve IDs `null`, `_eyeBlinkParameterIds = null`, `_lipSyncParameterIds = null`,
`_modelOpacity = 1`, `_debugMode = false`, `_motionBehavior = MotionBehavior_V2` (`enum MotionBehavior { V1 = 0, V2 = 1
}`); constant `UseOldBeziersCurveMotion = false`; names `EffectNameEyeBlink = 'EyeBlink'`, `EffectNameLipSync =
'LipSync'`, `TargetNameModel = 'Model'`, `TargetNameParameter = 'Parameter'`, `TargetNamePartOpacity = 'PartOpacity'`,
`IdNameOpacity = 'Opacity'`.

`parse(json, size, shouldCheck)`:
1. `json = new CubismMotionJson(...)` (throws on invalid JSON). If checking and `!hasConsistency()` →
   `CubismLogError('Inconsistent motion3.json.')`, return (`_motionData` stays null).
2. `_motionData = new CubismMotionData()`: `duration`, `loop`, `curveCount` (Meta), `fps`, `eventCount`;
   `areBeziersRestricted = getEvaluationOptionFlag(0)`.
3. `_fadeInSeconds = exists(Meta.FadeInTime) ? (value < 0 ? 1.0 : value) : 1.0`; `_fadeOutSeconds` likewise.
4. Allocate `curves[CurveCount]`, `segments[TotalSegmentCount]`, `points[TotalPointCount]`, `events[UserDataCount]` as
   plain objects (4.5). (**RISK:** Meta smaller than data → `TypeError`; Rust sizes from the data — safe deviation.)
5. For each curve `c < curveCount` (running `totalPointCount`, `totalSegmentCount` from 0):
   * `Target`: `'Model'` → Model, `'Parameter'` → Parameter, `'PartOpacity'` → PartOpacity, else
     `CubismLogWarning('Warning : Unable to get segment type from Curve! The number of "CurveCount" may be incorrect!')`
     (type stays **Model**).
   * `id = getId(Id)`, `baseSegmentIndex = totalSegmentCount`, `fadeInTime = exists ? value : -1.0`, `fadeOutTime` same.
   * Walk `Segments` with `pos = 0` while `pos < segmentFloatCount`:
     ```
     if pos == 0: segments[totalSegmentCount].basePointIndex = totalPointCount
                  points[totalPointCount] = (S[0], S[1]); totalPointCount += 1; pos += 2
     else:        segments[totalSegmentCount].basePointIndex = totalPointCount - 1     // shares previous end point
     type = S[pos]
     Linear(0):   segmentType = 0; evaluate = linearEvaluate;   points[tp] = (S[pos+1], S[pos+2]); tp += 1; pos += 3
     Bezier(1):   segmentType = 1; evaluate = (areBeziersRestricted || UseOldBeziersCurveMotion) ? bezierEvaluate
                                                                    : bezierEvaluateCardanoInterpretation
                  points[tp] = (S[pos+1], S[pos+2]); points[tp+1] = (S[pos+3], S[pos+4]); points[tp+2] = (S[pos+5], S[pos+6])
                  tp += 3; pos += 7
     Stepped(2):  segmentType = 2; evaluate = steppedEvaluate;  points[tp] = (S[pos+1], S[pos+2]); tp += 1; pos += 3
     InverseStepped(3): segmentType = 3; evaluate = inverseSteppedEvaluate; same point/advance
     default:     CSM_ASSERT(0)        // QUIRK: pos not advanced → infinite loop. Rust: return Err(UnknownSegmentType)
     ++curves[c].segmentCount; ++totalSegmentCount
     ```
     (`type` is compared with `switch` strict equality, so `1.0` matches `1`.) A curve whose `Segments` has only 2 floats
     reads `S[2]` as 0 (Linear) with point `(0, 0)` (degenerate; Editor never writes it).
6. Events: `events[u] = { fireTime: getEventTime(u), value: getEventValue(u) }` for `u < getEventCount()`.

Model-setting override (`CubismUserModel.loadMotion`): model3.json `FadeInTime/FadeOutTime` per motion entry, applied
only if `>= 0`.

### 14.4 Curve evaluation

```
lerpPoints(a, b, t) = { time: a.time + (b.time - a.time) * t, value: a.value + (b.value - a.value) * t }

linearEvaluate(P, time):    t = (time - P0.time) / (P1.time - P0.time); if (t < 0) t = 0      // no upper clamp
                            return P0.value + (P1.value - P0.value) * t
bezierEvaluate(P, time):    t = (time - P0.time) / (P3.time - P0.time); if (t < 0) t = 0      // "restricted" handles
                            p01 = lerp(P0,P1,t); p12 = lerp(P1,P2,t); p23 = lerp(P2,P3,t)
                            p012 = lerp(p01,p12,t); p123 = lerp(p12,p23,t); return lerp(p012,p123,t).value
bezierEvaluateCardanoInterpretation(P, time):
                            x1 = P0.time; x2 = P3.time; cx1 = P1.time; cx2 = P2.time
                            a = x2 - 3*cx2 + 3*cx1 - x1; b = 3*cx2 - 6*cx1 + 3*x1; c = 3*cx1 - 3*x1; d = x1 - time
                            t = CubismMath.cardanoAlgorithmForBezier(a, b, c, d)            // 16.1
                            (same de Casteljau as above with this t)
steppedEvaluate(P, time) = P0.value;     inverseSteppedEvaluate(P, time) = P1.value
```
`P` is `points.slice(segment.basePointIndex)` (relative indexing; a copy in TS — use a slice reference in Rust).
`bezierEvaluateBinarySearch` exists but is never referenced (dead code, Tier C).

```
evaluateCurve(md, c, time, isCorrection, endTime):
    curve = md.curves[c]; target = -1
    total = curve.baseSegmentIndex + curve.segmentCount; pointPosition = 0
    for i in curve.baseSegmentIndex .. total-1:
        pointPosition = md.segments[i].basePointIndex + (md.segments[i].segmentType == Bezier ? 3 : 1)   // segment's last point
        if md.points[pointPosition].time > time: target = i; break
    if target == -1:                                   // time at/after the last point
        if isCorrection && time < endTime:
            return correctEndPoint(md, total - 1, md.segments[curve.baseSegmentIndex].basePointIndex, pointPosition, time, endTime)
        return md.points[pointPosition].value
    seg = md.segments[target]
    return seg.evaluate(md.points[seg.basePointIndex ..], time)

correctEndPoint(md, segmentIndex, beginIndex, endIndex, time, endTime):    // loop wrap: last point → first value
    P = [ {time: md.points[endIndex].time, value: md.points[endIndex].value},
          {time: endTime,                  value: md.points[beginIndex].value} ]
    switch md.segments[segmentIndex].segmentType:
        Stepped        → P[0].value
        InverseStepped → P[1].value
        Linear, Bezier, default → linearEvaluate(P, time)
```
Golden: `golden/curve_<motion>.json` — `samples[k] = {t = k/60, v: [evaluateCurve(md, c, t, false, duration) per
curve], vCorr: [evaluateCurve(md, c, t, true, duration + 1/fps) per curve]}` for
`haru_normal_04` (all four segment types), `hiyori_m01` (`AreBeziersRestricted: false` → Cardano), `Epsilon_idle_01`,
`haru_g_idle` (Model `Opacity` curve + 20 PartOpacity stepped curves), `mtn_02` (60 fps, Meta fades 0.25).

### 14.5 `ACubismMotion` (abstract base)

| Member | Behaviour |
|---|---|
| `updateParameters(model, entry, userTime)` | if `!entry.isAvailable() \|\| entry.isFinished()` return; `setupMotionQueueEntry(entry, userTime)`; `fw = updateFadeWeight(entry, userTime)`; `doUpdateParameters(model, userTime, fw, entry)`; then `if (entry.getEndTime() > 0 && entry.getEndTime() < userTime) entry.setIsFinished(true)` (no finished callback on this path) |
| `setupMotionQueueEntry(entry, userTime)` | `entry == null \|\| entry.isStarted()` → return; `!isAvailable` → return; `setIsStarted(true)`; `setStartTime(userTime - _offsetSeconds)`; `setFadeInStartTime(userTime)`; `if (entry.getEndTime() < 0) adjustEndTime(entry)`; if `entry._motion._onBeganMotion` → call it with the motion |
| `updateFadeWeight(entry, userTime)` | `entry == null` → `CubismDebug.print(Error, 'motionQueueEntry is null.')`; `fadeIn = _fadeInSeconds == 0 ? 1 : getEasingSine((userTime - entry.getFadeInStartTime()) / _fadeInSeconds)`; `fadeOut = (_fadeOutSeconds == 0 \|\| entry.getEndTime() < 0) ? 1 : getEasingSine((entry.getEndTime() - userTime) / _fadeOutSeconds)`; `w = _weight * fadeIn * fadeOut`; `entry.setState(userTime, w)`; `CSM_ASSERT(0 <= w <= 1)`; return `w` |
| `adjustEndTime(entry)` | `d = getDuration()`; `entry.setEndTime(d <= 0 ? -1 : entry.getStartTime() + d)` |
| `getDuration()` / `getLoopDuration()` | −1 / −1 (overridden) |
| `setFadeInTime/getFadeInTime`, `setFadeOutTime/getFadeOutTime`, `setWeight/getWeight`, `setOffsetTime`, `setLoop/getLoop`, `setLoopFadeIn/getLoopFadeIn` | fields |
| `getFiredEvent(before, now)` | base returns `_firedEventValues` |
| `set/getBeganMotionHandler`, `set/getFinishedMotionHandler` | arrow-function fields storing `_onBeganMotion` / `_onFinishedMotion` |
| `isExistModelOpacity()` → false, `getModelOpacityIndex()` → −1, `getModelOpacityId(i)` → null, `getModelOpacityValue()` → 1.0 | overridden by `CubismMotion` |
| `release()` | `_weight = 0` |
| `static delete(motion)` | `motion.release()` |

### 14.6 `CubismMotion.doUpdateParameters(model, userTime, fadeWeight, entry)` (exact)

```
lazily cache ids: _modelCurveIdEyeBlink = getId('EyeBlink'), _modelCurveIdLipSync = getId('LipSync'), _modelCurveIdOpacity = getId('Opacity')
if _motionBehavior == V2 && _previousLoopState != _isLoop: adjustEndTime(entry); _previousLoopState = _isLoop
timeOffsetSeconds = max(0, userTime - entry.getStartTime())
lipSyncValue = eyeBlinkValue = Number.MAX_VALUE
maxTargetSize = 64; lipSyncFlags = eyeBlinkFlags = 0
if _eyeBlinkParameterIds.length > 64: CubismLogDebug('too many eye blink targets : {0}', n)      // TypeError if setEffectIds never called
if _lipSyncParameterIds.length > 64: CubismLogDebug('too many lip sync targets : {0}', n)
tmpFadeIn  = _fadeInSeconds <= 0 ? 1 : getEasingSine((userTime - entry.getFadeInStartTime()) / _fadeInSeconds)      // note "<=" here
tmpFadeOut = (_fadeOutSeconds <= 0 || entry.getEndTime() < 0) ? 1 : getEasingSine((entry.getEndTime() - userTime) / _fadeOutSeconds)
time = timeOffsetSeconds; duration = _motionData.duration
isCorrection = (_motionBehavior == V2) && _isLoop
if _isLoop:
    if V2: duration += 1.0 / _motionData.fps
    while time > duration: time -= duration
c = 0
// (1) Model curves — ONLY while consecutive curves from index 0 are Model curves
while c < curveCount && curves[c].type == Model:
    value = evaluateCurve(md, c, time, isCorrection, duration)
    if curves[c].id == EyeBlink id: eyeBlinkValue = value
    else if == LipSync id: lipSyncValue = value
    else if == Opacity id: _modelOpacity = value; model.setModelOapcity(getModelOpacityValue())
    c++
// (2) Parameter curves — continue only while consecutive curves are Parameter curves
while c < curveCount && curves[c].type == Parameter:
    parameterIndex = model.getParameterIndex(curves[c].id)          // never -1 (virtual slots)
    if parameterIndex == -1: continue (unreachable; NOTE in TS this `continue` still runs ++c)
    sourceValue = model.getParameterValueByIndex(parameterIndex)
    value = evaluateCurve(md, c, time, isCorrection, duration)
    if eyeBlinkValue != MAX_VALUE: for i < min(len, 64): if _eyeBlinkParameterIds[i] == curves[c].id: value *= eyeBlinkValue; eyeBlinkFlags |= 1 << i; break
    if lipSyncValue  != MAX_VALUE: for i < min(len, 64): if _lipSyncParameterIds[i]  == curves[c].id: value += lipSyncValue;  lipSyncFlags  |= 1 << i; break
    if model.isRepeat(parameterIndex): value = model.getParameterRepeatValue(parameterIndex, value)
    if curves[c].fadeInTime < 0 && curves[c].fadeOutTime < 0:
        v = sourceValue + (value - sourceValue) * fadeWeight
    else:
        fin  = curves[c].fadeInTime < 0 ? tmpFadeIn
             : (curves[c].fadeInTime == 0 ? 1 : getEasingSine((userTime - entry.getFadeInStartTime()) / curves[c].fadeInTime))
        fout = curves[c].fadeOutTime < 0 ? tmpFadeOut
             : ((curves[c].fadeOutTime == 0 || entry.getEndTime() < 0) ? 1 : getEasingSine((entry.getEndTime() - userTime) / curves[c].fadeOutTime))
        v = sourceValue + (value - sourceValue) * (_weight * fin * fout)
    model.setParameterValueByIndex(parameterIndex, v, 1.0)           // clamp, f32 store
    c++
// (3) automatic eye blink / lip sync for targets not driven by a curve
if eyeBlinkValue != MAX_VALUE: for i < min(len, 64):
    sourceValue = model.getParameterValueById(_eyeBlinkParameterIds[i])
    if (eyeBlinkFlags >> i) & 1: continue
    model.setParameterValueById(_eyeBlinkParameterIds[i], sourceValue + (eyeBlinkValue - sourceValue) * fadeWeight)
if lipSyncValue != MAX_VALUE: (same with lip-sync ids/flags/value)
// (4) PartOpacity curves — written to a PARAMETER with the part's ID (usually a virtual slot), unblended
while c < curveCount && curves[c].type == PartOpacity:
    parameterIndex = model.getParameterIndex(curves[c].id)
    value = evaluateCurve(md, c, time, isCorrection, duration)
    model.setParameterValueByIndex(parameterIndex, value)
    c++
if timeOffsetSeconds >= duration:                                   // duration includes the V2 loop extension
    if _isLoop: updateForNextLoop(entry, userTime, time)
    else: if _onFinishedMotion: _onFinishedMotion(this); entry.setIsFinished(true)
_lastWeight = fadeWeight
```
**QUIRKs:** (a) curve groups must appear in the order Model → Parameter → PartOpacity; a curve out of order stops the
corresponding loop and all later curves of other types are skipped (all 93 shipped motions are ordered — verified by
`motions_summary.py`); (b) PartOpacity curves never touch part opacities directly — only a pose (12.3) reads those
virtual parameters; (c) the finished callback fires only on natural end (not when the entry ends by fade-out end time).

`updateForNextLoop(entry, userTime, time)`: V2 (default): `entry.setStartTime(userTime - time)`; if `_isLoopFadeIn` →
`entry.setFadeInStartTime(userTime - time)`; if `_onFinishedMotion != null` → call it (**fires on every loop**). V1:
`setStartTime(userTime)`; if loop-fade-in → `setFadeInStartTime(userTime)`.

Other `CubismMotion` members: `getDuration() = _isLoop ? -1 : _loopDurationSeconds`; `getLoopDuration() =
_loopDurationSeconds`; `set/getMotionBehavior`; `setParameterFadeInTime(id, v)` / `setParameterFadeOutTime(id, v)` /
`getParameterFadeInTime(id)` / `getParameterFadeOutTime(id)` (first curve with that id; getters return −1 if absent);
`setEffectIds(eyeBlinkIds, lipSyncIds)` (stores the arrays by reference); `isExistModelOpacity()` (a Model curve whose
id string equals `'Opacity'`, via `localeCompare == 0`), `getModelOpacityIndex()`, `getModelOpacityId(i)`,
`getModelOpacityValue() = _modelOpacity`, `setDebugMode(b)`, `release()` (drops data).

`getFiredEvent(before, now)`: `_firedEventValues.length = 0`; push `events[u].value` for every `u < eventCount` with
`before < fireTime <= now`; return the array (shared — copy it in Rust).

### 14.7 `CubismMotionQueueEntry`

Fields (constructor): `_autoDelete = false`, `_motion = null`, `_available = true`, `_finished = false`, `_started =
false`, `_startTimeSeconds = -1.0`, `_fadeInStartTimeSeconds = 0.0`, `_endTimeSeconds = -1.0`, `_stateTimeSeconds = 0.0`,
`_stateWeight = 0.0`, `_lastEventCheckSeconds = 0.0`, `_motionQueueEntryHandle = this` (the handle **is** the entry
object), `_fadeOutSeconds = 0.0`, `_isTriggeredFadeOut = false`.

Methods: `release()` (deletes the motion if `_autoDelete`), `setFadeOut(s)` (`_fadeOutSeconds = s;
_isTriggeredFadeOut = true`), `startFadeOut(s, userTime)` (`newEnd = userTime + s; _isTriggeredFadeOut = true; if
(_endTimeSeconds < 0 || newEnd < _endTimeSeconds) _endTimeSeconds = newEnd`), getters/setters for finished, started,
start time, fade-in start time, end time, available, `setState(t, w)`, `getStateTime()`, `getStateWeight()`,
`get/setLastCheckEventSeconds`, `isTriggeredFadeOut()`, `getFadeOutSeconds()`, `getCubismMotion()`.

### 14.8 `CubismMotionQueueManager`

State: `_userTimeSeconds = 0`, `_eventCallBack = null`, `_eventCustomData = null`, `_motions = []`.
`InvalidMotionQueueEntryHandleValue = -1`.

| Method | Behaviour |
|---|---|
| `startMotion(motion, autoDelete, userTime?)` | `motion == null` → −1. For every existing non-null entry: `entry.setFadeOut(entry._motion.getFadeOutTime())` (end time is applied later by `startFadeOut`). New entry `{_autoDelete, _motion}` pushed at the **end**; return its handle |
| `isFinished()` | iterate: null entries and entries with null motion are removed (splice); return false at the first unfinished entry; true otherwise (also true when empty) |
| `isFinishedByHandle(h)` | false iff an entry with that handle exists and is not finished |
| `stopAllMotions()` | `for (i = 0; i < len; i++) { entry.release(); splice(i, 1); continue }` — **QUIRK:** splicing while incrementing removes only every other entry (e.g. `[A,B,C]` → `[B]`) |
| `getCubismMotionQueueEntries()` / `getCubismMotionQueueEntry(h)` | array / entry by handle or null |
| `setEventCallback(cb, customData = null)` | store |
| `doUpdateMotion(model, userTime)` | `updated = false`; for each entry `i` (while-loop with manual index): null → splice; null motion → release + splice; else `motion.updateParameters(model, entry, userTime)`; `updated = true`; `fired = motion.getFiredEvent(entry.getLastCheckEventSeconds() - entry.getStartTime(), userTime - entry.getStartTime())`; for each value → `_eventCallBack(this, value, _eventCustomData)`; `entry.setLastCheckEventSeconds(userTime)`; if `entry.isFinished()` → release + splice (index not advanced); else { if `entry.isTriggeredFadeOut()` → `entry.startFadeOut(entry.getFadeOutSeconds(), userTime)`; `i++` }. Return `updated` |

Entries are processed oldest first, so a newly started motion blends **on top of** the fading-out one (each writes
`source + (value − source) · weight`).

### 14.9 `CubismMotionManager` (priorities)

`_currentPriority = 0`, `_reservePriority = 0`.
* `startMotionPriority(motion, autoDelete, priority)`: `if (priority == _reservePriority) _reservePriority = 0`;
  `_currentPriority = priority`; `return startMotion(motion, autoDelete)`.
* `updateMotion(model, dt)`: `_userTimeSeconds += dt`; `updated = doUpdateMotion(model, _userTimeSeconds)`; `if
  (isFinished()) _currentPriority = 0`; return `updated`.
* `reserveMotion(priority)`: `if (priority <= _reservePriority || priority <= _currentPriority) return false`;
  `_reservePriority = priority`; return true.
* `getCurrentPriority()`, `getReservePriority()`, `setReservePriority(v)`.

Golden: `golden/motion_manager.jsonl` — `hiyori_m01` started at frame 0 (priority 2), `mtn_02` at frame 120 (priority 2),
dt = 1/60, per-frame `{userTime, updated, isFinished, currentPriority, entries[{start, fadeInStart, end, weight,
finished, triggeredFadeOut}], values{…}, events[]}` with the V1 frame order `loadParameters → updateMotion →
saveParameters`. Observed: began events at frames 0/120; first motion's end time becomes `3.0167` (= 2.0167 + 1 s
fade-out) at frame 120, weight reaches 0 at frame 180 and the entry is removed at frame 181 (no finished event);
`mtn_02` ends naturally at frame 576 (`finished:` event), after which `updated = false`, `isFinished = true`,
`currentPriority = 0`.

---

## 15. Expression system (Tier B — unused by this model)

### 15.1 exp3.json (`CubismExpressionMotion.create(buffer, size)` → `parse`)

Keys/defaults: `FadeInTime`, `FadeOutTime` (default `DefaultFadeTime = 1.0` via `toFloat(1.0)` when missing),
`Parameters`, `Id`, `Value` (`toFloat()`, default 0), `Blend`: missing (null) or `'Add'` → Additive; `'Multiply'` →
Multiply; `'Overwrite'` → Overwrite; anything else → Additive. `ExpressionBlendType { Additive 0, Multiply 1,
Overwrite 2 }`; `DefaultAdditiveValue = 0.0`, `DefaultMultiplyValue = 1.0`. `create` never returns null (bad JSON
throws). Each parameter → `ExpressionParameter { parameterId, blendType, value }` in file order.

### 15.2 Legacy `doUpdateParameters(model, userTime, weight, entry)` (only via `updateParameters`, not used by the manager)

Additive → `model.addParameterValueById(id, value, weight)`; Multiply → `multiplyParameterValueById(id, value, weight)`;
Overwrite → `setParameterValueById(id, value, weight)`.

### 15.3 `calculateExpressionParameters(model, userTime, entry, values[], expressionIndex, fadeWeight)`

```
if entry == null || values == null: return
if !entry.isAvailable(): return
for each ev in values (ExpressionParameterValue {parameterId, additiveValue, multiplyValue, overwriteValue}):
    if ev.parameterId == null: continue
    current = ev.overwriteValue = model.getParameterValueById(ev.parameterId)      // RESET to the model value every call
    j = index of ev.parameterId in this expression's parameters, or -1
    if j < 0:
        if expressionIndex == 0: ev.additive = 0; ev.multiply = 1; ev.overwrite = current
        else: ev.additive  = ev.additive  * (1 - fw) + 0 * fw
              ev.multiply  = ev.multiply  * (1 - fw) + 1 * fw
              ev.overwrite = ev.overwrite * (1 - fw) + current * fw            // = current
        continue
    value = params[j].value
    Additive:  (newAdd, newMul, newOvr) = (value, 1, current)
    Multiply:  (0, value, current)
    Overwrite: (0, 1, value)
    default:   return                                                           // (unreachable)
    if expressionIndex == 0: ev.additive = newAdd; ev.multiply = newMul; ev.overwrite = newOvr
    else: ev.additive  = ev.additive  * (1 - fw) + newAdd * fw
          ev.multiply  = ev.multiply  * (1 - fw) + newMul * fw
          ev.overwrite = ev.overwrite * (1 - fw) + newOvr * fw                  // ev.overwrite == current here
```
`calculateValue(source, destination, fw) = source * (1 - fw) + destination * fw`; `getExpressionParameters()`.

### 15.4 `CubismExpressionMotionManager.updateMotion(model, dt)`

State: `_expressionParameterValues = []`, `_fadeWeights = []` (plus inherited queue state). Base `startMotion` is used
unchanged (old entries get `setFadeOut(fadeOutTime)`).

```
_userTimeSeconds += dt; updated = false; motions = _motions
expressionWeight = 0.0; expressionIndex = 0
if _fadeWeights.length != motions.length: resize (new slots = 0.0; shrink truncates)
for i over motions (manual index):
    entry = motions[i]; null → splice, continue
    em = entry.getCubismMotion(); null → splice, continue
    if entry.isAvailable():
        for p in em.getExpressionParameters(): if p.parameterId not yet in _expressionParameterValues:
            push { parameterId: p.parameterId, additiveValue: 0, multiplyValue: 1, overwriteValue: model.getParameterValueById(p.parameterId) }
    em.setupMotionQueueEntry(entry, _userTimeSeconds)
    setFadeWeight(expressionIndex, em.updateFadeWeight(entry, _userTimeSeconds))
    em.calculateExpressionParameters(model, _userTimeSeconds, entry, _expressionParameterValues, expressionIndex, getFadeWeight(expressionIndex))
    expressionWeight += em.getFadeInTime() == 0 ? 1.0 : getEasingSine((_userTimeSeconds - entry.getFadeInStartTime()) / em.getFadeInTime())
    updated = true
    if entry.isTriggeredFadeOut(): entry.startFadeOut(entry.getFadeOutSeconds(), _userTimeSeconds)
    i++; expressionIndex++
if motions.length > 1 && getFadeWeight(_fadeWeights.length - 1) >= 1.0:
    for i = motions.length - 2 down to 0: motions.splice(i, 1); _fadeWeights.splice(i, 1)       // keep only the newest
if expressionWeight > 1.0: expressionWeight = 1.0
for ev in _expressionParameterValues (in insertion order):
    model.setParameterValueById(ev.parameterId, (ev.overwriteValue + ev.additiveValue) * ev.multiplyValue, expressionWeight)
    ev.additiveValue = 0; ev.multiplyValue = 1
return updated
```
`getFadeWeight(i)` / `setFadeWeight(i, w)` warn (4.3) and return −1 / do nothing when `i` is out of range.
Entries are never removed because they "finished"; expression motions have `getDuration() = -1` → end time −1 until a
newer expression triggers their fade-out.

**QUIRKs (pinned by `golden/expression_manager.jsonl`):**
1. Because `overwriteValue` is reset to the current model value before each expression's contribution, an older
   expression's **Overwrite** is discarded as soon as a newer expression is queued: at frame 90 (B started)
   `ParamBrowLY` jumps 0.7 → 0; at frame 200 (A restarted over B) `ParamMouthForm` jumps −0.5 → 0, then fades to 0.5.
   Additive and multiply contributions do cross-fade.
2. `expressionWeight` sums only **fade-in** curves (not fade-outs) and is clamped to 1.
3. Virtual parameters (IDs not in the moc) are not restored by `loadParameters`, so Additive expressions on them
   **accumulate every frame** (`PARAM_TERE` grows 0 → 95 over the test). Real parameters are restored each frame, so they
   do not.
4. A `Blend` value like `"Bogus"` is treated as Additive (`ParamEyeROpen` +2 → clamped at 1).

### 15.5 This model's expression files (unreferenced)

`expressions/Angry, Blushing, f01, f02, Normal, Sad, Smile, Surprised .exp3.json` (51–1132 bytes; `"Type": "Live2D
Expression"`, no fade keys → 1.0 s fades, all `Blend: "Add"`; `Normal` has `Parameters: []`). They use Cubism-2 IDs
(`PARAM_EYE_L_OPEN`, `PARAM_EYE_R_OPEN`, `PARAM_EYE_L_SMILE`, `PARAM_EYE_R_SMILE`, `PARAM_EYE_BALL_X/Y`,
`PARAM_BROW_{L,R}_{Y,X,ANGLE,FORM}`, `PARAM_MOUTH_FORM`, `PARAM_MOUTH_OPEN_Y`, `PARAM_TERE`) — none exists in the moc
(which uses `ParamEyeLOpen` etc.). If they were ever referenced they would only create (and keep accumulating) virtual
parameters. V2 parity: do not reference them. (A V2 feature that wants real expressions must ship exp3 files with the
moc's IDs.)

---

## 16. Math (`CubismMath`, `CubismVector2`)

### 16.1 `CubismMath` (all static, f64)

| Function | Exact definition |
|---|---|
| `Epsilon` | `0.00001` |
| `range(v, min, max)` | `v < min ? min : (v > max ? max : v)` |
| `clamp(v, min, max)` | `v < min ? min : (max < v ? max : v)` (NaN passes through) |
| `sin/cos/abs/sqrt(x)` | `Math.*` |
| `max(l, r)` / `min(l, r)` | `l > r ? l : r` / `l > r ? r : l` (NaN-order sensitive: `max(NaN, x) = x`, `max(x, NaN) = NaN`) |
| `getEasingSine(v)` | `v < 0 → 0`; `v > 1 → 1`; else `0.5 - 0.5 * cos(v * π)` |
| `degreesToRadian(d)` | `(d / 180.0) * π` |
| `radianToDegrees(r)` | `(r * 180.0) / π` |
| `directionToRadian(from, to)` | 13.9 |
| `directionToDegrees(from, to)` | `deg = radianToDegrees(directionToRadian(from, to)); if (to.x - from.x > 0) deg = -deg` |
| `radianToDirection(a)` | `(sin a, cos a)` |
| `cbrt(x)` | **not `Math.cbrt`**: `if (x === 0) return x`; `cx = abs(x)`; `ret = (cx === Infinity) ? Infinity : exp(log(cx) / 3)` then one Newton step `ret = (cx / (ret * ret) + 2 * ret) / 3`; restore sign |
| `quadraticEquation(a, b, c)` | `abs(a) < Eps → (abs(b) < Eps ? -c : -c / b)`; else `-(b + sqrt(b*b - 4.0*a*c)) / (2.0*a)` |
| `cardanoAlgorithmForBezier(a, b, c, d)` | see below |
| `mod(dividend, divisor)` | if `!isFinite(dividend) \|\| divisor === 0 \|\| isNaN(dividend) \|\| isNaN(divisor)` → ``console.warn(`divided: ${dividend}, divisor: ${divisor} mod() returns 'NaN'.`)``, return NaN; `r = abs(dd) - floor(abs(dd) / abs(ds)) * abs(ds)`; `return r * Math.sign(dividend)` |

```
cardanoAlgorithmForBezier(a, b, c, d):
    if |a| < Epsilon: return range(quadraticEquation(b, c, d), 0.0, 1.0)
    ba = b/a; ca = c/a; da = d/a
    p = (3.0*ca - ba*ba) / 3.0;  p3 = p / 3.0
    q = (2.0*ba*ba*ba - 9.0*ba*ca + 27.0*da) / 27.0;  q2 = q / 2.0
    discriminant = q2*q2 + p3*p3*p3
    center = 0.5; threshold = center + 0.01                       // 0.51
    if discriminant < 0.0:
        mp3 = -p / 3.0; mp33 = mp3*mp3*mp3; r = sqrt(mp33)
        t = -q / (2.0*r); cosphi = range(t, -1.0, 1.0); phi = acos(cosphi)
        crtr = cbrt(r); t1 = 2.0*crtr
        root1 = t1*cos(phi/3.0) - ba/3.0;               if |root1 - center| < threshold: return range(root1, 0, 1)
        root2 = t1*cos((phi + 2.0*π)/3.0) - ba/3.0;     if |root2 - center| < threshold: return range(root2, 0, 1)
        root3 = t1*cos((phi + 4.0*π)/3.0) - ba/3.0;     return range(root3, 0, 1)
    if discriminant == 0.0:
        u1 = (q2 < 0.0) ? cbrt(-q2) : -cbrt(q2)
        root1 = 2.0*u1 - ba/3.0;  if |root1 - center| < threshold: return range(root1, 0, 1)
        root2 = -u1 - ba/3.0;     return range(root2, 0, 1)
    sd = sqrt(discriminant); u1 = cbrt(sd - q2); v1 = cbrt(sd + q2)
    return range(u1 - v1 - ba/3.0, 0.0, 1.0)
```

### 16.2 `CubismVector2` (f64 `x`, `y`; `constructor(x?, y?)` → missing = 0)

`add(v)`, `substract(v)` (sic), `multiply(v)` (component-wise), `multiplyByScaler(s)` (sic) = `multiply((s, s))`,
`division(v)`, `divisionByScalar(s)`, `getLength() = sqrt(x² + y²)`, `getDistanceWith(a)`, `dot(a)`, `normalize()`
(in place; `len = Math.pow(x² + y², 0.5)` ≡ `sqrt`; `x /= len; y /= len`; zero vector → NaN), `isEqual(v)`,
`isNotEqual(v)`. All operations except `normalize` return new vectors. Rust: `#[derive(Copy)] struct Vec2 { x: f64, y: f64 }`
with the same method names in snake_case (keep `normalize` in place).

`CubismMatrix44`, `CubismModelMatrix`, `CubismViewMatrix`, `CubismTargetPoint`, `csmRect`: render spec 3.

---

## 17. Asset inventory (this model)

Folder: `/home/elvelt_admin/Elvelt/dsuv/public/resources/models/AmadeusKurisu5.0/reama5.0/` (served at
`/resources/models/AmadeusKurisu5.0/reama5.0/`). All JSON files are UTF-8 **without BOM**, tab-indented. File names
contain `紅莉栖` → URL-encode when fetching (`encodeURIComponent` per path segment; V1 relies on the browser's automatic
encoding of `fetch(url)` with raw Unicode, which is equivalent).

### 17.1 `Live2D紅莉栖forSDK5.0.model3.json` (verbatim)

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

### 17.2 `Live2D紅莉栖forSDK5.0.moc3`

284,480 bytes; first 8 bytes `4D 4F 43 33 05 00 00 00` (`"MOC3"`, version byte 5, rest zero) → `MocVersion_50`;
`hasMocConsistency` = 1. Canvas 7000 × 10000 px, origin (3500, 5000), PixelsPerUnit 7000. 27 parameters, 15 parts,
49 drawables, 0 offscreens, render-order array length 49. Texture: render spec 1.

### 17.3 Parameters (Core values + cdi3 display names; all type 0, repeat 0)

| # | Id | Name (cdi3) | min | max | default | key values |
|---|---|---|---|---|---|---|
| 0 | `ParamAngleX` | 角度 X | −30 | 30 | 0 | −30, 0, 30 |
| 1 | `ParamAngleY` | 角度 Y | −30 | 30 | 0 | −30, 0, 30 |
| 2 | `ParamAngleZ` | 角度 Z | −30 | 30 | 0 | −30, 0, 30 |
| 3 | `ParamEyeLOpen` | 左目　開閉 | 0 | 1 | **1** | 0, 1 |
| 4 | `ParamEyeLSmile` | 左目　笑顔 | 0 | 1 | 0 | 0, 1 |
| 5 | `ParamEyeROpen` | 右目　開閉 | 0 | 1 | **1** | 0, 1 |
| 6 | `ParamEyeRSmile` | 右目　笑顔 | 0 | 1 | 0 | 0, 1 |
| 7 | `ParamEyeBallX` | 目玉 X | −1 | 1 | 0 | −1, 0, 1 |
| 8 | `ParamEyeBallY` | 目玉 Y | −1 | 1 | 0 | −1, 0, 1 |
| 9 | `ParamBrowLForm` | 左眉　変形 | −1 | 1 | 0 | −1, −0.7, 0, 0.7, 1 |
| 10 | `ParamBrowLY` | 左眉　上下 | −1 | 1 | 0 | −1, 0, 1 |
| 11 | `ParamBrowRAngle` | 右眉　角度 | −1 | 1 | 0 | (none) |
| 12 | `ParamBrowLX` | 左眉　左右 | −1 | 1 | 0 | (none) |
| 13 | `ParamBrowRX` | 右眉　左右 | −1 | 1 | 0 | (none) |
| 14 | `ParamBrowLAngle` | 左眉　角度 | −1 | 1 | 0 | (none) |
| 15 | `ParamBrowRY` | 右眉　上下 | −1 | 1 | 0 | −1, 0, 1 |
| 16 | `ParamBrowRForm` | 右眉　変形 | −1 | 1 | 0 | −1, −0.7, 0, 0.7, 1 |
| 17 | `ParamMouthForm` | 口　変形 | −1 | 1 | 0 | −1, 0, 1 |
| 18 | `ParamMouthOpenY` | 口　開閉 | 0 | 1 | 0 | 0, 0.3, 0.7, 1 |
| 19 | `ParamCheek` | 照れ | 0 | 1 | 0 | (none) |
| 20 | `ParamBodyAngleX` | 体の回転 X | −10 | 10 | 0 | (none) |
| 21 | `ParamBodyAngleY` | 体の回転 Y | −10 | 10 | 0 | (none) |
| 22 | `ParamBodyAngleZ` | 体の回転 Z | −10 | 10 | 0 | (none) |
| 23 | `ParamBreath` | 呼吸 | 0 | 1 | 0 | 0, 1 |
| 24 | `ParamHairFront` | 髪揺れ　前 | −1 | 1 | 0 | −1, 0, 1 |
| 25 | `ParamHairSide` | 髪揺れ　横 | −1 | 1 | 0 | −1, 0, 1 |
| 26 | `ParamHairBack` | 髪揺れ　後 | −1 | 1 | 0 | −1, 0, 1 |

(Key values `0.7`/`0.3` are the f32 values `0.699999988079071`/`0.30000001192092896`.) Parameters with no key values
(11–14, 19–22) exist but deform nothing (`ParamBrowRAngle`, `ParamBrowLX`, `ParamBrowRX`, `ParamBrowLAngle`,
`ParamCheek`, `ParamBodyAngleX/Y/Z`) — the player still writes some of them (visual no-op, but `ParamBodyAngleX/Z` are
physics inputs).

cdi3 `CombinedParameters` (editor UI only, unused): `[ParamAngleX, ParamAngleY]`, `[ParamEyeBallX, ParamEyeBallY]`,
`[ParamMouthForm, ParamMouthOpenY]`; `ParameterGroups: []`.

### 17.4 Parts (Core order, cdi3 names, parents)

| # | Id | Name | Parent | Drawables (direct) |
|---|---|---|---|---|
| 0 | `Part` | 前髪 | − | 0–9, 42 |
| 1 | `Part6` | 手 | − | 13, 14 |
| 2 | `Part7` | 鼻 | − | 15 |
| 3 | `Part16` | 口 | − | 33–39 |
| 4 | `Part11` | 新右目 | − | 26, 27, 43, 44 (+ child part 13) |
| 5 | `Part9` | 左目 | − | 24, 25, 47, 48 (+ child part 14) |
| 6 | `Part13` | 顔 | − | 16 |
| 7 | `Part2` | 白衣 | − | 10 |
| 8 | `Part3` | 足 | − | 40, 41 |
| 9 | `Part4` | 裏白衣 | − | 11 |
| 10 | `Part5` | シャツ | − | 12 |
| 11 | `Part14` | 首 | − | 17 |
| 12 | `Part15` | 後髪 | − | 18 |
| 13 | `Part12` | 黒目 | 4 (`Part11`) | 28–32, 45 |
| 14 | `Part10` | 黒目 | 5 (`Part9`) | 19–23, 46 |

Drawable parent-part array (index = drawable): `0,0,0,0,0,0,0,0,0,0,7,9,10,1,1,2,6,11,12,14,14,14,14,14,5,5,4,4,13,13,13,
13,13,3,3,3,3,3,3,3,8,8,0,4,4,13,14,5,5`.

### 17.5 Physics: 13.12. Expressions: 15.5.

### 17.6 Motions folder (unreferenced; Tier B test material)

`motions/` holds 93 `*.motion3.json` plus 93 Unity `*.anim` files (unused by Web/V2) and 7 stray Finder-style copies
(`mtn_01.motion3 1.json`, `mtn_01.motion3 2.json`, `mtn_02.motion3 1.json`, `mtn_02.motion3 2.json`, `mtn_03.motion3 1.json`,
`mtn_03.motion3 2.json`, `mtn_04.motion3 1.json`; contents differ from the originals; not matched by `*.motion3.json`
and not referenced anywhere). Every motion: `Meta.Loop: true`
(ignored by the framework), curve groups correctly ordered, Meta counts consistent with the data (verified), no user-data
events. Families (counts of Parameter curves whose ID exists in this moc / total):

| Family | Files | Fps | Beziers restricted | Curves | IDs present in moc |
|---|---|---|---|---|---|
| `01_kei_{en,jp,ko,zh}` | 4 | 60 | yes | 9 Parameter | 6/9 |
| `Epsilon_idle_01`, `Epsilon_m_01…08`, `Epsilon_m_sp_01…05`, `Epsilon_shake_01` | 15 | 30 | yes | 37 Parameter (+1 PartOpacity, except `Epsilon_shake_01`) | 0/37 (Cubism-2 `PARAM_*`) |
| `Scene1…3` | 3 | 30 | yes | 68–71 Parameter (+1 Model `Opacity` in Scene3) | 20 |
| `haru_g_idle`, `haru_g_m01…26` | 27 | 30 | yes | 1 Model `Opacity` + 42 Parameter + 20 PartOpacity (stepped) | 26/42 |
| `haru_idle_01…03`, `haru_m_01…10`, `haru_normal_01…10` | 23 | 30 | yes | 33 Parameter + 4 PartOpacity; `haru_normal_*` contain Stepped and InverseStepped segments | 0/33 |
| `hiyori_m01…10` | 10 | 30 | **no** (Cardano) | 29–33 Parameter + 2 PartOpacity | 24 |
| `mtn_00…07` | 8 | 30/60 | yes | 68–128 Parameter (+4–8 PartOpacity); `mtn_00/05/06/07` have per-curve `FadeInTime: 0.2` on 2 curves; `mtn_02/03/04` have Meta fades 0.25/0.25/0.5 | 24–27 |
| `special_01…03` | 3 | 30 | yes | 128 Parameter + 4 PartOpacity, Meta fades 0.25 | 24/128 |

Full per-file table: run `python3 -I .../l2d-model-probe/motions_summary.py <motions dir>`.

---

## 18. Golden test data

Generator: `/tmp/claude-0/-home-elvelt-admin-Elvelt-dsuv/1f3badd8-3ee9-4a13-9340-35fac0e7abf1/scratchpad/l2d-model-probe/harness.cjs`
(`node harness.cjs <outDir>`; Node ≥ 18). It loads the shipped Core and the deployed bundle into one `vm` context,
replaces the bundle's last three lines (`runtimeWindow.ReamaCubismRuntime = …; void preloadDefaultAssets();`) with an
export of the framework classes, waits for wasm readiness, and writes into
`/tmp/claude-0/-home-elvelt-admin-Elvelt-dsuv/1f3badd8-3ee9-4a13-9340-35fac0e7abf1/scratchpad/l2d-model-probe/golden/`.
**Copy these files into the repo** (suggested: `RealAmadeusRust/crates/amadeus-live2d/tests/golden/`) together with the
harness (`tools/live2d-golden/harness.cjs`); the scratchpad is not durable.

| File | Content | Use |
|---|---|---|
| `model_info.json` | versions, canvas, full parameter table (min/max/default/repeat/keys), parts, hierarchy, drawable parents, `isRepeat` defaults, model-setting accessor results | FFI + model wrapper + setting tests |
| `physics_rig.jsonl` | parsed rig: settings, inputs, outputs (incl. `translationScale {0,0}`, `valueBelowMinimum: "undefined"`), particles after `initialize`, rig gravity `[0, 0]` (after `create`), option gravity `[0, -1]` | parse tests |
| `pipeline_dt60.jsonl` (600 frames), `pipeline_dt144.jsonl` (400), `pipeline_dt30.jsonl` (300) | V1 frame pipeline (load → save → Look/Breath/Physics → core update) at fixed dt; per frame `{frame, dt, remain, cur[3], prev[3], params{27 ids → value}}` | breath + look + physics parity |
| `pipeline_jitter.jsonl` (360) | dt pattern `[16,17,17,16,17,17,16,17,17,33,16,17,0,17,16,17,50,17]` ms repeated, frame 200 = 6.0 s | V1-realistic dt, zero-dt quirk (frame 12), MaxDeltaTime reset (frame 200) |
| `pipeline_drag.jsonl` (240) | as dt60 but drag target `(0.8, −0.5)` | TargetPoint + Look + physics response |
| `curve_*.json` (5 files) | `evaluateCurve` samples every 1/60 s up to `duration + 12/60` (plain and loop-corrected) | curve evaluation |
| `motion_manager.jsonl` | two motions with priorities, cross-fade, natural finish, events | queue/manager/fade |
| `expression_manager.jsonl` | synthetic A/B expressions (Add/Multiply/Overwrite/missing/bogus blends, virtual ID) started at frames 0, 90, 200 | expression blending quirks |
| `eyeblink.jsonl` | 900 frames with the injected random sequence | eye-blink state machine |

Comparison rules: parse each JSON number as f64; compare model parameter values after `as f32` with ≤ 2 ulp (or
`1e-6` absolute); compare `cur/prev/remain` and curve samples with relative error ≤ `1e-9` (bit-exact if the `libm`
crate is used). Breath values at frame `k` depend on accumulated `_currentTime` (sum of f64 dts) — accumulate exactly
like JS (left fold of `+=`).

---

## 19. Porting order and what can be skipped

1. **`core_ffi`**: script injection + readiness poll + one-shot log binding; `Moc`/`Model` creation; assert counts
   27/15/49/0 and versions 6.0.1 / moc 5 against `model_info.json`.
2. **IDs + JSON helpers + `ModelSetting`**: assert every accessor in `model_info.modelSetting`.
3. **`CubismModel`**: shadows + `core_update`, parameter API with clamp/weight/virtual slots, save/load, parts,
   drawable accessors for the renderer (implement the render spec's `RenderModel` trait).
4. **Math** (`CubismMath` incl. `cbrt`/Cardano, `Vec2`) with unit tests copied from the formulas above.
5. **Breath, `CubismTargetPoint`, Look, scheduler**: compare `ParamAngleX/Y/Z`, `ParamBodyAngleX`, `ParamBreath`,
   `ParamEyeBallX/Y` of `pipeline_dt60` and `pipeline_drag`.
6. **Physics** (parse → initialize → evaluate → interpolate): all five `pipeline_*` goldens, including the zero-dt and
   stall frames.
7. Wire the exact V1 frame order (1.2) together with the renderer and the player driver.
8. Tier B in this order: motion3 parse + curve evaluation (`curve_*`), queue entry/manager + priorities
   (`motion_manager`), `LAppModel`-style motion loading/`startMotion`/`startRandomMotion`, expressions
   (`expression_manager`), eye blink (`eyeblink`), pose, user data, colour overrides, physics `stabilization`, parameter
   repeat.
9. Tier C only if needed: consistency checks, lip-sync updater/provider, hit testing.

For pixel/behaviour parity of the current avatar, steps 1–7 are sufficient; everything in step 8 is unreachable with the
current model3.json.

---

## Rust mapping

Crate `amadeus-live2d` (wasm32-only; `#![cfg(target_arch = "wasm32")]` at the crate root, or feature `web`), module tree
for this subsystem (renderer modules per the render spec):

```
amadeus_live2d::
  core_ffi            // 3.5 externs; CoreHandle { model, values_view, part_opacities_view, drawables, ... }
  core_loader         // ensure_core_loaded(base_url) -> Result<CoreInfo, CoreError>   (script tag + readiness poll)
  framework           // start() / FrameworkOptions { log_level }; one-shot csmSetLogFunction (OnceCell)
  id                  // CubismId(u32), IdManager (global interner), default_ids::{PARAM_ANGLE_X: &str = "ParamAngleX", …}
  json                // parse_cubism_json(&[u8]) -> Result<Value>; trait CubismValueExt { get, idx, to_float(d), to_int(d), to_bool(d), raw_str() }; js_key_order()
  setting             // ModelSetting (model3.json) — all accessors of section 5
  moc                 // CubismMoc { handle: core_ffi::Moc, moc_version: u32, model_count: u32 }
  model               // CubismModel (section 7) + impl RenderModel (render spec 15)
  color_override      // MultiplyScreenColors (section 8)
  user_data           // ModelUserData (section 9)
  math::{cubism_math, vector2}           // CubismMath fns, Vec2   (matrix44/model_matrix/view_matrix/target_point: render spec)
  physics::{rig, json, physics}          // CubismPhysics (section 13)
  effect::{breath, eye_blink, pose, look}
  motion::{data, json, curve, base, motion, queue, manager}   // section 14
  expression::{motion, manager}                               // section 15
  update::{scheduler, UpdaterKind}                            // section 11
  user_model          // UserModel: owns moc/model/physics/breath/look/drag/eye_blink/pose/managers/scheduler (10.1 + V1 LAppModel subset)
```

Key types and signatures:

```rust
pub enum ParamSlot { Real(usize), Virtual(usize) }            // virtual = index >= count (7.4)

pub struct CubismModel {
    core: core_ffi::CoreHandle,
    param_ids: Vec<CubismId>, part_ids: Vec<CubismId>, drawable_ids: Vec<CubismId>,
    param_values: Vec<f32>, param_min: Vec<f32>, param_max: Vec<f32>, param_default: Vec<f32>, param_repeats: Vec<i32>,
    part_opacities: Vec<f32>, saved_parameters: Vec<f32>,
    virtual_param_ids: HashMap<CubismId, usize>, virtual_param_values: HashMap<usize, Option<f64>>,
    virtual_part_ids: HashMap<CubismId, usize>, virtual_part_opacities: HashMap<usize, Option<f64>>,
    overridden_parameter_repeat: bool /* default true */, user_parameter_repeat: Vec<ParameterRepeatData>,
    overridden_cullings: bool, drawable_culling: CullingData /* shared */, offscreen_culling: CullingData,
    model_opacity: f32, blend_mode_enabled: bool,
    drawable_color_blend_cache: Vec<i32>, drawable_alpha_blend_cache: Vec<i32>,
    parts_hierarchy: Vec<PartInfo>, colors: MultiplyScreenColors,
}
impl CubismModel {
    pub fn from_moc(moc: &CubismMoc) -> Result<Self, ModelError>;           // createModel + initialize
    pub fn core_update(&mut self);                                         // 3.6 (push, update, pull, resetDynamicFlags)
    pub fn parameter_count(&self) -> usize;
    pub fn parameter_index(&mut self, id: CubismId) -> ParamSlot;          // registers virtual slots
    pub fn parameter_value(&self, slot: ParamSlot) -> f64;                 // None -> 0.0
    pub fn parameter_value_raw(&self, slot: ParamSlot) -> Option<f64>;
    pub fn set_parameter_value(&mut self, slot: ParamSlot, value: f64, weight: f64);
    pub fn add_parameter_value(&mut self, slot: ParamSlot, value: f64, weight: f64);
    pub fn multiply_parameter_value(&mut self, slot: ParamSlot, value: f64, weight: f64);
    pub fn set_parameter_value_by_id(&mut self, id: CubismId, value: f64, weight: f64);   // + add_/multiply_/get_ by_id
    pub fn is_repeat(&self, slot: ParamSlot) -> bool;
    pub fn repeat_value(&self, i: usize, v: f64) -> f64; pub fn clamp_value(&self, i: usize, v: f64) -> f64;
    pub fn save_parameters(&mut self); pub fn load_parameters(&mut self);
    pub fn part_index(&mut self, id: CubismId) -> ParamSlot;
    pub fn part_opacity(&self, slot: ParamSlot) -> f64; pub fn set_part_opacity(&mut self, slot: ParamSlot, v: f64);
    pub fn drawable_index(&self, id: CubismId) -> Option<usize>;
    pub fn canvas_width(&self) -> f64; pub fn canvas_height(&self) -> f64; pub fn pixels_per_unit(&self) -> f64;
    pub fn params_raw_mut(&mut self) -> &mut [f32];                        // physics direct access (13.6/13.7)
    // drawable accessors: see render spec 4.1 / RenderModel
}

pub trait Motion {                                   // ACubismMotion + CubismMotion/CubismExpressionMotion
    fn base(&self) -> &MotionBase; fn base_mut(&mut self) -> &mut MotionBase;      // fades, weight, loop flags, callbacks
    fn duration(&self) -> f64; fn loop_duration(&self) -> f64;
    fn do_update_parameters(&mut self, model: &mut CubismModel, user_time: f64, fade_weight: f64, entry: &mut MotionQueueEntry);
    fn fired_events(&mut self, before: f64, now: f64) -> Vec<String> { Vec::new() }
    fn as_expression(&self) -> Option<&ExpressionMotion> { None }
}
pub type MotionRef = Rc<RefCell<dyn Motion>>;        // shared: the same motion object may sit in several entries
pub struct MotionHandle(u64);                         // TS handle = entry identity; −1 → Option::None
pub struct MotionQueueEntry { handle: MotionHandle, motion: MotionRef, auto_delete: bool, available: bool, finished: bool,
    started: bool, start_time: f64 /* -1 */, fade_in_start_time: f64, end_time: f64 /* -1 */, state_time: f64,
    state_weight: f64, last_event_check: f64, fade_out_seconds: f64, triggered_fade_out: bool }
pub struct MotionQueueManager { user_time: f64, entries: Vec<MotionQueueEntry>, event_callback: Option<Box<dyn FnMut(&str)>> }
pub struct MotionManager { queue: MotionQueueManager, current_priority: i32, reserve_priority: i32 }
pub struct ExpressionMotionManager { queue: MotionQueueManager, values: Vec<ExpressionParameterValue>, fade_weights: Vec<f64> }

pub struct UserModel {                                // V1 LAppModel framework subset
    pub model: CubismModel, physics: Option<CubismPhysics>, breath: CubismBreath, look: CubismLook,
    drag: CubismTargetPoint, eye_blink: Option<CubismEyeBlink>, pose: Option<CubismPose>,
    motion_manager: MotionManager, expression_manager: ExpressionMotionManager,
    scheduler: UpdateScheduler, motions: IndexMap<String, MotionRef>, expressions: IndexMap<String, MotionRef>,
    eye_blink_ids: Vec<CubismId>, lip_sync_ids: Vec<CubismId>, motion_updated: bool, user_time: f64,
}
impl UserModel {
    pub async fn load(base_url: &str, model3_file: &str, fetch: &dyn AssetFetcher) -> Result<Self, ModelError>;  // 10.2 order
    pub fn update(&mut self, dt: f64);                // 1.2 LAppModel.update (ends with model.core_update())
    pub fn start_motion(&mut self, group: &str, no: usize, priority: i32) -> Option<MotionHandle>;
    pub fn start_random_motion(&mut self, group: &str, priority: i32, rng: &mut dyn Rng) -> Option<MotionHandle>;
    pub fn set_expression(&mut self, name: &str);
    pub fn set_dragging(&mut self, x: f64, y: f64);
}
```

Desktop note: `amadeus-desktop` (Tauri 2) hosts the same wasm frontend in a webview, so this crate runs unchanged there;
the Core file must be bundled as a frontend asset and the webview CSP must allow wasm compilation
(`script-src 'self' 'wasm-unsafe-eval'`), because the Core instantiates its embedded wasm with `WebAssembly.instantiate`.

---

## Open questions / risks

1. **Transcendental-function drift**: Rust `std` math may differ from V8 by 1 ulp → tiny physics divergence over time
   (chaotic only mildly; outputs are damped). Use the `libm` crate for `sin/cos/atan2/acos/exp/log` in physics and
   motion to maximise agreement, and compare goldens with tolerances (section 18).
2. **One-shot `csmSetLogFunction`** (3.3): a second call throws (wasm table cannot grow). V1 has a latent remount bug
   (`startUp` after `cleanUp`). V2 must bind once per page and never pass `undefined`.
3. **Core readiness / load order** (3.1): V1 races two `async` scripts and never waits for wasm init. V2 must load the
   Core first and poll readiness; failure must surface as a UI error, not a silent blank avatar.
4. **Heap growth detaches views** (3.4): safe for one model; add the length check + recreate path.
5. **Core clamps in `update()`** (3.3): any Rust shadow must be pulled back after each Core update (3.6), otherwise NaN /
   out-of-range writes would diverge.
6. **dt source**: V1 feeds `Date.now()` deltas in whole milliseconds (1.2). Physics/breath goldens assume the same dt
   sequence; the player spec should keep `Date.now()` (or document the change). The very first V1 delta is ~1.7e9 s but
   is never consumed by the model; V2 should start from a sane first delta.
7. **Driver after physics** (1.2): V1 hair physics never reacts to the driver's head/body angles (only to breath). Moving
   the driver earlier would be a visible behaviour change; keep V1 order for parity and expose an opt-in flag if the
   product wants hair to follow head motion.
8. **Zero-dt glitch** (13.7): reproduced by default; a fix (skip the whole model update when `dt == 0`) is a one-line,
   declared deviation if desired.
9. **Unreferenced assets with mismatched IDs** (15.5, 17.6): adding them to model3.json would not animate the face
   (Cubism-2 IDs) and Additive expressions on virtual IDs grow without bound (15.4 QUIRK 3). Any future "expressions"
   feature needs new exp3/motion files authored for this moc.
10. **Meta-count trust** (13.3, 14.3): V1 crashes (TypeError/infinite loop) on inconsistent or unknown data; the Rust
    port should size from actual arrays and return errors instead (declared safe deviations, unreachable with shipped
    files).
11. **JSON key order** (4.4): only matters if `Motions` group names are integer-like strings; implement `js_key_order`
    anyway.
12. **Licensing**: `live2dcubismcore.min.js` is under the Live2D Proprietary Software License (redistributable as-is);
    the Cubism Web Framework is under the Live2D Open Software License — a Rust port is a derivative work and must keep
    the copyright notice and license terms; commercial distribution of apps using the Cubism SDK may require a Live2D
    publication license depending on the publisher's revenue. Not a code issue, but must be confirmed by the user before
    public V2 releases (desktop installers redistribute the Core).
13. **Coordination with the render spec**: the renderer consumes `CubismModel` through its `RenderModel` trait
    (render spec section 15); the method names above should be reconciled with that trait when both crates' modules are
    created (semantics are identical — render spec 4.1).
14. **`ParamBreath` double source**: the framework breath writes `ParamBreath` (feeds nothing in physics) and the player
    driver overwrites it every frame with its own sine → only the driver's value is visible. Keep both for parity (the
    framework breath on `ParamAngleX/Y/Z`/`ParamBodyAngleX` is what drives the hair).

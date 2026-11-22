# Porting spec: `live2d-framework-render` (Cubism Web Framework rendering subset → `amadeus-live2d`)

Target crate: `amadeus-live2d` (wasm32 only, `web-sys` WebGL). This spec covers the Cubism Web Framework
rendering layer (renderer, shaders, clipping masks, render targets, offscreen pool) and the math classes it
depends on (`CubismMatrix44`, `CubismModelMatrix`, `CubismViewMatrix`, `CubismTargetPoint`, `csmRect`),
exactly as they run inside V1 (RealAmadeusWeb). It also covers the small amount of sample-layer glue that
decides what the renderer receives every frame (GL context, textures, MVP, frame order, GL state at frame start).

Everything below was read line by line from the sources listed in section 0 and cross-checked with probes that
run the project's real `live2dcubismcore.min.js` against the real `.moc3` (section 0.5). Where V1 behaviour differs
from what the code "obviously intends", the V1 behaviour is documented and marked **QUIRK**. The Rust port must
reproduce QUIRKs unless the spec explicitly says a deviation is safe (marked **DEVIATION-OK**) or recommends a fix
(marked **BUG**).

Conventions used in this document:
* `FW/` = `/tmp/claude-0/-home-elvelt-admin-Elvelt-dsuv/1f3badd8-3ee9-4a13-9340-35fac0e7abf1/scratchpad/ref/RealAmadeusWeb/CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/`
* `BUNDLE` = `/home/elvelt_admin/Elvelt/dsuv/public/resources/live2d/reama-cubism-runtime.js` (deployed V1 runtime)
* `SHADERS/` = `/home/elvelt_admin/Elvelt/dsuv/public/resources/live2d/shaders/`
* "f32" = IEEE binary32 (JS `Float32Array` element), "f64" = JS `number`.
* `gl.X` constants are the standard WebGL enum values (`ONE` = 1, `ZERO` = 0, `ONE_MINUS_SRC_ALPHA` = 0x0303, ...).

---

## 0. Sources and provenance

### 0.1 TypeScript originals (SDK 5-r.5, as vendored by the user)

| File | Lines | Content |
|---|---|---|
| `FW/rendering/cubismrenderer.ts` | 425 | abstract `CubismRenderer`, `CubismBlendMode`, `DrawableObjectType`, `CubismTextureColor`, abstract `CubismClippingContext` |
| `FW/rendering/cubismrenderer_webgl.ts` | 1780 | `CubismClippingManager_WebGL`, `CubismClippingContext_WebGL`, `CubismRendererProfile_WebGL`, `CubismRenderer_WebGL` |
| `FW/rendering/cubismshader_webgl.ts` | 2067 | `CubismShader_WebGL`, `CubismShaderManager_WebGL`, `CubismShaderSet`, `ShaderNames`, `ShaderType` (mtime Apr 11, one day newer than the rest = user-touched, see 0.4) |
| `FW/rendering/cubismclippingmanager.ts` | 994 | generic `CubismClippingManager<T>` (layout, bounds, mask matrices) |
| `FW/rendering/cubismrendertarget_webgl.ts` | 289 | `CubismRenderTarget_WebGL` (FBO + color texture, `copyBuffer`) |
| `FW/rendering/cubismoffscreenrendertarget_webgl.ts` | 240 | `CubismOffscreenRenderTarget_WebGL` |
| `FW/rendering/cubismoffscreenmanager.ts` | 591 | `CubismWebGLOffscreenManager` (pool of FBOs per GL context) |
| `FW/math/cubismmatrix44.ts` | 361 | `CubismMatrix44` |
| `FW/math/cubismmodelmatrix.ts` | 217 | `CubismModelMatrix` |
| `FW/math/cubismviewmatrix.ts` | 339 | `CubismViewMatrix` |
| `FW/math/cubismtargetpoint.ts` | 169 | `CubismTargetPoint` |
| `FW/type/csmrectf.ts` | 89 | `csmRect` |

Also read (owned by other subsystems; only the parts the renderer consumes are specified here):
`FW/model/cubismmodel.ts` (enums `CubismColorBlend`/`CubismAlphaBlend`, all drawable/offscreen accessors,
`update()`, `initialize()` blend-mode detection, parts hierarchy), `FW/model/cubismmodelmultiplyandscreencolor.ts`
(multiply/screen overrides), `FW/math/cubismmath.ts` (`Epsilon`, `clamp`), `FW/live2dcubismframework.ts`
(`Constant.vertexOffset = 0`, `Constant.vertexStep = 2`, `LogLevel`), `FW/utils/cubismdebug.ts` (log gating),
`FW/utils/cubismarrayutils.ts` (`updateSize`).

### 0.2 Shader files

`SHADERS/` contains 13 files, **byte-identical** to `.../Framework/Shaders/WebGL/` (`diff -r` empty).
All ASCII, LF line endings, every file starts with the same 6-line Live2D license comment followed by 2 empty lines.

| File | Bytes | md5 |
|---|---|---|
| `vertshadersrc.vert` | 465 | 4e1c5fb89f0524e9987747e4f0fe6e15 |
| `vertshadersrcmasked.vert` | 559 | eb7251aed7ae67dfaa4e6bee6362a7c2 |
| `vertshadersrcsetupmask.vert` | 536 | b2dcf650cbae2ea8dfbc6bd1ea3a2b43 |
| `fragshadersrcsetupmask.frag` | 736 | bec774fe1a2d4df16baf5d340c929452 |
| `fragshadersrcpremultipliedalpha.frag` | 747 | 9b6c1b6774b7e6a4d207df46f7586c18 |
| `fragshadersrcmaskpremultipliedalpha.frag` | 1043 | a39e6abc07d37e1fec6a19f810813237 |
| `fragshadersrcmaskinvertedpremultipliedalpha.frag` | 1051 | da498fd5b159b0d1c09ca12cb47079f4 |
| `vertshadersrccopy.vert` | 392 | ab779e034dfcbe1a5af2873f5f3c518c |
| `fragshadersrccopy.frag` | 429 | fcdda38373872a30cbff7bc7ee7ba67f |
| `fragshadersrccolorblend.frag` | 7525 | 8fe459c2c27e2ad4453da78286f2805a |
| `fragshadersrcalphablend.frag` | 2189 | db4b7df97b150e4116bce74e70f86edc |
| `vertshadersrcblend.vert` | 738 | 936c093963d5cdb0c1cee3456d4286ec |
| `fragshadersrcpremultipliedalphablend.frag` | 2038 | 7a183c23f482ae31c43589377fa3f834 |

### 0.3 The deployed V1 runtime bundle

`BUNDLE` — 669,953 bytes, 18,643 lines, esbuild IIFE (`target: "es2017"`, starts with `"use strict";`), NOT
minified, comments preserved, one `// <path>` marker per source file. Loaded by
`components/amadeus/Live2DCanvas.tsx` as `/resources/live2d/reama-cubism-runtime.js?v=19` after
`/resources/live2d/live2dcubismcore.min.js` (constants `CORE_SCRIPT_SRC` / `RUNTIME_SCRIPT_SRC`, lines 31–32).

Bundle line ranges of the files relevant here:

| Source | Bundle lines |
|---|---|
| `math/cubismmath.ts` | 313–612 |
| `math/cubismmatrix44.ts` | 613–939 |
| `type/csmrectf.ts` | 940–1006 |
| `utils/cubismdebug.ts` | 1015–1127 |
| `rendering/cubismrenderer.ts` | 1128–1403 |
| Samples `lapppal.ts` | 2625–2664 |
| Samples `lappdefine.ts` (+ user-added cache helpers 2699–2764) | 2665–2765 |
| Samples `lappglmanager.ts` | 2766–2799 |
| `utils/cubismarrayutils.ts` | 2800–2821 |
| `rendering/cubismrendertarget_webgl.ts` | 2822–3037 |
| `rendering/cubismoffscreenmanager.ts` | 3038–3422 |
| `math/cubismmodelmatrix.ts` | 4686–4861 |
| `math/cubismtargetpoint.ts` | 4862–4957 |
| `model/cubismmodelmultiplyandscreencolor.ts` | 8529–9352 |
| `model/cubismmodel.ts` (enums at 9356–9386) | 9353–10748 |
| `rendering/cubismclippingmanager.ts` | 10749–11393 |
| `rendering/cubismshader_webgl.ts` | 11394–12907 |
| `rendering/cubismoffscreenrendertarget_webgl.ts` | 12908–13084 |
| `rendering/cubismrenderer_webgl.ts` | 13085–14311 |
| `model/cubismusermodel.ts` (`createRenderer` at 14859) | 14590–14964 |
| Samples `lappmodel.ts` | 15666–16384 |
| Samples `lapplive2dmanager.ts` | 16385–16573 |
| Samples `lapptexturemanager.ts` | 16574–16726 |
| `math/cubismviewmatrix.ts` | 16727–16997 |
| Samples `lappview.ts` | 17274–17475 |
| Samples `lappsubdelegate.ts` | 17476–17688 |
| `scripts/cubism-runtime-entry.ts` (`ReamaCubismPlayer`, `preloadDefaultAssets`) | 17689–18643 |

Key deployed function lines (for reviewers): `copyBuffer` 2830, `createRenderTarget` 2902, `createMatrixForMask`
11138, `setupLayoutBounds` 11180, `calcClippedDrawableTotalBounds` 11291, `loadShader` 11450,
`setupShaderProgramForDrawable` 11564, `setupShaderProgramForMask` 11991, `registerShader` 12205,
`registerBlendShader` 12587, `loadShaderProgram` 12724, `compileShaderSource` 12775, `setupClippingContext` 13119,
`doDrawModel` 13647, `drawObjectLoop` 13718, `drawDrawable` 13776, `drawMeshWebGL` 13836, `preDraw` 14147,
`startUp` 14234, `createTextureFromPngFile` 16598.

**Provenance check performed:** the bundle was rebuilt from the ref TS sources with the same esbuild options
(`.../scratchpad/build-l2d/rebuilt.js`) and diffed against the deployed file (`.../scratchpad/build-l2d/bundle.diff`).
For every file assigned to this spec the **only** difference is:

```js
// CubismShader_WebGL.loadShader(url) — deployed V1 (BUNDLE line 11450):
async loadShader(url) {
  return await cachedFetchText(url);
}
// ref TS / pristine SDK (FW/rendering/cubismshader_webgl.ts lines 68–71):
//   const response = await fetch(url); return await response.text();
```

All other deployed-vs-ref differences are in the sample/entry layer (cache helpers in lappdefine, asset prefetch in
lappmodel, projection + second `update()` in lapplive2dmanager, image cache in lapptexturemanager, emotion/idle logic
in the entry). The ones that influence rendering are documented in section 12. **Therefore the authoritative
renderer behaviour = the TS files in 0.1 + the `cachedFetchText` change.**

### 0.4 User customizations already present in the ref sources (not in the pristine SDK)

1. `Samples/.../lappglmanager.ts` requests **WebGL1 first** and falls back to WebGL2, with the comment (verbatim):
   `// WebGL2だとCubism sample shaderの初期化に失敗する環境があるため、` /
   `// Qt版と同じ安定挙動を優先してWebGL1を先に使う。`
   ("Because there are environments where Cubism sample shader initialization fails with WebGL2, prefer WebGL1 first
   for the same stable behaviour as the Qt version.")
2. `cubismshader_webgl.ts` contains direct `console.error(...)` diagnostics in `loadShaderProgram` /
   `compileShaderSource` (strings in section 13) in addition to the SDK's `CubismLogError` calls.
3. `CubismShader_WebGL` constructor: `_defaultShaderPath = '/resources/live2d/shaders/'` (absolute app path).
   `lappdefine.ts` `ShaderPath = "/resources/live2d/shaders/"` (same value).

### 0.5 Ground-truth data extracted from the real model with the real Core

The project's `live2dcubismcore.min.js` ("Live2D Cubism SDK Core Version 6.0.1", `csmGetVersion()` = 100663297,
latest supported moc version 6 = MocVersion_53) was run in a Node `vm` sandbox against
`/home/elvelt_admin/Elvelt/dsuv/public/resources/models/AmadeusKurisu5.0/reama5.0/Live2D紅莉栖forSDK5.0.moc3`.

* Probes written for this spec (re-runnable, `node <script> <core.js> <model.moc3>`):
  `.../scratchpad/render-spec-probe/flags.cjs` (dynamic-flag semantics, colors, opacities, render orders,
  MIN_VALUE quirk), `.../scratchpad/render-spec-probe/masks.cjs` (independent re-implementation of the mask layout and
  mask matrices with exact `Float32Array` semantics; output saved in `masks.out`), `bits.cjs` (Core bit helpers and
  blend constants), `mvp.cjs` (MVP values), `blendmap.cjs` (blend shader index table), `layout.cjs` (layout table).
* Earlier probes from the same session (consistent with the above): `.../scratchpad/l2d-probe/` (`probe.cjs`,
  `golden.jsonl`, `drawables.md`, `layout.cjs`, `blendmap.cjs`).

---

## 1. Executive summary — what THIS model needs

Model: `/resources/models/AmadeusKurisu5.0/reama5.0/Live2D紅莉栖forSDK5.0.model3.json` (FileReferences: Moc,
1 texture, Physics, DisplayInfo; **no** Motions, Expressions, Pose, HitAreas, Layout or UserData; Groups `EyeBlink`
and `LipSync` with empty `Ids`).

| Fact | Value | Consequence for the renderer |
|---|---|---|
| moc3 version | MocVersion 5.0 (`csmGetMocVersion` = 5) | No Cubism 5.3 color/alpha blend modes or offscreens in the data |
| Core | 6.0.1 (supports MocVersion_53, exposes `drawables.blendModes`, `model.offscreens`, `model.getRenderOrders()`) | Bind to the 5.3-style Core API (section 4) |
| Canvas info | CanvasWidth 7000, CanvasHeight 10000, Origin (3500, 5000), PixelsPerUnit 7000 | `getCanvasWidth()` = 1.0, `getCanvasHeight()` = 1.4285714285714286 model units; `getPixelsPerUnit()` = 7000 |
| Drawables / parts / parameters | 49 / 15 / 27 | |
| Offscreens | 0 | `getOffscreenCount()` = 0, `isUsingMaskingForOffscreen()` = false |
| Textures | 1: `Live2D紅莉栖forSDK5.0.8192/texture_00.png`, **4096×4096 RGBA8** (despite the folder name) | All 49 drawables use texture index 0 |
| `drawables.blendModes` | 47× `0` (Normal/Over), 2× `1` (AddCompatible/Over) | `isBlendModeEnabled()` = **false** |
| `constantFlags` | 45× `4` (double-sided), 2× `5` (additive + double-sided), 2× `12` (double-sided + inverted mask) | **Culling never enabled**; 2 additive drawables (ArtMesh30, ArtMesh53); 2 inverted-mask drawables (ArtMesh57, ArtMesh33); no multiplicative |
| Masks | 16 masked drawables, **6 clipping contexts**, exactly 1 mask drawable per context | 1 mask render texture 256×256, layout R:2, G:2, B:1, A:1 (R and G split in halves) |
| Dynamic flags at draw time | `33` (= IsVisible \| VertexPositionsDidChange) for all 49 drawables at every V1 draw (V1 runs two `update()`s per frame; a model that has seen only one `update()` reports `127`, which also contains both bits) | Every drawable drawn every frame; every mask drawable re-rendered every frame |
| Opacity / multiply / screen at default pose | opacity 1.0 for all; multiply (1,1,1,1); screen (0,0,0,1) for all | Shader still receives them every draw |
| Max vertices / indices per drawable | 328 / 1767 (ArtMesh35) | Indices fit `Uint16` |
| `drawOrders` | all 500; `parts.parentIndices` = [-1 ×13, 4, 5]; `parts.offscreenIndices` all -1 | Render order is static in practice but is re-read every frame |
| UV range | 0.00142 … 0.99858 | Texture wrap mode (default REPEAT) never matters |
| High-precision mask | off (only forced when `isBlendModeEnabled()`) | Use the shared-atlas mask path |

**Needed for this model (must be behaviour-faithful):**
`CubismMatrix44` (exact f32 semantics), `CubismModelMatrix`, `CubismViewMatrix` (identity in V1 but cheap),
`CubismTargetPoint` (drives `ParamAngle*` via the look updater; owned by the model subsystem but the class is in this
spec), `csmRect` (f64), `CubismRenderer_WebGL` non-blend path (`initialize`, `startUp`, `bindTexture`,
`setIsPremultipliedAlpha`, `setMvpMatrix`, `setRenderState`, `drawModel`/`doDrawModel`, `preDraw`,
`drawObjectLoop` for drawables, `drawDrawable`, `drawMeshWebGL`), 4 GLSL programs (SetupMask, Normal, NormalMasked,
NormalMaskedInverted) with the Normal / Additive / Mask blend functions, the clipping manager (shared 256² atlas,
`findSameClip`, `setupClippingContext`, `calcClippedDrawableTotalBounds` **including the `Number.MIN_VALUE` QUIRK,
which is active for 2 of the 6 contexts of this model** (section 9.5/9.10), `setupLayoutBounds`,
`createMatrixForMask`), `CubismRenderTarget_WebGL` (create/begin/end/destroy).

**Implement but trivially (cheap, keeps generality):** multiplicative blend, culling, 4-split and 9-split mask
layouts, multiple mask render textures (`maskBufferCount > 1`), high-precision masks, `setClippingMaskBufferSize`
(with the BUG of section 5.5 fixed), anisotropy (no-op at 0).

**May be stubbed (return `Err(Unsupported)` / feature-gated `blend53`):** Cubism 5.3 color/alpha blend shaders
(237 programs) and the copy shader, `copyBuffer` (needs WebGL2), offscreens (`CubismOffscreenRenderTarget_WebGL`,
`CubismWebGLOffscreenManager`, `addOffscreen`, `submitDrawToParentOffscreen`, `drawOffscreen*`),
`_modelRenderTargets` (before/afterDrawModelRenderTarget), `CubismRendererProfile_WebGL` (never called in V1).
They are fully specified below so they can be implemented later without re-reading the TS.

V1 compiles **242** GL programs at startup for every model (4 distinct base programs + 1 copy + 237 blend programs),
because `registerBlendShader()` runs unconditionally. For this model only 4 are ever used (section 7.3, risk R1).

---

## 2. GL context: WebGL1 vs WebGL2 decision

### 2.1 What V1 does (BUNDLE 2766–2799, `LAppGlManager.initialize(canvas)`)

```js
const contextAttributes = { alpha: true, antialias: true, premultipliedAlpha: true, preserveDrawingBuffer: false };
this._gl = canvas.getContext("webgl", contextAttributes);     // WebGL1 FIRST
if (!this._gl) this._gl = canvas.getContext("webgl2", contextAttributes);
if (!this._gl) { console.error("Cannot initialize WebGL context."); this._gl = null; return false; }
return true;
```

`depth`/`stencil` attributes are left at their defaults (depth: true, stencil: false). So in every real browser V1
runs on **WebGL1**. Consequences inside the framework:

* `CubismRenderTarget_WebGL.copyBuffer` throws `Error('WebGL2RenderingContext is required for buffer copy.')` under
  WebGL1. It is only reached for Cubism 5.3 blend modes / offscreens, never for this model. (If it were reached, the
  exception would escape `drawModel` → `LAppSubdelegate.update` → `ReamaCubismPlayer.tick` *before*
  `requestAnimationFrame(this.tick)` is re-armed, i.e. the animation loop would stop forever.)
* All shaders are GLSL ES 1.00 (no `#version` line), valid in both WebGL1 and WebGL2.
* The only extension probed is `EXT_texture_filter_anisotropic` / `WEBKIT_EXT_texture_filter_anisotropic` /
  `MOZ_EXT_texture_filter_anisotropic` (first non-null wins, probed in `startUp`), used only if anisotropy > 0
  (never in V1).
* No VAOs, no instancing, no `OES_*` extensions, no float textures, no MRT, no depth/stencil in FBOs.

### 2.2 Decision for V2

* **Primary: WebGL1** (`web_sys::WebGlRenderingContext`) with exactly the same context attributes;
  **fallback: WebGL2** (`web_sys::WebGl2RenderingContext`) with the same attributes. This matches V1 pixel behaviour
  (MSAA on the default framebuffer via `antialias: true`, non-multisampled FBOs, premultiplied canvas compositing)
  and is the safest choice for the Tauri webviews (WebView2 on Windows, WKWebView on macOS, WebKitGTK on Linux — the
  last has the weakest WebGL2 story). The desktop builds run the same wasm UI inside the webview, so no native GL
  backend is needed.
* Abstract both behind one internal `Gl` enum (section 15); every call used by the renderer exists with identical
  semantics on both.
* `copy_buffer` (5.3 only) returns `Err(RenderError::NeedsWebGl2)` on WebGL1 instead of throwing; the caller then
  skips the blend-mode draw of that object (**DEVIATION-OK**, unreachable for this model) and the frame loop must
  never die because of a render error (always re-arm `requestAnimationFrame` first or catch errors).

### 2.3 Complete WebGL API surface used by the framework renderer

`getParameter(FRAMEBUFFER_BINDING | VIEWPORT | ARRAY_BUFFER_BINDING | ELEMENT_ARRAY_BUFFER_BINDING | CURRENT_PROGRAM |
ACTIVE_TEXTURE | TEXTURE_BINDING_2D | FRONT_FACE | COLOR_WRITEMASK | BLEND_SRC_RGB | BLEND_DST_RGB | BLEND_SRC_ALPHA |
BLEND_DST_ALPHA)`, `getExtension`, `viewport`, `enable/disable(SCISSOR_TEST | STENCIL_TEST | DEPTH_TEST | BLEND |
CULL_FACE)`, `isEnabled`, `frontFace(CW | CCW)`, `colorMask`, `clearColor`, `clear(COLOR_BUFFER_BIT)`,
`blendFuncSeparate`, `createBuffer/deleteBuffer/bindBuffer/bufferData(ARRAY_BUFFER | ELEMENT_ARRAY_BUFFER,
DYNAMIC_DRAW | STATIC_DRAW)`, `enableVertexAttribArray/disableVertexAttribArray/vertexAttribPointer(…, 2, FLOAT,
false, stride, 0)`, `getVertexAttrib(i, VERTEX_ATTRIB_ARRAY_ENABLED)`, `useProgram`, `createProgram/deleteProgram/
attachShader/linkProgram/getProgramParameter(LINK_STATUS)/getProgramInfoLog`, `createShader/shaderSource/
compileShader/getShaderParameter(COMPILE_STATUS)/getShaderInfoLog/deleteShader`, `getAttribLocation`,
`getUniformLocation`, `uniform1i`, `uniform1f`, `uniform4f`, `uniformMatrix4fv(loc, false, Float32Array(16))`,
`activeTexture(TEXTURE0 | TEXTURE1 | TEXTURE2)`, `createTexture/deleteTexture/bindTexture(TEXTURE_2D)/texImage2D(…,
RGBA, w, h, 0, RGBA, UNSIGNED_BYTE, null)/texParameteri(WRAP_S/WRAP_T = CLAMP_TO_EDGE, MIN/MAG = LINEAR)/
texParameterf(TEXTURE_MAX_ANISOTROPY_EXT)`, `createFramebuffer/deleteFramebuffer/bindFramebuffer(FRAMEBUFFER)/
framebufferTexture2D(FRAMEBUFFER, COLOR_ATTACHMENT0, TEXTURE_2D, tex, 0)/checkFramebufferStatus`,
`drawElements(TRIANGLES, n, UNSIGNED_SHORT, 0)`; WebGL2 only: `bindFramebuffer(READ_FRAMEBUFFER | DRAW_FRAMEBUFFER)`,
`blitFramebuffer(…, COLOR_BUFFER_BIT, NEAREST)`.
Host glue additionally uses `isContextLost`, `depthFunc(LEQUAL)`, `clearDepth`, `clear(COLOR|DEPTH)`, `blendFunc`,
`flush`, `pixelStorei(UNPACK_PREMULTIPLY_ALPHA_WEBGL, 1)`, `texImage2D(…, image)`, `generateMipmap`,
`getParameter(MAX_TEXTURE_SIZE)`, `drawingBufferWidth/Height`.

---

## 3. Math (exact semantics)

### 3.1 Number representation rules (critical for golden tests)

* JS `number` = IEEE-754 binary64. A store into a `Float32Array` element rounds to binary32 (round-half-to-even).
* `CubismMatrix44._tr` is a `Float32Array(16)`. Every matrix element is therefore an f32. **All arithmetic on
  elements happens in f64** (elements are widened when read) and is rounded to f32 only when stored back into a
  `Float32Array`.
* `csmRect` fields, `CubismTextureColor` fields, `CubismModelMatrix._width/_height`, `CubismViewMatrix` screen
  fields, `CubismTargetPoint` fields, clipping scale factors are plain JS numbers → **f64**.
* Core arrays (`vertexPositions`, `opacities`, `multiplyColors`, …) are `Float32Array` → values are f32, widened to
  f64 when read in JS.
* WebGL `uniform*f` / `uniformMatrix4fv` convert JS numbers to f32 (round-to-nearest).

Rust rule of thumb: store matrices as `[f32; 16]`; rectangles, colors and view parameters as `f64`; perform each JS
expression in `f64` and cast `as f32` exactly where JS stores into a `Float32Array` (Rust `f64 as f32` rounds to
nearest-even, identical to JS `ToFloat32`).

### 3.2 `CubismMatrix44` storage convention

* 16 floats `tr[0..16]`, interpreted as a **row-major 4×4 matrix M for row vectors** (`v' = v · M`), i.e.
  `M[r][c] = tr[r*4 + c]`, translation in `tr[12], tr[13], tr[14]`. Byte-for-byte this is the OpenGL column-major
  layout of `Mᵀ`, so it is uploaded with `uniformMatrix4fv(loc, false /*transpose*/, tr)` and used in GLSL as
  `gl_Position = u_matrix * a_position` (column vector). **Never transpose.**
* Scale X = `tr[0]`, scale Y = `tr[5]`, translate X = `tr[12]`, translate Y = `tr[13]`.
* GLSL `a_position` is declared `vec4` but fed with 2 floats → `(x, y, 0, 1)`; so
  `x' = tr[0]·x + tr[4]·y + tr[12]`, `y' = tr[1]·x + tr[5]·y + tr[13]`, `w' = tr[3]·x + tr[7]·y + tr[15]`.

### 3.3 `CubismMatrix44` methods (all of them)

| Method | Exact behaviour |
|---|---|
| `constructor()` | `_tr = new Float32Array(16)`; `loadIdentity()` |
| `static multiply(a, b, dst)` | `c = new Float32Array(16)` (zeros); `for i in 0..4, j in 0..4, k in 0..4: c[j + i*4] += a[k + i*4] * b[j + k*4]`; then copy `c` → `dst` (16 elements). Row-major `c = a · b`. **Precision:** each `+=` reads `c` (f32→f64), adds the exact f64 product of two f32 values, rounds the f64 sum to f32 on store; accumulation order k = 0,1,2,3. Rust: `c[idx] = ((c[idx] as f64) + (a[..] as f64) * (b[..] as f64)) as f32`. `dst` may alias `a` or `b` (the temporary makes it safe). |
| `loadIdentity()` | `setMatrix([1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1])` |
| `setMatrix(tr)` | copy 16 elements (f32→f32) |
| `getArray()` | returns the live `_tr` (callers may mutate it, e.g. `LAppLive2DManager.setViewMatrix` writes into it) |
| `getScaleX()` / `getScaleY()` | `tr[0]` / `tr[5]` |
| `getTranslateX()` / `getTranslateY()` | `tr[12]` / `tr[13]` |
| `transformX(src)` | `tr[0]*src + tr[12]` (f64 result, not rounded) |
| `transformY(src)` | `tr[5]*src + tr[13]` |
| `invertTransformX(src)` | `(src - tr[12]) / tr[0]` |
| `invertTransformY(src)` | `(src - tr[13]) / tr[5]` |
| `translateRelative(x, y)` | `tr1 = Float32Array([1,0,0,0, 0,1,0,0, 0,0,1,0, x,y,0,1])` (**x, y rounded to f32 here**); `multiply(tr1, _tr, _tr)` ⇒ `M ← T·M` (translation applied to the vertex **before** the existing transform; new translation = old translation + x·row0 + y·row1) |
| `translate(x, y)` | `tr[12] = x; tr[13] = y` (absolute, f32 store) |
| `translateX(x)` / `translateY(y)` | `tr[12] = x` / `tr[13] = y` |
| `scaleRelative(x, y)` | `tr1 = Float32Array([x,0,0,0, 0,y,0,0, 0,0,1,0, 0,0,0,1])`; `multiply(tr1, _tr, _tr)` ⇒ `M ← S·M` (rows 0/1 scaled, translation untouched) |
| `scale(x, y)` | `tr[0] = x; tr[5] = y` (absolute) |
| `multiplyByMatrix(m)` | `multiply(m.getArray(), _tr, _tr)` ⇒ `this ← m · this` (m applied to the vertex **first**). The TS doc admits the name is misleading: "関数名と実際の計算内容に乖離があるため、今後計算順が修正される可能性があります。" |
| `getInvert()` | see below |
| `clone()` | new matrix, copy all 16 (`for i < _tr.length`) |

`getInvert()` (affine inverse; f64 math, f32 store):
```
r00=tr[0] r10=tr[1] r20=tr[2] r01=tr[4] r11=tr[5] r21=tr[6] r02=tr[8] r12=tr[9] r22=tr[10]
tx=tr[12] ty=tr[13] tz=tr[14]
det = r00*(r11*r22 - r12*r21) - r01*(r10*r22 - r12*r20) + r02*(r10*r21 - r11*r20)
dst = new CubismMatrix44()                       // identity
if |det| < CubismMath.Epsilon (0.00001): dst.loadIdentity(); return dst
invDet = 1.0/det
inv00 =  (r11*r22 - r12*r21)*invDet   inv01 = -(r01*r22 - r02*r21)*invDet   inv02 =  (r01*r12 - r02*r11)*invDet
inv10 = -(r10*r22 - r12*r20)*invDet   inv11 =  (r00*r22 - r02*r20)*invDet   inv12 = -(r00*r12 - r02*r10)*invDet
inv20 =  (r10*r21 - r11*r20)*invDet   inv21 = -(r00*r21 - r01*r20)*invDet   inv22 =  (r00*r11 - r01*r10)*invDet
dst.tr = [inv00, inv10, inv20, 0,  inv01, inv11, inv21, 0,  inv02, inv12, inv22, 0,
          -(inv00*tx + inv01*ty + inv02*tz), -(inv10*tx + inv11*ty + inv12*tz), -(inv20*tx + inv21*ty + inv22*tz), 1]
```
(The translation terms use the **f64** `inv*` values, not the f32-rounded stored ones.) `tr[3], tr[7], tr[11]` of the
input are ignored. Only used for 5.3 offscreen masks.

### 3.4 `CubismModelMatrix extends CubismMatrix44`

Fields `_width`, `_height` (f64).

* `constructor(w?, h?)`: `super()`; `_width = (w !== undefined) ? w : 0.0`; `_height = (h !== undefined) ? h : 0.0`; then `setHeight(2.0)`.
* `setWidth(w)`: `s = w / _width; scale(s, s)` (absolute scale on both axes; division by 0 → Infinity, no guard).
* `setHeight(h)`: `s = h / _height; scale(s, s)`.
* `setPosition(x, y)`: `translate(x, y)`.
* `setCenterPosition(x, y)`: `centerX(x); centerY(y)` (doc note: must be called after width/height are set).
* `top(y)`: `setY(y)`. `bottom(y)`: `h = _height * getScaleY(); translateY(y - h)`.
* `left(x)`: `setX(x)`. `right(x)`: `w = _width * getScaleX(); translateX(x - w)`.
* `centerX(x)`: `w = _width * getScaleX(); translateX(x - w / 2.0)`.
* `centerY(y)`: `h = _height * getScaleY(); translateY(y - h / 2.0)`.
* `setX(x)`: `translateX(x)`. `setY(y)`: `translateY(y)`.
* `setupFromLayout(layout: Map<string, number>)`: two passes over the map **in insertion order**.
  Pass 1: key `"width"` → `setWidth(v)`, key `"height"` → `setHeight(v)`. Pass 2: `"x"`→`setX`, `"y"`→`setY`,
  `"center_x"`→`centerX`, `"center_y"`→`centerY`, `"top"`→`top`, `"bottom"`→`bottom`, `"left"`→`left`,
  `"right"`→`right`. Other keys ignored. (Keys come from model3.json `Layout`; values via `toFloat()`.)

V1/this model: created in `CubismUserModel.loadModel` as `new CubismModelMatrix(model.getCanvasWidth(),
model.getCanvasHeight())` = `(1.0, 1.4285714285714286)` → `setHeight(2.0)` → `scale(1.4, 1.4)` stored as f32
`1.399999976158142`. model3.json has **no `Layout`** → `setupFromLayout` gets an empty map → no-op. Translation stays
(0, 0).

### 3.5 `CubismViewMatrix extends CubismMatrix44`

Fields (f64, all `0.0` initially): `_screenLeft, _screenRight, _screenTop, _screenBottom, _maxLeft, _maxRight,
_maxTop, _maxBottom, _maxScale, _minScale`.

* `adjustTranslate(x, y)`:
  ```
  if (tr[0]*_maxLeft   + (tr[12] + x) > _screenLeft)   x = _screenLeft   - tr[0]*_maxLeft   - tr[12]
  if (tr[0]*_maxRight  + (tr[12] + x) < _screenRight)  x = _screenRight  - tr[0]*_maxRight  - tr[12]
  if (tr[5]*_maxTop    + (tr[13] + y) < _screenTop)    y = _screenTop    - tr[5]*_maxTop    - tr[13]
  if (tr[5]*_maxBottom + (tr[13] + y) > _screenBottom) y = _screenBottom - tr[5]*_maxBottom - tr[13]
  multiply(Float32Array([1,0,0,0, 0,1,0,0, 0,0,1,0, x,y,0,1]), tr, tr)      // same as translateRelative(x, y)
  ```
  QUIRK: the clamp assumes the new translation is `tr[12]+x`, but `T·M` actually adds `x*tr[0]` (only equal when
  scale = 1). Reproduce as written.
* `adjustScale(cx, cy, scale)`: `maxScale = getMaxScale(); minScale = getMinScale(); targetScale = scale * tr[0]`;
  if `targetScale < minScale` → (if `tr[0] > 0`) `scale = minScale / tr[0]`; else if `targetScale > maxScale` →
  (if `tr[0] > 0`) `scale = maxScale / tr[0]`. Then `multiply(T(-cx,-cy), tr, tr)`; `multiply(S(scale,scale), tr, tr)`;
  `multiply(T(cx,cy), tr, tr)` in that order (each `T`/`S` built as `Float32Array`, so `cx, cy, scale` are rounded to
  f32).
* `setScreenRect(left, right, bottom, top)`: `_screenLeft=left, _screenRight=right, _screenBottom=bottom, _screenTop=top`.
* `setMaxScreenRect(left, right, bottom, top)`: `_maxLeft=left, _maxRight=right, _maxTop=top, _maxBottom=bottom`.
* `setMaxScale(v)`, `setMinScale(v)`, `getMaxScale()`, `getMinScale()`.
* `isMaxScale()`: `getScaleX() >= _maxScale`. `isMinScale()`: `getScaleX() <= _minScale`.
* Getters: `getScreenLeft/Right/Bottom/Top()`, `getMaxLeft/Right/Bottom/Top()`.

V1 usage (`LAppView.initialize`, BUNDLE 17290–17317), run at init and on every resize:
```
ratio = width / height                       // canvas.width / canvas.height (device pixels)
left = -ratio; right = ratio; bottom = ViewLogicalLeft (-1); top = ViewLogicalRight (1)
_viewMatrix.setScreenRect(left, right, bottom, top)
_viewMatrix.scale(ViewScale=1, ViewScale=1)  // absolute → resets zoom, keeps translation
_deviceToScreen.loadIdentity()
if (width > height) { screenW = |right - left|; _deviceToScreen.scaleRelative(screenW/width, -screenW/width) }
else                { screenH = |top - bottom|; _deviceToScreen.scaleRelative(screenH/height, -screenH/height) }
_deviceToScreen.translateRelative(-width*0.5, -height*0.5)
_viewMatrix.setMaxScale(ViewMaxScale=2); _viewMatrix.setMinScale(ViewMinScale=0.8)
_viewMatrix.setMaxScreenRect(ViewLogicalMaxLeft=-2, ViewLogicalMaxRight=2, ViewLogicalMaxBottom=-2, ViewLogicalMaxTop=2)
```
`adjustTranslate/adjustScale` are never called in V1 (no caller exists in the bundle; the canvas has
`pointer-events: none` and `LAppSubdelegate.onPoint*` are never invoked), so the view matrix is the identity for the
whole session.

### 3.6 `csmRect` (f64)

* `constructor(x?, y?, w?, h?)`: assigns the arguments as-is → **`new csmRect()` has all four fields `undefined`**
  (they are always written before being read in the paths this model uses; see 9.5 QUIRK for the exception).
* `getCenterX() = x + 0.5*width`, `getCenterY() = y + 0.5*height`, `getRight() = x + width`, `getBottom() = y + height`.
* `setRect(r)`: copy 4 fields.
* `expand(w, h)`: `x -= w; y -= h; width += w*2.0; height += h*2.0`.

Rust: `struct Rect { x: f64, y: f64, width: f64, height: f64 }`; `Default` = zeros, plus an `is_set` notion only if
the `undefined` → NaN propagation of 9.5 is to be reproduced (not observable for this model).

### 3.7 `CubismTargetPoint` (face direction smoothing)

Module constants: `FrameRate = 30`, `Epsilon = 0.01`. Fields (f64, all 0.0): `_faceTargetX, _faceTargetY, _faceX,
_faceY, _faceVX, _faceVY, _lastTimeSeconds, _userTimeSeconds`.

`update(deltaTimeSeconds)` — copy exactly (including the shadowing of `maxV` inside the block):
```
_userTimeSeconds += deltaTimeSeconds
faceParamMaxV = 40.0 / 10.0                         // 4.0   (comment: "7.5秒間に40分移動(5.3/sc)")
maxV = (faceParamMaxV * 1.0) / FrameRate            // 0.1333…
if (_lastTimeSeconds == 0.0) { _lastTimeSeconds = _userTimeSeconds; return }
deltaTimeWeight = (_userTimeSeconds - _lastTimeSeconds) * FrameRate
_lastTimeSeconds = _userTimeSeconds
timeToMaxSpeed = 0.15
frameToMaxSpeed = timeToMaxSpeed * FrameRate        // 4.5
maxA = (deltaTimeWeight * maxV) / frameToMaxSpeed
dx = _faceTargetX - _faceX ; dy = _faceTargetY - _faceY
if (|dx| <= Epsilon && |dy| <= Epsilon) return
d = sqrt(dx*dx + dy*dy)
vx = (maxV * dx) / d ; vy = (maxV * dy) / d
ax = vx - _faceVX ; ay = vy - _faceVY
a = sqrt(ax*ax + ay*ay)
if (a < -maxA || a > maxA) { ax *= maxA / a ; ay *= maxA / a }
_faceVX += ax ; _faceVY += ay
{ maxV2 = 0.5 * (sqrt(maxA*maxA + 16.0*maxA*d - 8.0*maxA*d) - maxA)     // literally +16 then -8
  curV = sqrt(_faceVX*_faceVX + _faceVY*_faceVY)
  if (curV > maxV2) { _faceVX *= maxV2 / curV ; _faceVY *= maxV2 / curV } }
_faceX += _faceVX ; _faceY += _faceVY
```
`getX()`, `getY()` return `_faceX/_faceY`; `set(x, y)` sets the targets. Note: while `_userTimeSeconds` stays 0
(zero deltas) the early return repeats.

V1: one instance per model (`CubismUserModel._dragManager`), updated every model update by
`CubismLookUpdater.onLateUpdate` (execution order 400) which feeds `getX()/getY()` into `CubismLook`. Targets are only
changed by `LAppModel.setDragging` ← `LAppLive2DManager.onDrag`, which V1 never calls → target stays (0,0) → face stays
0. Implement anyway (the player may later use it).

### 3.8 `CubismMath` items used here

`Epsilon = 0.00001`; `abs = Math.abs`; `sqrt = Math.sqrt`;
`clamp(val, min, max)`: `if (val < min) return min; else if (max < val) return max; return val` (NaN passes through).

### 3.9 The MVP V1 feeds to the renderer (per frame)

`LAppLive2DManager.onUpdate()` (BUNDLE 16465–16509) + `LAppModel.draw(matrix)` (16296–16305):
```
projection = new CubismMatrix44()              // identity, fresh every frame
if (model.getModel()) {
   if (model.getModel().getCanvasWidth() > 1 && width < height) {   // FALSE for this model (canvas width = 1.0)
       model.getModelMatrix().setWidth(2); projection.scale(1, width / height)
   } else {
       projection.scale(height / width, 1)                             // always taken for this model
   }
   if (this._viewMatrix != null) projection.multiplyByMatrix(this._viewMatrix)   // identity in V1
}
model.update()                                                          // LAppModel.update → … → CubismModel.update()
if (this._parameterDriver) {                                            // set by ReamaCubismPlayer (always non-null while running)
   this._parameterDriver(model)
   if (model.getModel()) model.getModel().update()                      // second CubismModel.update() (user-added)
}
if (width < height) { projection.scaleRelative(3.3, 3.3); projection.translateY(-2.1) }   // "Mobile"
else                { projection.scaleRelative(4.1, 4.1); projection.translateY(-2.75) }  // "Desktop"
model.draw(projection):  if (state == CompleteSetup) { projection.multiplyByMatrix(modelMatrix);
                                                      renderer.setMvpMatrix(projection); doDraw() }
```
(`width`/`height` are `canvas.width`/`canvas.height` in device pixels; `width < height` is strict, so a square canvas
uses the Desktop constants.) QUIRK (not reachable for this model): the portrait branch's `setWidth(2)` permanently
changes the model matrix and is never undone on rotation back. The `_viewMatrix` here is
`LAppLive2DManager._viewMatrix` (a plain `CubismMatrix44`) which `LAppView.render()` overwrites every frame with the
16 elements of `LAppView._viewMatrix` via `setViewMatrix(m)`.

Resulting MVP (all other elements identity; exact f32 values computed with a faithful `Float32Array` replica,
`.../render-spec-probe/mvp.cjs`):

| canvas (px) | tr[0] | tr[5] | tr[12] | tr[13] |
|---|---|---|---|---|
| 1280×720 | 3.2287497520446777 | 5.739999771118164 | 0 | -2.75 |
| 1920×1080 | 3.2287497520446777 | 5.739999771118164 | 0 | -2.75 |
| 2560×1440 | 3.2287497520446777 | 5.739999771118164 | 0 | -2.75 |
| 1366×768 | 3.2271740436553955 | 5.739999771118164 | 0 | -2.75 |
| 800×800 | 5.739999771118164 | 5.739999771118164 | 0 | -2.75 |
| 390×844 | 9.998153686523438 | 4.619999885559082 | 0 | -2.0999999046325684 |
| 1080×1920 | 8.213333129882812 | 4.619999885559082 | 0 | -2.0999999046325684 |

I.e. Desktop: `x_ndc = 5.74·(H/W)·x`, `y_ndc = 5.74·y − 2.75`; Mobile: `x_ndc = 4.62·(H/W)·x`, `y_ndc = 4.62·y − 2.1`
(model units; model y ranges about −0.715…0.697, so only the bust/face region is on screen). These numbers belong to
the player subsystem but are listed because they are the renderer's main input and a good end-to-end test.

### 3.10 Coordinate systems summary

| Space | Range / orientation | Produced by |
|---|---|---|
| Model space | units = canvas px / PixelsPerUnit; origin = canvas origin; **y up** | Core `vertexPositions` |
| Clip/NDC (main) | `u_matrix * pos` | MVP (3.9) |
| Mask buffer NDC | `[-1,1]²` of the 256² mask texture | `_matrixForMask` (9.7) |
| Mask texture UV | `[0,1]²`, (0,0) = bottom-left texel row in GL convention | `_matrixForDraw` (9.7) |
| Mesh UV | Core `vertexUvs`; shader uses `(u, 1 - v)` | vertex shaders |
| Image texture | uploaded without `UNPACK_FLIP_Y_WEBGL`; first image row at t = 0 | texture manager (12.3) |

---

## 4. Core data the renderer consumes (model accessor contract)

The renderer only talks to the model through `CubismModel` accessors that read the Core's typed arrays. The Rust
renderer should depend on a trait (section 15, `RenderModel`) with exactly these semantics.

### 4.1 Accessors

| Accessor (TS) | Core source | Type / semantics |
|---|---|---|
| `getDrawableCount()` | `drawables.count` | int |
| `getRenderOrders()` | `model.getRenderOrders()` (Core 5.3+ API; one entry per drawable **then** per offscreen) | `Int32Array`; `renderOrders[objectIndex] = drawing position` |
| `getDrawableTextureIndex(i)` | `drawables.textureIndices[i]` | int |
| `getDrawableVertexCount(i)` | `drawables.vertexCounts[i]` | int |
| `getDrawableVertices(i)` = `getDrawableVertexPositions(i)` | `drawables.vertexPositions[i]` | `Float32Array` (2 floats per vertex, `Constant.vertexOffset = 0`, `Constant.vertexStep = 2`) |
| `getDrawableVertexUvs(i)` | `drawables.vertexUvs[i]` | `Float32Array` (2 per vertex) |
| `getDrawableVertexIndexCount(i)` | `drawables.indexCounts[i]` | int |
| `getDrawableVertexIndices(i)` | `drawables.indices[i]` | `Uint16Array` |
| `getDrawableOpacity(i)` | `drawables.opacities[i]` | f32 |
| `getDrawableMultiplyColor(i)` / `getDrawableScreenColor(i)` | `drawables.multiplyColors[i*4..i*4+4]` / `screenColors[...]` | RGBA f32. **QUIRK (harmless):** the TS caches results in an array filled with ONE shared `CubismTextureColor` object (`.fill(new CubismTextureColor())`), so every call overwrites and returns the same object; callers read it immediately. Rust: return by value. |
| `getDrawableParentPartIndex(i)` | `drawables.parentPartIndices[i]` | int, `-1` = none |
| `getDrawableMasks()` / `getDrawableMaskCounts()` | `drawables.masks` (array of `Int32Array`) / `drawables.maskCounts` | mask drawable indices of each drawable |
| `getDrawableBlendMode(i)` | `constantFlags[i]` | `hasBlendAdditiveBit` → Additive(1); else `hasBlendMultiplicativeBit` → Multiplicative(2); else Normal(0) |
| `getDrawableColorBlend(i)` | `drawables.blendModes[i] & 0xff` | cached after first call (cache sentinel `-1`) |
| `getDrawableAlphaBlend(i)` | `(drawables.blendModes[i] >> 8) & 0xff` | cached after first call |
| `getDrawableInvertedMaskBit(i)` | `constantFlags[i]` | `hasIsInvertedMaskBit` |
| `getDrawableCulling(i)` | override flags else `constantFlags[i]` | `if (_isOverriddenCullings \|\| per-drawable override) return userCulling.isCulling; return !hasIsDoubleSidedBit(constantFlags[i])` (overrides are never set in V1; note that `initialize()` stores ONE shared `CullingData` object in every slot) |
| `getDrawableDynamicFlagIsVisible(i)` | `dynamicFlags[i]` | `hasIsVisibleBit` |
| `getDrawableDynamicFlagVertexPositionsDidChange(i)` | `dynamicFlags[i]` | `hasVertexPositionsDidChangeBit` |
| `isUsingMasking()` | `drawables.maskCounts` | true if any `maskCounts[d] > 0` |
| `isUsingMaskingForOffscreen()` | `offscreens.maskCounts` | true if any `> 0` for `d < getOffscreenCount()` |
| `isBlendModeEnabled()` | computed once in `initialize()` (4.3) | bool |
| `getPixelsPerUnit()` | `canvasinfo.PixelsPerUnit` (0.0 if no model) | |
| `getCanvasWidth()` / `getCanvasHeight()` | `CanvasWidth / PixelsPerUnit`, `CanvasHeight / PixelsPerUnit` | |
| `getOffscreenCount()` | `offscreens.count` | |
| `getOffscreenOwnerIndices()` | `offscreens.ownerIndices` | owner **part** index per offscreen |
| `getOffscreenOpacity(o)` | `offscreens.opacities[o]` (returns 1.0 if `o < 0 \|\| o >= count`) | |
| `getOffscreenColorBlend/AlphaBlend(o)` | `offscreens.blendModes[o] & 0xff` / `>> 8 & 0xff` (cached) | |
| `getOffscreenInvertedMask(o)` | `hasIsInvertedMaskBit(offscreens.constantFlags[o])` | |
| `getOffscreenCulling(o)` | override else `!hasIsDoubleSidedBit(offscreens.constantFlags[o])` | |
| `getOffscreenMasks()` / `getOffscreenMaskCounts()` | `offscreens.masks` / `offscreens.maskCounts` | |
| `getOffscreenMultiplyColor/ScreenColor(o)` | `offscreens.multiplyColors/screenColors` | RGBA |
| `getPartParentPartIndices()` | `parts.parentIndices` | `-1` = `NoParentIndex` |
| `getPartsHierarchy()` | built in `setupPartsHierarchy()` | `[{objects, childDrawObjects:{drawableIndices, offscreenIndices}}]` per part |
| `getOverrideMultiplyAndScreenColor()` | `CubismModelMultiplyAndScreenColor` | 4.4 |

Core bit helpers (verified with `bits.cjs` against Core 6.0.1):

| Constant flags | bit | | Dynamic flags | bit |
|---|---|---|---|---|
| `hasBlendAdditiveBit` | 1 | | `hasIsVisibleBit` | 1 |
| `hasBlendMultiplicativeBit` | 2 | | `hasVisibilityDidChangeBit` | 2 |
| `hasIsDoubleSidedBit` | 4 | | `hasOpacityDidChangeBit` | 4 |
| `hasIsInvertedMaskBit` | 8 | | `hasDrawOrderDidChangeBit` | 8 |
| | | | `hasRenderOrderDidChangeBit` | 16 |
| | | | `hasVertexPositionsDidChangeBit` | 32 |
| | | | `hasBlendColorDidChangeBit` | 64 |

Core blend constants: `ColorBlendType_Normal 0, AddCompatible 1, MultiplyCompatible 2, Add 3, AddGlow 4, Darken 5,
Multiply 6, ColorBurn 7, LinearBurn 8, Lighten 9, Screen 10, ColorDodge 11, Overlay 12, SoftLight 13, HardLight 14,
LinearLight 15, Hue 16, Color 17`; `AlphaBlendType_Over 0, Atop 1, Out 2, ConjointOver 3, DisjointOver 4`.

### 4.2 Per-frame dynamic flag semantics (Core 6.0.1, measured)

`CubismModel.update()` = `this._model.update(); this._model.drawables.resetDynamicFlags();`.
Measured with `flags.cjs` (histograms over the 49 drawables):

| Step | dynamicFlags |
|---|---|
| fresh model, before any update | all `127` |
| after 1st `update()` | all `127` |
| after `resetDynamicFlags()` | all `127` (**no immediately visible effect**) |
| after 2nd `update()` (no parameter change) | all `33` |
| after `resetDynamicFlags()` | all `33` |
| after `update()` with `ParamAngleX = 10` | all `33` |
| V1 frame (`update`+`reset` twice) with or without parameter changes | all `33` at draw time |

So in Core 6.0.1 `resetDynamicFlags()` does not clear the flags seen by the renderer; every `update()` recomputes
them, and **`VertexPositionsDidChange` is set by every `update()` even if nothing changed**. Consequences:

* `IsVisible` is checked per drawable per frame in `drawDrawable` (skip if false). With this model all 49 are visible
  at the default pose; parameters/part opacity can change that, so always read it.
* `VertexPositionsDidChange` is checked **only** for mask drawables during mask generation (skip drawing that mask
  drawable if false). Main drawables ignore it (vertices are re-uploaded with `bufferData(DYNAMIC_DRAW)` on every
  draw).
* `VisibilityDidChange`, `OpacityDidChange`, `DrawOrderDidChange`, `RenderOrderDidChange`, `BlendColorDidChange` are
  never consulted by the renderer. Opacity, colors and render orders are simply re-read every frame.
* Mask-drawable visibility is NOT checked: an invisible mask drawable still writes into the mask.
* **Porting rule:** call the Core's `update()` (and `resetDynamicFlags()`, harmless) every frame before drawing, like
  V1. If the port ever skipped `update()` on "idle" frames, `VertexPositionsDidChange` semantics could change and the
  lazily-cleared mask buffer (9.4) would wipe masks whose drawables are skipped (masked parts vanish, inverted-mask
  parts become fully visible).

### 4.3 Model-level derived flags

`CubismModel.initialize()` (blend detection, FW/model/cubismmodel.ts 1606–1629):
```
if (getOffscreenCount() > 0) _isBlendModeEnabled = true
else for i in 0..drawableCount:
    c = getDrawableColorBlend(i); a = getDrawableAlphaBlend(i)
    if (!(c == Normal(0) && a == Over(0)) && c != AddCompatible(1) && c != MultiplyCompatible(2)) { _isBlendModeEnabled = true; break }
```
This model: blendModes ∈ {0, 1} → `false`.

### 4.4 Multiply / screen color resolution (`CubismModelMultiplyAndScreenColor`)

`initialize(partCount, drawableCount, offscreenCount)` creates per-object `ColorData{isOverridden:false,
color}` with multiply default `(1,1,1,1)` and screen default `(0,0,0,1)`.
`getDrawableMultiplyColor(i)`: if `i` out of range → warn `getDrawableMultiplyColor: index is out of range.
index=${index}, valid range=[0, ${max}].` (CubismLogWarning, suppressed in V1) and return `(1,1,1,1)`; else if
`_isOverriddenModelMultiplyColors || _userDrawableMultiplyColors[i].isOverridden` → the user color; else the Core
color (4.1). `getDrawableScreenColor(i)` identical with default `(0,0,0,1)`. Offscreen variants identical with
`getOffscreenCount()`. V1 never enables overrides → Core colors. (Part-level setters propagate to children; not used by
V1 — belongs to the model subsystem.)

### 4.5 Parts hierarchy (needed only for offscreens)

`setupPartsHierarchy()`: one `CubismModelPartInfo` per part; each part with a parent is appended (as type Parts) to
its parent's `objects`; each drawable with a parent part is appended (type Drawable) to that part's `objects`; then
`getPartChildDrawObjects(i)` for every part recursively flattens: drawables → `drawableIndices`; child parts → their
flattened `drawableIndices`/`offscreenIndices`, plus the child part's own offscreen index
(`parts.offscreenIndices[childPart]`) if not `-1`. Already-flattened parts are skipped (non-empty lists).

---

## 5. Renderer types and lifecycle

### 5.1 Enums and small types

```ts
enum CubismBlendMode   { CubismBlendMode_Normal = 0, CubismBlendMode_Additive = 1, CubismBlendMode_Multiplicative = 2 }
enum DrawableObjectType{ DrawableObjectType_Drawable = 0, DrawableObjectType_Offscreen = 1 }
enum CubismColorBlend  { ColorBlend_None = -1, ColorBlend_Normal = 0, ColorBlend_AddGlow = 4, ColorBlend_Add = 3,
                         ColorBlend_Darken = 5, ColorBlend_Multiply = 6, ColorBlend_ColorBurn = 7, ColorBlend_LinearBurn = 8,
                         ColorBlend_Lighten = 9, ColorBlend_Screen = 10, ColorBlend_ColorDodge = 11, ColorBlend_Overlay = 12,
                         ColorBlend_SoftLight = 13, ColorBlend_HardLight = 14, ColorBlend_LinearLight = 15,
                         ColorBlend_Hue = 16, ColorBlend_Color = 17,
                         ColorBlend_AddCompatible = 1, ColorBlend_MultiplyCompatible = 2 }      // declaration order matters, see 7.2
enum CubismAlphaBlend  { AlphaBlend_None = -1, AlphaBlend_Over = 0, AlphaBlend_Atop = 1, AlphaBlend_Out = 2,
                         AlphaBlend_ConjointOver = 3, AlphaBlend_DisjointOver = 4 }
enum ShaderNames       { SetupMask = 0, NormalPremultipliedAlpha = 1, NormalMaskedPremultipliedAlpha = 2,
                         NomralMaskedInvertedPremultipliedAlpha = 3 /* sic */, AddPremultipliedAlpha = 4,
                         AddMaskedPremultipliedAlpha = 5, AddMaskedPremultipliedAlphaInverted = 6,
                         MultPremultipliedAlpha = 7, MultMaskedPremultipliedAlpha = 8,
                         MultMaskedPremultipliedAlphaInverted = 9, ShaderCount = 10 }
enum ShaderType        { ShaderType_Normal = 0, ShaderType_Masked = 1, ShaderType_MaskedInverted = 2, ShaderType_Count = 3 }
const s_invalidValue = -1;
const NoParentIndex = -1, NoOffscreenIndex = -1;
```

`CubismTextureColor { r, g, b, a }` — constructor defaults `(1.0, 1.0, 1.0, 1.0)`; plain f64 fields.

`CubismClippingContext` (abstract base; see 9.2 for fields).

### 5.2 `CubismRenderer` (abstract base)

Fields and constructor defaults (`protected constructor(width, height)`):

| Field | Default | Meaning |
|---|---|---|
| `_modelRenderTargetWidth/Height` | ctor args | size of the "model render target" (= canvas size, updated each frame by `setRenderState`) |
| `_isCulling` | `false` | current culling state used by `drawMeshWebGL` |
| `_isPremultipliedAlpha` | `false` | must be set `true` (V1 does, 12.2); `false` only logs `NoPremultipliedAlpha is not allowed` |
| `_anisotropy` | `0.0` | |
| `_model` | `null` | |
| `_modelColor` | `CubismTextureColor(1,1,1,1)` | |
| `_useHighPrecisionMask` | `false` | |
| `_mvpMatrix4x4` | identity `CubismMatrix44` | |

Methods:
* `static create()` → `null`; `static delete(r)` → no-op (sets local to null). `static staticRelease` is assigned by
  the WebGL module to `() => CubismRenderer_WebGL.doStaticRelease()`.
* `initialize(model)`: `_model = model`; if `model.isBlendModeEnabled()` → `useHighPrecisionMask(true)` and
  `CubismLogInfo('This model uses a high-resolution mask because it operates in blend mode.')`.
* `drawModel(shaderPath = null)`: `if (getModel() == null) return;` → `doDrawModel(shaderPath)`. (`saveProfile()` /
  `restoreProfile()` calls around it are commented out "WebGL最適化のため" → never executed.)
* `setMvpMatrix(m)`: `_mvpMatrix4x4.setMatrix(m.getArray())` (copy). `getMvpMatrix()` returns the live matrix.
* `setModelColor(r, g, b, a)`: each component `CubismMath.clamp(v, 0.0, 1.0)`.
* `getModelColor()`: `JSON.parse(JSON.stringify(_modelColor))` (a plain-object deep copy; NaN would become `null` →
  arithmetic 0; unreachable since V1 never sets the color).
* `getModelColorWithOpacity(opacity)`: `c = getModelColor(); c.a *= opacity; if (isPremultipliedAlpha()) { c.r *= c.a;
  c.g *= c.a; c.b *= c.a; } return c`. V1: `(o, o, o, o)` for drawable opacity `o`.
* `setIsPremultipliedAlpha(b)`, `isPremultipliedAlpha()`, `setIsCulling(b)`, `isCulling()`, `setAnisotropy(n)`,
  `getAnisotropy()`, `getModel()`, `useHighPrecisionMask(b)`, `isUsingHighPrecisionMask()`,
  `setRenderTargetSize(w, h)` (sets `_modelRenderTargetWidth/Height`).
* Abstract: `beforeDrawModelRenderTarget()`, `afterDrawModelRenderTarget()`, `doDrawModel(shaderPath)`,
  `saveProfile()`, `restoreProfile()`.

### 5.3 `CubismRenderer_WebGL` fields

| Field | Init | Notes |
|---|---|---|
| `_textures: Map<number, WebGLTexture>` | empty | model texture number → GL texture (`bindTexture`) |
| `_sortedObjectsIndexList`, `_sortedObjectsTypeList` | `[]` | sized to `drawableCount + offscreenCount` in `initialize` (filled with 0) |
| `_rendererProfile` | `new CubismRendererProfile_WebGL()` | unused (11.3) |
| `_drawableClippingManager` | `null` | created in `initialize` iff `model.isUsingMasking()` |
| `_offscreenClippingManager` | `null` | created iff `model.isUsingMaskingForOffscreen()` |
| `_clippingContextBufferForMask` | `null` | non-null ⇔ "currently generating a mask" (`isGeneratingMask()`) |
| `_clippingContextBufferForDraw` | `null` | clip context of the drawable being drawn (null ⇒ unmasked) |
| `_clippingContextBufferForOffscreen` | (unset) | same for offscreens |
| `_bufferData = {vertex, uv, index}` | all `null` | three GL buffers, created lazily on first use and **shared by all drawables** |
| `_modelRenderTargets: CubismOffscreenRenderTarget_WebGL[]` | `[]` | 3 targets only if `isBlendModeEnabled()` |
| `_drawableMasks: CubismRenderTarget_WebGL[]` | `[]` | one per mask render texture (`getRenderTextureCount()`) |
| `_offscreenMasks` | `[]` | 5.3 |
| `_offscreenList` | `[]` | one `CubismOffscreenRenderTarget_WebGL` per model offscreen (5.3) |
| `_currentFbo` | `null` | FBO the current object is drawn into |
| `_currentOffscreen` | (unset → `null` in `drawObjectLoop`) | 5.3 |
| `_modelRootFbo` | (unset) | FBO bound when `drawObjectLoop` started |
| `_extension` | (unset) | anisotropic extension object or `null` |
| `gl` | (unset) | the context |

### 5.4 Constructor, `initialize`, `startUp`

`constructor(width, height)`: `super(width, height)` then the field inits above (`this._textures = new Map()`, etc.).

`initialize(model, maskBufferCount = 1)` (no GL calls — "WebGLコンテキストが初期化されていない可能性があるため"):
1. If `model.isUsingMasking()`: `_drawableClippingManager = new CubismClippingManager_WebGL();
   _drawableClippingManager.initializeForDrawable(model, maskBufferCount)` (9.3).
2. If `model.isUsingMaskingForOffscreen()`: `_offscreenClippingManager = new CubismClippingManager_WebGL();
   _offscreenClippingManager.initializeForOffscreen(model, maskBufferCount)`.
3. `updateSize(_sortedObjectsIndexList, drawableCount + (model.getOffscreenCount ? model.getOffscreenCount() : 0), 0,
   true)` and the same for `_sortedObjectsTypeList` (fills new slots with `0`).
4. `super.initialize(model)` (sets `_model`, may force high precision).

`startUp(gl)` (must be called after `initialize`; V1 calls it right after `createRenderer` + `setupTextures`):
1. `this.gl = gl`; `_drawableClippingManager?.setGL(gl)`; `_offscreenClippingManager?.setGL(gl)`.
2. `CubismShaderManager_WebGL.getInstance().setGlContext(gl)` (creates the per-context `CubismShader_WebGL` if absent);
   `_rendererProfile.setGl(gl)`.
3. `_extension = gl.getExtension('EXT_texture_filter_anisotropic') || gl.getExtension('WEBKIT_EXT_texture_filter_anisotropic')
   || gl.getExtension('MOZ_EXT_texture_filter_anisotropic')`.
4. If `_model.isUsingMasking()`: `_drawableMasks.length = _drawableClippingManager.getRenderTextureCount()`; for each:
   `new CubismRenderTarget_WebGL().createRenderTarget(gl, maskSize, maskSize, this._currentFbo /* null */)` (10.1).
5. If `_model.isBlendModeEnabled()`:
   * `_modelRenderTargets.length = 0`, then 3 × `new CubismOffscreenRenderTarget_WebGL().createRenderTarget(gl,
     _modelRenderTargetWidth, _modelRenderTargetHeight, _currentFbo)` (comment: "TextureBarrierの代替用にオフスクリーンを2つ作成する",
     `createSize = 3`). Index 0 = model render target, 1 = copy of the destination for blending, 2 = copy of an
     offscreen's content.
   * If `isUsingMaskingForOffscreen()`: `_offscreenMasks` = `getRenderTextureCount()` × mask-size render targets.
   * If `offscreenCount > 0`: `_offscreenList = new Array(offscreenCount)` of `CubismOffscreenRenderTarget_WebGL` with
     `setOffscreenIndex(i)`; then `setupParentOffscreens(model, offscreenCount)` (6.5).
6. `gl.bindFramebuffer(FRAMEBUFFER, this._currentFbo)` (= `null`; needed because `createRenderTarget` leaves the new
   FBO bound, 10.1).

### 5.5 Textures, sizes, clipping-mask size

* `bindTexture(modelTextureNo, glTexture)`: `_textures.set(no, tex)`. `getBindedTextures()` returns the map.
* `getClippingMaskBufferSize()`: `isUsingMasking() ? manager.getClippingMaskBufferSize() : -1`.
* `getRenderTextureCount()`: `isUsingMasking() ? manager.getRenderTextureCount() : -1`.
* `getModelRenderTarget(i)`, `getDrawableMaskBuffer(i)`, `getOffscreenMaskBuffer(i)`: plain array access.
* `setClippingMaskBufferSize(size)` ("マスク用のFrameBufferを破棄、再作成する為処理コストは高い"):
  `if (!_model.isUsingMasking()) return;` `rtc = manager.getRenderTextureCount()`; `manager.release()`; create a new
  `CubismClippingManager_WebGL`; `setClippingMaskBufferSize(size)`; `initializeForDrawable(getModel(), rtc)`.
  The mask render targets are re-created lazily by the size check in `doDrawModel` (6.1 step 4).
  **BUG:** the new manager never receives `setGL(gl)` (only `startUp` does that), so the next
  `setupClippingContext` dereferences `this.gl` = undefined → TypeError. V1 never calls this method. Rust: pass the GL
  handle explicitly / keep it in the renderer so the bug cannot occur.

### 5.6 `setRenderState`, `preDraw`, `loadShaders`

`setRenderState(fbo, viewport)` — called by the host every frame before `drawModel` (12.4):
`bindFramebuffer(FRAMEBUFFER, fbo)`; `viewport(v[0], v[1], v[2], v[3])`; if `_modelRenderTargetWidth != v[2] ||
_modelRenderTargetHeight != v[3]` → store `v[2]`, `v[3]`.

`preDraw()` ("描画開始時の追加処理"), exact call order:
```
gl.disable(SCISSOR_TEST); gl.disable(STENCIL_TEST); gl.disable(DEPTH_TEST);
gl.frontFace(CW);                       // comment "カリング（1.0beta3）"; overridden to CCW before every draw
gl.enable(BLEND); gl.colorMask(true, true, true, true);
gl.bindBuffer(ARRAY_BUFFER, null); gl.bindBuffer(ELEMENT_ARRAY_BUFFER, null);
if (getAnisotropy() > 0.0 && _extension) {
  for (i = 0; i < _textures.size; ++i) {      // assumes keys 0..size-1
    gl.bindTexture(TEXTURE_2D, _textures.get(i));
    gl.texParameterf(TEXTURE_2D, _extension.TEXTURE_MAX_ANISOTROPY_EXT /* 0x84FE */, getAnisotropy());
  }
}
```

`loadShaders(shaderPath = null)`:
```
if (gl == null) { CubismLogError("'gl' is null. WebGLRenderingContext is required.\nPlease call 'CubimRenderer_WebGL.startUp' function."); return }
shader = CubismShaderManager_WebGL.getInstance().getShader(gl)
if (shader._shaderSets.length == 0 || !shader._isShaderLoaded) {
    if (shaderPath != null) shader.setShaderPath(shaderPath)
    shader.generateShaders()                  // no-op while a load is in flight (7.3)
}
```
Called explicitly once by `LAppModel` after `startUp` (`loadShaders(ShaderPath)`) and implicitly at the start of every
`doDrawModel`.

### 5.7 `release`, `doStaticRelease`

`release()`: `_drawableClippingManager?.release()` and null it; `if (gl == null) return`; delete the three
`_bufferData` buffers and null `_bufferData`; `_textures = null`; destroy every valid `_modelRenderTargets[i]`,
`_drawableMasks[i]`, `_offscreenMasks[i]`, `_offscreenList[i]` (`destroyRenderTarget()` if `isValid()`), set their
lengths to 0 and null them; null both clipping managers, both clipping-context buffers, `_rendererProfile`, sorted
lists, `_currentFbo`, `_model`, `gl`. (The model's GL textures are owned by the texture manager, not deleted here.)

`static doStaticRelease()`: `CubismShaderManager_WebGL.deleteInstance()` → releases every per-context shader (deletes
all programs) and drops the singleton. Called by `CubismFramework.dispose()` (V1: when the last player is destroyed,
`releaseFramework()`).

### 5.8 Lifecycle order in V1

1. Model assets loaded (model subsystem). When all motions are loaded (or there are none — this model has none, so
   immediately in `loadCubismMotion`): `_state = LoadTexture`, `createRenderer(canvas.width, canvas.height)` →
   `new CubismRenderer_WebGL(w, h)` + `initialize(model, 1)` (deletes any previous renderer first).
2. `setupTextures()`: for each texture: start async `createTextureFromPngFile(path, usePremultiply = true, onLoad)` and
   **synchronously** `setIsPremultipliedAlpha(true)`; `_state = WaitLoadTexture`. `onLoad` →
   `renderer.bindTexture(no, textureInfo.id)`; when all loaded → `_state = CompleteSetup`.
3. `startUp(gl)`.
4. `loadShaders(ShaderPath)` → starts the async shader fetch + compile.
5. Per frame (only once `_state == CompleteSetup`): `setMvpMatrix` → `setRenderState(null, [0,0,w,h])` →
   `drawModel(ShaderPath)`.
Frames that run before the shaders finish loading draw nothing (each `drawMeshWebGL` returns early, 6.4) but still run
the mask-buffer setup (viewport, FBO bind, clear to white).

---

## 6. Per-frame pipeline

### 6.1 `doDrawModel(shaderPath)` step by step

```
1. loadShaders(shaderPath)
2. beforeDrawModelRenderTarget()                      // no-op unless blend mode (6.5)
3. lastFbo = gl.getParameter(FRAMEBUFFER_BINDING)     // null = default framebuffer (V1)
   lastViewport = gl.getParameter(VIEWPORT)           // Int32Array [0,0,w,h] (set by setRenderState)
4. if (_drawableClippingManager != null) {
       preDraw()
       for i in 0..manager.getRenderTextureCount():
           if (_drawableMasks[i].getBufferWidth() != maskSize || getBufferHeight() != maskSize)
               _drawableMasks[i].createRenderTarget(gl, maskSize, maskSize, lastFbo)   // leaves new FBO bound!
       if (isUsingHighPrecisionMask()) manager.setupMatrixForHighPrecision(model, false)          // 9.9
       else manager.setupClippingContext(model, this, lastFbo, lastViewport, Drawable)           // 9.4
   }
5. if (_offscreenClippingManager != null) {
       preDraw()
       same size check over _offscreenMasks
       if (isUsingHighPrecisionMask()) manager.setupMatrixForOffscreenHighPrecision(model, false, getMvpMatrix())
       else manager.setupClippingContext(model, this, lastFbo, lastViewport, Offscreen)
   }
6. preDraw()                                          // comment: "上記クリッピング処理内でも一度PreDrawを呼ぶので注意!!"
7. drawObjectLoop(lastFbo)
8. afterDrawModelRenderTarget()                       // no-op unless blend mode
```
QUIRK: if a mask render target is (re)created in step 4 and then `setupClippingContext` returns early because no
context is in use, the newly created mask FBO stays bound and the model is drawn into it. Unreachable in V1 (size never
changes; all contexts always in use).

### 6.2 `drawObjectLoop(lastFbo)` and render order

```
drawableCount = model.getDrawableCount(); offscreenCount = model.getOffscreenCount(); total = drawableCount + offscreenCount
renderOrder = model.getRenderOrders()                 // re-read EVERY frame
_currentOffscreen = null; _currentFbo = lastFbo; _modelRootFbo = lastFbo
for i in 0..total:
    order = renderOrder[i]
    if (i < drawableCount) { sortedIndex[order] = i;                  sortedType[order] = Drawable }
    else                   { sortedIndex[order] = i - drawableCount;  sortedType[order] = Offscreen }
for i in 0..total: renderObject(sortedIndex[i], sortedType[i])
while (_currentOffscreen != null)
    submitDrawToParentOffscreen(_currentOffscreen.getOffscreenIndex(), Offscreen)
```
`renderObject(index, type)`: Drawable → `drawDrawable(index, _modelRootFbo)`; Offscreen → `addOffscreen(index)`;
otherwise `CubismLogError('Unknown object type: ' + objectType)`.

Draw sequence of this model (render order position → drawable index), constant in practice:
`18,17,12,11,40,41,10,16,19,20,21,22,23,46,47,48,25,24,28,29,30,31,32,45,43,44,26,27,33,34,35,36,37,38,39,15,14,13,9,42,8,7,6,5,4,3,2,1,0`
(raw `getRenderOrders()` = `48,47,46,45,44,43,42,41,40,38,6,3,2,37,36,35,7,1,0,8,9,10,11,12,17,16,26,27,18,19,20,21,22,28,29,30,31,32,33,34,4,5,39,24,25,23,13,14,15`).

### 6.3 `drawDrawable(drawableIndex, rootFbo)`

```
if (!model.getDrawableDynamicFlagIsVisible(i)) return
submitDrawToParentOffscreen(i, Drawable)                     // no-op without offscreens
clipContext = (_drawableClippingManager != null) ? manager.getClippingContextListForDraw()[i] : null
if (clipContext != null && isUsingHighPrecisionMask()) {     // ---- high-precision branch (not used by V1) ----
    if (clipContext._isUsing) {
        gl.viewport(0, 0, maskSize, maskSize)
        preDraw()
        getDrawableMaskBuffer(clipContext._bufferIndex).beginDraw(_currentFbo)
        gl.clearColor(1.0, 1.0, 1.0, 1.0); gl.clear(COLOR_BUFFER_BIT)
    }
    for each clipDrawIndex in clipContext._clippingIdList[0 .. _clippingIdCount]:
        if (!model.getDrawableDynamicFlagVertexPositionsDidChange(clipDrawIndex)) continue
        setIsCulling(model.getDrawableCulling(clipDrawIndex) != false)
        setClippingContextBufferForMask(clipContext)
        drawMeshWebGL(model, clipDrawIndex)
    getDrawableMaskBuffer(clipContext._bufferIndex).endDraw()      // QUIRK: runs even if !_isUsing (no beginDraw → rebinds a stale _oldFbo)
    setClippingContextBufferForMask(null)
    gl.viewport(0, 0, _modelRenderTargetWidth, _modelRenderTargetHeight)
    preDraw()
}
setClippingContextBufferForDrawable(clipContext)
setIsCulling(model.getDrawableCulling(i))
drawMeshWebGL(model, i)
```

### 6.4 `drawMeshWebGL(model, index)`

```
if (isCulling()) gl.enable(CULL_FACE) else gl.disable(CULL_FACE)
gl.frontFace(CCW)                      // "Cubism SDK OpenGLはマスク・アートメッシュ共にCCWが表面"; cullFace mode never set → default BACK
shader = CubismShaderManager_WebGL.getInstance().getShader(gl)
if (isGeneratingMask()) shader.setupShaderProgramForMask(this, model, index)        // 8.2
else                    shader.setupShaderProgramForDrawable(this, model, index)    // 8.1
if (!shader._isShaderLoaded) return    // NOTE in source: avoids drawing unmasked Cubism ≤5.2 models before load
gl.drawElements(TRIANGLES, model.getDrawableVertexIndexCount(index), UNSIGNED_SHORT, 0)
gl.useProgram(null)
setClippingContextBufferForDrawable(null)
setClippingContextBufferForMask(null)
```
(On the early-return path the clip-context buffers are NOT reset; harmless because callers set them before every call.)

### 6.5 Offscreen / blend-mode pipeline (Cubism 5.3; stub-able, specified for completeness)

`setupParentOffscreens(model, offscreenCount)`: for each offscreen `o`: `parent = null; owner = ownerIndices[o];
p = partParentIndices[owner]`; while `p != -1`: scan all `_offscreenList[k]`; if `ownerIndices[_offscreenList[k].getOffscreenIndex()] == p`
→ `parent = _offscreenList[k]`, break; if found break; `p = partParentIndices[p]`. Then
`_offscreenList[o].setParentPartOffscreen(parent)`.

`beforeDrawModelRenderTarget()`: if `_modelRenderTargets.length == 0` return; re-create any model render target whose
size differs from `_modelRenderTargetWidth/Height` (`createRenderTarget(gl, w, h, _currentFbo)`); then
`_modelRenderTargets[0].beginDraw()` (no restore FBO → remembers the currently bound one) and
`_modelRenderTargets[0].clear(0,0,0,0)`.

`afterDrawModelRenderTarget()`: if empty return; `_modelRenderTargets[0].endDraw()`;
`shader.setupShaderProgramForOffscreenRenderTarget(this)` (8.4); if `shader._isShaderLoaded`: create a temporary
index buffer, `bufferData(ELEMENT_ARRAY_BUFFER, Uint16Array[0,1,2,2,1,3], STATIC_DRAW)`, `drawElements(TRIANGLES, 6,
UNSIGNED_SHORT, 0)`, delete it; finally `useProgram(null)`.

`submitDrawToParentOffscreen(objectIndex, objectType)`:
```
if (_currentOffscreen == null || objectIndex == -1) return
currentOwner = ownerIndices[_currentOffscreen.getOffscreenIndex()]
if (currentOwner == -1) return
target = (type == Drawable) ? model.getDrawableParentPartIndex(objectIndex)
       : (type == Offscreen) ? partParentIndices[ownerIndices[objectIndex]] : return
while (target != -1) { if (target == currentOwner) return; target = partParentIndices[target] }
drawOffscreen(_currentOffscreen)                       // composites the current offscreen into its parent target
submitDrawToParentOffscreen(objectIndex, objectType)   // recurse: maybe close further ancestors
```

`addOffscreen(offscreenIndex)`:
```
if (_currentOffscreen != null && _currentOffscreen.getOffscreenIndex() != offscreenIndex) {
    isParent = false; p = partParentIndices[ownerIndices[offscreenIndex]]
    curOwner = ownerIndices[_currentOffscreen.getOffscreenIndex()]
    while (p != -1) { if (p == curOwner) { isParent = true; break } p = partParentIndices[p] }
    if (!isParent) submitDrawToParentOffscreen(offscreenIndex, Offscreen)
}
off = _offscreenList[offscreenIndex]
if (off.getRenderTexture() == null || off.getBufferWidth() != _modelRenderTargetWidth
    || off.getBufferHeight() != _modelRenderTargetHeight || off.getUsingRenderTextureState())
    off.setOffscreenRenderTarget(gl, _modelRenderTargetWidth, _modelRenderTargetHeight, _currentFbo)   // 10.2
else off.startUsingRenderTexture()
old = off.getParentPartOffscreen(); off.setOldOffscreen(old)
oldFBO = old?.getRenderTexture() ?? null; if (oldFBO == null) oldFBO = _modelRootFbo
off.beginDraw(oldFBO); gl.viewport(0, 0, _modelRenderTargetWidth, _modelRenderTargetHeight); off.clear(0,0,0,0)
_currentOffscreen = off; _currentFbo = off.getRenderTexture()
```

`drawOffscreen(off)`: `o = off.getOffscreenIndex()`; `clipContext = _offscreenClippingManager ?
getClippingContextListForOffscreen()[o] : null`; high-precision branch identical in shape to 6.3 but with
`_offscreenClippingManager`, `getOffscreenMaskBuffer`, `getClippingMaskBufferSize()` of that manager (mask drawables are
still *drawables*: `getDrawableDynamicFlagVertexPositionsDidChange`, `getDrawableCulling`); then
`setClippingContextBufferForOffscreen(clipContext)`; `setIsCulling(model.getOffscreenCulling(o) != false)`;
`drawOffscreenWebGL(model, off)`.

`drawOffscreenWebGL(model, off)`:
```
cull enable/disable; gl.frontFace(CCW)
shader.setupShaderProgramForOffscreen(this, model, off)           // 8.3 (copies via blit, needs WebGL2)
off.endDraw()                                                      // rebinds off._oldFbo
_currentOffscreen = _currentOffscreen.getOldOffscreen()
_currentFbo = off.getOldFBO()
if (_currentFbo == null) { _currentOffscreen = _modelRenderTargets[0]; _currentFbo = _modelRenderTargets[0].getRenderTexture();
                           gl.bindFramebuffer(FRAMEBUFFER, _currentFbo) }
temp index buffer [0,1,2,2,1,3] STATIC_DRAW → drawElements(TRIANGLES, 6, UNSIGNED_SHORT, 0) → deleteBuffer
off.stopUsingRenderTexture(); gl.useProgram(null)
setClippingContextBufferForMask(null); setClippingContextBufferForOffscreen(null)
```
QUIRK: `drawOffscreenWebGL` draws even when the shader is not loaded (no `_isShaderLoaded` guard), unlike
`drawMeshWebGL`; and when `_currentFbo` becomes null it sets `_currentOffscreen` to `_modelRenderTargets[0]`, which
makes the `while (_currentOffscreen != null)` loop in `drawObjectLoop` call `submitDrawToParentOffscreen` with that
target's offscreen index (`-1` → returns immediately) — an **infinite loop** risk in the SDK if that state is ever
reached. Port 5.3 support only with a guard.

### 6.6 Golden GL call trace — one frame of THIS model (shaders loaded, 1280×720 canvas)

Host (12.4) before the renderer: `clearColor(0,0,0,0)`, `enable(DEPTH_TEST)`, `depthFunc(LEQUAL)`,
`clear(COLOR|DEPTH)`, `clearDepth(1)`, `enable(BLEND)`, `blendFunc(SRC_ALPHA, ONE_MINUS_SRC_ALPHA)`,
`useProgram(spriteProgram)`, `flush()`. Then `setRenderState`: `bindFramebuffer(FRAMEBUFFER, null)`,
`viewport(0,0,1280,720)`.

`drawModel`:
```
getParameter(FRAMEBUFFER_BINDING) → null ; getParameter(VIEWPORT) → [0,0,1280,720]
preDraw()                                               (#1)
— setupClippingContext —
viewport(0,0,256,256)
[beginDraw] getParameter(FRAMEBUFFER_BINDING) (restoreFbo was null) ; bindFramebuffer(FRAMEBUFFER, maskFbo0)
preDraw()                                               (#2)
for ctx in 0..6 (masks 18, 46, 45, 33, 28, 19):
   ctx0 only: clearColor(1,1,1,1); clear(COLOR_BUFFER_BIT)
   disable(CULL_FACE); frontFace(CCW)
   useProgram(P0 SetupMask); bindBuffer(ARRAY, vbo); bufferData(ARRAY, positions[mask], DYNAMIC_DRAW)
   enableVertexAttribArray(P0.a_position); vertexAttribPointer(P0.a_position, 2, FLOAT, false, 0, 0)
   bindBuffer(ARRAY, uvbo); activeTexture(TEXTURE0); bindTexture(TEXTURE_2D, modelTex0); uniform1i(P0.s_texture0, 0)
   bindBuffer(ARRAY, uvbo); bufferData(ARRAY, uvs[mask], DYNAMIC_DRAW)
   enableVertexAttribArray(P0.a_texCoord); vertexAttribPointer(P0.a_texCoord, 2, FLOAT, false, 0, 0)
   uniform4f(P0.u_channelFlag, channel color)           // R,R,G,G,B,A for ctx0..5
   uniformMatrix4fv(P0.u_clipMatrix, false, ctx._matrixForMask)
   uniform4f(P0.u_baseColor, x*2-1, y*2-1, right*2-1, bottom*2-1)   // layout rect in mask NDC
   bindBuffer(ELEMENT_ARRAY, ibo); bufferData(ELEMENT_ARRAY, indices[mask], DYNAMIC_DRAW)
   blendFuncSeparate(ZERO, ONE_MINUS_SRC_COLOR, ZERO, ONE_MINUS_SRC_ALPHA)
   drawElements(TRIANGLES, indexCount[mask], UNSIGNED_SHORT, 0); useProgram(null)
[endDraw] bindFramebuffer(FRAMEBUFFER, null)
viewport(0,0,1280,720)
preDraw()                                               (#3)
— drawObjectLoop — for i in draw sequence (6.2), 49 times:
   disable(CULL_FACE); frontFace(CCW)
   useProgram(P1 | P2 | P3)                              // unmasked | masked | masked+inverted (additive uses the same programs)
   bindBuffer(ARRAY, vbo); bufferData(ARRAY, positions[i], DYNAMIC_DRAW); enable+pointer(a_position)
   bindBuffer(ARRAY, uvbo); bufferData(ARRAY, uvs[i], DYNAMIC_DRAW); enable+pointer(a_texCoord)
   if masked: activeTexture(TEXTURE1); bindTexture(TEXTURE_2D, maskTex0); uniform1i(s_texture1, 1)
              uniformMatrix4fv(u_clipMatrix, false, ctx._matrixForDraw); uniform4f(u_channelFlag, channel color)
   activeTexture(TEXTURE0); bindTexture(TEXTURE_2D, modelTex0); uniform1i(s_texture0, 0)
   uniformMatrix4fv(u_matrix, false, MVP)
   uniform4f(u_baseColor, o, o, o, o)                    // o = opacity (premultiplied model color)
   uniform4f(u_multiplyColor, 1,1,1,1); uniform4f(u_screenColor, 0,0,0,1)
   bindBuffer(ELEMENT_ARRAY, ibo); bufferData(ELEMENT_ARRAY, indices[i], DYNAMIC_DRAW)
   blendFuncSeparate(ONE, ONE_MINUS_SRC_ALPHA, ONE, ONE_MINUS_SRC_ALPHA)   // additive drawables 21, 30: (ONE, ONE, ZERO, ONE)
   drawElements(TRIANGLES, indexCount[i], UNSIGNED_SHORT, 0); useProgram(null)
```
Program per drawable: P2 (masked) for 3, 9, 19, 20, 21, 22, 23, 28, 29, 30, 31, 32, 34, 35; P3 (masked+inverted) for
45, 46; P1 for the other 33. Additive blend state for 21 (ArtMesh30, masked) and 30 (ArtMesh53, masked).
Totals per frame: 55 `drawElements`, 3 `preDraw`, 1 mask clear, 2 FBO binds inside `drawModel` (+1 in `setRenderState`).

---

## 7. Shaders

### 7.1 Source files, URLs and loading

Module constants (FW/rendering/cubismshader_webgl.ts 21–43):
```
VertShaderSrcPath                               = 'vertshadersrc.vert'
VertShaderSrcMaskedPath                         = 'vertshadersrcmasked.vert'
VertShaderSrcSetupMaskPath                      = 'vertshadersrcsetupmask.vert'
FragShaderSrcSetupMaskPath                      = 'fragshadersrcsetupmask.frag'
FragShaderSrcPremultipliedAlphaPath             = 'fragshadersrcpremultipliedalpha.frag'
FragShaderSrcMaskPremultipliedAlphaPath         = 'fragshadersrcmaskpremultipliedalpha.frag'
FragShaderSrcMaskInvertedPremultipliedAlphaPath = 'fragshadersrcmaskinvertedpremultipliedalpha.frag'
VertShaderSrcCopyPath                           = 'vertshadersrccopy.vert'
FragShaderSrcCopyPath                           = 'fragshadersrccopy.frag'
FragShaderSrcColorBlendPath                     = 'fragshadersrccolorblend.frag'
FragShaderSrcAlphaBlendPath                     = 'fragshadersrcalphablend.frag'
VertShaderSrcBlendPath                          = 'vertshadersrcblend.vert'
FragShaderSrcBlendPath                          = 'fragshadersrcpremultipliedalphablend.frag'
ColorBlendPrefix = 'ColorBlend_' ; AlphaBlendPrefix = 'AlphaBlend_'
s_renderTargetVertexArray    = Float32Array[-1,-1,  1,-1,  -1,1,  1,1]
s_renderTargetUvArray        = Float32Array[ 0, 0,  1, 0,   0,1,  1,1]
s_renderTargetReverseUvArray = Float32Array[ 0, 1,  1, 1,   0,0,  1,0]
s_renderTargetIndexArray     = Uint16Array [0, 1, 2, 2, 1, 3]        // in cubismrenderer_webgl.ts
```

URL of each file = `shaderDir + fileName`, `shaderDir = this._shaderPath ?? this._defaultShaderPath`. In V1 both are
`"/resources/live2d/shaders/"` (origin-absolute, no base path), e.g. `/resources/live2d/shaders/vertshadersrc.vert`.

V1 fetch path (`loadShaders()` private async):
```
results = await Promise.all(shaderFiles.map(file =>
    this.loadShader(file.path)                         // = cachedFetchText(url) in V1
        .then(data => ({ prop: file.prop, data }))
        .catch(error => { console.error(`Error loading ${file.path} shader:`, error); return { prop: file.prop, data: '' } })))
results.forEach(r => this[r.prop] = r.data)            // _vertShaderSrc, _vertShaderSrcMasked, … (13 string fields)
```
The 13 `{path, prop}` pairs are in the order of the constant list above → props `_vertShaderSrc`,
`_vertShaderSrcMasked`, `_vertShaderSrcSetupMask`, `_fragShaderSrcSetupMask`, `_fragShaderSrcPremultipliedAlpha`,
`_fragShaderSrcMaskPremultipliedAlpha`, `_fragShaderSrcMaskInvertedPremultipliedAlpha`, `_vertShaderSrcCopy`,
`_fragShaderSrcCopy`, `_fragShaderSrcColorBlend`, `_fragShaderSrcAlphaBlend`, `_vertShaderSrcBlend`,
`_fragShaderSrcBlend`.

`cachedFetchText(url)` (user-added, BUNDLE 2718–2733): module-level `Map<url, Promise<string>>`; on miss:
`fetch(url).then(res => { if (!res.ok) throw new Error(`Failed to fetch ${url}: ${res.status}`); return res.text() })
.catch(err => { cache.delete(url); throw err })`, stored before it settles (concurrent callers share the promise);
failures are evicted so the next call refetches. `preloadDefaultAssets()` (BUNDLE 18604–18632, runs once when the
script loads and again via `runtime.preload`) warms the cache with all 13 shader URLs (`ShaderPath + name`, each with
`.catch(() => {})` on a derived promise).

**V2 recommendation (DEVIATION-OK):** embed the 13 files with `include_str!` (copy them byte-identically into
`amadeus-live2d/shaders/`, verify the md5 table in 0.2 in a unit test). This removes 13 requests and makes the first
frame drawable as soon as textures are bound (V1 shows nothing until fetch + compile finish). The V2 web server must
still serve `/resources/live2d/shaders/*` only if V1 pages are served side by side (not needed by V2 itself).

### 7.2 `CubismShader_WebGL` fields and constructor

Fields: `_shaderSets: CubismShaderSet[]`, `gl`, `_colorBlendMap: Map<number,string>`, `_alphaBlendMap`,
`_colorBlendValues: number[]`, `_alphaBlendValues`, `_blendShaderSetMap: Map<string, number>`, `_shaderCount`, the 13
source strings, `_isShaderLoading`, `_isShaderLoaded`, `_defaultShaderPath`, `_shaderPath`.

Constructor:
1. `_shaderSets = []`, `_isShaderLoading = false`, `_isShaderLoaded = false`.
2. Color-blend map: iterate `Object.keys(CubismColorBlend)` (a TS numeric enum object; integer-like keys come first in
   ascending order, then string keys in insertion order); keep only keys containing `'ColorBlend_'`; name =
   key without the prefix; value = `parseInt(String(enumValue))`. Result (in this order):
   `_colorBlendValues = [-1, 0, 4, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 1, 2]` with names
   `None, Normal, AddGlow, Add, Darken, Multiply, ColorBurn, LinearBurn, Lighten, Screen, ColorDodge, Overlay,
   SoftLight, HardLight, LinearLight, Hue, Color, AddCompatible, MultiplyCompatible` (19 entries).
3. Alpha-blend map likewise: `_alphaBlendValues = [-1, 0, 1, 2, 3, 4]`, names `None, Over, Atop, Out, ConjointOver,
   DisjointOver`.
4. `_blendShaderSetMap = new Map()`.
5. `_shaderCount = ShaderNames_ShaderCount(10) + 1 + (19 - 3) * (6 - 1) * 3 = 251`.
6. `_defaultShaderPath = '/resources/live2d/shaders/'`, `_shaderPath = _defaultShaderPath`.

Rust: hard-code the two ordered tables (do not derive them from enum reflection).

### 7.3 Loading, compiling and registering programs

`generateShaders()`:
```
if (_isShaderLoading) return
_isShaderLoading = true; _isShaderLoaded = false
_shaderSets.length = _shaderCount (251); each slot = new CubismShaderSet()      // previous programs are NOT deleted
this.loadShaders()                                                              // async fetch, 7.1
  .then(() => { registerShader(); registerBlendShader(); _isShaderLoading = false; _isShaderLoaded = true })
  .catch(error => { _isShaderLoading = false; console.error('Failed to load shaders:', error) })
```
Failure semantics (QUIRK, V1): a fetch failure yields `''` source → compile fails → `loadShaderProgram` returns `0` →
the following `gl.getAttribLocation(0, …)` throws `TypeError` (0 is not a `WebGLProgram`) → `.catch` logs
`Failed to load shaders:` → `_isShaderLoaded` stays false → the next frame's `loadShaders()` calls `generateShaders()`
again (cached successes are reused, failures refetched). A deterministic compile/link failure therefore retries **every
frame forever**, leaking the programs created by each attempt, and the model never renders. The same happens if **any
one** of the 237 blend programs fails, even for models that never use blend modes.

`registerShader()` — programs for sets 0–9:
```
set[0].program = loadShaderProgram(_vertShaderSrcSetupMask, _fragShaderSrcSetupMask)
set[1].program = loadShaderProgram(_vertShaderSrc,          _fragShaderSrcPremultipliedAlpha)
set[2].program = loadShaderProgram(_vertShaderSrcMasked,    _fragShaderSrcMaskPremultipliedAlpha)
set[3].program = loadShaderProgram(_vertShaderSrcMasked,    _fragShaderSrcMaskInvertedPremultipliedAlpha)
set[4..6].program = set[1..3].program      // "加算も通常と同じシェーダーを利用する"
set[7..9].program = set[1..3].program      // "乗算も通常と同じシェーダーを利用する"
then locations (queried separately for every set, including the aliases):
 set0: a_position, a_texCoord (attrib); s_texture0, u_clipMatrix, u_channelFlag, u_baseColor (uniform)
 set1,4,7: a_position, a_texCoord; s_texture0, u_matrix, u_baseColor, u_multiplyColor, u_screenColor
 set2,3,5,6,8,9: a_position, a_texCoord; s_texture0, s_texture1, u_matrix, u_clipMatrix, u_channelFlag,
                 u_baseColor, u_multiplyColor, u_screenColor
```

`registerBlendShader()`:
```
set[10].program = loadShaderProgram(_vertShaderSrcCopy, _fragShaderSrcCopy)
set[10]: attributeTexCoordLocation = 'a_texCoord', attributePositionLocation = 'a_position', uniformBaseColorLocation = 'u_baseColor'
         (QUIRK: samplerTexture0Location is never queried → uniform1i(undefined, 0) is a silent no-op; the sampler defaults to unit 0)
shaderSetIndex = 11
for c in _colorBlendValues (in the order of 7.2):
    if c ∈ {None(-1), AddCompatible(1), MultiplyCompatible(2)}: continue
    colorMacro = `#define COLOR_BLEND_${colorName.toUpperCase()}\n`
    for a in _alphaBlendValues:
        if a == None(-1) || (c == Normal(0) && a == Over(0)): continue
        alphaMacro = `#define ALPHA_BLEND_${alphaName.toUpperCase()}\n`
        generateBlendShader(colorMacro, alphaMacro, shaderSetIndex)
        _blendShaderSetMap.set(colorName + alphaName, shaderSetIndex)     // e.g. "MultiplyAtop" → 71
        shaderSetIndex += ShaderType_Count (3)
```
→ 79 combinations, indices 11…247 (slots 248–250 stay empty `CubismShaderSet`s).

`generateBlendShader(colorMacro, alphaMacro, base)` for `t` in `Normal(0), Masked(1), MaskedInverted(2)`:
```
vs = ''
fs = 'precision mediump float;\n' + colorMacro + alphaMacro + _fragShaderSrcColorBlend + _fragShaderSrcAlphaBlend
if (t == Masked || t == MaskedInverted) { vs += '#define CLIPPING_MASK\n'; fs += '#define CLIPPING_MASK\n' }
vs += _vertShaderSrcBlend; fs += _fragShaderSrcBlend
set[base+t].program = loadShaderProgram(vs, fs)
set[base+t]: a_position, a_texCoord, s_texture0, u_matrix, u_baseColor, u_multiplyColor, u_screenColor,
             samplerFrameBufferTextureLocation = 's_blendTexture'
if masked types: samplerTexture1Location = 's_texture1', uniformClipMatrixLocation = 'u_clipMatrix',
                 uniformChannelFlagLocation = 'u_channelFlag', uniformInvertMaskFlagLocation = 'u_invertClippingMask'
```
Note: Masked and MaskedInverted produce **identical** source; inversion is a runtime uniform (`u_invertClippingMask`).

`loadShaderProgram(vsSrc, fsSrc)`:
```
program = gl.createProgram()
vs = compileShaderSource(VERTEX_SHADER, vsSrc);   if (!vs) { CubismLogError('Vertex shader compile error!');   return 0 }   // program leaked
fs = compileShaderSource(FRAGMENT_SHADER, fsSrc); if (!fs) { CubismLogError('Fragment shader compile error!'); return 0 }   // program + vs leaked
gl.attachShader(program, vs); gl.attachShader(program, fs); gl.linkProgram(program)
if (!gl.getProgramParameter(program, LINK_STATUS)) {
    console.error('Cubism shader link failed.', gl.getProgramInfoLog(program))
    CubismLogError('Failed to link program: {0}', program)
    gl.deleteShader(vs); gl.deleteShader(fs); gl.deleteProgram(program); return 0
}
gl.deleteShader(vs); gl.deleteShader(fs)
return program
```

`compileShaderSource(type, src)`:
```
shader = gl.createShader(type); gl.shaderSource(shader, src); gl.compileShader(shader)
if (!shader) { log = gl.getShaderInfoLog(shader); console.error('Cubism shader compile failed.', log); CubismLogError('Shader compile log: {0} ', log) }
if (!gl.getShaderParameter(shader, COMPILE_STATUS)) {
    log = gl.getShaderInfoLog(shader); console.error('Cubism shader compile failed.', log)
    CubismLogError('Shader compile log: {0} ', log); gl.deleteShader(shader); return null
}
return shader
```
(The `!shader` check is after use, i.e. dead code; with a lost context `shaderSource(null, …)` would throw.)

`releaseShaderProgram()`: for every set: `gl.deleteProgram(set.shaderProgram)` (aliases deleted 3×, empty slots
`deleteProgram(undefined)` — both harmless in WebGL), `shaderProgram = 0`, slot = `null`. `_isShaderLoaded` is not
reset. `release()` = `releaseShaderProgram()`. `setGl(gl)`, `setShaderPath(p)`, `getShaderPath()`.

**V2 recommendation (DEVIATION-OK, strongly advised):** compile only sets 0–3 eagerly (aliases 4–9 reuse them).
Compile the copy program and the 237 blend programs lazily and only if `model.isBlendModeEnabled()` (or on first
request of a given combination). This removes ~238 compile/link round trips from startup (hundreds of ms on ANGLE /
mobile) and removes the V1 failure mode where a single unused blend shader kills all rendering. Bind attribute
locations by querying (`get_attrib_location`, returns `-1` if inactive) exactly as V1 does; do not use
`bindAttribLocation` (keeps parity of which locations are enabled).

### 7.4 Shader set table

| Set | ShaderNames | Program (vert + frag) | Used for |
|---|---|---|---|
| 0 | SetupMask | `vertshadersrcsetupmask.vert` + `fragshadersrcsetupmask.frag` | rendering mask drawables into the mask texture |
| 1 | NormalPremultipliedAlpha | `vertshadersrc.vert` + `fragshadersrcpremultipliedalpha.frag` | unmasked, Normal blend |
| 2 | NormalMaskedPremultipliedAlpha | `vertshadersrcmasked.vert` + `fragshadersrcmaskpremultipliedalpha.frag` | masked, Normal |
| 3 | NomralMaskedInvertedPremultipliedAlpha | `vertshadersrcmasked.vert` + `fragshadersrcmaskinvertedpremultipliedalpha.frag` | inverted-masked, Normal |
| 4 / 5 / 6 | Add… | = programs of 1 / 2 / 3 | Additive (blend func differs) |
| 7 / 8 / 9 | Mult… | = programs of 1 / 2 / 3 | Multiplicative (blend func differs) |
| 10 | (Copy) | `vertshadersrccopy.vert` + `fragshadersrccopy.frag` | model render target → framebuffer (5.3) |
| 11 + 3k + t | blend combo k, type t | `#define`s + blend sources | Cubism 5.3 color/alpha blend (t: 0 normal, 1 masked, 2 masked-inverted) |

`CubismShaderSet` fields: `shaderProgram`, `attributePositionLocation`, `attributeTexCoordLocation`,
`uniformMatrixLocation`, `uniformClipMatrixLocation`, `samplerTexture0Location`, `samplerTexture1Location`,
`uniformBaseColorLocation`, `uniformChannelFlagLocation`, `uniformMultiplyColorLocation`,
`uniformScreenColorLocation`, `samplerFrameBufferTextureLocation`, `uniformInvertMaskFlagLocation`.
A location that was never queried is `undefined`; `uniform*(undefined/null, …)` is a silent no-op in WebGL (the Rust
port uses `Option<WebGlUniformLocation>` and skips `None`).

### 7.5 Blend shader index map (`_blendShaderSetMap`, base index of the Normal variant; +1 masked, +2 masked-inverted)

```
NormalAtop=11 NormalOut=14 NormalConjointOver=17 NormalDisjointOver=20
AddGlowOver=23 AddGlowAtop=26 AddGlowOut=29 AddGlowConjointOver=32 AddGlowDisjointOver=35
AddOver=38 AddAtop=41 AddOut=44 AddConjointOver=47 AddDisjointOver=50
DarkenOver=53 DarkenAtop=56 DarkenOut=59 DarkenConjointOver=62 DarkenDisjointOver=65
MultiplyOver=68 MultiplyAtop=71 MultiplyOut=74 MultiplyConjointOver=77 MultiplyDisjointOver=80
ColorBurnOver=83 ColorBurnAtop=86 ColorBurnOut=89 ColorBurnConjointOver=92 ColorBurnDisjointOver=95
LinearBurnOver=98 LinearBurnAtop=101 LinearBurnOut=104 LinearBurnConjointOver=107 LinearBurnDisjointOver=110
LightenOver=113 LightenAtop=116 LightenOut=119 LightenConjointOver=122 LightenDisjointOver=125
ScreenOver=128 ScreenAtop=131 ScreenOut=134 ScreenConjointOver=137 ScreenDisjointOver=140
ColorDodgeOver=143 ColorDodgeAtop=146 ColorDodgeOut=149 ColorDodgeConjointOver=152 ColorDodgeDisjointOver=155
OverlayOver=158 OverlayAtop=161 OverlayOut=164 OverlayConjointOver=167 OverlayDisjointOver=170
SoftLightOver=173 SoftLightAtop=176 SoftLightOut=179 SoftLightConjointOver=182 SoftLightDisjointOver=185
HardLightOver=188 HardLightAtop=191 HardLightOut=194 HardLightConjointOver=197 HardLightDisjointOver=200
LinearLightOver=203 LinearLightAtop=206 LinearLightOut=209 LinearLightConjointOver=212 LinearLightDisjointOver=215
HueOver=218 HueAtop=221 HueOut=224 HueConjointOver=227 HueDisjointOver=230
ColorOver=233 ColorAtop=236 ColorOut=239 ColorConjointOver=242 ColorDisjointOver=245
```
Macro names: `COLOR_BLEND_` + `NORMAL, ADDGLOW, ADD, DARKEN, MULTIPLY, COLORBURN, LINEARBURN, LIGHTEN, SCREEN,
COLORDODGE, OVERLAY, SOFTLIGHT, HARDLIGHT, LINEARLIGHT, HUE, COLOR`; `ALPHA_BLEND_` + `OVER, ATOP, OUT, CONJOINTOVER,
DISJOINTOVER`. (The shader file also has unreachable branches `COLOR_BLEND_ADDCOMPATIBLE` / `COLOR_BLEND_MULTCOMPATIBLE`.)

### 7.6 Verbatim GLSL (bodies after the 6-line license comment and 2 blank lines)

`vertshadersrc.vert`
```glsl
attribute vec4 a_position;
attribute vec2 a_texCoord;
varying vec2 v_texCoord;
uniform mat4 u_matrix;

void main()
{
    gl_Position = u_matrix * a_position;
    v_texCoord = a_texCoord;
    v_texCoord.y = 1.0 - v_texCoord.y;
}
```

`vertshadersrcmasked.vert`
```glsl
attribute vec4 a_position;
attribute vec2 a_texCoord;
varying vec2 v_texCoord;
varying vec4 v_clipPos;
uniform mat4 u_matrix;
uniform mat4 u_clipMatrix;

void main()
{
    gl_Position = u_matrix * a_position;
    v_clipPos = u_clipMatrix * a_position;
    v_texCoord = a_texCoord;
    v_texCoord.y = 1.0 - v_texCoord.y;
}
```

`vertshadersrcsetupmask.vert`
```glsl
attribute vec4 a_position;
attribute vec2 a_texCoord;
varying vec2 v_texCoord;
varying vec4 v_myPos;
uniform mat4 u_clipMatrix;

void main()
{
    gl_Position = u_clipMatrix * a_position;
    v_myPos = u_clipMatrix * a_position;
    v_texCoord = a_texCoord;
    v_texCoord.y = 1.0 - v_texCoord.y;
}
```

`fragshadersrcsetupmask.frag`
```glsl
precision mediump float;
varying vec2 v_texCoord; //v2f.texcoord
varying vec4 v_myPos;
uniform vec4 u_baseColor; //v2f.color
uniform vec4 u_channelFlag;
uniform sampler2D s_texture0; //_MainTex


void main()
{
  float isInside =
    step(u_baseColor.x, v_myPos.x/v_myPos.w)
    * step(u_baseColor.y, v_myPos.y/v_myPos.w)
    * step(v_myPos.x/v_myPos.w, u_baseColor.z)
    * step(v_myPos.y/v_myPos.w, u_baseColor.w);
  gl_FragColor = u_channelFlag * texture2D(s_texture0, v_texCoord).a * isInside;
}
```

`fragshadersrcpremultipliedalpha.frag`
```glsl
precision mediump float;
varying vec2 v_texCoord; //v2f.texcoord
uniform vec4 u_baseColor; //v2f.color
uniform sampler2D s_texture0; //_MainTex
uniform vec4 u_multiplyColor;
uniform vec4 u_screenColor;

void main()
{
  vec4 texColor = texture2D(s_texture0, v_texCoord);
  texColor.rgb = texColor.rgb * u_multiplyColor.rgb;
  texColor.rgb = (texColor.rgb + u_screenColor.rgb * texColor.a) - (texColor.rgb * u_screenColor.rgb);
  vec4 color = texColor * u_baseColor;
  gl_FragColor = vec4(color.rgb, color.a);
}
```

`fragshadersrcmaskpremultipliedalpha.frag`
```glsl
precision mediump float;
varying vec2 v_texCoord; //v2f.texcoord
varying vec4 v_clipPos;
uniform vec4 u_baseColor; //v2f.color
uniform vec4 u_channelFlag;
uniform sampler2D s_texture0; //_MainTex
uniform sampler2D s_texture1; // _ClippingMaskTex
uniform vec4 u_multiplyColor;
uniform vec4 u_screenColor;

void main()
{
  vec4 texColor = texture2D(s_texture0, v_texCoord);
  texColor.rgb = texColor.rgb * u_multiplyColor.rgb;
  texColor.rgb = (texColor.rgb + u_screenColor.rgb * texColor.a) - (texColor.rgb * u_screenColor.rgb);
  vec4 col_formask = texColor * u_baseColor;
  vec4 clipMask = (1.0 - texture2D(s_texture1, v_clipPos.xy / v_clipPos.w)) * u_channelFlag;
  float maskVal = clipMask.r + clipMask.g + clipMask.b + clipMask.a;
  col_formask = col_formask * maskVal;
  gl_FragColor = col_formask;
}
```

`fragshadersrcmaskinvertedpremultipliedalpha.frag` — identical to the previous one except the uniform declaration
order (`s_texture0, s_texture1, u_channelFlag, u_baseColor, u_multiplyColor, u_screenColor`) and the line
`col_formask = col_formask * (1.0 - maskVal);`:
```glsl
precision mediump float;
varying vec2 v_texCoord; //v2f.texcoord
varying vec4 v_clipPos;
uniform sampler2D s_texture0; //_MainTex
uniform sampler2D s_texture1; // _ClippingMaskTex
uniform vec4 u_channelFlag;
uniform vec4 u_baseColor; //v2f.color
uniform vec4 u_multiplyColor;
uniform vec4 u_screenColor;

void main()
{
  vec4 texColor = texture2D(s_texture0, v_texCoord);
  texColor.rgb = texColor.rgb * u_multiplyColor.rgb;
  texColor.rgb = (texColor.rgb + u_screenColor.rgb * texColor.a) - (texColor.rgb * u_screenColor.rgb);
  vec4 col_formask = texColor * u_baseColor;
  vec4 clipMask = (1.0 - texture2D(s_texture1, v_clipPos.xy / v_clipPos.w)) * u_channelFlag;
  float maskVal = clipMask.r + clipMask.g + clipMask.b + clipMask.a;
  col_formask = col_formask * (1.0 - maskVal);
  gl_FragColor = col_formask;
}
```

`vertshadersrccopy.vert`
```glsl
attribute vec4 a_position;
attribute vec2 a_texCoord;
varying vec2 v_texCoord;

void main()
{
    v_texCoord = a_texCoord;
    gl_Position = a_position;
}
```

`fragshadersrccopy.frag`
```glsl
precision mediump float;

varying vec2 v_texCoord;
uniform vec4 u_baseColor;
uniform sampler2D s_texture0;

void main()
{
    gl_FragColor = texture2D(s_texture0, v_texCoord) * u_baseColor;
}
```

`vertshadersrcblend.vert`
```glsl
attribute vec4 a_position;
attribute vec2 a_texCoord;
varying vec2 v_texCoord;
varying vec2 v_blendCoord;
varying vec4 v_clipPos;
uniform mat4 u_matrix;
uniform mat4 u_clipMatrix;

void main()
{
    gl_Position = u_matrix * a_position;

#ifdef CLIPPING_MASK
    v_clipPos = u_clipMatrix * a_position;
#else
    v_clipPos = vec4(0.0);
#endif

    v_texCoord = a_texCoord;
    v_texCoord.y = 1.0 - v_texCoord.y;
    vec2 ndcPos = gl_Position.xy / gl_Position.w;
    v_blendCoord = ndcPos * 0.5 + 0.5;
}
```

`fragshadersrcpremultipliedalphablend.frag` (no precision line — it is prepended by `generateBlendShader`):
```glsl
varying vec2 v_texCoord; //v2f.texcoord
varying vec2 v_blendCoord;
varying vec4 v_clipPos;
uniform sampler2D s_texture0; //_MainTex
uniform sampler2D s_blendTexture;
uniform vec4 u_baseColor; //v2f.color
uniform vec4 u_multiplyColor;
uniform vec4 u_screenColor;
uniform sampler2D s_texture1; // _ClippingMaskTex
uniform float u_invertClippingMask;
uniform vec4 u_channelFlag;

vec3 ColorBlend(vec3 colorSource, vec3 colorDestination);
vec4 AlphaBlend(vec3 C, vec3 Cs, float As, vec3 Cd, float Ad);

void main()
{
  vec4 renderTextureColor = texture2D(s_blendTexture, v_blendCoord);
  vec3 colorDestination = renderTextureColor.rgb;
  float alphaDestination = renderTextureColor.a;

  if (alphaDestination < 0.00001)
  {
    colorDestination = vec3(0.0, 0.0, 0.0);
  }
  else {
    colorDestination /= alphaDestination;
  }

  vec4 texColor = texture2D(s_texture0, v_texCoord);
  texColor.rgb *= u_multiplyColor.rgb;
  texColor.rgb = (texColor.rgb + u_screenColor.rgb) - (texColor.rgb * u_screenColor.rgb);

  texColor *= u_baseColor;
  vec3 colorSource = texColor.rgb;
  float alphaSource = texColor.a;

  if (alphaSource < 0.00001)
  {
    colorSource = vec3(0.0, 0.0, 0.0);
  }
  else {
    colorSource /= alphaSource;
  }

#ifdef CLIPPING_MASK
    float maskVal = 1.0;
    vec4 clipMask = (1.0 - texture2D(s_texture1, v_clipPos.xy / v_clipPos.w)) * u_channelFlag;
    maskVal = clipMask.r + clipMask.g + clipMask.b + clipMask.a;
    maskVal = abs(u_invertClippingMask - maskVal);

    alphaSource *= maskVal;
#endif

  vec4 source = vec4(colorSource.r, colorSource.g, colorSource.b, alphaSource);
  vec4 destination = vec4(colorDestination.r, colorDestination.g, colorDestination.b, alphaDestination);

  gl_FragColor = AlphaBlend(ColorBlend(colorSource, colorDestination), source, destination);
}
```
(The 5-parameter `AlphaBlend` prototype is an unused overload declaration; the 3-parameter definition comes from
`fragshadersrcalphablend.frag`, which is concatenated earlier. Note the screen formula here omits the `* texColor.a`
factor used by the non-blend shaders — SDK behaviour, keep.)

`fragshadersrcalphablend.frag` and `fragshadersrccolorblend.frag` are long; embed them byte-identically from `SHADERS/`
(md5 in 0.2). Their logic:
* `OverlapRgba(color, Cs, Cd, p) = vec4(color*p.x + Cs*p.y + Cd*p.z, p.x + p.y + p.z)`.
* `AlphaBlend(color, src, dst)` parameter vector `p` per macro: OVER `(As·Ad, As·(1−Ad), Ad·(1−As))`; ATOP
  `(As·Ad, 0, Ad·(1−As))`; OUT `(0, 0, Ad·(1−As))`; CONJOINTOVER `(min(As,Ad), max(As−Ad,0), max(Ad−As,0))`;
  DISJOINTOVER `(max(As+Ad−1,0), min(As,1−Ad), min(Ad,1−As))`; otherwise `#error not supported alpha blend function`.
* `ColorBlend(Cs, Cd)`: NORMAL `Cs`; ADD `min(Cs+Cd, 1.0)`; ADDGLOW `Cs+Cd`; DARKEN `min`; MULTIPLY `Cs·Cd`;
  COLORBURN per channel `|Cd−1|<1e-6 → 1; |Cs|<1e-6 → 0; else 1 − min(1, (1−Cd)/Cs)`; LINEARBURN `max(0, Cs+Cd−1)`;
  LIGHTEN `max`; SCREEN `Cs+Cd−Cs·Cd`; COLORDODGE `Cd≤0 → 0; Cs==1 → 1; else min(1, Cd/(1−Cs))`; OVERLAY
  `Cd<0.5 ? 2CsCd : 1−2(1−Cs)(1−Cd)`; SOFTLIGHT `Cs≤0.5 → Cd−(1−2Cs)Cd(1−Cd); Cd≤0.25 → Cd+(2Cs−1)Cd((16Cd−12)Cd+3);
  else Cd+(2Cs−1)(sqrt(Cd)−Cd)`; HARDLIGHT `Cs<0.5 ? 2CsCd : 1−2(1−Cs)(1−Cd)`; LINEARLIGHT `Cs<0.5 ? max(0,2Cs+Cd−1) :
  min(1, 2(Cs−0.5)+Cd)`; HUE `SetLuma(SetSaturation(Cs, Sat(Cd)), Luma(Cd))`; COLOR `SetLuma(Cs, Luma(Cd))` with
  `Luma = 0.30r+0.59g+0.11b`, `ClipColor`/`SetSaturation` as in the W3C compositing spec; otherwise
  `#error not supported color blend function.`

All sources are GLSL ES 1.00 and compile on WebGL1 (only `texture2D`, `gl_FragColor`, `step`, `min/max/abs/sqrt`,
int literal inside a `vec3` constructor in ATOP, which ES 1.00 allows).

### 7.7 Shader math (what the pixels mean)

* Textures are premultiplied (uploaded with `UNPACK_PREMULTIPLY_ALPHA_WEBGL = 1`, 12.3), so `texColor.rgb` is
  premultiplied. Multiply color: `rgb *= M.rgb` (alpha of M ignored). Screen color in premultiplied form:
  `rgb = rgb + S.rgb·a − rgb·S.rgb` (alpha of S ignored). Then `color = texColor * u_baseColor` where
  `u_baseColor = (o,o,o,o)` (premultiplied model color × opacity) → still premultiplied.
* Mask generation (set 0): output `u_channelFlag * tex.a * isInside` (e.g. `(a,0,0,0)` for channel R). `isInside` is 1
  only inside the context's layout rectangle (in mask NDC, `u_baseColor = (x*2−1, y*2−1, right*2−1, bottom*2−1)`; edges
  inclusive via `step`) so a mask cannot bleed into a neighbour's atlas cell. Blend
  `blendFuncSeparate(ZERO, ONE_MINUS_SRC_COLOR, ZERO, ONE_MINUS_SRC_ALPHA)` on a buffer cleared to `(1,1,1,1)` ⇒ each
  channel multiplies independently: `dst.c *= (1 − src.c)`, `dst.a *= (1 − src.a)`; unused channels get `src = 0` →
  unchanged. A texel value of 1 means "not covered", 0 "fully covered"; several mask drawables in one context multiply.
* Masked draw: `clipMask = (1 − mask(v_clipPos.xy / w)) * u_channelFlag`; `maskVal = r+g+b+a` = coverage of the
  context's channel; normal: `color *= maskVal`; inverted: `color *= (1 − maskVal)`. The mask texture is sampled with
  LINEAR filtering, CLAMP_TO_EDGE.
* Copy (set 10): `texture * u_baseColor` with the full-screen quad (`a_position` used directly as clip coordinates).
* Blend (5.3): un-premultiplies source and destination (destination = copy of the current target, `s_blendTexture`,
  sampled at the fragment's own NDC position), applies `ColorBlend`/`AlphaBlend`, writes with `blendFunc(ONE, ZERO)`.

### 7.8 `CubismShaderManager_WebGL` (singleton)

`getInstance()` lazily creates the module singleton; `deleteInstance()` → `release()` (calls `release()` on every
`CubismShader_WebGL`, clears the map) and drops it. `_shaderMap: Map<gl, CubismShader_WebGL>`; `getShader(gl)`
returns `undefined` for an unregistered context (callers then throw TypeError); `setGlContext(gl)` creates + `setGl`
a new `CubismShader_WebGL` only if absent. Rust: make the shader cache a field of the renderer (or of an
`Rc<GlResources>` shared per context) instead of a global.

---

## 8. Shader selection, uniforms and blend state per object

### 8.1 `setupShaderProgramForDrawable(renderer, model, index)` — exact steps

```
if (!renderer.isPremultipliedAlpha()) CubismLogError('NoPremultipliedAlpha is not allowed')   // continues anyway
if (_shaderSets.length == 0) generateShaders()
if (_isShaderLoaded == false) { CubismLogWarning('Shader program is not initialized.'); return }
masked   = renderer.getClippingContextBufferForDrawable() != null
inverted = model.getDrawableInvertedMaskBit(index)
offset   = masked ? (inverted ? 2 : 1) : 0                       // inverted bit is ignored for unmasked drawables
isUsingCompatible = true
if (model.isBlendModeEnabled()) {
    c = model.getDrawableColorBlend(index); a = model.getDrawableAlphaBlend(index)
    if (c == None || a == None || (c == Normal && a == Over)) { set = sets[1 + offset]; blend = (ONE, ONE_MINUS_SRC_ALPHA, ONE, ONE_MINUS_SRC_ALPHA) }
    else switch (c) {
        AddCompatible:      set = sets[4 + offset]; blend = (ONE, ONE, ZERO, ONE)
        MultiplyCompatible: set = sets[7 + offset]; blend = (DST_COLOR, ONE_MINUS_SRC_ALPHA, ZERO, ONE)
        default: srcBuffer = renderer._currentOffscreen ?? renderer.getModelRenderTarget(0)
                 CubismRenderTarget_WebGL.copyBuffer(gl, srcBuffer, renderer.getModelRenderTarget(1))   // throws on WebGL1
                 set = sets[_blendShaderSetMap.get(colorName(c) + alphaName(a)) + offset]
                 blend = (ONE, ZERO, ONE, ZERO); isUsingCompatible = false
    }
} else switch (model.getDrawableBlendMode(index)) {      // V1 path for this model
    Normal (default): set = sets[1 + offset]; blend = (ONE, ONE_MINUS_SRC_ALPHA, ONE, ONE_MINUS_SRC_ALPHA)
    Additive:         set = sets[4 + offset]; blend = (ONE, ONE, ZERO, ONE)
    Multiplicative:   set = sets[7 + offset]; blend = (DST_COLOR, ONE_MINUS_SRC_ALPHA, ZERO, ONE)
}
gl.useProgram(set.shaderProgram)
if (renderer._bufferData.vertex == null) renderer._bufferData.vertex = gl.createBuffer()
gl.bindBuffer(ARRAY_BUFFER, vertex); gl.bufferData(ARRAY_BUFFER, model.getDrawableVertices(index), DYNAMIC_DRAW)
gl.enableVertexAttribArray(set.attributePositionLocation); gl.vertexAttribPointer(set.attributePositionLocation, 2, FLOAT, false, 0, 0)
if (renderer._bufferData.uv == null) renderer._bufferData.uv = gl.createBuffer()
gl.bindBuffer(ARRAY_BUFFER, uv); gl.bufferData(ARRAY_BUFFER, model.getDrawableVertexUvs(index), DYNAMIC_DRAW)
gl.enableVertexAttribArray(set.attributeTexCoordLocation); gl.vertexAttribPointer(set.attributeTexCoordLocation, 2, FLOAT, false, 0, 0)
if (masked) {
    ctx = renderer.getClippingContextBufferForDrawable()
    gl.activeTexture(TEXTURE1); gl.bindTexture(TEXTURE_2D, renderer.getDrawableMaskBuffer(ctx._bufferIndex).getColorBuffer())
    gl.uniform1i(set.samplerTexture1Location, 1)
    gl.uniformMatrix4fv(set.uniformClipMatrixLocation, false, ctx._matrixForDraw.getArray())
    ch = ctx.getClippingManager().getChannelFlagAsColor(ctx._layoutChannelIndex)
    gl.uniform4f(set.uniformChannelFlagLocation, ch.r, ch.g, ch.b, ch.a)
    if (model.isBlendModeEnabled()) gl.uniform1f(set.uniformInvertMaskFlagLocation, inverted ? 1.0 : 0.0)   // no-op location for sets 1..9
}
tex = renderer.getBindedTextures().get(model.getDrawableTextureIndex(index))
gl.activeTexture(TEXTURE0); gl.bindTexture(TEXTURE_2D, tex); gl.uniform1i(set.samplerTexture0Location, 0)
gl.uniformMatrix4fv(set.uniformMatrixLocation, false, renderer.getMvpMatrix().getArray())
base = model.isBlendModeEnabled() ? TextureColor(op, op, op, op)                 // "モデルカラーは最後に処理するため不透明度のみ"
                                  : renderer.getModelColorWithOpacity(model.getDrawableOpacity(index))
mul = model.getOverrideMultiplyAndScreenColor().getDrawableMultiplyColor(index)
scr = model.getOverrideMultiplyAndScreenColor().getDrawableScreenColor(index)
gl.uniform4f(u_baseColor, base.r, base.g, base.b, base.a)
gl.uniform4f(u_multiplyColor, mul.r, mul.g, mul.b, mul.a)
gl.uniform4f(u_screenColor, scr.r, scr.g, scr.b, scr.a)
if (model.isBlendModeEnabled()) {
    gl.activeTexture(TEXTURE2)                                           // left active even for compatible sets
    if (!isUsingCompatible) { gl.bindTexture(TEXTURE_2D, renderer.getModelRenderTarget(1).getColorBuffer());
                              gl.uniform1i(set.samplerFrameBufferTextureLocation, 2) }
}
if (renderer._bufferData.index == null) renderer._bufferData.index = gl.createBuffer()
gl.bindBuffer(ELEMENT_ARRAY_BUFFER, index); gl.bufferData(ELEMENT_ARRAY_BUFFER, model.getDrawableVertexIndices(index), DYNAMIC_DRAW)
gl.blendFuncSeparate(srcColor, dstColor, srcAlpha, dstAlpha)
```
Missing texture (`get()` → `undefined`): `bindTexture(TEXTURE_2D, null)` → the sampler reads an incomplete texture,
which WebGL defines as `(0,0,0,1)` → the drawable renders opaque black (scaled by opacity). In V1 the draw never
happens before all textures are bound (`CompleteSetup`); the Rust port must keep that gate.

**Rust zero-copy note:** pass the Core's JS `Float32Array`/`Uint16Array` views directly to
`buffer_data_with_array_buffer_view` (do not copy into wasm memory and back).

### 8.2 `setupShaderProgramForMask(renderer, model, index)`

```
premultiplied check / generateShaders-if-empty / not-loaded early return (same as 8.1)
set = sets[0]; gl.useProgram(set.shaderProgram)
vertex buffer (create-if-null) → bindBuffer(ARRAY, vertex) → bufferData(positions, DYNAMIC_DRAW) → enable + pointer(a_position, 2, FLOAT, false, 0, 0)
uv buffer (create-if-null) → bindBuffer(ARRAY, uv)
tex = textures.get(model.getDrawableTextureIndex(index)); gl.activeTexture(TEXTURE0); gl.bindTexture(TEXTURE_2D, tex); gl.uniform1i(set.samplerTexture0Location, 0)
uv buffer (create-if-null) → bindBuffer(ARRAY, uv) → bufferData(uvs, DYNAMIC_DRAW) → enable + pointer(a_texCoord, 2, FLOAT, false, 0, 0)
ctx = renderer.getClippingContextBufferForMask()
ch = ctx.getClippingManager().getChannelFlagAsColor(ctx._layoutChannelIndex); gl.uniform4f(set.uniformChannelFlagLocation, ch.r, ch.g, ch.b, ch.a)
gl.uniformMatrix4fv(set.uniformClipMatrixLocation, false, ctx._matrixForMask.getArray())
r = ctx._layoutBounds
gl.uniform4f(set.uniformBaseColorLocation, r.x*2.0 - 1.0, r.y*2.0 - 1.0, r.getRight()*2.0 - 1.0, r.getBottom()*2.0 - 1.0)
index buffer (create-if-null) → bindBuffer(ELEMENT_ARRAY, index) → bufferData(indices, DYNAMIC_DRAW)
gl.blendFuncSeparate(ZERO, ONE_MINUS_SRC_COLOR, ZERO, ONE_MINUS_SRC_ALPHA)
```
No MVP, no opacity, no multiply/screen color: masks ignore drawable opacity (only texture alpha × triangle coverage).

### 8.3 `setupShaderProgramForOffscreen(renderer, model, offscreen)` (5.3)

Same preamble. `o = offscreen.getOffscreenIndex()`; `masked = getClippingContextBufferForOffscreen() != null`;
`inverted = model.getOffscreenInvertedMask(o)`; `offset` as 8.1; blend selection identical to the blend-enabled branch
of 8.1 but with `getOffscreenColorBlend/AlphaBlend(o)`, and the `default` case copies
`offscreen.getOldOffscreen() ?? renderer.getModelRenderTarget(0)` into model render target 1. Then:
```
gl.useProgram(set.shaderProgram)
CubismRenderTarget_WebGL.copyBuffer(gl, offscreen, renderer.getModelRenderTarget(2))      // always (WebGL2 only)
gl.activeTexture(TEXTURE0); gl.bindTexture(TEXTURE_2D, modelRenderTarget(2).getColorBuffer()); gl.uniform1i(s_texture0, 0)
gl.uniformMatrix4fv(u_matrix, false, identity)
op = model.getOffscreenOpacity(o); gl.uniform4f(u_baseColor, op, op, op, op)                // "乗算済みアルファ…1.0に乗算した状態"
mul/scr = override.getOffscreenMultiplyColor(o) / getOffscreenScreenColor(o) → uniform4f ×2
gl.activeTexture(TEXTURE2); if (!isUsingCompatible) { bindTexture(modelRenderTarget(1).getColorBuffer()); uniform1i(s_blendTexture, 2) }
if (masked) { activeTexture(TEXTURE1); bindTexture(getOffscreenMaskBuffer(ctx._bufferIndex).getColorBuffer()); uniform1i(s_texture1, 1);
              uniformMatrix4fv(u_clipMatrix, false, ctx._matrixForDraw); uniform4f(u_channelFlag, …);
              if (model.isBlendModeEnabled()) uniform1f(u_invertClippingMask, inverted ? 1 : 0) }
vertex buffer (create-if-falsy) → bindBuffer → bufferData(s_renderTargetVertexArray, STATIC_DRAW) → enable + pointer(a_position, 2, FLOAT, false, 8, 0)
uv buffer → bufferData(s_renderTargetReverseUvArray, STATIC_DRAW) → enable + pointer(a_texCoord, 2, FLOAT, false, 8, 0)
gl.blendFuncSeparate(srcColor, dstColor, srcAlpha, dstAlpha)
```
(The reverse UV array compensates the `v = 1 − v` flip in the blend vertex shader.)

### 8.4 `setupShaderProgramForOffscreenRenderTarget(renderer)` / `copyTexture(renderer, baseColor)` (5.3)

`setupShaderProgramForOffscreenRenderTarget`: generate-if-empty, not-loaded early return (warning
`Shader program is not initialized.`), then `c = renderer.getModelColor(); c.r *= c.a; c.g *= c.a; c.b *= c.a;`
(comment: "この時点のテクスチャはPMAになっているはずなので計算を行う") → `copyTexture(renderer, c)`.
`copyTexture`: `set = sets[10]`; `useProgram`; `uniform4f(u_baseColor, c)`; `activeTexture(TEXTURE0)`;
`bindTexture(modelRenderTarget(0).getColorBuffer())`; `uniform1i(set.samplerTexture0Location /* undefined */, 0)`;
vertex buffer ← `s_renderTargetVertexArray` (STATIC_DRAW, stride 8); uv buffer ← `s_renderTargetUvArray` (stride 8);
`blendFuncSeparate(ONE, ONE_MINUS_SRC_ALPHA, ONE, ONE_MINUS_SRC_ALPHA)`.

### 8.5 Summary tables

Shader set + GL blend state (`blendFuncSeparate(srcRGB, dstRGB, srcA, dstA)`), non-blend-mode models (V1):

| Drawable state | Normal | Additive (`constantFlags & 1`) | Multiplicative (`& 2`) |
|---|---|---|---|
| unmasked | set 1 | set 4 (= prog 1) | set 7 (= prog 1) |
| masked | set 2 | set 5 (= prog 2) | set 8 (= prog 2) |
| masked + inverted (`& 8`) | set 3 | set 6 (= prog 3) | set 9 (= prog 3) |
| blend state | `ONE, ONE_MINUS_SRC_ALPHA, ONE, ONE_MINUS_SRC_ALPHA` | `ONE, ONE, ZERO, ONE` | `DST_COLOR, ONE_MINUS_SRC_ALPHA, ZERO, ONE` |

Mask generation: set 0, `ZERO, ONE_MINUS_SRC_COLOR, ZERO, ONE_MINUS_SRC_ALPHA`.
Blend-mode models: `None`/`Normal+Over` → as Normal; `AddCompatible` → Additive; `MultiplyCompatible` →
Multiplicative; anything else → blend program `map[name] + offset` with `ONE, ZERO, ONE, ZERO` (after copying the
destination into model render target 1). Copy to screen: set 10, `ONE, ONE_MINUS_SRC_ALPHA, ONE, ONE_MINUS_SRC_ALPHA`.

Uniform values per draw (non-blend path):

| Uniform | Value |
|---|---|
| `u_matrix` | MVP `_mvpMatrix4x4` (f32[16], transpose false) |
| `u_baseColor` | `(mc.r·mc.a·o, mc.g·mc.a·o, mc.b·mc.a·o, mc.a·o)` with model color `mc` (V1: `(o,o,o,o)`) |
| `u_multiplyColor` | resolved multiply color RGBA (V1: Core value, default `(1,1,1,1)`) |
| `u_screenColor` | resolved screen color RGBA (V1: Core value, default `(0,0,0,1)`) |
| `s_texture0` | 0 (model texture on unit 0) |
| `s_texture1` | 1 (mask color buffer on unit 1), masked only |
| `u_clipMatrix` | `ctx._matrixForDraw` (masked) / `ctx._matrixForMask` (mask generation) |
| `u_channelFlag` | channel color of `ctx._layoutChannelIndex`: R `(1,0,0,0)`, G `(0,1,0,0)`, B `(0,0,1,0)`, A `(0,0,0,1)` |
| `u_baseColor` (mask gen) | layout rect in mask NDC `(x·2−1, y·2−1, right·2−1, bottom·2−1)` |

---

## 9. Clipping masks

### 9.1 Constants and manager fields (`CubismClippingManager<T>`, FW/rendering/cubismclippingmanager.ts)

```
ColorChannelCount = 4                                  // "実験時に1チャンネルの場合は1、RGBだけの場合は3、アルファも含める場合は4"
ClippingMaskMaxCountOnDefault = 36                     // max contexts per render texture when there is 1 texture
ClippingMaskMaxCountOnMultiRenderTexture = 32          // per texture when there are ≥ 2 textures
```

| Field | Init (constructor) | Meaning |
|---|---|---|
| `_renderTextureCount` | 0 (set by `initializeFor*`) | number of mask render textures |
| `_clippingMaskBufferSize` | **256** | mask texture width = height (px) |
| `_clippingContextListForMask` | `[]` | unique contexts, creation order (layout order!) |
| `_clippingContextListForDraw` | `[]` | per drawable index: its context or `null` |
| `_clippingContextListForOffscreen` | `[]` | per offscreen index (5.3) |
| `_tmpBoundsOnModel` | `new csmRect()` | scratch rect |
| `_tmpMatrix`, `_tmpMatrixForMask`, `_tmpMatrixForDraw` | identity | scratch matrices |
| `_clearedMaskBufferFlags` | `[]` | per render texture: cleared this frame? |
| `_channelColors` | `[(1,0,0,0), (0,1,0,0), (0,0,1,0), (0,0,0,1)]` | `getChannelFlagAsColor(ch)` |
| `_clippingContexttConstructor` (sic) | factory | creates `CubismClippingContext_WebGL(manager, masks, count)` |
| `_currentMaskBuffer` (WebGL subclass) | — | render target currently bound during mask generation |
| `gl` (WebGL subclass) | — | set by `setGL(gl)` |

Accessors: `getClippingContextListForDraw()`, `getClippingContextListForOffscreen()`,
`getClippingMaskBufferSize()`, `setClippingMaskBufferSize(size)`, `getRenderTextureCount()`,
`getChannelFlagAsColor(ch)`, `getClippingMaskCount()` (= `_clippingContextListForMask.length`, WebGL subclass).

`release()`: release + null every context in `_clippingContextListForMask`, null the list; null every entry of
`_clippingContextListForDraw` and the list (they alias the same objects); null channel colors; clear + null
`_clearedMaskBufferFlags`.

### 9.2 Clipping context (`CubismClippingContext` + `_WebGL`)

Constructor `(manager, clippingDrawableIndices: Int32Array, clipCount)`:

| Field | Init | Meaning |
|---|---|---|
| `_owner` | manager | `getClippingManager()` |
| `_clippingIdList` | the drawable's Core `masks[i]` array (**aliased**, not copied) | mask drawable indices |
| `_clippingIdCount` | `clipCount` (= `maskCounts[i]`) | number of mask drawables |
| `_isUsing` | `undefined` (falsy) until the first bounds pass | mask needed this frame |
| `_allClippedDrawRect` | `new csmRect()` (fields `undefined`) | bounds (model space) of all drawables clipped by this context |
| `_layoutBounds` | `new csmRect()` (fields `undefined`) | cell inside the mask texture in `[0,1]` units |
| `_layoutChannelIndex` | 0 | 0=R 1=G 2=B 3=A |
| `_bufferIndex` | 0 | which mask render texture |
| `_matrixForMask`, `_matrixForDraw` | identity | model → mask NDC / model → mask UV |
| `_clippedDrawableIndexList` | `[]` | drawables using this context (`addClippedDrawable(i)`) |
| `_clippedOffscreenIndexList` | `[]` | offscreens using it (`addClippedOffscreen(i)`) |

`release()` nulls the two rects and two lists. `setGl(gl)` forwards to `_owner.setGL(gl)`.

### 9.3 Initialization

`initializeForDrawable(model, renderTextureCount)`:
```
if (renderTextureCount % 1 != 0) { CubismLogWarning('The number of render textures must be specified as an integer. The decimal point is rounded down and corrected to an integer.'); renderTextureCount = ~~renderTextureCount }
if (renderTextureCount < 1) CubismLogWarning('The number of render textures must be an integer greater than or equal to 1. Set the number of render textures to 1.')
_renderTextureCount = renderTextureCount < 1 ? 1 : renderTextureCount
_clearedMaskBufferFlags = new Array(_renderTextureCount)              // holes (undefined)
_clippingContextListForDraw.length = drawableCount
for i in 0..drawableCount:
    if (maskCounts[i] <= 0) { listForDraw[i] = null; continue }
    ctx = findSameClip(masks[i], maskCounts[i])
    if (ctx == null) { ctx = new Context(this, masks[i], maskCounts[i]); _clippingContextListForMask.push(ctx) }
    ctx.addClippedDrawable(i)
    listForDraw[i] = ctx
```

`initializeForOffscreen(model, maskBufferCount)` (5.3):
```
_renderTextureCount = maskBufferCount                                  // no validation here
_clearedMaskBufferFlags.length = _renderTextureCount; all = false
_clippingContextListForOffscreen.length = offscreenCount
for i in 0..offscreenCount:
    if (offscreenMaskCounts[i] <= 0) { listForOffscreen.push(null); continue }   // QUIRK: push appends past the end;
                                                                                 // slot i stays undefined (== null for checks)
    cc = findSameClip(offscreenMasks[i], offscreenMaskCounts[i]) ?? push new Context into _clippingContextListForMask
    cc.addClippedOffscreen(i); listForOffscreen[i] = cc
```
(Rust: just store `None` at index i.)

`findSameClip(masks, count)`: for each existing context in `_clippingContextListForMask` (creation order): skip if
`ctx._clippingIdCount != count`; `same = 0`; for `j < count`: `id = ctx._clippingIdList[j]`; for `k < count`: if
`masks[k] == id` → `same++`, break; if `same == count` → return ctx. Return null. (Order-insensitive set equality;
QUIRK: duplicate ids can produce false matches, e.g. `[a,a]` vs `[a,b]`. Reproduce.)

This model: 6 contexts in creation order (by first clipped drawable index): ctx0 masks [18] clips [3, 9];
ctx1 [46] clips [19..23]; ctx2 [45] clips [28..32]; ctx3 [33] clips [34, 35]; ctx4 [28] clips [45] (inverted);
ctx5 [19] clips [46] (inverted). Note the mutual masking 19↔46 and 28↔45 — mask drawables are rendered with the
SetupMask program and are not themselves clipped during mask generation.

### 9.4 Per-frame mask generation: `CubismClippingManager_WebGL.setupClippingContext(model, renderer, lastFbo, lastViewport, type)`

```
usingClipCount = 0
for cc in _clippingContextListForMask:
    (type == Offscreen) ? calcClippedOffscreenTotalBounds(model, cc) : calcClippedDrawableTotalBounds(model, cc)   // default → Drawable
    if (cc._isUsing) usingClipCount++
if (usingClipCount <= 0) return
gl.viewport(0, 0, _clippingMaskBufferSize, _clippingMaskBufferSize)
_currentMaskBuffer = (type == Offscreen) ? renderer.getOffscreenMaskBuffer(0) : renderer.getDrawableMaskBuffer(0)
_currentMaskBuffer.beginDraw(lastFbo)                    // binds mask FBO 0
renderer.preDraw()                                       // comment says "バッファをクリアする" but it only sets state
setupLayoutBounds(usingClipCount)                        // 9.6
if (_clearedMaskBufferFlags.length != _renderTextureCount) { _clearedMaskBufferFlags.length = 0; _clearedMaskBufferFlags = new Array(_renderTextureCount).fill(false) }
for all flags: = false                                   // "マスクのクリアフラグを毎フレーム開始時に初期化"
for clipContext in _clippingContextListForMask:          // ALL contexts, including ones not in use
    rect = clipContext._allClippedDrawRect; lb = clipContext._layoutBounds
    margin = 0.05
    maskBuffer = (type == Offscreen) ? renderer.getOffscreenMaskBuffer(clipContext._bufferIndex) : renderer.getDrawableMaskBuffer(clipContext._bufferIndex)
    if (_currentMaskBuffer != maskBuffer) { _currentMaskBuffer.endDraw(); _currentMaskBuffer = maskBuffer; _currentMaskBuffer.beginDraw(lastFbo); renderer.preDraw() }
    _tmpBoundsOnModel.setRect(rect)
    _tmpBoundsOnModel.expand(rect.width * margin, rect.height * margin)      // 5 % on each side
    scaleX = lb.width / _tmpBoundsOnModel.width                               // f64
    scaleY = lb.height / _tmpBoundsOnModel.height
    createMatrixForMask(false, lb, scaleX, scaleY)                            // 9.7
    clipContext._matrixForMask.setMatrix(_tmpMatrixForMask.getArray())
    clipContext._matrixForDraw.setMatrix(_tmpMatrixForDraw.getArray())
    if (type == Offscreen) clipContext._matrixForDraw.multiplyByMatrix(renderer.getMvpMatrix().getInvert())   // "clipContext * mvp^-1"
    for i in 0..clipContext._clippingIdCount:
        clipDrawIndex = clipContext._clippingIdList[i]
        if (!model.getDrawableDynamicFlagVertexPositionsDidChange(clipDrawIndex)) continue   // "頂点情報が更新されておらず、信頼性がない場合は描画をパスする"
        renderer.setIsCulling(model.getDrawableCulling(clipDrawIndex) != false)
        if (!_clearedMaskBufferFlags[clipContext._bufferIndex]) {
            gl.clearColor(1.0, 1.0, 1.0, 1.0); gl.clear(COLOR_BUFFER_BIT)     // "1が無効（描かれない）領域、0が有効（描かれる）領域"
            _clearedMaskBufferFlags[clipContext._bufferIndex] = true
        }
        renderer.setClippingContextBufferForMask(clipContext)
        renderer.drawMeshWebGL(model, clipDrawIndex)                          // set 0, 8.2
_currentMaskBuffer.endDraw()
renderer.setClippingContextBufferForMask(null)
gl.viewport(lastViewport[0], lastViewport[1], lastViewport[2], lastViewport[3])
```
Ordering subtleties to keep:
* The clear is **lazy**: a render texture is cleared only right before the first mask drawable that passes the
  `VertexPositionsDidChange` test is drawn into it. If no mask drawable of a texture passes, the texture keeps last
  frame's content. With Core 6.0.1 every mask drawable passes every frame (4.2).
* The matrices of every context are recomputed every frame from the current vertex positions.
* Contexts are drawn in creation order; with one texture the FBO is bound once.

### 9.5 Bounds

`calcClippedDrawableTotalBounds(model, cc)` (exact, including two QUIRKs):
```
totalMinX = Number.MAX_VALUE;  totalMinY = Number.MAX_VALUE        // 1.7976931348623157e308
totalMaxX = Number.MIN_VALUE;  totalMaxY = Number.MIN_VALUE        // QUIRK A: 5e-324 (smallest POSITIVE subnormal), not -MAX_VALUE
for drawableIndex in cc._clippedDrawableIndexList:
    n = model.getDrawableVertexCount(drawableIndex); v = model.getDrawableVertices(drawableIndex)
    minX = minY = Number.MAX_VALUE; maxX = maxY = -Number.MAX_VALUE
    for (pi = 0; pi < n*2; pi += 2) { x = v[pi]; y = v[pi+1];
        if (x < minX) minX = x; if (x > maxX) maxX = x; if (y < minY) minY = y; if (y > maxY) maxY = y }
    if (minX == Number.MAX_VALUE) continue                          // "有効な点が一つも取れなかったのでスキップ" (0 vertices or NaN)
    if (minX < totalMinX) totalMinX = minX; if (minY < totalMinY) totalMinY = minY
    if (maxX > totalMaxX) totalMaxX = maxX; if (maxY > totalMaxY) totalMaxY = maxY
    // QUIRK B: the result block is INSIDE the loop (runs after every accepted drawable)
    if (totalMinX == Number.MAX_VALUE) { rect = (0,0,0,0); cc._isUsing = false }          // unreachable here
    else { cc._isUsing = true; rect.x = totalMinX; rect.y = totalMinY; rect.width = totalMaxX - totalMinX; rect.height = totalMaxY - totalMinY }
```
* QUIRK A: if every clipped drawable lies entirely at negative X (or negative Y), the total max stays `5e-324`
  (≈ 0), so the bounds extend to x = 0 (or y = 0). **Active for 2 of the 6 contexts of this model** (ctx2 and ctx4,
  the left-side eye parts: true maxX −0.024428565055131912 / −0.02100001648068428 → V1 uses 5e-324). Effect: the
  bounds of those contexts are 0.0590 / 0.0587 wide instead of 0.0346 / 0.0377 (≈1.71× / ≈1.56× wider), so their
  masks get proportionally fewer mask texels (softer mask edges), and the mask cell is not centred on the parts.
  Rust: initialize with
  `f64::from_bits(1)` (= `5e-324`; NOT `f64::MIN_POSITIVE` = 2.2e-308, NOT `f64::MIN`), compare in f64 with the
  vertex values widened from f32. Recommended: keep the quirk (pixel parity) behind a `const SDK_BOUNDS_QUIRK: bool = true`.
* QUIRK B: if `_clippedDrawableIndexList` is empty or every clipped drawable is skipped, `_isUsing` and the rect are
  **not updated at all** (keep last frame's values; on the first frame `_isUsing` = undefined → false, rect fields
  undefined → NaN math later). Not reachable for this model.
* Visibility of the clipped drawables is not considered: `_isUsing` is true as long as they have vertices.

`calcClippedOffscreenTotalBounds(model, cc)` (5.3) — differences: `totalMax* = -Number.MAX_VALUE` (correct init);
the drawables considered are the flattened child drawables of every clipped offscreen
(`getOffscreenChildDrawableIndexList(model, offscreenIndex, list)` for each, appending into one list, duplicates
possible); the result block is **after** the loop (so with no points: rect = (0,0,0,0), `_isUsing = false`).

`getOffscreenChildDrawableIndexList(model, offscreenIndex, list)`: `owner = offscreenOwnerIndices[offscreenIndex]`;
`getPartChildDrawableIndexList(model, owner, list)`.
`getPartChildDrawableIndexList(model, partIndex, list)`: `cdo = model.getPartsHierarchy()[partIndex].childDrawObjects`;
`list.push(...cdo.drawableIndices)`; for each `o` in `cdo.offscreenIndices`: `getOffscreenChildDrawableIndexList(model, o, list)`.

### 9.6 `setupLayoutBounds(usingClipCount)`

```
max = (_renderTextureCount <= 1) ? 36 : 32 * _renderTextureCount
if (usingClipCount <= 0 || usingClipCount > max) {
    if (usingClipCount > max) CubismLogError('not supported mask count : {0}\n[Details] render texture count : {1}, mask count : {2}', usingClipCount - max, _renderTextureCount, usingClipCount)
    for cc in _clippingContextListForMask: cc._layoutChannelIndex = 0; cc._layoutBounds = (0, 0, 1, 1); cc._bufferIndex = 0   // "どうせ毎回消すので固定"
    return
}
layoutCountMaxValue = (_renderTextureCount <= 1) ? 9 : 8
countPerSheetDiv = Math.ceil(usingClipCount / _renderTextureCount)      // contexts per texture
reduceLayoutTextureCount = usingClipCount % _renderTextureCount         // textures that get one more (see below)
divCount = ~~(countPerSheetDiv / 4)                                     // base contexts per channel
modCount = countPerSheetDiv % 4                                         // first modCount channels get +1
curClipIndex = 0
for rt in 0.._renderTextureCount:
  for ch in 0..4:
    layoutCount = divCount + (ch < modCount ? 1 : 0)
    checkChannelIndex = modCount + (divCount < 1 ? -1 : 0)
    if (ch == checkChannelIndex && reduceLayoutTextureCount > 0)
        layoutCount -= !(rt < reduceLayoutTextureCount) ? 1 : 0
    if (layoutCount == 0) {}
    else if (layoutCount == 1) { cc = list[curClipIndex++]; cc.ch = ch; cc.lb = (0, 0, 1, 1); cc.buf = rt }
    else if (layoutCount == 2) for i in 0..2: { cc = list[curClipIndex++]; cc.ch = ch; cc.lb = ((i % 2) * 0.5, 0, 0.5, 1); cc.buf = rt }
    else if (layoutCount <= 4) for i in 0..layoutCount: { cc = list[curClipIndex++]; cc.ch = ch;
                                    cc.lb = ((i % 2) * 0.5, ~~(i / 2) * 0.5, 0.5, 0.5); cc.buf = rt }
    else if (layoutCount <= layoutCountMaxValue) for i in 0..layoutCount: { cc = list[curClipIndex++]; cc.ch = ch;
                                    cc.lb = ((i % 3) / 3.0, ~~(i / 3) / 3.0, 1.0/3.0, 1.0/3.0); cc.buf = rt }
    else { CubismLogError(same message as above); for i in 0..layoutCount: { cc = list[curClipIndex++]; cc.ch = 0; cc.lb = (0,0,1,1); cc.buf = 0 } }
```
(`~~` = truncate toward zero; all `lb` values are f64; `1.0/3.0` = 0.3333333333333333.)
QUIRK: cells are assigned to the first `usingClipCount` contexts **in list order**, not to the contexts that are in
use; when some context is not in use, the last context(s) keep stale layout (or `undefined` on the first frame) while
`setupClippingContext` still draws all contexts → overlapping cells/NaN. Not reachable for this model (always 6/6).

Layout table for one render texture (channel: contexts in that channel, split used):

| contexts | R | G | B | A |
|---|---|---|---|---|
| 1 | 1 (full) | – | – | – |
| 2 | 1 | 1 | – | – |
| 3 | 1 | 1 | 1 | – |
| 4 | 1 | 1 | 1 | 1 |
| 5 | 2 (halves) | 1 | 1 | 1 |
| **6 (this model)** | **2** | **2** | **1** | **1** |
| 7 | 2 | 2 | 2 | 1 |
| 8 | 2 | 2 | 2 | 2 |
| 9 | 3 (quarters) | 2 | 2 | 2 |
| 10 / 11 / 12 | 3,3,2,2 / 3,3,3,2 / 3,3,3,3 (quarters/halves) | | | |
| 13 … 16 | 4,3,3,3 … 4,4,4,4 (quarters) | | | |
| 17 … 20 | 5,4,4,4 … 5,5,5,5 (5 → ninths) | | | |
| 21 … 36 | +1 per context in R,G,B,A rotation, up to 9,9,9,9 (ninths) | | | |
| ≥ 37 | error log; every context: channel R, full cell, buffer 0 | | | |

Contexts fill channels in order: all cells of R first (index order), then G, then B, then A. Half cells: i=0 → x 0,
i=1 → x 0.5. Quarter cells: i → (x = (i%2)·0.5, y = ⌊i/2⌋·0.5). Ninth cells: i → (x = (i%3)/3, y = ⌊i/3⌋/3).
Multi-texture examples (rt = 2): 9 contexts → tex0 R:2 G:1 B:1 A:1, tex1 R:2 G:0 B:1 A:1; 17 → tex0 3,2,2,2 / tex1
3,1,2,2; 64 → 8 per channel in both; 65 → error path.

### 9.7 `createMatrixForMask(isRightHanded, lb, scaleX, scaleY)`

```
_tmpMatrix.loadIdentity()
_tmpMatrix.translateRelative(-1.0, -1.0)               // "Layout0..1 を -1..1に変換"
_tmpMatrix.scaleRelative(2.0, 2.0)
_tmpMatrix.translateRelative(lb.x, lb.y)               // "view to Layout0..1"
_tmpMatrix.scaleRelative(scaleX, scaleY)
_tmpMatrix.translateRelative(-_tmpBoundsOnModel.x, -_tmpBoundsOnModel.y)
_tmpMatrixForMask.setMatrix(_tmpMatrix.getArray())     // model → mask NDC
_tmpMatrix.loadIdentity()
_tmpMatrix.translateRelative(lb.x, lb.y * (isRightHanded ? -1.0 : 1.0))
_tmpMatrix.scaleRelative(scaleX, scaleY * (isRightHanded ? -1.0 : 1.0))
_tmpMatrix.translateRelative(-_tmpBoundsOnModel.x, -_tmpBoundsOnModel.y)
_tmpMatrixForDraw.setMatrix(_tmpMatrix.getArray())     // model → mask UV [0,1]
```
Because of the `M ← T·M` semantics, the vertex sees the operations in **reverse order of the calls**:
`maskNdc = ((p − tmp.xy)·scale + lb.xy)·2 − 1`, `maskUv = (p − tmp.xy)·scale + lb.xy`. Every intermediate is rounded
to f32 (each `translateRelative/scaleRelative` argument is stored into a `Float32Array` and each product is stored
into `_tr`). Always called with `isRightHanded = false` by the WebGL renderer.

### 9.8 Channel colors and mask texture semantics

`getChannelFlagAsColor(ch)` returns the shared `CubismTextureColor` instances `R=(1,0,0,0)`, `G=(0,1,0,0)`,
`B=(0,0,1,0)`, `A=(0,0,0,1)`. Mask texture after generation: per channel, `1` = not covered, `0` = covered (product of
`1 − alpha·inside` over the context's mask drawables). See 7.7 for the shader math.

### 9.9 High-precision mask path (not used by V1; implement cheaply or stub)

`setupMatrixForHighPrecision(model, isRightHanded)` (called from `doDrawModel` with `false`):
```
usingClipCount = count of contexts with _isUsing after calcClippedDrawableTotalBounds(model, cc) for all
if (usingClipCount > 0) {
    setupLayoutBounds(0)                                // → every context: channel 0, full cell, buffer 0
    resize/reset _clearedMaskBufferFlags to _renderTextureCount × false (same two-branch code as 9.4 but via .length)
    for clipContext in list:
        rect = _allClippedDrawRect; lb = _layoutBounds; margin = 0.05; ppu = model.getPixelsPerUnit()
        maskPixelSize = clipContext.getClippingManager().getClippingMaskBufferSize()
        physicalMaskWidth = lb.width * maskPixelSize; physicalMaskHeight = lb.height * maskPixelSize
        _tmpBoundsOnModel.setRect(rect)
        if (_tmpBoundsOnModel.width * ppu > physicalMaskWidth) { _tmpBoundsOnModel.expand(rect.width * margin, 0.0); scaleX = lb.width / _tmpBoundsOnModel.width }
        else scaleX = ppu / physicalMaskWidth
        if (_tmpBoundsOnModel.height * ppu > physicalMaskHeight) { _tmpBoundsOnModel.expand(0.0, rect.height * margin); scaleY = lb.height / _tmpBoundsOnModel.height }
        else scaleY = ppu / physicalMaskHeight
        createMatrixForMask(isRightHanded, lb, scaleX, scaleY); copy _tmpMatrixForMask/_tmpMatrixForDraw into the context
}
```
The masks are then rendered per drawable right before it is drawn (6.3 high-precision branch), each time clearing the
whole buffer. `setupMatrixForOffscreenHighPrecision(model, isRightHanded, mvp)`: identical but with
`calcClippedOffscreenTotalBounds`, an early `return` when `usingClipCount <= 0`, and finally
`clipContext._matrixForDraw.multiplyByMatrix(mvp.getInvert())` for every context.

### 9.10 This model's 6 contexts — golden values (default parameters, after two `update()` calls)

Computed by `.../render-spec-probe/masks.cjs` (independent re-implementation, exact f32/f64 semantics; identical to
the earlier `l2d-probe/golden.jsonl`). Matrices: all elements identity except `tr[0], tr[5], tr[12], tr[13]`.
`u_baseColor` = SetupMask rect uniform. All 6 contexts use buffer 0; `_isUsing = true` for all.

| ctx | masks → clipped | ch | layoutBounds (x,y,w,h) | allClippedDrawRect (x, y, w, h) |
|---|---|---|---|---|
| 0 | [18] → [3,9] | R | (0, 0, 0.5, 1) | (-0.0773627832531929, 0.5326372385025024, 0.15954887121915817, 0.16458606719970703) |
| 1 | [46] → [19,20,21,22,23] | R | (0.5, 0, 0.5, 1) | (0.018705639988183975, 0.5207143425941467, 0.0421590693295002, 0.056571364402770996) |
| 2 | [45] → [28,29,30,31,32] | G | (0, 0, 0.5, 1) | (-0.05900000035762787, 0.5222856998443604, 0.05900000035762787, 0.05285722017288208) ← QUIRK A |
| 3 | [33] → [34,35] | G | (0.5, 0, 0.5, 1) | (-0.05271623283624649, 0.46300840377807617, 0.1055753044784069, 0.056157052516937256) |
| 4 | [28] → [45] inverted | B | (0, 0, 1, 1) | (-0.058714333921670914, 0.5294286012649536, 0.058714333921670914, 0.026285767555236816) ← QUIRK A |
| 5 | [19] → [46] inverted | A | (0, 0, 1, 1) | (0.021000003442168236, 0.5294286012649536, 0.0377142783254385, 0.02628570795059204) |

| ctx | tmpBounds (x, y, w, h) | scaleX | scaleY |
|---|---|---|---|
| 0 | (-0.08534022681415081, 0.524407935142517, 0.17550375834107398, 0.18104467391967774) | 2.848941838774188 | 5.523498583800703 |
| 1 | (0.016597686521708965, 0.5178857743740082, 0.04637497626245022, 0.062228500843048096) | 10.781676677748504 | 16.06980702495448 |
| 2 | (-0.06195000037550926, 0.5196428388357163, 0.06490000039339065, 0.058142942190170285) | 7.704160199834444 | 17.198992041532104 |
| 3 | (-0.057994998060166836, 0.4602005511522293, 0.1161328349262476, 0.06177275776863098) | 4.305414573901814 | 16.18836581241016 |
| 4 | (-0.06165005061775446, 0.5281143128871918, 0.064585767313838, 0.0289143443107605) | 15.483287442274333 | 34.584910148830495 |
| 5 | (0.019114289525896312, 0.528114315867424, 0.04148570615798235, 0.028914278745651244) | 24.10468791809605 | 34.5849885724852 |

| ctx | `_matrixForMask` tr[0], tr[5], tr[12], tr[13] | `_matrixForDraw` tr[0], tr[5], tr[12], tr[13] | SetupMask `u_baseColor` |
|---|---|---|---|
| 0 | 5.697883605957031, 11.0469970703125, -0.5137413740158081, -6.793132781982422 | 2.8489418029785156, 5.52349853515625, 0.24312932789325714, -2.896566390991211 | (-1, -1, 0, 1) |
| 1 | 21.563352584838867, 32.13961410522461, -0.35790175199508667, -17.644649505615234 | 10.781676292419434, 16.069807052612305, 0.32104912400245667, -8.322324752807617 | (0, -1, 1, 1) |
| 2 | 15.408320426940918, 34.39798355102539, -0.04545450210571289, -18.874666213989258 | 7.704160213470459, 17.198991775512695, 0.47727274894714355, -8.937333106994629 | (-1, -1, 0, 1) |
| 3 | 8.61082935333252, 32.376731872558594, 0.4993850290775299, -15.899789810180664 | 4.30541467666626, 16.188365936279297, 0.7496924996376038, -7.449894905090332 | (0, -1, 1, 1) |
| 4 | 30.966575622558594, 69.1698226928711, 0.9090908765792847, -37.52957534790039 | 15.483287811279297, 34.58491134643555, 0.9545454382896423, -18.264787673950195 | (-1, -1, 1, 1) |
| 5 | 48.20937728881836, 69.16997528076172, -1.9214880466461182, -37.52965545654297 | 24.10468864440918, 34.58498764038086, -0.4607439935207367, -18.264827728271484 | (-1, -1, 1, 1) |

These are excellent native unit-test vectors for the pure-Rust clipping math (feed the same vertex arrays: dump them
once from the Core with `masks.cjs`, or compute from bounds + layout directly).

---

## 10. Render targets and the offscreen pool

### 10.1 `CubismRenderTarget_WebGL` (mask buffers; base class of offscreen targets)

Fields (constructor): `_gl = null`, `_colorBuffer = null` (WebGLTexture), `_renderTexture = null` (**the
WebGLFramebuffer**, misnamed), `_bufferWidth = 0`, `_bufferHeight = 0`, `_oldFbo = null`.

`createRenderTarget(gl, w, h, previousFramebuffer) -> boolean`:
```
destroyRenderTarget()
_colorBuffer = gl.createTexture(); gl.bindTexture(TEXTURE_2D, _colorBuffer)
gl.texImage2D(TEXTURE_2D, 0, RGBA, w, h, 0, RGBA, UNSIGNED_BYTE, null)          // zero-filled (0,0,0,0)
gl.texParameteri(TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_EDGE); gl.texParameteri(TEXTURE_2D, TEXTURE_WRAP_T, CLAMP_TO_EDGE)
gl.texParameteri(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR);    gl.texParameteri(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR)
gl.bindTexture(TEXTURE_2D, null)
fbo = gl.createFramebuffer(); if (fbo == null) { CubismLogError('Failed to create framebuffer'); return false }   // texture leaked, _gl not set
gl.bindFramebuffer(FRAMEBUFFER, fbo)
gl.framebufferTexture2D(FRAMEBUFFER, COLOR_ATTACHMENT0, TEXTURE_2D, _colorBuffer, 0)
if (gl.checkFramebufferStatus(FRAMEBUFFER) !== FRAMEBUFFER_COMPLETE) {
    CubismLogError('Framebuffer is not complete'); gl.bindFramebuffer(FRAMEBUFFER, previousFramebuffer); gl.deleteFramebuffer(fbo)
    destroyRenderTarget()          // QUIRK: on a first creation _gl is still null → TypeError thrown here
    return false
}
_renderTexture = fbo; _bufferWidth = w; _bufferHeight = h; _gl = gl
return true                        // NOTE: the new FBO stays BOUND (previousFramebuffer is only used on failure)
```
`destroyRenderTarget()`: if `_colorBuffer`: `_gl.bindTexture(TEXTURE_2D, null)`, delete, null; if `_renderTexture`:
`_gl.bindFramebuffer(FRAMEBUFFER, null)`, delete, null (leaves the default framebuffer bound).
`beginDraw(restoreFbo = null)`: if `_renderTexture == null` → `console.error('_renderTexture is null')`, return;
`_oldFbo = (restoreFbo == null) ? gl.getParameter(FRAMEBUFFER_BINDING) : restoreFbo`;
`bindFramebuffer(FRAMEBUFFER, _renderTexture)`. (In V1 `restoreFbo` is the default framebuffer = `null`, so the
current binding is queried — which is also `null`.)
`endDraw()`: `bindFramebuffer(FRAMEBUFFER, _oldFbo)`.
`clear(r, g, b, a)`: `clearColor(r, g, b, a); clear(COLOR_BUFFER_BIT)`.
Getters: `getGL()`, `getRenderTexture()` (FBO), `getColorBuffer()` (texture), `getBufferWidth()`, `getBufferHeight()`,
`isValid()` (`_renderTexture != null`), `getOldFBO()`.

`static copyBuffer(gl, src, dst)` (WebGL2 only):
```
if (src == null || dst == null) return
if (!(gl instanceof WebGL2RenderingContext)) throw new Error('WebGL2RenderingContext is required for buffer copy.')
prev = gl.getParameter(FRAMEBUFFER_BINDING)
gl.bindFramebuffer(READ_FRAMEBUFFER, src.getRenderTexture()); gl.bindFramebuffer(DRAW_FRAMEBUFFER, dst.getRenderTexture())
gl.blitFramebuffer(0, 0, src.w, src.h, 0, 0, dst.w, dst.h, COLOR_BUFFER_BIT, NEAREST)
gl.bindFramebuffer(FRAMEBUFFER, prev)
```

This model: one 256×256 RGBA8 mask target (LINEAR, CLAMP_TO_EDGE, no mipmaps), created in `startUp`, never resized.

### 10.2 `CubismOffscreenRenderTarget_WebGL extends CubismRenderTarget_WebGL` (5.3)

Extra fields: `_offscreenIndex = -1`, `_parentOffscreenRenderTarget = null`, `_oldOffscreen = null`,
`_webGLOffscreenManager = null`.
* `initializeOffscreenManager(gl, w, h)` (private): `_gl = gl`; `_webGLOffscreenManager = CubismWebGLOffscreenManager.getInstance()`;
  if `getContainerSize(gl) === 0` → `initialize(gl, w, h)`.
* `setOffscreenRenderTarget(gl, w, h, previousFramebuffer)`: init the manager if null;
  `c = manager.getOffscreenRenderTargetContainers(gl, w, h, previousFramebuffer)`; if null →
  `CubismLogError('Failed to acquire offscreen render texture container.')`, return; `_colorBuffer = c.getColorBuffer();
  _renderTexture = c.getRenderTexture(); _bufferWidth = w; _bufferHeight = h; _gl = gl`; if `_renderTexture == null` →
  `_renderTexture = previousFramebuffer` and `CubismLogError('Failed to create offscreen render texture.')`.
* `getUsingRenderTextureState()`: `true` if manager or gl is null, else `manager.getUsingRenderTextureState(gl, _renderTexture)`.
* `startUsingRenderTexture()` / `stopUsingRenderTexture()`: forward to the manager if manager and gl are set.
* `setOffscreenIndex/getOffscreenIndex`, `setOldOffscreen/getOldOffscreen`, `setParentPartOffscreen/getParentPartOffscreen`.
* `release()`: if manager, gl and FBO set → `stopUsingRenderTexture`; delete `_colorBuffer` and `_renderTexture`
  (QUIRK: these may be pooled objects still referenced by the pool); null manager, old and parent.
* The three `_modelRenderTargets` are of this class but created with the base `createRenderTarget` (own FBO, not pooled).

### 10.3 `CubismWebGLOffscreenManager` (singleton pool, per GL context) (5.3)

`CubismRenderTargetContainer {colorBuffer = null, renderTexture = null, inUse = false}`, `clear()` nulls both and sets
`inUse = false`.
`CubismWebGLContextManager(gl)`: `offscreenRenderTargetContainers = []`, `previousActiveRenderTextureMaxCount = 0`,
`currentActiveRenderTextureCount = 0`, `hasResetThisFrame = false`, `width = 0`, `height = 0`; `release()` deletes every
container's texture and FBO, empties and nulls the list.

Manager (`getInstance()` lazy static; `release()` releases all context managers, clears the map, nulls it and the
instance; `getContextManager(gl)` creates on demand; `removeContext(gl)` releases + deletes):
* `initialize(gl, w, h)`: if the list exists: delete every container's texture + FBO, `container.clear()`, length 0;
  else create it. Set `width = w, height = h, previousMax = 0, currentActive = 0, hasResetThisFrame = false`.
* `beginFrameProcess(gl)`: if `hasResetThisFrame` return; `previousActiveRenderTextureMaxCount = 0`;
  `hasResetThisFrame = true`.
* `endFrameProcess(gl)`: `hasResetThisFrame = false`.
* `getContainerSize(gl)`: list length (0 if null).
* `getOffscreenRenderTargetContainers(gl, w, h, prevFbo)`: if `width != w || height != h || list == null` →
  `initialize(gl, w, h)`; `updateRenderTargetContainerCount(gl)`; return the first unused container (marking it
  `inUse`) or `createOffscreenRenderTargetContainer(gl, w, h, prevFbo)`.
* `getUsingRenderTextureState(gl, fbo)`: `inUse` of the container whose `renderTexture == fbo`; `true` if none.
* `startUsingRenderTexture(gl, fbo)`: matching container → `inUse = true`; `updateRenderTargetContainerCount(gl)`.
* `stopUsingRenderTexture(gl, fbo)`: matching container → `inUse = false`; `currentActive--` clamped at 0.
* `stopUsingAllRenderTextures(gl)`: all `inUse = false`; `currentActive = 0`.
* `releaseStaleRenderTextures(gl)`:
  ```
  n = list.length; if (hasResetThisFrame || n === 0) return        // "使用する量が変化する場合は開放しない"
  findPos = 0; resize = previousMax
  for (i = n; previousMax < i; --i) {
      index = i - 1
      if (list[index].inUse) {
          isFind = false
          for (; findPos < previousMax; ++findPos)
              if (!list[findPos].inUse) { swap(list[findPos], list[index]); list[findPos].inUse = true; list[index].inUse = false; isFind = true; break }
          if (!isFind) { resize = i; break }
      }
      c = list[index]; bindTexture(TEXTURE_2D, null); deleteTexture(c.colorBuffer); bindFramebuffer(FRAMEBUFFER, null); deleteFramebuffer(c.renderTexture); c.clear()
  }
  updateSize(list, resize)                                           // truncate (or pad with null)
  ```
* `getPreviousActiveRenderTextureCount(gl)`, `getCurrentActiveRenderTextureCount(gl)`: getters.
* `updateRenderTargetContainerCount(gl)`: `++currentActive`; `previousMax = max(currentActive, previousMax)`.
* `getUnusedOffscreenRenderTargetContainer(gl)`: first `inUse == false` → set `inUse = true`, return; else `null`.
* `createOffscreenRenderTargetContainer(gl, w, h, prevFbo)`: `rt = new CubismRenderTarget_WebGL()`; if
  `!rt.createRenderTarget(gl, w, h, prevFbo)` → `CubismLogError('Failed to create offscreen render texture.')`, return
  null; push and return `new Container(rt.getColorBuffer(), rt.getRenderTexture(), true)`.

V1 calls `beginFrameProcess(gl)` at the start and `endFrameProcess(gl)` + `releaseStaleRenderTextures(gl)` at the end
of every `LAppLive2DManager.onUpdate()` (also on the early-return path without a model). With no offscreens the list is
always empty → no GL calls. Rust: implement as a per-renderer `OffscreenPool` behind the `blend53` feature; the
begin/end-frame hooks become no-ops otherwise.

`updateSize(arr, newSize, value = null, callPlacementNew = null)` (cubismarrayutils): grow by assigning `value` (or,
when `callPlacementNew` and `value` is a constructor, `JSON.parse(JSON.stringify(new value()))`) to new slots; shrink by
`arr.length = newSize`.

---

## 11. GL state contract

### 11.1 State the renderer sets itself (per frame)

Via `preDraw` (×3 per frame): SCISSOR/STENCIL/DEPTH test disabled, BLEND enabled, colorMask all true, frontFace CW,
ARRAY/ELEMENT buffers unbound. Per draw: CULL_FACE enable/disable, frontFace CCW, program, attribute pointers
(attribute arrays are enabled and never disabled), texture units 0/1(/2), uniforms, `blendFuncSeparate`. Mask pass:
viewport `0,0,256,256`, mask FBO, `clearColor(1,1,1,1)` + clear, then FBO and viewport restored to
`lastFbo`/`lastViewport`.

State the renderer **relies on but never sets** (must stay at WebGL defaults): `blendEquation` = FUNC_ADD,
`cullFace` = BACK, `depthMask`, stencil masks, `polygonOffset`, `sampleCoverage`, `DITHER`, `UNPACK_*` (irrelevant to
drawing). V1's host never changes them.

### 11.2 State left behind after `drawModel` (V1, this model)

Framebuffer = `lastFbo` (null); viewport = `lastViewport`; BLEND on with the last drawable's `blendFuncSeparate`
(drawable 0 → `ONE, ONE_MINUS_SRC_ALPHA, ONE, ONE_MINUS_SRC_ALPHA`); depth/scissor/stencil off; CULL_FACE off;
frontFace CCW; program `null`; ARRAY_BUFFER = uv buffer; ELEMENT_ARRAY_BUFFER = index buffer; ACTIVE_TEXTURE =
TEXTURE0 with the model texture; TEXTURE1 = mask texture; `clearColor = (1,1,1,1)`. V1's host resets clear color,
depth test and blend func at the start of the next frame (12.4). If V2 shares the context with other drawing, it must
set its own state (or enable the profile below).

### 11.3 `CubismRendererProfile_WebGL` (save/restore — unused in V1)

`save()` (logs `'gl' is null…` and returns if no gl): reads `ARRAY_BUFFER_BINDING`, `ELEMENT_ARRAY_BUFFER_BINDING`,
`CURRENT_PROGRAM`, `ACTIVE_TEXTURE`; then `activeTexture(TEXTURE1)` → `TEXTURE_BINDING_2D`; `activeTexture(TEXTURE0)` →
`TEXTURE_BINDING_2D`; `getVertexAttrib(0..3, VERTEX_ATTRIB_ARRAY_ENABLED)`; `isEnabled(SCISSOR_TEST, STENCIL_TEST,
DEPTH_TEST, CULL_FACE, BLEND)`; `FRONT_FACE`; `COLOR_WRITEMASK`; `BLEND_SRC_RGB, BLEND_DST_RGB, BLEND_SRC_ALPHA,
BLEND_DST_ALPHA`. `restore()`: `useProgram(last)`; attrib arrays 0–3 enable/disable; the 5 enables; `frontFace`;
`colorMask`; `bindBuffer(ARRAY)`, `bindBuffer(ELEMENT_ARRAY)`; `activeTexture(TEXTURE1)` + bind; `activeTexture(TEXTURE0)`
+ bind; `activeTexture(last)`; `blendFuncSeparate(last 4)`. `setGl(gl)`. Only reachable through the commented-out
`saveProfile()/restoreProfile()` calls → do not port unless needed.

---

## 12. Host glue that determines the renderer's input (sample layer, V1)

Owned by the player subsystem; summarized here because rendering parity depends on it.

### 12.1 Canvas and context (`ReamaCubismPlayer.beginInitialize` / `LAppSubdelegate.initialize`)

* The player creates `<canvas>` with inline style `display:block; width:100%; height:100%; pointer-events:none`,
  replaces the host element's content with it, and retries via `requestAnimationFrame` until
  `canvas.clientWidth || host.clientWidth` and the height are > 0.
* `LAppSubdelegate.initialize(canvas)`: context (2.1) → `resizeCanvas()` (`CanvasSize === "auto"`):
  `canvas.width = canvas.clientWidth * window.devicePixelRatio`, `canvas.height = canvas.clientHeight *
  window.devicePixelRatio` (the DOM setter truncates toward zero), `gl.viewport(0, 0, gl.drawingBufferWidth,
  gl.drawingBufferHeight)` → `_frameBuffer = gl.getParameter(FRAMEBUFFER_BINDING)` (= `null`) → `gl.enable(BLEND)`,
  `gl.blendFunc(SRC_ALPHA, ONE_MINUS_SRC_ALPHA)` → `LAppView.initialize` (3.5) → `setOffscreenSize(w, h)` →
  `initializeSprite()` (compiles an unused sprite program because `BackImageName`/`GearImageName` are `""`) →
  `LAppLive2DManager.initialize` (creates `LAppModel`, starts loading) → `ResizeObserver` on the canvas sets
  `_needResize = true`.

### 12.2 Renderer creation and textures

See 5.8. `createRenderer(canvas.width, canvas.height)`, `maskBufferCount = 1`, `setIsPremultipliedAlpha(true)`,
`startUp(gl)`, `loadShaders("/resources/live2d/shaders/")`.

### 12.3 Texture upload (`LAppTextureManager.createTextureFromPngFile`, BUNDLE 16598–16661)

```
if a TextureInfo with same fileName and usePremultply exists → callback(it) synchronously; return
cachedLoadImage(fileName)                     // Image(), decoding="async", resolves after load (+ decode() if available)
 .then(img => {
   if (!glManager || !gl) return
   tex = gl.createTexture(); gl.bindTexture(TEXTURE_2D, tex)                     // on whatever unit is active
   gl.texParameteri(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR_MIPMAP_LINEAR)
   gl.texParameteri(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR)                      // wrap left at REPEAT
   if (usePremultiply) gl.pixelStorei(UNPACK_PREMULTIPLY_ALPHA_WEBGL, 1)        // global, never reset
   max = gl.getParameter(MAX_TEXTURE_SIZE)
   if (img.width > max || img.height > max) {
      console.warn(`[ReamaCubism] Texture ${fileName} (${img.width}x${img.height}) exceeds MAX_TEXTURE_SIZE (${max}). Downscaling...`)
      scale = max / Math.max(w, h); c = <canvas> floor(w*scale) × floor(h*scale); ctx2d.drawImage(img, 0, 0, cw, ch)
      if (ctx2d) gl.texImage2D(TEXTURE_2D, 0, RGBA, RGBA, UNSIGNED_BYTE, c)
   } else gl.texImage2D(TEXTURE_2D, 0, RGBA, RGBA, UNSIGNED_BYTE, img)          // no UNPACK_FLIP_Y
   gl.generateMipmap(TEXTURE_2D); gl.bindTexture(TEXTURE_2D, null)
   info = {fileName, width: img.width, height: img.height, id: tex, img, usePremultply}; push; callback(info)
 })
 .catch(e => console.error(`[ReamaCubism] Failed to load texture image: ${fileName}`, e))
```
For parity the Rust port should upload from an `HtmlImageElement` with `UNPACK_PREMULTIPLY_ALPHA_WEBGL = true`
(browser premultiplication), generate mipmaps and use the same filters. The 4096² texture costs ~85 MiB of GPU memory
with mipmaps.

### 12.4 Frame loop and GL state at frame start

```
ReamaCubismPlayer.tick = () => { if (destroyed || !subdelegate) return; LAppPal.updateTime(); subdelegate.update(); rafHandle = requestAnimationFrame(tick) }
LAppSubdelegate.update():
   if (gl.isContextLost()) return                       // no restore handling anywhere in V1
   if (_needResize) { onResize(); _needResize = false } // resizeCanvas(); view.initialize(); setOffscreenSize(w, h) → renderer.setRenderTargetSize
   gl.clearColor(0, 0, 0, 0); gl.enable(DEPTH_TEST); gl.depthFunc(LEQUAL)
   gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT); gl.clearDepth(1)
   gl.enable(BLEND); gl.blendFunc(SRC_ALPHA, ONE_MINUS_SRC_ALPHA)
   LAppView.render(): gl.useProgram(spriteProgram); (no sprites); gl.flush(); manager.setViewMatrix(view); manager.onUpdate()
LAppLive2DManager.onUpdate(): offscreenManager.beginFrameProcess(gl); projection (3.9); model.update(); if (driver) { driver(model); model.getModel().update() }
   model.draw(projection) → LAppModel.doDraw(): renderer.setRenderState(subdelegate.getFrameBuffer() /* null */, [0, 0, canvas.width, canvas.height]);
                                              renderer.drawModel("/resources/live2d/shaders/")
   offscreenManager.endFrameProcess(gl); offscreenManager.releaseStaleRenderTextures(gl)
```
Everything the renderer draws is premultiplied (`ONE, ONE_MINUS_SRC_ALPHA`) onto a transparent framebuffer, and the
canvas context has `premultipliedAlpha: true`, so the page composites it correctly over the UI background. Any
exception thrown inside `subdelegate.update()` prevents the `requestAnimationFrame` re-arm → the animation stops
(V2: re-arm first or catch).

### 12.5 Context loss

V1 only skips frames while `isContextLost()`; there is no `webglcontextlost`/`webglcontextrestored` handling, so after
a restore all GL objects are invalid and nothing renders until reload. V2 may add proper restore (re-create renderer,
textures, shaders) — DEVIATION-OK, recommended.

---

## 13. User-visible strings (console output)

### 13.1 Printed in V1 (direct `console.*`, verbatim)

Renderer / framework:
* `console.error(`Error loading ${file.path} shader:`, error)` — a shader fetch failed (`file.path` = full URL).
* `console.error('Failed to load shaders:', error)` — registration threw (7.3).
* `console.error('Cubism shader link failed.', linkLog)`
* `console.error('Cubism shader compile failed.', log)`
* `console.error('_renderTexture is null')` — `beginDraw` on an invalid target.
* `Error('WebGL2RenderingContext is required for buffer copy.')` — thrown by `copyBuffer` on WebGL1.
* `Error(`Failed to fetch ${url}: ${res.status}`)` — rejection reason from `cachedFetchText` (appears inside the
  "Error loading … shader:" message).

Host glue (player/sample layer, for reference):
* `console.error("Cannot initialize WebGL context.")`
* `console.warn(`[ReamaCubism] Texture ${fileName} (${img.width}x${img.height}) exceeds MAX_TEXTURE_SIZE (${maxTextureSize}). Downscaling...`)`
* `console.error(`[ReamaCubism] Failed to load texture image: ${fileName}`, e)`
* `console.log(`[ReamaCubism] Loading ${textureCount} texture(s)...`)`, `console.log("getTextureFileName null")`,
  `console.log(`[ReamaCubism] Loading texture ${modelTextureNumber}: ${texturePath}`)`,
  `console.log(`[ReamaCubism] Texture ${modelTextureNumber} loaded: ${textureInfo.width}x${textureInfo.height}`)`,
  `console.log(`[ReamaCubism] All textures loaded. Model setup complete!`)`
* `console.log(`[ReamaCubism] Initializing. Canvas client size: ${canvas.clientWidth}x${canvas.clientHeight}`)`,
  `console.log(`[ReamaCubism] WebGL ready. Max texture size: ${maxTex}, Canvas buffer: ${gl.drawingBufferWidth}x${gl.drawingBufferHeight}`)`,
  `console.error("[ReamaCubism] Failed to initialize Cubism subdelegate.")`,
  `console.error("[ReamaCubism] Live2DCubismCore is not loaded.")`

### 13.2 `CubismLog*` messages (suppressed in V1)

The player sets `option.loggingLevel = LogLevel_Off (5)` and no `logFunction`, so `CubismDebug.print` returns early.
Format if enabled: `'[CSM]' + levelTag + fmt + '\n'` with `{n}` replaced by `args[n]` (`/\{(\d+)\}/g`), tags `[V]`
`[D]` `[I]` `[W]` `[E]`, printed through the Core log function only when `level >= loggingLevel`. Messages in this
subsystem (verbatim):
`This model uses a high-resolution mask because it operates in blend mode.` (I) ·
`NoPremultipliedAlpha is not allowed` (E) · `Shader program is not initialized.` (W) · `Vertex shader compile error!` (E) ·
`Fragment shader compile error!` (E) · `Failed to link program: {0}` (E) · `Shader compile log: {0} ` (E, trailing space) ·
`'gl' is null. WebGLRenderingContext is required.\nPlease call 'CubimRenderer_WebGL.startUp' function.` (E, sic "Cubim") ·
`Unknown object type: ` + type (E) ·
`not supported mask count : {0}\n[Details] render texture count : {1}, mask count : {2}` (E) ·
`The number of render textures must be specified as an integer. The decimal point is rounded down and corrected to an integer.` (W) ·
`The number of render textures must be an integer greater than or equal to 1. Set the number of render textures to 1.` (W) ·
`Failed to create framebuffer` (E) · `Framebuffer is not complete` (E) ·
`Failed to acquire offscreen render texture container.` (E) · `Failed to create offscreen render texture.` (E).
Rust: route these to `log::{info,warn,error}!` at the same levels (filtered out by default in release builds); keep
the 13.1 `console.*` messages as real console output for DevTools parity.

---

## 14. Golden test vectors and test strategy

Pure CPU math must match V1 **bit-exactly** (native `cargo test`, no browser needed):
1. `CubismMatrix44::multiply` / `translate_relative` / `scale_relative` / `multiply_by_matrix`: reproduce the MVP table
   of 3.9 (inputs: canvas size, canvas width 1.0 / height 1.4285714285714286, identity view matrix).
2. `CubismModelMatrix::new(1.0, 1.4285714285714286)` → `tr[0] = tr[5] = 1.399999976158142_f32`.
3. Clipping: from the six `allClippedDrawRect`s of 9.10 → `setup_layout_bounds(6, 1)` → channels R,R,G,G,B,A, cells as
   listed → `tmpBounds`, `scaleX/Y`, `_matrixForMask`, `_matrixForDraw`, SetupMask `u_baseColor` exactly as in 9.10.
4. Bounds QUIRK A: a context whose drawables all have x < 0 must produce `width = 5e-324 − minX` (e.g. ctx2:
   `rect.width = 0.05900000035762787`).
5. Layout table of 9.6 for 1…37 contexts and the rt = 2 examples.
6. Shader tables: `_shaderCount = 251`; blend index map of 7.5; 79 combos; slots 248–250 empty.
7. Render order → draw sequence of 6.2.
8. Dynamic flags (needs the Core; wasm-bindgen-test): after two `update()` calls every drawable has flags `33`.

GPU tests (`wasm-bindgen-test --headless --chrome/--firefox`): compile sets 0–3 on a WebGL1 context; render one frame
of the model into a 1280×720 FBO (no MSAA) with the MVP of 3.9 and compare with a V1 capture taken the same way
(tolerance ≤ 2/255 per channel; GPU rounding differs across vendors, so never require exact pixels).

---

## 15. Rust mapping

Crate `amadeus-live2d` (wasm32 only; `web-sys` features: `WebGlRenderingContext`, `WebGl2RenderingContext`,
`WebGlProgram`, `WebGlShader`, `WebGlBuffer`, `WebGlTexture`, `WebGlFramebuffer`, `WebGlUniformLocation`,
`HtmlCanvasElement`, `HtmlImageElement`, `console`; `js-sys` for typed-array views). Pure math lives in modules with no
`web-sys` dependency so it can be unit-tested natively (`#[cfg(test)]` on the host target).

```
amadeus-live2d/
  shaders/                       13 files copied byte-identically from public/resources/live2d/shaders (md5-checked test)
  src/framework/math/
      cubism_math.rs             EPSILON = 0.00001, clamp()
      matrix44.rs                CubismMatrix44
      model_matrix.rs            CubismModelMatrix
      view_matrix.rs             CubismViewMatrix
      target_point.rs            CubismTargetPoint
      rect.rs                    Rect (csmRect)
  src/framework/rendering/
      types.rs                   BlendMode, DrawableObjectType, ColorBlend, AlphaBlend, ShaderName, ShaderType, TextureColor
      model_view.rs              trait RenderModel (accessor contract of §4)
      gl.rs                      enum Gl { WebGl1(WebGlRenderingContext), WebGl2(WebGl2RenderingContext) } + forwarding macro
      shader.rs                  ShaderSources (include_str!), ShaderSet, ShaderLibrary
      render_target.rs           RenderTarget, OffscreenRenderTarget
      offscreen_pool.rs          OffscreenPool                          (feature "blend53")
      clipping.rs                ClippingContext, ClippingManager + pure fns
      renderer.rs                CubismRendererWebGl
      profile.rs                 RendererProfile                        (optional)
```

Types and functions (names recommended):

```rust
// math
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubismMatrix44 { pub tr: [f32; 16] }
impl CubismMatrix44 {
    pub const IDENTITY: [f32; 16];
    pub fn new() -> Self;                                   // identity
    pub fn multiply(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16];   // f64 accumulate, f32 store, k-order 0..4
    pub fn load_identity(&mut self); pub fn set_matrix(&mut self, tr: &[f32; 16]);
    pub fn array(&self) -> &[f32; 16]; pub fn array_mut(&mut self) -> &mut [f32; 16];
    pub fn scale_x(&self) -> f32; pub fn scale_y(&self) -> f32; pub fn translate_x(&self) -> f32; pub fn translate_y(&self) -> f32;
    pub fn transform_x(&self, src: f64) -> f64; pub fn transform_y(&self, src: f64) -> f64;
    pub fn invert_transform_x(&self, src: f64) -> f64; pub fn invert_transform_y(&self, src: f64) -> f64;
    pub fn translate_relative(&mut self, x: f64, y: f64);   // rounds x,y to f32 first
    pub fn translate(&mut self, x: f64, y: f64); pub fn translate_x_to(&mut self, x: f64); pub fn translate_y_to(&mut self, y: f64);
    pub fn scale_relative(&mut self, x: f64, y: f64); pub fn scale(&mut self, x: f64, y: f64);
    pub fn multiply_by_matrix(&mut self, m: &CubismMatrix44);  // self = m · self
    pub fn get_invert(&self) -> CubismMatrix44;
}
pub struct CubismModelMatrix { pub m: CubismMatrix44, width: f64, height: f64 }        // Deref/DerefMut to CubismMatrix44
impl CubismModelMatrix { pub fn new(w: Option<f64>, h: Option<f64>) -> Self; pub fn set_width(&mut self, w: f64); pub fn set_height(&mut self, h: f64);
    pub fn set_position(..); pub fn set_center_position(..); pub fn top/bottom/left/right/center_x/center_y/set_x/set_y(..);
    pub fn setup_from_layout(&mut self, layout: &[(String, f64)]) }   // ordered pairs = insertion order
pub struct CubismViewMatrix { pub m: CubismMatrix44, screen_left: f64, …, max_scale: f64, min_scale: f64 }
pub struct CubismTargetPoint { face_target_x: f64, … }     // update(dt), get_x(), get_y(), set(x, y)
#[derive(Clone, Copy, Default)] pub struct Rect { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }

// rendering
#[derive(Clone, Copy)] pub struct TextureColor { pub r: f64, pub g: f64, pub b: f64, pub a: f64 }   // Default = (1,1,1,1)
pub trait RenderModel {
    fn drawable_count(&self) -> usize; fn offscreen_count(&self) -> usize;
    fn render_orders(&self) -> js_sys::Int32Array;                       // or Vec<i32> copied once per frame
    fn vertex_positions(&self, i: usize) -> js_sys::Float32Array;        // zero-copy upload via buffer_data_with_array_buffer_view
    fn copy_vertex_positions(&self, i: usize, out: &mut Vec<f32>);       // for bounds math
    fn vertex_count(&self, i: usize) -> usize; fn vertex_uvs(&self, i: usize) -> js_sys::Float32Array;
    fn indices(&self, i: usize) -> js_sys::Uint16Array; fn index_count(&self, i: usize) -> i32;
    fn texture_index(&self, i: usize) -> u32; fn opacity(&self, i: usize) -> f32;
    fn multiply_color(&self, i: usize) -> TextureColor; fn screen_color(&self, i: usize) -> TextureColor;   // override-resolved (§4.4)
    fn blend_mode(&self, i: usize) -> BlendMode; fn color_blend(&self, i: usize) -> ColorBlend; fn alpha_blend(&self, i: usize) -> AlphaBlend;
    fn inverted_mask(&self, i: usize) -> bool; fn culling(&self, i: usize) -> bool;
    fn is_visible(&self, i: usize) -> bool; fn vertex_positions_did_change(&self, i: usize) -> bool;
    fn masks(&self, i: usize) -> Vec<i32>; fn mask_count(&self, i: usize) -> i32; fn parent_part_index(&self, i: usize) -> i32;
    fn is_using_masking(&self) -> bool; fn is_using_masking_for_offscreen(&self) -> bool; fn is_blend_mode_enabled(&self) -> bool;
    fn pixels_per_unit(&self) -> f64;
    // blend53 only: offscreen_owner_indices, part_parent_indices, offscreen_* accessors, parts_hierarchy
}
pub struct ShaderSet { pub program: Option<WebGlProgram>, pub a_position: i32, pub a_tex_coord: i32,
    pub u_matrix: Option<WebGlUniformLocation>, pub u_clip_matrix: Option<_>, pub s_texture0: Option<_>, pub s_texture1: Option<_>,
    pub u_base_color: Option<_>, pub u_channel_flag: Option<_>, pub u_multiply_color: Option<_>, pub u_screen_color: Option<_>,
    pub s_blend_texture: Option<_>, pub u_invert_clipping_mask: Option<_> }
pub struct ShaderLibrary { sets: Vec<ShaderSet> /* 251 slots */, state: LoadState, blend_index: HashMap<(ColorBlend, AlphaBlend), usize> }
impl ShaderLibrary { pub fn ensure_base(&mut self, gl: &Gl) -> Result<(), RenderError>;     // sets 0..9
                     pub fn ensure_blend(&mut self, gl: &Gl) -> Result<(), RenderError>;    // set 10 + 11..247 (blend53, lazy)
                     pub fn set(&self, index: usize) -> &ShaderSet; pub fn blend_base(&self, c: ColorBlend, a: AlphaBlend) -> Option<usize> }
pub fn compile_program(gl: &Gl, vs: &str, fs: &str) -> Result<WebGlProgram, RenderError>;   // logs the 13.1 strings
pub struct RenderTarget { color: Option<WebGlTexture>, fbo: Option<WebGlFramebuffer>, width: i32, height: i32, old_fbo: Option<WebGlFramebuffer> }
impl RenderTarget { pub fn create(&mut self, gl: &Gl, w: i32, h: i32, prev: Option<&WebGlFramebuffer>) -> Result<(), RenderError>;
                    pub fn destroy(&mut self, gl: &Gl); pub fn begin_draw(&mut self, gl: &Gl, restore: Option<WebGlFramebuffer>);
                    pub fn end_draw(&self, gl: &Gl); pub fn clear(&self, gl: &Gl, r: f32, g: f32, b: f32, a: f32); pub fn is_valid(&self) -> bool }
pub fn copy_buffer(gl: &Gl, src: &RenderTarget, dst: &RenderTarget) -> Result<(), RenderError>;    // NeedsWebGl2 on WebGL1
pub struct ClippingContext { pub clipping_ids: Vec<i32>, pub is_using: bool, pub layout_channel: usize, pub layout_bounds: Rect,
    pub all_clipped_draw_rect: Rect, pub matrix_for_mask: CubismMatrix44, pub matrix_for_draw: CubismMatrix44,
    pub clipped_drawables: Vec<usize>, pub clipped_offscreens: Vec<usize>, pub buffer_index: usize }
pub struct ClippingManager { pub contexts: Vec<ClippingContext>, pub for_draw: Vec<Option<usize>>, pub for_offscreen: Vec<Option<usize>>,
    pub mask_buffer_size: i32 /* 256 */, pub render_texture_count: usize, cleared: Vec<bool>, tmp_bounds: Rect }
pub const CHANNEL_COLORS: [TextureColor; 4];
pub fn find_same_clip(contexts: &[ClippingContext], masks: &[i32], count: usize) -> Option<usize>;
pub fn calc_clipped_drawable_total_bounds<'a>(drawables: impl Iterator<Item = &'a [f32]>, sdk_quirk: bool) -> Option<Rect>;  // None → keep old (QUIRK B)
pub fn setup_layout_bounds(contexts: &mut [ClippingContext], using: usize, render_texture_count: usize);
pub fn create_matrix_for_mask(right_handed: bool, lb: &Rect, scale_x: f64, scale_y: f64, tmp: &Rect) -> (CubismMatrix44, CubismMatrix44);
pub struct CubismRendererWebGl { gl: Option<Gl>, mvp: CubismMatrix44, model_color: TextureColor, culling: bool, premultiplied: bool,
    anisotropy: f32, high_precision: bool, rt_width: i32, rt_height: i32, textures: HashMap<u32, WebGlTexture>,
    sorted: Vec<(usize, DrawableObjectType)>, drawable_clipping: Option<ClippingManager>, offscreen_clipping: Option<ClippingManager>,
    clip_for_mask: Option<ClipRef>, clip_for_draw: Option<usize>, clip_for_offscreen: Option<usize>, buffers: GlBuffers,
    drawable_masks: Vec<RenderTarget>, model_render_targets: Vec<RenderTarget>, offscreen_masks: Vec<RenderTarget>,
    offscreens: Vec<OffscreenRenderTarget>, current_fbo: Option<WebGlFramebuffer>, model_root_fbo: Option<WebGlFramebuffer>,
    current_offscreen: Option<usize>, anisotropy_ext: Option<js_sys::Object>, shaders: ShaderLibrary }
impl CubismRendererWebGl {
    pub fn new(width: i32, height: i32) -> Self;
    pub fn initialize(&mut self, model: &dyn RenderModel, mask_buffer_count: usize);
    pub fn start_up(&mut self, gl: Gl, model: &dyn RenderModel) -> Result<(), RenderError>;
    pub fn bind_texture(&mut self, model_texture_no: u32, tex: WebGlTexture);
    pub fn set_is_premultiplied_alpha(&mut self, v: bool); pub fn set_mvp_matrix(&mut self, m: &CubismMatrix44);
    pub fn set_render_state(&mut self, fbo: Option<&WebGlFramebuffer>, viewport: [i32; 4]);
    pub fn draw_model(&mut self, model: &dyn RenderModel) -> Result<(), RenderError>;    // = drawModel/doDrawModel
    pub fn set_model_color(&mut self, r: f64, g: f64, b: f64, a: f64); pub fn use_high_precision_mask(&mut self, v: bool);
    pub fn set_clipping_mask_buffer_size(&mut self, size: i32, model: &dyn RenderModel);  // bug-free variant of 5.5
    pub fn set_render_target_size(&mut self, w: i32, h: i32); pub fn set_anisotropy(&mut self, n: f32);
    pub fn release(&mut self);                                                           // also on Drop
    fn pre_draw(&self); fn setup_clipping_context(&mut self, ..); fn draw_object_loop(&mut self, ..);
    fn draw_drawable(&mut self, ..); fn draw_mesh(&mut self, ..); fn setup_program_for_drawable(&mut self, ..); fn setup_program_for_mask(&mut self, ..);
}
pub enum RenderError { NoContext, ContextLost, ShaderCompile { stage: &'static str, log: String }, ShaderLink { log: String },
                       FramebufferIncomplete, FramebufferCreate, NeedsWebGl2, Unsupported(&'static str) }
```

Implementation notes:
* Use indices (`usize`) instead of shared object references for contexts/targets (the TS aliases context objects
  between `_clippingContextListForMask` and `_clippingContextListForDraw`).
* Keep the renderer **stateless about the model** (pass `&dyn RenderModel` to every call) or hold an
  `Rc<RefCell<CoreModel>>`; the TS keeps `_model`.
* Track the bound FBO in Rust (`current_fbo`) instead of `getParameter(FRAMEBUFFER_BINDING)`; results are identical
  (V1 always has the default framebuffer = `None` at these points).
* Uniform uploads: `uniform_matrix4fv_with_f32_array(Some(&loc), false, &m.tr)`; `uniform4f(..., x as f32, ...)` (values
  computed in f64 then cast, matching WebGL's conversion).
* Vertex/UV/index uploads: pass the Core's JS typed arrays straight to `buffer_data_with_array_buffer_view`.
* Shader loading is synchronous in V2 (embedded sources) — `draw_model` can render on its first call; keep a
  `LoadState::Failed` that stops retrying and reports once (DEVIATION-OK vs V1's per-frame retry).
* Feature `blend53` gates: blend/copy programs, `copy_buffer`, offscreen pipeline, `OffscreenPool`, model render
  targets. Without it, `initialize` returns `Err(Unsupported("cubism 5.3 blend/offscreen"))` if
  `is_blend_mode_enabled()` or `offscreen_count() > 0` (never the case for the shipped model), so a future model change
  fails loudly instead of rendering wrongly.
* The same wasm renderer runs in the Web build and in all three Tauri desktop builds (inside the webview); no
  `amadeus-backend`/`amadeus-desktop` involvement.

---

## 16. Open questions / risks

* **R1 — Startup shader cost / all-or-nothing loading (V1).** V1 compiles 242 programs per page load and any single
  failure (including unused blend shaders) leaves the model invisible with a per-frame retry + program leak. V2 should
  compile sets 0–3 only (lazy blend). Visually identical for this model; confirm acceptance of the deviation.
* **R2 — First-frame timing.** Embedding shaders makes the model appear as soon as the texture is uploaded (V1 waits for
  13 fetches + 242 compiles). If the UI relied on the delay (e.g. a fade-in synchronized to it), adjust there.
* **R3 — Core semantics dependency.** Mask correctness relies on Core 6.0.1 setting `VertexPositionsDidChange` on every
  `update()` and on `resetDynamicFlags()` having no visible effect (4.2). If `live2dcubismcore.min.js` is ever
  upgraded, re-run `render-spec-probe/flags.cjs`. Always call `update()` every frame.
* **R4 — Exactness.** CPU math can and should be bit-exact (f64 math, f32 stores). GPU output cannot be bit-exact
  across devices; use tolerance-based pixel tests.
* **R5 — `Number.MIN_VALUE` bounds quirk** (9.5) is active for ctx2/ctx4 of this model. Reproduce by default for
  parity; fixing it would slightly sharpen the left-eye masks (product decision).
* **R6 — WebGL availability in Tauri webviews.** WebKitGTK (Linux) can run with WebGL disabled or software-rendered
  depending on drivers/settings; WKWebView/WebView2 are fine. Verify WebGL1 + `antialias: true` on all three OSes;
  MSAA availability affects edge smoothness only.
* **R7 — Context loss.** V1 never recovers (12.5). Mobile browsers and laptops switching GPUs lose contexts; V2 should
  rebuild on `webglcontextrestored` (requires the texture/model loaders to support re-upload).
* **R8 — Frame-loop death on exceptions.** In V1 any render exception stops `requestAnimationFrame`. V2 must catch
  `RenderError`s per frame.
* **R9 — Texture path parity.** Premultiplication by the browser (`UNPACK_PREMULTIPLY_ALPHA_WEBGL`) vs. CPU or
  `ImageBitmap` premultiplication can differ by ±1/255 at edges; mipmaps + `LINEAR_MIPMAP_LINEAR` must be kept; the
  MAX_TEXTURE_SIZE (< 4096) downscale path goes through a 2D canvas. Owned by the texture/player spec.
* **R10 — Fixed 256² mask atlas.** Independent of canvas DPR; masks of small parts look soft on large/high-DPR canvases.
  Parity says keep 256; changing it is a visual change (the API exists: `set_clipping_mask_buffer_size`).
* **R11 — `setClippingMaskBufferSize` BUG** (5.5): do not replicate; design the Rust API so the GL handle cannot go
  missing.
* **R12 — Licensing.** The Framework is under the Live2D Open Software License (a Rust port is a derivative work: keep
  the copyright/license notice in ported files). `live2dcubismcore.min.js` is under the Live2D Proprietary Software
  License and must be shipped unmodified; commercial distribution (web + desktop installers) may require a Cubism SDK
  Release License depending on the publisher's revenue — confirm with the user before release.
* **R13 — Cubism 5.3 paths in SDK 5-r.5 contain latent issues** (6.5 infinite-loop risk, `release()` deleting pooled
  objects, `drawOffscreenWebGL` without a shader-loaded guard, WebGL2 requirement while V1 prefers WebGL1). If a 5.3
  model is ever adopted, V2 must switch to WebGL2 for that model and port these paths with tests.
* **R14 — Shared GL state.** The renderer leaves state dirty (11.2). If any other V2 code draws on the same context,
  it must set its own state; or port `RendererProfile` save/restore.
* **R15 — Singletons.** V1 keeps per-context singletons (shader manager, offscreen manager, framework ref-count). V2
  should own these per renderer to allow re-mounting the canvas (e.g. route changes) without leaks.
* **R16 — Per-frame call volume.** ~1,400 GL calls/frame across the wasm↔JS boundary — fine at 60 fps on desktop;
  avoid per-frame allocations (no `Float32Array::from` per uniform, reuse scratch `Vec`s for bounds).
* **R17 — Open question:** should V2 keep V1's unbounded `clientWidth × devicePixelRatio` canvas size (very large
  drawing buffers on 4K/3× DPR screens) or cap it? Rendering code is size-agnostic; this is a player decision.
* **R18 — Asset paths.** V1 loads the Core from `/resources/live2d/live2dcubismcore.min.js` and the model from
  `/resources/models/AmadeusKurisu5.0/reama5.0/` (origin-absolute, no base path). The V2 server and the Tauri asset
  protocol must serve the Core JS (unmodified) and the model files at equivalent URLs; with embedded shaders the
  renderer needs nothing else.

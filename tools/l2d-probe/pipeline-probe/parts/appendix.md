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

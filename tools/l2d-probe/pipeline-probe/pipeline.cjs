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

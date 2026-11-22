// Golden-vector harness for the amadeus-live2d "framework-model" port.
// Runs the REAL deployed V1 code (reama-cubism-runtime.js, i.e. Cubism Web Framework 5-r.5 as bundled)
// together with the REAL Live2DCubismCore (live2dcubismcore.min.js, Core 6.0.1) inside a Node vm sandbox,
// and writes deterministic golden data (JSON / JSONL) for Rust unit tests.
//
// Usage: node harness.cjs <outDir>
'use strict';
const fs = require('fs');
const vm = require('vm');
const path = require('path');

const ROOT = '/home/elvelt_admin/Elvelt/dsuv/public/resources';
const CORE = ROOT + '/live2d/live2dcubismcore.min.js';
const BUNDLE = ROOT + '/live2d/reama-cubism-runtime.js';
const MODEL_DIR = ROOT + '/models/AmadeusKurisu5.0/reama5.0/';
const OUT = process.argv[2] || path.join(__dirname, 'golden');
fs.mkdirSync(OUT, { recursive: true });

// ---------- deterministic Math.random replacement (only used by eye-blink golden) ----------
const RANDOM_SEQ = [0.3, 0.7, 0.1, 0.9, 0.5, 0.05, 0.95, 0.42];
let randomIdx = 0;
const sandboxMath = Object.create(Math);
sandboxMath.random = () => { const v = RANDOM_SEQ[randomIdx % RANDOM_SEQ.length]; randomIdx++; return v; };

const sandbox = {
  console, setTimeout, clearTimeout, setInterval, clearInterval,
  Uint8Array, Int32Array, Uint32Array, Float32Array, Float64Array, Uint16Array, Int16Array, Int8Array, ArrayBuffer, DataView,
  WebAssembly, Math: sandboxMath, Date, Promise, TextDecoder, TextEncoder,
  performance: { now: () => Date.now() },
  fetch: () => Promise.reject(new Error('fetch disabled in harness')),
  Image: class { addEventListener() {} },
  requestAnimationFrame: () => 0, cancelAnimationFrame: () => {},
  navigator: { userAgent: 'node' },
  location: { href: 'http://localhost/' },
  atob: (b) => Buffer.from(b, 'base64').toString('binary'),
  btoa: (s) => Buffer.from(s, 'binary').toString('base64'),
};
sandbox.document = { currentScript: { src: 'http://localhost/live2dcubismcore.min.js' }, createElement: () => ({ style: {}, getContext: () => null }) };
sandbox.window = sandbox; sandbox.self = sandbox; sandbox.globalThis = sandbox;
vm.createContext(sandbox);

const coreCode = fs.readFileSync(CORE, 'utf8');
let bundle = fs.readFileSync(BUNDLE, 'utf8');
const tailOld = '  var runtimeWindow = window;\n  runtimeWindow.ReamaCubismRuntime = runtime;\n  void preloadDefaultAssets();';
if (!bundle.includes(tailOld)) throw new Error('bundle tail not found');
bundle = bundle.replace(tailOld, `  globalThis.__fw = { CubismFramework, Option, CubismModelSettingJson, CubismMoc, CubismModel, CubismPhysics,
    CubismBreath, BreathParameterData, CubismLook, LookParameterData, CubismTargetPoint, CubismMotion, CubismExpressionMotion,
    CubismExpressionMotionManager, CubismMotionManager, CubismEyeBlink, CubismUpdateScheduler, CubismBreathUpdater,
    CubismLookUpdater, CubismPhysicsUpdater, CubismExpressionUpdater, CubismEyeBlinkUpdater, CubismPose, evaluateCurve,
    CubismMath, CubismVector2, CubismJson, CubismMotionQueueEntry, ACubismMotion };`);
vm.runInContext(coreCode + '\n;globalThis.__core = Live2DCubismCore;', sandbox, { filename: 'core.js' });
vm.runInContext(bundle, sandbox, { filename: 'bundle.js' });

function readAB(p) { const b = fs.readFileSync(p); return b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength); }
function abFromString(s) { const b = Buffer.from(s, 'utf8'); return b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength); }
function writeJson(name, obj) { fs.writeFileSync(path.join(OUT, name), JSON.stringify(obj, null, 1)); }
function writeJsonl(name, rows) { fs.writeFileSync(path.join(OUT, name), rows.map((r) => JSON.stringify(r)).join('\n') + '\n'); }

const MOC = MODEL_DIR + 'Live2D紅莉栖forSDK5.0.moc3';
const PHYS = MODEL_DIR + 'Live2D紅莉栖forSDK5.0.physics3.json';
const MODEL3 = MODEL_DIR + 'Live2D紅莉栖forSDK5.0.model3.json';

function main() {
  const Core = sandbox.__core;
  const FW = sandbox.__fw;
  const opt = new FW.Option();
  opt.loggingLevel = 5; // LogLevel_Off, exactly like V1 acquireFramework()
  FW.CubismFramework.startUp(opt);
  FW.CubismFramework.initialize();
  const idm = FW.CubismFramework.getIdManager();

  // ---------------- A. model info ----------------
  const mocAB = readAB(MOC);
  const moc = FW.CubismMoc.create(mocAB, false);
  const model = moc.createModel();
  const cm = model.getModel();
  const P = cm.parameters;
  const info = {
    coreVersion: Core.Version.csmGetVersion(),
    latestMocVersion: Core.Version.csmGetLatestMocVersion(),
    mocVersionFromBytes: Core.Version.csmGetMocVersion(mocAB),
    cubismMocGetMocVersion: moc.getMocVersion(),
    hasMocConsistency: FW.CubismMoc.hasMocConsistency(mocAB),
    canvas: { CanvasWidth: cm.canvasinfo.CanvasWidth, CanvasHeight: cm.canvasinfo.CanvasHeight, CanvasOriginX: cm.canvasinfo.CanvasOriginX, CanvasOriginY: cm.canvasinfo.CanvasOriginY, PixelsPerUnit: cm.canvasinfo.PixelsPerUnit, getCanvasWidth: model.getCanvasWidth(), getCanvasHeight: model.getCanvasHeight() },
    parameterCount: P.count,
    parameters: Array.from({ length: P.count }, (_, i) => ({ index: i, id: P.ids[i], type: P.types[i], min: P.minimumValues[i], max: P.maximumValues[i], default: P.defaultValues[i], value: P.values[i], repeat: P.repeats[i], keyCount: P.keyCounts[i], keyValues: Array.from(P.keyValues[i]) })),
    partCount: cm.parts.count,
    parts: Array.from({ length: cm.parts.count }, (_, i) => ({ index: i, id: cm.parts.ids[i], parent: cm.parts.parentIndices[i], offscreen: cm.parts.offscreenIndices[i], opacity: cm.parts.opacities[i] })),
    drawableCount: cm.drawables.count,
    drawableParentParts: Array.from(cm.drawables.parentPartIndices),
    offscreenCount: cm.offscreens.count,
    renderOrdersLength: cm.getRenderOrders().length,
    isBlendModeEnabled: model.isBlendModeEnabled(),
    isUsingMasking: model.isUsingMasking(),
    partsHierarchy: model.getPartsHierarchy().map((p, i) => ({ part: i, objects: p.objects.map((o) => [o.objectType === 1 ? 'Part' : 'Drawable', o.objectIndex]), childDrawables: p.childDrawObjects.drawableIndices.slice() })),
    isRepeatDefault: Array.from({ length: P.count }, (_, i) => model.isRepeat(i)),
  };
  // model setting facts
  const setting = new FW.CubismModelSettingJson(readAB(MODEL3), fs.statSync(MODEL3).size);
  info.modelSetting = {
    getModelFileName: setting.getModelFileName(), getTextureCount: setting.getTextureCount(), getTextureDirectory: setting.getTextureDirectory(),
    getTextureFileName0: setting.getTextureFileName(0), getPhysicsFileName: setting.getPhysicsFileName(), getPoseFileName: setting.getPoseFileName(),
    getExpressionCount: setting.getExpressionCount(), getMotionGroupCount: setting.getMotionGroupCount(), getMotionCount_Idle: setting.getMotionCount('Idle'),
    getEyeBlinkParameterCount: setting.getEyeBlinkParameterCount(), getLipSyncParameterCount: setting.getLipSyncParameterCount(),
    getHitAreasCount: setting.getHitAreasCount(), getUserDataFile: setting.getUserDataFile(), getLayoutMap: (() => { const m = new Map(); const r = setting.getLayoutMap(m); return { ret: r, size: m.size }; })(),
    getMotionFadeInTimeValue_Idle0: setting.getMotionFadeInTimeValue('Idle', 0), getMotionFileName_Idle0: setting.getMotionFileName('Idle', 0),
  };
  writeJson('model_info.json', info);

  // ---------------- B. V1 pipeline physics golden ----------------
  function makeV1Pipeline() {
    const moc2 = FW.CubismMoc.create(mocAB, false);
    const m = moc2.createModel();
    m.saveParameters(); // CubismUserModel.loadModel
    const physBuf = readAB(PHYS);
    const physics = FW.CubismPhysics.create(physBuf, physBuf.byteLength);
    const sched = new FW.CubismUpdateScheduler();
    sched.addUpdatableList(new FW.CubismPhysicsUpdater(physics));
    const breath = FW.CubismBreath.create();
    breath.setParameters([
      new FW.BreathParameterData(idm.getId('ParamAngleX'), 0, 15, 6.5345, 0.5),
      new FW.BreathParameterData(idm.getId('ParamAngleY'), 0, 8, 3.5345, 0.5),
      new FW.BreathParameterData(idm.getId('ParamAngleZ'), 0, 10, 5.5345, 0.5),
      new FW.BreathParameterData(idm.getId('ParamBodyAngleX'), 0, 4, 15.5345, 0.5),
      new FW.BreathParameterData(idm.getId('ParamBreath'), 0.5, 0.5, 3.2345, 1),
    ]);
    sched.addUpdatableList(new FW.CubismBreathUpdater(breath));
    const look = FW.CubismLook.create();
    look.setParameters([
      new FW.LookParameterData(idm.getId('ParamAngleX'), 30, 0, 0),
      new FW.LookParameterData(idm.getId('ParamAngleY'), 0, 30, 0),
      new FW.LookParameterData(idm.getId('ParamAngleZ'), 0, 0, -30),
      new FW.LookParameterData(idm.getId('ParamBodyAngleX'), 10, 0, 0),
      new FW.LookParameterData(idm.getId('ParamEyeBallX'), 1, 0, 0),
      new FW.LookParameterData(idm.getId('ParamEyeBallY'), 0, 1, 0),
    ]);
    const drag = new FW.CubismTargetPoint();
    sched.addUpdatableList(new FW.CubismLookUpdater(look, drag));
    sched.sortUpdatableList();
    m.saveParameters(); // LAppModel.loadCubismMotion
    return { m, physics, sched, drag };
  }
  function runPipeline(name, dts, opts = {}) {
    const { m, physics, sched, drag } = makeV1Pipeline();
    if (opts.drag) drag.set(opts.drag[0], opts.drag[1]);
    const rows = [];
    const ids = info.parameters.map((p) => p.id);
    for (let f = 0; f < dts.length; f++) {
      const dt = dts[f];
      m.loadParameters();
      // V1: motionManager.isFinished() === true -> startRandomMotion('Idle', 1) returns -1 immediately (no motions in model3.json)
      m.saveParameters();
      sched.onLateUpdate(m, dt);
      m.update();
      const vals = Array.from(m.getModel().parameters.values);
      rows.push({
        frame: f, dt, remain: physics._currentRemainTime,
        cur: physics._currentRigOutputs.map((o) => o.outputs[0]),
        prev: physics._previousRigOutputs.map((o) => o.outputs[0]),
        params: Object.fromEntries(ids.map((id, i) => [id, vals[i]])),
      });
    }
    writeJsonl(name, rows);
  }
  const N60 = Array(600).fill(1 / 60);
  runPipeline('pipeline_dt60.jsonl', N60);
  runPipeline('pipeline_dt144.jsonl', Array(400).fill(1 / 144));
  runPipeline('pipeline_dt30.jsonl', Array(300).fill(1 / 30));
  // realistic V1 deltas: Date.now() integer milliseconds, with a dropped frame, a zero delta and a long stall (> MaxDeltaTime 5s)
  const jitter = [];
  const pattern = [16, 17, 17, 16, 17, 17, 16, 17, 17, 33, 16, 17, 0, 17, 16, 17, 50, 17];
  for (let i = 0; i < 360; i++) jitter.push(pattern[i % pattern.length] / 1000);
  jitter[200] = 6.0; // tab was hidden for 6 seconds
  runPipeline('pipeline_jitter.jsonl', jitter);
  // with a drag target (exercises CubismTargetPoint + CubismLook + physics response)
  runPipeline('pipeline_drag.jsonl', Array(240).fill(1 / 60), { drag: [0.8, -0.5] });

  // ---------------- C. motion curve golden (evaluateCurve) ----------------
  const curveFiles = ['haru_normal_04.motion3.json', 'hiyori_m01.motion3.json', 'Epsilon_idle_01.motion3.json', 'haru_g_idle.motion3.json', 'mtn_02.motion3.json'];
  for (const f of curveFiles) {
    const ab = readAB(MODEL_DIR + 'motions/' + f);
    const mo = FW.CubismMotion.create(ab, ab.byteLength, null, null, false);
    const md = mo._motionData;
    const out = { file: f, duration: md.duration, fps: md.fps, curveCount: md.curveCount, fadeIn: mo.getFadeInTime(), fadeOut: mo.getFadeOutTime(), loopFlagInJsonIgnored: md.loop, isLoop: mo.getLoop(),
      curves: md.curves.map((c) => ({ id: c.id.getString(), type: c.type, segmentCount: c.segmentCount, baseSegmentIndex: c.baseSegmentIndex, fadeInTime: c.fadeInTime, fadeOutTime: c.fadeOutTime })), samples: [] };
    const endTime = md.duration + 1 / md.fps;
    const nT = Math.ceil(md.duration * 60) + 12;
    for (let k = 0; k <= nT; k++) {
      const t = k / 60;
      out.samples.push({ t, v: md.curves.map((_, c) => FW.evaluateCurve(md, c, t, false, md.duration)), vCorr: md.curves.map((_, c) => FW.evaluateCurve(md, c, t, true, endTime)) });
    }
    writeJson('curve_' + f.replace('.motion3.json', '') + '.json', out);
  }

  // ---------------- D. motion manager golden ----------------
  {
    const moc3 = FW.CubismMoc.create(mocAB, false);
    const m = moc3.createModel();
    m.saveParameters();
    const mgr = new FW.CubismMotionManager();
    const events = [];
    mgr.setEventCallback((caller, value) => events.push(value), null);
    const load = (f) => { const ab = readAB(MODEL_DIR + 'motions/' + f); const mo = FW.CubismMotion.create(ab, ab.byteLength, (self) => events.push('finished:' + f), (self) => events.push('began:' + f), false); mo.setEffectIds([], []); return mo; };
    const mA = load('hiyori_m01.motion3.json');
    const mB = load('mtn_02.motion3.json');
    const watch = ['ParamAngleX', 'ParamAngleY', 'ParamEyeLOpen', 'ParamMouthForm', 'ParamBodyAngleX', 'ParamArmLA', 'PartArmA', 'PartArmB'];
    const rows = [];
    const dt = 1 / 60;
    for (let f = 0; f < 600; f++) {
      if (f === 0) rows.push({ action: 'startMotionPriority(hiyori_m01, autoDelete=false, priority=2)', handle: typeof mgr.startMotionPriority(mA, false, 2) });
      if (f === 120) rows.push({ action: 'startMotionPriority(mtn_02, autoDelete=false, priority=2)', handle: typeof mgr.startMotionPriority(mB, false, 2) });
      m.loadParameters();
      const updated = mgr.updateMotion(m, dt);
      m.saveParameters();
      rows.push({ frame: f, userTime: mgr._userTimeSeconds, updated, isFinished: mgr.isFinished(), currentPriority: mgr.getCurrentPriority(), entries: mgr._motions.map((e) => ({ start: e.getStartTime(), fadeInStart: e.getFadeInStartTime(), end: e.getEndTime(), weight: e.getStateWeight(), finished: e.isFinished(), triggeredFadeOut: e.isTriggeredFadeOut() })), values: Object.fromEntries(watch.map((id) => [id, m.getParameterValueById(idm.getId(id))])), events: events.splice(0) });
    }
    writeJsonl('motion_manager.jsonl', rows);
  }

  // ---------------- E. expression manager golden (synthetic exp3 using real model ids) ----------------
  {
    const moc4 = FW.CubismMoc.create(mocAB, false);
    const m = moc4.createModel();
    m.saveParameters();
    const expA = JSON.stringify({ Type: 'Live2D Expression', FadeInTime: 0.5, FadeOutTime: 0.5, Parameters: [
      { Id: 'ParamMouthForm', Value: 0.5, Blend: 'Add' }, { Id: 'ParamEyeLOpen', Value: 0.5, Blend: 'Multiply' }, { Id: 'ParamBrowLY', Value: 0.7, Blend: 'Overwrite' }, { Id: 'ParamCheek', Value: 1 } ] });
    const expB = JSON.stringify({ Type: 'Live2D Expression', Parameters: [
      { Id: 'ParamMouthForm', Value: -0.5, Blend: 'Overwrite' }, { Id: 'ParamEyeLOpen', Value: -0.2, Blend: 'Add' }, { Id: 'PARAM_TERE', Value: 1, Blend: 'Add' }, { Id: 'ParamEyeROpen', Value: 2, Blend: 'Bogus' } ] });
    const A = FW.CubismExpressionMotion.create(abFromString(expA), Buffer.byteLength(expA));
    const Bx = FW.CubismExpressionMotion.create(abFromString(expB), Buffer.byteLength(expB));
    const emgr = new FW.CubismExpressionMotionManager();
    const watch = ['ParamMouthForm', 'ParamEyeLOpen', 'ParamEyeROpen', 'ParamBrowLY', 'ParamCheek', 'PARAM_TERE'];
    const rows = [{ expA, expB, parsedA: A.getExpressionParameters().map((p) => ({ id: p.parameterId.getString(), blendType: p.blendType, value: p.value })), parsedB: Bx.getExpressionParameters().map((p) => ({ id: p.parameterId.getString(), blendType: p.blendType, value: p.value })), fadeA: [A.getFadeInTime(), A.getFadeOutTime()], fadeB: [Bx.getFadeInTime(), Bx.getFadeOutTime()] }];
    const dt = 1 / 60;
    for (let f = 0; f < 300; f++) {
      if (f === 0) emgr.startMotion(A, false);
      if (f === 90) emgr.startMotion(Bx, false);
      if (f === 200) emgr.startMotion(A, false);
      m.loadParameters();
      m.saveParameters();
      emgr.updateMotion(m, dt);
      rows.push({ frame: f, userTime: emgr._userTimeSeconds, entryCount: emgr._motions.length, fadeWeights: emgr._fadeWeights.slice(), values: Object.fromEntries(watch.map((id) => [id, m.getParameterValueById(idm.getId(id))])) });
    }
    writeJsonl('expression_manager.jsonl', rows);
  }

  // ---------------- F. eye blink golden (deterministic random sequence) ----------------
  {
    const moc5 = FW.CubismMoc.create(mocAB, false);
    const m = moc5.createModel();
    const eb = FW.CubismEyeBlink.create(null);
    eb.setParameterIds([idm.getId('ParamEyeLOpen'), idm.getId('ParamEyeROpen')]);
    randomIdx = 0;
    const rows = [{ randomSequence: RANDOM_SEQ }];
    for (let f = 0; f < 900; f++) {
      eb.updateParameters(m, 1 / 60);
      rows.push({ frame: f, state: eb._blinkingState, userTime: eb._userTimeSeconds, next: eb._nextBlinkingTime, stateStart: eb._stateStartTimeSeconds, value: m.getParameterValueById(idm.getId('ParamEyeLOpen')) });
    }
    writeJsonl('eyeblink.jsonl', rows);
  }

  // ---------------- G. normalizeParameterValue / physics single-step micro golden ----------------
  {
    const rows = [];
    const ab = readAB(PHYS);
    const ph = FW.CubismPhysics.create(ab, ab.byteLength);
    rows.push({ rig: { subRigCount: ph._physicsRig.subRigCount, fps: ph._physicsRig.fps, gravity: [ph._physicsRig.gravity.x, ph._physicsRig.gravity.y], wind: [ph._physicsRig.wind.x, ph._physicsRig.wind.y], optionsGravity: [ph._options.gravity.x, ph._options.gravity.y],
      settings: ph._physicsRig.settings.map((s) => ({ inputCount: s.inputCount, outputCount: s.outputCount, particleCount: s.particleCount, baseInputIndex: s.baseInputIndex, baseOutputIndex: s.baseOutputIndex, baseParticleIndex: s.baseParticleIndex, normalizationPosition: s.normalizationPosition, normalizationAngle: s.normalizationAngle })),
      inputs: ph._physicsRig.inputs.map((i) => ({ id: i.source.id.getString(), type: i.type, weight: i.weight, reflect: i.reflect })),
      outputs: ph._physicsRig.outputs.map((o) => ({ id: o.destination.id.getString(), type: o.type, vertexIndex: o.vertexIndex, angleScale: o.angleScale, translationScale: o.translationScale, weight: o.weight, reflect: o.reflect, valueBelowMinimum: o.valueBelowMinimum === undefined ? 'undefined' : o.valueBelowMinimum })),
      particlesAfterInitialize: ph._physicsRig.particles.map((p) => ({ initialPosition: [p.initialPosition.x, p.initialPosition.y], position: [p.position.x, p.position.y], lastPosition: [p.lastPosition.x, p.lastPosition.y], lastGravity: [p.lastGravity.x, p.lastGravity.y], mobility: p.mobility, delay: p.delay, acceleration: p.acceleration, radius: p.radius })) } });
    writeJsonl('physics_rig.jsonl', rows);
  }
  console.log('golden written to', OUT);
}

(function wait(tries) {
  try { sandbox.__core.Version.csmGetVersion(); } catch (e) { if (tries < 100) return setTimeout(() => wait(tries + 1), 50); throw e; }
  main();
})(0);

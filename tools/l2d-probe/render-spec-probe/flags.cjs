// Independent probe: Core dynamic-flag semantics, colors, opacities, render orders,
// clipping contexts and the Number.MIN_VALUE bounds quirk for the V1 model.
'use strict';
const fs = require('fs');
const vm = require('vm');
const corePath = process.argv[2];
const mocPath = process.argv[3];
const ctx = { console, setTimeout, clearTimeout, WebAssembly, TextDecoder, Uint8Array, Uint16Array, Float32Array, Int32Array, ArrayBuffer, Math, performance, Date };
ctx.window = ctx; ctx.self = ctx; ctx.globalThis = ctx; ctx.document = { currentScript: { src: '' } }; ctx.atob = atob; ctx.btoa = btoa;
vm.createContext(ctx);
vm.runInContext(fs.readFileSync(corePath, 'utf8') + '\n;globalThis.__core = Live2DCubismCore;', ctx);
const Core = ctx.__core;

function run() {
  const buf = fs.readFileSync(mocPath);
  const ab = buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength);
  const moc = Core.Moc.fromArrayBuffer(ab);
  const model = Core.Model.fromMoc(moc);
  const d = model.drawables;
  const flags = () => Array.from(d.dynamicFlags);
  const hist = (a) => { const h = {}; a.forEach(v => h[v] = (h[v] || 0) + 1); return JSON.stringify(h); };
  console.log('Utils keys:', Object.keys(Core.Utils || {}).join(','));
  console.log('A: before any update:', hist(flags()));
  model.update();
  console.log('B: after 1st update (no reset):', hist(flags()));
  d.resetDynamicFlags();
  console.log('C: after reset:', hist(flags()));
  model.update();
  console.log('D: after 2nd update, no param change:', hist(flags()));
  d.resetDynamicFlags();
  console.log('E: after reset:', hist(flags()));
  // change a parameter
  const p = model.parameters;
  const ids = Array.from(p.ids);
  const iAngleX = ids.indexOf('ParamAngleX');
  console.log('param ids:', ids.join(','));
  p.values[iAngleX] = 10;
  model.update();
  console.log('F: after update with ParamAngleX=10:', hist(flags()));
  d.resetDynamicFlags();
  console.log('G: after reset:', hist(flags()));
  // V1 per-frame sequence: CubismModel.update() = update+reset, twice
  for (let f = 0; f < 3; f++) {
    p.values[iAngleX] = f * 3;
    model.update(); d.resetDynamicFlags();
    model.update(); d.resetDynamicFlags();
    console.log(`H${f}: V1 frame (update+reset x2) flags at draw time:`, hist(flags()));
  }
  // no parameter change across V1 frames
  model.update(); d.resetDynamicFlags(); model.update(); d.resetDynamicFlags();
  console.log('I: V1 frame without param change:', hist(flags()));
  // opacity / colors
  console.log('opacities distinct:', JSON.stringify(Array.from(new Set(Array.from(d.opacities)))));
  const mc = Array.from(d.multiplyColors), sc = Array.from(d.screenColors);
  const groups = (a) => { const s = new Set(); for (let i = 0; i < a.length; i += 4) s.add(a.slice(i, i + 4).join(',')); return Array.from(s); };
  console.log('multiplyColors distinct:', JSON.stringify(groups(mc)));
  console.log('screenColors distinct:', JSON.stringify(groups(sc)));
  console.log('renderOrders:', Array.from(model.getRenderOrders()).join(','));
  console.log('drawOrders:', Array.from(d.drawOrders).join(','));
  console.log('textureIndices distinct:', JSON.stringify(Array.from(new Set(Array.from(d.textureIndices)))));
  console.log('vertexUv range:', (() => { let mn = 1e9, mx = -1e9; for (const uv of d.vertexUvs) for (const v of uv) { if (v < mn) mn = v; if (v > mx) mx = v; } return mn + '..' + mx; })());
  console.log('parts parentIndices:', Array.from(model.parts.parentIndices).join(','));
  console.log('parts offscreenIndices:', model.parts.offscreenIndices ? Array.from(model.parts.offscreenIndices).join(',') : 'n/a');
  console.log('offscreens:', model.offscreens ? JSON.stringify({ count: model.offscreens.count }) : 'n/a');
  // reset params to defaults & run one V1 frame for bounds analysis
  for (let i = 0; i < p.count; i++) p.values[i] = p.defaultValues[i];
  model.update(); d.resetDynamicFlags(); model.update(); d.resetDynamicFlags();
  // clipping contexts exactly like findSameClip / initializeForDrawable
  const contexts = [];
  for (let i = 0; i < d.count; i++) {
    const c = d.maskCounts[i]; if (c <= 0) continue;
    const masks = d.masks[i];
    let found = null;
    for (const cc of contexts) {
      if (cc.count !== c) continue;
      let same = 0; for (let j = 0; j < c; j++) { const id = cc.ids[j]; for (let k = 0; k < c; k++) if (masks[k] === id) { same++; break; } }
      if (same === c) { found = cc; break; }
    }
    if (!found) { found = { ids: Array.from(masks).slice(0, c), count: c, clipped: [] }; contexts.push(found); }
    found.clipped.push(i);
  }
  contexts.forEach((cc, k) => {
    let tminX = Number.MAX_VALUE, tminY = Number.MAX_VALUE, tmaxX = Number.MIN_VALUE, tmaxY = Number.MIN_VALUE; // V1 (quirk)
    let cmaxX = -Number.MAX_VALUE, cmaxY = -Number.MAX_VALUE; // correct
    for (const di of cc.clipped) {
      const v = d.vertexPositions[di]; const n = d.vertexCounts[di];
      let minX = Number.MAX_VALUE, minY = Number.MAX_VALUE, maxX = -Number.MAX_VALUE, maxY = -Number.MAX_VALUE;
      for (let pi = 0; pi < n * 2; pi += 2) { const x = v[pi], y = v[pi + 1]; if (x < minX) minX = x; if (x > maxX) maxX = x; if (y < minY) minY = y; if (y > maxY) maxY = y; }
      if (minX == Number.MAX_VALUE) continue;
      if (minX < tminX) tminX = minX; if (minY < tminY) tminY = minY; if (maxX > tmaxX) tmaxX = maxX; if (maxY > tmaxY) tmaxY = maxY;
      if (maxX > cmaxX) cmaxX = maxX; if (maxY > cmaxY) cmaxY = maxY;
    }
    const quirk = (tmaxX !== cmaxX) || (tmaxY !== cmaxY);
    console.log(`ctx${k}: masks=[${cc.ids}] clipped=[${cc.clipped}] minX=${tminX} minY=${tminY} V1maxX=${tmaxX} V1maxY=${tmaxY} trueMaxX=${cmaxX} trueMaxY=${cmaxY} MIN_VALUE-quirk-active=${quirk}`);
  });
}
let tries = 0;
(function wait() { try { run(); } catch (e) { if (tries++ < 50 && /not|undefined|ready|null/i.test(String(e))) setTimeout(wait, 100); else console.error('fail', e); } })();

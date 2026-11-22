// Independent re-implementation of CubismClippingManager_WebGL.setupClippingContext math
// (layout, bounds incl. Number.MIN_VALUE quirk, createMatrixForMask) with exact Float32Array semantics.
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

class M44 {
  constructor() { this._tr = new Float32Array(16); this.loadIdentity(); }
  static multiply(a, b, dst) { const c = new Float32Array(16); for (let i = 0; i < 4; ++i) for (let j = 0; j < 4; ++j) for (let k = 0; k < 4; ++k) c[j + i * 4] += a[k + i * 4] * b[j + k * 4]; for (let i = 0; i < 16; ++i) dst[i] = c[i]; }
  loadIdentity() { this.setMatrix(new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1])); }
  setMatrix(t) { for (let i = 0; i < 16; ++i) this._tr[i] = t[i]; }
  getArray() { return this._tr; }
  translateRelative(x, y) { M44.multiply(new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, x, y, 0, 1]), this._tr, this._tr); }
  scaleRelative(x, y) { M44.multiply(new Float32Array([x, 0, 0, 0, 0, y, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]), this._tr, this._tr); }
}
class Rect { constructor(x, y, w, h) { this.x = x; this.y = y; this.width = w; this.height = h; } setRect(r) { this.x = r.x; this.y = r.y; this.width = r.width; this.height = r.height; } expand(w, h) { this.x -= w; this.y -= h; this.width += w * 2.0; this.height += h * 2.0; } getRight() { return this.x + this.width; } getBottom() { return this.y + this.height; } }

function layout(contexts, usingClipCount, rtc) {
  const max = rtc <= 1 ? 36 : 32 * rtc;
  if (usingClipCount <= 0 || usingClipCount > max) { for (const c of contexts) { c.ch = 0; c.lb = new Rect(0, 0, 1, 1); c.buf = 0; } return; }
  const lmax = rtc <= 1 ? 9 : 8;
  let cps = usingClipCount / rtc; const reduce = usingClipCount % rtc; cps = Math.ceil(cps);
  let div = cps / 4; const mod = cps % 4; div = ~~div; let cur = 0;
  for (let r = 0; r < rtc; r++) for (let ch = 0; ch < 4; ch++) {
    let lc = div + (ch < mod ? 1 : 0); const chk = mod + (div < 1 ? -1 : 0);
    if (ch == chk && reduce > 0) lc -= !(r < reduce) ? 1 : 0;
    if (lc == 0) { } else if (lc == 1) { const c = contexts[cur++]; c.ch = ch; c.lb = new Rect(0, 0, 1, 1); c.buf = r; }
    else if (lc == 2) { for (let i = 0; i < lc; i++) { const c = contexts[cur++]; c.ch = ch; c.lb = new Rect((~~(i % 2)) * 0.5, 0, 0.5, 1); c.buf = r; } }
    else if (lc <= 4) { for (let i = 0; i < lc; i++) { const c = contexts[cur++]; c.ch = ch; c.lb = new Rect((~~(i % 2)) * 0.5, (~~(i / 2)) * 0.5, 0.5, 0.5); c.buf = r; } }
    else if (lc <= lmax) { for (let i = 0; i < lc; i++) { const c = contexts[cur++]; c.ch = ch; c.lb = new Rect((~~(i % 3)) / 3.0, (~~(i / 3)) / 3.0, 1 / 3, 1 / 3); c.buf = r; } }
    else { for (let i = 0; i < lc; i++) { const c = contexts[cur++]; c.ch = 0; c.lb = new Rect(0, 0, 1, 1); c.buf = 0; } }
  }
}

function run() {
  const buf = fs.readFileSync(mocPath);
  const ab = buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength);
  const moc = Core.Moc.fromArrayBuffer(ab);
  const model = Core.Model.fromMoc(moc);
  const d = model.drawables;
  model.update(); d.resetDynamicFlags(); model.update(); d.resetDynamicFlags();
  const contexts = [];
  for (let i = 0; i < d.count; i++) {
    const c = d.maskCounts[i]; if (c <= 0) continue; const masks = d.masks[i]; let found = null;
    for (const cc of contexts) { if (cc.count !== c) continue; let same = 0; for (let j = 0; j < c; j++) { const id = cc.ids[j]; for (let k = 0; k < c; k++) if (masks[k] === id) { same++; break; } } if (same === c) { found = cc; break; } }
    if (!found) { found = { ids: Array.from(masks).slice(0, c), count: c, clipped: [] }; contexts.push(found); }
    found.clipped.push(i);
  }
  // bounds (V1, with MIN_VALUE quirk)
  let using = 0;
  for (const cc of contexts) {
    let tminX = Number.MAX_VALUE, tminY = Number.MAX_VALUE, tmaxX = Number.MIN_VALUE, tmaxY = Number.MIN_VALUE;
    cc.rect = new Rect(); cc.isUsing = undefined;
    for (const di of cc.clipped) {
      const v = d.vertexPositions[di]; const n = d.vertexCounts[di];
      let minX = Number.MAX_VALUE, minY = Number.MAX_VALUE, maxX = -Number.MAX_VALUE, maxY = -Number.MAX_VALUE;
      for (let pi = 0; pi < n * 2; pi += 2) { const x = v[pi], y = v[pi + 1]; if (x < minX) minX = x; if (x > maxX) maxX = x; if (y < minY) minY = y; if (y > maxY) maxY = y; }
      if (minX == Number.MAX_VALUE) continue;
      if (minX < tminX) tminX = minX; if (minY < tminY) tminY = minY; if (maxX > tmaxX) tmaxX = maxX; if (maxY > tmaxY) tmaxY = maxY;
      if (tminX == Number.MAX_VALUE) { cc.rect = new Rect(0, 0, 0, 0); cc.isUsing = false; }
      else { cc.isUsing = true; cc.rect = new Rect(tminX, tminY, tmaxX - tminX, tmaxY - tminY); }
    }
    if (cc.isUsing) using++;
  }
  layout(contexts, using, 1);
  const out = [];
  contexts.forEach((cc, k) => {
    const tmp = new Rect(); tmp.setRect(cc.rect); tmp.expand(cc.rect.width * 0.05, cc.rect.height * 0.05);
    const sx = cc.lb.width / tmp.width, sy = cc.lb.height / tmp.height;
    const m = new M44();
    m.loadIdentity(); m.translateRelative(-1.0, -1.0); m.scaleRelative(2.0, 2.0); m.translateRelative(cc.lb.x, cc.lb.y); m.scaleRelative(sx, sy); m.translateRelative(-tmp.x, -tmp.y);
    const forMask = Array.from(m.getArray());
    m.loadIdentity(); m.translateRelative(cc.lb.x, cc.lb.y * 1.0); m.scaleRelative(sx, sy * 1.0); m.translateRelative(-tmp.x, -tmp.y);
    const forDraw = Array.from(m.getArray());
    const baseColor = [cc.lb.x * 2 - 1, cc.lb.y * 2 - 1, cc.lb.getRight() * 2 - 1, cc.lb.getBottom() * 2 - 1];
    out.push({ ctx: k, masks: cc.ids, clipped: cc.clipped, channel: 'RGBA'[cc.ch], layoutBounds: [cc.lb.x, cc.lb.y, cc.lb.width, cc.lb.height], buffer: cc.buf, allClippedDrawRect: [cc.rect.x, cc.rect.y, cc.rect.width, cc.rect.height], tmpBounds: [tmp.x, tmp.y, tmp.width, tmp.height], scaleX: sx, scaleY: sy, matrixForMask_tr0_5_12_13: [forMask[0], forMask[5], forMask[12], forMask[13]], matrixForDraw_tr0_5_12_13: [forDraw[0], forDraw[5], forDraw[12], forDraw[13]], otherElementsIdentity: [1,2,3,4,6,7,8,9,11,14].every(i => forMask[i] === 0 && forDraw[i] === 0) && forMask[10] === 1 && forMask[15] === 1 && forDraw[10] === 1 && forDraw[15] === 1, u_baseColor_setupMask: baseColor });
  });
  console.log('usingClipCount', using);
  for (const o of out) console.log(JSON.stringify(o));
}
let tries = 0;
(function wait() { try { run(); } catch (e) { if (tries++ < 50 && /not|undefined|ready|null/i.test(String(e))) setTimeout(wait, 100); else console.error('fail', e); } })();

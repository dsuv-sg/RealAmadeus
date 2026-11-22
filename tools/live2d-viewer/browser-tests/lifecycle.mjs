// node lifecycle.mjs <viewerUrl> <outDir>: real-time lifecycle checks of the V2 viewer (screenshots per step).
// Exit code 0 iff every step reached its expected state and the console shows no error/warning/exception.
import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
const [url, outDir] = process.argv.slice(2);
mkdirSync(outDir, { recursive: true });
const port = 9700 + Math.floor(Math.random() * 90);
const profile = join(outDir, 'profile'); rmSync(profile, { recursive: true, force: true });
const env = { ...process.env }; delete env.DISPLAY; delete env.XAUTHORITY;
const chrome = spawn('google-chrome', ['--headless=new', '--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader',
  `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--window-size=1280,720', 'about:blank'], { env, stdio: ['ignore', 'ignore', 'pipe'] });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function pageWs() { for (let i = 0; i < 100; i++) { try { const l = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json(); const p = l.find((t) => t.type === 'page'); if (p) return p.webSocketDebuggerUrl; } catch (e) { } await sleep(100); } throw new Error('no chrome'); }
const ws = new WebSocket(await pageWs()); await new Promise((r, j) => { ws.onopen = r; ws.onerror = j; });
let id = 1; const pending = new Map(); const lines = []; const t0 = Date.now();
ws.onmessage = (m) => { const msg = JSON.parse(m.data); if (msg.id && pending.has(msg.id)) { const p = pending.get(msg.id); pending.delete(msg.id); msg.error ? p.rej(new Error(JSON.stringify(msg.error))) : p.res(msg.result); }
  if (msg.method === 'Runtime.consoleAPICalled') lines.push(`[${((Date.now() - t0) / 1000).toFixed(2)}] ${msg.params.type}: ` + msg.params.args.map((a) => a.value ?? a.description ?? '').join(' '));
  if (msg.method === 'Runtime.exceptionThrown') lines.push(`[${((Date.now() - t0) / 1000).toFixed(2)}] EXCEPTION: ` + (msg.params.exceptionDetails.exception?.description || msg.params.exceptionDetails.text)); };
const send = (method, params = {}) => { const i = id++; ws.send(JSON.stringify({ id: i, method, params })); return new Promise((res, rej) => pending.set(i, { res, rej })); };
const ev = async (expr) => { const r = await send('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: true }); return r.exceptionDetails ? { exception: r.exceptionDetails.exception?.description } : r.result.value; };
const state = () => ev("window.l2dViewer ? l2dViewer.state() : ''");
async function waitState(prefix, ms = 30000) { const s = Date.now(); while (Date.now() - s < ms) { const st = await state(); if (typeof st === 'string' && st.startsWith(prefix)) return st; await sleep(100); } return 'TIMEOUT ' + (await state()); }
async function shot(name) { const r = await send('Page.captureScreenshot', { format: 'png' }); writeFileSync(join(outDir, name + '.png'), Buffer.from(r.data, 'base64')); }
const results = {};
await send('Runtime.enable'); await send('Page.enable');
await send('Page.navigate', { url });
results.loaded = await waitState('23|');
await sleep(500); await shot('1-loaded');
// D6: context loss + restore
results.lose = await ev(`(() => { const c = document.querySelector('#host canvas'); window.__gl = c.getContext('webgl') || c.getContext('webgl2'); window.__ext = __gl.getExtension('WEBGL_lose_context'); __ext.loseContext(); return __gl.isContextLost(); })()`);
await sleep(800); results.whileLost = await state(); await shot('2-lost');
results.restore = await ev(`(() => { __ext.restoreContext(); return 'restoring'; })()`);
await sleep(300);
results.restored = await waitState('23|');
await sleep(600); await shot('3-restored');
results.glAfterRestore = await ev('__gl.isContextLost()');
// Resize the host (portrait 600x700)
await ev(`(() => { const h = document.getElementById('host'); h.style.inset = 'auto'; h.style.left = '0'; h.style.top = '0'; h.style.width = '600px'; h.style.height = '700px'; return 1; })()`);
await sleep(800); results.resized = await ev(`(() => { const c = document.querySelector('#host canvas'); return c.width + 'x' + c.height + ' client ' + c.clientWidth + 'x' + c.clientHeight; })()`);
await shot('4-resized');
await ev(`(() => { const h = document.getElementById('host'); h.style.cssText = ''; return 1; })()`);
// destroy
results.destroy = await ev(`(() => { const c = document.querySelector('#host canvas'); const gl = c.getContext('webgl'); l2dViewer.destroy(); const after = document.getElementById('host').childElementCount; gl.getExtension('WEBGL_lose_context').loseContext(); return { hostChildren: after, state: l2dViewer.state() }; })()`);
await sleep(500); await shot('5-destroyed');
// remount
await ev('l2dViewer.remount()');
results.remounted = await waitState('23|');
await sleep(500); await shot('6-remounted');
// destroy while loading (immediately after remount)
results.destroyWhileLoading = await ev(`(() => { l2dViewer.remount(); const s1 = l2dViewer.state(); l2dViewer.destroy(); return s1 + ' -> ' + l2dViewer.state() + ' children=' + document.getElementById('host').childElementCount; })()`);
await sleep(2500);
results.afterLoadingDestroy = await state();
// 20 rapid remounts like the UI (capture ctx, destroy, loseContext)
results.rapid = await ev(`(async () => { for (let i = 0; i < 20; i++) { const c = document.querySelector('#host canvas'); const gl = c && (c.getContext('webgl') || c.getContext('webgl2')); l2dViewer.destroy(); if (gl) { const e = gl.getExtension('WEBGL_lose_context'); e && e.loseContext(); } l2dViewer.remount(); await new Promise(r => setTimeout(r, 30)); } return l2dViewer.state(); })()`);
results.finalLoaded = await waitState('23|');
await sleep(800); await shot('7-after-rapid');
results.consoleErrors = lines.filter((l) => /error|EXCEPTION|warn/i.test(l));
console.log(JSON.stringify(results, null, 1));
writeFileSync(join(outDir, 'console.log'), lines.join('\n'));
const ok = [results.loaded, results.restored, results.remounted, results.finalLoaded].every((s) => typeof s === 'string' && s.startsWith('23|'))
  && results.lose === true && results.glAfterRestore === false && results.resized.startsWith('600x700')
  && results.destroy.hostChildren === 0 && results.destroy.state.endsWith('|true')
  && results.afterLoadingDestroy.endsWith('|true') && results.consoleErrors.length === 0;
console.log(ok ? 'LIFECYCLE OK' : 'LIFECYCLE FAILED');
ws.close(); chrome.kill('SIGKILL'); process.exit(ok ? 0 : 1);

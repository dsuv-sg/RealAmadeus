// node parity.mjs <urlA> <urlB> <outDir> <w> <h> <frames> <setupA> <setupB>   (env DSF = device scale factor)
// Deterministic clocks: frozen while loading; after both pages logged "All textures loaded", run exactly
// <frames> animation frames with Date.now = floor(T0 + k*1000/60), performance.now = P0 + k*1000/60,
// Math.random = mulberry32(20261116); then rAF stops and the page is captured.
import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
const [urlA, urlB, outDir, wArg, hArg, framesArg, setupA, setupB] = process.argv.slice(2);
mkdirSync(outDir, { recursive: true });
const port = 9800 + Math.floor(Math.random() * 90);
const profile = join(outDir, 'profile'); rmSync(profile, { recursive: true, force: true });
const env = { ...process.env }; delete env.DISPLAY; delete env.XAUTHORITY;
const chrome = spawn('google-chrome', ['--headless=new', '--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader',
  `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', `--window-size=${wArg},${hArg}`, ...(process.env.DSF ? [`--force-device-scale-factor=${process.env.DSF}`] : []), 'about:blank'], { env, stdio: ['ignore', 'ignore', 'pipe'] });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const init = `(() => {
  const T0 = 1763000000000, P0 = 1000000; let run = false, k = 0, left = 0;
  function mulberry32(a) { return function () { a |= 0; a = (a + 0x6D2B79F5) | 0; let t = Math.imul(a ^ (a >>> 15), 1 | a); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; }
  Math.random = mulberry32(20261116);
  Date.now = () => run || k > 0 ? Math.floor(T0 + k * 1000 / 60) : T0;
  performance.now = () => P0 + k * 1000 / 60;
  const raf = window.requestAnimationFrame.bind(window);
  window.requestAnimationFrame = (cb) => raf((t) => { if (run) { if (left <= 0) { window.__done = true; return; } left--; k++; } cb(t); });
  window.__startRun = (n) => { run = true; left = n; };
})();`;
async function browserWs() { for (let i = 0; i < 100; i++) { try { return (await (await fetch(`http://127.0.0.1:${port}/json/version`)).json()).webSocketDebuggerUrl; } catch (e) { await sleep(100); } } throw new Error('no chrome'); }
const ws = new WebSocket(await browserWs()); await new Promise((r, j) => { ws.onopen = r; ws.onerror = j; });
let id = 1; const pending = new Map(); const handlers = [];
ws.onmessage = (m) => { const msg = JSON.parse(m.data); if (msg.id && pending.has(msg.id)) { const p = pending.get(msg.id); pending.delete(msg.id); msg.error ? p.rej(new Error(JSON.stringify(msg.error))) : p.res(msg.result); } for (const h of handlers) h(msg); };
const send = (method, params = {}, sessionId) => { const i = id++; ws.send(JSON.stringify({ id: i, method, params, sessionId })); return new Promise((res, rej) => pending.set(i, { res, rej })); };
const evaluate = async (expr, s) => { const r = await send('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: true }, s); return r.exceptionDetails ? { exception: r.exceptionDetails.exception?.description } : r.result.value; };
async function open(url) {
  const { targetId } = await send('Target.createTarget', { url: 'about:blank' });
  const { sessionId } = await send('Target.attachToTarget', { targetId, flatten: true });
  const tab = { targetId, sessionId, lines: [], ready: false };
  handlers.push((msg) => { if (msg.sessionId !== sessionId) return; if (msg.method === 'Runtime.consoleAPICalled') { const t = msg.params.args.map((a) => a.value ?? a.description ?? '').join(' '); tab.lines.push(msg.params.type + ': ' + t); if (t.includes('All textures loaded')) tab.ready = true; } if (msg.method === 'Runtime.exceptionThrown') tab.lines.push('exception: ' + (msg.params.exceptionDetails.exception?.description || msg.params.exceptionDetails.text)); });
  await send('Runtime.enable', {}, sessionId); await send('Page.enable', {}, sessionId);
  await send('Page.addScriptToEvaluateOnNewDocument', { source: init }, sessionId);
  await send('Page.navigate', { url }, sessionId);
  return tab;
}
async function runTab(tab, setup, name) {
  // Only one tab is visible at a time in headless; activate it so rAF runs.
  await send('Target.activateTarget', { targetId: tab.targetId });
  const t0 = Date.now(); while (!tab.ready && Date.now() - t0 < 60000) await sleep(100);
  await sleep(300);
  const r = await evaluate(`(() => { ${setup}; window.__startRun(${framesArg}); return 'started'; })()`, tab.sessionId);
  const t1 = Date.now(); while (Date.now() - t1 < 60000) { if (await evaluate('!!window.__done', tab.sessionId)) break; await sleep(100); }
  await sleep(300);
  const shot = await send('Page.captureScreenshot', { format: 'png' }, tab.sessionId);
  writeFileSync(join(outDir, name + '.png'), Buffer.from(shot.data, 'base64'));
  writeFileSync(join(outDir, name + '.log'), tab.lines.join('\n'));
  return { ready: tab.ready, started: r, done: await evaluate('!!window.__done', tab.sessionId) };
}
const a = await open(urlA); const ra = await runTab(a, setupA, 'A'); await send('Target.closeTarget', { targetId: a.targetId });
const b = await open(urlB); const rb = await runTab(b, setupB, 'B'); await send('Target.closeTarget', { targetId: b.targetId });
console.log(JSON.stringify({ A: ra, B: rb }));
ws.close(); chrome.kill('SIGKILL'); process.exit(0);

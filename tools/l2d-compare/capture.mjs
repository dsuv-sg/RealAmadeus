#!/usr/bin/env node
// tools/l2d-compare/capture.mjs: headless-Chrome captures of the V1 and V2 pages (run.sh starts it).
//
//   node capture.mjs --base http://127.0.0.1:PORT --out DIR
//        [--viewports 1280x720,390x844] [--emotions NORMAL,SMILE,SLEEPING] [--frames 1,60,300]
//        [--versions v1,v2] [--dpr 1] [--seed 20261118] [--repeat 0] [--controls 1] [--timeout 180]
//        [--query 'lip=0.8&char=あ&do=120:setEmotion:"NORMAL"'] [--tag NAME]
//        [--v1-path /v1.html] [--v2-path /v2/] [--chrome google-chrome]
//
// --query is appended to every page URL (both pages understand emotion/lip/char/eyeX/eyeY and the shim's
// frames/seed/timeout/readback/stepms/do); --tag NAME is appended to the scenario directory names (-NAME).
//
// One Chrome per viewport (SwiftShader WebGL); a single tab is navigated to every page in turn (so it
// is always the visible, unthrottled tab). For each page it waits until the shim sets document.title
// to DONE (frame N drawn) or TIMEOUT, waits for the compositor to present it, and saves:
//   DIR/<scenario>/<ver>.png         Page.captureScreenshot (W x H x DPR)
//   DIR/<scenario>/<ver>.canvas.png  RGBA readback of the WebGL canvas taken right after frame N
//   DIR/<scenario>/<ver>.json        shim report (draw calls, Math.random log, contexts, canvases,
//                                    console, errors) + CDP console/exception/log entries + timings
//   DIR/<scenario>/<ver>.log         the CDP console lines as text
// <scenario> = <W>x<H>[@<DPR>]-<EMOTION>-f<NNN>[-<TAG>]. --repeat K captures the first scenario K more times
// per version (<ver>.rep<k>.*): V1-vs-V1 and V2-vs-V2 must be identical (harness noise floor).
// --controls 1 (default) also captures V1 one frame later (frames N+1) for every frame count of the first
// viewport and emotion (<scenario>/ctrl-f<NNN>.v1.*): diff.py compares it with v1.png, which proves that a
// 1/60 s timing error would be visible (sensitivity control; it must NOT be identical).
// Exit code: 0 if every page finished with DONE, 1 otherwise (captures are still written).
import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync, rmSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';

const opts = {
  base: '',
  out: '',
  viewports: '1280x720,390x844',
  emotions: 'NORMAL,SMILE,SLEEPING',
  frames: '1,60,300',
  versions: 'v1,v2',
  dpr: '1',
  seed: '20261118',
  repeat: '0',
  controls: '1',
  timeout: '180',
  'v1-path': '/v1.html',
  'v2-path': '/v2/',
  query: '',
  tag: '',
  chrome: process.env.CHROME || 'google-chrome',
};
const argv = process.argv.slice(2);
for (let i = 0; i < argv.length; i++) {
  const m = /^--([a-z0-9-]+)(?:=(.*))?$/.exec(argv[i]);
  if (!m || !(m[1] in opts)) throw new Error(`unknown argument ${argv[i]}`);
  opts[m[1]] = m[2] !== undefined ? m[2] : argv[++i];
}
if (!opts.base || !opts.out) throw new Error('--base and --out are required');
const list = (s) => s.split(',').map((x) => x.trim()).filter(Boolean);
const viewports = list(opts.viewports).map((v) => {
  const m = /^(\d+)x(\d+)$/.exec(v);
  if (!m) throw new Error(`bad viewport ${v}`);
  return { w: +m[1], h: +m[2] };
});
const emotions = list(opts.emotions);
const frameCounts = list(opts.frames).map((n) => Math.max(1, parseInt(n, 10)));
const versions = list(opts.versions);
const dpr = parseFloat(opts.dpr);
const repeat = parseInt(opts.repeat, 10) || 0;
const controls = opts.controls !== '0' && versions.includes('v1');
const timeoutMs = parseFloat(opts.timeout) * 1000;
const pagePath = { v1: opts['v1-path'], v2: opts['v2-path'] };
mkdirSync(opts.out, { recursive: true });

// ---- minimal CDP client (flattened sessions over the browser WebSocket) ------------------------------
class Cdp {
  constructor(ws) {
    this.ws = ws;
    this.id = 1;
    this.pending = new Map();
    this.listeners = new Set();
    ws.onmessage = (m) => {
      const msg = JSON.parse(m.data);
      if (msg.id && this.pending.has(msg.id)) {
        const p = this.pending.get(msg.id);
        this.pending.delete(msg.id);
        if (msg.error) p.reject(new Error(`${p.method}: ${JSON.stringify(msg.error)}`));
        else p.resolve(msg.result);
      } else if (msg.method) {
        for (const l of this.listeners) l(msg);
      }
    };
    ws.onclose = () => {
      for (const p of this.pending.values()) p.reject(new Error('CDP connection closed'));
      this.pending.clear();
    };
  }
  static async connect(url) {
    const ws = new WebSocket(url);
    await new Promise((resolve, reject) => {
      ws.onopen = resolve;
      ws.onerror = () => reject(new Error(`cannot connect to ${url}`));
    });
    return new Cdp(ws);
  }
  send(method, params = {}, sessionId = undefined) {
    const id = this.id++;
    const msg = { id, method, params };
    if (sessionId) msg.sessionId = sessionId;
    this.ws.send(JSON.stringify(msg));
    return new Promise((resolve, reject) => this.pending.set(id, { resolve, reject, method }));
  }
  async evaluate(expression, sessionId) {
    const r = await this.send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, sessionId);
    if (r.exceptionDetails) return { __exception: r.exceptionDetails.exception?.description || r.exceptionDetails.text };
    return r.result.value;
  }
  close() {
    try {
      this.ws.close();
    } catch {
      /* already closed */
    }
  }
}

async function launchChrome(w, h, profile) {
  rmSync(profile, { recursive: true, force: true });
  mkdirSync(profile, { recursive: true });
  const env = { ...process.env };
  delete env.DISPLAY;
  delete env.XAUTHORITY;
  const args = [
    '--headless=new', '--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader',
    '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check',
    `--window-size=${w},${h}`, `--force-device-scale-factor=${dpr}`, '--hide-scrollbars', '--force-color-profile=srgb',
    '--disable-background-timer-throttling', '--disable-renderer-backgrounding',
    '--disable-backgrounding-occluded-windows', '--disable-extensions', '--mute-audio', 'about:blank',
  ];
  const proc = spawn(opts.chrome, args, { env, stdio: ['ignore', 'ignore', 'pipe'] });
  let stderr = '';
  proc.stderr.on('data', (d) => {
    stderr += d;
    if (stderr.length > 1_000_000) stderr = stderr.slice(-500_000);
  });
  for (let i = 0; i < 300; i++) {
    try {
      const [port, path] = readFileSync(join(profile, 'DevToolsActivePort'), 'utf8').split('\n').map((s) => s.trim());
      if (port && path) return { proc, url: `ws://127.0.0.1:${port}${path}`, stderr: () => stderr };
    } catch {
      /* not written yet */
    }
    if (proc.exitCode !== null) break;
    await sleep(100);
  }
  proc.kill('SIGKILL');
  throw new Error(`Chrome did not start:\n${stderr.slice(-3000)}`);
}

function fmtRemote(a) {
  if (a.type === 'string') return a.value;
  if (a.value !== undefined) return JSON.stringify(a.value);
  return a.description ?? a.unserializableValue ?? a.type;
}

async function openTab(cdp, w, h) {
  const { targetInfos } = await cdp.send('Target.getTargets');
  let target = targetInfos.find((t) => t.type === 'page');
  if (!target) target = { targetId: (await cdp.send('Target.createTarget', { url: 'about:blank' })).targetId };
  const { sessionId } = await cdp.send('Target.attachToTarget', { targetId: target.targetId, flatten: true });
  const tab = { sessionId, targetId: target.targetId, sink: [] };
  cdp.listeners.add((msg) => {
    if (msg.sessionId !== sessionId) return;
    const p = msg.params;
    if (msg.method === 'Runtime.consoleAPICalled') tab.sink.push(`${p.type}: ${p.args.map(fmtRemote).join(' ')}`);
    else if (msg.method === 'Runtime.exceptionThrown') {
      const d = p.exceptionDetails;
      tab.sink.push(`EXCEPTION: ${d.exception?.description || d.text} @${d.url || ''}:${d.lineNumber}`);
    } else if (msg.method === 'Log.entryAdded') {
      const e = p.entry;
      tab.sink.push(`log(${e.source}/${e.level}): ${e.text}${e.url ? ` @${e.url}` : ''}`);
    }
  });
  await cdp.send('Page.enable', {}, sessionId);
  await cdp.send('Runtime.enable', {}, sessionId);
  await cdp.send('Log.enable', {}, sessionId);
  await cdp.send('Emulation.setDeviceMetricsOverride', { width: w, height: h, deviceScaleFactor: dpr, mobile: false }, sessionId);
  await cdp.send('Target.activateTarget', { targetId: target.targetId });
  return tab;
}

let runCounter = 0;
async function capturePage(cdp, tab, ver, scenario, prefix) {
  const token = `${process.pid}-${++runCounter}`;
  const qs = new URLSearchParams({ emotion: scenario.emotion, frames: String(scenario.frames), seed: opts.seed, resources: '/resources/' });
  for (const [k, v] of new URLSearchParams(opts.query)) qs.set(k, v);
  qs.set('run', token);
  const url = `${opts.base}${pagePath[ver]}?${qs}`;
  tab.sink = [];
  const t0 = Date.now();
  await cdp.send('Page.navigate', { url }, tab.sessionId);
  let state = null;
  while (Date.now() - t0 < timeoutMs) {
    state = await cdp.evaluate(
      `(() => { const s = window.__l2dCompare; return { run: new URLSearchParams(location.search).get('run'), title: document.title, status: s ? s.status : null, frame: s ? s.frame : null }; })()`,
      tab.sessionId,
    ).catch((e) => ({ error: String(e) }));
    if (state && state.run === token && (state.title === 'DONE' || state.title === 'TIMEOUT')) break;
    await sleep(100);
  }
  const wallMs = Date.now() - t0;
  // Let the compositor present the frame drawn in the last virtual rAF callback (no further frames run):
  // two real animation frames (shim), with a fixed wait as the fallback.
  const presented = await Promise.race([
    cdp.evaluate('window.__l2dCompare && window.__l2dCompare.presented ? window.__l2dCompare.presented() : false', tab.sessionId).catch(() => false),
    sleep(5000).then(() => false),
  ]);
  await sleep(presented === true ? 100 : 400);
  const shot = await cdp.send('Page.captureScreenshot', { format: 'png' }, tab.sessionId);
  writeFileSync(`${prefix}.png`, Buffer.from(shot.data, 'base64'));
  const report = await cdp.evaluate('window.__l2dCompare ? window.__l2dCompare.report() : null', tab.sessionId).catch((e) => ({ __error: String(e) }));
  const readback = await cdp.evaluate('window.__l2dCompare ? window.__l2dCompare.readback : null', tab.sessionId).catch(() => null);
  const prefixPng = 'data:image/png;base64,';
  if (typeof readback === 'string' && readback.startsWith(prefixPng)) {
    writeFileSync(`${prefix}.canvas.png`, Buffer.from(readback.slice(prefixPng.length), 'base64'));
  } else {
    rmSync(`${prefix}.canvas.png`, { force: true });
  }
  const ok = !!state && state.run === token && state.title === 'DONE';
  const meta = { version: ver, url, scenario, dpr, ok, wallMs, final: state, report, cdp: tab.sink };
  writeFileSync(`${prefix}.json`, JSON.stringify(meta, null, 1));
  writeFileSync(`${prefix}.log`, tab.sink.join('\n') + '\n');
  await cdp.send('Page.navigate', { url: 'about:blank' }, tab.sessionId);
  await sleep(150);
  const r = report && !report.__error ? report : {};
  console.log(
    `${prefix.slice(opts.out.length + 1)}: ${ok ? 'DONE' : `FAILED (${JSON.stringify(state)})`} ` +
      `frame=${r.frame ?? '?'} status=${r.status ?? '?'} armedAtTick=${r.armedAtTick ?? '?'} ` +
      `draws(last)=${(r.lastFrameDrawLog || []).length} random=${r.randomCalls ?? '?'} ` +
      `errors=${(r.errors || []).length} wall=${wallMs}ms`,
  );
  return ok;
}

const scenarioName = (vp, emotion, n) =>
  `${vp.w}x${vp.h}${dpr !== 1 ? `@${dpr}` : ''}-${emotion}-f${String(n).padStart(3, '0')}${opts.tag ? `-${opts.tag}` : ''}`;

let allOk = true;
const index = { opts, chrome: null, scenarios: [] };
for (const [vi, vp] of viewports.entries()) {
  const profile = join(opts.out, `.profile-${vp.w}x${vp.h}`);
  const chrome = await launchChrome(vp.w, vp.h, profile);
  const cdp = await Cdp.connect(chrome.url);
  try {
    if (!index.chrome) index.chrome = await cdp.send('Browser.getVersion');
    const tab = await openTab(cdp, vp.w, vp.h);
    for (const emotion of emotions) {
      for (const frames of frameCounts) {
        const name = scenarioName(vp, emotion, frames);
        const dir = join(opts.out, name);
        mkdirSync(dir, { recursive: true });
        const scenario = { viewport: vp, dpr, emotion, frames, query: opts.query, tag: opts.tag };
        for (const ver of versions) allOk = (await capturePage(cdp, tab, ver, scenario, join(dir, ver))) && allOk;
        index.scenarios.push({ name, ...scenario });
      }
    }
    if (vi === 0 && repeat > 0 && emotions.length && frameCounts.length) {
      const name = scenarioName(vp, emotions[0], frameCounts[0]);
      const scenario = { viewport: vp, dpr, emotion: emotions[0], frames: frameCounts[0] };
      for (let k = 1; k <= repeat; k++) {
        for (const ver of versions) allOk = (await capturePage(cdp, tab, ver, scenario, join(opts.out, name, `${ver}.rep${k}`))) && allOk;
      }
    }
    if (vi === 0 && controls && emotions.length) {
      for (const frames of frameCounts) {
        const name = scenarioName(vp, emotions[0], frames);
        const scenario = { viewport: vp, dpr, emotion: emotions[0], frames: frames + 1 };
        const prefix = join(opts.out, name, `ctrl-f${String(frames + 1).padStart(3, '0')}.v1`);
        allOk = (await capturePage(cdp, tab, 'v1', scenario, prefix)) && allOk;
      }
    }
  } finally {
    cdp.close();
    const exited = chrome.proc.exitCode !== null ? Promise.resolve() : new Promise((r) => chrome.proc.once('exit', r));
    chrome.proc.kill('SIGKILL');
    await Promise.race([exited, sleep(5000)]);
    writeFileSync(join(opts.out, `.chrome-${vp.w}x${vp.h}.stderr.log`), chrome.stderr());
    rmSync(profile, { recursive: true, force: true }); // ~60-220 MB of Chrome cache per viewport
  }
}
writeFileSync(join(opts.out, 'index.json'), JSON.stringify(index, null, 1));
process.exit(allOk ? 0 : 1);

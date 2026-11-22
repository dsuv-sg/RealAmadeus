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

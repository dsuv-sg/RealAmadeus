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

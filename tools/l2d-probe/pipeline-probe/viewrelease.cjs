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

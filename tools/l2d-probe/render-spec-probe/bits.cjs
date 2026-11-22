'use strict';
const fs = require('fs'); const vm = require('vm');
const ctx = { console, setTimeout, clearTimeout, WebAssembly, TextDecoder, Uint8Array, Uint16Array, Float32Array, Int32Array, ArrayBuffer, Math, performance, Date };
ctx.window = ctx; ctx.self = ctx; ctx.globalThis = ctx; ctx.document = { currentScript: { src: '' } }; ctx.atob = atob; ctx.btoa = btoa;
vm.createContext(ctx);
vm.runInContext(fs.readFileSync(process.argv[2], 'utf8') + '\n;globalThis.__core = Live2DCubismCore;', ctx);
const C = ctx.__core;
const U = C.Utils; const out = {};
for (const k of Object.keys(U)) { out[k] = [0,1,2,3,4,5,6,7].filter(b => U[k](1 << b)).map(b => 1 << b); }
console.log(JSON.stringify(out));
const consts = {}; for (const k of Object.keys(C)) { if (/^(ColorBlendType|AlphaBlendType)/.test(k)) consts[k] = C[k]; } console.log(JSON.stringify(consts));
console.log('Core top-level keys:', Object.keys(C).join(','));

"use strict";
const fs = require("fs");
const src = fs.readFileSync("/home/elvelt_admin/Elvelt/dsuv/public/resources/live2d/reama-cubism-runtime.js", "utf8").split("\n");
const seg = src.slice(17873, 18010).join("\n"); // frameworkRefCount..getEmotionIdleMotion (0-based slice of L17874..L18010)
const f = new Function(seg + "\nreturn { lerp, qBound, noise01, noiseSigned, drift, MOTION_BURSTS, getEmotionIdleMotion };")();
const fmt = (x) => Number(x.toPrecision(17));
const out = [];
out.push("noise01(x, seed):");
for (const [x, s] of [[0,0],[0,5],[1,0],[1,5],[2.5,5],[10,7],[123.456,99]]) out.push(`  noise01(${x}, ${s}) = ${fmt(f.noise01(x, s))}`);
out.push("noiseSigned(x, seed):");
for (const [x, s] of [[0,0],[3,7],[60,999]]) out.push(`  noiseSigned(${x}, ${s}) = ${fmt(f.noiseSigned(x, s))}`);
out.push("drift(phase, speed, seed, amp):");
for (const a of [[0,0.6,0,1.8],[1,0.6,0,1.8],[5,1.2,5,3],[10,0.2,55,0.5],[2,3,36,3]]) out.push(`  drift(${a.join(", ")}) = ${fmt(f.drift(...a))}`);
out.push("getEmotionIdleMotion(tag, phase):");
for (const tag of ["NORMAL","SMILE","ANGRY","SAD","SURPRISED","BLUSH","WINK","DISGUST","SMUG","THINKING","PANIC","SLEEPING","sneeze","UNKNOWN"]) {
  for (const ph of [0, 1.5, 7.25]) { const m = f.getEmotionIdleMotion(tag, ph); out.push(`  ${tag} @${ph}: ` + ["bodyX","bodyY","bodyZ","headX","headY","headZ"].map(k => k + "=" + fmt(m[k])).join(" ")); }
}
const all = src.join("\n");
const m = all.match(/function getVowelMouthOpening\(chr\) \{[\s\S]*?\n        return 0\.5;\n      \}/);
const gv = new Function(m[0] + "\nreturn getVowelMouthOpening;")();
out.push("getVowelMouthOpening(chr):");
const chars = ["a","A","o","O","e","E","u","U","i","I","í","Í","é","x","1","あ","か","ア","カ","ぁ","ゃ","ャ","お","オ","ょ","ョ","を","ヲ","え","エ","う","ウ","ゅ","ュ","い","イ","ん","ン","っ","ッ","ー","ヴ","ゎ","ゐ","ゑ","。","、","？","！",".",",","?","!"," ","　","\n","\r","\t","-","_","紅","😀","ab","おはよう"," x",""];
for (const c of chars) out.push(`  ${JSON.stringify(c)} (${Array.from(c).map(ch => "U+" + ch.codePointAt(0).toString(16).toUpperCase().padStart(4, "0")).join(" ") || "empty"}) -> ${gv(c)}`);
// spring curve samples
out.push("burst spring factor sin(t*PI*2.5)*(1-t)^2:");
for (const t of [0,0.1,0.2,0.3,0.4,0.5,0.6,0.8,1]) out.push(`  t=${t}: ${fmt(Math.sin(t*Math.PI*2.5)*(1-t)*(1-t))}`);
console.log(out.join("\n"));

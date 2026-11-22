const Live2DCubismCore = { ColorBlendType_Normal:0, ColorBlendType_Add:3, ColorBlendType_AddGlow:4, ColorBlendType_Darken:5, ColorBlendType_Multiply:6, ColorBlendType_ColorBurn:7, ColorBlendType_LinearBurn:8, ColorBlendType_Lighten:9, ColorBlendType_Screen:10, ColorBlendType_ColorDodge:11, ColorBlendType_Overlay:12, ColorBlendType_SoftLight:13, ColorBlendType_HardLight:14, ColorBlendType_LinearLight:15, ColorBlendType_Hue:16, ColorBlendType_Color:17, ColorBlendType_AddCompatible:1, ColorBlendType_MultiplyCompatible:2 };
var CubismColorBlend = ((C) => {
    C[C["ColorBlend_None"] = -1] = "ColorBlend_None";
    C[C["ColorBlend_Normal"] = Live2DCubismCore.ColorBlendType_Normal] = "ColorBlend_Normal";
    C[C["ColorBlend_AddGlow"] = Live2DCubismCore.ColorBlendType_AddGlow] = "ColorBlend_AddGlow";
    C[C["ColorBlend_Add"] = Live2DCubismCore.ColorBlendType_Add] = "ColorBlend_Add";
    C[C["ColorBlend_Darken"] = Live2DCubismCore.ColorBlendType_Darken] = "ColorBlend_Darken";
    C[C["ColorBlend_Multiply"] = Live2DCubismCore.ColorBlendType_Multiply] = "ColorBlend_Multiply";
    C[C["ColorBlend_ColorBurn"] = Live2DCubismCore.ColorBlendType_ColorBurn] = "ColorBlend_ColorBurn";
    C[C["ColorBlend_LinearBurn"] = Live2DCubismCore.ColorBlendType_LinearBurn] = "ColorBlend_LinearBurn";
    C[C["ColorBlend_Lighten"] = Live2DCubismCore.ColorBlendType_Lighten] = "ColorBlend_Lighten";
    C[C["ColorBlend_Screen"] = Live2DCubismCore.ColorBlendType_Screen] = "ColorBlend_Screen";
    C[C["ColorBlend_ColorDodge"] = Live2DCubismCore.ColorBlendType_ColorDodge] = "ColorBlend_ColorDodge";
    C[C["ColorBlend_Overlay"] = Live2DCubismCore.ColorBlendType_Overlay] = "ColorBlend_Overlay";
    C[C["ColorBlend_SoftLight"] = Live2DCubismCore.ColorBlendType_SoftLight] = "ColorBlend_SoftLight";
    C[C["ColorBlend_HardLight"] = Live2DCubismCore.ColorBlendType_HardLight] = "ColorBlend_HardLight";
    C[C["ColorBlend_LinearLight"] = Live2DCubismCore.ColorBlendType_LinearLight] = "ColorBlend_LinearLight";
    C[C["ColorBlend_Hue"] = Live2DCubismCore.ColorBlendType_Hue] = "ColorBlend_Hue";
    C[C["ColorBlend_Color"] = Live2DCubismCore.ColorBlendType_Color] = "ColorBlend_Color";
    C[C["ColorBlend_AddCompatible"] = Live2DCubismCore.ColorBlendType_AddCompatible] = "ColorBlend_AddCompatible";
    C[C["ColorBlend_MultiplyCompatible"] = Live2DCubismCore.ColorBlendType_MultiplyCompatible] = "ColorBlend_MultiplyCompatible";
    return C; })({});
var CubismAlphaBlend = ((A) => {
    A[A["AlphaBlend_None"] = -1] = "AlphaBlend_None"; A[A["AlphaBlend_Over"] = 0] = "AlphaBlend_Over"; A[A["AlphaBlend_Atop"] = 1] = "AlphaBlend_Atop"; A[A["AlphaBlend_Out"] = 2] = "AlphaBlend_Out"; A[A["AlphaBlend_ConjointOver"] = 3] = "AlphaBlend_ConjointOver"; A[A["AlphaBlend_DisjointOver"] = 4] = "AlphaBlend_DisjointOver"; return A; })({});
const colorBlendMap = new Map(), colorBlendValues = [];
const ck = Object.keys(CubismColorBlend); const cv = ck.map(k => CubismColorBlend[k]);
for (let i=0;i<ck.length;i++){ if (ck[i].includes('ColorBlend_')) { const n=ck[i].slice('ColorBlend_'.length); const v=parseInt(cv[i].toString()); colorBlendMap.set(v,n); colorBlendValues.push(v);} }
const alphaBlendMap = new Map(), alphaBlendValues = [];
const ak = Object.keys(CubismAlphaBlend); const av = ak.map(k => CubismAlphaBlend[k]);
for (let i=0;i<ak.length;i++){ if (ak[i].includes('AlphaBlend_')) { const n=ak[i].slice('AlphaBlend_'.length); const v=parseInt(av[i].toString()); alphaBlendMap.set(v,n); alphaBlendValues.push(v);} }
console.log('Object.keys(CubismColorBlend)=', JSON.stringify(ck));
console.log('colorBlendValues', JSON.stringify(colorBlendValues));
console.log('alphaBlendValues', JSON.stringify(alphaBlendValues));
const shaderCount = 10 + 1 + (colorBlendValues.length - 3) * (alphaBlendValues.length - 1) * 3;
console.log('shaderCount', shaderCount);
let idx = 11; const rows = [];
for (const c of colorBlendValues) { if (c===-1||c===1||c===2) continue; const cname = colorBlendMap.get(c).toUpperCase();
  for (const a of alphaBlendValues) { if (a===-1 || (c===0 && a===0)) continue; const aname = alphaBlendMap.get(a).toUpperCase();
    rows.push(`| ${colorBlendMap.get(c)}${alphaBlendMap.get(a)} | ${c} | ${a} | COLOR_BLEND_${cname} | ALPHA_BLEND_${aname} | ${idx} | ${idx+1} | ${idx+2} |`); idx += 3; } }
console.log('last used index', idx-1, 'combos', rows.length);
console.log(rows.join('\n'));

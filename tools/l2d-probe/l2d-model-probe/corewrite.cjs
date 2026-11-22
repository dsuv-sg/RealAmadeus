const fs=require('fs'),vm=require('vm');
const sb={console,setTimeout,Uint8Array,Int32Array,Uint32Array,Float32Array,Uint16Array,ArrayBuffer,WebAssembly,Math,Date,Promise,TextDecoder,atob:(b)=>Buffer.from(b,'base64').toString('binary')};
sb.document={currentScript:{src:'http://localhost/x.js'}};sb.window=sb;sb.self=sb;sb.globalThis=sb;sb.location={href:'http://localhost/'};
vm.createContext(sb);
vm.runInContext(fs.readFileSync('/home/elvelt_admin/Elvelt/dsuv/public/resources/live2d/live2dcubismcore.min.js','utf8')+';globalThis.C=Live2DCubismCore;',sb);
const b=fs.readFileSync(process.argv[2]);const ab=b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength);
(function w(){try{sb.C.Version.csmGetVersion()}catch(e){return setTimeout(w,20)}
 const moc=sb.C.Moc.fromArrayBuffer(ab); const m=sb.C.Model.fromMoc(moc);
 const v=m.parameters.values, po=m.parts.opacities;
 v[0]=100; v[3]=-5; v[24]=NaN; po[0]=3; po[1]=-1;
 m.update();
 console.log('after update: AngleX=',v[0],'EyeLOpen=',v[3],'HairFront=',v[24],'partOp0=',po[0],'partOp1=',po[1]);
 console.log('drawable0 opacity', m.drawables.opacities[0], 'dyn', m.drawables.dynamicFlags[0], 'vtx0', m.drawables.vertexPositions[0][0]);
 v[0]=0; v[3]=1; v[24]=0; po[0]=1; po[1]=1; m.update();
 console.log('restored update: drawable0 opacity', m.drawables.opacities[0], 'vtx0', m.drawables.vertexPositions[0][0]);
 console.log('typeof ids', typeof m.parameters.ids[0], 'canvasinfo', JSON.stringify(m.canvasinfo));
 console.log('renderOrders', Array.from(m.getRenderOrders()).slice(0,10).join(','));
 console.log('blendModes len', m.drawables.blendModes.length, 'multiplyColors len', m.drawables.multiplyColors.length, 'offscreens', m.offscreens.count, 'parts.offscreenIndices', Array.from(m.parts.offscreenIndices).join(','));
})();

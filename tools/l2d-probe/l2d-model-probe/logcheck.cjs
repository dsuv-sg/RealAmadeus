const fs=require('fs'),vm=require('vm');
const sb={console:{log:(...a)=>process.stdout.write('[console.log] '+a.join(' ')+'\n'),warn:(...a)=>process.stdout.write('[console.warn] '+a.join(' ')+'\n'),error:(...a)=>process.stdout.write('[console.error] '+a.join(' ')+'\n')},setTimeout,Uint8Array,Int32Array,Uint32Array,Float32Array,Uint16Array,ArrayBuffer,WebAssembly,Math,Date,Promise,TextDecoder,atob:(b)=>Buffer.from(b,'base64').toString('binary')};
sb.document={currentScript:{src:'http://localhost/x.js'}};sb.window=sb;sb.self=sb;sb.globalThis=sb;sb.location={href:'http://localhost/'};
vm.createContext(sb);
vm.runInContext(fs.readFileSync('/home/elvelt_admin/Elvelt/dsuv/public/resources/live2d/live2dcubismcore.min.js','utf8')+';globalThis.C=Live2DCubismCore;',sb);
const b=fs.readFileSync(process.argv[2]);const ab=b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength);
(function w(n){try{sb.C.Version.csmGetVersion()}catch(e){return setTimeout(()=>w(n+1),20)}
 console.log('ready after polls:',n,'version',sb.C.Version.csmGetVersion().toString(16));
 console.log('getLogFunction before set:', typeof sb.C.Logging.csmGetLogFunction());
 let m1=sb.C.Moc.fromArrayBuffer(ab); console.log('moc1 ptr', m1._ptr);
 sb.C.Logging.csmSetLogFunction((msg)=>process.stdout.write('[CSM LOG CALLBACK] '+JSON.stringify(msg)+'\n'));
 let m2=sb.C.Moc.fromArrayBuffer(ab); console.log('moc2 ptr', m2._ptr);
 sb.C.Logging.csmSetLogFunction(undefined);
 try { let m3=sb.C.Moc.fromArrayBuffer(ab); console.log('moc3 ptr', m3 && m3._ptr); } catch(e) { console.log('moc3 THREW', e.message); }
 // corrupt moc
 const bad=new Uint8Array(ab.slice(0)); bad[4]=99;
 try { const m4=sb.C.Moc.fromArrayBuffer(bad.buffer); console.log('bad moc ->', m4); } catch(e){ console.log('bad moc THREW', e.message); }
 console.log('consistency of good moc', sb.C.Moc.prototype.hasMocConsistency(ab));
 console.log('mocVersion bad', sb.C.Version.csmGetMocVersion(bad.buffer));
 const mdl=sb.C.Model.fromMoc(m2); const v1=mdl.parameters.values; const buf1=v1.buffer;
 for(let i=0;i<40;i++){ sb.C.Model.fromMoc(m2); }
 console.log('after 40 more models: values view length', v1.length, 'same buffer', v1.buffer===buf1, 'detached?', v1.byteLength===0);
})(0);

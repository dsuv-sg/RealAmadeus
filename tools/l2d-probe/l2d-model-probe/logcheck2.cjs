const fs=require('fs'),vm=require('vm');
const out=(s)=>process.stdout.write(s+'\n');
const sb={console:{log:(...a)=>out('[console.log] '+a.join(' ').slice(0,200)),warn:(...a)=>out('[console.warn] '+a.join(' ').slice(0,200)),error:(...a)=>out('[console.error] '+a.join(' ').slice(0,200))},setTimeout,Uint8Array,Int32Array,Uint32Array,Float32Array,Uint16Array,ArrayBuffer,WebAssembly,Math,Date,Promise,TextDecoder,atob:(b)=>Buffer.from(b,'base64').toString('binary')};
sb.document={currentScript:{src:'http://localhost/x.js'}};sb.window=sb;sb.self=sb;sb.globalThis=sb;sb.location={href:'http://localhost/'};
vm.createContext(sb);
vm.runInContext(fs.readFileSync('/home/elvelt_admin/Elvelt/dsuv/public/resources/live2d/live2dcubismcore.min.js','utf8')+';globalThis.C=Live2DCubismCore;',sb);
const b=fs.readFileSync(process.argv[2]);const ab=b.buffer.slice(b.byteOffset,b.byteOffset+b.byteLength);
const mode=process.argv[3];
(function w(n){try{sb.C.Version.csmGetVersion()}catch(e){return setTimeout(()=>w(n+1),20)}
 if(mode==='undef'){ sb.C.Logging.csmSetLogFunction(undefined); }
 if(mode==='fn'){ sb.C.Logging.csmSetLogFunction((m)=>out('[CSM CALLBACK] '+JSON.stringify(m))); }
 const bad=new Uint8Array(ab.slice(0)); bad[4]=99;
 try { const m4=sb.C.Moc.fromArrayBuffer(bad.buffer); out('bad version moc -> '+m4); } catch(e){ out('bad version moc THREW '+(e&&e.message||e)); }
 const trunc=ab.slice(0,1000);
 try { const m5=sb.C.Moc.fromArrayBuffer(trunc); out('truncated moc -> '+m5); } catch(e){ out('truncated moc THREW '+(e&&e.message||e)); }
 try { out('consistency(trunc)='+sb.C.Moc.prototype.hasMocConsistency(trunc)); } catch(e){ out('consistency(trunc) THREW '+(e&&e.message||e)); }
 out('consistency(good)='+sb.C.Moc.prototype.hasMocConsistency(ab)+' mocVersion(bad)='+sb.C.Version.csmGetMocVersion(bad.buffer)+' mocVersion(good)='+sb.C.Version.csmGetMocVersion(ab)+' latest='+sb.C.Version.csmGetLatestMocVersion());
 const moc=sb.C.Moc.fromArrayBuffer(ab); const mdl=sb.C.Model.fromMoc(moc); const v1=mdl.parameters.values; const before=v1.length;
 let k=0; try { for(;k<200;k++){ sb.C.Model.fromMoc(moc); } } catch(e){ out('model alloc threw at '+k+': '+(e&&e.message||e)); }
 out('values view length before='+before+' after '+k+' extra models='+v1.length+' (0 => detached by heap growth)');
 const v2 = mdl.parameters.values; out('re-read mdl.parameters.values length='+v2.length+' (wrapper does not refresh views)');
})(0);

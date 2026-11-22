class M44 { constructor(){ this._tr=new Float32Array(16); this.loadIdentity(); }
  static multiply(a,b,dst){ const c=new Float32Array(16); for(let i=0;i<4;++i)for(let j=0;j<4;++j)for(let k=0;k<4;++k) c[j+i*4]+=a[k+i*4]*b[j+k*4]; for(let i=0;i<16;++i) dst[i]=c[i]; }
  loadIdentity(){ this.setMatrix(new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1])); }
  setMatrix(t){ for(let i=0;i<16;++i) this._tr[i]=t[i]; } getArray(){ return this._tr; }
  scaleRelative(x,y){ M44.multiply(new Float32Array([x,0,0,0,0,y,0,0,0,0,1,0,0,0,0,1]), this._tr, this._tr); }
  translateY(y){ this._tr[13]=y; } scale(x,y){ this._tr[0]=x; this._tr[5]=y; }
  multiplyByMatrix(m){ M44.multiply(m.getArray(), this._tr, this._tr); } }
const canvasW = 7000/7000, canvasH = 10000/7000;
const modelM = new M44(); modelM.scale(2.0/canvasH, 2.0/canvasH);
const view = new M44(); view.scale(1,1);
for (const [w,h] of [[1280,720],[1920,1080],[800,800],[390,844],[2560,1440],[1080,1920],[1366,768]]) {
  const p = new M44();
  if (canvasW > 1 && w < h) { p.scale(1, w/h); } else { p.scale(h/w, 1); }
  p.multiplyByMatrix(view);
  if (w < h) { p.scaleRelative(3.3,3.3); p.translateY(-2.1); } else { p.scaleRelative(4.1,4.1); p.translateY(-2.75); }
  p.multiplyByMatrix(modelM);
  const t = p.getArray();
  const nonId = []; for (let i=0;i<16;i++) if (![0,5,12,13].includes(i) && t[i] !== ([10,15].includes(i)?1:0)) nonId.push(i);
  console.log(`${w}x${h}: tr0=${t[0]} tr5=${t[5]} tr12=${t[12]} tr13=${t[13]} otherNonIdentity=[${nonId}]`);
}
console.log('modelMatrix scale f32:', modelM.getArray()[0], ' canvasHeight units:', canvasH);

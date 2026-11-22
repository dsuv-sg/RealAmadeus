function layout(usingClipCount, rtc, nctx) {
  const max = rtc <= 1 ? 36 : 32 * rtc; const res = []; const ctx = Array.from({length: nctx}, () => ({}));
  if (usingClipCount <= 0 || usingClipCount > max) { return {err: usingClipCount > max, all: 'ch0 full buf0'}; }
  const layoutCountMaxValue = rtc <= 1 ? 9 : 8;
  let countPerSheetDiv = usingClipCount / rtc; const reduce = usingClipCount % rtc; countPerSheetDiv = Math.ceil(countPerSheetDiv);
  let divCount = countPerSheetDiv / 4; const modCount = countPerSheetDiv % 4; divCount = ~~divCount; let cur = 0; const perCh = [];
  for (let r = 0; r < rtc; r++) for (let ch = 0; ch < 4; ch++) {
    let lc = divCount + (ch < modCount ? 1 : 0); const check = modCount + (divCount < 1 ? -1 : 0);
    if (ch == check && reduce > 0) lc -= !(r < reduce) ? 1 : 0;
    let kind = lc==0?'-':lc==1?'1':lc==2?'2':lc<=4?'4':lc<=layoutCountMaxValue?'9':'ERR';
    perCh.push(`${'RGBA'[ch]}${rtc>1?r:''}:${lc}(${kind})`); cur += Math.max(lc,0);
  }
  return perCh.join(' ') + `  assigned=${cur}`;
}
const rows=[]; for (let n=1;n<=37;n++) rows.push(`| ${n} | ${layout(n,1,n)} |`); console.log(rows.join('\n'));
console.log('rtc=2 examples:'); for (const n of [1,2,3,5,9,17,33,64,65]) console.log(n, JSON.stringify(layout(n,2,n)));

import sys
B = '/home/elvelt_admin/Elvelt/dsuv/public/resources/live2d/reama-cubism-runtime.js'
OUT = '/tmp/claude-0/-home-elvelt-admin-Elvelt-dsuv/1f3badd8-3ee9-4a13-9340-35fac0e7abf1/scratchpad/specs/spec-live2d-pipeline.md'
lines = open(B, encoding='utf-8').read().split('\n')
def ext(a, b):
    seg = lines[a-1:b]
    out = []
    for l in seg:
        out.append(l[2:] if l.startswith('  ') else l)
    return '\n'.join(out)
def rd(p): return open('parts/' + p, encoding='utf-8').read()
subs = {
    '%%PARAMTABLE%%': rd('paramtable.md').rstrip('\n'),
    '%%CACHEHELPERS%%': ext(2699, 2764),
    '%%PRELOAD%%': ext(18605, 18633),
    '%%EMOTION_TARGETS%%': ext(17690, 17872),
    '%%HELPERS%%': ext(17873, 17894),
    '%%MOTION_BURSTS%%': ext(17895, 17909),
    '%%IDLE%%': ext(17910, 18010),
    '%%FINDEXPR%%': ext(18034, 18074),
    '%%SETTERS%%': ext(18131, 18166),
    '%%APPLY%%': ext(18275, 18603),
    '%%VOWELSTRINGS%%': rd('vowels.md').rstrip('\n'),
    '%%FUNCS%%': rd('funcs.txt').rstrip('\n'),
    '%%TRACE1%%': rd('trace1.md').rstrip('\n'),
    '%%TRACE2%%': rd('trace2.md').rstrip('\n'),
    '%%APPENDIX%%': rd('appendix.md').rstrip('\n'),
}
doc = ''.join(rd(f) for f in ['f00.md', 'f01.md', 'f02.md', 'f03.md', 'f04.md', 'f05.md'])
for k, v in subs.items():
    n = doc.count(k)
    assert n == 1, (k, n)
    doc = doc.replace(k, v)
assert '%%' not in doc.replace('%%xx', ''), [i for i in range(len(doc)) if doc.startswith('%%', i)][:5]
open(OUT, 'w', encoding='utf-8').write(doc)
print('written', OUT, len(doc.encode('utf-8')), 'bytes', doc.count('\n'), 'lines')

import json, os, sys, glob, collections
d = sys.argv[1]
model_params = ["ParamAngleX","ParamAngleY","ParamAngleZ","ParamEyeLOpen","ParamEyeLSmile","ParamEyeROpen","ParamEyeRSmile","ParamEyeBallX","ParamEyeBallY","ParamBrowLForm","ParamBrowLY","ParamBrowRAngle","ParamBrowLX","ParamBrowRX","ParamBrowLAngle","ParamBrowRY","ParamBrowRForm","ParamMouthForm","ParamMouthOpenY","ParamCheek","ParamBodyAngleX","ParamBodyAngleY","ParamBodyAngleZ","ParamBreath","ParamHairFront","ParamHairSide","ParamHairBack"]
mp = set(model_params)
rows = []
allids = collections.Counter()
for f in sorted(glob.glob(os.path.join(d, "*.motion3.json"))):
    j = json.load(open(f, encoding="utf-8"))
    m = j.get("Meta", {})
    curves = j.get("Curves", [])
    targets = [c.get("Target") for c in curves]
    # ordering check: Model* Parameter* PartOpacity*
    order = {"Model":0,"Parameter":1,"PartOpacity":2}
    ok = all(order.get(targets[i],9) <= order.get(targets[i+1],9) for i in range(len(targets)-1))
    segtypes = collections.Counter()
    segs = 0; pts = 0
    curve_fades = 0
    for c in curves:
        s = c["Segments"]
        if "FadeInTime" in c or "FadeOutTime" in c: curve_fades += 1
        i = 0
        while i < len(s):
            if i == 0:
                pts += 1; i += 2
            t = int(s[i])
            segtypes[t] += 1
            if t == 1: pts += 3; i += 7
            elif t in (0,2,3): pts += 1; i += 3
            else: print("BAD SEGTYPE", f, t); break
            segs += 1
    ids = [c["Id"] for c in curves if c.get("Target")=="Parameter"]
    for x in ids: allids[x]+=1
    missing = [x for x in ids if x not in mp]
    tcount = collections.Counter(targets)
    rows.append((os.path.basename(f), m.get("Duration"), m.get("Fps"), m.get("Loop"), m.get("AreBeziersRestricted"), m.get("CurveCount"), len(curves), m.get("TotalSegmentCount"), segs, m.get("TotalPointCount"), pts, m.get("UserDataCount",0), m.get("FadeInTime"), m.get("FadeOutTime"), dict(tcount), ok, dict(segtypes), curve_fades, len(missing), len(ids)))
print("file | Duration | Fps | Loop | AreBeziersRestricted | CurveCount(meta/actual) | Segs(meta/actual) | Points(meta/actual) | UserData | FadeIn | FadeOut | targets | orderOK | segtypes | curvesWithOwnFade | missingParamIds/paramCurves")
for r in rows:
    print(f"{r[0]} | {r[1]} | {r[2]} | {r[3]} | {r[4]} | {r[5]}/{r[6]} | {r[7]}/{r[8]} | {r[9]}/{r[10]} | {r[11]} | {r[12]} | {r[13]} | {r[14]} | {r[15]} | {r[16]} | {r[17]} | {r[18]}/{r[19]}")
print()
print("All Parameter ids used across motions (count):")
for k,v in sorted(allids.items()):
    print(f"  {k}: {v} {'(exists)' if k in mp else '(NOT IN MODEL)'}")

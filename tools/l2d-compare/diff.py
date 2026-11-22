#!/usr/bin/env python3
"""V1-vs-V2 pixel statistics for an l2d-compare capture directory (tools/l2d-compare/run.sh calls it).

    python3 -I tools/l2d-compare/diff.py SHOTS_DIR [--threshold 8]

For every scenario directory (<W>x<H>[@DPR]-<EMOTION>-f<NNN>) holding v1.png and v2.png it writes:
  diff.png         heatmap of the screenshots: dimmed V1 luma, per-pixel max-channel |V1 - V2|
                   (blue = 1..T, red -> yellow -> white = T+1 .. 96+)
  compare.png      sheet V1 | V2 | heatmap (half size when wider than 2400 px)
  canvas-diff.png  heatmap of the WebGL canvas readbacks (premultiplied RGB and alpha), when both exist
  rep-*.png        heatmaps of repeated captures (<ver>.rep<k>.png vs <ver>.png): the harness noise floor
  ctrl-diff-f<NNN>.png  heatmap of the sensitivity control (V1 at frame N vs ctrl-f<NNN>.v1.png = V1 at N+1)
and SHOTS_DIR/results.json + results.txt (the table printed to stdout).

Trace column (from the shim reports, V1 vs V2): "ok" when the WebGL draw-call sequences of the first and
last frame (count + framebuffer target per call), the draw count of every frame, the Math.random trace
(drawn frame of each call + value), the console lines, the context attributes, the canvas sizes/styles and
the do= action log are all equal and neither page logged an error; otherwise the names of the checks that
failed.

Screenshot metrics (0..255 scale, T = --threshold, default 8):
  max        largest channel difference
  mean       mean |V1 - V2| over all pixels and RGB channels
  %>T        share of pixels whose largest channel difference exceeds T
  meanM/%>TM the same restricted to the union of the two model silhouettes (pixel max channel > 16)
  IoU        intersection over union of the silhouettes; bbox/centroid/phase-correlation shift help to
             tell framing/scale problems from local (motion, mask, colour) differences
Canvas metrics: the same on premultiplied RGB, plus alpha (%a>T).
"""
import json
import os
import re
import sys

import numpy as np
from PIL import Image, ImageDraw

SILHOUETTE = 16  # pixel max channel above this = model (screenshots are composited on black)
ALPHA_SIL = 8


def load_rgba(path):
    return np.asarray(Image.open(path).convert("RGBA")).astype(np.int16)


def bbox(mask):
    if not mask.any():
        return None
    ys = np.flatnonzero(mask.any(axis=1))
    xs = np.flatnonzero(mask.any(axis=0))
    return [int(xs[0]), int(ys[0]), int(xs[-1]), int(ys[-1])]


def centroid(mask):
    if not mask.any():
        return None
    ys, xs = np.nonzero(mask)
    return [round(float(xs.mean()), 2), round(float(ys.mean()), 2)]


def phase_shift(ga, gb):
    """Integer translation (dx, dy) that best maps B onto A, and the correlation peak (1 = pure shift)."""
    fa = np.fft.rfft2(ga)
    fb = np.fft.rfft2(gb)
    r = fa * np.conj(fb)
    r /= np.abs(r) + 1e-9
    corr = np.fft.irfft2(r, s=ga.shape)
    py, px = np.unravel_index(int(np.argmax(corr)), corr.shape)
    h, w = ga.shape
    if py > h // 2:
        py -= h
    if px > w // 2:
        px -= w
    return int(px), int(py), round(float(corr.max()), 3)


def luma(img):
    return 0.299 * img[..., 0] + 0.587 * img[..., 1] + 0.114 * img[..., 2]


def compare(a, b, thr, canvas=False):
    """Statistics of one image pair (int16 HxWx4)."""
    if a.shape != b.shape:
        return {"size_mismatch": [list(a.shape), list(b.shape)]}
    if canvas:
        # toDataURL un-premultiplies; compare what the compositor uses (premultiplied RGB) + alpha.
        pa = a.astype(np.float32)
        pb = b.astype(np.float32)
        ca = np.concatenate([pa[..., :3] * pa[..., 3:4] / 255.0, pa[..., 3:4]], axis=2)
        cb = np.concatenate([pb[..., :3] * pb[..., 3:4] / 255.0, pb[..., 3:4]], axis=2)
        rgb_d = np.abs(np.rint(ca[..., :3]) - np.rint(cb[..., :3]))
        alpha_d = np.abs(a[..., 3] - b[..., 3])
        ma = a[..., 3] > ALPHA_SIL
        mb = b[..., 3] > ALPHA_SIL
        ga, gb = luma(ca), luma(cb)
    else:
        rgb_d = np.abs(a[..., :3] - b[..., :3]).astype(np.float32)
        alpha_d = None
        ma = a[..., :3].max(axis=2) > SILHOUETTE
        mb = b[..., :3].max(axis=2) > SILHOUETTE
        ga, gb = luma(a), luma(b)
    d = rgb_d.max(axis=2)
    if alpha_d is not None:
        d = np.maximum(d, alpha_d)
    union = ma | mb
    inter = ma & mb
    big = d > thr
    s = {
        "max": int(d.max()),
        "mean": round(float(rgb_d.mean()), 4),
        "pct_gt": round(float(big.mean() * 100), 4),
        "n_diff": int((d > 0).sum()),
        "n_gt": int(big.sum()),
        "mean_model": round(float(rgb_d[union].mean()), 4) if union.any() else 0.0,
        "pct_gt_model": round(float(big[union].mean() * 100), 4) if union.any() else 0.0,
        "iou": round(float(inter.sum() / max(1, union.sum())), 5),
        "area": [int(ma.sum()), int(mb.sum())],
        "bbox": [bbox(ma), bbox(mb)],
        "centroid": [centroid(ma), centroid(mb)],
        "diff_bbox": bbox(big),
    }
    if alpha_d is not None:
        s["alpha_max"] = int(alpha_d.max())
        s["alpha_pct_gt"] = round(float((alpha_d > thr).mean() * 100), 4)
    if s["pct_gt"] > 0.01:
        s["shift"] = phase_shift(ga, gb)
    s["_d"] = d
    return s


def classify(s):
    if "size_mismatch" in s:
        return "size mismatch (canvas/viewport size)"
    if not s["area"][0] and not s["area"][1]:
        return "both blank"
    if not s["area"][0] or not s["area"][1]:
        return "one side blank (%s missing)" % ("V1" if not s["area"][0] else "V2")
    if s["max"] == 0:
        return "identical"
    if s["n_gt"] == 0:
        return "sub-threshold noise (<=T)"
    ba, bb = s["bbox"]
    edge = max(abs(x - y) for x, y in zip(ba, bb))
    area_ratio = s["area"][1] / max(1, s["area"][0])
    shift = s.get("shift", (0, 0, 0))
    if s["iou"] < 0.97 or edge > 4:
        if (shift[0] or shift[1]) and shift[2] > 0.3:
            return "framing: translated (dx=%d, dy=%d)" % (shift[0], shift[1])
        if abs(area_ratio - 1) > 0.03:
            return "framing/scale or missing parts (area x%.3f, bbox edge %dpx)" % (area_ratio, edge)
        return "silhouette differs (IoU %.3f, bbox edge %dpx)" % (s["iou"], edge)
    if s["pct_gt_model"] > 25:
        return "colour/alpha (widespread, %.1f%% of the model)" % s["pct_gt_model"]
    return "local differences (%.2f%% of the model): motion/timing, parts or masks" % s["pct_gt_model"]


def heatmap(base, d, thr):
    """Dimmed luma of `base` + |diff| colour map; returns uint8 RGB."""
    img = np.repeat((luma(base.astype(np.float32)) * 0.3)[..., None], 3, axis=2)
    small = (d > 0) & (d <= thr)
    img[small] = (40, 70, 220)
    big = d > thr
    if big.any():
        t = np.clip((d[big] - thr) / float(max(1, 96 - thr)), 0.0, 1.0)
        r = np.full_like(t, 255.0)
        g = np.clip(t * 2.0, 0, 1) * 255.0
        b = np.clip(t * 2.0 - 1.0, 0, 1) * 255.0
        img[big] = np.stack([r, g, b], axis=1)
    return np.clip(img, 0, 255).astype(np.uint8)


def sheet(panels, labels, path):
    h = max(p.shape[0] for p in panels)
    gap = 8
    w = sum(p.shape[1] for p in panels) + gap * (len(panels) - 1)
    out = Image.new("RGB", (w, h + 24), (40, 40, 40))
    draw = ImageDraw.Draw(out)
    x = 0
    for p, label in zip(panels, labels):
        out.paste(Image.fromarray(p[..., :3].astype(np.uint8)), (x, 24))
        draw.text((x + 6, 6), label, fill=(255, 255, 255))
        x += p.shape[1] + gap
    if out.width > 2400:
        out = out.resize((out.width // 2, out.height // 2), Image.LANCZOS)
    out.save(path)


def report_meta(path):
    try:
        with open(path, encoding="utf-8") as f:
            m = json.load(f)
    except (OSError, ValueError):
        return None
    r = m.get("report") or {}
    return {
        "ok": m.get("ok"),
        "status": r.get("status"),
        "frame": r.get("frame"),
        "armedAtTick": r.get("armedAtTick"),
        "draws_last": len(r.get("lastFrameDrawLog") or []),
        "draws_first": len(r.get("firstFrameDrawLog") or []),
        "first_draw_log": r.get("firstFrameDrawLog") or [],
        "last_draw_log": r.get("lastFrameDrawLog") or [],
        "per_frame_draws": r.get("perFrameDraws") or [],
        "console": r.get("console") or [],
        "action_log": r.get("actionLog") or [],
        "random_calls": r.get("randomCalls"),
        "random_at_arm": r.get("randomCallsAtArm"),
        "random_log": r.get("randomLog") or [],
        "errors": r.get("errors") or [],
        "canvases": r.get("canvases") or [],
        "contexts": r.get("contexts") or [],
        "console_errors": [c for c in (r.get("console") or []) if c.startswith(("error", "warn"))],
        "cdp_errors": [c for c in m.get("cdp", []) if c.startswith(("EXCEPTION", "error", "warn", "log("))],
    }


def rng_trace(m):
    """Math.random calls as [drawn frame (0 = before frame 1), value]: comparable across pages that need a
    different number of loading ticks."""
    armed = m.get("armedAtTick") or 0
    out = []
    for tick, in_frame, value in m.get("random_log") or []:
        out.append([tick - armed + 1 if (in_frame and armed and tick >= armed) else 0, value])
    return out


def trace_checks(m1, m2):
    """Structural V1-vs-V2 checks from the shim reports; returns {check: passed}."""
    if not m1 or not m2:
        return {"reports": False}

    def ctx(m):
        return [[c.get("requested"), c.get("actual")] if isinstance(c, dict) else c for c in m["contexts"]]

    return {
        "finished": bool(m1["ok"] and m2["ok"]),
        "draws_first": m1["first_draw_log"] == m2["first_draw_log"],
        "draws_last": m1["last_draw_log"] == m2["last_draw_log"],
        "draws_per_frame": m1["per_frame_draws"] == m2["per_frame_draws"],
        "rng": m1["random_calls"] == m2["random_calls"] and rng_trace(m1) == rng_trace(m2),
        "console": m1["console"] == m2["console"],
        "context": ctx(m1) == ctx(m2),
        "canvas": m1["canvases"] == m2["canvases"],
        "actions": m1["action_log"] == m2["action_log"],
        "no_errors": not (m1["errors"] or m2["errors"] or m1["console_errors"] or m2["console_errors"]),
    }


def strip(s):
    return {k: v for k, v in s.items() if not k.startswith("_")}


def main():
    args = sys.argv[1:]
    thr = 8
    if "--threshold" in args:
        i = args.index("--threshold")
        thr = int(args[i + 1])
        del args[i : i + 2]
    if len(args) != 1:
        print(__doc__)
        sys.exit(2)
    root = args[0]
    results = []
    names = sorted(
        n for n in os.listdir(root) if os.path.isfile(os.path.join(root, n, "v1.png")) and os.path.isfile(os.path.join(root, n, "v2.png"))
    )
    if not names:
        print("no scenario directories with v1.png + v2.png under", root)
        sys.exit(1)

    order = ["NORMAL", "SMILE", "SLEEPING"]

    def sort_key(n):
        m = re.match(r"^(\d+)x(\d+)(?:@([\d.]+))?-(.+)-f(\d+)(?:-(.*))?$", n)
        if not m:
            return (1, n)
        w, h, dpr, emo, f, tag = m.groups()
        return (0, tag or "", -int(w), int(h), float(dpr or 1), order.index(emo) if emo in order else 9, emo, int(f))

    names.sort(key=sort_key)
    for name in names:
        d = os.path.join(root, name)
        a = load_rgba(os.path.join(d, "v1.png"))
        b = load_rgba(os.path.join(d, "v2.png"))
        s = compare(a, b, thr)
        entry = {"scenario": name, "shot": strip(s), "class": classify(s)}
        if "_d" in s:
            Image.fromarray(heatmap(a, s["_d"], thr)).save(os.path.join(d, "diff.png"))
            sheet([a, b, heatmap(a, s["_d"], thr)], ["V1 (deployed bundle)", "V2 (amadeus-live2d)", "|V1-V2| (T=%d)" % thr], os.path.join(d, "compare.png"))
        ca_p, cb_p = os.path.join(d, "v1.canvas.png"), os.path.join(d, "v2.canvas.png")
        if os.path.isfile(ca_p) and os.path.isfile(cb_p):
            ca, cb = load_rgba(ca_p), load_rgba(cb_p)
            cs = compare(ca, cb, thr, canvas=True)
            entry["canvas"] = strip(cs)
            if "_d" in cs:
                Image.fromarray(heatmap(ca, cs["_d"], thr)).save(os.path.join(d, "canvas-diff.png"))
        reps = {}
        for f in sorted(os.listdir(d)):
            if f.startswith(("v1.rep", "v2.rep")) and f.endswith(".png") and ".canvas." not in f:
                ver = f.split(".rep")[0]
                ref = load_rgba(os.path.join(d, ver + ".png"))
                rs = compare(ref, load_rgba(os.path.join(d, f)), thr)
                reps[f] = {"max": rs.get("max"), "pct_gt": rs.get("pct_gt"), "n_diff": rs.get("n_diff")}
                if "_d" in rs and rs["max"]:
                    Image.fromarray(heatmap(ref, rs["_d"], thr)).save(os.path.join(d, "rep-" + f))
        if reps:
            entry["repeat"] = reps
        ctrls = {}
        for f in sorted(os.listdir(d)):
            m = re.match(r"^ctrl-f(\d+)\.v1\.png$", f)
            if not m:
                continue
            cs2 = compare(a, load_rgba(os.path.join(d, f)), thr)
            if "_d" not in cs2:
                ctrls[f] = {"size_mismatch": cs2.get("size_mismatch")}
                continue
            ctrls[f] = {k: cs2.get(k) for k in ("max", "mean", "pct_gt", "n_diff", "n_gt", "mean_model", "pct_gt_model")}
            ctrls[f]["detected"] = cs2["max"] > 0
            Image.fromarray(heatmap(a, cs2["_d"], thr)).save(os.path.join(d, "ctrl-diff-f%s.png" % m.group(1)))
        if ctrls:
            entry["controls"] = ctrls
        entry["v1"] = report_meta(os.path.join(d, "v1.json"))
        entry["v2"] = report_meta(os.path.join(d, "v2.json"))
        entry["trace"] = trace_checks(entry["v1"], entry["v2"])
        results.append(entry)

    width = max(26, max(len(e["scenario"]) for e in results))
    hdr = "%-*s %4s %8s %8s %8s %8s %7s %-9s | %4s %6s %6s | %-11s %-11s | %-6s | %s" % (
        width, "scenario", "max", "mean", "%>T", "meanM", "%>TM", "IoU", "shift", "cmax", "c%>T", "c%a>T",
        "v1 drw/rnd/e", "v2 drw/rnd/e", "trace", "class")
    lines = [hdr, "-" * len(hdr)]
    for e in results:
        s, c = e["shot"], e.get("canvas", {})
        sh = s.get("shift")
        meta = []
        for v in ("v1", "v2"):
            m = e.get(v) or {}
            meta.append("%s/%s/%s%s" % (m.get("draws_last", "?"), m.get("random_calls", "?"), len(m.get("errors", [])) + len(m.get("console_errors", [])), "" if m.get("ok") else " !"))
        failed = [k for k, ok in e["trace"].items() if not ok]
        trace = "ok" if not failed else "DIFF"
        if "size_mismatch" in s:
            lines.append("%-*s size mismatch %s | trace %s" % (width, e["scenario"], s["size_mismatch"], trace))
        else:
            lines.append("%-*s %4d %8.4f %8.4f %8.4f %8.4f %7.4f %-9s | %4s %6s %6s | %-11s %-11s | %-6s | %s" % (
                width, e["scenario"], s["max"], s["mean"], s["pct_gt"], s["mean_model"], s["pct_gt_model"], s["iou"],
                ("%d,%d" % (sh[0], sh[1])) if sh else "-",
                c.get("max", "-"), c.get("pct_gt", "-"), c.get("alpha_pct_gt", "-"), meta[0], meta[1], trace, e["class"]))
        if failed:
            lines.append("%-*s   trace checks failed: %s" % (width, "", ", ".join(failed)))
        if e.get("repeat"):
            lines.append("%-*s   noise floor (repeat): %s" % (width, "", ", ".join("%s max=%s" % (k, v["max"]) for k, v in e["repeat"].items())))
        for k, v in (e.get("controls") or {}).items():
            if "size_mismatch" in v:
                lines.append("%-*s   control %s: size mismatch %s" % (width, "", k, v["size_mismatch"]))
                continue
            lines.append("%-*s   control V1 vs %s (one frame later): max=%d mean=%.4f %%>T=%.4f n_diff=%d -> %s" % (
                width, "", k, v["max"], v["mean"], v["pct_gt"], v["n_diff"], "detected" if v["detected"] else "NOT DETECTED"))
    n = len(results)
    same = sum(1 for e in results if e["shot"].get("max") == 0 and (e.get("canvas") or {}).get("max", 0) == 0)
    tr = sum(1 for e in results if all(e["trace"].values()))
    lines.append("")
    lines.append("summary: %d scenarios, %d pixel-identical (screenshot and canvas readback), %d with all trace checks ok" % (n, same, tr))
    text = "\n".join(lines)
    print(text)
    with open(os.path.join(root, "results.txt"), "w", encoding="utf-8") as f:
        f.write("threshold T=%d (0..255)\n%s\n" % (thr, text))
    with open(os.path.join(root, "results.json"), "w", encoding="utf-8") as f:
        json.dump({"threshold": thr, "results": results}, f, indent=1)


if __name__ == "__main__":
    main()

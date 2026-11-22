# python3 -I imgdiff.py A.png B.png [diff.png]  — exit 0 iff both images are pixel-identical.
import sys

import numpy as np
from PIL import Image

a = np.asarray(Image.open(sys.argv[1]).convert("RGBA")).astype(int)
b = np.asarray(Image.open(sys.argv[2]).convert("RGBA")).astype(int)
print("sizes", a.shape, b.shape)
if a.shape != b.shape:
    sys.exit(1)
d = np.abs(a - b)
nz = d.max(axis=2) > 0
print(
    "max diff", d.max(),
    "differing pixels", int(nz.sum()), "of", nz.size,
    "non-black A", int((a[..., :3].sum(axis=2) > 0).sum()),
    "non-black B", int((b[..., :3].sum(axis=2) > 0).sum()),
)
if len(sys.argv) > 3 and nz.any():
    Image.fromarray(np.clip(d[..., :3] * 16, 0, 255).astype("uint8")).save(sys.argv[3])
if (a[..., :3].sum(axis=2) > 0).sum() == 0:
    print("both images are black")
    sys.exit(3)
sys.exit(0 if not nz.any() else 2)

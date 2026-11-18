#!/usr/bin/env python3
"""Scratch pipeline for Nodalia app-icon candidates (derived from tools/app-portrait/portrait.py).
BiRefNet-portrait matte on the crop that fills the tile, alpha clean-up (main figure only, small holes
filled, smooth contour pulled in ~0.5px), edge foreground-colour estimation, optional soft side fade,
clip to the rounded tile (radius 112/512 = 224 px at 1024) with transparent corners."""
import json, os, sys
import cv2
import numpy as np
from PIL import Image, ImageDraw, ImageEnhance, ImageFilter, ImageOps

Image.MAX_IMAGE_PIXELS = None
SIZE, RADIUS, MARGIN = 1024, 224, 0.25
HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.expanduser('~/Developer/nodalia-portraits/2026-11-18')
os.environ.setdefault('U2NET_HOME', os.path.join(HERE, 'models'))
_s = {}

def session(m):
    from rembg import new_session
    if m not in _s: _s[m] = new_session(m)
    return _s[m]

def rgb(h):
    h = h.lstrip('#'); return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))

def load(path):
    im = ImageOps.exif_transpose(Image.open(path))
    if im.mode in ('I;16', 'I'):
        a = np.asarray(im, dtype=np.float32); a = (a - a.min()) / max(1, a.max() - a.min())
        im = Image.fromarray((a * 255).astype(np.uint8))
    return im.convert('RGB')

def model_matte(img, model):
    from rembg import remove
    return np.asarray(remove(img, session=session(model), only_mask=True), dtype=np.float32) / 255.0

def region(img, box, pad):
    L, T, R, B = box; s = SIZE / (R - L)
    pl, pt, pr, pb = L - pad * (R - L), T - pad * (B - T), R + pad * (R - L), B + pad * (B - T)
    W, H = round((pr - pl) * s), round((pb - pt) * s)
    il, it = max(0, int(np.floor(pl))), max(0, int(np.floor(pt)))
    ir, ib = min(img.width, int(np.ceil(pr))), min(img.height, int(np.ceil(pb)))
    out = Image.new('RGB', (W, H), (128, 128, 128)); valid = Image.new('L', (W, H), 0)
    piece = img.crop((il, it, ir, ib)).resize((max(1, round((ir - il) * s)), max(1, round((ib - it) * s))), Image.LANCZOS)
    ox, oy = round((il - pl) * s), round((it - pt) * s)
    out.paste(piece, (ox, oy)); valid.paste(255, (ox, oy, ox + piece.width, oy + piece.height))
    arr = np.asarray(out).copy(); v = np.asarray(valid) > 0
    ys, xs = np.where(v); y0, y1, x0, x1 = ys.min(), ys.max(), xs.min(), xs.max()
    arr[:y0] = arr[y0:y0 + 1]; arr[y1 + 1:] = arr[y1:y1 + 1]; arr[:, :x0] = arr[:, x0:x0 + 1]; arr[:, x1 + 1:] = arr[:, x1:x1 + 1]
    return Image.fromarray(arr), v.astype(np.float32)

def keep_main(alpha):
    b = (alpha > 0.5).astype(np.uint8)
    if b.sum() == 0: return alpha
    n, lab, st, _ = cv2.connectedComponentsWithStats(cv2.dilate(b, np.ones((9, 9), np.uint8)), connectivity=8)
    main = 1 + int(np.argmax(st[1:, cv2.CC_STAT_AREA]))
    a = alpha * cv2.dilate((lab == main).astype(np.uint8), np.ones((13, 13), np.uint8))
    inv = 1 - (a > 0.5).astype(np.uint8)
    n, lab, st, _ = cv2.connectedComponentsWithStats(inv, connectivity=4)
    for i in range(1, n):
        x, y, w, h, area = st[i]
        if not (x == 0 or y == 0 or x + w >= a.shape[1] or y + h >= a.shape[0]) and area < 0.012 * a.size: a[lab == i] = 1.0
    return a

def smooth_alpha(a, sigma=2.4, k=3.5, thr=0.56):
    b = np.clip((cv2.GaussianBlur(a, (0, 0), sigma) - thr) * k + 0.5, 0, 1)
    return b * b * (3 - 2 * b)

def decontaminate(img, alpha):
    from pymatting import estimate_foreground_ml
    f = estimate_foreground_ml(np.asarray(img, dtype=np.float64) / 255.0, alpha.astype(np.float64))
    return Image.fromarray((np.clip(f, 0, 1) * 255).astype(np.uint8))

def grey(fig, alpha, gamma=1.0, clahe=1.6, lo=0.5, hi=99.7):
    g = np.asarray(ImageOps.grayscale(fig), dtype=np.float32); m = alpha > 0.9
    a, b = np.percentile(g[m], [lo, hi]) if m.sum() > 100 else (g.min(), g.max())
    g = np.clip((g - a) / max(1.0, b - a), 0, 1) ** gamma
    g8 = (g * 255).astype(np.uint8)
    if clahe: g8 = cv2.createCLAHE(clipLimit=clahe, tileGridSize=(6, 6)).apply(g8)
    return ImageEnhance.Sharpness(ImageEnhance.Contrast(Image.fromarray(g8)).enhance(1.12)).enhance(1.35)

def field(top, bottom):
    t, b = rgb(top), rgb(bottom)
    col = np.zeros((SIZE, SIZE, 3), np.float32)
    for c in range(3): col[:, :, c] = np.linspace(t[c], b[c], SIZE)[:, None]
    return Image.fromarray(col.astype(np.uint8)).convert('RGBA')

def tile_mask():
    ss = 4; m = Image.new('L', (SIZE * ss,) * 2, 0)
    ImageDraw.Draw(m).rounded_rectangle((0, 0, SIZE * ss - 1, SIZE * ss - 1), RADIUS * ss, fill=255)
    return m.resize((SIZE, SIZE), Image.LANCZOS)

def side_fade(a, w):
    if not w: return a
    d = np.clip(np.minimum(np.arange(SIZE), SIZE - 1 - np.arange(SIZE)) / w, 0, 1); d = d * d * (3 - 2 * d)
    return a * d[None, :].astype(np.float32)

def cut_polygons(a, polys, off=0):
    """Remove foreground inside manual polygons (tile px; `off` = tile origin inside `a`) -- crops a stray hand out of the frame."""
    if not polys: return a
    h, w = a.shape; m = Image.new('L', (w * 4, h * 4), 0); d = ImageDraw.Draw(m)
    for poly in polys: d.polygon([((x + off) * 4, (y + off) * 4) for x, y in poly], fill=255)
    m = np.asarray(m.resize((w, h), Image.LANCZOS), dtype=np.float32) / 255.0
    return a * (1 - m)

def make(spec, out_png, stem=None):
    """Writes <out_png> (rounded tile, transparent corners) and, when stem is given, <stem>-square.png
    (full-bleed field + figure, same framing, no clip) and <stem>-figure.png (figure alone on transparency)."""
    img = load(os.path.join(SRC, spec['file']))
    if spec.get('pre'): img = img.crop(tuple(spec['pre']))
    if spec.get('blur'): img = img.filter(ImageFilter.GaussianBlur(spec['blur']))
    box = spec['box']
    reg, valid = region(img, box, MARGIN)
    m0 = model_matte(reg, spec.get('model', 'birefnet-portrait'))
    if spec.get('bleed'):
        # garment continues past the source edge: the model reads the replicated border as background (matte decays over the last
        # ~8 px), so replicate the matte outward from a row/column 16 px inside the real source
        ys, xs = np.where(valid > 0.5); y0, y1, x0, x1 = ys.min() + 16, ys.max() - 16, xs.min() + 16, xs.max() - 16
        m0[:y0] = m0[y0:y0 + 1]; m0[y1 + 1:] = m0[y1:y1 + 1]; m0[:, :x0] = m0[:, x0:x0 + 1]; m0[:, x1 + 1:] = m0[:, x1:x1 + 1]
    else: m0 = m0 * (valid > 0.5)
    o = round(MARGIN * SIZE); sl = (slice(o, o + SIZE), slice(o, o + SIZE))
    tile = reg.crop((o, o, o + SIZE, o + SIZE))
    if spec.get('bleed'):
        # the garment runs off the tile edge: clean the matte on the padded region so alpha reaches the edge, crop after
        a = smooth_alpha(cut_polygons(keep_main(m0), spec.get('cut'), o), spec.get('sigma', 2.4), spec.get('k', 3.5), spec.get('thr', 0.56))[sl]
    else:
        a = cut_polygons(keep_main(m0[sl]), spec.get('cut'))
        a = smooth_alpha(a, spec.get('sigma', 2.4), spec.get('k', 3.5), spec.get('thr', 0.56))
    a = side_fade(a, spec.get('fade', 0))
    g = grey(decontaminate(tile, a), a, gamma=spec.get('gamma', 1.0), clahe=spec.get('clahe', 1.6))
    ai = Image.fromarray((a * 255).astype(np.uint8))
    canvas = field(*spec['colors'])
    sh = Image.new('L', (SIZE, SIZE), 0); sh.paste(ai.point(lambda v: v * 0.28).filter(ImageFilter.GaussianBlur(16)), (5, 9))
    lay = Image.new('RGBA', (SIZE, SIZE), (0, 0, 0, 0)); lay.putalpha(sh)
    canvas = Image.alpha_composite(canvas, lay)
    fig = g.convert('RGBA'); fig.putalpha(ai)
    canvas = Image.alpha_composite(canvas, fig)
    if stem:
        canvas.convert('RGB').save(stem + '-square.png')
        fig.save(stem + '-figure.png')
    canvas.putalpha(tile_mask()); canvas.save(out_png)
    return dict(scale=SIZE / (box[2] - box[0]), side_left=float(a[:, :4].max(1).mean()), side_right=float(a[:, -4:].max(1).mean()))

if __name__ == '__main__':
    cfg = json.load(open(os.path.join(HERE, 'figures.json')))
    out = os.environ.get('ICON_OUT', os.path.join(HERE, 'out')); os.makedirs(out, exist_ok=True)
    for n in (sys.argv[1:] or list(cfg)):
        print(n, json.dumps(make(cfg[n], os.path.join(out, n + '.png'), os.path.join(out, n))), flush=True)

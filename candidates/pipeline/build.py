"""build.py DEST  -- sheets + README for the kept candidates (keep.json: {app: [name,...]}, notes.json)"""
import json, os, re, shutil, sys
from PIL import Image, ImageDraw, ImageFont
HERE = os.path.dirname(os.path.abspath(__file__))
DEST = sys.argv[1]
keep = json.load(open(os.path.join(HERE, 'keep.json')))
notes = json.load(open(os.path.join(HERE, 'notes.json')))
cfg = json.load(open(os.path.join(HERE, 'figures.json')))
src = {e['file']: e for e in json.load(open(os.path.expanduser('~/Developer/nodalia-portraits/2026-11-18/sources.json')))}
os.makedirs(DEST, exist_ok=True)
BG = (238, 238, 240)
try: font = ImageFont.truetype('/System/Library/Fonts/Helvetica.ttc', 15); small = ImageFont.truetype('/System/Library/Fonts/Helvetica.ttc', 11)
except Exception: font = small = ImageFont.load_default()

def flat(im, size):
    t = im.resize((size, size), Image.LANCZOS)
    bg = Image.new('RGBA', t.size, BG + (255,)); bg.alpha_composite(t); return bg.convert('RGB')

for app, names in keep.items():
    cols = 2; cw, ch = 360 + 24 + 64 + 12 + 16 + 24, 360 + 44
    rows = (len(names) + cols - 1) // cols
    sheet = Image.new('RGB', (cols * cw + 24, rows * ch + 24 + 30), BG)
    d = ImageDraw.Draw(sheet); d.text((24, 12), app.capitalize() + ' - icon candidates', fill=(40, 40, 44), font=font)
    for i, n in enumerate(names):
        im = Image.open(os.path.join(HERE, 'out', n + '.png')).convert('RGBA')
        im.save(os.path.join(DEST, n + '.png'))
        x, y = 24 + (i % cols) * cw, 40 + (i // cols) * ch
        sheet.paste(flat(im, 360), (x, y)); d.text((x, y + 366), n, fill=(30, 30, 34), font=font)
        sheet.paste(flat(im, 64), (x + 384, y)); sheet.paste(flat(im, 16), (x + 384 + 76, y + 24))
        d.text((x + 384, y + 68), '64 px', fill=(110, 110, 116), font=small); d.text((x + 384 + 76, y + 44), '16 px', fill=(110, 110, 116), font=small)
    sheet.save(os.path.join(DEST, app + '-sheet.png'))

lines = ['# Icon candidates (2026-11-18)', '',
 'Candidate app-icon portraits: black-and-white historical portrait on the app colour, 1024 px, clipped to the rounded tile (radius 112/512 = 224 px at 1024, transparent corners). One `<app>-sheet.png` per app (candidate at 360 px, plus 64 px and 16 px); the individual 1024 px PNGs sit beside them.', '',
 'Field colours are the approximations given in the brief (top / bottom); the DMG colours in `tools/dmg-background/draw_background.py` differ for Forja, Hito, Pliego and Vinilo.', '',
 'Cut-out: BiRefNet-portrait matte on the crop that fills the tile, main figure only, smooth contour pulled in ~0.5 px (no halo), edge colour decontamination, soft side fade where shoulders cannot fit. Scripts in `pipeline/`. `scale` = tile px per source px (below 1 = downsampled, never upscaled).', '']
for app, names in keep.items():
    lines += ['## ' + app.capitalize(), '', '| Candidate | Figure | Source (Commons) | Author | Licence | Source px | Scale | Note |', '|---|---|---|---|---|---|---|---|']
    for n in names:
        s = src[cfg[n]['file']]; sc = 1024 / (cfg[n]['box'][2] - cfg[n]['box'][0])
        lines.append('| `%s.png` | %s | [%s](%s) | %s | %s | %dx%d | %.2f | %s |' % (n, s['figure'], s['commons_page_url'].split('File:')[-1][:40] + '...', s['commons_page_url'], re.sub(r'\s+', ' ', s['author'])[:60], s['licence'], s['width'], s['height'], sc, notes.get(n, '')))
    lines.append('')
lines += ['## Dropped', ''] + ['- ' + l for l in notes.get('_dropped', [])]
open(os.path.join(DEST, 'README.md'), 'w').write('\n'.join(lines) + '\n')
print('built', DEST)

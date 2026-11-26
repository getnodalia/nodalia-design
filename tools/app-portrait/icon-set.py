#!/usr/bin/env python3
"""Turn an app's portrait artwork (the 1024 px output of portrait.py) into the icon files an app
ships, without needing resvg or any other renderer.

Usage:
  icon-set.py PORTRAIT.png OUT_DIR --slug codigo --app-id dev.danielmiguelt.Codigo --comment TEXT

Writes into OUT_DIR:
  <slug>.svg                      the 512 full-bleed master (portrait clipped to a rounded square,
                                  corner radius 112) with the artwork embedded
  <slug>-linux.svg                Colloid's application grid: a 224 tile at 16 on a 256 canvas,
                                  corner radius 51.5, soft shadow 5 px below
  <slug>-<size>.png               the master on Apple's grid (the tile is 824/1024 of the canvas,
                                  transparent margin), sizes 16 to 1024
  <slug>.ico                      Windows icon, 16 to 256 px
  hicolor/<size>x<size>/apps/<app-id>.png   the Linux grid at 16, 24, 32, 48, 64, 128, 256, 512 px
  hicolor/scalable/apps/<app-id>.svg        the Linux grid SVG

Needs Pillow only. The drawing is supersampled, so small sizes stay clean.
"""
import argparse
import base64
import io
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

QUALITY = 90
SS = 4  # supersampling factor for the rounded masks
LINUX_SIZES = (16, 24, 32, 48, 64, 128, 256, 512)
APPLE_SIZES = (16, 32, 48, 64, 128, 256, 512, 1024)
ICO_SIZES = (16, 24, 32, 48, 64, 128, 256)


def jpeg(image, size):
    buffer = io.BytesIO()
    image.convert('RGB').resize((size, size), Image.LANCZOS).save(
        buffer, 'JPEG', quality=QUALITY, optimize=True, progressive=True)
    return base64.b64encode(buffer.getvalue()).decode()


def rounded_mask(size, radius):
    big = Image.new('L', (size * SS, size * SS), 0)
    ImageDraw.Draw(big).rounded_rectangle((0, 0, size * SS - 1, size * SS - 1), radius=radius * SS, fill=255)
    return big.resize((size, size), Image.LANCZOS)


def shadow_layer():
    """Colloid's soft shadow, 256 canvas, as an RGBA image."""
    scale, canvas, margin, tile = 2, 256, 16, 224
    mask = Image.new('L', (canvas * scale, canvas * scale), 0)
    top = (margin + 5) * scale
    ImageDraw.Draw(mask).rounded_rectangle(
        (margin * scale, top, (margin + tile) * scale, top + tile * scale),
        radius=51.5 * scale, fill=round(255 * 0.144))
    mask = mask.filter(ImageFilter.GaussianBlur(3.5 * scale))
    layer = Image.new('LA', mask.size, 0)
    layer.putalpha(mask)
    return layer


def master_svg(art, comment):
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">\n'
        f'  {comment}\n'
        '  <defs><clipPath id="tile"><rect width="512" height="512" rx="112"/></clipPath></defs>\n'
        f'  <image width="512" height="512" clip-path="url(#tile)" href="data:image/jpeg;base64,{jpeg(art, 1024)}"/>\n'
        '</svg>\n')


def linux_svg(art, comment):
    buffer = io.BytesIO()
    shadow_layer().save(buffer, 'PNG', optimize=True)
    shadow = base64.b64encode(buffer.getvalue()).decode()
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">\n'
        f'  {comment}\n'
        '  <defs><clipPath id="tile"><rect x="16" y="16" width="224" height="224" rx="51.5"/></clipPath></defs>\n'
        f'  <image width="256" height="256" href="data:image/png;base64,{shadow}"/>\n'
        f'  <image x="16" y="16" width="224" height="224" clip-path="url(#tile)" href="data:image/jpeg;base64,{jpeg(art, 448)}"/>\n'
        '</svg>\n')


def tile(art, size, radius_ratio=112 / 512):
    """The artwork as a rounded square of `size` px with transparent corners."""
    image = art.convert('RGBA').resize((size, size), Image.LANCZOS)
    image.putalpha(rounded_mask(size, size * radius_ratio))
    return image


def apple_png(art, size):
    """1024 canvas, the tile 824 px wide in the middle (Apple's icon grid), scaled to `size`."""
    canvas = Image.new('RGBA', (1024, 1024), (0, 0, 0, 0))
    canvas.alpha_composite(tile(art, 824), (100, 100))
    return canvas.resize((size, size), Image.LANCZOS)


def linux_png(art, size):
    """Colloid grid at 1024 (tile 896 at 64, radius 206), scaled to `size`."""
    canvas = Image.new('RGBA', (1024, 1024), (0, 0, 0, 0))
    canvas.alpha_composite(shadow_layer().convert('RGBA').resize((1024, 1024), Image.LANCZOS))
    canvas.alpha_composite(tile(art, 896, 51.5 / 224), (64, 64))
    return canvas.resize((size, size), Image.LANCZOS)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('portrait')
    parser.add_argument('out')
    parser.add_argument('--slug', required=True)
    parser.add_argument('--app-id', required=True)
    parser.add_argument('--comment', required=True, help='text of the XML comment at the top of the SVGs')
    args = parser.parse_args()

    art = Image.open(args.portrait).convert('RGB')
    out = Path(args.out)
    comment = f'<!-- {args.comment} -->'
    out.mkdir(parents=True, exist_ok=True)
    (out / f'{args.slug}.svg').write_text(master_svg(art, comment), encoding='utf-8')
    (out / f'{args.slug}-linux.svg').write_text(linux_svg(art, comment), encoding='utf-8')
    for size in APPLE_SIZES:
        apple_png(art, size).save(out / f'{args.slug}-{size}.png', optimize=True)
    # The .ico holds the full-bleed rounded tile (Windows has no padding grid).
    tile(art, 256).save(out / f'{args.slug}.ico', sizes=[(s, s) for s in ICO_SIZES])
    for size in LINUX_SIZES:
        folder = out / 'hicolor' / f'{size}x{size}' / 'apps'
        folder.mkdir(parents=True, exist_ok=True)
        linux_png(art, size).save(folder / f'{args.app_id}.png', optimize=True)
    scalable = out / 'hicolor' / 'scalable' / 'apps'
    scalable.mkdir(parents=True, exist_ok=True)
    (scalable / f'{args.app_id}.svg').write_text(linux_svg(art, comment), encoding='utf-8')
    print(f'wrote {out}')


if __name__ == '__main__':
    main()

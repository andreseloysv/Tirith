"""macOS-style icon (Big Sur grid: an 824 px rounded square on a 1024 canvas, soft shadow) and a sheet of small sizes."""
import sys
from PIL import Image, ImageDraw, ImageFilter
def squircle_mask(size, radius_frac=0.2237, scale=4):
    big = Image.new('L', (size * scale, size * scale), 0)
    ImageDraw.Draw(big).rounded_rectangle((0, 0, size * scale - 1, size * scale - 1), radius=int(size * scale * radius_frac), fill=255)
    return big.resize((size, size), Image.LANCZOS)
def macos(art_path, out_path):
    art = Image.open(art_path).convert('RGB').resize((824, 824), Image.LANCZOS)
    canvas = Image.new('RGBA', (1024, 1024), (0, 0, 0, 0))
    m = squircle_mask(824)
    shadow = Image.new('RGBA', (1024, 1024), (0, 0, 0, 0))
    sh = Image.new('RGBA', (824, 824), (0, 0, 0, 110))
    shadow.paste(sh, (100, 112), m)
    shadow = shadow.filter(ImageFilter.GaussianBlur(14))
    canvas.alpha_composite(shadow)
    tile = art.convert('RGBA')
    tile.putalpha(m)
    canvas.alpha_composite(tile, (100, 100))
    canvas.save(out_path)
    return canvas
def sheet(icons, out_path):
    sizes = [256, 128, 64, 32]
    rows = []
    for bg in ((236, 236, 240), (30, 30, 34)):
        row = Image.new('RGB', (len(icons) * (sum(sizes) + 60) + 20, 290), bg)
        x = 20
        for ic in icons:
            for s in sizes:
                im = ic.resize((s, s), Image.LANCZOS)
                row.paste(im, (x, 145 - s // 2), im)
                x += s + 12
            x += 20
        rows.append(row)
    out = Image.new('RGB', (rows[0].width, 580))
    out.paste(rows[0], (0, 0)); out.paste(rows[1], (0, 290))
    out.save(out_path)
if __name__ == '__main__':
    icons = [macos(p, p.rsplit('.', 1)[0] + '-macos.png') for p in sys.argv[2:]]
    sheet(icons, sys.argv[1])
    print('ok')

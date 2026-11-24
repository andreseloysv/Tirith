"""Every app icon size from one 1024 px square of art: argv[1], else option D (icon-d-1024.png, the owner's
lumaform hero, rendered by hero3d/ in Blender 5.2, from the repo root:
  Blender -b --factory-startup -P app/src-tauri/icons/source/hero3d/build.py -- --size=1024 --samples=64
    --hero=app/src-tauri/icons/source/hero3d/mesh.glb --herohue=0.463 --heroemit=1.5 --herosat=1.08
    --out=app/src-tauri/icons/source/icon-d-1024.png).
Since 2026-11-23 the icon is icon-bot-1024.png, the owner's pick: a bot drawn as the faces are (one flat lime
body #8fd14f in cel shading, the Calm eyes of their expression sheet in ink) filling most of a dark stage,
with a soft glow and a light rim. From the repo root:
  python3 app/src-tauri/icons/source/make-icons-d.py app/src-tauri/icons/source/icon-bot-1024.png
Tauri PNGs and the .ico are full-bleed rounded squares; icon.png and the .icns follow the
macOS grid (824 px tile on a 1024 canvas with a soft shadow); the browser icons and the site's logo come
from make-web-icons.py, run at the end."""
import os, subprocess, sys, tempfile
from PIL import Image, ImageDraw, ImageFilter

ICONS = 'app/src-tauri/icons'
SRC = f'{ICONS}/source'
ART = sys.argv[1] if len(sys.argv) > 1 else f'{SRC}/icon-d-1024.png'
art = Image.open(ART).convert('RGB')

def mask(size, frac=0.2237, k=4):
    m = Image.new('L', (size * k, size * k), 0)
    ImageDraw.Draw(m).rounded_rectangle((0, 0, size * k - 1, size * k - 1), radius=int(size * k * frac), fill=255)
    return m.resize((size, size), Image.LANCZOS)

def tile(size):
    t = art.resize((size, size), Image.LANCZOS).convert('RGBA')
    t.putalpha(mask(size))
    return t

def macos():
    canvas = Image.new('RGBA', (1024, 1024), (0, 0, 0, 0))
    sh = Image.new('RGBA', (1024, 1024), (0, 0, 0, 0))
    sh.paste(Image.new('RGBA', (824, 824), (0, 0, 0, 110)), (100, 112), mask(824))
    canvas.alpha_composite(sh.filter(ImageFilter.GaussianBlur(14)))
    canvas.alpha_composite(tile(824), (100, 100))
    return canvas

mac = macos()
art.save(f'{SRC}/tirith-glow-1024.png')
mac.save(f'{SRC}/tirith-glow-macos-1024.png')
mac.resize((512, 512), Image.LANCZOS).save(f'{ICONS}/icon.png')
for name, n in {'32x32': 32, '64x64': 64, '128x128': 128, '128x128@2x': 256, 'StoreLogo': 50,
                **{f'Square{s}x{s}Logo': s for s in (30, 44, 71, 89, 107, 142, 150, 284, 310)}}.items():
    tile(n).save(f'{ICONS}/{name}.png')
tile(256).save(f'{ICONS}/icon.ico', sizes=[(s, s) for s in (16, 24, 32, 48, 64, 256)])

with tempfile.TemporaryDirectory() as d:
    iconset = f'{d}/icon.iconset'
    os.mkdir(iconset)
    for s in (16, 32, 128, 256, 512):
        mac.resize((s, s), Image.LANCZOS).save(f'{iconset}/icon_{s}x{s}.png')
        mac.resize((s * 2, s * 2), Image.LANCZOS).save(f'{iconset}/icon_{s}x{s}@2x.png')
    subprocess.run(['iconutil', '-c', 'icns', iconset, '-o', f'{ICONS}/icon.icns'], check=True)

subprocess.run([sys.executable, f'{SRC}/make-web-icons.py', ART], check=True)

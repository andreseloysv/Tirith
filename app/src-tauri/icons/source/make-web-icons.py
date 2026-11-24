"""The browser icons of the page (web/) and of the site (site/) from one 1024 px square of art: argv[1], else
icon-bot-1024.png (the app icon since 2026-11-23). From the repo root:
  python3 app/src-tauri/icons/source/make-web-icons.py [art]
A tab shows the app's own rounded tile (as the Tauri PNGs and the .ico are), transparent around it, drawn at
each size rather than shrunk by the browser. The names are new on purpose: browsers kept older pictures under
the old name, img/tirith-logo-64.png, for a long time. iOS rounds its home-screen icon itself, so
apple-touch-icon.png is the art square and opaque. make-icons-d.py runs this after the app's icons."""
import sys
from PIL import Image, ImageDraw

SRC = 'app/src-tauri/icons/source'
art = Image.open(sys.argv[1] if len(sys.argv) > 1 else f'{SRC}/icon-bot-1024.png').convert('RGB')


def tile(size, frac=0.2237, k=4):
    """The art as the app's rounded tile, `size` px square, its corners antialiased."""
    m = Image.new('L', (size * k, size * k), 0)
    ImageDraw.Draw(m).rounded_rectangle((0, 0, size * k - 1, size * k - 1), radius=int(size * k * frac), fill=255)
    t = art.resize((size, size), Image.LANCZOS).convert('RGBA')
    t.putalpha(m.resize((size, size), Image.LANCZOS))
    return t


ico = [tile(s) for s in (16, 32, 48)]
ico[-1].save('site/favicon.ico', sizes=[(16, 16), (32, 32), (48, 48)], append_images=ico[:-1])
for root in ('site', 'web'):
    tile(96).save(f'{root}/img/favicon-96.png', optimize=True)
tile(32).save('web/img/favicon-32.png', optimize=True)
art.resize((180, 180), Image.LANCZOS).save('site/img/apple-touch-icon.png', optimize=True)
art.save('site/img/tirith-logo.png')

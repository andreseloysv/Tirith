"""Turn the raw Blender renders into the files the landing uses. Needs Pillow and ffmpeg. Usage: publish.py RAW_DIR

RAW_DIR holds what these wrote (Blender -b --factory-startup -P site/3d/<scene>.py -- ...):
  logo.py -- RAW/logo.png 2048 samples=512
  logo.py -- RAW/unused.png 512 RAW/spin 60 samples=64 limit=2048
  scene_agents.py -- RAW/agents.png 1600 1000 samples=512
  scene_gate.py -- RAW/gate.png 1600 1000 samples=512
  social.py -- RAW/social.png samples=512
Renders are kept to MAX_BYTES each, WebP for browsers and an indexed PNG as the fallback.
"""
import glob, os, subprocess, sys, tempfile
from PIL import Image
import numpy as np

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
IMG = os.path.join(ROOT, "site", "img")
MAX_BYTES = 400_000
# the .logo3d tile in site/style.css: the spin video has no alpha, so its frames carry this same gradient
TILE = ((0.5, 0.42), ((0.0, (42, 56, 88)), (0.52, (20, 27, 45)), (1.0, (10, 14, 24))))


def scaled(im, size):
    """Resize through premultiplied alpha, or the edges of a transparent image go dark."""
    return im.convert("RGBa").resize(size, Image.LANCZOS).convert("RGBA")


def tile(size):
    (cx, cy), stops = TILE
    y, x = np.mgrid[0:size, 0:size] / size
    far = max(np.hypot(a - cx, b - cy) for a in (0, 1) for b in (0, 1))
    t = np.hypot(x - cx, y - cy) / far
    at, colors = zip(*stops)
    rgb = np.stack([np.interp(t, at, [c[i] for c in colors]) for i in range(3)], axis=-1)
    return Image.fromarray(np.uint8(rgb.round()), "RGB").convert("RGBA")


def under_limit(save, path):
    """save(setting) with each setting in turn until the file fits."""
    for setting in save.settings:
        save(setting)
        if os.path.getsize(path) <= MAX_BYTES:
            return
    sys.exit("%s is %d bytes, over %d" % (path, os.path.getsize(path), MAX_BYTES))


def webp_and_png(src, name):
    im = Image.open(src).convert("RGB")
    w, h = im.size
    webp = os.path.join(IMG, name + ".webp")
    png = os.path.join(IMG, name + ".png")

    def save_webp(q):
        im.save(webp, "WEBP", quality=q, method=6)
    save_webp.settings = (88, 84, 80, 75, 70, 65, 60)
    under_limit(save_webp, webp)

    def save_png(width):
        small = im.resize((width, round(width * h / w)), Image.LANCZOS)
        small.quantize(256, method=Image.MEDIANCUT, dither=Image.FLOYDSTEINBERG).save(png, optimize=True)
    save_png.settings = (1000, 920, 840, 760)
    under_limit(save_png, png)
    print(name, os.path.getsize(webp), os.path.getsize(png))


def logo(src):
    im = Image.open(src).convert("RGBA")
    im.save(os.path.join(IMG, "logo-3d@2x.png"), optimize=True)
    scaled(im, (1024, 1024)).save(os.path.join(IMG, "logo-3d.png"), optimize=True)
    scaled(im, (64, 64)).save(os.path.join(IMG, "logo-3d-64.png"), optimize=True)
    scaled(im, (440, 440)).save(os.path.join(IMG, "logo-3d.webp"), "WEBP", quality=88, alpha_quality=100, method=6)


def spin(frames_dir):
    frames = sorted(glob.glob(os.path.join(frames_dir, "f_*.png")))
    if not frames:
        return
    bg = None
    with tempfile.TemporaryDirectory() as tmp:
        for i, f in enumerate(frames):
            im = Image.open(f).convert("RGBA")
            bg = bg or tile(im.size[0])
            over = bg.copy()
            over.alpha_composite(im)
            over.convert("RGB").save(os.path.join(tmp, "o_%04d.png" % i))
        subprocess.run(["ffmpeg", "-y", "-v", "error", "-framerate", "20", "-i", os.path.join(tmp, "o_%04d.png"),
                        "-c:v", "libx264", "-crf", "28", "-preset", "veryslow", "-pix_fmt", "yuv420p", "-movflags", "+faststart",
                        os.path.join(IMG, "logo-3d.mp4")], check=True)
    subprocess.run(["ffmpeg", "-y", "-v", "error", "-framerate", "20", "-pattern_type", "glob", "-i", os.path.join(frames_dir, "f_*.png"),
                    "-c:v", "libvpx-vp9", "-crf", "44", "-b:v", "0", "-pix_fmt", "yuva420p", os.path.join(IMG, "logo-3d.webm")], check=True)


def social(src):
    im = Image.open(src).convert("RGB")
    indexed = im.quantize(256, method=Image.MEDIANCUT, dither=Image.FLOYDSTEINBERG)
    for path in (os.path.join(ROOT, "docs", "social-preview-3d.png"), os.path.join(IMG, "social-preview-3d.png")):
        indexed.save(path, optimize=True)
        print(path, os.path.getsize(path))


raw = sys.argv[1]
logo(os.path.join(raw, "logo.png"))
spin(os.path.join(raw, "spin"))
webp_and_png(os.path.join(raw, "agents.png"), "render-agents")
webp_and_png(os.path.join(raw, "gate.png"), "render-gate")
social(os.path.join(raw, "social.png"))

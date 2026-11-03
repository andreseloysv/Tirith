"""Generate the Tirith app icons from the stone tower (site/img/logo-3d@2x.png). Needs Pillow. Usage: make_icon.py [OUT_DIR]

Large sizes show the whole tower on a dusk tile. From 64 px down the crop tightens on the battlements and the
window, the part that still reads when a pixel is a whole stone. Every size at or below 64 px is checked in code:
the stone tower must keep a luminance contrast of at least MIN_CONTRAST (WCAG ratio) against the tile around it,
otherwise the script stops. Without OUT_DIR it writes app/src-tauri/icons.
"""
import os
import sys
from PIL import Image, ImageDraw, ImageEnhance

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
STONE_TOWER = os.path.join(ROOT, "site", "img", "logo-3d@2x.png")
PNGS = {
    "32x32.png": 32, "64x64.png": 64, "128x128.png": 128, "128x128@2x.png": 256, "icon.png": 512,
    "Square30x30Logo.png": 30, "Square44x44Logo.png": 44, "Square71x71Logo.png": 71, "Square89x89Logo.png": 89,
    "Square107x107Logo.png": 107, "Square142x142Logo.png": 142, "Square150x150Logo.png": 150,
    "Square284x284Logo.png": 284, "Square310x310Logo.png": 310, "StoreLogo.png": 50,
}
ICNS = (16, 32, 64, 128, 256, 512, 1024)
ICO = (16, 24, 32, 48, 64, 256)
SMALL = 64  # this size and below use the tight crop
# in the 2048 px logo: the whole mark, and the tower's top down to below the window
WHOLE = (64, 90, 1984, 2010)
TIGHT = (470, 120, 1570, 1220)
MIN_CONTRAST = 3.0  # WCAG ratio for large graphics (3:1) between the stone tower and the tile
GLOW, DUSK = (64, 84, 132), (9, 13, 24)


def tile(s):
    """A rounded dusk square, lighter behind the tower so its silhouette cuts out."""
    bg = Image.radial_gradient("L").resize((s, s), Image.BICUBIC)
    bg = Image.composite(Image.new("RGB", (s, s), DUSK), Image.new("RGB", (s, s), GLOW), bg)
    mask = Image.new("L", (s * 4, s * 4), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, s * 4 - 1, s * 4 - 1), radius=s * 4 * 0.2237, fill=255)
    out = bg.convert("RGBA")
    out.putalpha(mask.resize((s, s), Image.LANCZOS))
    return out


def _luminance(rgb):
    """Relative luminance (WCAG) of an sRGB colour with channels 0..255."""
    lin = [(c / 255 / 12.92) if c / 255 <= 0.03928 else ((c / 255 + 0.055) / 1.055) ** 2.4 for c in rgb[:3]]
    return 0.2126 * lin[0] + 0.7152 * lin[1] + 0.0722 * lin[2]


def stone_tower_contrast(art, bg):
    """WCAG contrast ratio between the mean luminance of the tower's pixels (art alpha) and the tile behind it."""
    alpha = art.getchannel("A")
    tower, back = [], []
    for a, (r, g, b, _), (br, bgc, bb, _) in zip(*(getattr(im, "get_flattened_data", im.getdata)() for im in (alpha, art, bg))):
        (tower if a > 200 else back if a < 20 else []).append(_luminance((r, g, b)) if a > 200 else _luminance((br, bgc, bb)))
    hi, lo = sorted((sum(tower) / len(tower), sum(back) / len(back)), reverse=True)
    return (hi + 0.05) / (lo + 0.05)


def icon(logo, s):
    """The icon at s px, scaled from the logo through premultiplied alpha so the edges stay clean."""
    small = s <= SMALL
    crop = logo.crop(TIGHT if small else WHOLE)
    art = crop.convert("RGBa").resize((s, s), Image.LANCZOS).convert("RGBA")
    if small:  # at this size stone mid-tones merge with the tile, so lift them
        rgb, alpha = art.convert("RGB"), art.getchannel("A")
        rgb = ImageEnhance.Brightness(ImageEnhance.Contrast(rgb).enhance(1.2)).enhance(1.25)
        art = rgb.convert("RGBA")
        art.putalpha(alpha)
    out = tile(s)
    if small:
        ratio = stone_tower_contrast(art, out)
        print(f"stone tower contrast at {s}px: {ratio:.2f}:1")
        assert ratio >= MIN_CONTRAST, f"stone tower at {s}px has contrast {ratio:.2f}:1, below {MIN_CONTRAST}:1"
    mask = out.getchannel("A")
    out.alpha_composite(art)
    out.putalpha(mask)
    return out


def main():
    out_dir = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "app", "src-tauri", "icons")
    logo = Image.open(STONE_TOWER).convert("RGBA")
    for name, s in PNGS.items():
        icon(logo, s).save(os.path.join(out_dir, name), optimize=True)
    icns = [icon(logo, s) for s in ICNS]
    icns[-1].save(os.path.join(out_dir, "icon.icns"), append_images=icns[:-1])
    ico = [icon(logo, s) for s in ICO]
    ico[-1].save(os.path.join(out_dir, "icon.ico"), sizes=[(s, s) for s in ICO], append_images=ico[:-1])


if __name__ == "__main__":
    main()

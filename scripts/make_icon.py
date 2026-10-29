"""Generate the Tirith source icon (1024x1024 PNG). Usage: make_icon.py OUT.png"""
import sys
from PIL import Image, ImageDraw

S = 1024
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
d.rounded_rectangle((0, 0, S - 1, S - 1), radius=230, fill=(18, 24, 38, 255))
d.rounded_rectangle((40, 40, S - 41, S - 41), radius=200, outline=(96, 165, 250, 255), width=14)
# a tower: crenellated top, a lit window
c = (226, 232, 240, 255)
d.rectangle((392, 330, 632, 800), fill=c)
for x in (372, 470, 568):
    d.rectangle((x, 250, x + 84, 340), fill=c)
d.rectangle((372, 330, 652, 370), fill=c)
d.rounded_rectangle((472, 470, 552, 600), radius=40, fill=(96, 165, 250, 255))
img.save(sys.argv[1])

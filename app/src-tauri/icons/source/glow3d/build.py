# Tirith's app icon in 3D: our bot shapes as soft, glowing, translucent bodies in deep space, one big bot in front
# and the others flying out from behind it with sparkle trails; depth of field and bloom.
# Blender -b --factory-startup -P build.py -- --out=out/r.png --size=768 --samples=48
# The steps live in the icon3d package next to this file; icon3d/main.py runs them in order.
import os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from icon3d import main

main.run()

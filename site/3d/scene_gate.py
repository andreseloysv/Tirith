"""A gate of glowing rules: pillars with a light barrier between them. Usage: -- OUT.png W H"""
import sys, math
sys.path.insert(0, __file__.rsplit("/", 1)[0])
from common import *

out, W, H = sys.argv[-3], int(sys.argv[-2]), int(sys.argv[-1])
reset()
world((0.9, 0.94, 1), 0.7)
body = mat("body", PEARL, metallic=0.55, rough=0.22)
beam = mat("beam", LIGHT_BLUE, emit=9)
for dx in (-2.4, 2.4):
    box("pillar", (dx, 0, 1.7), (0.8, 0.8, 3.4), body, 0.07)
    box("cap", (dx, 0, 3.5), (1.1, 1.1, 0.22), body, 0.05)
    box("foot", (dx, 0, 0.12), (1.3, 1.3, 0.24), body, 0.05)
box("lintel", (0, 0, 3.75), (6.4, 0.9, 0.5), body, 0.08)
for i in range(7):
    z = 0.7 + i * 0.4
    box("rule%d" % i, (0, 0, z), (4.0, 0.07, 0.07 + (0.03 if i % 3 == 0 else 0)), mat("r%d" % i, BLUE if i % 2 else LIGHT_BLUE, emit=3), 0.02)
# a small agent cube meeting the gate
box("agent", (0.4, -1.8, 0.5), (0.8, 0.8, 0.8), mat("ag", (0.7, 0.78, 0.9, 1), rough=0.25), 0.1, rot=(0, 0, 0.5))
box("agent_core", (0.4, -1.8, 0.5), (0.32, 0.32, 0.32), mat("agc", BLUE, emit=3), 0.04, rot=(0, 0, 0.5))
shadow_floor(0)
look((6.0, -14.0, 4.0), (0, -0.8, 1.9), 55)
area((-5, -7, 8), (0, 0, 1.8), 700, 6)
area((7, -3, 4), (0, 0, 1.8), 300, 5, (0.7, 0.85, 1))
setup(W, H, 128, True, out)
bpy.ops.render.render(write_still=True)

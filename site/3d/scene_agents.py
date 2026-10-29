"""The tower watching floating agent cubes joined by light lines. Usage: -- OUT.png W H"""
import sys, math, random
sys.path.insert(0, __file__.rsplit("/", 1)[0])
from common import *

out, W, H = sys.argv[-3], int(sys.argv[-2]), int(sys.argv[-1])
reset()
world((0.9, 0.94, 1), 0.7)
random.seed(7)
tower("t_", (0, 0, 0), 1.0)
line = mat("line", LIGHT_BLUE, emit=2.5)
glass = mat("glass", (0.8, 0.9, 1, 1), rough=0.05, trans=1.0, ior=1.3)
cube_m = mat("cube", (0.7, 0.78, 0.9, 1), metallic=0.1, rough=0.25)
pts = [(-3.2, -1.0, 1.6), (3.0, -1.4, 2.2), (-2.2, 2.2, 3.0), (2.8, 2.0, 1.2), (0.2, -3.0, 3.4), (-4.6, 0.6, 3.2), (4.8, 0.6, 3.0)]
for i, p in enumerate(pts):
    s = 0.55 + 0.15 * (i % 3)
    c = box("agent%d" % i, p, (s, s, s), cube_m, 0.07, rot=(random.random(), random.random(), random.random()))
    box("core%d" % i, p, (s * 0.4, s * 0.4, s * 0.4), mat("core%d" % i, BLUE if i % 2 else LIGHT_BLUE, emit=3), 0.03, rot=c.rotation_euler)
    tube("beam%d" % i, (0, 0, 1.05), p, 0.012, line)
    sphere("node%d" % i, p, 0.0, line) if False else None
shadow_floor(0)
look((0, -13, 4.5), (0, 0, 2.0), 50)
area((-5, -6, 8), (0, 0, 1.5), 700, 6)
area((6, -4, 4), (0, 0, 1.5), 300, 5, (0.7, 0.85, 1))
setup(W, H, 128, True, out)
bpy.ops.render.render(write_still=True)

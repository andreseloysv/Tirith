"""A glass plan board: rows of items, the first ones lit. Usage: -- OUT.png W H"""
import sys, math
sys.path.insert(0, __file__.rsplit("/", 1)[0])
from common import *

out, W, H = sys.argv[-3], int(sys.argv[-2]), int(sys.argv[-1])
reset()
world((0.9, 0.94, 1), 0.7)
glass = mat("glass", (0.85, 0.92, 1, 1), rough=0.04, trans=1.0, ior=1.35)
frame = mat("frame", PEARL, metallic=0.6, rough=0.2)
off = mat("off", (0.6, 0.66, 0.76, 1), rough=0.3)
board = bpy.data.objects.new("board", None)
bpy.context.collection.objects.link(board)
parts = [box("panel", (0, 0, 0), (6.4, 0.16, 4.4), glass, 0.12)]
for i, (dx, dz, sx, sz) in enumerate(((0, 2.25, 6.6, 0.12), (0, -2.25, 6.6, 0.12), (-3.3, 0, 0.12, 4.6), (3.3, 0, 0.12, 4.6))):
    parts.append(box("fr%d" % i, (dx, 0, dz), (sx, 0.3, sz), frame, 0.04))
lit = [True, True, True, False, False]
for r in range(5):
    z = 1.5 - r * 0.75
    parts.append(box("row%d" % r, (0.3, -0.12, z), (4.9, 0.08, 0.5), off if not lit[r] else mat("ml%d" % r, (0.75, 0.82, 0.92, 1), rough=0.2), 0.05))
    if lit[r]:
        parts.append(box("tick%d" % r, (-2.55, -0.18, z), (0.42, 0.1, 0.42), mat("tk%d" % r, BLUE, emit=3), 0.1))
        parts.append(box("bar%d" % r, (0.3 - (1 - (0.55 + 0.15 * r)) * 2.4, -0.18, z), (4.9 * (0.55 + 0.15 * r), 0.06, 0.18), mat("bl%d" % r, LIGHT_BLUE, emit=2.5), 0.04))
    else:
        parts.append(box("tick%d" % r, (-2.55, -0.18, z), (0.42, 0.1, 0.42), mat("tk%d" % r, (0.7, 0.76, 0.86, 1), rough=0.3), 0.1))
for o in parts:
    o.parent = board
board.rotation_euler = (0, 0, math.radians(-24))
board.location = (0, 0, 2.6)
shadow_floor(0.2)
look((4.5, -15, 4.5), (0, 0, 2.5), 55)
area((-5, -7, 9), (0, 0, 2.5), 800, 6)
area((7, -2, 4), (0, 0, 2.5), 300, 5, (0.7, 0.85, 1))
area((0, 6, 5), (0, 0, 2.5), 400, 6, (0.8, 0.9, 1))
setup(W, H, 160, True, out)
bpy.ops.render.render(write_still=True)

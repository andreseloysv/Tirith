"""Social preview 1280x640: the 3D logo and the name. Usage: -- OUT.png"""
import sys, math
sys.path.insert(0, __file__.rsplit("/", 1)[0])
from common import *

out = sys.argv[-1]
reset()
world((0.95, 0.97, 1), 1.0)
# opaque light backdrop
bpy.ops.mesh.primitive_plane_add(size=60, location=(0, 8, 0))
bg = bpy.context.object
bg.rotation_euler = (math.pi / 2, 0, 0)
bg.data.materials.append(mat("bg", (0.9, 0.94, 0.98, 1), rough=1))
bpy.ops.mesh.primitive_plane_add(size=60, location=(0, 0, 0))
bpy.context.object.data.materials.append(mat("floor", (0.9, 0.94, 0.98, 1), rough=0.6))
box("plinth", (-4.2, 0, -0.18), (3.0, 3.0, 0.36), mat("plinth", INK, metallic=0.3, rough=0.25), 0.3)
box("rim", (-4.2, 0, -0.02), (3.12, 3.12, 0.06), mat("rim", BLUE, metallic=0.8, rough=0.2), 0.03)
tower("t_", (-4.2, 0, 0), 0.95)
sphere("orb", (-4.2, 0, 2.95), 0.12, mat("orb", LIGHT_BLUE, emit=3))
bpy.ops.object.text_add(location=(-2.3, -0.9, 0.7), rotation=(math.pi / 2, 0, 0))
t = bpy.context.object
t.data.body = "Tirith"
t.data.size = 2.0
t.data.extrude = 0.15
t.data.bevel_depth = 0.02
t.data.materials.append(mat("txt", INK, metallic=0.4, rough=0.25))
look((-1.6, -13.5, 2.3), (-1.6, 0, 1.4), 50)
area((-4, -7, 8), (-1, 0, 1), 700, 6)
area((6, -4, 4), (0, 0, 1), 300, 5, (0.7, 0.85, 1))
setup(1280, 640, 128, False, out)
bpy.ops.render.render(write_still=True)

"""Tirith 3D logo. Usage: Blender -b -P logo.py -- OUT.png SIZE [turntable_dir FRAMES]"""
import sys, math
sys.path.insert(0, __file__.rsplit("/", 1)[0])
from common import *

args = sys.argv[sys.argv.index("--") + 1:]
out, size = args[0], int(args[1])
turn_dir = args[2] if len(args) > 2 else None
frames = int(args[3]) if len(args) > 3 else 0  # optional 5th arg: only this frame

reset()
world((0.85, 0.9, 1), 0.25)
# rounded dark plinth like the app icon, with a blue rim
plinth = box("plinth", (0, 0, -0.18), (3.0, 3.0, 0.36), mat("plinth", INK, metallic=0.3, rough=0.25), 0.3)
rim = box("rim", (0, 0, -0.02), (3.12, 3.12, 0.06), mat("rim", BLUE, metallic=0.8, rough=0.2), 0.03)
pivot = bpy.data.objects.new("pivot", None)
bpy.context.collection.objects.link(pivot)
parts = tower("t_", (0, 0, 0), 0.95)
for o in [plinth, rim] + parts:
    o.parent = pivot
# floating glow orb above, a nod to the watching eye
orb = sphere("orb", (0, 0, 2.95), 0.12, mat("orb", LIGHT_BLUE, emit=3))
orb.parent = pivot
shadow_floor(-0.36)
look((4.6, -6.0, 3.6), (0, 0, 1.0), 62)
area((-4, -5, 7), (0, 0, 1), 350, 5)
area((6, -2, 3), (0, 0, 1), 160, 4, (0.7, 0.85, 1))
area((0, 5, 4), (0, 0, 1), 120, 4, (0.8, 0.9, 1))
setup(size, size, 128, True, out)
if turn_dir:
    s = bpy.context.scene
    s.cycles.samples = 48
    s.frame_start, s.frame_end = 1, frames
    pivot.rotation_euler = (0, 0, 0)
    pivot.keyframe_insert("rotation_euler", frame=1)
    pivot.rotation_euler = (0, 0, math.tau * (frames - 1) / frames)
    pivot.keyframe_insert("rotation_euler", frame=frames)
    for fc in pivot.animation_data.action.layers[0].strips[0].channelbags[0].fcurves:
        for kp in fc.keyframe_points:
            kp.interpolation = "LINEAR"
    s.render.filepath = turn_dir + "/f_"
    if len(args) > 4:  # re-render a single frame
        s.frame_start = s.frame_end = int(args[4])
    bpy.ops.render.render(animation=True)
else:
    bpy.ops.render.render(write_still=True)

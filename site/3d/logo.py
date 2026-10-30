"""Tirith 3D logo. Usage: Blender -b -P logo.py -- OUT.png SIZE [turntable_dir FRAMES [FRAME]] [samples=N] [stone=PACK] [sky=PACK] [limit=PX]

Final still: OUT.png 2048 samples=512. Spin: OUT.png 512 DIR 60 samples=64 limit=2048, then composite the
frames over the dark .logo3d background at 20 fps. Scale the still with premultiplied alpha, or the edges go dark.
"""
import sys, math
sys.path.insert(0, __file__.rsplit("/", 1)[0])
from common import *

argv = sys.argv[sys.argv.index("--") + 1:]
opts = dict(a.split("=", 1) for a in argv if "=" in a)
args = [a for a in argv if "=" not in a]
out, size = args[0], int(args[1])
turn_dir = args[2] if len(args) > 2 else None
frames = int(args[3]) if len(args) > 3 else 0  # optional 5th arg: only this frame

reset()
sky(opts.get("sky", SKY), float(opts.get("fill", 0.14)), float(opts.get("rot", 0)), (0.62, 0.74, 1.0))
pivot = bpy.data.objects.new("pivot", None)
bpy.context.collection.objects.link(pivot)
for o in stone_tower("t_", stone=opts.get("stone", STONE)):
    o.parent = pivot
look((0, -14.5, 3.75), (0, 0, 1.45), 92)
dusk_rig(key_energy=float(opts.get("key", 9)), rim_energy=float(opts.get("rimE", 18)))
setup(size, size, int(opts.get("samples", 512)), True, out)
if "limit" in opts:
    bpy.context.scene.cycles.texture_limit_render = opts["limit"]
if turn_dir:
    s = bpy.context.scene
    s.cycles.samples = int(opts.get("samples", 48))
    s.render.use_persistent_data = True
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

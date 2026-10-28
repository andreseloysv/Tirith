"""Prepares a generated character GLB for the citadel (Blender 5.x, headless).

Decimates to a triangle budget, optionally removes a display base, grounds the feet at the origin,
scales to a target height, downsizes textures and exports a GLB.

usage: blender -b --python prep_character.py -- <in.glb> <out.glb> [--tris 25000] [--height 1.3]
       [--cut-base 0.0]   (fraction of the height, from the bottom, to cut away; 0 = keep)
       [--base-ring R F]  (also cut what is farther than R from the centre below fraction F: a display base)
"""
import argparse
import sys

import bpy
import mathutils

a = argparse.ArgumentParser()
a.add_argument("src"); a.add_argument("dst")
a.add_argument("--tris", type=int, default=25000)
a.add_argument("--height", type=float, default=1.3)
a.add_argument("--cut-base", type=float, default=0.0)
a.add_argument("--tex", type=int, default=2048)
a.add_argument("--base-ring", type=float, nargs=2, default=None, metavar=("RADIUS", "FRACTION"),
               help="also cut vertices farther than RADIUS from the centre below FRACTION of the height")
args = a.parse_args(sys.argv[sys.argv.index("--") + 1:])

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=args.src)
ob = [o for o in bpy.context.scene.objects if o.type == "MESH"][0]
bpy.context.view_layer.objects.active = ob
ob.select_set(True)
bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)

if args.cut_base > 0:
    zs = [v.co.z for v in ob.data.vertices]
    zmin, zmax = min(zs), max(zs)
    cut = zmin + args.cut_base * (zmax - zmin)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="DESELECT")
    bpy.ops.object.mode_set(mode="OBJECT")
    ring = None
    if args.base_ring:
        ring = (args.base_ring[0], zmin + args.base_ring[1] * (zmax - zmin))
    for v in ob.data.vertices:
        r = (v.co.x ** 2 + v.co.y ** 2) ** 0.5
        v.select = v.co.z < cut or (ring is not None and v.co.z < ring[1] and r > ring[0])
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.delete(type="VERT")
    bpy.ops.object.mode_set(mode="OBJECT")

tris = sum(len(p.vertices) - 2 for p in ob.data.polygons)
if tris > args.tris:
    m = ob.modifiers.new("decimate", "DECIMATE")
    m.ratio = args.tris / tris
    bpy.ops.object.modifier_apply(modifier="decimate")

# Ground: feet at z = 0, centred on x/y, scaled to the target height.
bb = [ob.matrix_world @ mathutils.Vector(c) for c in ob.bound_box]
zmin = min(v.z for v in bb); zmax = max(v.z for v in bb)
cx = sum(v.x for v in bb) / 8; cy = sum(v.y for v in bb) / 8
ob.location -= mathutils.Vector((cx, cy, zmin))
bpy.ops.object.transform_apply(location=True)
s = args.height / (zmax - zmin)
ob.scale = (s, s, s)
bpy.ops.object.transform_apply(scale=True)

for img in bpy.data.images:
    if img.size[0] > args.tex:
        img.scale(args.tex, args.tex * img.size[1] // img.size[0])

bpy.ops.export_scene.gltf(filepath=args.dst, export_format="GLB", use_selection=False,
                          export_image_format="JPEG", export_jpeg_quality=90)
tris = sum(len(p.vertices) - 2 for p in ob.data.polygons)
print(f"PREPARED {args.dst} tris={tris} height={args.height}")

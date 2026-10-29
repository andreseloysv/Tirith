"""Shared helpers for the Tirith Blender scenes. Run via: Blender -b -P <scene>.py -- <args>"""
import math
import bpy
from mathutils import Vector

BLUE = (0.05, 0.22, 0.9, 1)
LIGHT_BLUE = (0.12, 0.45, 1.0, 1)
INK = (0.07, 0.094, 0.15, 1)
PEARL = (0.62, 0.7, 0.82, 1)


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    return bpy.context.scene


def mat(name, color, metallic=0.0, rough=0.3, emit=0.0, alpha=1.0, trans=0.0, ior=1.45):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    b = m.node_tree.nodes["Principled BSDF"]
    b.inputs["Base Color"].default_value = color
    b.inputs["Metallic"].default_value = metallic
    b.inputs["Roughness"].default_value = rough
    b.inputs["IOR"].default_value = ior
    if trans:
        b.inputs["Transmission Weight"].default_value = trans
    if emit:
        b.inputs["Emission Color"].default_value = color
        b.inputs["Emission Strength"].default_value = emit
    return m


def box(name, loc, size, m, bevel=0.03, rot=(0, 0, 0)):
    bpy.ops.mesh.primitive_cube_add(location=loc, rotation=rot)
    o = bpy.context.object
    o.name = name
    o.scale = (size[0] / 2, size[1] / 2, size[2] / 2)
    bpy.ops.object.transform_apply(scale=True)
    if bevel:
        bv = o.modifiers.new("bevel", "BEVEL")
        bv.width = bevel
        bv.segments = 4
        bv.limit_method = "ANGLE"
    o.data.materials.append(m)
    bpy.ops.object.shade_smooth()
    return o


def sphere(name, loc, r, m):
    bpy.ops.mesh.primitive_uv_sphere_add(location=loc, radius=r, segments=48, ring_count=24)
    o = bpy.context.object
    o.name = name
    o.data.materials.append(m)
    bpy.ops.object.shade_smooth()
    return o


def tube(name, a, b, r, m):
    a, b = Vector(a), Vector(b)
    d = b - a
    bpy.ops.mesh.primitive_cylinder_add(radius=r, depth=d.length, location=(a + b) / 2, vertices=16)
    o = bpy.context.object
    o.name = name
    o.rotation_mode = "QUATERNION"
    o.rotation_quaternion = d.to_track_quat("Z", "Y")
    o.data.materials.append(m)
    bpy.ops.object.shade_smooth()
    return o


def tower(prefix, loc=(0, 0, 0), s=1.0, body=None, glow=None):
    """The Tirith watchtower mark. Height about 2.4*s, centred on loc (base at loc.z)."""
    body = body or mat("tower_body", PEARL, metallic=0.55, rough=0.22)
    glow = glow or mat("tower_glow", LIGHT_BLUE, emit=3.0, rough=0.2)
    x, y, z = loc
    parts = [
        box(prefix + "base", (x, y, z + 0.1 * s), (1.5 * s, 1.5 * s, 0.2 * s), body, 0.05 * s),
        box(prefix + "shaft", (x, y, z + 0.95 * s), (1.0 * s, 1.0 * s, 1.5 * s), body, 0.05 * s),
        box(prefix + "ledge", (x, y, z + 1.78 * s), (1.35 * s, 1.35 * s, 0.16 * s), body, 0.04 * s),
    ]
    for i, (dx, dy) in enumerate(((-0.46, -0.46), (0.46, -0.46), (-0.46, 0.46), (0.46, 0.46), (0, -0.46), (0, 0.46))):
        parts.append(box(prefix + "merlon%d" % i, (x + dx * s, y + dy * s, z + 2.07 * s), (0.34 * s, 0.34 * s, 0.42 * s), body, 0.035 * s))
    parts.append(box(prefix + "window", (x, y - 0.5 * s, z + 1.05 * s), (0.34 * s, 0.06 * s, 0.56 * s), glow, 0.08 * s))
    return parts


def look(cam_loc, target, lens=70):
    bpy.ops.object.camera_add(location=cam_loc)
    c = bpy.context.object
    c.data.lens = lens
    d = Vector(target) - Vector(cam_loc)
    c.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.camera = c
    return c


def area(loc, target, energy, size, color=(1, 1, 1)):
    bpy.ops.object.light_add(type="AREA", location=loc)
    l = bpy.context.object
    l.data.energy = energy
    l.data.size = size
    l.data.color = color
    d = Vector(target) - Vector(loc)
    l.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
    return l


def world(color=(0.9, 0.94, 1), strength=0.6):
    w = bpy.context.scene.world or bpy.data.worlds.new("w")
    bpy.context.scene.world = w
    w.use_nodes = True
    bg = w.node_tree.nodes["Background"]
    bg.inputs[0].default_value = (*color, 1)
    bg.inputs[1].default_value = strength


def shadow_floor(z=0.0, size=40):
    """A floor that only shows shadows, so the PNG stays transparent."""
    bpy.ops.mesh.primitive_plane_add(size=size, location=(0, 0, z))
    f = bpy.context.object
    f.is_shadow_catcher = True
    return f


def setup(w, h, samples=96, transparent=True, out=None, fmt="PNG"):
    s = bpy.context.scene
    s.render.engine = "CYCLES"
    s.cycles.samples = samples
    s.cycles.use_denoising = True
    try:
        prefs = bpy.context.preferences.addons["cycles"].preferences
        prefs.compute_device_type = "METAL"
        prefs.get_devices()
        for d in prefs.devices:
            d.use = True
        s.cycles.device = "GPU"
    except Exception as e:  # CPU fallback
        print("gpu unavailable:", e)
    s.render.resolution_x, s.render.resolution_y = w, h
    s.render.resolution_percentage = 100
    s.render.film_transparent = transparent
    s.render.image_settings.file_format = fmt
    s.render.image_settings.color_mode = "RGBA" if transparent else "RGB"
    s.view_settings.view_transform = "Standard"
    s.view_settings.look = "None"
    s.view_settings.exposure = -0.3
    if out:
        s.render.filepath = out

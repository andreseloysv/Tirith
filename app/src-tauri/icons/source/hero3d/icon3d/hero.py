# the generated hero model (GLB) standing in for the main bot, in the same place/pose the blob had
import math
import bpy
from mathutils import Vector
from .opts import opt
from .stage import at


def _boost(mat):
    """Saturate the baked texture and let it emit a little, so the hero glows like the other bots."""
    if not mat or not mat.use_nodes:
        return
    nt = mat.node_tree
    bsdf = next((n for n in nt.nodes if n.type == 'BSDF_PRINCIPLED'), None)
    if not bsdf:
        return
    bsdf.inputs['Roughness'].default_value = max(bsdf.inputs['Roughness'].default_value, 0.45)
    link = next((l for l in nt.links if l.to_node == bsdf and l.to_socket.name == 'Base Color'), None)
    if link:
        src = link.from_socket
        hs = nt.nodes.new('ShaderNodeHueSaturation')
        hs.inputs['Saturation'].default_value = float(opt.get('herosat', '1.35'))
        hs.inputs['Hue'].default_value = float(opt.get('herohue', '0.5'))
        hs.inputs['Value'].default_value = float(opt.get('heroval', '1.05'))
        nt.links.new(src, hs.inputs['Color'])
        nt.links.remove(link)
        nt.links.new(hs.outputs['Color'], bsdf.inputs['Base Color'])
        nt.links.new(hs.outputs['Color'], bsdf.inputs['Emission Color'])
    bsdf.inputs['Emission Strength'].default_value = float(opt.get('heroemit', '0.45'))


def make_hero(spec):
    path = opt['hero']
    before = set(bpy.context.scene.objects)
    bpy.ops.import_scene.gltf(filepath=path)
    new = [o for o in bpy.context.scene.objects if o not in before]
    tops = [o for o in new if o.parent is None or o.parent not in new]
    mins = Vector((1e9, 1e9, 1e9))
    maxs = Vector((-1e9, -1e9, -1e9))
    for o in new:
        if o.type != 'MESH':
            continue
        for c in o.bound_box:
            w = o.matrix_world @ Vector(c)
            mins = Vector(map(min, mins, w))
            maxs = Vector(map(max, maxs, w))
    center = (mins + maxs) / 2
    size = maxs - mins
    root = bpy.data.objects.new('main', None)
    bpy.context.scene.collection.objects.link(root)
    root.location = center
    for o in tops:
        o.parent = root
        o.matrix_parent_inverse = root.matrix_world.inverted()
    for o in new:
        o.pass_index = 1
        if o.type == 'MESH':
            for m in o.data.materials:
                _boost(m)
    pos = at(spec['sx'], spec['sy'], spec['d'])
    face_cam = math.atan2(pos.x, pos.y)
    root.location = pos
    root.rotation_euler = (math.radians(spec.get('pitch', 0)), math.radians(spec.get('roll', 0)), face_cam + math.radians(spec.get('yaw', 0)))
    s = spec['r'] * float(opt.get('heroscale', '1.18')) * (2.0 / max(size))
    root.scale = (s, s, s)
    root.pass_index = 1
    return root, pos

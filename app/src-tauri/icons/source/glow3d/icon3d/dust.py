# ---------------------------------------------------------------- dust: little streaks of light at every depth
import math
import bpy, bmesh
from mathutils import Matrix
from .opts import rng
from .cast import MAIN, lin
from .stage import at
from .geometry import link


def streaks(bm):
    glow = bm.verts.layers.float.new('glow')
    cols = bm.verts.layers.float_color.new('tint')
    palette = [lin('#ff6fd8'), lin('#7fe3ff'), (1, 1, 1), lin('#ffd27a'), lin('#b78cff')]
    for i in range(70):
        d = 1.2 + rng.random() ** 1.6 * 24
        sx, sy = rng.uniform(-0.05, 1.05), rng.uniform(-0.05, 1.05)
        if d < MAIN['d'] + 1.5 and (sx - MAIN['sx']) ** 2 + (sy - MAIN['sy']) ** 2 < 0.36 ** 2:
            continue  # nothing floats over the big bot's face
        c = at(sx, sy, d)
        ln = (0.01 + 0.05 * rng.random()) * d / 6
        th = rng.uniform(0, math.pi)
        m = Matrix.Translation(c) @ Matrix.Rotation(th, 4, 'Y') @ Matrix.Diagonal((ln, 0.004 * d / 6 + 0.002, 0.004 * d / 6 + 0.002, 1))
        before = set(bm.verts)
        bmesh.ops.create_icosphere(bm, subdivisions=1, radius=1.0, matrix=m)
        g = 0.3 + 0.7 * rng.random()
        tint = palette[rng.randrange(len(palette))]
        for v in set(bm.verts) - before:
            v[glow] = g
            v[cols] = (*tint, 1)

def dust_material():
    dm = bpy.data.materials.new('dust')
    dm.use_nodes = True
    N, L = dm.node_tree.nodes, dm.node_tree.links
    for n in list(N):
        if n.type != 'OUTPUT_MATERIAL':
            N.remove(n)
    em = N.new('ShaderNodeEmission')
    ga = N.new('ShaderNodeAttribute'); ga.attribute_name = 'glow'
    ca = N.new('ShaderNodeAttribute'); ca.attribute_name = 'tint'
    mul = N.new('ShaderNodeMath'); mul.operation = 'MULTIPLY'; mul.inputs[1].default_value = 5.0
    L.new(ga.outputs['Fac'], mul.inputs[0]); L.new(mul.outputs['Value'], em.inputs['Strength'])
    L.new(ca.outputs['Color'], em.inputs['Color'])
    L.new(em.outputs['Emission'], N['Material Output'].inputs['Surface'])
    return dm

def make_dust():
    bm = bmesh.new()
    streaks(bm)
    me = bpy.data.meshes.new('dust')
    bm.to_mesh(me)
    bm.free()
    dust = link(bpy.data.objects.new('dust', me))
    dust.data.materials.append(dust_material())
    return dust

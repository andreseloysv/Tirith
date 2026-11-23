# ---------------------------------------------------------------- trails of sparkles from behind the big bot to each flyer
import math
import bpy, bmesh
from mathutils import Vector, Matrix
from .opts import rng
from .cast import MEET, lin, mixc
from .stage import at
from .geometry import link
from .materials import emit_material


def sparkles(bm, glow, meet, pos, f):
    direction = pos - meet
    length = direction.length
    side = direction.cross(Vector((0, 1, 0))).normalized()
    up = side.cross(direction).normalized()
    count = 900
    for i in range(count):
        t = rng.random() ** 0.8
        spread = (0.01 + 0.16 * t * t) * f['r'] / 0.4
        c = meet + direction * t + up * (0.06 * length * math.sin(math.pi * t)) + side * rng.gauss(0, spread) + up * rng.gauss(0, spread)
        c -= direction.normalized() * (f['r'] * 0.6)  # stop at the bot's back
        size = (0.0018 + 0.0045 * rng.random()) * (0.6 + 0.9 * t) * (pos.y / 8.0)
        before = set(bm.verts)
        bmesh.ops.create_icosphere(bm, subdivisions=1, radius=size, matrix=Matrix.Translation(c))
        g = (0.10 + 0.90 * t) * (0.3 + 0.7 * rng.random()) * (4.0 if rng.random() < 0.03 else 1.0)
        for v in set(bm.verts) - before:
            v[glow] = g

def make_trails(flyer_pos):
    meet = at(MEET['sx'], MEET['sy'], MEET['d'])
    for k, (pos, f) in enumerate(flyer_pos):
        bm = bmesh.new()
        glow = bm.verts.layers.float.new('glow')
        sparkles(bm, glow, meet, pos, f)
        me = bpy.data.meshes.new(f'trail{k}')
        bm.to_mesh(me)
        bm.free()
        # the float layer becomes a point attribute called "glow"
        ob = link(bpy.data.objects.new(f'trail{k}', me))
        whiteness = min(lin(f['color']))
        ob.data.materials.append(emit_material(f'trail{k}', mixc(lin(f['color']), (1, 1, 1), 0.12), 9.0 * (1 - 0.6 * whiteness), use_attr=True))

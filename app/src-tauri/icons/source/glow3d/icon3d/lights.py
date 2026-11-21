# ---------------------------------------------------------------- lights
import bpy
from mathutils import Vector
from .opts import opt
from .geometry import link


def area(name, loc, target, size, power, rgb):
    ld = bpy.data.lights.new(name, 'AREA')
    ld.size = size
    ld.energy = power
    ld.color = rgb
    ob = link(bpy.data.objects.new(name, ld))
    ob.location = loc
    dirv = (Vector(target) - Vector(loc)).normalized()
    ob.rotation_euler = dirv.to_track_quat('-Z', 'Y').to_euler()
    return ob

def make_lights(main_pos):
    mp = main_pos
    area('catch', (mp.x - 1.4, mp.y - 3.2, mp.z + 1.3), mp, 0.12, 25, (1.0, 0.97, 0.92))
    area('key', (mp.x - 1.6, mp.y - 2.2, mp.z + 2.0), mp, 2.4, float(opt.get('key', '110')), (1.0, 0.96, 0.88))
    area('rim', (mp.x + 1.6, mp.y + 1.8, mp.z + 1.6), mp, 1.6, 50, (1.0, 0.88, 0.70))
    area('fill', (mp.x - 2.0, mp.y - 1.5, mp.z - 1.2), mp, 3.0, 15, (0.75, 0.55, 1.0))

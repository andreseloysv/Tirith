# ---------------------------------------------------------------- the sky: nebula, stars, a warm light behind the big bot
import math
import bpy, bmesh
from . import stage
from .geometry import link
from .skyglow import glows
from .stars import stars


def sky_plane():
    sky_d = 60
    bm = bmesh.new()
    bmesh.ops.create_grid(bm, x_segments=1, y_segments=1, size=sky_d * stage.TAN * 1.25)
    me = bpy.data.meshes.new('sky')
    bm.to_mesh(me)
    bm.free()
    sky = link(bpy.data.objects.new('sky', me))
    sky.location = (0, sky_d, 0)
    sky.rotation_euler = (math.radians(90), 0, 0)
    return sky

def nebula(N, L, tc):
    """Nebula: two layers of noise."""
    n1 = N.new('ShaderNodeTexNoise'); n1.inputs['Scale'].default_value = 1.6; n1.inputs['Detail'].default_value = 10; n1.inputs['Roughness'].default_value = 0.62; n1.inputs['Distortion'].default_value = 0.35
    L.new(tc.outputs['Generated'], n1.inputs['Vector'])
    r1 = N.new('ShaderNodeValToRGB')
    r1.color_ramp.elements[0].position = 0.36; r1.color_ramp.elements[0].color = (0.004, 0.005, 0.022, 1)
    r1.color_ramp.elements[1].position = 0.82; r1.color_ramp.elements[1].color = (0.13, 0.035, 0.22, 1)
    mid = r1.color_ramp.elements.new(0.6); mid.color = (0.018, 0.02, 0.085, 1)
    L.new(n1.outputs['Fac'], r1.inputs['Fac'])
    n2 = N.new('ShaderNodeTexNoise'); n2.inputs['Scale'].default_value = 3.2; n2.inputs['Detail'].default_value = 8; n2.inputs['Roughness'].default_value = 0.55
    L.new(tc.outputs['Generated'], n2.inputs['Vector'])
    r2 = N.new('ShaderNodeValToRGB')
    r2.color_ramp.elements[0].position = 0.5; r2.color_ramp.elements[0].color = (0, 0, 0, 1)
    r2.color_ramp.elements[1].position = 0.82; r2.color_ramp.elements[1].color = (0.03, 0.10, 0.40, 1)
    L.new(n2.outputs['Fac'], r2.inputs['Fac'])
    add1 = N.new('ShaderNodeMix'); add1.data_type = 'RGBA'; add1.blend_type = 'ADD'; add1.inputs['Factor'].default_value = 1.0
    L.new(r1.outputs['Color'], add1.inputs['A']); L.new(r2.outputs['Color'], add1.inputs['B'])
    return add1

def make_sky(dust):
    sky = sky_plane()
    sm = bpy.data.materials.new('sky')
    sm.use_nodes = True
    N, L = sm.node_tree.nodes, sm.node_tree.links
    for n in list(N):
        if n.type != 'OUTPUT_MATERIAL':
            N.remove(n)
    tc = N.new('ShaderNodeTexCoord')
    add3 = glows(N, L, tc, nebula(N, L, tc))
    add4 = stars(N, L, tc, add3)
    emsky = N.new('ShaderNodeEmission'); emsky.inputs['Strength'].default_value = 1.0
    L.new(add4.outputs['Result'], emsky.inputs['Color'])
    L.new(emsky.outputs['Emission'], N['Material Output'].inputs['Surface'])
    sky.data.materials.append(sm)
    for ob_ in (sky, dust):  # seen by the camera, but they light nothing
        ob_.visible_diffuse = ob_.visible_glossy = ob_.visible_transmission = ob_.visible_volume_scatter = False

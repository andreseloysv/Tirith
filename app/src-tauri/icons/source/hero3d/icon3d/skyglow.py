# ---------------------------------------------------------------- the sky's lights: radial glows on the sky plane
from .opts import opt
from .cast import MAIN, MEET


def radial_glow(N, L, tc, cx, cy, radius, rgb, power):
    sep = N.new('ShaderNodeSeparateXYZ')
    L.new(tc.outputs['Generated'], sep.inputs['Vector'])
    dx = N.new('ShaderNodeMath'); dx.operation = 'SUBTRACT'; dx.inputs[1].default_value = cx
    dy = N.new('ShaderNodeMath'); dy.operation = 'SUBTRACT'; dy.inputs[1].default_value = cy
    L.new(sep.outputs['X'], dx.inputs[0]); L.new(sep.outputs['Y'], dy.inputs[0])
    cmb = N.new('ShaderNodeCombineXYZ')
    L.new(dx.outputs['Value'], cmb.inputs['X']); L.new(dy.outputs['Value'], cmb.inputs['Y'])
    ln_ = N.new('ShaderNodeVectorMath'); ln_.operation = 'LENGTH'
    L.new(cmb.outputs['Vector'], ln_.inputs[0])
    dv = N.new('ShaderNodeMath'); dv.operation = 'DIVIDE'; dv.inputs[1].default_value = radius
    L.new(ln_.outputs['Value'], dv.inputs[0])
    inv = N.new('ShaderNodeMath'); inv.operation = 'SUBTRACT'; inv.inputs[0].default_value = 1.0; inv.use_clamp = True
    L.new(dv.outputs['Value'], inv.inputs[1])
    pw = N.new('ShaderNodeMath'); pw.operation = 'POWER'; pw.inputs[1].default_value = power
    L.new(inv.outputs['Value'], pw.inputs[0])
    mx_ = N.new('ShaderNodeMix'); mx_.data_type = 'RGBA'
    L.new(pw.outputs['Value'], mx_.inputs['Factor'])
    mx_.inputs['A'].default_value = (0, 0, 0, 1)
    mx_.inputs['B'].default_value = (*rgb, 1)
    return mx_.outputs['Result']

# generated coords of the plane run 0..1 across what the camera sees (the plane is 1.25x the view)
def gen(sx, sy):
    return 0.5 + (sx - 0.5) / 1.25, 0.5 + (0.5 - sy) / 1.25

def glows(N, L, tc, add1):
    """Warm glow centred behind the big bot, and a violet haze around where the trails meet, added onto add1."""
    gx, gy = gen(MAIN['sx'] - 0.04, MAIN['sy'] - 0.05)
    warm = radial_glow(N, L, tc, gx, gy, float(opt.get('warmr', '0.40')), (2.2, 0.9, 0.22), 1.7)
    mx2, my2 = gen(MEET['sx'], MEET['sy'])
    violet = radial_glow(N, L, tc, mx2, my2, 0.55, (0.32, 0.11, 0.52), 2.0)
    add2 = N.new('ShaderNodeMix'); add2.data_type = 'RGBA'; add2.blend_type = 'ADD'; add2.inputs['Factor'].default_value = 1.0
    L.new(add1.outputs['Result'], add2.inputs['A']); L.new(warm, add2.inputs['B'])
    add3 = N.new('ShaderNodeMix'); add3.data_type = 'RGBA'; add3.blend_type = 'ADD'; add3.inputs['Factor'].default_value = 1.0
    L.new(add2.outputs['Result'], add3.inputs['A']); L.new(violet, add3.inputs['B'])
    return add3

# ---------------------------------------------------------------- materials: flat, emitting, and the ones every bot shares
import bpy
from .cast import lin

EYE = MOUTH = TONGUE = SPARK = REDEYE = OMOUTH = None  # made by shared(), once the scene exists


def flat_material(name, rgb, rough=0.3, emit=None, strength=0.0, coat=0.0):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    b = mat.node_tree.nodes['Principled BSDF']
    b.inputs['Base Color'].default_value = (*rgb, 1)
    b.inputs['Roughness'].default_value = rough
    b.inputs['Coat Weight'].default_value = coat
    if emit:
        b.inputs['Emission Color'].default_value = (*emit, 1)
        b.inputs['Emission Strength'].default_value = strength
    return mat

def emit_material(name, rgb, strength, use_attr=False):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    nt = mat.node_tree
    N, L = nt.nodes, nt.links
    for n in list(N):
        if n.type != 'OUTPUT_MATERIAL':
            N.remove(n)
    em = N.new('ShaderNodeEmission')
    em.inputs['Color'].default_value = (*rgb, 1)
    if use_attr:
        at_ = N.new('ShaderNodeAttribute')
        at_.attribute_name = 'glow'
        mul = N.new('ShaderNodeMath')
        mul.operation = 'MULTIPLY'
        mul.inputs[1].default_value = strength
        L.new(at_.outputs['Fac'], mul.inputs[0])
        L.new(mul.outputs['Value'], em.inputs['Strength'])
    else:
        em.inputs['Strength'].default_value = strength
    L.new(em.outputs['Emission'], N['Material Output'].inputs['Surface'])
    return mat

def shared():
    global EYE, MOUTH, TONGUE, SPARK, REDEYE, OMOUTH
    EYE = flat_material('eye', (0.010, 0.005, 0.008), rough=0.12, coat=1.0)
    REDEYE = flat_material('redeye', lin('#7d0033'), rough=0.12, coat=1.0)
    OMOUTH = flat_material('omouth', lin('#c90016'), rough=0.35, emit=lin('#ff2438'), strength=0.35, coat=0.5)
    MOUTH = flat_material('mouth', (0.035, 0.004, 0.006), rough=0.6, emit=(0.30, 0.02, 0.02), strength=0.25)
    TONGUE = flat_material('tongue', lin('#d2405d'), rough=0.45, emit=lin('#ff6a7a'), strength=0.6)
    SPARK = emit_material('spark', (1, 1, 1), 9.0)

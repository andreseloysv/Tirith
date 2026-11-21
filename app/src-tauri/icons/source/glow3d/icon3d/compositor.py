# ---------------------------------------------------------------- compositor: a halo of their colour around the bots, bloom
import bpy
from .opts import opt, SIZE


def comp(scene):
    tree = bpy.data.node_groups.new('comp', 'CompositorNodeTree')
    scene.compositing_node_group = tree
    tree.interface.new_socket('Image', in_out='OUTPUT', socket_type='NodeSocketColor')
    scene.view_layers[0].use_pass_object_index = True
    T, TL = tree.nodes, tree.links
    rl = T.new('CompositorNodeRLayers')
    idm = T.new('CompositorNodeIDMask')
    idm.inputs['Index'].default_value = 1
    print('RL outputs:', [o.name for o in rl.outputs])
    TL.new(next(o for o in rl.outputs if 'ndex' in o.name and 'Mat' not in o.name), idm.inputs['ID value'])
    def mix(kind, a, b, fac=1.0):
        m = T.new('ShaderNodeMix')
        m.data_type = 'RGBA'
        m.blend_type = kind
        m.inputs['Factor'].default_value = fac
        if isinstance(a, tuple):
            m.inputs[6].default_value = a
        else:
            TL.new(a, m.inputs[6])
        if isinstance(b, tuple):
            m.inputs[7].default_value = b
        else:
            TL.new(b, m.inputs[7])
        return m.outputs[2]
    def blur(src, px):
        bl = T.new('CompositorNodeBlur')
        try:
            bl.inputs['Size'].default_value = (px, px)
        except Exception:
            bl.inputs['Size'].default_value = px
        try:
            bl.inputs['Extend Bounds'].default_value = True
        except Exception:
            pass
        TL.new(src, bl.inputs['Image'])
        return bl.outputs['Image']
    soft = blur(idm.outputs['Alpha'], SIZE * float(opt.get('mblur', '0.006')))  # a soft mask, also for the defocused ones
    bots = mix('MULTIPLY', rl.outputs['Image'], soft)  # only the bots
    near = blur(bots, SIZE * float(opt.get('h1', '0.035')))
    far = blur(bots, SIZE * float(opt.get('h2', '0.10')))
    halo = mix('ADD', near, far, float(opt.get('h2w', '0.9')))
    inv = T.new('ShaderNodeMath')
    inv.operation = 'SUBTRACT'
    inv.inputs[0].default_value = 1.0
    TL.new(soft, inv.inputs[1])
    outside = mix('MULTIPLY', halo, inv.outputs['Value'])
    lit = mix('ADD', rl.outputs['Image'], outside, float(opt.get('haloamt', '1.1')))
    bloom(T, TL, lit)
    scene.render.use_compositing = not opt.get('nocomp')

def bloom(T, TL, lit):
    g1 = T.new('CompositorNodeGlare')
    g1.inputs['Type'].default_value = 'Bloom'
    g1.inputs['Threshold'].default_value = float(opt.get('bthr', '0.95'))
    g1.inputs['Strength'].default_value = float(opt.get('bloom', '0.3'))
    g1.inputs['Size'].default_value = 0.7
    TL.new(lit, g1.inputs['Image'])
    hs = T.new('CompositorNodeHueSat')
    hs.inputs['Saturation'].default_value = float(opt.get('sat', '1.0'))
    TL.new(g1.outputs['Image'], hs.inputs['Image'])
    out = T.new('NodeGroupOutput')
    TL.new(hs.outputs['Image'], out.inputs[0])

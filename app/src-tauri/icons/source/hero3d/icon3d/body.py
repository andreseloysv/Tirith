# ---------------------------------------------------------------- materials: a bot's glowing body
import bpy
from .opts import opt
from .cast import lin


def body_material(name, color, gain=1.0, core_hex=None, edge_hex=None):
    base = lin(color)
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    nt = mat.node_tree
    N, L = nt.nodes, nt.links
    bsdf = N['Principled BSDF']
    bsdf.inputs['Base Color'].default_value = (*base, 1)
    bsdf.inputs['Roughness'].default_value = 0.45
    bsdf.inputs['Specular IOR Level'].default_value = 0.25
    bsdf.inputs['Subsurface Weight'].default_value = float(opt.get('sss', '0.35'))
    bsdf.inputs['Subsurface Radius'].default_value = tuple(0.15 + 0.85 * c for c in base)
    bsdf.inputs['Subsurface Scale'].default_value = 0.3
    bsdf.inputs['Coat Weight'].default_value = float(opt.get('coat', '0.35'))
    bsdf.inputs['Coat Roughness'].default_value = 0.28
    lw = N.new('ShaderNodeLayerWeight')
    lw.inputs['Blend'].default_value = float(opt.get('blend', '0.5'))
    core = N.new('ShaderNodeMix')
    core.data_type = 'RGBA'
    L.new(lw.outputs['Facing'], core.inputs['Factor'])
    core.inputs['A'].default_value = (*(lin(core_hex) if core_hex else [c ** float(opt.get('coreexp', '0.72')) for c in base]), 1)  # facing the camera: a lighter tint of its own colour
    deep = tuple(c ** 1.25 for c in base)
    core.inputs['B'].default_value = (*(lin(edge_hex) if edge_hex else base), 1)  # the rim: its own colour
    geo0 = N.new('ShaderNodeNewGeometry')
    sep0 = N.new('ShaderNodeSeparateXYZ')
    L.new(geo0.outputs['Normal'], sep0.inputs['Vector'])
    low = N.new('ShaderNodeMapRange')
    L.new(sep0.outputs['Z'], low.inputs['Value'])
    low.inputs['From Min'].default_value = -1.0
    low.inputs['From Max'].default_value = 0.2
    low.inputs['To Min'].default_value = float(opt.get('lowmix', '0.45'))
    low.inputs['To Max'].default_value = 0.0
    warmmix = N.new('ShaderNodeMix')
    warmmix.data_type = 'RGBA'
    L.new(low.outputs['Result'], warmmix.inputs['Factor'])
    L.new(core.outputs['Result'], warmmix.inputs['A'])
    warmmix.inputs['B'].default_value = (*tuple(c ** 1.6 for c in (lin(edge_hex) if edge_hex else base)), 1)
    L.new(warmmix.outputs['Result'], bsdf.inputs['Emission Color'])
    st = N.new('ShaderNodeMapRange')
    L.new(lw.outputs['Facing'], st.inputs['Value'])
    st.inputs['To Min'].default_value = gain * float(opt.get('ecenter', '0.95'))
    st.inputs['To Max'].default_value = gain * float(opt.get('eedge', '0.9'))
    geo = N.new('ShaderNodeNewGeometry')
    sep = N.new('ShaderNodeSeparateXYZ')
    L.new(geo.outputs['Normal'], sep.inputs['Vector'])
    under = N.new('ShaderNodeMapRange')
    L.new(sep.outputs['Z'], under.inputs['Value'])
    under.inputs['From Min'].default_value = -1.0
    under.inputs['From Max'].default_value = 0.6
    under.inputs['To Min'].default_value = float(opt.get('under', '0.7'))
    under.inputs['To Max'].default_value = 1.0
    shade = N.new('ShaderNodeMath')
    shade.operation = 'MULTIPLY'
    L.new(st.outputs['Result'], shade.inputs[0])
    L.new(under.outputs['Result'], shade.inputs[1])
    L.new(shade.outputs['Value'], bsdf.inputs['Emission Strength'])
    return mat

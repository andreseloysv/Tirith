# ---------------------------------------------------------------- the sky's stars, added onto the nebula and its glows


def stars(N, L, tc, add3):
    vo = N.new('ShaderNodeTexVoronoi'); vo.inputs['Scale'].default_value = 260.0; vo.inputs['Randomness'].default_value = 1.0
    L.new(tc.outputs['Generated'], vo.inputs['Vector'])
    lt = N.new('ShaderNodeMath'); lt.operation = 'LESS_THAN'; lt.inputs[1].default_value = 0.045
    L.new(vo.outputs['Distance'], lt.inputs[0])
    keep = N.new('ShaderNodeSeparateColor')
    L.new(vo.outputs['Color'], keep.inputs['Color'])
    few = N.new('ShaderNodeMath'); few.operation = 'GREATER_THAN'; few.inputs[1].default_value = 0.55
    L.new(keep.outputs['Red'], few.inputs[0])
    both = N.new('ShaderNodeMath'); both.operation = 'MULTIPLY'
    L.new(lt.outputs['Value'], both.inputs[0]); L.new(few.outputs['Value'], both.inputs[1])
    bright = N.new('ShaderNodeMath'); bright.operation = 'MULTIPLY'; bright.inputs[1].default_value = 3.0
    L.new(both.outputs['Value'], bright.inputs[0])
    tint = N.new('ShaderNodeMix'); tint.data_type = 'RGBA'; tint.inputs['Factor'].default_value = 0.75
    L.new(vo.outputs['Color'], tint.inputs['A']); tint.inputs['B'].default_value = (1, 1, 1, 1)
    starc = N.new('ShaderNodeMix'); starc.data_type = 'RGBA'; starc.blend_type = 'MULTIPLY'; starc.inputs['Factor'].default_value = 1.0
    L.new(tint.outputs['Result'], starc.inputs['A'])
    cmbv = N.new('ShaderNodeCombineColor')
    L.new(bright.outputs['Value'], cmbv.inputs['Red']); L.new(bright.outputs['Value'], cmbv.inputs['Green']); L.new(bright.outputs['Value'], cmbv.inputs['Blue'])
    L.new(cmbv.outputs['Color'], starc.inputs['B'])
    add4 = N.new('ShaderNodeMix'); add4.data_type = 'RGBA'; add4.blend_type = 'ADD'; add4.inputs['Factor'].default_value = 1.0
    L.new(add3.outputs['Result'], add4.inputs['A']); L.new(starc.outputs['Result'], add4.inputs['B'])
    return add4

"""Social preview 1280x640: the stone tower at dusk and the name standing beside it. Usage: -- OUT.png [samples=N]"""
import os, sys
sys.path.insert(0, __file__.rsplit("/", 1)[0])
from common import *

argv = sys.argv[sys.argv.index("--") + 1:]
opts = dict(a.split("=", 1) for a in argv if "=" in a)
out = [a for a in argv if "=" not in a][0]
FONT = "/System/Library/Fonts/Supplemental/Georgia Bold.ttf"
reset()
sky(SKY, 0.16, float(opts.get("rot", 3.1)), (0.66, 0.76, 1.0), seen=0.36)
_, rim = dusk_rig(key_energy=11, rim_energy=18)
tower = stone_tower("t_", (-3.3, 0, 0.42))
ground = terrain("ground", "desert_rocks_01", flat=9.0)
bpy.ops.object.text_add(location=(-0.85, -0.6, 0.0), rotation=(math.pi / 2, 0, 0))
name = bpy.context.object
name.data.body = "Tirith"
if os.path.isfile(FONT):
    name.data.font = bpy.data.fonts.load(FONT)
name.data.size = 2.75
name.data.extrude = 0.16
name.data.bevel_depth = 0.022
name.data.bevel_resolution = 3
# pale weathered stone: a text curve has no UVs for the rock maps, so its pits are procedural
carved = mat("name", (0.66, 0.58, 0.47, 1), rough=0.8)
n, l = carved.node_tree.nodes, carved.node_tree.links
pits = n.new("ShaderNodeTexNoise")
pits.inputs["Scale"].default_value = 26
pits.inputs["Detail"].default_value = 8
bump = n.new("ShaderNodeBump")
bump.inputs["Strength"].default_value = 0.55
bump.inputs["Distance"].default_value = 0.05
l.new(pits.outputs[0], bump.inputs["Height"])
l.new(bump.outputs["Normal"], n["Principled BSDF"].inputs["Normal"])
name.data.materials.append(carved)
area((2.2, -7, 3.4), (2.2, -0.6, 1.4), 95, 9, (1.0, 0.72, 0.48)).visible_camera = False
light = glow_mat("light", (0.03, 0.38, 1.0), 1.9)
window = (-3.57, -1.03, 2.05)
parts = []
for i, (x, y, z, h) in enumerate(((-5.55, -0.8, 3.1, 0.62), (-5.3, -0.9, 1.2, 0.56), (-0.6, -1.7, 3.45, 0.6))):
    parts += crystal("agent%d" % i, (x, y, z), h, seed=i * 5 + 2)
    beam("beam%d" % i, window, (x, y, z), light, lift=0.4)
unlit_by(rim, [ground, name])
for i in range(3):
    unlit_by(bpy.data.objects["agent%d_lamp" % i], [o for o in tower if o.type == "MESH"] + [name])
look((0, -14.3, 1.0), (0, 0, 2.05), 37)
setup(1280, 640, int(opts.get("samples", 512)), False, out, bloom=0.3)
in_frame(parts + tower + [name])
bpy.ops.render.render(write_still=True)

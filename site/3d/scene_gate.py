"""A stone gate at dusk with rules of light across its arch, and an agent crystal stopped in front. Usage: -- OUT.png W H [samples=N]"""
import sys
sys.path.insert(0, __file__.rsplit("/", 1)[0])
from common import *

argv = sys.argv[sys.argv.index("--") + 1:]
opts = dict(a.split("=", 1) for a in argv if "=" in a)
out, W, H = [a for a in argv if "=" not in a]
reset()
sky(SKY, 0.16, float(opts.get("rot", 3.1)), (0.66, 0.76, 1.0), seen=0.36)
_, rim = dusk_rig(key_energy=11, rim_energy=18)
mats = stone_materials("g_")
solid = stone_wall("w_", mats)
for side in (-1, 1):
    solid += stone_turret("tr%d_" % side, (side * 3.05, 0.2, 0), mats, facing=-0.05 * side, seed=4 + side)
ground = terrain("ground", opts.get("ground", "desert_rocks_01"), flat=10.0)
rocks = stone_materials("rock_", "desert_rocks_01")["stone"]
boulders = []
for i, (x, y, r) in enumerate(((-4.6, -1.6, 0.46), (-3.9, -2.6, 0.24), (4.4, -3.6, 0.42), (3.5, -4.6, 0.2), (5.2, 1.4, 0.6), (-5.6, 2.0, 0.5))):
    boulders.append(boulder("rock%d" % i, (x, y, r * 0.2), r, rocks, seed=i + 11))
# the road through the gate
road = box("road", (0, -6.4, 0.012), (2.4, 17.0, 0.03), mats["floor"], 0)
uv = road.data.uv_layers[0]
for loop in road.data.loops:
    co = road.data.vertices[loop.vertex_index].co
    uv.data[loop.index].uv = (co.x, co.y)
dice(road)
# the rules: bars of light across the opening, with iron uprights behind them
opening, spring = 1.2, 1.75
rule = glow_mat("rule", (1.0, 0.3, 0.05), 6.0)
for i in range(7):
    z = 0.32 + i * 0.41
    half = opening if z <= spring else math.sqrt(opening ** 2 - (z - spring) ** 2)
    box("rule%d" % i, (0, 0.1, z), (2 * half, 0.05, 0.05), rule, 0.012)
for x in (-0.8, -0.4, 0, 0.4, 0.8):
    top = spring + math.sqrt(opening ** 2 - x ** 2)
    box("bar", (x, 0.2, top / 2), (0.05, 0.05, top), mats["iron"], 0.006)
area((0, -0.4, 1.4), (0, -6, 0.2), 90, 2.2, (1.0, 0.42, 0.12)).visible_camera = False
# the agent stops at the rules
agent = (0.25, -2.5, 1.12)
queued = (0.35, -6.2, 0.74)
parts = crystal("agent", agent, 0.95, power=26, seed=7)
parts += crystal("queued", queued, 0.52, power=10, seed=12)
light = glow_mat("light", (0.03, 0.38, 1.0), 1.9)
beam("stop", agent, (0.25, 0.08, 1.14), light, lift=0.0, r=0.012)
beam("queue", queued, agent, light, lift=0.1, r=0.008)
unlit_by(rim, [ground, road] + boulders)
look((4.3, -11.8, 1.15), (0.1, -0.2, 2.1), 33)
setup(int(W), int(H), int(opts.get("samples", 512)), False, out)
in_frame(parts + solid + boulders)
bpy.ops.render.render(write_still=True)

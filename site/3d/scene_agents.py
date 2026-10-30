"""The stone tower at dusk, its window joined by light to the agents, glowing crystals. Usage: -- OUT.png W H [samples=N]"""
import sys
sys.path.insert(0, __file__.rsplit("/", 1)[0])
from common import *

argv = sys.argv[sys.argv.index("--") + 1:]
opts = dict(a.split("=", 1) for a in argv if "=" in a)
out, W, H = [a for a in argv if "=" not in a]
reset()
sky(SKY, 0.16, float(opts.get("rot", 3.1)), (0.66, 0.76, 1.0), seen=0.36)
_, rim = dusk_rig(key_energy=11, rim_energy=18)
tower = stone_tower("t_", (0, 0, 0.42))
ground = terrain("ground", opts.get("ground", "desert_rocks_01"))
rocks = stone_materials("rock_", "desert_rocks_01")["stone"]
boulders = []
for i, (x, y, r) in enumerate(((-3.3, -3.0, 0.36), (-2.7, -3.7, 0.18), (3.4, -2.8, 0.42), (2.7, -3.7, 0.2), (-5.2, 1.4, 0.7), (5.4, 1.6, 0.55), (2.6, 2.6, 0.5))):
    boulders.append(boulder("rock%d" % i, (x, y, r * 0.2), r, rocks, seed=i + 3))
window = (-0.27, -1.03, 2.05)
agents = [(-4.5, -1.3, 1.5, 0.92), (-3.05, -2.3, 3.5, 0.74), (-1.95, -3.7, 0.95, 0.66), (2.0, -3.6, 1.05, 0.7), (3.1, -1.9, 3.55, 0.82), (4.6, -1.0, 1.75, 0.96)]
light = glow_mat("light", (0.03, 0.38, 1.0), 1.9)
parts = []
for i, (x, y, z, h) in enumerate(agents):
    parts += crystal("agent%d" % i, (x, y, z), h, seed=i * 5 + 2)
    beam("beam%d" % i, window, (x, y, z), light, lift=0.55)
unlit_by(rim, [ground] + boulders)
# the crystals pool on the ground; on the tower their blue would cancel the warm key
for i in range(len(agents)):
    unlit_by(bpy.data.objects["agent%d_lamp" % i], [o for o in tower if o.type == "MESH"])
look((0, -10.6, 1.0), (0, 0, 2.0), 31)
setup(int(W), int(H), int(opts.get("samples", 512)), False, out, bloom=0.3)  # more would veil the tower in the blue of six crystals
in_frame(parts + tower + boulders)
bpy.ops.render.render(write_still=True)

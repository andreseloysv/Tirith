# the options after "--" on Blender's command line (--name=value), the output and the random seed
import os, random, sys

argv = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
opt = dict(a[2:].split('=', 1) for a in argv if a.startswith('--') and '=' in a)
HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))  # glow3d/, where build.py and masks/ live
OUT = opt.get('out', os.path.join(HERE, 'out', 'render.png'))
SIZE = int(opt.get('size', '768'))
SAMPLES = int(opt.get('samples', '48'))
SEED = int(opt.get('seed', '7'))
FSTOP = float(opt.get('fstop', '2.2'))
rng = random.Random(SEED)

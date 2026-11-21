# Tirith's app icon in 3D: our bot shapes as soft, glowing, translucent bodies in deep space, one big bot in front
# and the others flying out from behind it with sparkle trails; depth of field and bloom.
# Blender -b --factory-startup -P build.py -- --out=out/r.png --size=768 --samples=48
import bpy, bmesh, math, os, random, sys
import numpy as np
from mathutils import Vector, Matrix

argv = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
opt = dict(a[2:].split('=', 1) for a in argv if a.startswith('--') and '=' in a)
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = opt.get('out', os.path.join(HERE, 'out', 'render.png'))
SIZE = int(opt.get('size', '768'))
SAMPLES = int(opt.get('samples', '48'))
SEED = int(opt.get('seed', '7'))
FSTOP = float(opt.get('fstop', '2.2'))
rng = random.Random(SEED)

# ---------------------------------------------------------------- the bots (our shapes; eyes [xL, xR, y, rx, ry] in the 100 box)
EYES = {
    'round': [36, 62, 44, 7, 10], 'dome': [37, 63, 52, 7, 10], 'cloud': [40, 64, 48, 7, 9.5], 'tile': [38, 62, 46, 7, 9.5],
    'star': [42, 58, 46, 4.5, 7], 'flower': [39, 61, 48, 6, 9], 'drop': [37, 61, 46, 7, 9.5], 'pill': [37, 63, 49, 7.5, 10.5],
    'tri': [42, 58, 58, 5, 7.5], 'hex': [38, 62, 46, 7, 9.5], 'burst': [39, 61, 47, 6, 9], 'blob': [36, 62, 45, 7.5, 10.5],
}
MAIN = dict(shape=opt.get('main', 'blob'), color='#ffd23f', core=opt.get('maincore', '#fff6a0'), edge=opt.get('mainedge', '#e9a823'), sx=0.60, sy=0.67, d=float(opt.get('maind', '3.0')), r=1.05, yaw=float(opt.get('yaw', '6')), pitch=float(opt.get('pitch', '3')), roll=float(opt.get('roll', '-10')), mood='grin')
FLYERS = [  # screen position from the top left, distance, radius
    dict(shape='cloud', color='#3ee08f', sx=0.15, sy=0.63, d=6.0, r=0.50, mood='grin', yaw=-14, roll=8),
    dict(shape='hex', color='#4aa8ff', sx=0.86, sy=0.25, d=7.0, r=0.46, mood='grin', yaw=12, roll=-10),
    dict(shape='flower', color='#ff5fd2', sx=0.20, sy=0.30, d=9.0, r=0.40, mood='smile', yaw=-10, roll=12),
    dict(shape='drop', color='#ff8a3d', sx=0.43, sy=0.27, d=8.5, r=0.40, mood='grin', yaw=-4, roll=-6),
    dict(shape='burst', color='#2fd3c4', sx=0.58, sy=0.10, d=11.0, r=0.40, mood='smile', yaw=6, roll=-8),
    dict(shape='tri', color='#ff5d73', sx=0.33, sy=0.10, d=13.0, r=0.30, mood='smile', yaw=-6, roll=10),
    dict(shape='star', color='#9b6bff', sx=0.17, sy=0.09, d=15.0, r=0.26, mood='smile', yaw=-8, roll=14),
    dict(shape='pill', color='#f4f1ff', sx=0.46, sy=0.05, d=17.0, r=0.24, mood='smile', yaw=0, roll=-4),
]
MEET = dict(sx=0.40, sy=0.55, d=9.5)  # where the trails start, behind the big bot

def lin(hexs):  # sRGB hex -> linear rgb
    c = [int(hexs[i:i + 2], 16) / 255 for i in (1, 3, 5)]
    return tuple(((v + 0.055) / 1.055) ** 2.4 if v > 0.04045 else v / 12.92 for v in c)

def mixc(a, b, t):
    return tuple(x + (y - x) * t for x, y in zip(a, b))

# ---------------------------------------------------------------- scene
bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
scene.render.engine = 'CYCLES'
prefs = bpy.context.preferences.addons['cycles'].preferences
prefs.compute_device_type = 'METAL'
prefs.get_devices()
for dv in prefs.devices:
    dv.use = dv.type == 'METAL'
scene.cycles.device = 'GPU'
scene.cycles.samples = SAMPLES
scene.cycles.use_denoising = True
scene.cycles.max_bounces = 8
scene.cycles.transparent_max_bounces = 8
scene.render.resolution_x = scene.render.resolution_y = SIZE
scene.render.resolution_percentage = 100
scene.render.image_settings.file_format = 'PNG'
scene.render.image_settings.color_mode = 'RGB'
scene.render.filepath = OUT
scene.view_settings.view_transform = opt.get('view', 'Standard')
try:
    scene.view_settings.look = opt.get('look', 'AgX - Punchy')
except Exception:
    pass
scene.world = bpy.data.worlds.new('space')
scene.world.use_nodes = True
scene.world.node_tree.nodes['Background'].inputs['Color'].default_value = (0.002, 0.003, 0.010, 1)

# camera at the origin looking along +Y
cam_data = bpy.data.cameras.new('cam')
cam_data.lens = float(opt.get('lens', '32'))
cam_data.sensor_width = 36
cam_data.dof.use_dof = True
cam_data.dof.aperture_fstop = FSTOP
cam = bpy.data.objects.new('cam', cam_data)
scene.collection.objects.link(cam)
cam.location = (0, 0, 0)
cam.rotation_euler = (math.radians(90), 0, 0)
scene.camera = cam
TAN = 18 / cam_data.lens

def at(sx, sy, d):
    u, v = sx - 0.5, 0.5 - sy
    return Vector((2 * u * TAN * d, d, 2 * v * TAN * d))

# ---------------------------------------------------------------- shapes
def load_mask(name):
    img = bpy.data.images.load(os.path.join(HERE, 'masks', name + '.png'))
    w, h = img.size
    px = np.array(img.pixels[:], dtype=np.float32).reshape(h, w, 4)
    bpy.data.images.remove(img)
    return px[..., 0] > 0.5

def radial(mask, n=720):
    h, w = mask.shape
    ys, xs = np.nonzero(mask)
    cx, cy = xs.mean(), ys.mean()
    ang = np.linspace(0, 2 * np.pi, n, endpoint=False)
    steps = np.arange(0, 300, 0.5)
    X = np.clip(np.round(cx + np.outer(np.cos(ang), steps)).astype(int), 0, w - 1)
    Y = np.clip(np.round(cy + np.outer(np.sin(ang), steps)).astype(int), 0, h - 1)
    out = ~mask[Y, X]
    first = np.where(out.any(axis=1), out.argmax(axis=1), len(steps) - 1)
    r = steps[np.maximum(first - 1, 1)]
    k = np.array([1, 2, 3, 2, 1], float)
    k /= k.sum()
    r = np.convolve(np.concatenate([r[-2:], r, r[:2]]), k, mode='valid')
    return cx, cy, r

class Shape:
    """A bot's soft body: the outline r(theta) blown up into a pillow (superellipse profile)."""
    P = 2.35
    def __init__(self, name, depth=0.62):
        self.name, self.depth = name, depth
        mask = load_mask(name)
        self.cx, self.cy, r = radial(mask)
        self.rmax = r.max()
        self.r = r / self.rmax
        self.n = len(r)
        self.px = 512 / 150  # mask pixels per unit of the 100 box

    def R(self, th):
        f = (th % (2 * math.pi)) / (2 * math.pi) * self.n
        i = int(f) % self.n
        t = f - int(f)
        return self.r[i] * (1 - t) + self.r[(i + 1) % self.n] * t

    def box(self, x, y):  # a point of the 100 box (y down) -> plane (u right, w up), bot radius 1
        u = ((x + 25) * self.px - self.cx) / self.rmax
        w = ((512 - (y + 25) * self.px) - self.cy) / self.rmax
        return u, w

    def front(self, u, w):  # the front surface (towards -Y)
        rho = math.hypot(u, w)
        t = min(rho / self.R(math.atan2(w, u)), 0.995)
        return Vector((u, -self.depth * (1 - t ** self.P) ** (1 / self.P), w))

    def normal(self, u, w, e=1e-3):
        p = self.front(u, w)
        du = self.front(u + e, w) - p
        dw = self.front(u, w + e) - p
        return du.cross(dw).normalized()  # outward (towards -Y)

    def mesh(self, name, subdiv=6):
        bm = bmesh.new()
        bmesh.ops.create_icosphere(bm, subdivisions=subdiv, radius=1.0)
        p = self.P
        for v in bm.verts:
            a, b, c = v.co
            th = math.atan2(c, a)
            m = min(1.0, math.hypot(a, c))
            rr = self.R(th) * m ** (2 / p)
            v.co = Vector((rr * math.cos(th), self.depth * math.copysign(abs(b) ** (2 / p), b), rr * math.sin(th)))
        for _ in range(2):
            bmesh.ops.smooth_vert(bm, verts=bm.verts, factor=0.35, use_axis_x=True, use_axis_y=True, use_axis_z=True)
        for f in bm.faces:
            f.smooth = True
        me = bpy.data.meshes.new(name)
        bm.to_mesh(me)
        bm.free()
        return me

SHAPES = {}
def shape(name):
    if name not in SHAPES:
        SHAPES[name] = Shape(name)
    return SHAPES[name]

# ---------------------------------------------------------------- materials
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

EYE = flat_material('eye', (0.010, 0.005, 0.008), rough=0.12, coat=1.0)
MOUTH = flat_material('mouth', (0.035, 0.004, 0.006), rough=0.6, emit=(0.30, 0.02, 0.02), strength=0.25)
TONGUE = flat_material('tongue', lin('#d2405d'), rough=0.45, emit=lin('#ff6a7a'), strength=0.6)
SPARK = emit_material('spark', (1, 1, 1), 9.0)

# ---------------------------------------------------------------- a bot
def link(obj, parent=None):
    scene.collection.objects.link(obj)
    if parent:
        obj.parent = parent
    return obj

def prism(sh, poly, name, front=0.22, back=0.14):
    """A cutter that follows the bot's front surface over a 2D outline (plane coords)."""
    bm = bmesh.new()
    top, bot = [], []
    for u, w in poly:
        p = sh.front(u, w)
        n = sh.normal(u, w)
        top.append(bm.verts.new(p + n * front))
        bot.append(bm.verts.new(p - n * back))
    bm.faces.new(top)
    bm.faces.new(list(reversed(bot)))
    k = len(poly)
    for i in range(k):
        j = (i + 1) % k
        bm.faces.new([top[j], top[i], bot[i], bot[j]])
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    return me

def cubic(p0, p1, p2, p3, n):
    return [tuple((1 - t) ** 3 * a + 3 * (1 - t) ** 2 * t * b + 3 * (1 - t) * t * t * c + t ** 3 * d for a, b, c, d in zip(p0, p1, p2, p3)) for t in [i / n for i in range(n)]]

def quad(p0, p1, p2, n):
    return [tuple((1 - t) ** 2 * a + 2 * (1 - t) * t * b + t * t * c for a, b, c in zip(p0, p1, p2)) for t in [i / n for i in range(n)]]

def make_bot(spec, tag):
    sh = shape(spec['shape'])
    root = link(bpy.data.objects.new(f'{tag}', None))
    body = link(bpy.data.objects.new(f'{tag}-body', sh.mesh(f'{tag}-body')), root)
    body.data.materials.append(body_material(f'{tag}-mat', spec['color'], float(opt.get('maingain', '0.8')) if tag == 'main' else float(opt.get('flygain', '0.95')), spec.get('core'), spec.get('edge')))
    e = EYES[spec['shape']]
    # eyes: glossy dark lenses set into the surface
    for i, x in enumerate((e[0], e[1])):
        u, w = sh.box(x, e[2])
        rx, ry = 0.84 * e[3] * sh.px / sh.rmax, 0.84 * e[4] * sh.px / sh.rmax
        p, n = sh.front(u, w), sh.normal(u, w)
        me = bpy.data.meshes.new(f'{tag}-eye{i}')
        bm = bmesh.new()
        bmesh.ops.create_uvsphere(bm, u_segments=32, v_segments=16, radius=1.0)
        for f in bm.faces:
            f.smooth = True
        bm.to_mesh(me)
        bm.free()
        eye = link(bpy.data.objects.new(f'{tag}-eye{i}', me), root)
        eye.data.materials.append(EYE)
        yv = -n
        xv = Vector((1, 0, 0))
        zv = xv.cross(yv).normalized()
        if zv.z < 0:
            zv = -zv
        xv = yv.cross(zv).normalized()
        rot = Matrix((xv, yv, zv)).transposed().to_4x4()
        dep = 0.30 * rx
        eye.matrix_basis = Matrix.Translation(p - n * dep * 0.55) @ rot @ Matrix.Diagonal((rx, dep, ry, 1))
        cu, cw = u - 0.28 * rx, w + 0.42 * ry
        cp = sh.front(cu, cw) + sh.normal(cu, cw) * dep * 0.30
        me2 = bpy.data.meshes.new(f'{tag}-catch{i}')
        bm2 = bmesh.new()
        bmesh.ops.create_icosphere(bm2, subdivisions=2, radius=1.0)
        bm2.to_mesh(me2)
        bm2.free()
        dot = link(bpy.data.objects.new(f'{tag}-catch{i}', me2), root)
        dot.data.materials.append(SPARK)
        dot.matrix_basis = Matrix.Translation(cp) @ rot @ Matrix.Diagonal((0.17 * rx, 0.05 * rx, 0.17 * rx, 1))
    # mouth
    mx, my, half = (e[0] + e[1]) / 2, e[2] + e[4] + 6, (e[1] - e[0]) * 0.5
    if spec['mood'] == 'grin':
        top = quad((mx + half, my), (mx, my + half * 0.16), (mx - half, my), 18)
        bottom = cubic((mx - half, my), (mx - half, my + half * 1.35), (mx + half, my + half * 1.35), (mx + half, my), 36)
        poly = [sh.box(x, y) for x, y in bottom + top]
        cut = link(bpy.data.objects.new(f'{tag}-mouthcut', prism(sh, poly, f'{tag}-mouthcut', back=0.24)), root)
        cut.data.materials.append(MOUTH)
        cut.hide_render = True
        cut.display_type = 'WIRE'
        mod = body.modifiers.new('mouth', 'BOOLEAN')
        mod.operation = 'DIFFERENCE'
        mod.solver = 'EXACT'
        mod.object = cut
        try:
            mod.material_mode = 'TRANSFER'
        except Exception:
            pass
        # the tongue, low in the mouth
        tu, tw = sh.box(mx, my + half * 0.80)
        p, n = sh.front(tu, tw), sh.normal(tu, tw)
        me = bpy.data.meshes.new(f'{tag}-tongue')
        bm = bmesh.new()
        bmesh.ops.create_uvsphere(bm, u_segments=32, v_segments=16, radius=1.0)
        for f in bm.faces:
            f.smooth = True
        bm.to_mesh(me)
        bm.free()
        tg = link(bpy.data.objects.new(f'{tag}-tongue', me), root)
        tg.data.materials.append(TONGUE)
        s = half * sh.px / sh.rmax
        yv = -n
        zv = Vector((1, 0, 0)).cross(yv).normalized()
        zv = zv if zv.z > 0 else -zv
        xv = yv.cross(zv).normalized()
        rot = Matrix((xv, yv, zv)).transposed().to_4x4()
        tg.matrix_basis = Matrix.Translation(p - n * 0.09) @ rot @ Matrix.Diagonal((s * 0.56, 0.08, s * 0.27, 1))
        im_ = tg.modifiers.new('inside', 'BOOLEAN')
        im_.operation = 'INTERSECT'
        im_.solver = 'EXACT'
        im_.object = cut
    else:  # a closed smile: a dark tube that follows the face
        a = quad((mx - half * 0.6, my - 1), (mx, my - 1 + half * 1.3), (mx + half * 0.6, my - 1), 24) + [(mx + half * 0.6, my - 1)]
        cd = bpy.data.curves.new(f'{tag}-smile', 'CURVE')
        cd.dimensions = '3D'
        cd.bevel_depth = 1.6 * sh.px / sh.rmax
        cd.bevel_resolution = 4
        cd.use_fill_caps = True
        sp = cd.splines.new('POLY')
        sp.points.add(len(a) - 1)
        for k, (x, y) in enumerate(a):
            u, w = sh.box(x, y)
            q = sh.front(u, w) - sh.normal(u, w) * 0.3 * cd.bevel_depth
            sp.points[k].co = (q.x, q.y, q.z, 1)
        sm_ = link(bpy.data.objects.new(f'{tag}-smile', cd), root)
        sm_.data.materials.append(EYE)
    dg = bpy.context.evaluated_depsgraph_get()
    if body.modifiers and len(body.evaluated_get(dg).data.vertices) < 100:
        body.modifiers.clear()
        print('dropped the mouth cut of', tag)
    for ch in root.children:
        ch.pass_index = 1
    # place it: size, screen position, a turn towards the camera plus its own
    pos = at(spec['sx'], spec['sy'], spec['d'])
    face_cam = math.atan2(pos.x, pos.y)
    root.location = pos
    root.rotation_euler = (math.radians(spec.get('pitch', 0)), math.radians(spec.get('roll', 0)), face_cam + math.radians(spec.get('yaw', 0)))
    root.scale = (spec['r'] * (1.0 if tag == 'main' else float(opt.get('flyscale', '1.25'))),) * 3
    return root, pos

main, main_pos = make_bot(MAIN, 'main')
flyer_pos = []
for i, f in enumerate(FLYERS):
    _, pos = make_bot(f, f'fly{i}')
    flyer_pos.append((pos, f))

# ---------------------------------------------------------------- trails of sparkles from behind the big bot to each flyer
meet = at(MEET['sx'], MEET['sy'], MEET['d'])
for k, (pos, f) in enumerate(flyer_pos):
    bm = bmesh.new()
    glow = bm.verts.layers.float.new('glow')
    direction = pos - meet
    length = direction.length
    side = direction.cross(Vector((0, 1, 0))).normalized()
    up = side.cross(direction).normalized()
    count = 900
    for i in range(count):
        t = rng.random() ** 0.8
        spread = (0.01 + 0.16 * t * t) * f['r'] / 0.4
        c = meet + direction * t + up * (0.06 * length * math.sin(math.pi * t)) + side * rng.gauss(0, spread) + up * rng.gauss(0, spread)
        c -= direction.normalized() * (f['r'] * 0.6)  # stop at the bot's back
        size = (0.0018 + 0.0045 * rng.random()) * (0.6 + 0.9 * t) * (pos.y / 8.0)
        before = set(bm.verts)
        bmesh.ops.create_icosphere(bm, subdivisions=1, radius=size, matrix=Matrix.Translation(c))
        g = (0.10 + 0.90 * t) * (0.3 + 0.7 * rng.random()) * (4.0 if rng.random() < 0.03 else 1.0)
        for v in set(bm.verts) - before:
            v[glow] = g
    me = bpy.data.meshes.new(f'trail{k}')
    bm.to_mesh(me)
    bm.free()
    # the float layer becomes a point attribute called "glow"
    ob = link(bpy.data.objects.new(f'trail{k}', me))
    whiteness = min(lin(f['color']))
    ob.data.materials.append(emit_material(f'trail{k}', mixc(lin(f['color']), (1, 1, 1), 0.12), 9.0 * (1 - 0.6 * whiteness), use_attr=True))

# ---------------------------------------------------------------- dust: little streaks of light at every depth
bm = bmesh.new()
glow = bm.verts.layers.float.new('glow')
cols = bm.verts.layers.float_color.new('tint')
palette = [lin('#ff6fd8'), lin('#7fe3ff'), (1, 1, 1), lin('#ffd27a'), lin('#b78cff')]
for i in range(70):
    d = 1.2 + rng.random() ** 1.6 * 24
    sx, sy = rng.uniform(-0.05, 1.05), rng.uniform(-0.05, 1.05)
    if d < MAIN['d'] + 1.5 and (sx - MAIN['sx']) ** 2 + (sy - MAIN['sy']) ** 2 < 0.36 ** 2:
        continue  # nothing floats over the big bot's face
    c = at(sx, sy, d)
    ln = (0.01 + 0.05 * rng.random()) * d / 6
    th = rng.uniform(0, math.pi)
    m = Matrix.Translation(c) @ Matrix.Rotation(th, 4, 'Y') @ Matrix.Diagonal((ln, 0.004 * d / 6 + 0.002, 0.004 * d / 6 + 0.002, 1))
    before = set(bm.verts)
    bmesh.ops.create_icosphere(bm, subdivisions=1, radius=1.0, matrix=m)
    g = 0.3 + 0.7 * rng.random()
    tint = palette[rng.randrange(len(palette))]
    for v in set(bm.verts) - before:
        v[glow] = g
        v[cols] = (*tint, 1)
me = bpy.data.meshes.new('dust')
bm.to_mesh(me)
bm.free()
dust = link(bpy.data.objects.new('dust', me))
dm = bpy.data.materials.new('dust')
dm.use_nodes = True
N, L = dm.node_tree.nodes, dm.node_tree.links
for n in list(N):
    if n.type != 'OUTPUT_MATERIAL':
        N.remove(n)
em = N.new('ShaderNodeEmission')
ga = N.new('ShaderNodeAttribute'); ga.attribute_name = 'glow'
ca = N.new('ShaderNodeAttribute'); ca.attribute_name = 'tint'
mul = N.new('ShaderNodeMath'); mul.operation = 'MULTIPLY'; mul.inputs[1].default_value = 5.0
L.new(ga.outputs['Fac'], mul.inputs[0]); L.new(mul.outputs['Value'], em.inputs['Strength'])
L.new(ca.outputs['Color'], em.inputs['Color'])
L.new(em.outputs['Emission'], N['Material Output'].inputs['Surface'])
dust.data.materials.append(dm)

# ---------------------------------------------------------------- the sky: nebula, stars, a warm light behind the big bot
sky_d = 60
bm = bmesh.new()
bmesh.ops.create_grid(bm, x_segments=1, y_segments=1, size=sky_d * TAN * 1.25)
me = bpy.data.meshes.new('sky')
bm.to_mesh(me)
bm.free()
sky = link(bpy.data.objects.new('sky', me))
sky.location = (0, sky_d, 0)
sky.rotation_euler = (math.radians(90), 0, 0)
sm = bpy.data.materials.new('sky')
sm.use_nodes = True
N, L = sm.node_tree.nodes, sm.node_tree.links
for n in list(N):
    if n.type != 'OUTPUT_MATERIAL':
        N.remove(n)
tc = N.new('ShaderNodeTexCoord')
# nebula: two layers of noise
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
# warm glow centred behind the big bot, and a violet haze around where the trails meet
def radial_glow(cx, cy, radius, rgb, power):
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
gx, gy = gen(MAIN['sx'] - 0.04, MAIN['sy'] - 0.05)
warm = radial_glow(gx, gy, float(opt.get('warmr', '0.40')), (2.2, 0.9, 0.22), 1.7)
mx2, my2 = gen(MEET['sx'], MEET['sy'])
violet = radial_glow(mx2, my2, 0.55, (0.32, 0.11, 0.52), 2.0)
add2 = N.new('ShaderNodeMix'); add2.data_type = 'RGBA'; add2.blend_type = 'ADD'; add2.inputs['Factor'].default_value = 1.0
L.new(add1.outputs['Result'], add2.inputs['A']); L.new(warm, add2.inputs['B'])
add3 = N.new('ShaderNodeMix'); add3.data_type = 'RGBA'; add3.blend_type = 'ADD'; add3.inputs['Factor'].default_value = 1.0
L.new(add2.outputs['Result'], add3.inputs['A']); L.new(violet, add3.inputs['B'])
# stars
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
emsky = N.new('ShaderNodeEmission'); emsky.inputs['Strength'].default_value = 1.0
L.new(add4.outputs['Result'], emsky.inputs['Color'])
L.new(emsky.outputs['Emission'], N['Material Output'].inputs['Surface'])
sky.data.materials.append(sm)
for ob_ in (sky, dust):  # seen by the camera, but they light nothing
    ob_.visible_diffuse = ob_.visible_glossy = ob_.visible_transmission = ob_.visible_volume_scatter = False

# ---------------------------------------------------------------- lights
def area(name, loc, target, size, power, rgb):
    ld = bpy.data.lights.new(name, 'AREA')
    ld.size = size
    ld.energy = power
    ld.color = rgb
    ob = link(bpy.data.objects.new(name, ld))
    ob.location = loc
    dirv = (Vector(target) - Vector(loc)).normalized()
    ob.rotation_euler = dirv.to_track_quat('-Z', 'Y').to_euler()
    return ob
mp = main_pos
area('catch', (mp.x - 1.4, mp.y - 3.2, mp.z + 1.3), mp, 0.12, 25, (1.0, 0.97, 0.92))
area('key', (mp.x - 1.6, mp.y - 2.2, mp.z + 2.0), mp, 2.4, float(opt.get('key', '110')), (1.0, 0.96, 0.88))
area('rim', (mp.x + 1.6, mp.y + 1.8, mp.z + 1.6), mp, 1.6, 50, (1.0, 0.88, 0.70))
area('fill', (mp.x - 2.0, mp.y - 1.5, mp.z - 1.2), mp, 3.0, 15, (0.75, 0.55, 1.0))

# ---------------------------------------------------------------- focus, compositor (bloom), render
cam_data.dof.focus_distance = (main_pos - cam.location).length - 0.6
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
scene.render.use_compositing = not opt.get('nocomp')

os.makedirs(os.path.dirname(OUT), exist_ok=True)
if opt.get('blend'):
    bpy.ops.wm.save_as_mainfile(filepath=opt['blend'])
bpy.ops.render.render(write_still=True)
print('WROTE', OUT)

"""Shared helpers for the Tirith Blender scenes. Run via: Blender -b -P <scene>.py -- <args>"""
import math, os, random, sys, tempfile, zipfile
import bmesh
import bpy
from bpy_extras.object_utils import world_to_camera_view
from mathutils import Matrix, Vector, noise

BLUE = (0.05, 0.22, 0.9, 1)
LIGHT_BLUE = (0.12, 0.45, 1.0, 1)
INK = (0.07, 0.094, 0.15, 1)
PEARL = (0.62, 0.7, 0.82, 1)
WARM = (1.0, 0.42, 0.1)

# Texture packs are not in the repo (unknown license): only renders are committed.
TEXTURES_ENV = "TIRITH_TEXTURES"
TEXTURES_DEFAULT = "~/Downloads/texture and references"
TEXTURE_CACHE = "~/.cache/tirith-textures"
PACK_MAPS = {
    "color": ("baseColor", "color"),
    "height": ("height",),
    "normal": ("normal_gl",),
    "rough": ("roughness",),
    "ao": ("ambientOcclusion", "ambient_occlusion"),
}
STONE = "cliff_rocks_09"
FLOOR = "ground_tiles_20"
SKY = "sky_94"
DICE_PX = 1.5


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    return bpy.context.scene


def mat(name, color, metallic=0.0, rough=0.3, emit=0.0, alpha=1.0, trans=0.0, ior=1.45):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    b = m.node_tree.nodes["Principled BSDF"]
    b.inputs["Base Color"].default_value = color
    b.inputs["Metallic"].default_value = metallic
    b.inputs["Roughness"].default_value = rough
    b.inputs["IOR"].default_value = ior
    if trans:
        b.inputs["Transmission Weight"].default_value = trans
    if emit:
        b.inputs["Emission Color"].default_value = color
        b.inputs["Emission Strength"].default_value = emit
    return m


def box(name, loc, size, m, bevel=0.03, rot=(0, 0, 0)):
    bpy.ops.mesh.primitive_cube_add(location=loc, rotation=rot)
    o = bpy.context.object
    o.name = name
    o.scale = (size[0] / 2, size[1] / 2, size[2] / 2)
    bpy.ops.object.transform_apply(scale=True)
    if bevel:
        bv = o.modifiers.new("bevel", "BEVEL")
        bv.width = bevel
        bv.segments = 4
        bv.limit_method = "ANGLE"
    o.data.materials.append(m)
    bpy.ops.object.shade_smooth()
    return o


def sphere(name, loc, r, m):
    bpy.ops.mesh.primitive_uv_sphere_add(location=loc, radius=r, segments=48, ring_count=24)
    o = bpy.context.object
    o.name = name
    o.data.materials.append(m)
    bpy.ops.object.shade_smooth()
    return o


def tube(name, a, b, r, m):
    a, b = Vector(a), Vector(b)
    d = b - a
    bpy.ops.mesh.primitive_cylinder_add(radius=r, depth=d.length, location=(a + b) / 2, vertices=16)
    o = bpy.context.object
    o.name = name
    o.rotation_mode = "QUATERNION"
    o.rotation_quaternion = d.to_track_quat("Z", "Y")
    o.data.materials.append(m)
    bpy.ops.object.shade_smooth()
    return o


def tower(prefix, loc=(0, 0, 0), s=1.0, body=None, glow=None):
    """The Tirith watchtower mark. Height about 2.4*s, centred on loc (base at loc.z)."""
    body = body or mat("tower_body", PEARL, metallic=0.55, rough=0.22)
    glow = glow or mat("tower_glow", LIGHT_BLUE, emit=3.0, rough=0.2)
    x, y, z = loc
    parts = [
        box(prefix + "base", (x, y, z + 0.1 * s), (1.5 * s, 1.5 * s, 0.2 * s), body, 0.05 * s),
        box(prefix + "shaft", (x, y, z + 0.95 * s), (1.0 * s, 1.0 * s, 1.5 * s), body, 0.05 * s),
        box(prefix + "ledge", (x, y, z + 1.78 * s), (1.35 * s, 1.35 * s, 0.16 * s), body, 0.04 * s),
    ]
    for i, (dx, dy) in enumerate(((-0.46, -0.46), (0.46, -0.46), (-0.46, 0.46), (0.46, 0.46), (0, -0.46), (0, 0.46))):
        parts.append(box(prefix + "merlon%d" % i, (x + dx * s, y + dy * s, z + 2.07 * s), (0.34 * s, 0.34 * s, 0.42 * s), body, 0.035 * s))
    parts.append(box(prefix + "window", (x, y - 0.5 * s, z + 1.05 * s), (0.34 * s, 0.06 * s, 0.56 * s), glow, 0.08 * s))
    return parts


def texture_zip(name):
    """The zip of a texture pack, or stop the render: never fall back to a flat look."""
    folder = os.path.expanduser(os.environ.get(TEXTURES_ENV) or TEXTURES_DEFAULT)
    path = os.path.join(folder, name + ".zip")
    if not os.path.isfile(path):
        print("ERROR: texture pack %s not found in %s. Set %s to the folder with the texture zips." % (name, folder, TEXTURES_ENV), file=sys.stderr)
        sys.exit(2)
    return path


def extract_png(zip_path, pack, wanted):
    """Unzip the .png members whose file name passes wanted() to the cache; return {file name: path}."""
    folder = os.path.join(os.path.expanduser(TEXTURE_CACHE), pack)
    os.makedirs(folder, exist_ok=True)
    found = {}
    with zipfile.ZipFile(zip_path) as z:
        for info in z.infolist():
            base = info.filename.rsplit("/", 1)[-1]  # never trust the paths inside a zip
            if info.is_dir() or not base.lower().endswith(".png") or not wanted(base):
                continue
            path = os.path.join(folder, base)
            if not os.path.isfile(path) or os.path.getsize(path) != info.file_size:
                fd, tmp = tempfile.mkstemp(dir=folder)
                with os.fdopen(fd, "wb") as dst, z.open(info) as src:
                    while chunk := src.read(1 << 20):
                        dst.write(chunk)
                os.replace(tmp, path)
            found[base] = path
    return found


def pack_maps(pack, res="4k"):
    """The PBR maps of a pack as {color, height, normal, rough, ao: path}."""
    names = {}
    for key, aliases in PACK_MAPS.items():
        for alias in aliases:
            names["%s_%s_%s.png" % (pack, alias, res)] = key
    found = extract_png(texture_zip("%s_%s" % (pack, res)), pack, lambda base: base in names)
    maps = {names[base]: path for base, path in found.items()}
    missing = set(PACK_MAPS) - set(maps)
    if missing:
        print("ERROR: pack %s has no maps for %s (folder from %s)." % (pack, sorted(missing), TEXTURES_ENV), file=sys.stderr)
        sys.exit(2)
    return maps


def sky(name=SKY, strength=0.8, rot=0.0, tint=(1, 1, 1), seen=None):
    """Sky panorama for soft ambient and reflections. It is PNG, not HDR: add a sun for the shadows.

    tint colours the light (a cool fill from a warm sky); seen is the strength the camera sees, if different.
    """
    base = name + "_2k.png"
    found = extract_png(texture_zip(name + "_2k"), name, lambda b: b == base)
    w = bpy.context.scene.world or bpy.data.worlds.new("w")
    bpy.context.scene.world = w
    w.use_nodes = True
    n, l = w.node_tree.nodes, w.node_tree.links
    env = n.new("ShaderNodeTexEnvironment")
    env.image = bpy.data.images.load(found[base], check_existing=True)
    env.interpolation = "Cubic"
    mp = n.new("ShaderNodeMapping")
    mp.inputs["Rotation"].default_value[2] = rot
    l.new(n.new("ShaderNodeTexCoord").outputs["Generated"], mp.inputs["Vector"])
    l.new(mp.outputs[0], env.inputs["Vector"])
    bg = n["Background"]
    mix = n.new("ShaderNodeMix")
    mix.data_type = "RGBA"
    mix.blend_type = "MULTIPLY"
    mix.inputs["Factor"].default_value = 1.0
    mix.inputs["B"].default_value = (*tint, 1)
    l.new(env.outputs["Color"], mix.inputs["A"])
    l.new(mix.outputs["Result"], bg.inputs[0])
    bg.inputs[1].default_value = strength
    if seen is not None:
        by_ray = n.new("ShaderNodeMapRange")
        by_ray.inputs["To Min"].default_value = strength
        by_ray.inputs["To Max"].default_value = seen
        l.new(n.new("ShaderNodeLightPath").outputs["Is Camera Ray"], by_ray.inputs["Value"])
        l.new(by_ray.outputs[0], bg.inputs[1])


def sun(direction, energy=3.0, angle=5.0, color=(1, 0.96, 0.9)):
    """Direction is where the sun stands, seen from the scene; a wide angle keeps the shadow soft."""
    bpy.ops.object.light_add(type="SUN")
    l = bpy.context.object
    l.data.energy = energy
    l.data.angle = math.radians(angle)
    l.data.color = color
    l.rotation_euler = Vector(direction).to_track_quat("Z", "Y").to_euler()
    l["no_floor"] = True  # a long sun shadow on a transparent floor would run out of the frame
    return l


def dusk_rig(key=(1.0, -0.42, 0.3), rim=(-1.0, 0.72, 0.4), key_energy=7.0, rim_energy=6.0):
    """Cinematic light: a warm low sun rakes one side across the courses, a cool sun from behind cuts out the other."""
    return sun(key, key_energy, 2.5, (1.0, 0.64, 0.36)), sun(rim, rim_energy, 4.0, (0.45, 0.62, 1.0))


def textured_mat(name, maps, tile, disp, tint=1.0, vary=0.0, sat=1.0, contrast=0.0, grain=0.0, cavity=0.0, normal=1.0, rough=(0.0, 1.0), stain=0.0):
    """PBR material with real displacement. UVs are in world units; vary shifts the tone of each block.

    cavity darkens the low parts of the height map, stain adds slow tone changes across the stones, so
    the colour comes from relief and weathering and not from the pattern of the map.
    """
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    m.displacement_method = "DISPLACEMENT"  # bump on top would sharpen the strata lines in the rock maps
    n, l = m.node_tree.nodes, m.node_tree.links
    b = n["Principled BSDF"]
    b.inputs["Specular IOR Level"].default_value = 0.35
    mp = n.new("ShaderNodeMapping")
    mp.inputs["Scale"].default_value = (1 / tile, 1 / tile, 1)
    l.new(n.new("ShaderNodeTexCoord").outputs["UV"], mp.inputs["Vector"])

    def tex(key, data=True):
        t = n.new("ShaderNodeTexImage")
        t.image = bpy.data.images.load(maps[key], check_existing=True)
        if data:
            t.image.colorspace_settings.name = "Non-Color"
        l.new(mp.outputs[0], t.inputs["Vector"])
        return t

    def multiply(a, b, factor=1.0):
        mix = n.new("ShaderNodeMix")
        mix.data_type = "RGBA"
        mix.blend_type = "MULTIPLY"
        mix.inputs["Factor"].default_value = factor
        l.new(a, mix.inputs["A"])
        l.new(b, mix.inputs["B"])
        return mix.outputs["Result"]

    height_tex = tex("height")
    height_tex.interpolation = "Cubic"
    height = height_tex.outputs["Color"]
    color = multiply(tex("color", False).outputs["Color"], tex("ao").outputs["Color"], 0.8)
    bc = n.new("ShaderNodeBrightContrast")
    bc.inputs["Contrast"].default_value = contrast
    l.new(color, bc.inputs["Color"])
    color = bc.outputs["Color"]
    if cavity:
        dark = n.new("ShaderNodeMapRange")
        dark.inputs["From Min"].default_value = 0.22
        dark.inputs["From Max"].default_value = 0.55
        dark.inputs["To Min"].default_value = 1 - cavity
        l.new(height, dark.inputs["Value"])
        color = multiply(color, dark.outputs[0])
    if stain:
        patch = n.new("ShaderNodeTexNoise")
        patch.inputs["Scale"].default_value = 2.6
        patch.inputs["Detail"].default_value = 5
        l.new(n.new("ShaderNodeTexCoord").outputs["Object"], patch.inputs["Vector"])
        tone = n.new("ShaderNodeMapRange")
        tone.inputs["From Min"].default_value = 0.32
        tone.inputs["From Max"].default_value = 0.68
        tone.inputs["To Min"].default_value = 1 - stain
        l.new(patch.outputs[0], tone.inputs["Value"])
        color = multiply(color, tone.outputs[0])
    hsv = n.new("ShaderNodeHueSaturation")
    l.new(color, hsv.inputs["Color"])
    hsv.inputs["Value"].default_value = tint
    hsv.inputs["Saturation"].default_value = sat
    if vary:
        rnd = n.new("ShaderNodeNewGeometry").outputs["Random Per Island"]
        val = n.new("ShaderNodeMapRange")
        val.inputs["To Min"].default_value = tint * (1 - vary)
        val.inputs["To Max"].default_value = tint * (1 + vary)
        l.new(rnd, val.inputs["Value"])
        l.new(val.outputs[0], hsv.inputs["Value"])
        hue = n.new("ShaderNodeMapRange")
        hue.inputs["To Min"].default_value = 0.485
        hue.inputs["To Max"].default_value = 0.515
        l.new(rnd, hue.inputs["Value"])
        l.new(hue.outputs[0], hsv.inputs["Hue"])
    l.new(hsv.outputs["Color"], b.inputs["Base Color"])
    rmap = n.new("ShaderNodeMapRange")
    rmap.inputs["To Min"].default_value, rmap.inputs["To Max"].default_value = rough
    l.new(tex("rough").outputs["Color"], rmap.inputs["Value"])
    l.new(rmap.outputs[0], b.inputs["Roughness"])
    nm = n.new("ShaderNodeNormalMap")
    nm.inputs["Strength"].default_value = normal
    l.new(tex("normal").outputs["Color"], nm.inputs["Color"])
    if grain:  # pits finer than the maps resolve at this distance
        noise = n.new("ShaderNodeTexNoise")
        noise.inputs["Scale"].default_value = 90
        noise.inputs["Detail"].default_value = 6
        l.new(mp.outputs[0], noise.inputs["Vector"])
        bump = n.new("ShaderNodeBump")
        bump.inputs["Strength"].default_value = grain
        bump.inputs["Distance"].default_value = 0.02
        l.new(noise.outputs[0], bump.inputs["Height"])
        l.new(nm.outputs["Normal"], bump.inputs["Normal"])
        l.new(bump.outputs["Normal"], b.inputs["Normal"])
    else:
        l.new(nm.outputs["Normal"], b.inputs["Normal"])
    dn = n.new("ShaderNodeDisplacement")
    dn.inputs["Scale"].default_value = disp
    dn.inputs["Midlevel"].default_value = 0.5
    l.new(height, dn.inputs["Height"])
    l.new(dn.outputs[0], n["Material Output"].inputs["Displacement"])
    return m


def _on_tower(th, z, r):
    """Tower coordinates: th=0 faces -Y and grows towards +X."""
    return (r * math.sin(th), -r * math.cos(th), z)


def dice(o):
    """Subdivide finer where the camera sees it, so the displacement is real relief at any distance."""
    sub = o.modifiers.new("detail", "SUBSURF")
    sub.subdivision_type = "SIMPLE"
    sub.levels = sub.render_levels = 1
    sub.use_adaptive_subdivision = True
    sub.adaptive_space = "PIXEL"
    sub.adaptive_pixel_size = DICE_PX
    return sub


class Mason:
    """Lays individual stones into one mesh: each one a closed, slightly irregular box, curved to the wall."""

    def __init__(self, seed, place=None):
        self.place = place or _on_tower  # (th, z, r) -> world; a flat wall passes its own
        self.rnd = random.Random(seed)
        self.bm = bmesh.new()
        self.uv = self.bm.loops.layers.uv.new("UVMap")

    def stone(self, corners, r_out, depth, nd=1, bulge=0.014, seg=0.035):
        """corners are (th, z): bottom-left, bottom-right, top-right, top-left. r_out(th, z) is the wall."""
        rnd, bm, uv = self.rnd, self.bm, self.uv
        (t0, z0), (t1, z1), (t2, z2), (t3, z3) = corners
        r_mid = r_out((t0 + t2) / 2, (z0 + z2) / 2)
        wide = max(abs(t1 - t0), abs(t2 - t3)) * r_mid
        high = max(abs(z3 - z0), abs(z2 - z1))
        nu, nv = max(3, min(30, int(wide / seg))), max(3, min(30, int(high / seg)))
        off, tilt_u, tilt_v = rnd.uniform(-0.012, 0.018), rnd.uniform(-0.012, 0.012), rnd.uniform(-0.01, 0.01)

        def pos(i, j, k):
            u, v, w = i / nu, j / nv, k / nd
            th = (t0 * (1 - u) + t1 * u) * (1 - v) + (t3 * (1 - u) + t2 * u) * v
            z = (z0 * (1 - u) + z1 * u) * (1 - v) + (z3 * (1 - u) + z2 * u) * v
            r = r_out(th, z)
            pillow = (1 - abs(2 * u - 1) ** 4) * (1 - abs(2 * v - 1) ** 4)
            front = r + off + tilt_u * (u - 0.5) + tilt_v * (v - 0.5) + bulge * pillow
            return self.place(th, z, r - depth + (front - r + depth) * w)

        verts = {}

        def vert(i, j, k):
            key = (i, j, k)
            if key not in verts:
                verts[key] = bm.verts.new(pos(i, j, k))
            return verts[key]

        ou, ov, rot = rnd.random() * 9, rnd.random() * 9, rnd.random() * math.tau
        cr, sr = math.cos(rot), math.sin(rot)
        made = []

        def sheet(na, nb, key, size_a, size_b, shift):
            for a in range(na):
                for b in range(nb):
                    f = bm.faces.new([vert(*key(a, b)), vert(*key(a + 1, b)), vert(*key(a + 1, b + 1)), vert(*key(a, b + 1))])
                    made.append(f)
                    for loop, (da, db) in zip(f.loops, ((a, b), (a + 1, b), (a + 1, b + 1), (a, b + 1))):
                        x, y = da / na * size_a, db / nb * size_b
                        loop[uv].uv = (ou + shift + x * cr - y * sr, ov + shift + x * sr + y * cr)

        sheet(nu, nv, lambda a, b: (a, b, nd), wide, high, 0)
        sheet(nu, nv, lambda a, b: (a, b, 0), wide, high, 1.3)
        sheet(nu, nd, lambda a, b: (a, 0, b), wide, depth, 2.1)
        sheet(nu, nd, lambda a, b: (a, nv, b), wide, depth, 3.7)
        sheet(nd, nv, lambda a, b: (0, b, a), depth, high, 4.3)
        sheet(nd, nv, lambda a, b: (nu, b, a), depth, high, 5.9)
        bmesh.ops.recalc_face_normals(bm, faces=made)

    def rect(self, th0, th1, z0, z1, r, gap=0.02):
        """A stone's corners for a span of the wall, with the joint taken off and a little jitter."""
        g, j = gap / 2, lambda: self.rnd.uniform(-0.004, 0.004)
        dt = g / r
        return ((th0 + dt + j() / r, z0 + g + j()), (th1 - dt + j() / r, z0 + g + j()), (th1 - dt + j() / r, z1 - g + j()), (th0 + dt + j() / r, z1 - g + j()))

    def course(self, z0, z1, r_out, depth, arc, prev=(), skip=None, nd=1):
        """A full ring of stones of varied width. Joints keep away from those of the course below."""
        rnd = self.rnd
        r = r_out(0, (z0 + z1) / 2)
        n = max(5, round(math.tau * r / arc))
        widths = [rnd.uniform(0.7, 1.35) for _ in range(n)]
        total = sum(widths)
        widths = [w / total * math.tau for w in widths]

        def clearance(ph):
            joints, t = [], ph
            for w in widths:
                joints.append(t)
                t += w
            if not prev:
                return 1, joints
            return min(min(abs((a - b + math.pi) % math.tau - math.pi) for b in prev) for a in joints), joints

        joints = max((clearance(rnd.random() * math.tau) for _ in range(40)), key=lambda c: c[0])[1]
        for a, w in zip(joints, widths):
            if skip is None or not skip(a, a + w, z0, z1):
                self.stone(self.rect(a, a + w, z0, z1, r), r_out, depth, nd)
        return joints

    def carved(self, corners, r_out, depth, cutter, nd=2):
        """A stone with an opening's shape taken out of it."""
        keep = self.bm
        self.bm = bmesh.new()
        self.uv = self.bm.loops.layers.uv.new("UVMap")
        self.stone(corners, r_out, depth, nd=nd)
        me = bpy.data.meshes.new("cutting")
        self.bm.to_mesh(me)
        self.bm.free()
        tmp = bpy.data.objects.new("cutting", me)
        bpy.context.collection.objects.link(tmp)
        done = _carved(tmp, cutter)
        bpy.data.objects.remove(tmp)
        keep.from_mesh(done)
        bpy.data.meshes.remove(done)
        bpy.data.meshes.remove(me)
        self.bm = keep
        self.uv = keep.loops.layers.uv["UVMap"]

    def finish(self, name, m, bevel=0.016):
        me = bpy.data.meshes.new(name)
        self.bm.to_mesh(me)
        self.bm.free()
        o = bpy.data.objects.new(name, me)
        bpy.context.collection.objects.link(o)
        me.materials.append(m)
        me.shade_smooth()
        bv = o.modifiers.new("bevel", "BEVEL")
        bv.width = bevel
        bv.segments = 2
        bv.limit_method = "ANGLE"
        bv.angle_limit = math.radians(35)
        dice(o)
        return o


def _lathe(name, profile, m, segments=128):
    """A closed solid of revolution from (r, z) points, bottom to top."""
    bm = bmesh.new()
    rings = [[bm.verts.new(_on_tower(i / segments * math.tau, z, r)) for i in range(segments)] for r, z in profile]
    for a, b in zip(rings, rings[1:]):
        for i in range(segments):
            bm.faces.new((a[i], a[(i + 1) % segments], b[(i + 1) % segments], b[i]))
    bm.faces.new(rings[0])
    bm.faces.new(rings[-1])
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces[:])
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    o = bpy.data.objects.new(name, me)
    bpy.context.collection.objects.link(o)
    me.materials.append(m)
    return o


def _arch_outline(half, z0, spring, steps=24):
    """(x, z) points of an arched opening, counter-clockwise from the bottom left."""
    pts = [(-half, z0), (half, z0)]
    pts += [(half * math.cos(i / steps * math.pi), spring + half * math.sin(i / steps * math.pi)) for i in range(steps + 1)]
    return pts


def _prism(name, th, outline, y_in, y_out, m=None):
    """An arched solid along the axis of a window at angle th; y counts outwards from the tower axis."""
    bm = bmesh.new()
    cs, sn = math.cos(th), math.sin(th)

    def p(x, y, z):
        return (x * cs + y * sn, x * sn - y * cs, z)

    a = [bm.verts.new(p(x, y_in, z)) for x, z in outline]
    b = [bm.verts.new(p(x, y_out, z)) for x, z in outline]
    for i in range(len(a)):
        j = (i + 1) % len(a)
        bm.faces.new((a[i], a[j], b[j], b[i]))
    bm.faces.new(a)
    bm.faces.new(b)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces[:])
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    o = bpy.data.objects.new(name, me)
    bpy.context.collection.objects.link(o)
    if m:
        me.materials.append(m)
    return o


def _carved(target, cutter):
    """target's mesh with cutter taken out, as a new mesh."""
    bo = target.modifiers.new("cut", "BOOLEAN")
    bo.operation = "DIFFERENCE"
    bo.object = cutter
    bo.solver = "EXACT"
    bpy.context.view_layer.update()
    return bpy.data.meshes.new_from_object(target.evaluated_get(bpy.context.evaluated_depsgraph_get()))


def _glass_mat(name):
    """Warm firelight: hot in the middle, deep orange at the edges, so it stays orange at 32 px."""
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    n, l = m.node_tree.nodes, m.node_tree.links
    n.remove(n["Principled BSDF"])
    grad = n.new("ShaderNodeTexGradient")
    grad.gradient_type = "SPHERICAL"
    mp = n.new("ShaderNodeMapping")
    mp.inputs["Scale"].default_value = (1.4, 1.4, 1.4)
    l.new(n.new("ShaderNodeTexCoord").outputs["Object"], mp.inputs["Vector"])
    l.new(mp.outputs[0], grad.inputs["Vector"])
    ramp = n.new("ShaderNodeValToRGB")
    ramp.color_ramp.elements[0].color = (1.0, 0.16, 0.012, 1)
    ramp.color_ramp.elements[1].color = (1.0, 0.42, 0.05, 1)
    l.new(grad.outputs["Color"], ramp.inputs["Fac"])
    em = n.new("ShaderNodeEmission")
    em.inputs["Strength"].default_value = 1.0
    l.new(ramp.outputs["Color"], em.inputs["Color"])
    l.new(em.outputs[0], n["Material Output"].inputs["Surface"])
    return m


def _dressed_opening(mason, to_wall, proud, z0, half, tall, ring, depth, jamb, block):
    """Jamb stones and an arch with a proud keystone around an opening; to_wall maps (x, z) onto the wall."""
    spring = z0 + tall
    for side in (-1, 1):
        z, n = z0, max(2, round(tall / jamb))
        for i in range(n):
            top = z0 + tall * (i + 1) / n
            a, b = side * half, side * (half + ring)
            lo, hi = (a, b) if side > 0 else (b, a)
            mason.stone((to_wall(lo, z + 0.006), to_wall(hi, z + 0.006), to_wall(hi, top - 0.006), to_wall(lo, top - 0.006)), proud(0.022), depth, nd=3)
            z = top
    n = max(5, 2 * round(math.pi * (half + ring / 2) / block / 2) + 1)
    for i in range(n):
        a0, a1 = math.pi - (i + 0.03) / n * math.pi, math.pi - (i + 0.97) / n * math.pi
        key = i == n // 2
        ro = half + ring * (1.28 if key else 1.0)
        ri = half - (0.012 if key else 0)
        c = [(ri * math.cos(a0), spring + ri * math.sin(a0)), (ri * math.cos(a1), spring + ri * math.sin(a1)),
             (ro * math.cos(a1), spring + ro * math.sin(a1)), (ro * math.cos(a0), spring + ro * math.sin(a0))]
        mason.stone(tuple(to_wall(*q) for q in c), proud(0.05 if key else 0.03), depth, nd=3)


def _window(mason, parts, prefix, th, z0, half, tall, wall, glass, iron, light):
    """Dressed arch and jambs around a lit opening. Returns what the wall must clear: (cutter, th, z_lo, z_hi, half width, inner half width, spring)."""
    spring, ring = z0 + tall, 0.17
    r0 = wall(z0)

    def to_wall(x, z):
        return (th + math.asin(max(-0.95, min(0.95, x / wall(z)))), z)

    def proud(extra):
        return lambda t, z: wall(z) + extra

    # sill
    sill = 0.13
    mason.stone((to_wall(-half - ring - 0.05, z0 - sill), to_wall(half + ring + 0.05, z0 - sill), to_wall(half + ring + 0.05, z0 - 0.006), to_wall(-half - ring - 0.05, z0 - 0.006)), proud(0.06), 0.32, nd=3)
    _dressed_opening(mason, to_wall, proud, z0, half, tall, ring, 0.3, 0.21, 0.17)
    # what the wall stones must clear
    cutter = _prism(prefix + "cut", th, _arch_outline(half + ring - 0.005, z0 - 0.02, spring), r0 - 0.75, r0 + 0.7)
    cutter.hide_render = True
    # firelit glass, set back in the wall, with an iron cross
    pane = _prism(prefix + "pane", th, _arch_outline(half + 0.07, z0 - 0.05, spring), r0 - 0.27, r0 - 0.25, glass)
    centre = Vector(_on_tower(th, spring - 0.05, r0 - 0.26))
    pane.data.transform(Matrix.Translation(-centre))
    pane.location = centre
    pane["window_light"] = True
    parts.append(pane)
    bar = half * 0.075
    parts.append(box(prefix + "bar_v", _on_tower(th, z0 + (tall + half) / 2, r0 - 0.15), (bar, bar, tall + half + 0.1), iron, 0.004, (0, 0, th)))
    parts.append(box(prefix + "bar_h", _on_tower(th, spring - tall * 0.05, r0 - 0.15), (2 * half + 0.1, bar, bar), iron, 0.004, (0, 0, th)))
    # the fire: an area light inside the arch washes the reveals, a lamp just outside spills on the wall around it
    def lamp(name, kind, r, z, energy):
        bpy.ops.object.light_add(type=kind, location=_on_tower(th, z, r))
        o = bpy.context.object
        o.name = prefix + name
        o.data.energy = energy
        o.data.color = WARM
        o.visible_camera = False
        o["window_light"] = True
        parts.append(o)
        return o

    fire = lamp("fire", "AREA", r0 - 0.2, z0 + (tall + half) / 2, light * 0.3)
    fire.data.shape = "RECTANGLE"
    fire.data.size, fire.data.size_y = 2 * half, tall + half
    fire.rotation_euler = Vector(_on_tower(th, 0, 1)).to_track_quat("-Z", "Z").to_euler()
    lamp("spill", "POINT", r0 + 0.34, spring - 0.1, light * 0.7).data.shadow_soft_size = 0.16
    return cutter, th, z0 - 0.02, spring + half + ring, half + ring, half, spring


def _shaft(mason, wall, z, high, radius, openings):
    """A round wall, course by course, its stones going around the openings."""
    r_wall = lambda t, zz: wall(zz)

    def hits(a, b, z0, z1):
        """The cutter of an opening this stone overlaps; None if it is clear of all of them."""
        for cutter, th, lo, hi, half, _, _ in openings:
            m = (a + b) / 2 - th
            m = (m + math.pi) % math.tau - math.pi
            if z1 > lo and z0 < hi and abs(m) - (b - a) / 2 < half / radius + 0.2:
                return cutter
        return None

    prev, heights = (), []
    while z < high - 0.12:
        h = mason.rnd.uniform(0.19, 0.3)
        if high - z - h < 0.17:
            h = high - z
        heights.append((z, z + h))
        z += h
    arcs = (0.5, 0.62, 0.44, 0.7)
    for i, (z0, z1) in enumerate(heights):
        cut = []

        def divert(a, b, zz0, zz1):
            c = hits(a, b, zz0, zz1)
            if c:
                cut.append((a, b, c))
            return bool(c)

        prev = mason.course(z0, z1, r_wall, 0.25, arcs[i % len(arcs)], prev, skip=divert)
        r = wall((z0 + z1) / 2)
        for a, b, c in cut:
            mason.carved(mason.rect(a, b, z0, z1, r), r_wall, 0.25, c)


def _battlements(mason, top_r, z_m, merlons):
    """Merlons with cap stones around the top of a round wall."""
    r_par = lambda t, z: top_r
    width = 0.54 * math.tau / merlons
    for i in range(merlons):
        c = i / merlons * math.tau
        a, b = c - width / 2, c + width / 2
        split = c + mason.rnd.uniform(-0.25, 0.25) * width
        mason.stone(mason.rect(a, split, z_m, z_m + 0.24, top_r), r_par, 0.26, nd=3)
        mason.stone(mason.rect(split, b, z_m, z_m + 0.24, top_r), r_par, 0.26, nd=3)
        mason.stone(mason.rect(a, b, z_m + 0.24, z_m + 0.48, top_r), r_par, 0.26, nd=3)
        over = 0.03 / top_r
        mason.stone(mason.rect(a - over, b + over, z_m + 0.48, z_m + 0.62, top_r + 0.03), lambda t, z: top_r + 0.03, 0.32, nd=3)


def stone_materials(prefix, stone=STONE, floor=FLOOR):
    """The tower's materials: {stone, floor, mortar, iron, glass}."""
    return {
        "stone": textured_mat(prefix + "stone", pack_maps(stone), 0.42, 0.024, tint=0.68, vary=0.24, contrast=-0.2, sat=0.26, grain=0.9, cavity=0.45, normal=0.3, rough=(0.5, 0.95), stain=0.3),
        "floor": textured_mat(prefix + "floor", pack_maps(floor), 1.7, 0.024, tint=0.62, sat=0.3, cavity=0.5, normal=1.3, rough=(0.45, 0.95)),
        "mortar": mat(prefix + "mortar", (0.035, 0.032, 0.028, 1), rough=0.95),
        "iron": mat(prefix + "iron", (0.015, 0.015, 0.016, 1), metallic=0.6, rough=0.55),
        "glass": _glass_mat(prefix + "glass"),
    }


def stone_tower(prefix, loc=(0, 0, 0), s=1.0, stone=STONE, floor=FLOOR, seed=11, facing=-0.26, merlons=8, light=45.0):
    """The Tirith mark: a round stone castle tower on a circular base. Needs the texture packs.

    Height about 3.85*s above loc.z, base 4.2*s wide and 0.42*s below loc.z. Every stone is its own
    mesh island, bevelled and displaced at render. The main window faces th=facing (0 is -Y); a merlon
    faces th=0, so a camera on -Y sees clean gaps between the battlements. Returns the objects: parent
    them to one empty to move or spin the tower.
    """
    mats = stone_materials(prefix, stone, floor)
    stone_m, floor_m, mortar, iron, glass = (mats[k] for k in ("stone", "floor", "mortar", "iron", "glass"))

    mason = Mason(seed)
    high, radius, talus = 2.55, 1.0, 0.62

    def wall(z):
        if z < talus:
            return radius + 0.2 * (1 - z / talus) ** 1.5
        return radius - 0.05 * (z - talus) / (high - talus)

    parts = []
    # the base: two stepped rings of stone around a tiled floor
    prev = mason.course(-0.42, -0.2, lambda t, z: 2.08, 0.5, 0.72)
    mason.course(-0.2, 0.05, lambda t, z: 1.93, 0.28, 0.6, prev, nd=3)
    # openings first, the wall stones go around them
    openings = [_window(mason, parts, prefix + "w0_", facing, 1.28, 0.3, 0.56, wall, glass, iron, light),
                _window(mason, parts, prefix + "w1_", facing + math.pi, 1.55, 0.19, 0.34, wall, glass, iron, light * 0.5)]

    _shaft(mason, wall, -0.02, high, radius, openings)
    # corbels carrying the overhanging parapet, two tiers each
    top_r, par_h = 1.3, high + 0.24
    n_cor = 22
    for i in range(n_cor):
        t0, t1 = (i + 0.2) / n_cor * math.tau, (i + 0.8) / n_cor * math.tau
        mason.stone(mason.rect(t0, t1, high - 0.02, high + 0.11, 1.1), lambda t, z: 1.1, 0.24, nd=3)
        mason.stone(mason.rect(t0, t1, high + 0.11, par_h, 1.2), lambda t, z: 1.2, 0.3, nd=3)
    # parapet wall, then the merlons with cap stones
    r_par = lambda t, z: top_r
    prev = mason.course(par_h, par_h + 0.2, r_par, 0.26, 0.55, nd=3)
    prev = mason.course(par_h + 0.2, par_h + 0.38, r_par, 0.26, 0.48, prev, nd=3)
    _battlements(mason, top_r, par_h + 0.38, merlons)
    parts.append(mason.finish(prefix + "stones", stone_m))

    # dark core behind the joints: mortar, the flare under the parapet and the roof deck
    prof = [(wall(z) - 0.06, z) for z in (-0.02, 0.15, 0.3, 0.45, 0.62, 1.2, 1.9, high)]
    prof += [(1.12, high + 0.2), (1.1, par_h + 0.08)]
    core = _lathe(prefix + "core", prof, mortar)
    for cutter, th, z_lo, _, _, inner_half, spring in openings:
        hole = _prism(prefix + "hole", th, _arch_outline(inner_half + 0.03, z_lo, spring), wall(z_lo) - 0.3, wall(z_lo) + 0.7)
        new = _carved(core, hole)
        core.modifiers.clear()
        old, core.data = core.data, new
        bpy.data.meshes.remove(old)
        bpy.data.objects.remove(hole)
    parts.append(core)
    parts.append(_lathe(prefix + "base_core", [(1.88, -0.42), (1.88, 0.0)], mortar))
    for opening in openings:
        bpy.data.objects.remove(opening[0])

    # tiled floor, a polar grid dense enough to displace
    bm = bmesh.new()
    uv = bm.loops.layers.uv.new("UVMap")
    rings, seg = 28, 240
    grid = [[bm.verts.new(_on_tower(i / seg * math.tau, 0.012, 0.9 + (1.92 - 0.9) * k / rings)) for i in range(seg)] for k in range(rings + 1)]
    for k in range(rings):
        for i in range(seg):
            f = bm.faces.new((grid[k][i], grid[k][(i + 1) % seg], grid[k + 1][(i + 1) % seg], grid[k + 1][i]))
            for loop in f.loops:
                loop[uv].uv = (loop.vert.co.x, loop.vert.co.y)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces[:])
    me = bpy.data.meshes.new(prefix + "floor")
    bm.to_mesh(me)
    bm.free()
    fl = bpy.data.objects.new(prefix + "floor", me)
    bpy.context.collection.objects.link(fl)
    me.materials.append(floor_m)
    me.shade_smooth()
    dice(fl)
    parts.append(fl)

    # place and scale as a whole: UVs and displacement scale with the object
    for o in parts:
        o.scale = (s, s, s)
        o.location = Vector(loc) + Vector(o.location) * s
        if o.type == "LIGHT":
            o.data.energy *= s * s
            o.data.shadow_soft_size *= s
            if o.data.type == "AREA":
                o.data.size *= s
                o.data.size_y *= s
    return parts


def unlit_by(light, objs):
    """Keep a light off these objects: the cool rim is for the silhouette, on the ground it reads as moonlit snow."""
    skip = bpy.data.collections.new(light.name + "_skip")
    for o in objs:
        skip.objects.link(o)
    for item in skip.collection_objects:
        item.light_linking.link_state = "EXCLUDE"
    light.light_linking.receiver_collection = skip


def _hazed(m, color, start, end):
    """Fade a material into the air with distance, as far hills do at dusk."""
    n, l = m.node_tree.nodes, m.node_tree.links
    out = n["Material Output"]
    shader = out.inputs["Surface"].links[0].from_socket
    far = n.new("ShaderNodeMapRange")
    far.inputs["From Min"].default_value = start
    far.inputs["From Max"].default_value = end
    l.new(n.new("ShaderNodeCameraData").outputs["View Distance"], far.inputs["Value"])
    air = n.new("ShaderNodeEmission")
    air.inputs["Color"].default_value = (*color, 1)
    mix = n.new("ShaderNodeMixShader")
    l.new(far.outputs[0], mix.inputs[0])
    l.new(shader, mix.inputs[1])
    l.new(air.outputs[0], mix.inputs[2])
    l.new(mix.outputs[0], out.inputs["Surface"])


def terrain(name, pack, z=0.0, flat=4.0, R=170.0, tile=2.0, haze=(0.05, 0.055, 0.115), bumps=0.5, hills=20.0):
    """Rolling ground on a polar grid, flat around the centre, rising to far hills that fade into the air."""
    m = textured_mat(name, pack_maps(pack), tile, 0.07, tint=0.2, sat=0.6, cavity=0.5, normal=0.8, rough=(0.6, 1.0), stain=0.4)
    _hazed(m, haze, 14.0, 110.0)
    rings, seg = 240, 900
    rr = [0.4 * (R / 0.4) ** (k / rings) for k in range(rings + 1)]

    def height(x, y, r):
        near = min(1.0, max(0.0, (r - flat) / (flat * 1.6)))
        far = max(0.0, (r - 34.0) / (R - 34.0))
        low = noise.fractal(Vector((x * 0.16, y * 0.16, 3.0)), 0.8, 2.1, 6) * bumps * near * near
        peak = (noise.ridged_multi_fractal(Vector((x * 0.014, y * 0.014, 7.0)), 1.1, 2.0, 3, 1.0, 2.0) - 0.45) * hills * far ** 0.7
        return z + low + max(peak, 0.0)

    verts = []
    for r in rr:
        for i in range(seg):
            t = i / seg * math.tau
            x, y = r * math.cos(t), r * math.sin(t)
            verts.append((x, y, height(x, y, r)))
    faces = [(k * seg + i, k * seg + (i + 1) % seg, (k + 1) * seg + (i + 1) % seg, (k + 1) * seg + i) for k in range(rings) for i in range(seg)]
    me = bpy.data.meshes.new(name)
    me.from_pydata(verts, [], faces)
    uv = me.uv_layers.new(name="UVMap")
    for loop in me.loops:
        v = verts[loop.vertex_index]
        uv.data[loop.index].uv = (v[0], v[1])
    me.shade_smooth()
    o = bpy.data.objects.new(name, me)
    bpy.context.collection.objects.link(o)
    me.materials.append(m)
    dice(o).adaptive_pixel_size = 3.0
    return o


def boulder(name, loc, size, m, seed=1):
    """A weathered rock half sunk in the ground."""
    rnd = random.Random(seed)
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=4, radius=1, location=loc, rotation=(rnd.random(), rnd.random(), rnd.random() * math.tau))
    o = bpy.context.object
    o.name = name
    o.scale = (size * rnd.uniform(0.9, 1.3), size * rnd.uniform(0.8, 1.1), size * rnd.uniform(0.55, 0.8))
    off = Vector((rnd.random(), rnd.random(), rnd.random())) * 40
    uv = o.data.uv_layers.new(name="UVMap")
    for loop in o.data.loops:
        co = o.data.vertices[loop.vertex_index].co
        uv.data[loop.index].uv = (co.x * size + co.z * size + off.x, co.y * size + co.z * size * 0.5 + off.y)
    for v in o.data.vertices:
        v.co *= 1 + 0.42 * noise.fractal(v.co * 1.1 + off, 0.9, 2.1, 4)
    o.data.materials.append(m)
    o.data.shade_smooth()
    dice(o)
    return o


def glow_mat(name, color, strength):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    n, l = m.node_tree.nodes, m.node_tree.links
    n.remove(n["Principled BSDF"])
    em = n.new("ShaderNodeEmission")
    em.inputs["Color"].default_value = (*color, 1)
    em.inputs["Strength"].default_value = strength
    l.new(em.outputs[0], n["Material Output"].inputs["Surface"])
    return m


def _gem(name, loc, h, m, seed, rot=(0, 0, 0), sides=6):
    """A faceted bipyramid, flat shaded, a little irregular."""
    rnd = random.Random(seed)
    w, waist = h * 0.27, h * 0.14
    bm = bmesh.new()
    ring = []
    for i in range(sides):
        t = (i + rnd.uniform(-0.12, 0.12)) / sides * math.tau
        rr = w * rnd.uniform(0.82, 1.12)
        ring.append(bm.verts.new((rr * math.cos(t), rr * math.sin(t), waist + rnd.uniform(-0.03, 0.03) * h)))
    top = bm.verts.new((rnd.uniform(-0.03, 0.03) * h, rnd.uniform(-0.03, 0.03) * h, h * 0.62))
    low = bm.verts.new((rnd.uniform(-0.03, 0.03) * h, rnd.uniform(-0.03, 0.03) * h, -h * 0.38))
    for i in range(sides):
        a, b = ring[i], ring[(i + 1) % sides]
        bm.faces.new((a, b, top))
        bm.faces.new((b, a, low))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces[:])
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    o = bpy.data.objects.new(name, me)
    bpy.context.collection.objects.link(o)
    o.location, o.rotation_euler = loc, rot
    me.materials.append(m)
    return o


def crystal(name, loc, h, color=(0.015, 0.3, 1.0), power=20.0, seed=1):
    """An agent: a floating crystal lit from within, with its own light on what is around it. Returns its solid parts."""
    rnd = random.Random(seed)
    shell = bpy.data.materials.new(name + "_shell")
    shell.use_nodes = True
    b = shell.node_tree.nodes["Principled BSDF"]
    b.inputs["Base Color"].default_value = (*color, 1)
    b.inputs["Roughness"].default_value = 0.22
    b.inputs["Transmission Weight"].default_value = 0.3
    b.inputs["Specular IOR Level"].default_value = 0.3
    b.inputs["IOR"].default_value = 1.6
    b.inputs["Emission Color"].default_value = (*color, 1)
    b.inputs["Emission Strength"].default_value = 0.8
    tilt = (rnd.uniform(-0.16, 0.16), rnd.uniform(-0.16, 0.16), rnd.random() * math.tau)
    core = glow_mat(name + "_core", (0.05, 0.5, 1.0), 2.0)
    parts = [_gem(name, loc, h, shell, seed, tilt), _gem(name + "_inner", loc, h * 0.42, core, seed + 1, tilt)]
    bpy.ops.object.light_add(type="POINT", location=loc)
    lamp = bpy.context.object
    lamp.name = name + "_lamp"
    lamp.data.energy = power
    lamp.data.color = color
    lamp.data.shadow_soft_size = h * 0.3
    lamp.visible_camera = False
    return parts


def beam(name, a, b, m, lift=0.0, r=0.011):
    """A line of light from a to b, bowed upward by lift."""
    a, b = Vector(a), Vector(b)
    c = bpy.data.curves.new(name, "CURVE")
    c.dimensions = "3D"
    c.bevel_depth = r
    c.bevel_resolution = 2
    sp = c.splines.new("POLY")
    pts = 32
    sp.points.add(pts - 1)
    for i in range(pts):
        t = i / (pts - 1)
        p = a.lerp(b, t) + Vector((0, 0, lift * 4 * t * (1 - t)))
        sp.points[i].co = (*p, 1)
    o = bpy.data.objects.new(name, c)
    bpy.context.collection.objects.link(o)
    c.materials.append(m)
    return o


def stone_turret(prefix, loc, mats, radius=0.92, high=3.8, merlons=7, seed=3, facing=0.0, light=14.0):
    """A small round tower of the same stones as the mark, with a lit arrow window. Base at loc.z."""
    mason = Mason(seed)
    parts = []
    talus = 0.5

    def wall(z):
        return radius + 0.17 * max(0.0, 1 - z / talus) ** 1.5 - 0.04 * max(0.0, z - talus) / high

    openings = [_window(mason, parts, prefix + "w_", facing, high * 0.56, 0.16, 0.38, wall, mats["glass"], mats["iron"], light)]
    _shaft(mason, wall, -0.05, high, radius, openings)
    top_r, par_h = radius + 0.2, high + 0.14
    n_cor = 16
    for i in range(n_cor):
        t0, t1 = (i + 0.2) / n_cor * math.tau, (i + 0.8) / n_cor * math.tau
        mason.stone(mason.rect(t0, t1, high - 0.02, par_h, radius + 0.1), lambda t, z: radius + 0.1, 0.24, nd=3)
    prev = mason.course(par_h, par_h + 0.2, lambda t, z: top_r, 0.26, 0.5, nd=3)
    mason.course(par_h + 0.2, par_h + 0.36, lambda t, z: top_r, 0.26, 0.44, prev, nd=3)
    _battlements(mason, top_r, par_h + 0.36, merlons)
    parts.append(mason.finish(prefix + "stones", mats["stone"]))
    core = _lathe(prefix + "core", [(wall(z) - 0.06, z) for z in (-0.05, 0.2, 0.5, 1.5, high)] + [(radius + 0.04, par_h), (radius + 0.04, par_h + 0.1)], mats["mortar"])
    for cutter, th, z_lo, _, _, inner_half, spring in openings:
        hole = _prism(prefix + "hole", th, _arch_outline(inner_half + 0.03, z_lo, spring), wall(z_lo) - 0.3, wall(z_lo) + 0.7)
        new = _carved(core, hole)
        core.modifiers.clear()
        old, core.data = core.data, new
        bpy.data.meshes.remove(old)
        bpy.data.objects.remove(hole)
        bpy.data.objects.remove(cutter)
    parts.append(core)
    for o in parts:
        o.location = Vector(loc) + Vector(o.location)
    return parts


def stone_wall(prefix, mats, half_w=2.2, high=3.5, thick=0.7, opening=1.2, tall=1.75, seed=5):
    """A gate wall in the XZ plane, facing -Y, with a dressed arch in the middle and battlements on top."""
    R = 40.0
    mason = Mason(seed, place=lambda th, z, r: (th * R, R - r, z))
    flat = lambda t, z: R
    ring, spring = 0.34, tall
    to_wall = lambda x, z: (x / R, z)
    proud = lambda extra: (lambda t, z: R + extra)
    _dressed_opening(mason, to_wall, proud, 0.0, opening, tall, ring, thick, 0.42, 0.34)
    cutter = _prism(prefix + "cut", 0.0, _arch_outline(opening + ring - 0.005, -0.02, spring), -thick - 0.2, 0.3)
    z, prev = 0.0, []
    while z < high - 0.05:
        h = mason.rnd.uniform(0.26, 0.38)
        if high - z - h < 0.22:
            h = high - z
        x, joints = -half_w, []
        while x < half_w - 0.01:
            w = mason.rnd.uniform(0.42, 0.78)
            if half_w - x - w < 0.3:
                w = half_w - x
            joints.append(x)
            a, b = x, x + w
            corners = mason.rect(a / R, b / R, z, z + h, R)
            if z < spring + opening + ring and b > -opening - ring and a < opening + ring:
                if z >= spring or a < -opening - ring or b > opening + ring:
                    mason.carved(corners, flat, thick, cutter)
            else:
                mason.stone(corners, flat, thick, nd=2)
            x += w
        z += h
    for i in range(7):
        c = -half_w + (i + 0.5) / 7 * 2 * half_w
        for z0, z1 in ((high, high + 0.26), (high + 0.26, high + 0.5)):
            mason.stone(mason.rect((c - 0.2) / R, (c + 0.2) / R, z0, z1, R), flat, thick, nd=3)
        mason.stone(mason.rect((c - 0.23) / R, (c + 0.23) / R, high + 0.5, high + 0.62, R), lambda t, z: R + 0.03, thick + 0.06, nd=3)
    stones = mason.finish(prefix + "stones", mats["stone"])
    bpy.data.objects.remove(cutter)
    body = box(prefix + "body", (0, thick / 2 + 0.03, high / 2), (2 * half_w, thick - 0.08, high), mats["mortar"], 0)
    hole = _prism(prefix + "hole", 0.0, _arch_outline(opening + 0.03, -0.1, spring), -thick - 0.2, 0.3)
    new = _carved(body, hole)
    body.modifiers.clear()
    old, body.data = body.data, new
    bpy.data.meshes.remove(old)
    bpy.data.objects.remove(hole)
    return [stones, body]


def look(cam_loc, target, lens=70):
    bpy.ops.object.camera_add(location=cam_loc)
    c = bpy.context.object
    c.data.lens = lens
    d = Vector(target) - Vector(cam_loc)
    c.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.camera = c
    return c


def area(loc, target, energy, size, color=(1, 1, 1)):
    bpy.ops.object.light_add(type="AREA", location=loc)
    l = bpy.context.object
    l.data.energy = energy
    l.data.size = size
    l.data.color = color
    d = Vector(target) - Vector(loc)
    l.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
    return l


def world(color=(0.9, 0.94, 1), strength=0.6):
    w = bpy.context.scene.world or bpy.data.worlds.new("w")
    bpy.context.scene.world = w
    w.use_nodes = True
    bg = w.node_tree.nodes["Background"]
    bg.inputs[0].default_value = (*color, 1)
    bg.inputs[1].default_value = strength


def shadow_floor(z=0.0, size=40):
    """A floor that only shows shadows, so the PNG stays transparent. Call it after stone_tower."""
    bpy.ops.mesh.primitive_plane_add(size=size, location=(0, 0, z))
    f = bpy.context.object
    f.is_shadow_catcher = True
    # window light the tower blocks from the floor would otherwise show up as a grey floor
    skip = bpy.data.collections.new("no_window_light")
    skip.objects.link(f)
    skip.collection_objects[0].light_linking.link_state = "EXCLUDE"
    for o in bpy.data.objects:
        if o.get("window_light") or o.get("no_floor"):
            o.light_linking.receiver_collection = skip
    return f


def _compositing(bloom):
    """Bloom on what is brighter than the stone can be (the window, the crystals). Keeps the render's alpha."""
    s = bpy.context.scene
    g = bpy.data.node_groups.new("comp", "CompositorNodeTree")
    g.interface.new_socket("Image", in_out="OUTPUT", socket_type="NodeSocketColor")
    n, l = g.nodes, g.links
    rl = n.new("CompositorNodeRLayers")
    gl = n.new("CompositorNodeGlare")
    gl.inputs["Type"].default_value = "Bloom"
    gl.inputs["Quality"].default_value = "High"
    gl.inputs["Threshold"].default_value = 0.85
    gl.inputs["Smoothness"].default_value = 0.4
    gl.inputs["Strength"].default_value = bloom
    gl.inputs["Size"].default_value = 0.55
    alpha = n.new("CompositorNodeSetAlpha")
    alpha.inputs["Type"].default_value = "Replace Alpha"
    l.new(rl.outputs["Image"], gl.inputs["Image"])
    l.new(gl.outputs["Image"], alpha.inputs["Image"])
    l.new(rl.outputs["Alpha"], alpha.inputs["Alpha"])
    l.new(alpha.outputs[0], n.new("NodeGroupOutput").inputs[0])
    s.compositing_node_group = g
    s.render.use_compositing = True


def in_frame(objs, margin=0.015):
    """Stop the render if an object pokes out of the frame: the composition must never be cut."""
    s = bpy.context.scene
    bpy.context.view_layer.update()
    aspect = s.render.resolution_x / s.render.resolution_y
    for o in objs:
        for corner in o.bound_box:
            p = world_to_camera_view(s, s.camera, o.matrix_world @ Vector(corner))
            if not (margin <= p.x <= 1 - margin and margin / aspect <= p.y <= 1 - margin / aspect):
                print("ERROR: %s is cut by the frame at (%.3f, %.3f)." % (o.name, p.x, p.y), file=sys.stderr)
                sys.exit(3)


def setup(w, h, samples=96, transparent=True, out=None, fmt="PNG", bloom=0.7, exposure=0.0, look_name="AgX - Medium High Contrast"):
    s = bpy.context.scene
    s.render.engine = "CYCLES"
    s.cycles.samples = samples
    s.cycles.use_adaptive_sampling = True
    s.cycles.adaptive_threshold = 0.01
    s.cycles.use_denoising = True
    s.cycles.denoiser = "OPENIMAGEDENOISE"
    s.cycles.denoising_input_passes = "RGB_ALBEDO_NORMAL"
    s.cycles.denoising_prefilter = "ACCURATE"
    s.cycles.sample_clamp_indirect = 6.0
    try:
        prefs = bpy.context.preferences.addons["cycles"].preferences
        prefs.compute_device_type = "METAL"
        prefs.get_devices()
        for d in prefs.devices:
            d.use = True
        s.cycles.device = "GPU"
    except Exception as e:  # CPU fallback
        print("gpu unavailable:", e)
    s.render.resolution_x, s.render.resolution_y = w, h
    s.render.resolution_percentage = 100
    s.render.film_transparent = transparent
    s.render.image_settings.file_format = fmt
    s.render.image_settings.color_mode = "RGBA" if transparent else "RGB"
    s.view_settings.view_transform = "AgX"
    s.view_settings.look = look_name
    s.view_settings.exposure = exposure
    if bloom:
        _compositing(bloom)
    if out:
        s.render.filepath = out

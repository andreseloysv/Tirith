# ---------------------------------------------------------------- a bot's mouth: an open grin with its tongue, or a closed smile
import bpy
from mathutils import Vector, Matrix
from . import materials
from .eyes import lens_frame
from .geometry import link, prism, cubic, quad, smooth_sphere


def cut_mouth(body, cut):
    mod = body.modifiers.new('mouth', 'BOOLEAN')
    mod.operation = 'DIFFERENCE'
    mod.solver = 'EXACT'
    mod.object = cut
    try:
        mod.material_mode = 'TRANSFER'
    except Exception:
        pass

def tongue(sh, tag, root, cut, mx, my, half):
    """The tongue, low in the mouth."""
    tu, tw = sh.box(mx, my + half * 0.80)
    p, n = sh.front(tu, tw), sh.normal(tu, tw)
    me = smooth_sphere(f'{tag}-tongue')
    tg = link(bpy.data.objects.new(f'{tag}-tongue', me), root)
    tg.data.materials.append(materials.TONGUE)
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

def grin(sh, tag, root, body, mx, my, half):
    top = quad((mx + half, my), (mx, my + half * 0.16), (mx - half, my), 18)
    bottom = cubic((mx - half, my), (mx - half, my + half * 1.35), (mx + half, my + half * 1.35), (mx + half, my), 36)
    poly = [sh.box(x, y) for x, y in bottom + top]
    cut = link(bpy.data.objects.new(f'{tag}-mouthcut', prism(sh, poly, f'{tag}-mouthcut', back=0.24)), root)
    cut.data.materials.append(materials.MOUTH)
    cut.hide_render = True
    cut.display_type = 'WIRE'
    cut_mouth(body, cut)
    tongue(sh, tag, root, cut, mx, my, half)

def smile(sh, tag, root, mx, my, half):
    """A closed smile: a dark tube that follows the face."""
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
    sm_.data.materials.append(materials.EYE)

def omouth(sh, tag, root, mx, my, half):
    """A small vertical oval 'o' mouth, bright red, set into the surface like an eye."""
    u, w = sh.box(mx, my + half * 0.22)
    rx, ry = 2.6 * sh.px / sh.rmax, 3.8 * sh.px / sh.rmax
    p, n = sh.front(u, w), sh.normal(u, w)
    mo = link(bpy.data.objects.new(f'{tag}-omouth', smooth_sphere(f'{tag}-omouth')), root)
    mo.data.materials.append(materials.OMOUTH)
    dep = 0.30 * rx
    mo.matrix_basis = Matrix.Translation(p - n * dep * 0.55) @ lens_frame(n) @ Matrix.Diagonal((rx, dep, ry, 1))

def add_mouth(sh, e, spec, tag, root, body):
    mx, my, half = (e[0] + e[1]) / 2, e[2] + e[4] + 6, (e[1] - e[0]) * 0.5
    if spec['mood'] == 'grin':
        grin(sh, tag, root, body, mx, my, half)
    elif spec['mood'] == 'o':
        omouth(sh, tag, root, mx, my, half)
    else:
        smile(sh, tag, root, mx, my, half)

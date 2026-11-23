# ---------------------------------------------------------------- a bot's eyes: glossy dark lenses set into the surface
import bpy, bmesh
from mathutils import Vector, Matrix
from . import materials
from .geometry import link, smooth_sphere


def lens_frame(n):
    """The rotation that turns a lens to face along -n, its x axis kept level."""
    yv = -n
    xv = Vector((1, 0, 0))
    zv = xv.cross(yv).normalized()
    if zv.z < 0:
        zv = -zv
    xv = yv.cross(zv).normalized()
    return Matrix((xv, yv, zv)).transposed().to_4x4()

def catchlight(sh, tag, i, root, u, w, rx, ry, dep, rot):
    cu, cw = u - 0.28 * rx, w + 0.42 * ry
    cp = sh.front(cu, cw) + sh.normal(cu, cw) * dep * 0.30
    me2 = bpy.data.meshes.new(f'{tag}-catch{i}')
    bm2 = bmesh.new()
    bmesh.ops.create_icosphere(bm2, subdivisions=2, radius=1.0)
    bm2.to_mesh(me2)
    bm2.free()
    dot = link(bpy.data.objects.new(f'{tag}-catch{i}', me2), root)
    dot.data.materials.append(materials.SPARK)
    dot.matrix_basis = Matrix.Translation(cp) @ rot @ Matrix.Diagonal((0.17 * rx, 0.05 * rx, 0.17 * rx, 1))

def add_eyes(sh, e, tag, root, mat=None):
    for i, x in enumerate((e[0], e[1])):
        u, w = sh.box(x, e[2])
        rx, ry = 0.84 * e[3] * sh.px / sh.rmax, 0.84 * e[4] * sh.px / sh.rmax
        p, n = sh.front(u, w), sh.normal(u, w)
        me = smooth_sphere(f'{tag}-eye{i}')
        eye = link(bpy.data.objects.new(f'{tag}-eye{i}', me), root)
        eye.data.materials.append(mat or materials.EYE)
        rot = lens_frame(n)
        dep = 0.30 * rx
        eye.matrix_basis = Matrix.Translation(p - n * dep * 0.55) @ rot @ Matrix.Diagonal((rx, dep, ry, 1))
        catchlight(sh, tag, i, root, u, w, rx, ry, dep, rot)

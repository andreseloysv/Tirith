# ---------------------------------------------------------------- a bot: linking objects, the mouth's cutter, curves
import bpy, bmesh


def link(obj, parent=None):
    bpy.context.scene.collection.objects.link(obj)
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

def smooth_sphere(name):
    """A smooth-shaded UV sphere mesh of radius 1 (the eyes and the tongue)."""
    me = bpy.data.meshes.new(name)
    bm = bmesh.new()
    bmesh.ops.create_uvsphere(bm, u_segments=32, v_segments=16, radius=1.0)
    for f in bm.faces:
        f.smooth = True
    bm.to_mesh(me)
    bm.free()
    return me

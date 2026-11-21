# ---------------------------------------------------------------- shapes
import math, os
import bpy, bmesh
import numpy as np
from mathutils import Vector
from .opts import HERE


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

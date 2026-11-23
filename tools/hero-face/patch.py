"""The model is modelled where its eyes and mouth are painted: raised lenses whose rims fold back over the face. The
page draws eyes and mouths of their own that blink and move, so each of those places is cut out and patched: rings of
new points shrink from the hole's edge to its middle, and their heights are a thin-plate spline through the face around
the hole (the surface of least bending that passes by it), so the patch meets the cushion without a crease."""
import numpy as np

RINGS = 5  # rings of new points inside a hole, its middle last


def edge_loop(tris):
    """The hole's edge in order, from the edges only one triangle has, or None when it is not one simple loop."""
    count = {}
    for t in tris:
        for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
            count[(min(a, b), max(a, b))] = count.get((min(a, b), max(a, b)), 0) + 1
    edges = [(a, b) for t in tris for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])) if count[(min(a, b), max(a, b))] == 1]
    after = dict(edges)
    loop = [edges[0][0]]
    while after.get(loop[-1], loop[0]) != loop[0] and len(loop) <= len(edges):
        loop.append(after[loop[-1]])
    return loop[::-1] if len(loop) == len(edges) == len(after) else None  # reversed, it runs as the patch's triangles do


def web(points, loop, mid):
    """New points in rings from the edge to the middle, and the patch's triangles (the edge's own points first)."""
    edge = points[loop, :2] - mid[:2]
    pts, tris, m, n0 = [], [], len(loop), len(points)
    ring = lambda k: [loop[i] for i in range(m)] if k == 0 else [n0 + (k - 1) * m + i for i in range(m)]  # noqa: E731
    for k in range(1, RINGS):
        pts += [np.r_[mid[:2] + e * (1 - k / RINGS), 0.0] for e in edge]
    centre = n0 + len(pts)
    pts.append(np.r_[mid[:2], 0.0])
    for k in range(RINGS - 1):
        o, i = ring(k), ring(k + 1)
        for j in range(m):
            tris += [(o[j], o[(j + 1) % m], i[(j + 1) % m]), (o[j], i[(j + 1) % m], i[j])]
    last = ring(RINGS - 1)
    tris += [(last[j], last[(j + 1) % m], centre) for j in range(m)]
    return np.array(pts), np.array(tris)


def spline(xy, z, at, smooth=1e-4):
    """A thin-plate spline through heights `z` at `xy`, evaluated at `at`."""
    def kernel(a, b):
        d = np.linalg.norm(a[:, None] - b[None], axis=2)
        return np.where(d > 0, d * d * np.log(np.maximum(d, 1e-12)), 0)
    n, ends = len(xy), np.c_[np.ones(len(xy)), xy]
    system = np.block([[kernel(xy, xy) + smooth * np.eye(n), ends], [ends.T, np.zeros((3, 3))]])
    w = np.linalg.solve(system, np.r_[z, np.zeros(3)])
    return kernel(at, xy) @ w[:n] + np.c_[np.ones(len(at)), at] @ w[n:]


def patch(points, tris, spots, cut=1.35):
    """`points` and `tris` with each spot ((middle, half width, half height)) cut out within `cut` times its size and
    patched; the points no triangle uses any more are dropped."""
    for mid, rx, ry in spots:
        d = np.sqrt(((points[:, 0] - mid[0]) / rx) ** 2 + ((points[:, 1] - mid[1]) / ry) ** 2)
        for grow in (1, 1.06, 1.12, 1.18, 1.24):  # a little wider until the hole's edge is one simple loop
            kept = tris[~((d < cut * grow) & (points[:, 2] > 0))[tris].any(1)]
            loop = edge_loop(kept)
            if loop:
                break
        pts, more = web(points, loop, mid)
        around = np.unique(kept[(d[kept] < cut * 2.2).all(1)])  # the face around the hole, its edge with it
        around = around[points[around, 2] > 0]
        pts[:, 2] = spline(points[around, :2], points[around, 2], pts[:, :2])
        points, tris = np.vstack([points, pts]), np.vstack([kept, more])
    used = np.zeros(len(points), bool)
    used[tris.ravel()] = True
    index = np.cumsum(used) - 1
    return points[used], index[tris]

"""Where the star's painted eyes and mouth are: the dark patches of its texture, found there and carried onto the mesh
through its texture coordinates (glTF's v runs down the picture)."""
import numpy as np

STEP = 4  # the picture is searched at a quarter of its size


def patches(picture, least=25):
    """The dark patches of the picture, biggest first: (count, middle x, middle y, width, height) in its pixels."""
    a = np.asarray(picture).astype(np.float64)[::STEP, ::STEP]
    dark = (a @ np.array([0.299, 0.587, 0.114]) < 95) & (a[..., 0] > 15)  # maroon paint, not the black background
    h, w = dark.shape
    seen, out = np.zeros_like(dark), []
    for y0, x0 in zip(*np.nonzero(dark)):
        if seen[y0, x0]:
            continue
        stack, cells = [(y0, x0)], []
        seen[y0, x0] = True
        while stack:
            y, x = stack.pop()
            cells.append((y, x))
            for yy in (y - 1, y, y + 1):
                for xx in (x - 1, x, x + 1):
                    if 0 <= yy < h and 0 <= xx < w and dark[yy, xx] and not seen[yy, xx]:
                        seen[yy, xx] = True
                        stack.append((yy, xx))
        if len(cells) >= least:
            ys, xs = np.array(cells).T * STEP
            out.append((len(cells), (xs.min() + xs.max()) / 2, (ys.min() + ys.max()) / 2, np.ptp(xs) + STEP, np.ptp(ys) + STEP))
    return sorted(out, reverse=True)


def on_mesh(points, uv, tris, at):
    """The point of the mesh whose texture coordinate is `at`, or None when no triangle holds it."""
    a, b, c = uv[tris[:, 0]], uv[tris[:, 1]], uv[tris[:, 2]]
    v0, v1, v2 = b - a, c - a, np.asarray(at)[None, :] - a
    d = v0[:, 0] * v1[:, 1] - v1[:, 0] * v0[:, 1]
    with np.errstate(divide='ignore', invalid='ignore'):
        s = (v2[:, 0] * v1[:, 1] - v1[:, 0] * v2[:, 1]) / d
        t = (v0[:, 0] * v2[:, 1] - v2[:, 0] * v0[:, 1]) / d
    inside = np.isfinite(s) & (s >= 0) & (t >= 0) & (s + t <= 1)
    if not inside.any():
        return None
    k = np.nonzero(inside)[0][0]
    return (1 - s[k] - t[k]) * points[tris[k, 0]] + s[k] * points[tris[k, 1]] + t[k] * points[tris[k, 2]]


def features(points, uv, tris, picture):
    """The two eyes (left, right) and the mouth, each as (middle, half width, half height) on the mesh."""
    w, h = picture.size
    found = []
    for _, x, y, pw, ph in patches(picture):
        mid = on_mesh(points, uv, tris, (x / w, y / h))
        side = on_mesh(points, uv, tris, ((x - pw / 2 + STEP) / w, y / h))
        low = on_mesh(points, uv, tris, (x / w, (y + ph / 2 - STEP) / h))
        if mid is not None and side is not None and low is not None and mid[2] > 0:  # on the front
            found.append((mid, abs(mid[0] - side[0]), abs(mid[1] - low[1])))
    eyes, mouth = sorted(found[:2], key=lambda f: f[0][0]), found[2]
    return eyes, mouth

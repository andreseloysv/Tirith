"""Read the one mesh of a binary glTF (.glb): its points, texture coordinates, triangles and base colour picture."""
import io
import json
import struct

import numpy as np
from PIL import Image

KINDS = {5126: np.float32, 5125: np.uint32, 5123: np.uint16, 5121: np.uint8}
WIDTH = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4}


def read(path):
    """(points (n, 3), uv (n, 2), triangles (m, 3), base colour image) of the first primitive of the first mesh."""
    data = open(path, 'rb').read()
    size, _ = struct.unpack('<II', data[12:20])
    doc = json.loads(data[20:20 + size])
    start = 20 + size + 8
    blob = data[start:start + struct.unpack('<I', data[20 + size:24 + size])[0]]

    def view(i):
        v = doc['bufferViews'][i]
        if v.get('byteStride'):
            raise ValueError('interleaved buffers are not read here')
        return v.get('byteOffset', 0)

    def accessor(i):
        a = doc['accessors'][i]
        kind, n = KINDS[a['componentType']], WIDTH[a['type']]
        out = np.frombuffer(blob, dtype=kind, count=a['count'] * n, offset=view(a['bufferView']) + a.get('byteOffset', 0))
        return out.reshape(a['count'], n) if n > 1 else out

    prim = doc['meshes'][0]['primitives'][0]
    points = accessor(prim['attributes']['POSITION']).astype(np.float64)
    uv = accessor(prim['attributes']['TEXCOORD_0']).astype(np.float64)
    tris = accessor(prim['indices']).astype(np.int64).reshape(-1, 3)
    pbr = doc['materials'][prim['material']]['pbrMetallicRoughness']
    image = doc['images'][doc['textures'][pbr['baseColorTexture']['index']]['source']]
    v = doc['bufferViews'][image['bufferView']]
    picture = Image.open(io.BytesIO(blob[v.get('byteOffset', 0):v.get('byteOffset', 0) + v['byteLength']])).convert('RGB')
    return points, uv, tris, picture

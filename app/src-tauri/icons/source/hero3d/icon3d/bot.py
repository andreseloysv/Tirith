# ---------------------------------------------------------------- a bot: body, eyes and mouth, placed in front of the camera
import math
import bpy
from .opts import opt
from .cast import EYES
from .stage import at
from .shapes import shape
from .body import body_material
from .geometry import link
from . import materials
from .eyes import add_eyes
from .mouth import add_mouth


def make_bot(spec, tag):
    sh = shape(spec['shape'])
    root = link(bpy.data.objects.new(f'{tag}', None))
    body = link(bpy.data.objects.new(f'{tag}-body', sh.mesh(f'{tag}-body')), root)
    body.data.materials.append(body_material(f'{tag}-mat', spec['color'], float(opt.get('maingain', '0.8')) if tag == 'main' else float(opt.get('flygain', '0.95')), spec.get('core'), spec.get('edge')))
    e = EYES[spec['shape']]
    add_eyes(sh, e, tag, root, materials.REDEYE if spec.get('redeyes') else None)
    add_mouth(sh, e, spec, tag, root, body)
    dg = bpy.context.evaluated_depsgraph_get()
    if body.modifiers and len(body.evaluated_get(dg).data.vertices) < 100:
        body.modifiers.clear()
        print('dropped the mouth cut of', tag)
    for ch in root.children:
        ch.pass_index = 1
    return root, place(root, spec, tag)

def place(root, spec, tag):
    """Size, screen position, a turn towards the camera plus its own."""
    pos = at(spec['sx'], spec['sy'], spec['d'])
    face_cam = math.atan2(pos.x, pos.y)
    root.location = pos
    root.rotation_euler = (math.radians(spec.get('pitch', 0)), math.radians(spec.get('roll', 0)), face_cam + math.radians(spec.get('yaw', 0)))
    root.scale = (spec['r'] * (1.0 if tag == 'main' else float(opt.get('flyscale', '1.25'))),) * 3
    return pos

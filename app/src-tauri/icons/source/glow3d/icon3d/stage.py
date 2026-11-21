# ---------------------------------------------------------------- scene
import math
import bpy
from mathutils import Vector
from .opts import opt, OUT, SIZE, SAMPLES, FSTOP

TAN = None  # half the view's width over the distance, set by camera()


def scene_setup():
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
    return scene


def camera(scene):
    """The camera at the origin looking along +Y."""
    global TAN
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
    return cam, cam_data


def at(sx, sy, d):
    u, v = sx - 0.5, 0.5 - sy
    return Vector((2 * u * TAN * d, d, 2 * v * TAN * d))

# the steps of build.py, in the order they always ran: scene, bots, trails, dust, sky, lights, focus, compositor, render
import os
import bpy
from .opts import opt, OUT
from .cast import MAIN, FLYERS
from . import stage, materials
from .bot import make_bot
from .hero import make_hero
from .trails import make_trails
from .dust import make_dust
from .sky import make_sky
from .lights import make_lights
from .compositor import comp


def run():
    scene = stage.scene_setup()
    cam, cam_data = stage.camera(scene)
    materials.shared()
    main, main_pos = make_hero(MAIN) if opt.get('hero') else make_bot(MAIN, 'main')
    flyer_pos = []
    for i, f in enumerate(FLYERS):
        _, pos = make_bot(f, f'fly{i}')
        flyer_pos.append((pos, f))
    make_trails(flyer_pos)
    dust = make_dust()
    make_sky(dust)
    make_lights(main_pos)
    cam_data.dof.focus_distance = (main_pos - cam.location).length - 0.6  # focus on the big bot
    comp(scene)
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    if opt.get('blend'):
        bpy.ops.wm.save_as_mainfile(filepath=opt['blend'])
    bpy.ops.render.render(write_still=True)
    print('WROTE', OUT)

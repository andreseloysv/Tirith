# ---------------------------------------------------------------- the bots (our shapes; eyes [xL, xR, y, rx, ry] in the 100 box)
from .opts import opt

EYES = {
    'round': [36, 62, 44, 7, 10], 'dome': [37, 63, 52, 7, 10], 'cloud': [40, 64, 48, 7, 9.5], 'tile': [38, 62, 46, 7, 9.5],
    'star': [42, 58, 46, 4.5, 7], 'flower': [39, 61, 48, 6, 9], 'drop': [37, 61, 46, 7, 9.5], 'pill': [37, 63, 49, 7.5, 10.5],
    'tri': [42, 58, 58, 5, 7.5], 'hex': [38, 62, 46, 7, 9.5], 'burst': [39, 61, 47, 6, 9], 'blob': [36, 62, 45, 7.5, 10.5],
}
MAIN = dict(shape=opt.get('main', 'blob'), color='#ffd23f', core=opt.get('maincore', '#fff6a0'), edge=opt.get('mainedge', '#e9a823'), sx=0.60, sy=0.67, d=float(opt.get('maind', '3.0')), r=1.05, yaw=float(opt.get('yaw', '6')), pitch=float(opt.get('pitch', '3')), roll=float(opt.get('roll', '-10')), mood='grin')
FLYERS = [  # screen position from the top left, distance, radius
    dict(shape='cloud', color='#3ee08f', sx=0.15, sy=0.63, d=6.0, r=0.50, mood='grin', yaw=-14, roll=8),
    dict(shape='hex', color='#4aa8ff', sx=0.86, sy=0.25, d=7.0, r=0.46, mood='grin', yaw=12, roll=-10),
    dict(shape='flower', color='#ff5fd2', sx=0.20, sy=0.30, d=9.0, r=0.40, mood='smile', yaw=-10, roll=12),
    dict(shape='drop', color='#ff8a3d', sx=0.43, sy=0.27, d=8.5, r=0.40, mood='grin', yaw=-4, roll=-6),
    dict(shape='burst', color='#2fd3c4', sx=0.58, sy=0.10, d=11.0, r=0.40, mood='smile', yaw=6, roll=-8),
    dict(shape='tri', color='#ff5d73', sx=0.33, sy=0.10, d=13.0, r=0.30, mood='smile', yaw=-6, roll=10),
    dict(shape='star', color='#9b6bff', sx=0.17, sy=0.09, d=15.0, r=0.26, mood='smile', yaw=-8, roll=14),
    dict(shape='pill', color='#f4f1ff', sx=0.46, sy=0.05, d=17.0, r=0.24, mood='smile', yaw=0, roll=-4),
]
MEET = dict(sx=0.40, sy=0.55, d=9.5)  # where the trails start, behind the big bot


def lin(hexs):  # sRGB hex -> linear rgb
    c = [int(hexs[i:i + 2], 16) / 255 for i in (1, 3, 5)]
    return tuple(((v + 0.055) / 1.055) ** 2.4 if v > 0.04045 else v / 12.92 for v in c)

def mixc(a, b, t):
    return tuple(x + (y - x) * t for x, y in zip(a, b))

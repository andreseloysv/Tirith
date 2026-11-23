// One live 3D bot: its scene (camera, body in one flat colour, eyes), the little canvas in its element that the
// shared sheet (sheet.js) copies its picture into, and its pointer.
import * as THREE from '../../../../vendor/three/three.module.min.js';
import { SHAPES3D } from '../shapes3d.js';
import { HERO3D } from './hero.js';
import { still } from './clock.js';
import { FREEZE } from './config.js';
import { bodyGeometry3 } from './geometry.js';
import { bodyMaterial3 } from './materials.js';
import { camera3, eyes3 } from './bot3d-parts.js';
import { lookOf } from './moods3d.js';
import { personaOf } from './persona.js';
import { frozenPose, stillPose, livePose } from './bot3d-pose.js';
import { Acts } from './schedule.js';
import { bots3, seen3 } from './registry.js';

const ALL3 = { ...SHAPES3D, hero: HERO3D }; /* the inflated outlines, and the hero with its own body */

export class Bot3D {
  constructor(el, name, color, mood, frozen, lively = false) {
    this.el = el; this.name = ALL3[name] ? name : 'round'; this.sh = ALL3[this.name]; this.color = color;
    this.reduced = !!(still && still.matches);
    this.quiet = !!frozen || this.reduced; // drawn once; moves only under the pointer (never, with reduced motion)
    this.drawnOnce = false; this.dirty = true; this.settled = true; this.pointerOver = false; this.hovered = false;
    this.scene = new THREE.Scene();
    this.cam = camera3(this.sh);
    this.rig = new THREE.Group(); this.scene.add(this.rig);
    this.rig.rotation.order = 'ZXY'; // turned, then nodded, then rolled as one sees it
    this.lively = lively; this.acting = false; this.fast = false;
    this.round = THREE.MathUtils.clamp((Math.min(...this.sh.r) - 0.68) / 0.3, 0, 1); // how far it can be a ball (a spin)
    this.persona = personaOf(color);
    this.acts = new Acts({ lively, key: `${this.name}:${color}`, persona: this.persona });
    const seg = el.offsetWidth > 90 ? 128 : 64;
    this.body = new THREE.Mesh(this.sh.mesh ? this.sh.mesh() : bodyGeometry3(this.name, seg), bodyMaterial3(this.color)); this.rig.add(this.body);
    this.eyes = eyes3(); this.look = lookOf(mood);
    this.rig.add(...this.eyes);
    this.yaw = 0; this.pitch = 0; this.ty = 0; this.tp = 0; this.nextLook = 0; this.nextBlink = 1 + Math.random() * 3; this.blinkAt = -1;
    this.phase = Math.random() * 10; this.setMood(mood);
    this.mountCanvas(el);
  }
  /** Its own little canvas in the element, the element's observer and its pointer. */
  mountCanvas(el) {
    this.cv = el.querySelector('canvas.botcv');
    if (!this.cv) { this.cv = document.createElement('canvas'); this.cv.className = 'botcv'; el.append(this.cv); }
    this.cv.width = this.cv.height = 0; // not 300 by default: the sheet sizes its corner from the element until it is drawn
    this.ctx = this.cv.getContext('2d');
    this.visible = true; el.bot3d = this; if (seen3) seen3.observe(el); this.id = Bot3D.n = (Bot3D.n || 0) + 1;
    this.onEnter = () => { this.pointerOver = true; };
    this.onLeave = () => { this.pointerOver = false; };
    el.addEventListener('pointerenter', this.onEnter);
    el.addEventListener('pointerleave', this.onLeave);
  }
  /** A new state; the change itself is played (the article's lifecycle): it acknowledges new work, cheers when done. */
  setMood(m) {
    if (this.mood && m !== this.mood && !this.quiet) this.acts.react(this.mood, m, performance.now() / 1000);
    this.mood = m; this.dirty = true;
  }
  update(t, dt, pointer) {
    const r = this.el.getBoundingClientRect();
    if (FREEZE) { frozenPose(this); return r; }
    const near = !this.reduced && pointer && pointer.x >= r.left && pointer.x <= r.right && pointer.y >= r.top && pointer.y <= r.bottom; // only the bot under the pointer
    this.hovered = !!near;
    if (this.quiet && !near) stillPose(this, dt);
    else livePose(this, t, dt, pointer, near, r);
    return r;
  }
  dispose() {
    bots3.delete(this);
    if (seen3) seen3.unobserve(this.el);
    this.el.removeEventListener('pointerenter', this.onEnter);
    this.el.removeEventListener('pointerleave', this.onLeave);
    if (this.el.bot3d === this) this.el.bot3d = null;
    this.body.material.dispose(); /* the ink is shared by every bot; the painted eyes are this bot's own */
    for (const m of this.eyes) m.geometry.dispose();
  }
}

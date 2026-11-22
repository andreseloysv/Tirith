// The 2D face's animator: the float, breathing, glances, blinks, the ten acts (acts.js, as the 3D face plays them)
// and, asleep, its own small ones and the z's, in the mood of its state. It computes the motion; `paint` (face.js)
// writes the float and the halo to the face's layers, features.js the eyes and the mouth.
import { R, TAU, MOOD, NUM, ACTS } from './motion.js';
import { still, start, stop } from './clock.js';
import { Acts } from './schedule.js';
import { partsOf } from './features.js';
import { happen, actPose, floatPose, moveEyes, moveZs } from './animator-steps.js';

export class Animator {
  /** `play`: how it plays its acts ({lively, key}, schedule.js). */
  constructor(root, state, play) {
    this.mood = { ...MOOD[state] || MOOD.idle };
    this.target = MOOD[state] || MOOD.idle;
    this.state = state;
    /* incommensurate periods and random phases: this face's float never repeats and never matches another's */
    this.w = [R(3.1, 3.9), R(5.3, 7.7), R(6.5, 9.5), R(7, 11.5), R(3.6, 4.9)].map((p) => TAU / p);
    this.p = this.w.map(() => R(0, TAU));
    this.t = null; this.clock = 0;
    this.visible = true;
    this.eye = { x: 0, y: 0, fx: 0, fy: 0, tx: 0, ty: 0, at: 0, d: 1, free: 1 };
    this.next = { blink: R(0.5, 4), look: R(0.3, 2), act: R(1.5, 7), z: R(0.5, 2) };
    this.blinks = []; this.act = null; this.zs = [];
    this.acts = new Acts(play);
    this.bind(root, state);
    start(this, root);
  }
  /** New markup (another state, shape or colour): the same motion goes on in it. */
  bind(root, state) {
    this.root = root;
    this.float = root.querySelector('.ffloat');
    this.halo = root.querySelector('.fhalo'); this.zg = root.querySelector('.zs');
    this.zs.forEach((z) => z.el.remove()); this.zs = [];
    this.parts = partsOf(root);
    if (state !== this.state) {
      if (this.state === 'sleep' && state !== 'sleep') { this.act = { ...ACTS.wake(), at: this.clock }; this.blinks.push(this.clock + 0.05); }
      this.state = state; this.target = MOOD[state] || MOOD.idle;
      this.next.act = this.clock + R(1.5, 4);
    }
  }
  stop() { stop(this); }
  step(t) {
    const dt = this.t == null ? 0 : Math.min(0.1, t - this.t);
    this.t = t;
    if (!this.visible || (still && still.matches)) return;
    const m = this.mood, g = this.target, c = (this.clock += dt * m.pace);
    for (const k of NUM) m[k] += (g[k] - m[k]) * Math.min(1, dt * 2.5); /* a new state blends in */
    const now = this.clock;
    happen(this, now, g);
    const pose = this.acts.step(t, this.state); /* in real seconds, whatever the mood's pace */
    this.paint(floatPose(this, m, c, actPose(this, now), pose));
    moveEyes(this, now, pose);
    moveZs(this, now);
  }
  paint() { /* the layers' writer: face.js */ }
  spawnZ(now) {
    const el = document.createElementNS('http://www.w3.org/2000/svg', 'text');
    el.textContent = 'z'; el.setAttribute('class', 'z'); el.setAttribute('font-size', R(11, 16).toFixed(1)); el.setAttribute('opacity', '0');
    this.zg.appendChild(el);
    this.zs.push({ el, at: now, d: R(2.2, 3.2), x: R(72, 82), y: R(16, 24) });
  }
}

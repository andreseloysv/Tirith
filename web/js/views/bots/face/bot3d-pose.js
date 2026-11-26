// How one 3D bot (bot3d.js) moves each frame: frozen for still pictures, a still face easing back to front off
// the pointer, or alive — turned whole towards the pointer over it, else floating, breathing, looking around,
// blinking and playing its acts (acts.js).
import * as THREE from '../../../../vendor/three/three.module.min.js';
import { FREEZE, PX } from './config.js';
import { showFace } from './bot3d-face.js';

const FEET = 0.8; /* squash and stretch keep the feet where they are: this far below the middle */

/** The rest pose: upright, its own size and the eyes open. */
function rest(b) {
  b.rig.position.set(0, 0, 0); b.rig.scale.set(1, 1, 1);
  b.body.material.uniforms.ball.value = 0;
  b.acting = b.fast = false;
  showFace(b);
}

/** No motion, no blink, the given angle: the same picture every time. */
export function frozenPose(b) {
  if (FREEZE.mood && b.mood !== FREEZE.mood) b.setMood(FREEZE.mood);
  b.rig.rotation.set(FREEZE.pitch, FREEZE.yaw, 0);
  rest(b);
}

/** A still face off the pointer: ease back to front and hold the pose. */
export function stillPose(b, dt) {
  const easeBack = Math.min(1, dt * 3.5), eyesBack = Math.min(1, dt * 10);
  b.yaw += (0 - b.yaw) * easeBack; b.pitch += (0 - b.pitch) * easeBack;
  b.hover -= b.hover * eyesBack; b.gx -= b.gx * eyesBack; b.gy -= b.gy * eyesBack; /* its eyes come back from the pointer */
  b.settled = Math.abs(b.yaw) < 0.002 && Math.abs(b.pitch) < 0.002 && b.hover < 0.01;
  if (b.settled) { b.yaw = 0; b.pitch = 0; b.hover = 0; }
  b.rig.rotation.set(b.pitch, b.yaw, 0);
  rest(b);
}

/** Where it wants to look: towards the pointer over it, to the front while it plays an act, or now and then
    somewhere of its own. */
function aim(b, t, pointer, near, r) {
  const lim = b.mood === 'sleep' ? 0.3 : 1;
  if (near) { // its eyes look straight at the pointer (the owner's «que los ojos miren directo al mouse»); it turns a little
    const cx = r.left + r.width / 2, cy = r.top + r.height / 2;
    let dx = (pointer.x - cx) / (r.width / 2), dy = (pointer.y - cy) / (r.height / 2);
    const len = Math.hypot(dx, dy); if (len > 1) { dx /= len; dy /= len; }
    b.tgx = dx; b.tgy = dy; b.ty = dx * 0.16 * lim; b.tp = dy * 0.12 * lim; /* a small turn: the eyes do the looking */
    b.nextLook = t + 0.8;
  } else if (b.acting) {
    b.tgx = b.tgy = 0; b.ty = 0; b.tp = 0; b.nextLook = t + 0.4;
  } else if (t > b.nextLook) { // on its own: it looks around now and then
    b.tgx = b.tgy = 0;
    const busy = b.mood === 'work' && !b.lively;
    b.ty = (Math.random() * 2 - 1) * (busy ? 0.45 : 0.32) * lim; b.tp = (Math.random() * 2 - 1) * 0.18 * lim;
    b.nextLook = t + (busy ? 0.6 + Math.random() : 1.6 + Math.random() * 2.6) * b.persona.look; /* its colour's curiosity */
  }
}

/** How open its own blinking leaves the eyes at `t`. */
function blinkAt(b, t) {
  if (t > b.nextBlink) { b.blinkAt = t; b.nextBlink = t + (2.2 + Math.random() * 4.5) * b.persona.blink; }
  return b.blinkAt < 0 ? 1 : Math.min(1, Math.abs(t - b.blinkAt - 0.08) / 0.08);
}

export function livePose(b, t, dt, pointer, near, r) {
  b.settled = false;
  const act = b.acts.step(t, b.mood, near);
  b.acting = !!b.acts.playing; b.fast = b.acts.fast;
  aim(b, t, pointer, near, r);
  const quick = Math.min(1, dt * 10); /* the eyes reach the pointer in about 0.3 s (the article's 280 to 340 ms), and come back as fast */
  b.hover += ((near ? 1 : 0) - b.hover) * quick; b.gx += ((near ? b.tgx : 0) - b.gx) * quick; b.gy += ((near ? b.tgy : 0) - b.gy) * quick;
  const easeAmt = Math.min(1, dt * (near ? 9 : 3.5));
  b.yaw += (b.ty - b.yaw) * easeAmt; b.pitch += (b.tp - b.pitch) * easeAmt;
  const { tempo, energy: e } = b.persona; /* its colour's temperament: how fast and how much it moves */
  const sp = (b.mood === 'work' ? 2.2 : b.mood === 'sleep' ? 0.45 : 1) * tempo, ph = b.phase + t * sp;
  b.rig.rotation.set(b.pitch + 0.04 * e * Math.sin(ph * 0.9) + act.pitch, b.yaw + act.yaw, 0.05 * e * Math.sin(ph * 0.63) - act.roll);
  const unit = PX / b.sh.rmax; /* a box unit in the model's */
  const moved = -act.y * unit - (1 - act.sy) * FEET;
  b.rig.position.set(act.x * unit, 0.05 * e * Math.sin(ph * 1.1) + moved, 0);
  const br = 1 + 0.025 * e * Math.sin(ph * 1.7);
  const puff = (act.puff - 1) * b.round; /* as far as its shape is round: a star stays a star */
  b.rig.scale.set((2 - br) * act.sx, br * act.sy, 1 + puff);
  b.body.material.uniforms.ball.value = Math.min(1, puff / 0.6); /* puffed it is a ball, not a cushion */
  showFace(b, act, blinkAt(b, t), dt);
}

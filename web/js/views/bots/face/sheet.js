// One GPU renderer off screen draws every bot that is due into its own corner of one sheet; then each bot copies
// its corner into its own little canvas, which scrolls with the page. One sheet means one snapshot per frame.
// Bots under 64 px draw 15 times a second, the rest (and the one under the pointer) 30.
import * as THREE from '../../../../vendor/three/three.module.min.js';
import { MARGIN, CAP, FREEZE } from './config.js';
import { Bot3D } from './bot3d.js';
import { bots3 } from './registry.js';

let gl3 = null, glFailed = false, sheetW = 0, sheetH = 0, raf3 = 0, pointer3 = null;
let pack3 = { key: '', spots: new Map() };

/** Where the pointer is (only the bot under it turns), and a resize redraws every bot. */
function listen() {
  addEventListener('pointermove', (ev) => { pointer3 = { x: ev.clientX, y: ev.clientY, t: performance.now() / 1000 }; }, { passive: true });
  addEventListener('pointerdown', (ev) => { pointer3 = { x: ev.clientX, y: ev.clientY, t: performance.now() / 1000 }; }, { passive: true });
  document.documentElement.addEventListener('pointerleave', () => { pointer3 = null; bots3.forEach((b) => { b.pointerOver = false; }); });
  addEventListener('scroll', () => { pointer3 = null; }, { passive: true });
  addEventListener('resize', () => bots3.forEach((b) => { b.dirty = true; b.drawnOnce = false; }));
}
export function ensureGL() {
  if (gl3) return true;
  if (glFailed || typeof document === 'undefined') return false;
  try {
    const canvas = document.createElement('canvas');
    const renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true });
    renderer.setPixelRatio(1);
    renderer.setClearColor(0x000000, 0);
    renderer.autoClear = false;
    renderer.setScissorTest(true);
    gl3 = { canvas, renderer };
    listen();
    return true;
  } catch (err) { glFailed = true; return false; }
}
function place3(all) { // a fixed square on the sheet for every visible bot; recomputed only when that set or a size changes
  const key = all.map((d) => d.b.id + ':' + d.px).join(',');
  if (key === pack3.key) return pack3.spots;
  const order = [...all].sort((a, b) => b.px - a.px), spots = new Map();
  const W = 2048; let x = 0, y = 0, row = 0;
  for (const d of order) { if (x + d.px > W) { x = 0; y += row; row = 0; } spots.set(d.b, [x, y]); x += d.px; row = Math.max(row, d.px); }
  const H = y + row, w = Math.min(W, order.reduce((a, d) => a + d.px, 0));
  const nw = Math.max(256, Math.ceil(w / 256) * 256), nh = Math.max(256, Math.ceil(H / 256) * 256);
  if (nw !== sheetW || nh !== sheetH) { sheetW = nw; sheetH = nh; gl3.renderer.setSize(nw, nh, false); }
  pack3 = { key, spots };
  return spots;
}
/** The bots on screen ({b, px}) and, of them, the ones due for a new picture, moved to this moment. */
function dueBots(now) {
  const dpr = Math.min(2, devicePixelRatio);
  const due = [], shown = [];
  for (const b of bots3) {
    if (!b.visible) continue; // off screen: nothing to draw
    shown.push({ b, px: b.cv.width || Math.round(b.el.offsetWidth * MARGIN * dpr) });
    const hoverActive = b.pointerOver && !b.reduced;
    if (FREEZE && b.drawnOnce && !b.dirty) continue; // frozen: the same picture, drawn once
    if (b.quiet && b.drawnOnce && !b.dirty && !hoverActive && b.settled) continue; // still: drawn once, moves only under the pointer
    // small bots in lists move a pixel or two: 15 pictures a second are enough; big ones and the one under the pointer get 30
    const every = b.hovered || hoverActive || b.el.offsetWidth >= 64 ? 1 / 30 : 1 / 15;
    if (now - (b.drawnAt || 0) < every - 0.004) continue;
    const dt = Math.min(0.1, now - (b.drawnAt || now)); b.drawnAt = now;
    const r = b.update(now, dt, pointer3);
    const px = Math.min(CAP, Math.round(r.width * MARGIN * dpr));
    if (b.cv.width !== px) b.cv.width = b.cv.height = px;
    due.push({ b, px });
  }
  return { due, shown };
}
/** Draw the due bots into their corners of the sheet, then copy each corner into its bot's canvas. */
function drawSheet(due, shown) {
  const { renderer, canvas: glCanvas } = gl3;
  const spots = place3(shown);
  renderer.setScissorTest(true);
  for (const { b, px } of due) {
    const [x, y] = spots.get(b);
    renderer.setViewport(x, y, px, px); renderer.setScissor(x, y, px, px); renderer.clear();
    renderer.render(b.scene, b.cam);
  }
  for (const { b, px } of due) {
    const [x, y] = spots.get(b);
    b.ctx.clearRect(0, 0, px, px);
    b.ctx.drawImage(glCanvas, x, sheetH - y - px, px, px, 0, 0, px, px);
    b.drawnOnce = true; b.dirty = false;
  }
}
function frame3() {
  if (!bots3.size) { raf3 = 0; return; }
  raf3 = requestAnimationFrame(frame3);
  if (document.hidden) return;
  const { due, shown } = dueBots(performance.now() / 1000);
  if (due.length) drawSheet(due, shown);
}
/** Put a live 3D bot in an element (Face uses it; the test page web/bots3d-test.html too).
 *  Returns the bot (setMood, dispose), or null without WebGL. */
export function attachBot3D(el, { shape = 'round', color = '#3b82f6', state = 'idle', still: frozen = false } = {}) {
  if (!el || !ensureGL()) return null;
  const b = new Bot3D(el, shape, color, state, frozen);
  bots3.add(b);
  if (!raf3) raf3 = requestAnimationFrame(frame3);
  return b;
}

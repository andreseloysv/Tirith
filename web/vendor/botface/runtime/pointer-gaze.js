// How a face meets the pointer in its hover zone (the owner: «si estoy en la zona de hover sus ojos tienen que
// seguir el puntero», then «on hover la figura se tiene que poner completamente de frente, vertical, en 0,0,0
// … y los ojos seguir al mouse»): in the zone the face leaves its own motion and stands facing front, upright and
// centred (no turn, lean, sway or glance: its state's own drawing, in the middle), and only its eyes move: they
// follow the pointer, to the very point while it is over the face and towards it around it. Out of the zone it
// eases back into its own motion. A face that follows the pointer everywhere (follow-pointer) moves its eyes after
// it at any distance and stands front only in its zone. The zone is the button or link the face sits in (a sidebar
// face's tile, the one that shows its card), or, when it sits in none or in a much larger one, the face's own box
// and a little around it. Boxes are read at most five times a second; with reduced motion nothing follows.
import { SHEET_FOR } from "./ink-drawings.js";

const LOOK = [38, 32]; /* head units the eyes go at most towards the pointer: across, and up or down (about the
                          owner's example: they reach near the rim) */
const EASE = 0.14;     /* seconds to get most of the way to front, and back */
const BOX_MS = 200;    /* how long the boxes on screen are trusted */
const ZONE = 'button, a[href], [role="button"]';
const REDUCE = typeof globalThis.matchMedia === "function"
  ? globalThis.matchMedia("(prefers-reduced-motion: reduce)")
  : { matches: false };

/* the face's box and its hover zone's box, as they are on screen now */
function boxes(engine, now) {
  if (engine.gazeBoxes && now - engine.gazeBoxesAt <= BOX_MS && now >= engine.gazeBoxesAt) return engine.gazeBoxes;
  const face = engine.svg.getBoundingClientRect(), host = engine.svg.getRootNode()?.host;
  const tile = host && host.closest(ZONE);
  const box = tile && tile.getBoundingClientRect();
  const fits = box && box.width * box.height <= 9 * Math.max(face.width * face.height, 1);
  const m = face.width * 0.25;
  const zone = fits ? box : { left: face.left - m, top: face.top - m, right: face.right + m, bottom: face.bottom + m };
  engine.gazeBoxes = { face, zone, tile: Boolean(fits) };
  engine.gazeBoxesAt = now;
  return engine.gazeBoxes;
}

/* what the pointer asks of the face now: stand front (in the zone), eyes after it, and where they go */
function aim(engine, config, now) {
  const p = engine.pointer, rest = { front: 0, gaze: 0, eyeX: 0, eyeY: 0 };
  if (!p.active || REDUCE.matches) return rest;
  const { face, zone, tile } = boxes(engine, now);
  const inside = p.clientX >= zone.left && p.clientX <= zone.right && p.clientY >= zone.top && p.clientY <= zone.bottom;
  if (!inside && !config.pointer) return rest;
  const r = Math.max(face.width, 1) / 2;
  const dx = p.clientX - (face.left + face.width / 2), dy = p.clientY - (face.top + face.height / 2);
  const n = Math.max(Math.hypot(dx, dy), r); /* over the face: the point itself; around it: its direction */
  return { front: inside && tile ? 1 : 0, gaze: 1, eyeX: LOOK[0] * (dx / n) * (config.flipX ? -1 : 1), eyeY: LOOK[1] * (dy / n) };
}

/* facing the pointer, the face shows its state's own drawing in the middle, and keeps it: no glances */
function centre(engine, config, now) {
  if (engine.state === "sleeping" || engine.state === "waking") return;
  const index = SHEET_FOR[engine.state] ?? config.expressionPool?.[0];
  if (index === undefined) return;
  engine.expressionNext = Math.max(engine.expressionNext, now + 800);
  if (engine.expressionIndex !== index) engine.setExpression(index, 9);
}

/** Eases the engine's `look` ({ front, gaze }) for this frame and sets the eyes' pointer targets. The renderer
    keeps 1 - front of the face's own turn, lean, sway and squash, and weighs the eyes' own glances by 1 - gaze. */
export function pointerLook(engine, config, now) {
  const want = aim(engine, config, now), look = engine.look || (engine.look = { front: 0, gaze: 0 });
  const k = 1 - Math.exp(-(engine.delta || 1 / 60) / EASE);
  look.front += (want.front - look.front) * k;
  look.gaze += (want.gaze - look.gaze) * k;
  engine.pointer.targetX = want.eyeX;
  engine.pointer.targetY = want.eyeY;
  if (want.front) centre(engine, config, now);
  return look;
}

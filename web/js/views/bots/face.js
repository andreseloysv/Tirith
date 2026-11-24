// The bots' faces: the vendored animated face (vendor/botface/, fetched on first use; the tag upgrades in
// place when it lands), running exactly as it runs — every live face free at the screen's rate, paused by
// itself off screen, its own hand on reduced motion. A bot's look is its shape (the face's own eighteen are
// the picker's; an older bot's shape is said as its nearest, face/face-map.js), its material (solid in its
// colour, a gradient, or glass) and its eye colour. `state` is what the bot is doing: one of the four of
// before or any of the face's own (face-map.js). `still` faces (the shape picker) draw one frame and hold
// it. The size travels as --fs, so the stylesheet's own .bot boxes still win where they apply, and the
// element fills its box (bots.css). The 3D bench (web/bots3d-test.html) keeps its attachBot3D and the
// motion under face/ that it shares; nothing else draws in 3D any more.
import { html, useRef, useEffect } from '../../../vendor/preact-htm.js';
import { faceShape, faceState, SHAPE_IDS } from './face/face-map.js';

export { COLORS } from './face/shapes2d.js';
export { attachBot3D } from './face/sheet.js';
/** The picker's shapes: the face's own eighteen, in its order. */
export const SHAPES = Object.fromEntries(SHAPE_IDS.map((id) => [id, id]));

let loading = null;
/** The face element's module, fetched once for the whole page. */
export function loadFace() { return loading || (loading = import('../../../vendor/botface/bot-face.js')); }

/** The picker's ring around the chosen face: one plain circle, which fits every outline the face can take. */
export const ringOf = () => 'M50 -6A56 56 0 1 1 50 106A56 56 0 1 1 50 -6Z';

/** A held face draws one frame of its present look and state, and keeps it. */
function useHeld(ref, held, deps) {
  useEffect(() => {
    if (!held || !ref.current) return undefined;
    let on = true;
    loadFace().then(() => customElements.whenDefined('bot-face')).then(() => { if (on && ref.current && ref.current.step) ref.current.step(); });
    return () => { on = false; };
  }, [held, ...deps]);
}

/** The element's attributes for one look; a material's own settings only travel with that material. */
function attrsOf({ shape, color, size, state, material, eye, gradient, gstart, gend, gangle, glass }) {
  const a = { state: faceState(state), shape: faceShape(shape), size, color, 'eye-color': eye, material };
  if (material === 'gradient') {
    if (gradient && gradient !== 'custom') a['gradient-preset'] = gradient;
    if (gstart) a['gradient-start'] = gstart;
    if (gend) a['gradient-end'] = gend;
    if (typeof gangle === 'number') a['gradient-angle'] = gangle;
  }
  if (material === 'rainbow-glass' && glass) a['glass-preset'] = glass;
  return a;
}

/** One bot's face. `lively`: beside what its bot is doing in a chat, it follows the pointer. */
export function Face({ shape = 'round', color = '#3b82f6', size = 64, state = 'idle', still: frozen = false, lively = false,
  material = 'solid', eye = '#161616', gradient = '', gstart = '', gend = '', gangle = null, glass = '' }) {
  const ref = useRef(null);
  useEffect(() => { loadFace(); }, []);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.toggleAttribute('paused', frozen);
    el.toggleAttribute('follow-pointer', lively && !frozen);
    el.setAttribute('decorative', '');
  }, [frozen, lively]);
  useHeld(ref, frozen, [state, shape, color, material, eye, gradient, gstart, gend, gangle, glass]);
  const a = attrsOf({ shape, color, size, state, material, eye, gradient, gstart, gend, gangle, glass });
  return html`<span class=${`bot face ${a.state}`} style=${`--fs:${size}px`} aria-hidden="true"><bot-face ref=${ref} ...${a}></bot-face></span>`;
}

/** The "new bot" mark: a dashed outline that turns slowly around a plus. */
export function NewFace({ size = 44 }) {
  return html`<svg class="bot newbot" width=${size} height=${size} viewBox="-12 -12 124 124" aria-hidden="true">
    <g class="ring" fill="none" stroke="currentColor" stroke-width="2.4" stroke-dasharray="6 6"><circle cx="50" cy="50" r="42" /></g>
    <path d="M50 32v36M32 50h36" stroke="currentColor" stroke-width="3" stroke-linecap="round" /></svg>`;
}

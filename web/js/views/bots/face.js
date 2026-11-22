// The bots' faces: a live 3D character in one of SHAPES and one of COLORS — the owner-approved preview of
// 2026-11-21 (~/.tirith/logo-source/bots3d/live_template.html), ported with its own code and numbers: the body
// inflated from the shape's outline r(theta) (shapes3d.js; P 2.35, depth 0.62), its glowing body shader, glossy
// eyes with a light dot, the halo and the mouths per state (idle and watch smile, work opens its mouth with its
// tongue, sleep shuts its eyes). `state` is what the bot is doing: 'work', 'watch', 'sleep' or 'idle'.
//
// One WebGL context draws every bot that is due into a fixed slot of one sheet; each bot copies its slot into its
// own little canvas inside its element, so scrolling never lags behind them. Bots under 64 px draw 15 times a
// second, the rest (one playing an act, at work in a chat or under the pointer too) 30 and a fast act at work 60;
// nothing draws off screen or while the page is hidden.
// Motion is the reference's: float, breathing, looks around, blinks, the halo pulsing — and with the pointer over
// it, only that bot turns whole towards the pointer, then falls back to its animation. On top of that motion ten
// acts (face/acts.js), now and then when idle, one after another in a `lively` face (beside what its bot is doing in
// a chat). `still` faces (the shape picker) draw once and move only under the pointer; with reduced motion every face
// holds still; without WebGL the 2D face shows instead: the 2D SVG face of before, with its
// own animator (its blurred layers rasterize once into will-change layers and move by transform/opacity only).
//
// The parts live in face/: sheet.js, bot3d.js, bot3d-parts.js, bot3d-pose.js, bot3d-face.js, geometry.js, materials.js
// and decals.js (the 3D bots), animator.js, animator-steps.js, features.js, motion.js, clock.js, markup.js and
// shapes2d.js (the 2D face), acts.js, keys.js, pose.js and schedule.js (the acts of both).
import { html, useRef, useEffect, useMemo } from '../../../vendor/preact-htm.js';
import { markup } from './face/markup.js';
import { Animator } from './face/animator.js';
import { ensureGL, attachBot3D } from './face/sheet.js';

export { SHAPES, COLORS } from './face/shapes2d.js';
export { attachBot3D };

/** The 2D face's animator, drawing into the face's layers. */
class Layers extends Animator {
  /* the float and the halo pulse move HTML layers only (CSS transform/opacity on will-change elements), never
     attributes inside the filtered SVGs: 124 viewBox units span the layer, so a unit is 100/124 % of it, and the
     pivot (50 90) is the layers' transform-origin in bots.css */
  paint({ x, y, rot, sx, sy, halo }) {
    if (this.float) this.float.style.transform = `translate(${(x * 100 / 124).toFixed(3)}%, ${(y * 100 / 124).toFixed(3)}%) rotate(${rot.toFixed(2)}deg) scale(${sx.toFixed(4)}, ${sy.toFixed(4)})`;
    if (this.halo) this.halo.style.opacity = halo;
  }
}

/** One bot's face, live in 3D (see the header); without WebGL, the 2D face as stacked layers. The size travels as
 *  the --fs custom property, so the stylesheet's own `.bot` sizes still win where they apply, as the svg's did. */
export function Face({ shape = 'round', color = '#3b82f6', size = 64, state = 'idle', still: frozen = false, lively = false }) {
  const ref = useRef(null), bot = useRef(null), anim = useRef(null);
  const ok3 = ensureGL();
  const parts = useMemo(() => (ok3 ? null : markup(shape, color, state)), [ok3, shape, color, state]);
  useEffect(() => {
    if (!ok3 || !ref.current) return undefined;
    bot.current = attachBot3D(ref.current, { shape, color, state, still: frozen, lively });
    return () => { if (bot.current) bot.current.dispose(); bot.current = null; };
  }, [ok3, shape, color, frozen, lively]);
  useEffect(() => { if (bot.current) bot.current.setMood(state); }, [state]);
  useEffect(() => {
    if (ok3 || frozen || !ref.current) return undefined;
    anim.current = new Layers(ref.current, state, { lively, key: `${shape}:${color}` });
    return () => anim.current.stop();
  }, [ok3, frozen, lively]);
  useEffect(() => { if (anim.current && ref.current) anim.current.bind(ref.current, state); }, [parts]);
  if (ok3) return html`<span ref=${ref} class=${`bot face face3d ${state}`} style=${`--fs:${size}px`} aria-hidden="true"><canvas class="botcv"></canvas></span>`;
  return html`<span ref=${ref} class=${`bot face ${state}`} style=${`--fs:${size}px`} aria-hidden="true"><span class="ffloat"><span class="fhalo"><svg class="fsvg" viewBox="-12 -12 124 124" dangerouslySetInnerHTML=${{ __html: parts.halo }} /></span><svg class="fsvg" viewBox="-12 -12 124 124" dangerouslySetInnerHTML=${{ __html: parts.body }} /><svg class="fsvg feyes" viewBox="-12 -12 124 124" dangerouslySetInnerHTML=${{ __html: parts.eyes }} /></span><svg class="fsvg fzs" viewBox="-12 -12 124 124" dangerouslySetInnerHTML=${{ __html: '<g class="zs"></g>' }} /></span>`;
}

/** The "new bot" mark: a dashed outline that turns slowly around a plus. */
export function NewFace({ size = 44 }) {
  return html`<svg class="bot newbot" width=${size} height=${size} viewBox="-12 -12 124 124" aria-hidden="true">
    <g class="ring" fill="none" stroke="currentColor" stroke-width="2.4" stroke-dasharray="6 6"><circle cx="50" cy="50" r="42" /></g>
    <path d="M50 32v36M32 50h36" stroke="currentColor" stroke-width="3" stroke-linecap="round" /></svg>`;
}

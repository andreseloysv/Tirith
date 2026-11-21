// The 2D faces' clock: one animation frame loop steps every live face at most 30 times a second, none off screen
// or while the page is hidden. `still` is the reduced-motion setting, which the 3D faces follow too.
export const still = typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)');

const live = new Set();
let pageHidden = typeof document !== 'undefined' && document.hidden;
if (typeof document !== 'undefined') document.addEventListener('visibilitychange', () => { pageHidden = document.hidden; });
let frame = 0, lastStep = 0;
function loop(now) {
  frame = 0;
  if (now - lastStep >= 1000 / 30) { /* at most 30 steps a second, however fast the screen refreshes */
    lastStep = now;
    if (!pageHidden) for (const a of live) a.step(now / 1000);
  }
  if (live.size) frame = requestAnimationFrame(loop);
}
const seen = typeof IntersectionObserver === 'function'
  ? new IntersectionObserver((list) => list.forEach((e) => { if (e.target.animator) e.target.animator.visible = e.isIntersecting; })) : null;

/** Animator `a` of the face in `root` moves from now on. */
export function start(a, root) {
  live.add(a);
  if (seen) { root.animator = a; seen.observe(root); }
  if (!frame) frame = requestAnimationFrame(loop);
}
export function stop(a) { live.delete(a); if (seen && a.root) seen.unobserve(a.root); }

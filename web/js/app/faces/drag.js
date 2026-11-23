// Dragging a face up or down the sidebar to choose its place: the face follows the pointer, the others slide to
// make room, and on release the new order is kept (`place(id, index)`). A press that does not move is a click;
// Esc puts it back; Alt+↑ / Alt+↓ move the focused face. Touch scrolls the rail instead.
import { useRef, useLayoutEffect } from '../../../vendor/preact-htm.js';

const START = 6; /* px the pointer moves before a press becomes a drag */
const facesIn = (box) => [...box.querySelectorAll('[data-face]')];
const clamp = (x, lo, hi) => Math.max(lo, Math.min(hi, x));

/** `quiet()` is called when a drag starts (the hover card goes away). */
export function useDrag(ids, place, quiet) {
  const st = useRef(null), swallow = useRef(false), refocus = useRef('');
  useLayoutEffect(() => { /* a face moved by the keyboard keeps the focus */
    if (!refocus.current) return;
    const el = document.querySelector(`[data-face="${CSS.escape(refocus.current)}"]`);
    refocus.current = '';
    if (el) el.focus();
  });
  const shift = (d, to) => facesIn(d.box).forEach((x, i) => {
    if (x === d.el) return;
    const by = d.from < to ? (i > d.from && i <= to ? -d.step : 0) : (i >= to && i < d.from ? d.step : 0);
    x.style.transform = by ? `translateY(${by}px)` : '';
  });
  const finish = (cancel) => {
    const d = st.current;
    st.current = null;
    if (!d || !d.on) return;
    removeEventListener('keydown', d.esc);
    if (!cancel && d.to !== d.from) place(d.id, d.to); /* the new order renders before the next paint */
    facesIn(d.box).forEach((x) => { x.style.transform = ''; });
    d.box.classList.remove('dragging'); d.el.classList.remove('lift');
    swallow.current = true; setTimeout(() => { swallow.current = false; }, 0);
  };
  const start = (d, e) => {
    const rects = facesIn(d.box).map((x) => x.getBoundingClientRect());
    d.on = true; d.step = rects.length > 1 ? rects[1].top - rects[0].top : rects[0].height;
    d.esc = (k) => { if (k.key === 'Escape') finish(true); };
    addEventListener('keydown', d.esc);
    d.el.setPointerCapture(e.pointerId);
    d.box.classList.add('dragging'); d.el.classList.add('lift');
    quiet();
  };
  const down = (e) => {
    if (e.button !== 0 || e.pointerType === 'touch') return;
    const el = e.currentTarget;
    st.current = { id: el.dataset.face, el, box: el.parentElement, y0: e.clientY, pid: e.pointerId, on: false, from: ids.indexOf(el.dataset.face), to: ids.indexOf(el.dataset.face) };
  };
  const move = (e) => {
    const d = st.current;
    if (!d || e.pointerId !== d.pid) return;
    const dy = e.clientY - d.y0;
    if (!d.on && Math.abs(dy) < START) return;
    if (!d.on) start(d, e);
    d.el.style.transform = `translateY(${dy}px) scale(1.04)`;
    const to = clamp(Math.round(d.from + dy / d.step), 0, ids.length - 1);
    if (to !== d.to) { d.to = to; shift(d, to); }
  };
  const key = (e, id) => {
    if (!e.altKey || (e.key !== 'ArrowUp' && e.key !== 'ArrowDown')) return;
    e.preventDefault();
    const i = ids.indexOf(id), to = clamp(i + (e.key === 'ArrowUp' ? -1 : 1), 0, ids.length - 1);
    if (to !== i) { refocus.current = id; place(id, to); }
  };
  /** true once right after a drag: the click it ends with opens nothing */
  const swallowed = () => { const was = swallow.current; swallow.current = false; return was; };
  return { down, move, up: () => finish(false), cancel: () => finish(true), key, swallowed };
}

// The sidebar's width: dragged by its right edge (or with the arrow keys on the handle), kept per browser.
import { html, useState, useEffect } from '../../vendor/preact-htm.js';

const KEY = 'tirith.nav.width';
export const MIN = 244, MAX = 560, DEFAULT = 296, STEP = 16;
const clamp = (n) => Math.min(MAX, Math.max(MIN, Math.round(n)));

function load() {
  try { const n = Number(localStorage.getItem(KEY)); return n ? clamp(n) : DEFAULT; } catch { return DEFAULT; }
}
function apply(w) { document.documentElement.style.setProperty('--nav-w', `${w}px`); }
apply(load()); /* at import, before the first paint: no jump */

/** The handle on the sidebar's right edge: drag, arrow keys (Home / End: the limits), double click: the default. */
export function NavResizer() {
  const [w, setW] = useState(load);
  const [drag, setDrag] = useState(false);
  const set = (n) => { const v = clamp(n); setW(v); apply(v); try { localStorage.setItem(KEY, String(v)); } catch { /* private window */ } };
  useEffect(() => {
    if (!drag) return undefined;
    const move = (e) => { const v = clamp(e.clientX); setW(v); apply(v); };
    const up = (e) => { setDrag(false); set(e.clientX); };
    window.addEventListener('pointermove', move); window.addEventListener('pointerup', up);
    return () => { window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', up); };
  }, [drag]);
  const key = (e) => {
    const to = { ArrowLeft: w - STEP, ArrowRight: w + STEP, Home: MIN, End: MAX }[e.key];
    if (to != null) { e.preventDefault(); set(to); }
  };
  return html`<div class=${`navgrip${drag ? ' on' : ''}`} role="separator" aria-orientation="vertical" aria-label="Resize the sidebar"
    aria-valuemin=${MIN} aria-valuemax=${MAX} aria-valuenow=${w} tabindex="0" title="Drag to resize"
    onPointerDown=${(e) => { e.preventDefault(); setDrag(true); }} onKeyDown=${key} onDblClick=${() => set(DEFAULT)} />`;
}

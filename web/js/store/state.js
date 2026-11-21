// The page's data object and its repaints: every change is batched into one repaint of the components that read it.
import { useState, useEffect } from '../../vendor/preact-htm.js';

export const S = {
  projects: [], lanes: {}, talks: {}, events: [], owner: [], ended: {}, busy: {}, load: null,
  connected: false, loaded: false, updated: 0, boots: 0,
};
export const TALK_ROWS = 600;

const subs = new Set();
export const listeners = new Set();
let frame = 0;

export function changed() {
  if (frame) return;
  // A timer, not requestAnimationFrame: that one pauses in hidden windows, and the page must stay current there.
  frame = setTimeout(() => {
    frame = 0;
    S.updated = Date.now();
    subs.forEach((f) => f((n) => n + 1));
  }, 16);
}

/** Page-only state (the search text `q`, …): set it and every component repaints. */
export function setUi(key, value) {
  S[key] = value;
  changed();
}

/** Re-render the calling component whenever the data changes. */
export function useStore() {
  const [, set] = useState(0);
  useEffect(() => { subs.add(set); return () => subs.delete(set); }, []);
  return S;
}

/** `fn(message)` for every change of the stream ({t, d}); returns the unsubscribe function. */
export function onMessage(fn) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

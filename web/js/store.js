// The page's data: loaded once from /api/state, then kept current by the daemon's change stream (SSE).
// Components read it with useStore(); views that need other endpoints use useFetch(path, types), which
// refetches when the stream reports a change of one of those types. Repaints are batched per animation frame.
// Every project has its own conversation with its own manager ('' is the conversation of no project): `talks`
// holds the ones the page has opened (useTalk), `busy` the ones whose manager works on a turn.
import { useState, useEffect, useRef } from '../vendor/preact-htm.js';

export const S = {
  projects: [], lanes: {}, talks: {}, events: [], owner: [], ended: {}, busy: {}, load: null,
  connected: false, loaded: false, updated: 0, boots: 0,
};
const TALK_ROWS = 600;

const subs = new Set();
const listeners = new Set();
let frame = 0;

function changed() {
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

/** One call to the daemon: JSON when the answer is JSON, text otherwise; throws the daemon's message on errors. */
export async function api(method, path, body) {
  const r = await fetch(path, {
    method, headers: body === undefined ? {} : { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await r.text();
  if (!r.ok) throw new Error(text || r.statusText);
  try { return JSON.parse(text); } catch { return text; }
}

/** `[data, reload, error]` of GET `path`, refetched (debounced) when the stream reports one of `types`. */
export function useFetch(path, types = [], deps = []) {
  const [state, setState] = useState({ data: null, error: null });
  const timer = useRef(0);
  const load = () => {
    if (!path) return;
    api('GET', path).then((data) => setState({ data, error: null }), (error) => setState((s) => ({ data: s.data, error })));
  };
  useEffect(() => {
    load();
    if (!types.length) return undefined;
    const off = onMessage((m) => {
      if (!types.includes(m.t)) return;
      clearTimeout(timer.current);
      timer.current = setTimeout(load, 250);
    });
    return () => { off(); clearTimeout(timer.current); };
  }, [path, ...deps]);
  return [state.data, load, state.error];
}

/** One conversation: `{rows, loaded, error}`, read from the daemon when first asked for and after every
    reconnect, then kept current by the stream (rows that arrive while it loads are kept). */
export function useTalk(project) {
  const s = useStore();
  useEffect(() => {
    const t = S.talks[project] || (S.talks[project] = { rows: [], loaded: false, error: null });
    api('GET', `/api/chat?project=${encodeURIComponent(project)}`).then((rows) => {
      if (!Array.isArray(rows)) throw new Error('not found');
      const have = new Set(rows.map((r) => r.id));
      t.rows = rows.concat(t.rows.filter((r) => !have.has(r.id))).sort((a, b) => a.id - b.id);
      t.loaded = true; t.error = null;
      changed();
    }).catch((error) => { t.error = error; changed(); });
  }, [project, s.boots]);
  return S.talks[project] || { rows: [], loaded: false, error: null };
}

function apply(m) {
  const d = m.d;
  switch (m.t) {
    case 'lane': if (d.deleted) delete S.lanes[d.id]; else S.lanes[d.id] = d; break; // tirith lane delete
    case 'event': S.events.push(d); if (S.events.length > 400) S.events.splice(0, S.events.length - 400); break;
    case 'chat': {
      const t = S.talks[d.project || '']; // a conversation the page never opened is read when it is opened
      if (!t || t.rows.some((r) => r.id === d.id)) break;
      t.rows = t.rows.concat(d).slice(-TALK_ROWS);
      break;
    }
    case 'busy': if (d && d.on) S.busy[d.project || ''] = true; else if (d) delete S.busy[d.project || '']; break;
    case 'load': S.load = d; break;
    case 'owner':
      S.owner = S.owner.filter((o) => o.id !== d.id);
      if (d.status === 'answered' || d.status === 'closed') S.ended[d.id] = d.status; // how a card shown in the chat ended
      if (d.status !== 'answered' && d.status !== 'closed') S.owner.push(d);
      break;
    case 'projects': if (Array.isArray(d)) S.projects = d; break; // a project was added or config.toml changed
    case 'autopilot': {
      const p = S.projects.find((x) => x.id === d.project);
      if (p) p.autopilot = d.on;
      break;
    }
    default: break;
  }
}

let source = null;
async function boot() {
  try {
    const s = await api('GET', '/api/state');
    S.projects = s.projects || [];
    S.lanes = Object.fromEntries((s.lanes || []).map((l) => [l.id, l]));
    S.events = s.events || [];
    S.owner = s.owner || [];
    S.busy = Object.fromEntries((Array.isArray(s.busy) ? s.busy : []).map((p) => [p, true]));
    S.loaded = true;
    S.boots += 1;
  } catch {
    setTimeout(boot, 2000);
    return;
  }
  changed();
  if (source) source.close();
  source = new EventSource('/api/stream');
  source.onopen = () => { S.connected = true; changed(); };
  source.onerror = () => { S.connected = false; changed(); source.close(); setTimeout(boot, 2000); };
  source.onmessage = (e) => {
    let m;
    try { m = JSON.parse(e.data); } catch { return; }
    apply(m);
    listeners.forEach((fn) => { try { fn(m); } catch (err) { console.error(err); } });
    changed();
  };
}
boot();

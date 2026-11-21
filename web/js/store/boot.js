// Start-up: the whole state from /api/state, then the change stream; both start again after 2 s when they fail.
import { S, changed, listeners } from './state.js';
import { api } from './api.js';
import { apply } from './apply.js';

let source = null;

async function loadState() {
  const s = await api('GET', '/api/state');
  S.projects = s.projects || [];
  S.lanes = Object.fromEntries((s.lanes || []).map((l) => [l.id, l]));
  S.events = s.events || [];
  S.owner = s.owner || [];
  S.busy = Object.fromEntries((Array.isArray(s.busy) ? s.busy : []).map((p) => [p, true]));
  S.loaded = true;
  S.boots += 1;
}

function onStream(e) {
  let m;
  try { m = JSON.parse(e.data); } catch { return; }
  apply(m);
  listeners.forEach((fn) => { try { fn(m); } catch (err) { console.error(err); } });
  changed();
}

function openStream() {
  if (source) source.close();
  source = new EventSource('/api/stream');
  source.onopen = () => { S.connected = true; changed(); };
  source.onerror = () => { S.connected = false; changed(); source.close(); setTimeout(boot, 2000); };
  source.onmessage = onStream;
}

export async function boot() {
  try {
    await loadState();
  } catch {
    setTimeout(boot, 2000);
    return;
  }
  changed();
  openStream();
}

// Calls to the daemon: one request, and a GET kept current by the stream.
import { useState, useEffect, useRef } from '../../vendor/preact-htm.js';
import { onMessage } from './state.js';

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

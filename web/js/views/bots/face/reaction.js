// How a bot's big face answers what just happened, for a few seconds and no longer: a turn that ended well is
// celebrated and one that failed is mourned (its row in /api/turns says how it ended, read once when the
// working stops), and leaving sleep is a waking, not a jump. Both give their state back when they are done.
import { useState, useEffect, useRef } from '../../../../vendor/preact-htm.js';
import { api } from '../../../store.js';
import { reactionFor } from './face-map.js';

/** The face state a just-ended turn of conversation `conv` earns, or null. `busy`: a turn runs there now. */
export function useReaction(conv, busy) {
  const [mood, setMood] = useState(null);
  const was = useRef(busy);
  useEffect(() => {
    const ended = was.current && !busy;
    was.current = busy;
    if (!ended || !conv) return undefined;
    let on = true, t = 0;
    api('GET', `/api/turns?conv=${encodeURIComponent(conv)}&limit=5`).then((rows) => {
      const last = (rows || []).find((r) => r.role === conv);
      const m = last && reactionFor(last.status);
      if (!on || !m) return;
      setMood(m);
      t = setTimeout(() => on && setMood(null), 3200);
    }, () => {});
    return () => { on = false; clearTimeout(t); };
  }, [conv, busy]);
  return mood;
}

/** 'waking' for a moment when the state leaves 'sleep', else null. */
export function useWaking(state) {
  const [waking, setWaking] = useState(false);
  const was = useRef(state);
  useEffect(() => {
    const left = was.current === 'sleep' && state !== 'sleep';
    was.current = state;
    if (!left) return undefined;
    setWaking(true);
    const t = setTimeout(() => setWaking(false), 900);
    return () => clearTimeout(t);
  }, [state]);
  return waking ? 'waking' : null;
}

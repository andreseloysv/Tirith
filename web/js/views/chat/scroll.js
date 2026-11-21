// The thread's scroll and the box's height: the newest message stays in view when the list was at its end, and
// the box grows with its text up to half the window, then scrolls inside.
import { useEffect, useLayoutEffect, useRef } from '../../../vendor/preact-htm.js';

export function useScroll() {
  const anchor = useRef(null), log = useRef(null), stick = useRef(true);
  /* keep the newest message in view, but only when the list was already at its end */
  useLayoutEffect(() => {
    const el = log.current;
    if (el && stick.current) el.scrollTop = el.scrollHeight;
    else if (el && anchor.current != null) { el.scrollTop = el.scrollHeight - anchor.current; anchor.current = null; }
  });
  const onScroll = () => {
    const el = log.current; /* null once this conversation left the page: a late scroll event of its list */
    if (el) stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 48;
  };
  /* "Show earlier": the messages above grow the list, the one in view stays where it was */
  const earlier = (grow) => { const el = log.current; stick.current = false; anchor.current = el.scrollHeight - el.scrollTop; grow(); };
  return { log, stick, onScroll, earlier };
}

/** The box (`ta`) grows with its text (`draft`) up to half the window, then scrolls inside. */
export function useFit(ta, log, stick, draft) {
  const fit = () => {
    const t = ta.current;
    if (!t) return;
    t.style.height = 'auto';
    const cap = Math.floor(innerHeight * 0.5);
    t.style.height = Math.min(t.scrollHeight, cap) + 'px';
    t.style.overflowY = t.scrollHeight > cap ? 'auto' : 'hidden';
    const el = log.current;
    if (el && stick.current) el.scrollTop = el.scrollHeight;
  };
  useLayoutEffect(fit, [draft]);
  useEffect(() => { addEventListener('resize', fit); return () => removeEventListener('resize', fit); }, []);
}

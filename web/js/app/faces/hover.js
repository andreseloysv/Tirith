// The card a face shows while the pointer rests on it, as in the owner's example: its small face and its name, then
// where it works or its last message, two lines at most. Once one shows, the next face shows its own at once.
import { html, useState, useRef, useLayoutEffect } from '../../../vendor/preact-htm.js';
import { Face } from '../../views/bots/face.js';

const DELAY = 260; /* ms the pointer rests on a face before the first card */
const LEAVE = 80;  /* ms before it goes: long enough to slide to the next face */

/** `{at: {id, x, y} | null, show(id, el), hide(), now()}`; `now` hides it at once (a press, a drag). */
export function useHoverCard() {
  const [at, setAt] = useState(null);
  const timer = useRef(0), open = useRef(false);
  const show = (id, el) => {
    clearTimeout(timer.current);
    const put = () => {
      const r = el.getBoundingClientRect();
      open.current = true;
      setAt({ id, x: Math.round(r.right + 8), y: Math.round(r.top + r.height / 2) });
    };
    if (open.current) put(); else timer.current = setTimeout(put, DELAY);
  };
  const now = () => { clearTimeout(timer.current); open.current = false; setAt(null); };
  const hide = () => { clearTimeout(timer.current); timer.current = setTimeout(now, LEAVE); };
  return { at, show, hide, now };
}

/** The card beside its face, centred on it and kept inside the window. */
export function HoverCard({ face, at, line }) {
  const ref = useRef(null);
  const [top, setTop] = useState(at.y);
  useLayoutEffect(() => {
    const h = ref.current ? ref.current.offsetHeight : 0;
    setTop(Math.max(8, Math.min(innerHeight - h - 8, at.y - h / 2)));
  }, [at.y, at.id, line]);
  return html`<div ref=${ref} class="fcard" role="tooltip" id="face-card" style=${`left:${at.x}px;top:${top}px`}>
    <div class="fcard-h"><span class="fcard-f"><${Face} key=${face.id} shape=${face.shape} color=${face.color} size=${22} state=${face.state} still /></span><b>${face.name}</b></div>
    ${line ? html`<p class="fcard-t">${line}</p>` : null}
  </div>`;
}

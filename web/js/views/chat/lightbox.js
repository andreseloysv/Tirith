// A picture of the chat at full size.
import { html, useEffect, useRef } from '../../../vendor/preact-htm.js';

/** Full size: one overlay, closed by a click or Esc. */
export function Lightbox({ src, onClose }) {
  const btn = useRef(null);
  useEffect(() => {
    const key = (e) => { if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); onClose(); } };
    document.addEventListener('keydown', key, true);
    if (btn.current) btn.current.focus();
    return () => document.removeEventListener('keydown', key, true);
  }, []);
  return html`<div class="mc-lb" role="dialog" aria-label="Image" onClick=${onClose}>
    <img src=${src} alt="Attached image" /><button ref=${btn} type="button" class="mc-lbx" aria-label="Close">×</button></div>`;
}

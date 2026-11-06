// Small pieces of the Overview: a memo wrapper, the old page's icons (as SVG strings) and the agent avatar.
import { html, h, Component } from '../../../vendor/preact-htm.js';

/** `C` re-rendered only when one of its props changed (shallow); `skip` props are compared through `${key}Sig`. */
export function memo(C, skip = []) {
  class M extends Component {
    shouldComponentUpdate(n) {
      const p = this.props;
      for (const k in n) if (!skip.includes(k) && n[k] !== p[k]) return true;
      for (const k in p) if (!(k in n)) return true;
      return false;
    }
    render(p) { return h(C, p); }
  }
  return M;
}

/** A static SVG string as a span (the old page drew these icons as strings). */
export function Svg({ s, cls = '' }) {
  return html`<span class=${cls} dangerouslySetInnerHTML=${{ __html: s }}></span>`;
}

export const IC = {
  done: '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="7" fill="#2ea44f"/><path d="M4.8 8.2l2.2 2.2 4.2-4.6" fill="none" stroke="#fff" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>',
  active: '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="6.2" style="fill:var(--card)" stroke="#2f6fed" stroke-width="1.6"/><path d="M8 3.4a4.6 4.6 0 010 9.2z" fill="#2f6fed"/></svg>',
  todo: '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="6.2" fill="none" stroke="#b4bac4" stroke-width="1.6"/></svg>',
  empty: '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="6.2" fill="none" stroke="#d3d7de" stroke-width="1.6" stroke-dasharray="2 2.4"/></svg>',
};
export const OK = '<svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"><path d="M2.6 6.3l2.3 2.3 4.5-4.9" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>';
export const OPEN = '<svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"><circle cx="6" cy="6" r="3.6" fill="none" stroke="#b4bac4" stroke-width="1.3"/></svg>';
export const PLUS = '<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M8 3.2v9.6M3.2 8h9.6" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>';
export const CHEV = '<svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true"><path d="M3 4.5l3 3 3-3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>';
export const PAUSE = '<svg viewBox="0 0 12 12" width="11" height="11" aria-hidden="true"><path d="M3.5 2.5v7M8.5 2.5v7" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>';
export const RESUME = '<svg viewBox="0 0 12 12" width="11" height="11" aria-hidden="true"><path d="M3.5 2.2l6 3.8-6 3.8z" fill="currentColor"/></svg>';
export const REFRESH = '<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M13.2 8a5.2 5.2 0 11-1.6-3.7M13.4 2.4v3.1h-3.1" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>';

const PALETTE = ['#2f6fed', '#e0661f', '#2e9f4f', '#c2358f', '#b08400', '#7c4dd6', '#138f8f', '#d63a3a', '#8a5a3c', '#5b6b86'];
function hash(s) { let x = 0; for (let i = 0; i < s.length; i++) x = (x * 31 + s.charCodeAt(i)) | 0; return Math.abs(x); }
const GLYPH = 'M12 3a5 5 0 100 10 5 5 0 000-10z M4 21c0-5 3.5-7 8-7s8 2 8 7z';

/** The old round avatar: a gradient of the agent's color with a white glyph. */
export function Avatar({ id, size = 36 }) {
  const c = PALETTE[hash(String(id || '')) % PALETTE.length];
  const g = Math.round(size * 0.56);
  return html`<span class="avatar" style=${`width:${size}px;height:${size}px;background:linear-gradient(135deg, color-mix(in srgb, ${c} 62%, #fff), ${c})`}>
    <svg width=${g} height=${g} viewBox="0 0 24 24" aria-hidden="true"><path d=${GLYPH} fill="#fff" /></svg></span>`;
}

/** Plain text with `code` spans, as the old plan drew item texts. */
export function Rich({ t }) {
  const parts = String(t || '').split(/`([^`]+)`/);
  return parts.map((x, i) => (i % 2 ? html`<code>${x}</code>` : x));
}

export function clip(t, n) {
  t = String(t || '').replace(/\s+/g, ' ').trim();
  return t.length > n ? `${t.slice(0, n - 1)}…` : t;
}

/** Does `text` match the search `q` (already lower-cased)? */
export const hit = (q, ...texts) => !q || texts.some((t) => String(t || '').toLowerCase().includes(q));

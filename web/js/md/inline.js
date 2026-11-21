// Inline markdown of one line: escape, make the links (held aside), then bold, strikethrough and italics,
// and put the held links back.
import { esc } from './esc.js';
import { codeSpans, mdLinks, schemeUrls, filePaths, laneIds, bareDomains } from './links.js';

const LINK_STEPS = [codeSpans, mdLinks, schemeUrls, filePaths, laneIds, bareDomains];

function emphasis(s) {
  return s.replace(/\*\*(?=\S)([\s\S]*?\S)\*\*/g, '<strong>$1</strong>')
    .replace(/(^|[^\w])__(?=\S)([\s\S]*?\S)__(?!\w)/g, '$1<strong>$2</strong>')
    .replace(/~~(?=\S)([\s\S]*?\S)~~/g, '<del>$1</del>')
    .replace(/(^|[^*\w])\*(?=[^\s*])([^*]*?[^\s*])\*(?![*\w])/g, '$1<em>$2</em>')
    .replace(/(^|[^_\w])_(?=[^\s_])([^_]*?[^\s_])_(?![_\w])/g, '$1<em>$2</em>');
}

/* a placeholder can sit inside another one's label (a code span in a link), so unwrap until none is left */
function unwrap(s, hold) {
  for (let i = 0; i < 4 && /\u0001\d+\u0002/.test(s); i++) {
    s = s.replace(/\u0001(\d+)\u0002/g, (m, n) => (hold[+n] != null ? hold[+n] : ''));
  }
  return s;
}

export function inline(src) {
  const hold = [];
  const keep = (h) => { hold.push(h); return '\u0001' + (hold.length - 1) + '\u0002'; };
  let s = esc(String(src == null ? '' : src).replace(/[\u0001\u0002]/g, ''));
  for (const step of LINK_STEPS) s = step(s, keep);
  return unwrap(emphasis(s), hold);
}

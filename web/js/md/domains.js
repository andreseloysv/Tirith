// Bare domains: ejemplo.com/ruta links to https://…, a file name (notas.md) does not.
import { FILE_EXT, KNOWN_TLD } from './words.js';

/* a loose word whose last label is a known file extension is a local file (palabra.txt, notas.md, cli.py),
   not a domain; a www. prefix overrides this. A bare host with no tail also needs a two-letter or known TLD,
   so hola.mundo (or a full stop missing its space: vale.Luego) is not linked. */
export const BARE = /(^|[\s(\[{;*¿¡«“])((?:[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?\.)+[A-Za-z]{2,24}(?::\d+)?(?:[/?#](?:[^\s<\u0001\u0002&*"'`]|&amp;)*)?)/g;
const CLOSE_OF = { ')': '(', ']': '[', '}': '{' };
/** End of a URL as written: sentence punctuation and an unbalanced closing bracket stay outside the link,
    a balanced one (ejemplo.com/wiki/Foo_(bar)) stays inside. */
export function trimUrlEnd(url) {
  let u = String(url);
  while (u) {
    const last = u.charAt(u.length - 1), open = CLOSE_OF[last];
    if ('.,;:!?'.indexOf(last) >= 0) { u = /&amp;$/i.test(u) ? u.slice(0, -5) : u.slice(0, -1); continue; }
    if ('»”'.indexOf(last) >= 0) { u = u.slice(0, -1); continue; }
    if (!open) break;
    let opens = 0, closes = 0;
    for (const c of u) { if (c === open) opens++; else if (c === last) closes++; }
    if (closes <= opens) break;
    u = u.slice(0, -1);
  }
  return u;
}
export function isBareDomain(url) {
  const host = /^[^/?#:]+/.exec(String(url || ''))?.[0] || '';
  if (!host || host.indexOf('@') >= 0) return false;
  const labels = host.split('.');
  if (labels.length < 2 || labels.some((l) => !/^[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?$/.test(l))) return false;
  const tld = labels[labels.length - 1].toLowerCase();
  if (!/^[a-z]{2,24}$/.test(tld)) return false;
  if (labels[0].toLowerCase() === 'www' && labels.length >= 3) return true;
  if (FILE_EXT.has(tld)) return false;
  if (url.length > host.length) return true; /* a port, path, query or fragment follows the host */
  return tld.length === 2 || KNOWN_TLD.has(tld);
}

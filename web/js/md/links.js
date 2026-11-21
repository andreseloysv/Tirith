// The inline steps that make links, in their order: code spans, markdown links, scheme URLs, file paths, lane
// ids and bare domains. Each takes the escaped text and `keep(h)`, which holds finished HTML out of later steps.
import { ctx } from './refs.js';
import { splitRef, fileLink, PLAIN } from './files.js';
import { LANE, laneLink, laneWord } from './lanes.js';
import { BARE, trimUrlEnd, isBareDomain } from './domains.js';

export function codeSpans(s, keep) {
  return s.replace(/(`+)([^`]|[^`][\s\S]*?[^`])\1(?!`)/g, (m, t, code) => {
    const c = '<code>' + code.trim() + '</code>', ref = ctx.refs && !/\s/.test(code.trim()) ? splitRef(code.trim()) : null;
    return keep((ref && !ref.rest && fileLink(ref, c)) || laneLink(code.trim(), c) || c);
  });
}

export function mdLinks(s, keep) {
  return s.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (m, label, url) => {
    if (/^(?:https?:\/\/|file:\/\/)/i.test(url)) return keep(`<a href="${url}" target="_blank" rel="noopener">${label}</a>`);
    const ref = ctx.refs ? splitRef(url) : null, h = (ref && !ref.rest && fileLink(ref, label)) || laneLink(url.replace(/^lane[:\/]/, ''), label);
    if (h) return keep(h);
    return isBareDomain(url) ? keep(`<a href="https://${url}" target="_blank" rel="noopener">${label}</a>`) : m;
  });
}

export function schemeUrls(s, keep) {
  return s.replace(/(^|[\s(\[{;*¿¡«“])((?:https?|file):\/\/(?:[^\s<\u0001\u0002&*"'`]|&amp;)+)/gi, (m, pre, raw) => {
    const url = trimUrlEnd(raw);
    return !/[A-Za-z0-9]/.test(url.replace(/^(?:https?|file):\/\//i, '')) ? m :
      pre + keep(`<a href="${url}" target="_blank" rel="noopener">${url}</a>`) + raw.slice(url.length);
  });
}

export function filePaths(s, keep) {
  if (!(ctx.refs && ctx.refs.file)) return s;
  return s.replace(PLAIN, (m, pre, tok) => {
    const ref = splitRef(tok), h = ref && fileLink(ref, tok.slice(0, tok.length - ref.rest.length));
    return h ? pre + keep(h) + ref.rest : m;
  });
}

export function laneIds(s, keep) {
  if (!(ctx.refs && ctx.refs.lane)) return s;
  return s.replace(LANE, (m, pre, slash, id, at, all) => {
    const h = laneWord(all, at, m, pre, slash, id) && laneLink(id, (slash || '') + id);
    return h ? pre + keep(h) : m;
  });
}

/* after code, markdown links, scheme URLs, file refs and lanes, so a bare domain steals none of them;
   the prefix it needs (start, space or an opening bracket) is never @, : or /, so emails and a scheme
   written without // (javascript:, data:, file:) never reach it */
export function bareDomains(s, keep) {
  return s.replace(BARE, (m, pre, raw) => {
    const url = trimUrlEnd(raw);
    return isBareDomain(url) ? pre + keep(`<a href="https://${url}" target="_blank" rel="noopener">${url}</a>`) + raw.slice(url.length) : m;
  });
}

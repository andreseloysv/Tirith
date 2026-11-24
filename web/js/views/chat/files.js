// What a message carries besides its words: its Markdown drawn once and kept, its pictures as thumbnails and
// its other files as chips.
import { html } from '../../../vendor/preact-htm.js';
import { md, boldLead } from '../../md.js';
import { DOC } from './icons.js';
import { size } from './util.js';
import { t } from '../../i18n.js';

const mdCache = new Map();   /* message id -> HTML (texts never change) */
export function mdOf(key, text) {
  let h = mdCache.get(key);
  if (h == null) {
    h = md(boldLead(text));
    mdCache.set(key, h);
    if (mdCache.size > 800) mdCache.delete(mdCache.keys().next().value);
  }
  return h;
}

export function Pics({ files, onZoom }) {
  if (!files || !files.length) return null;
  const pics = files.filter((f) => f.pic), rest = files.filter((f) => !f.pic);
  return html`<div class="mc-pics">
    ${pics.map((f, i) => html`<button class="mc-pic" type="button" title=${t('Open full size')} aria-label=${t('Image {n}', { n: i + 1 })} onClick=${() => onZoom(f.url)}>
      <img src=${f.url} alt=${t('Image {n}', { n: i + 1 })} loading="lazy" /></button>`)}
    ${rest.map((f) => html`<${FileChip} name=${f.name} title=${f.path} href=${f.url} />`)}</div>`;
}

export function FileChip({ name, bytes, title, href, onRemove }) {
  const inner = html`${DOC}<span class="mc-fname">${name}</span>${bytes != null ? html`<span class="mc-fsize">${size(bytes)}</span>` : null}
    ${onRemove ? html`<button type="button" aria-label=${t('Remove {name}', { name })} title=${t('Remove')} onClick=${onRemove}>×</button>` : null}`;
  return href ? html`<a class="mc-file" title=${title} href=${href} target="_blank" rel="noopener">${inner}</a>`
    : html`<span class="mc-file" title=${title}>${inner}</span>`;
}

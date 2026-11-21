// A manager turn as the owner sees it, and a message of this page on its way to the daemon.
import { html } from '../../../vendor/preact-htm.js';
import { clockTime } from '../../common.js';
import { esc, ctxHtml } from '../../md.js';
import { Face } from '../bots/face.js';
import { pure } from './util.js';
import { answerOf, quiet, Intent } from './turns.js';
import { mdOf, FileChip } from './files.js';

/** One answer of an ended turn: a quiet line, or the words with the speaker's face beside them. */
function answer(r, who, face) {
  if (quiet(r.text)) return html`<div class="mc-ev quiet" key=${r.id} title=${clockTime(r.at)}><span>${r.text.replace(/^\s*∅\s*/, '')}</span></div>`;
  return html`<div class="mc-m mgr" key=${r.id}><div class="mc-line">
      ${face && face.shape ? html`<span class="mc-face"><${Face} shape=${face.shape} color=${face.color || '#3b82f6'} size=${22} still /></span>` : null}
      <div class="mc-col"><div class="mc-b md" dangerouslySetInnerHTML=${{ __html: mdOf(r.id, r.text) }} />
      <div class="mc-meta" dangerouslySetInnerHTML=${{ __html: ctxHtml(who || 'Manager') + ' · ' + esc(clockTime(r.at)) }} /></div></div></div>`;
}

/** One turn: the tiny intent line while it runs, then only its answer, with the face and name of who speaks beside it. */
export const Turn = pure(({ rows, running, who, face }) => {
  if (running) return html`<${Intent} who=${who} step=${rows[rows.length - 1]} />`;
  return html`${answerOf(rows, false).map((r) => answer(r, who, face))}`;
}, (a, b) => a.k === b.k && a.n === b.n && a.running === b.running && a.who === b.who && a.fk === b.fk);

/** A message sent from this page that the daemon has not shown yet (sending) or refused (failed). */
export function Local({ o, onRetry, onCopy }) {
  const files = o.pics.map((p) => ({ name: p.name, url: p.url, pic: !!p.url, path: p.name }));
  return html`<div class=${`mc-m me local ${o.state}`}>
    ${files.length ? html`<div class="mc-pics">${files.map((f) => (f.pic
      ? html`<span class="mc-pic"><img src=${f.url} alt=${f.name} /></span>`
      : html`<${FileChip} name=${f.name} title=${f.name} />`))}</div>` : null}
    ${o.text ? html`<div class="mc-b">${o.text}</div>` : null}
    <div class="mc-meta">${o.state === 'failed'
      ? html`<b class="mc-bad">Not sent</b> · <button type="button" class="mc-lnk" onClick=${onRetry}>Retry</button> · <button type="button" class="mc-lnk" onClick=${onCopy}>Copy text</button>${o.err ? html`<br /><span class="mc-why">${o.err}</span>` : null}`
      : html`Sending… · ${clockTime(new Date(o.at).toISOString())}`}</div></div>`;
}

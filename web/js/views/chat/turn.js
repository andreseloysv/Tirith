// A manager turn as the owner sees it, and a message of this page on its way to the daemon.
import { html, useRef } from '../../../vendor/preact-htm.js';
import { clockTime } from '../../common.js';
import { md } from '../../md.js';
import { pure } from './util.js';
import { answerOf, quiet } from './turns.js';
import { bubble, words } from './bubble.js';
import { mdOf, FileChip } from './files.js';
import { Reactions, Made, turnOf } from './react.js';
import { cited } from './cite.js';
import { t } from '../../i18n.js';

/** One answer of an ended turn: a quiet line, or a bubble with who said it (and under a bot's last one its 👍 / 👎 and how it was made: model, effort, time; its memory citations as small marks). */
function answer(r, { who, turn, notes, cls }) {
  if (quiet(r.text)) return html`<div class="mc-ev quiet" key=${r.id} title=${clockTime(r.at)}><span>${r.text.replace(/^\s*∅\s*/, '')}</span></div>`;
  return bubble({
    key: r.id,
    cls,
    from: who || t('Manager'),
    time: clockTime(r.at),
    meta: turn ? html`<${Reactions} turn=${turn} /><${Made} turn=${turn} />` : null,
    body: words(cited(mdOf(r.id, r.text), notes)),
  });
}

/** One turn: nothing while it runs (working.js shows the bot at work), then only its answer. An answer that was
    waited for here grows out of where the face was. */
export const Turn = pure(({ rows, running, who, turns, notes }) => {
  const waited = useRef(false);
  if (running) { waited.current = true; return null; }
  const shown = answerOf(rows, false), last = shown[shown.length - 1];
  const turn = turns && last && !quiet(last.text) ? turnOf(turns, last) : null;
  const cls = waited.current ? 'mgr arrive' : 'mgr';
  return html`${shown.map((r) => answer(r, { who, turn: r === last ? turn : null, notes, cls }))}`;
}, (a, b) => a.k === b.k && a.n === b.n && a.running === b.running && a.who === b.who && a.turns === b.turns && a.notes === b.notes);

/** A message sent from this page that the daemon has not shown yet (sending) or refused (failed). */
export function Local({ o, onRetry, onCopy }) {
  const files = o.pics.map((p) => ({ name: p.name, url: p.url, pic: !!p.url, path: p.name }));
  const body = html`${files.length ? html`<div class="mc-pics">${files.map((f) => (f.pic
    ? html`<span class="mc-pic"><img src=${f.url} alt=${f.name} /></span>`
    : html`<${FileChip} name=${f.name} title=${f.name} />`))}</div>` : null}
    ${words(o.text ? md(o.text) : '')}`;
  const failed = o.state === 'failed';
  const meta = failed ? html`<span class="mc-unsent"><b class="mc-bad">${t('Not sent')}</b> · <button type="button" class="mc-lnk" onClick=${onRetry}>${t('Retry')}</button> · <button type="button" class="mc-lnk" onClick=${onCopy}>${t('Copy text')}</button>${o.err ? html`<br /><span class="mc-why">${o.err}</span>` : null}</span>` : null;
  return bubble({ cls: `me local ${o.state}`, time: failed ? '' : `${t('Sending…')} · ${clockTime(new Date(o.at).toISOString())}`, meta, body });
}

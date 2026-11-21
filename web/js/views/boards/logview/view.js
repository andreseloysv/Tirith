// The log viewer: filter chips, newest first, system events hidden behind a toggle, long logs capped.
import { html, useState, useMemo } from '../../../../vendor/preact-htm.js';
import { parseLog } from './parse.js';
import { Entry } from './entry.js';

const FILTERS = [['all', 'All'], ['command', 'Commands'], ['edit', 'Edits'], ['error', 'Errors'], ['test', 'Tests']];
const KINDS = { command: ['command', 'test', 'output'], edit: ['edit'], error: ['error'], test: ['test'] };
const SHOW = 120;

function chips(all, f, setF) {
  return html`<div class="lchips" role="group" aria-label="Filter log">${FILTERS.map(([k, label]) => {
      const n = k === 'all' ? 0 : all.filter((e) => KINDS[k].includes(e.kind)).length;
      return html`<button class=${`lchip${f === k ? ' on' : ''}`} aria-pressed=${f === k} onClick=${() => setF(k)}>${label}${n ? html` <b>${n}</b>` : null}</button>`;
    })}</div>`;
}

export function LogView({ text }) {
  const all = useMemo(() => parseLog(text).reverse(), [text]);
  const [f, setF] = useState('all');
  const [sys, setSys] = useState(false);
  const [limit, setLimit] = useState(SHOW);
  const hidden = all.filter((e) => e.kind === 'system').length;
  const shown = all.filter((e) => (e.kind === 'system' ? sys && f === 'all' : f === 'all' || (KINDS[f] || []).includes(e.kind)));
  return html`<div class="logv">
    ${chips(all, f, setF)}
    ${shown.length ? html`<ul class="llist" tabindex="0">${shown.slice(0, limit).map((e) => html`<${Entry} key=${e.k} e=${e} />`)}</ul>` : html`<p class="quiet">${all.length ? 'Nothing matches this filter' : 'No log output yet'}</p>`}
    ${shown.length > limit ? html`<button class="link lsyst" onClick=${() => setLimit(limit + SHOW)}>Show ${Math.min(SHOW, shown.length - limit)} older entries</button>` : null}
    ${hidden && f === 'all' ? html`<button class="link lsyst" onClick=${() => setSys(!sys)}>${sys ? 'Hide ' : ''}${hidden} system event${hidden === 1 ? '' : 's'}${sys ? '' : ' hidden'}</button>` : null}
  </div>`;
}

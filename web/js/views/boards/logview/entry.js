// One log entry, one look per kind: a command, its output (folded when long), an edit with its counts, a read, an
// error, a test run with its verdict, a sub-agent, the final answer, a system event, or the agent's text.
import { html, useState } from '../../../../vendor/preact-htm.js';
import { Md } from '../md.js';

const ic = (d) => html`<svg class="lic" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"
  stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d=${d} /></svg>`;
const I = {
  file: 'M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z M14 3v5h5',
  flask: 'M9 3h6M10 3v6L4.5 19a1.6 1.6 0 0 0 1.4 2.4h12.2a1.6 1.6 0 0 0 1.4-2.4L14 9V3 M7.5 15h9',
  branch: 'M6 3a2 2 0 100 4 2 2 0 000-4z M6 17a2 2 0 100 4 2 2 0 000-4z M18 7a2 2 0 100 4 2 2 0 000-4z M6 7v10M18 11c0 4-6 3-12 6',
  flag: 'M5 21V4M5 4h11l-2 4 2 4H5',
};

function More({ open, set }) {
  return html`<button class="link lmore" aria-expanded=${open} onClick=${() => set(!open)}>${open ? 'show less' : 'show more'}</button>`;
}

function edit(e) {
  const i = e.text.lastIndexOf('/');
  return html`${ic(I.file)}<span class="lpath" title=${e.text}>${i >= 0 ? html`<span class="mut">${e.text.slice(0, i + 1)}</span>` : null}<b>${e.text.slice(i + 1)}</b></span>
        ${e.plus || e.minus ? html`<span class="lcnt"><span class="plus">+${e.plus}</span> <span class="minus">−${e.minus}</span></span>` : null}`;
}

/** The body of each kind: `(e, open, setOpen, long)`. */
const BODY = {
  command: (e) => html`<span class="lprompt">$</span><code class="lcmd" title=${e.text}>${e.text}</code>`,
  output: (e, open, setOpen, long) => html`<pre class=${`lout${open || !long ? ' open' : ''}`}>${e.text}</pre>${long ? html`<${More} open=${open} set=${setOpen} />` : null}`,
  edit,
  read: (e) => html`<span class="lread" title=${e.text}>${e.text}</span>`,
  error: (e, open, setOpen, long) => html`<div class="lerr"><span class="lerrt">${open || !long ? e.text : e.text.split('\n').slice(0, 3).join('\n')}</span>${long ? html`<${More} open=${open} set=${setOpen} />` : null}</div>`,
  test: (e) => html`${ic(I.flask)}<code class="lcmd" title=${e.text}>${e.text}</code>${e.sum ? html`<span class="mut lsum">${e.sum}</span>` : null}<span class=${`rb ${e.res}`}>${e.res}</span>`,
  spawn: (e) => html`${ic(I.branch)}<span class="lspawn"><span class="mut">Started sub-agent</span> <b>${e.text}</b></span>`,
  done: (e) => html`<div class="ldone"><div class="ldh">${ic(I.flag)}Final answer${e.usage ? html`<span class="mut lusage">${e.usage}</span>` : null}</div><div class="ldt"><${Md} text=${e.text} /></div></div>`,
  system: (e) => html`<span class="lsys" title=${e.detail || ''}>${e.text}</span>`,
};
const say = (e) => html`<div class="lsay"><${Md} text=${e.text} /></div>`;

export function Entry({ e }) {
  const [open, setOpen] = useState(false);
  const long = String(e.text).split('\n').length > 3 || e.text.length > 400;
  const body = (Object.prototype.hasOwnProperty.call(BODY, e.kind) ? BODY[e.kind] : say)(e, open, setOpen, long);
  return html`<li class=${`le ${e.kind}${e.kind === 'test' ? ` ${e.res}` : ''}`}><span class="lt"></span><div class="lb">${body}</div></li>`;
}

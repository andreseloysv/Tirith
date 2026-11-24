// Under the folder: what the dry run says (already a project, what adding does, why it cannot be one), or else the
// git repositories next to the projects that match what is typed.
import { html } from '../../../vendor/preact-htm.js';
import { t } from '../../i18n.js';

/** What adding `r` does: the steps Tirith takes and, when its plan has open items, that the autopilot waits. */
function says(r) {
  const todo = !r.git && !r.plan ? t('NeoBot will create a git repository in it and start its plan with a first step.') : !r.git ? t('NeoBot will create a git repository in it.') : !r.plan ? t('NeoBot will start its plan with a first step.') : t('Ready to add.');
  const held = !r.open ? '' : r.open === 1 ? ` ${t('Its plan has 1 open item: the autopilot stays off until you resume it.')}` : ` ${t('Its plan has {n} open items: the autopilot stays off until you resume it.', { n: r.open })}`;
  return [todo, held];
}

function options(d) {
  return html`<div class="ap-l">${t('Git repositories next to your projects')}</div>
          <div class="ap-opts">${d.list.map((f) => html`<button type="button" class="ap-opt" key=${f.folder} title=${f.folder}
            onClick=${() => { d.setFolder(f.folder); if (d.input.current) d.input.current.focus(); }}><b>${f.name}</b><span>${f.folder}</span></button>`)}</div>`;
}

export function hint(d, a) {
  const { r, check } = d;
  return r && r.exists ? html`<p class="ap-note">${t('Already a project:')} <b>${r.name || r.id}</b> · <button type="button" class="alink" onClick=${() => a.show(r.id)}>${t('Show it')}</button></p>`
    : r ? html`<p class="ap-note" title=${r.folder}>${says(r)[0]}${says(r)[1]}</p>`
    : check && check.err ? html`<div class="ierr ap-err" role="alert">${check.err}</div>`
    : d.list.length ? options(d)
    : null;
}

// The dialog's form: the folder (typed or chosen), what the dry run says, the name and the color, Cancel and Add.
import { html } from '../../../vendor/preact-htm.js';
import { COLORS, base } from './util.js';
import { hint } from './hint.js';

function nameAndColor(d, a) {
  const { r } = d;
  return html`<label class="ap-l" for="ap-name">Name</label>
      <input id="ap-name" class="ap-in" maxlength="80" autocomplete="off" placeholder=${(r && r.name) || base(d.folder) || 'Name'}
        value=${d.name} onInput=${(e) => d.setName(e.target.value)} onKeyDown=${a.enter} />
      <div class="ap-l" id="ap-color">Color</div>
      <div class="ap-sws" role="group" aria-labelledby="ap-color">${COLORS.map(([c, label]) => {
        const on = (d.color || (r && r.color)) === c;
        return html`<button type="button" class=${`ap-sw${on ? ' on' : ''}`} style=${`background:${c}`} aria-pressed=${on} aria-label=${label} title=${label}
          onClick=${() => d.setColor(c)}></button>`;
      })}</div>`;
}

export function formBody(d, a) {
  const { r, busy } = d;
  return html`<div key="form"><label class="ap-l" for="ap-folder">Folder</label>
      <div class="ap-row"><input id="ap-folder" ref=${d.input} class="ap-in" placeholder="~/DEVELOP/my-app" autocomplete="off" spellcheck="false"
        value=${d.folder} onInput=${(e) => d.setFolder(e.target.value)} onKeyDown=${a.enter} />
        <button type="button" class="ap-b" disabled=${!!busy} onClick=${a.pick}>${busy === 'pick' ? 'Choosing…' : 'Choose…'}</button></div>
      ${hint(d, a)}
      ${r && r.exists ? null : nameAndColor(d, a)}
      ${d.err ? html`<div class="ierr ap-err" role="alert">${d.err}</div>` : null}
      <div class="ap-act"><button type="button" class="ap-b" onClick=${d.close}>Cancel</button>
        <button type="button" class="ap-b pri" disabled=${!!busy || !r || r.exists} onClick=${a.add}>${busy === 'add' ? 'Adding…' : 'Add'}</button></div></div>`;
}

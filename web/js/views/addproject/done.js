// The dialog after Add: what was added (or that it already was a project), its folder, the daemon's notes, Done.
import { html } from '../../../vendor/preact-htm.js';
import { t } from '../../i18n.js';

export function doneBody(done, doneBtn, close) {
  return html`<div key="done"><p class="ap-p">${done.exists ? t('Already a project:') : t('Added')} <b>${done.name || done.id}</b></p>
      <p class="ap-folder" title=${done.folder}>${done.folder}</p>
      ${(done.notes || []).map((n) => html`<p class="ap-note">${t(n)}</p>`)}
      <div class="ap-act"><button type="button" class="ap-b pri" ref=${doneBtn} onClick=${close}>${t('Done')}</button></div></div>`;
}

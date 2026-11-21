// The dialog after Add: what was added (or that it already was a project), its folder, the daemon's notes, Done.
import { html } from '../../../vendor/preact-htm.js';

export function doneBody(done, doneBtn, close) {
  return html`<div key="done"><p class="ap-p">${done.exists ? 'Already a project: ' : 'Added '}<b>${done.name || done.id}</b></p>
      <p class="ap-folder" title=${done.folder}>${done.folder}</p>
      ${(done.notes || []).map((n) => html`<p class="ap-note">${n}</p>`)}
      <div class="ap-act"><button type="button" class="ap-b pri" ref=${doneBtn} onClick=${close}>Done</button></div></div>`;
}

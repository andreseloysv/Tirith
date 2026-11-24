// The chat's head: the face and name of who speaks, what it is doing, Stop while it works and the ⋯ menu.
import { html } from '../../../vendor/preact-htm.js';
import { Face } from '../bots/face.js';
import { ChatMenu } from './menu.js';

/** `sp`: useSpeaker's answer. `listening`: the owner is writing, so an idle face attends instead of dozing. */
export function head(sp, project, interrupt, listening = false) {
  const { face, faceState, unseen, whoName, isProject, name, stateLabel, busy } = sp;
  const shown = !busy && listening ? 'listening' : faceState;
  return html`<div class="mc-head"><h2 class="mc-id">${face ? html`<span class="mc-face"><${Face} ...${face} size=${64} state=${shown} />${unseen ? html`<em class="nunread">${unseen > 9 ? '9+' : unseen}</em>` : null}</span>` : null}<span class="mc-who"><span class="mc-name">${whoName}${isProject ? html` <span class="mc-of">${name}</span>` : null}</span><span class="mc-state">${stateLabel}</span></span></h2>
      <span class="mc-tools">${busy ? html`<button class="mc-stop" type="button" onClick=${interrupt} title="Stop the current turn (Esc)">Stop</button>` : null}<${ChatMenu} project=${project} /></span></div>`;
}

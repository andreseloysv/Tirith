// The welcome's messages as the chat draws them (../chat/bubble.js): the assistant's on the left, the owner's on the
// right, and the assistant's face only while it is "writing" the next one (never beside a written message).
import { html } from '../../../vendor/preact-htm.js';
import { md } from '../../md.js';
import { bubble, words } from '../chat/bubble.js';
import { Face } from '../bots/face.js';

export const LOOK = { shape: 'cloud', color: '#ec4899' };

export function Talk({ lines, typing }) {
  return html`<div class="ob-talk">
    ${lines.map((l, i) => bubble({ key: i, cls: l.me ? 'me' : 'mgr', time: '', body: words(md(l.text)) }))}
    ${typing ? html`<div class="ob-typing"><${Face} ...${LOOK} size=${40} state="work" /><span class="ob-dots"><i></i><i></i><i></i></span></div>` : null}
  </div>`;
}

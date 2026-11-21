// The thread: the stored messages and turns, what an empty conversation says, the tiny line of a turn that has
// not said anything yet, and the messages still on their way.
import { html } from '../../../vendor/preact-htm.js';
import { Intent } from './turns.js';
import { Msg, CARD } from './msg.js';
import { Turn, Local } from './turn.js';

/** The rows of `items` (group's answer): a Msg per message, a Turn per manager turn. */
function rowsOf(items, sp, project, setZoom) {
  const { s, busy, whoName, face, faceKey } = sp;
  const cardOf = (m) => { const c = CARD.exec(m.text); return (c && s.owner.find((o) => o.id === c[1])) || null; };
  const lastItem = items[items.length - 1];
  const runningKey = busy && lastItem && lastItem.kind === 'turn' ? lastItem.key : null;
  const rows = items.map((it) => (it.kind === 'msg'
    ? html`<${Msg} key=${it.key} m=${it.m} project=${project} projects=${s.projects} onZoom=${setZoom} open=${it.m.role === 'card' ? cardOf(it.m) : null} ended=${it.m.role === 'card' ? s.ended[(CARD.exec(it.m.text) || [])[1]] : null} />`
    : html`<${Turn} key=${it.key} k=${it.key} rows=${it.rows} n=${it.rows.length} running=${it.key === runningKey} who=${whoName} face=${face} fk=${faceKey} />`));
  /* busy but no output yet: the tiny line under the newest message */
  const pending = busy && !runningKey && lastItem ? html`<${Intent} who=${whoName} step=${null} />` : null;
  return { rows, pending };
}

/** What an empty conversation says. */
function emptyHint({ isProject, whoName, name }, hint) {
  return html`<p class="quiet">${hint || (isProject
    ? `Write to the ${whoName} of ${name} here. It works on this project only: its answers, its agents' questions and what lands on main show in this thread.`
    : 'Write here about anything that is not one project\'s: this manager gets the health reports and the CEO\'s passes. Every project has its own Project Manager: select a project to talk to it.')}</p>`;
}

/** v: {sp, project, hint, items, hidden, scroll, onEarlier, setZoom, send: useSend's answer}. */
export function thread({ sp, project, hint, items, hidden, scroll, onEarlier, setZoom, send }) {
  const { talk } = sp, { out, retry, copy } = send;
  const { rows, pending } = rowsOf(items, sp, project, setZoom);
  return html`<div class="mc-log" id="mc-log" ref=${scroll.log} onScroll=${scroll.onScroll}>
      ${hidden ? html`<button type="button" class="mc-earlier" onClick=${onEarlier}>Show earlier (${hidden})</button>` : null}
      ${talk.error && !talk.loaded ? html`<div class="ierr mc-note">${/not found|not allowed/i.test(talk.error.message)
          ? 'The running Tirith daemon is older than this page: quit Tirith, stop the daemon (pkill -f "tirith serve") and open Tirith again.'
          : `Could not read this conversation: ${talk.error.message}`}</div>` : null}
      ${rows.length || out.length ? rows : !talk.loaded ? null : emptyHint(sp, hint)}
      ${pending}
      ${out.map((o) => html`<${Local} key=${o.cid} o=${o} onRetry=${() => retry(o)} onCopy=${() => copy(o)} />`)}
    </div>`;
}

// The thread: the stored messages and turns, what an empty conversation says, the messages still on their way and,
// last while a turn runs, the bot at work.
import { html } from '../../../vendor/preact-htm.js';
import { Msg, CARD } from './msg.js';
import { Turn, Local } from './turn.js';
import { Working } from './working.js';

/** The rows of `items` (group's answer): a Msg per message, a Turn per manager turn; and the running turn's newest row. */
function rowsOf(items, sp, project, setZoom, turns, notes) {
  const { s, busy, whoName } = sp;
  const cardOf = (m) => { const c = CARD.exec(m.text); return (c && s.owner.find((o) => o.id === c[1])) || null; };
  const lastItem = items[items.length - 1];
  const running = busy && lastItem && lastItem.kind === 'turn' ? lastItem : null;
  const rows = items.map((it) => (it.kind === 'msg'
    ? html`<${Msg} key=${it.key} m=${it.m} project=${project} projects=${s.projects} onZoom=${setZoom} open=${it.m.role === 'card' ? cardOf(it.m) : null} ended=${it.m.role === 'card' ? s.ended[(CARD.exec(it.m.text) || [])[1]] : null} />`
    : html`<${Turn} key=${it.key} k=${it.key} rows=${it.rows} n=${it.rows.length} running=${it === running} who=${whoName} turns=${turns} notes=${notes} />`));
  return { rows, step: running ? running.rows[running.rows.length - 1] : null };
}

/** What an empty conversation says. */
function emptyHint({ isProject, whoName, name }, hint) {
  return html`<p class="quiet">${hint || (isProject
    ? `Write to the ${whoName} of ${name} here. It works on this project only: its answers, its agents' questions and what lands on main show in this thread.`
    : 'Write here about anything that is not one project\'s: this manager gets the health reports and the CEO\'s passes. Every project has its own Project Manager: select a project to talk to it.')}</p>`;
}

/** v: {sp, project, hint, items, hidden, scroll, onEarlier, setZoom, send: useSend's answer, turns: a bot's turns (useBotTurns), notes: its numbered notes (useNotes)}. */
export function thread({ sp, project, hint, items, hidden, scroll, onEarlier, setZoom, send, turns, notes }) {
  const { talk, busy, whoName, face } = sp, { out, retry, copy } = send;
  const { rows, step } = rowsOf(items, sp, project, setZoom, turns, notes);
  const empty = !rows.length && !out.length && !busy && talk.loaded;
  return html`<div class="mc-log" id="mc-log" ref=${scroll.log} onScroll=${scroll.onScroll}>
      ${hidden ? html`<button type="button" class="mc-earlier" onClick=${onEarlier}>Show earlier (${hidden})</button>` : null}
      ${talk.error && !talk.loaded ? html`<div class="ierr mc-note">${/not found|not allowed/i.test(talk.error.message)
          ? 'The running NeoBot daemon is older than this page: quit NeoBot, stop the daemon (pkill -f "neobot serve") and open NeoBot again.'
          : `Could not read this conversation: ${talk.error.message}`}</div>` : null}
      ${rows}
      ${empty ? emptyHint(sp, hint) : null}
      ${out.map((o) => html`<${Local} key=${o.cid} o=${o} onRetry=${() => retry(o)} onCopy=${() => copy(o)} />`)}
      ${busy ? html`<${Working} key="working" who=${whoName} face=${face} step=${step} />` : null}
    </div>`;
}

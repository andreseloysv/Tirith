// One message of the daemon's change stream ({t, d}) applied to the page's data: one named step per type.
import { S, TALK_ROWS } from './state.js';

const talkOf = (d) => S.talks[d.project || ''];

function lane(d) { if (d.deleted) delete S.lanes[d.id]; else S.lanes[d.id] = d; } // neobot lane delete
function event(d) { S.events.push(d); if (S.events.length > 400) S.events.splice(0, S.events.length - 400); }
function chat(d) {
  const t = talkOf(d); // a conversation the page never opened is read when it is opened
  if (!t || t.rows.some((r) => r.id === d.id)) return;
  t.rows = t.rows.concat(d).slice(-TALK_ROWS);
}
function chatClear(d) { const t = talkOf(d); if (t) t.rows = []; } // the chat was cleared (here, by its bot, or from the phone)
function chatTrim(d) { const t = talkOf(d); if (t) t.rows = t.rows.filter((r) => r.at >= d.before); } // its longest life dropped the older rows
function busy(d) { if (d && d.on) S.busy[d.project || ''] = true; else if (d) delete S.busy[d.project || '']; }
function load(d) { S.load = d; }
function owner(d) {
  S.owner = S.owner.filter((o) => o.id !== d.id);
  if (d.status === 'answered' || d.status === 'closed') S.ended[d.id] = d.status; // how a card shown in the chat ended
  if (d.status !== 'answered' && d.status !== 'closed') S.owner.push(d);
}
function projects(d) { if (Array.isArray(d)) S.projects = d; } // a project was added or config.toml changed
function autopilot(d) {
  const p = S.projects.find((x) => x.id === d.project);
  if (p) p.autopilot = d.on;
}

const STEP = { lane, event, chat, 'chat-clear': chatClear, 'chat-trim': chatTrim, busy, load, owner, projects, autopilot };

export function apply(m) {
  const step = Object.prototype.hasOwnProperty.call(STEP, m.t) ? STEP[m.t] : null;
  if (step) step(m.d);
}

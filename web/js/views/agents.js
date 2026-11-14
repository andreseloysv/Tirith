// Agents view: the old page's kanban (.board of .col, one per lane status) or its compact list (.rows), and the
// right drawer (.drawer) with everything about one lane: item, status, note, question, details, its readable
// log, its live tool events and its pipelines, and Stop. Lanes come live from the store (SSE "lane").
import { html, useState, useEffect, useMemo } from '../../vendor/preact-htm.js';
import { useStore, useFetch, api, onMessage, setUi } from '../store.js';
import { ICON, Icon, Span, ms, ago, clockTime, laneLabel, projectColor, go } from '../common.js';
import { Md, MdInline } from './boards/md.js';
import { LogView } from './boards/logview.js';
import { pipeName, laneIds, StageChip, glyph, sClass, startMs } from './boards/pipe.js';
import { Need, openCards } from './need.js';

const DAY = 86400000;
const CAP = 30;
const COLS = [
  ['queued', 'Queued', 'grey', ['queued']],
  ['running', 'Working', 'blue', ['running']],
  ['waiting', 'Waiting for an answer', 'amber', ['waiting']],
  ['merging', 'Merging', 'purple', ['merge-queued', 'merging']],
  ['merged', 'Done', 'green', ['merged']],
  ['failed', 'Failed / Stopped', 'red', ['failed', 'stopped']],
];
const STOPPABLE = ['queued', 'running', 'waiting', 'merge-queued'];
const DOT = { running: 'blue pulse', queued: 'grey', waiting: 'amber', 'merge-queued': 'blue', merging: 'blue', merged: 'green', failed: 'red', stopped: 'grey' };

const startOf = (a) => a.started_at || a.created_at;
const endOf = (a) => (['queued', 'running', 'waiting', 'merge-queued', 'merging'].includes(a.status) ? null : a.ended_at || a.merged_at || null);
const newest = (a) => ms(a.merged_at || a.ended_at || a.started_at || a.created_at) || 0;
const isOld = (a) => ['merged', 'failed', 'stopped'].includes(a.status) && Date.now() - newest(a) > DAY;
const itemText = (t) => String(t || '').replace(/\s*\(check:\s*`?[^)]*`?\)\s*$/, '');
const live = (a) => a.status === 'running' && a.last_activity && Date.now() / 1000 - a.last_activity < 90;
/** The open question of a waiting lane: a card for the owner when the manager passed it on (--lane), else the lane's own. */
const questionOf = (s, a) => (a.status === 'waiting' ? openCards(s).find((o) => o.lane === a.id) || null : null);
const forYou = (q) => !!q && q.kind !== 'lane';
const statusOf = (a, q) => (forYou(q) ? 'Waiting for you' : laneLabel(a.status));

function loadMode() {
  const u = new URLSearchParams(location.search).get('mode');
  if (u === 'list' || u === 'kanban') return u;
  try { return localStorage.getItem('tirith.agents.mode') === 'list' ? 'list' : 'kanban'; } catch { return 'kanban'; } }
function saveMode(m) { try { localStorage.setItem('tirith.agents.mode', m); } catch { /* private window */ } }

function Avatar({ a, color, size = 36 }) {
  const ch = String(a.agent || a.id || '?').trim().charAt(0).toUpperCase();
  return html`<span class="avatar lava" style=${`width:${size}px;height:${size}px;background:linear-gradient(135deg, ${color}, color-mix(in srgb, ${color} 55%, #1d2433));font-size:${Math.round(size * 0.42)}px`}>${ch}</span>`;
}

/** Stop with a confirm popover (the old stop.js): the lane ends as stopped and its worktree is kept. */
function StopBtn({ a, where }) {
  const [ask, setAsk] = useState(false);
  const [pend, setPend] = useState(false);
  const [err, setErr] = useState('');
  if (!STOPPABLE.includes(a.status)) return null;
  if (pend) return html`<span class=${`stopb stopping ${where}`} role="status">Stopping…</span>`;
  const stop = (e) => {
    e.stopPropagation();
    setAsk(false); setPend(true); setErr('');
    api('POST', `/api/lanes/${encodeURIComponent(a.id)}/stop`).catch((x) => { setPend(false); setErr(`Could not stop: ${x.message}`); });
  };
  const toggle = (e) => { e.stopPropagation(); setAsk(!ask); };
  return html`<span class=${`stopb ${where}`} role="button" tabindex="0" title="Stop this lane: it ends as stopped and its worktree is kept"
      onClick=${toggle} onKeyDown=${(e) => (e.key === 'Enter' || e.key === ' ') && toggle(e)}>
      <svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true"><rect x="2.5" y="2.5" width="7" height="7" rx="1.4" fill="currentColor" /></svg><span>Stop</span></span>
    ${ask ? html`<span class=${`stoppop ${where}`} role="dialog" aria-label="Confirm stop" onClick=${(e) => e.stopPropagation()}>
      <span class="sptxt">Stop <b>${itemText(a.item)}</b>? It ends as stopped and its worktree is kept.</span>
      <span class="spb"><span class="stopgo" role="button" tabindex="0" onClick=${stop}>Stop agent</span>
      <span class="alink" role="button" tabindex="0" onClick=${toggle}>Cancel</span></span></span>` : null}
    ${err ? html`<span class="ierr stoperr">${err}</span>` : null}`;
}

function Clock({ a }) {
  return html`<span class="aclock"><${Icon} d=${ICON.clock} size=${13} w=${1.7} /><${Span} start=${startOf(a)} end=${endOf(a)} /></span>`;
}

function LaneCard({ a, sel, color, projName, onOpen, doing, q }) {
  const work = a.status === 'running';
  const head = html`<span class="ahead"><${Avatar} a=${a} color=${color} /><span class="aname"><b title=${a.agent}>${a.agent || 'agent'}</b>
    <span class="pill" title=${a.id}>${a.model || a.id}</span></span></span>`;
  const now = q ? html`<span class="anow ask"><i class="pdot2"></i>${forYou(q) ? html`<b>For you: </b>` : null}<${MdInline} text=${q.question} /></span>`
    : work && doing ? html`<span class=${`anow${live(a) ? ' live' : ''}`} title=${doing.summary}><i class="pdot2"></i><span class="md">${doing.summary}</span></span>`
    : (work || a.status === 'failed') && a.note ? html`<span class=${`anow${live(a) ? ' live' : ''}${a.status === 'failed' ? ' bad' : ''}`}><i class="pdot2"></i><${MdInline} text=${a.note} /></span>` : null;
  return html`<button class=${`card acard${work ? ' work' : ''}${sel ? ' sel' : ''}`} onClick=${() => onOpen(a.id)}>
    ${head}<span class="afeat">${itemText(a.item)}</span>${now}
    <span class="afoot">${a.attempts ? html`<span class="chip atry" title="Sent back by the merge checks">Try ${a.attempts + 1}</span>` : null}
      ${a.step && work ? html`<span class="chip astep" title="Plan step">Step ${a.step}</span>` : null}
      <span class="aproj"><i class="pdot" style=${`background:${color};animation:none`}></i>${projName}</span><${Clock} a=${a} /></span>
    ${work ? html`<${StopBtn} a=${a} where="card" />` : null}
  </button>`;
}

/** The lane's log file, reloaded (debounced) while its lane reports activity. Plain text, never parsed as one JSON. */
function useLaneLog(id, active) {
  const [text, setText] = useState(null);
  useEffect(() => {
    let alive = true, t = 0;
    const load = () => fetch(`/api/lanes/${encodeURIComponent(id)}/log`).then((r) => r.text()).then((x) => alive && setText(x), () => alive && setText(''));
    setText(null);
    load();
    if (!active) return () => { alive = false; };
    const off = onMessage((m) => {
      if ((m.t === 'event' && m.d && m.d.lane === id) || (m.t === 'lane' && m.d && m.d.id === id)) { clearTimeout(t); t = setTimeout(load, 700); }
    });
    return () => { alive = false; off(); clearTimeout(t); };
  }, [id, active]);
  return text;
}

function Sec({ title, n, children }) {
  return html`<section class="asec"><h4>${title}${n ? html` <span class="n">${n}</span>` : null}</h4>${children}</section>`;
}

function KV({ rows }) {
  return html`<div class="lcard">${rows.filter((r) => r[1] != null && r[1] !== '').map(([k, v]) => html`<div class="lkv"><span class="lk">${k}</span><span class="lv">${v}</span></div>`)}</div>`;
}

function LanePipelines({ id }) {
  const [list] = useFetch('/api/pipelines?limit=50', ['pipeline']);
  if (!list) return html`<p class="quiet sm">Loading…</p>`;
  const mine = list.filter((p) => laneIds(p).includes(id)).sort((x, y) => startMs(y) - startMs(x));
  if (!mine.length) return html`<p class="quiet sm">No pipeline ran for this lane yet</p>`;
  const open = (p) => { setUi('openPipeline', p.id); go('pipelines', p.project || ''); };
  return html`<ul class="lpipes">${mine.map((p) => {
    const [kind, num] = pipeName(p);
    return html`<li key=${p.id}><button class="lprow" onClick=${() => open(p)}>
      <span class=${`pst s-${sClass(p.status)}`}>${glyph(p.status, 10)}</span><b>${kind} <span class="mut">${num}</span></b>
      <span class="pmini">${(p.stages || []).map((st) => html`<${StageChip} key=${st.id} st=${st} />`)}</span>
      <span class="mut sm lpwhen">${ago(p.started_at)}${ago(p.started_at) === 'now' ? '' : ' ago'}</span></button></li>`;
  })}</ul>`;
}

function Events({ id }) {
  const s = useStore();
  const l = [];
  for (let i = s.events.length - 1; i >= 0 && l.length < 12; i--) if (s.events[i].lane === id && s.events[i].tool) l.push(s.events[i]);
  if (!l.length) return html`<p class="quiet sm">No activity yet</p>`;
  return html`<ul class="erows">${l.map((e) => html`<li key=${e.id}><span class="mut t">${clockTime(e.at)}</span><span class="tool">${e.tool}</span>
    <span class="what" title=${e.summary}>${e.summary}</span></li>`)}</ul>`;
}

function PlanItem({ a }) {
  const [plan] = useFetch(a.project ? `/api/plan/${encodeURIComponent(a.project)}` : '', ['plan']);
  const step = plan && (plan.steps || []).find((x) => x.num === a.step);
  const it = step && (step.items || []).find((x) => x.text === a.item);
  return html`<div class="agoal"><${Md} text=${itemText(a.item)} /></div>
    ${it && it.check ? html`<p class="lcheck"><span class="mut">Check</span> <code>${it.check}</code></p>` : null}
    ${step ? html`<p class="mut sm lstep">Step ${step.num}${step.title ? ` · ${step.title}` : ''}</p>` : null}`;
}

/** Everything about one lane (drawer body, or the open row in the list). */
function LaneDetail({ a, projName, color }) {
  const q = questionOf(useStore(), a);
  const active = ['queued', 'running', 'waiting', 'merge-queued', 'merging'].includes(a.status);
  const log = useLaneLog(a.id, active);
  const st = a.status === 'running' ? 'active' : a.status === 'merged' ? 'done' : ['failed', 'stopped'].includes(a.status) ? 'failed' : 'idle';
  const last = a.last_activity ? new Date(a.last_activity * 1000).toISOString() : '';
  return html`<div class="activity">
    ${STOPPABLE.includes(a.status) ? html`<div class="stoprow"><${StopBtn} a=${a} where="row" /></div>` : null}
    <${Sec} title="Item"><${PlanItem} a=${a} /></${Sec}>
    <${Sec} title="Status"><div class="astat"><span class=${`sbadge ${st}${forYou(q) ? ' you' : ''}`}>${st === 'active' ? html`<i class="pdot2"></i>` : null}${statusOf(a, q)}</span>
      <span class="mut">${a.status === 'waiting' ? 'open for ' : active ? 'running ' : 'took '}<${Span} start=${startOf(a)} end=${endOf(a)} /></span>
      ${active && last ? html`<span class="mut">last activity ${ago(last) === 'now' ? 'just now' : `${ago(last)} ago`}</span>` : null}
      ${a.attempts ? html`<span class="mut">try ${a.attempts + 1} of 3</span>` : null}</div></${Sec}>
    ${q ? html`<${Sec} title=${forYou(q) ? 'Question for you' : 'Question to the manager'}>${forYou(q) ? null : html`<div class="lquest"><${Md} text=${a.question} /></div>`}
      <ul class="nlist lneed"><${Need} o=${forYou(q) ? q : { ...q, question: 'You can answer it too' }} full here /></ul></${Sec}>` : null}
    ${a.note ? html`<${Sec} title=${a.status === 'failed' ? 'Why it failed' : 'Report'}><div class=${`sum${a.status === 'failed' ? ' lbad' : ''}`}><${Md} text=${a.note} /></div></${Sec}>` : null}
    <${Sec} title="Lane"><${KV} rows=${[
      ['Project', html`<span><i class="pdot" style=${`background:${color};animation:none;display:inline-block;margin-right:6px`}></i>${projName}</span>`],
      ['Step', a.step], ['Branch', a.branch && html`<code>${a.branch}</code>`], ['Worktree', a.workspace && html`<code title=${a.workspace}>${a.workspace}</code>`],
      ['Agent', a.agent], ['Model', a.model], ['Session', a.session && html`<code>${a.session}</code>`], ['Attempts', String(a.attempts || 0)],
      ['Created', clockTime(a.created_at)], ['Started', clockTime(a.started_at)], ['Ended', clockTime(a.ended_at)], ['Merged', clockTime(a.merged_at)],
      ['Lane', html`<code>${a.id}</code>`],
    ]} /></${Sec}>
    <${Sec} title="Log">${log == null ? html`<p class="quiet sm">Loading the log…</p>` : html`<${LogView} text=${log} />`}</${Sec}>
    <${Sec} title="Live activity"><${Events} id=${a.id} /></${Sec}>
    <${Sec} title="Pipelines"><${LanePipelines} id=${a.id} /></${Sec}>
  </div>`;
}

function Drawer({ a, projName, color, onClose }) {
  const q = questionOf(useStore(), a);
  useEffect(() => {
    const key = (e) => { if (e.key === 'Escape' && !/^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement.tagName)) onClose(); };
    addEventListener('keydown', key);
    return () => removeEventListener('keydown', key);
  }, [onClose]);
  return html`<aside class="drawer" role="dialog" aria-label=${itemText(a.item)}>
    <header><span class="dhead"><${Avatar} a=${a} color=${color} size=${28} /><span class="dtit"><b>${itemText(a.item)}</b>
      <span class="mut">${a.agent}${a.model ? ` · ${a.model}` : ''} · ${statusOf(a, q)}</span></span></span>
      <button class="x" aria-label="Close" onClick=${onClose}><${Icon} d=${ICON.close} size=${14} w=${2} /></button></header>
    <div class="dbody" key=${a.id}><${LaneDetail} a=${a} projName=${projName} color=${color} /></div>
  </aside>`;
}

function Column({ col, lanes, sel, onOpen, colorOf, nameOf, doing, qOf }) {
  const [cap, setCap] = useState(CAP);
  return html`<section class=${`col ${col[2]}`}>
    <header><b>${col[1]}</b><span class="n">${lanes.length}</span></header>
    <div class="cbody">${lanes.slice(0, cap).map((a) => html`<${LaneCard} key=${a.id} a=${a} sel=${sel === a.id} color=${colorOf(a.project)} projName=${nameOf(a.project)} onOpen=${onOpen} doing=${doing[a.id]} q=${qOf(a)} />`)}
      ${lanes.length > cap ? html`<button class="link lshow" onClick=${() => setCap(cap + CAP)}>Show ${Math.min(CAP, lanes.length - cap)} more</button>` : null}</div>
  </section>`;
}

function ListRow({ a, open, onOpen, color, projName }) {
  const q = questionOf(useStore(), a);
  return html`<li class=${`row${open ? ' open' : ''}`}>
    <button class="head agent" aria-expanded=${open} onClick=${() => onOpen(open ? '' : a.id)}>
      <span class="chv"></span><i class=${`dot ${DOT[a.status] || 'grey'}`} title=${laneLabel(a.status)}></i>
      <span class="name" title=${a.id}>${a.agent} <span class="mut sm">${statusOf(a, q)}${a.attempts ? ` · try ${a.attempts + 1}` : ''}</span></span>
      <span class="feat">${itemText(a.item)}</span><span class="mut proj">${projName}</span>
      <span class="time"><${Span} start=${startOf(a)} end=${endOf(a)} /></span></button>
    ${open ? html`<div class="detail"><${LaneDetail} a=${a} projName=${projName} color=${color} /></div>` : null}
  </li>`;
}

export function Agents({ project }) {
  const s = useStore();
  const [mode, setMode] = useState(loadMode);
  const [sel, setSel] = useState(() => new URLSearchParams(location.search).get('lane') || '');
  const [earlier, setEarlier] = useState(false);
  const [cap, setCap] = useState(60);

  useEffect(() => { // another view asked to open a lane (setUi('openLane', id))
    if (s.openLane) { setSel(s.openLane); s.openLane = ''; }
  }, [s.openLane]);

  const colorOf = (id) => projectColor(s.projects, id);
  const nameOf = (id) => ((s.projects.find((p) => p.id === id) || {}).name || id);
  const q = String(s.q || '').trim().toLowerCase();
  const all = useMemo(() => Object.values(s.lanes)
    .filter((a) => !project || a.project === project)
    .filter((a) => !q || [a.item, a.id, a.agent, a.model, a.note, a.branch, a.status, a.question].some((x) => String(x || '').toLowerCase().includes(q)))
    .sort((x, y) => newest(y) - newest(x)), [s.updated, project, q]);
  const doing = useMemo(() => { // each lane's last tool call, for the "now doing" line of a working card
    const m = {};
    for (const e of s.events) if (e.lane && e.tool && e.kind === 'PreToolUse') m[e.lane] = e;
    return m;
  }, [s.events.length, s.events[s.events.length - 1]]);
  const old = all.filter(isOld);
  const vis = earlier ? all : all.filter((a) => !isOld(a));
  const cur = sel && s.lanes[sel];

  const tb = html`<div class="toolbar"><div class="seg" role="group" aria-label="Layout">
    ${[['list', 'List'], ['kanban', 'Kanban']].map(([m, label]) => html`<button class=${mode === m ? 'on' : ''} onClick=${() => { setMode(m); saveMode(m); }}>${label}</button>`)}</div></div>`;
  const more = old.length ? html`<button class="link" onClick=${() => setEarlier(!earlier)}>${earlier ? 'Hide earlier' : `Show ${old.length} earlier`}</button>` : null;

  if (!all.length) {
    return [tb, html`<div class="pad"><p class="quiet">${q ? 'No lanes match' : `No agents${project ? ' in this project' : ''} yet: lanes appear here when the plan or the manager starts them`}</p></div>`];
  }
  if (mode === 'list') {
    const rank = (a) => (a.status === 'running' ? 0 : a.status === 'waiting' ? 1 : ['queued', 'merge-queued', 'merging'].includes(a.status) ? 2 : 3);
    const rows = vis.slice().sort((x, y) => rank(x) - rank(y) || newest(y) - newest(x));
    return [tb, html`<div class="listwrap"><ul class="rows card">${rows.slice(0, cap).map((a) => html`<${ListRow} key=${a.id} a=${a} open=${sel === a.id}
        onOpen=${setSel} color=${colorOf(a.project)} projName=${nameOf(a.project)} />`)}</ul>
      ${rows.length > cap ? html`<button class="link lshow" onClick=${() => setCap(cap + 60)}>Show ${Math.min(60, rows.length - cap)} more</button>` : null} ${more}</div>`];
  }
  return [tb,
    html`<div class="board lboard">${COLS.filter((c) => !cur || vis.some((a) => c[3].includes(a.status))).map((c) => html`<${Column} key=${c[0]} col=${c} lanes=${vis.filter((a) => c[3].includes(a.status))}
      sel=${cur ? sel : ''} onOpen=${(id) => setSel(sel === id ? '' : id)} colorOf=${colorOf} nameOf=${nameOf} doing=${doing} qOf=${(a) => questionOf(s, a)} />`)}</div>`,
    more ? html`<div class="moreline">${more}</div>` : null,
    cur ? html`<${Drawer} a=${cur} projName=${nameOf(cur.project)} color=${colorOf(cur.project)} onClose=${() => setSel('')} />` : null,
  ];
}

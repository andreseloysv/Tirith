// Overview: the old page's .ovg grid. With no project selected the main column shows everything: status (metrics
// and the CEO's last review), every plan, what needs you, running pipelines, working agents. Inside a project it
// shows that project only: its plan, its cards, and its setup, pipelines and agents when there is something to see;
// the health of the whole and the CEO stay out. The chat on the right is the selected project's conversation.
import { html, useState } from '../../vendor/preact-htm.js';
import { useStore, useFetch, api } from '../store.js';
import { ICON, Icon, Span, ACTIVE, ago, clockTime, laneLabel, laneClass, go } from '../common.js';
import { Chat } from './chat.js';
import { Plan } from './overview/plan.js';
import { memo, Svg, Avatar, Rich, CHEV, REFRESH, clip, hit } from './overview/bits.js';

/* ---------- status: metrics and the CEO ---------- */
const num = (v) => (v == null ? '–' : Number.isInteger(v) ? String(v) : (Math.round(v * 10) / 10).toString());

function Metric({ m }) {
  const target = m.target == null ? '' : `${m.at_least ? '≥' : '≤'} ${num(m.target)}`;
  return html`<span class="stm" title=${m.facts || m.label}><i class=${`dot ${m.target == null ? 'grey' : m.red ? 'red' : 'green'}`}></i>${m.label}${' '}<b class=${m.red ? 'bad' : ''}>${num(m.value)}</b>${target ? html`<span class="mut"> / ${target}</span>` : null}</span>`;
}

function Ceo({ reviews, reviewing }) {
  const [open, setOpen] = useState(false);
  const [err, setErr] = useState('');
  const [asked, setAsked] = useState(false);
  const last = reviews[reviews.length - 1];
  const busy = reviewing || asked;
  const now = () => {
    setErr(''); setAsked(true);
    api('POST', '/api/ceo/review').catch((e) => setErr(String(e.message || e))).finally(() => setTimeout(() => setAsked(false), 3000));
  };
  return html`<div class="stl stceo"><span class="stk">CEO</span>
      ${busy ? html`<span class="ovlive"><i class="pdot2"></i>Reviewing how the work is run…</span>`
        : last ? html`<button class="iahead ovrev" aria-expanded=${open} onClick=${() => setOpen(!open)}>
            <${Svg} s=${CHEV} /><span><span class="mut">Last review ${clockTime(last.at)}${last.metric ? ` · ${last.metric}` : ''} ·</span> ${clip(last.text, 110)}</span></button>`
        : html`<span class="mut">No review yet</span>`}
      <button class="ibtn ovnow" disabled=${busy} onClick=${now} title="Ask the CEO to review how the work is run now">
        <${Svg} s=${REFRESH} /><span>Review now</span></button></div>
    ${open && last && !busy ? html`<div class="ia ovrevbody">${last.text}</div>` : null}
    ${err ? html`<div class="ierr ovinl">${err}</div>` : null}`;
}

const Status = memo(function Status({ data, working }) {
  const list = (data && data.metrics) || [];
  const reviews = (data && data.reviews) || [];
  if (!data) return null;
  const red = list.filter((m) => m.red);
  const merged = list.find((m) => m.id === 'merged');
  const why = red.length ? `${red.length} ${red.length === 1 ? 'metric' : 'metrics'} off target: ${red.map((m) => m.label).join(', ')}` : 'Every metric on target';
  return html`<section class=${`card stcard ${red.length ? 'bad' : 'ok'}`}>
    <div class="stl stl1"><b>${red.length ? '🔴' : '🟢'} ${why}</b>${merged ? ` · ${num(merged.value)} on main in 24 h` : ''} · ${working} ${working === 1 ? 'agent' : 'agents'} working</div>
    <div class="stl stms"><span class="stk">Metrics</span><span class="stmlist">${list.map((m) => html`<${Metric} key=${m.id} m=${m} />`)}</span></div>
    <${Ceo} reviews=${reviews} reviewing=${!!data.reviewing} />
  </section>`;
});

/* ---------- needs you ---------- */
const KIND = { credentials: 'Credentials', payment: 'Payment', account: 'Account', device: 'Device', decision: 'Decision' };

function Need({ o, p }) {
  const [draft, setDraft] = useState('');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  const send = async (answer) => {
    answer = String(answer || '').trim();
    if (!answer || busy) return;
    setBusy(true); setErr('');
    try { await api('POST', `/api/owner/${encodeURIComponent(o.id)}`, { answer }); setDraft(''); } catch (e) { setErr(String(e.message || e)); }
    setBusy(false);
  };
  return html`<li class="ncard">
    <div class="nhead"><div class="ntitle">${o.question}</div>${p ? html`<span class="nchip nproj"><i class="pdot" style=${`background:${p.color || '#9ca3af'}`}></i>${p.name || p.id}</span>` : null}
      <span class="nchip">${KIND[o.kind] || o.kind}</span><span class="nage">${ago(o.at)}</span></div>
    ${o.kind === 'decision' ? null : html`<div class="nacts"><button class="nbtn primary" disabled=${busy} onClick=${() => send('Done')}>Done</button></div>`}
    <div class="nwrite"><input class="iin" type="text" placeholder=${o.kind === 'decision' ? 'Your answer…' : 'Or write what you need…'} aria-label="Answer"
      value=${draft} disabled=${busy} onInput=${(e) => setDraft(e.target.value)} onKeyDown=${(e) => { if (e.key === 'Enter') send(draft); }} />
      <button class="isend ovsend" disabled=${busy || !draft.trim()} onClick=${() => send(draft)}>${busy ? 'Sending…' : 'Answer'}</button></div>
    ${err ? html`<div class="ierr">${err}</div>` : null}
  </li>`;
}

/** The open cards: inside a project only its own; with none selected all of them, each named by its project. */
function Needs({ owner, project, projects, q }) {
  const list = owner.filter((o) => (!project || o.project === project) && hit(q, o.question, o.kind));
  if (!list.length) return null;
  return html`<section class="needs-sec"><h2>Needs you <span class="icount">${list.length}</span></h2>
    <ul class="nlist">${list.map((o) => html`<${Need} key=${o.id} o=${o} p=${project ? null : projects.find((x) => x.id === o.project)} />`)}</ul></section>`;
}

/* ---------- setup (commit guard) of the routed project: drawn only when it needs attention ---------- */
function Setup({ project, name }) {
  const [guard, reload] = useFetch('/api/guard', []);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  const g = (guard || []).find((x) => x.project === project);
  if (!g || !g.words || g.ok) return null;
  const none = !g.words, on = g.ok && !none;
  const install = async () => {
    setBusy(true); setErr('');
    try { await api('POST', `/api/guard/${encodeURIComponent(project)}/install`); reload(); } catch (e) { setErr(String(e.message || e)); }
    setBusy(false);
  };
  const state = none ? ['grey', 'No commit guard', 'no forbidden words are set for this project in config.toml']
    : on ? ['green', 'Commit guard installed', `${g.words} forbidden ${g.words === 1 ? 'word' : 'words'} blocked in commits`]
    : ['red', 'Commit guard needs attention', g.reason];
  return html`<section><h2>Setup</h2><div class="card scard"><div class="phead"><b>${g.name || name}</b><span class="mut sm">Setup</span></div>
    ${none || on ? null : html`<div class="sban" role="alert"><span>Agents could commit forbidden words here, because the commit guard is not intact.</span></div>`}
    <div class="srow"><span class="slabel">Guard</span><div class="sbody">
      <div class=${`scheck ${on ? 'ok' : none ? '' : 'bad'}`}><span class=${`dot ${state[0]}`}></span><b>${state[1]}</b><span class="mut sm"> · ${state[2]}</span>
        ${none || on ? null : html`<button class="srun ovinst" disabled=${busy} onClick=${install}><${Icon} d=${ICON.guard} size=${11} w=${2} /><span>${busy ? 'Installing…' : 'Install'}</span></button>`}
      </div></div></div>
    ${err ? html`<div class="ierr">${err}</div>` : null}
  </div></section>`;
}

/* ---------- running pipelines (the Pipelines view's compact rows) ---------- */
const GLYPH = { passed: '<path d="M3.2 6.4l2.1 2.1 3.6-4.2"/>', failed: '<path d="M3.6 3.6l4.8 4.8M8.4 3.6L3.6 8.4"/>', skipped: '<path d="M2.6 3.4l2.6 2.6-2.6 2.6M6.2 3.4l2.6 2.6-2.6 2.6"/>' };
const glyph = (s, n) => (GLYPH[s] ? `<svg viewBox="0 0 12 12" width="${n}" height="${n}" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${GLYPH[s]}</svg>` : '');
const mark = (s) => (s === 'running' ? html`<i class="pdot"></i>` : s === 'pending' ? html`<i class="pring"></i>` : html`<${Svg} s=${glyph(s, 11)} />`);

function PipeRow({ p, color, name }) {
  const num = String(p.id).split('-').pop();
  const fail = (p.stages || []).find((x) => x.status === 'failed');
  const lane = String(p.lanes || '').split(',').filter(Boolean)[0];
  return html`<div class=${`prow2 s-${p.status} ovpipe`} onClick=${() => go('pipelines', p.project)} role="button" tabindex="0">
    <span class=${`pst s-${p.status}`}>${p.status === 'running' ? html`<i class="pspin"></i>` : p.status === 'pending' ? html`<i class="pring"></i>` : html`<${Svg} s=${glyph(p.status, 12)} />`}</span>
    <span class="pc pid"><span class="plink">${p.kind || 'pipeline'} <span class="mut">#${num}</span></span>
      ${name ? html`<span class="pchip"><i style=${`background:${color}`}></i>${name}</span>` : null}</span>
    <span class="pc ptr"><span class="ptw"><span class="ptw-ic"><${Icon} d=${ICON.guard} size=${12} w=${2} /></span><b>${lane || 'main'}</b></span></span>
    <span class="pc pmini">${(p.stages || []).map((st) => html`<span key=${st.id} class=${`pchp s-${st.status}`} title=${`${st.stage}: ${st.status}`}>${mark(st.status)}<span>${st.stage}</span></span>`)}
      ${fail ? html`<span class="pfail">failed at <b>${fail.stage}</b></span>` : null}</span>
    <span class="pc pdur">${p.status === 'pending' ? '' : html`<${Span} start=${p.started_at} end=${p.ended_at || null} />`}</span>
    <span class="pc pwhen" title=${clockTime(p.started_at)}>${ago(p.started_at)}${ago(p.started_at) === 'now' ? '' : html`<span class="mut"> ago</span>`}</span>
  </div>`;
}

function Pipelines({ project, projects, q }) {
  const [list] = useFetch(`/api/pipelines?limit=30${project ? `&project=${encodeURIComponent(project)}` : ''}`, ['pipeline', 'run']);
  const run = (list || []).filter((p) => p && (p.status === 'running' || p.status === 'pending') && (!project || p.project === project) && hit(q, p.kind, p.project, p.lanes, p.id));
  if (project && !run.length) return null;
  const pr = (id) => projects.find((x) => x.id === id) || {};
  return html`<section><h2>Pipelines running <span class="n">${run.length}</span></h2>
    ${run.length ? html`<div class="ptable">${run.map((p) => html`<${PipeRow} key=${p.id} p=${p} color=${pr(p.project).color || '#9aa1ac'} name=${project ? '' : pr(p.project).name || p.project} />`)}</div>`
      : html`<p class="quiet">Nothing is running</p>`}</section>`;
}

/* ---------- working agents (the old agent cards) ---------- */
const noCheck = (t) => String(t || '').replace(/\s*\(check:\s*`[^`]*`\)\s*/i, ' ').trim();
const AgentCard = memo(function AgentCard({ l, now, pname }) {
  const live = l.status === 'running';
  return html`<button class="card acard work" onClick=${() => go('agents', l.project)} title="Open the Agents view">
    <span class="ahead"><${Avatar} id=${l.id} size=${36} /><span class="aname"><b>${l.agent ? l.agent.charAt(0).toUpperCase() + l.agent.slice(1) : 'Worker'}</b>
      <span class="pill" title=${l.id}>${[l.id, l.model].filter(Boolean).join(' · ')}</span></span></span>
    ${l.item ? html`<span class="afeat"><${Rich} t=${noCheck(l.item)} /></span>` : null}
    ${now ? html`<span class=${`anow${live ? ' live' : ''}`}><i class="pdot2"></i><span class="md">${clip(now, 220)}</span></span>` : null}
    <span class="afoot"><span class=${`rb ${laneClass(l.status) === 'working' ? 'running' : 'pending'}`}>${laneLabel(l.status)}</span>
      ${l.step ? html`<span class="chip astep" title="Plan step">Step ${l.step}</span>` : null}
      <span class="aproj">${pname}</span>
      <span class="aclock"><${Icon} d=${ICON.clock} size=${13} w=${1.7} /><${Span} start=${l.started_at || l.created_at} /></span></span>
  </button>`;
});

function Agents({ lanes, events, projects, project, q }) {
  const act = lanes.filter((l) => ACTIVE.includes(l.status) && (!project || l.project === project) && hit(q, l.item, l.id, l.agent, l.project))
    .sort((a, b) => String(a.created_at).localeCompare(String(b.created_at)));
  const last = {};
  for (let i = events.length - 1; i >= 0; i--) { const e = events[i]; if (e.lane && !last[e.lane] && e.summary) last[e.lane] = e.summary; }
  const pname = (id) => (project ? '' : (projects.find((x) => x.id === id) || {}).name || id);
  if (project && !act.length) return null;
  return html`<section><h2>Working agents <span class="n">${act.length}</span></h2>
    ${act.length ? html`<div class="grid2">${act.map((l) => html`<${AgentCard} key=${l.id} l=${l} now=${last[l.id] || l.question || l.note} pname=${pname(l.project)} />`)}</div>`
      : html`<p class="quiet">No agents working</p>`}</section>`;
}

export function Overview({ project }) {
  const s = useStore();
  const [metrics] = useFetch(project ? '' : '/api/metrics', ['metrics', 'ceo']);
  const q = String(s.q || '').trim().toLowerCase();
  const lanes = Object.values(s.lanes);
  const projects = project ? s.projects.filter((p) => p.id === project) : s.projects;
  const working = lanes.filter((l) => ACTIVE.includes(l.status) && (!project || l.project === project)).length;
  const p = s.projects.find((x) => x.id === project);
  return html`<div class="ovg"><div class="ovmain">
      ${project ? null : html`<${Status} data=${metrics} working=${working} />`}
      <${Plan} projects=${projects} lanes=${lanes} q=${q} />
      <${Needs} owner=${s.owner} project=${project} projects=${s.projects} q=${q} />
      ${p ? html`<${Setup} project=${p.id} name=${p.name} />` : null}
      <${Pipelines} project=${project} projects=${s.projects} q=${q} />
      <${Agents} lanes=${lanes} events=${s.events} projects=${s.projects} project=${project} q=${q} />
    </div>
    <aside class="ovside"><${Chat} key=${project || ''} project=${project || ''} /></aside></div>`;
}

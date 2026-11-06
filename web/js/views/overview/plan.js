// The Plan section of the Overview (old v-plan.js): one card per project, a row per step with its bar, the current
// step unfolded to its open items, each with the state of its latest lane; the autopilot line and adding items.
import { html, useState, useEffect, useRef } from '../../../vendor/preact-htm.js';
import { api, onMessage } from '../../store.js';
import { laneLabel, ACTIVE } from '../../common.js';
import { memo, Svg, IC, OK, OPEN, PLUS, PAUSE, RESUME, Rich, hit } from './bits.js';

const CHECK = /\s*\(check:\s*`[^`]*`\)\s*/i;

/** GET /api/plan/<id>, refetched when the daemon says this project's plan changed or one of its lanes merged. */
function usePlan(id) {
  const [st, set] = useState({ data: null, error: null });
  useEffect(() => {
    let t = 0, live = true;
    const load = () => api('GET', `/api/plan/${encodeURIComponent(id)}`).then(
      (data) => live && set({ data, error: null }), (error) => live && set((s) => ({ data: s.data, error })));
    load();
    const off = onMessage((m) => {
      const d = m.d || {};
      const mine = (m.t === 'plan' && (!d.project || d.project === id)) || (m.t === 'lane' && d.project === id && d.status === 'merged');
      if (!mine) return;
      clearTimeout(t);
      t = setTimeout(load, 250);
    });
    return () => { live = false; off(); clearTimeout(t); };
  }, [id]);
  return st;
}

function Bar({ done, total, cls }) {
  const pct = total ? Math.round((100 * done) / total) : 0;
  return html`<span class=${`bar ${cls || ''}`}><i class=${done === total && total ? 'green' : 'blue'} style=${`width:${pct}%`}></i></span>`;
}

/** The old item state chip, from the item's latest lane. */
function LaneChip({ lane }) {
  if (!lane) return null;
  const s = lane.status;
  const cls = s === 'running' || s === 'queued' ? 'progress' : s === 'failed' ? 'blocked' : s === 'stopped' ? 'reopened' : '';
  if (s === 'merged') return null;
  return html`<span class=${`pstate ${cls}`} title=${`Lane ${lane.id}`}>${laneLabel(s)}</span>`;
}

function Item({ i, lane }) {
  const text = i.text.replace(CHECK, ' ').trim();
  return html`<li class=${i.done ? 'ok' : ''}>
    <${Svg} cls="pi" s=${i.done ? OK : OPEN} />
    <span class="pit"><${Rich} t=${text} /></span>
    ${i.check ? html`<span class="pwhen pchk" title="The command that proves this item is done"><code>${i.check}</code></span>` : null}
    ${i.done ? null : html`<${LaneChip} lane=${lane} />`}
  </li>`;
}

/** The "Add item" row at the end of a step's items, and its input. */
function AddItem({ project, step, editing, setEditing }) {
  const [draft, setDraft] = useState('');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  const ref = useRef(null);
  useEffect(() => { if (editing && ref.current) ref.current.focus(); }, [editing]);
  if (!editing) {
    return html`<li class="padd"><button class="pedadd" title="Add an item to this step" onClick=${() => setEditing(true)}>
      <${Svg} s=${PLUS} /><span>Add item</span></button></li>`;
  }
  const save = async () => {
    const text = draft.trim();
    if (!text || busy) return;
    setBusy(true); setErr('');
    try {
      await api('POST', `/api/plan/${encodeURIComponent(project)}`, { step, text });
      setDraft(''); setEditing(false);
    } catch (e) { setErr(String(e.message || e)); }
    setBusy(false);
  };
  return html`<li class="pediting"><${Svg} cls="pi" s=${OPEN} />
      <input ref=${ref} class="iin pedin" type="text" maxlength="500" aria-label="New item" placeholder="What needs doing?" value=${draft}
        disabled=${busy} onInput=${(e) => setDraft(e.target.value)}
        onKeyDown=${(e) => { if (e.key === 'Enter') save(); else if (e.key === 'Escape') { setEditing(false); setErr(''); } }} />
      <span class="ihint">${busy ? 'Saving…' : 'Enter saves · Esc cancels'}</span></li>
    ${err ? html`<li class="perr"><div class="ierr">${err}</div></li>` : null}`;
}

function Step({ project, s, mode, onToggle, laneOf, q, adding, setAdding }) {
  const items = q ? s.items.filter((i) => hit(q, i.text, s.title)) : s.items;
  const fin = s.status === 'done';
  const live = s.items.some((i) => !i.done && laneOf(i) && ACTIVE.includes(laneOf(i).status));
  const shown = mode === 'all' ? items : mode === 'quiet' ? items.filter((i) => !i.done) : [];
  const open = mode !== 'closed';
  return html`<li class=${`pstep ${s.status}${mode === 'all' ? ' open' : ''}`}>
    <div class="prow">
      <button class="ptog" aria-expanded=${open} onClick=${onToggle}>
        <${Svg} cls=${`pico${live ? ' pulse' : ''}`} s=${IC[s.status] || IC.todo} />
        <span class="ptit"><span class="pn">${s.num}.</span> ${s.title}</span>
        ${s.total ? html`<${Bar} done=${s.done} total=${s.total} cls="pbar" />` : html`<span class="pbar"></span>`}
        <span class="pcnt">${s.total ? `${s.done}/${s.total}` : '–'}</span>
      </button>
      <span class="pedtools pstit"><button class="pedbtn" title="Add item" aria-label="Add item" onClick=${() => setAdding(s.num)}><${Svg} s=${PLUS} /></button></span>
    </div>
    ${shown.length || adding ? html`<ul class=${`pitems${mode === 'quiet' ? ' pquiet' : ''}`}>
      ${shown.map((i) => html`<${Item} key=${i.text} i=${i} lane=${laneOf(i)} />`)}
      ${open || adding ? html`<${AddItem} project=${project} step=${s.num} editing=${adding} setEditing=${(on) => setAdding(on ? s.num : null)} />` : null}
    </ul>` : mode === 'all' ? html`<ul class="pitems"><li class="none">No items yet</li>
      <${AddItem} project=${project} step=${s.num} editing=${adding} setEditing=${(on) => setAdding(on ? s.num : null)} /></ul>` : null}
  </li>`;
}

function Autopilot({ p, working, open }) {
  const [want, setWant] = useState(null);
  const [err, setErr] = useState('');
  const on = want != null && want !== !!p.autopilot ? want : !!p.autopilot;
  useEffect(() => { if (want === !!p.autopilot) setWant(null); }, [p.autopilot]);
  const flip = () => {
    const next = !on;
    setWant(next); setErr('');
    api('POST', `/api/autopilot/${encodeURIComponent(p.id)}`, { on: next }).catch((e) => { setWant(null); setErr(String(e.message || e)); });
  };
  const bits = on ? [working ? `${working} ${working === 1 ? 'agent' : 'agents'} working` : null, open ? `${open} open` : 'all done'].filter(Boolean) : ['Paused'];
  return html`<div class=${`pauto${on ? ' on' : ''}`}><i class="pdot3"></i>
    <span><b>${on ? 'Autopilot on' : 'Autopilot off'}</b>${bits.map((b) => html` <span class="mut">·</span> ${b}`)}</span>
    <button class="ibtn" onClick=${flip} title=${on ? 'Stop starting lanes for open items; running lanes finish' : 'Start lanes for the open items again'}>
      <${Svg} s=${on ? PAUSE : RESUME} /><span>${on ? 'Pause' : 'Resume'}</span></button>
    ${err ? html`<span class="ierr ovinl">${err}</span>` : null}</div>`;
}

/** One project's card; re-rendered only when its plan, search, lanesSig or pSig change (`lanes` itself is a new
    array on every store change and `p` is changed in place, so they are compared through their signatures). */
const PlanCard = memo(function PlanCard({ p, plan, lanes, q, multi }) {
  const [modes, setModes] = useState({});
  const [adding, setAdding] = useState(null);
  // the latest lane of each item (as the daemon picks it: newest created_at)
  const latest = {};
  for (const l of lanes) {
    const k = `${l.step}\n${l.item}`;
    if (!latest[k] || String(l.created_at) > String(latest[k].created_at)) latest[k] = l;
  }
  const steps = (plan.steps || []).map((st) => {
    const total = st.items.length, done = st.items.filter((i) => i.done).length;
    const busy = st.items.some((i) => { const l = latest[`${st.num}\n${i.text}`] || i.lane; return !i.done && l && ACTIVE.includes(l.status); });
    const status = !total ? 'empty' : done === total ? 'done' : done || busy ? 'active' : 'todo';
    return { ...st, total, done, status };
  });
  const laneOf = (num) => (i) => latest[`${num}\n${i.text}`] || i.lane || null;
  const current = (steps.find((s) => s.status !== 'done' && s.status !== 'empty') || {}).num;
  const total = steps.reduce((n, s) => n + s.total, 0), done = steps.reduce((n, s) => n + s.done, 0);
  const visible = q && !hit(q, p.name, p.id) ? steps.filter((s) => s.items.some((i) => hit(q, i.text, s.title))) : steps;
  if (q && !visible.length) return null;
  const working = lanes.filter((l) => ACTIVE.includes(l.status)).length;
  const red = plan.main_red || p.main_red;
  const modeOf = (s) => (q ? 'all' : modes[s.num] || (s.num === current ? 'quiet' : 'closed'));
  return html`<div class="ovplan">
    <${Autopilot} p=${p} working=${working} open=${total - done} />
    <div class=${`card pcard${multi ? ' compact' : ''}`}>
      <div class="phead"><b>${p.name || p.id}</b><${Bar} done=${done} total=${total} cls="ptotal" /><span class="pcnt big">${done} / ${total}</span></div>
      ${red ? html`<div class="ierr ovred" role="alert"><b>Main is red.</b> Merges wait until it is green again${typeof red === 'string' && red !== 'true' && red !== '1' ? html`: <span>${red}</span>` : '.'}</div>` : null}
      <ul class="psteps">${visible.map((s) => html`<${Step} key=${s.num} project=${p.id} s=${s} q=${q} laneOf=${laneOf(s.num)}
        mode=${modeOf(s)} adding=${adding === s.num} setAdding=${(n) => { setAdding(n); if (n && modeOf(s) === 'closed') setModes((m) => ({ ...m, [s.num]: 'quiet' })); }}
        onToggle=${() => setModes((m) => ({ ...m, [s.num]: modeOf(s) === 'all' ? 'closed' : 'all' }))} />`)}</ul>
    </div></div>`;
}, ['lanes']);

function PlanProject({ p, lanes, q, multi }) {
  const { data } = usePlan(p.id);
  if (!data || !(data.steps || []).length) return null;
  const lanesSig = lanes.map((l) => `${l.id}:${l.status}`).join(',');
  const pSig = `${p.name}|${p.autopilot}|${p.main_red}`;
  return html`<${PlanCard} p=${p} plan=${data} lanes=${lanes} lanesSig=${lanesSig} pSig=${pSig} q=${q} multi=${multi} />`;
}

/** The Plan section: every project, or only `project`. */
export function Plan({ projects, lanes, q }) {
  if (!projects.length) return null;
  return html`<section class="plan"><h2>Plan</h2>
    ${projects.map((p) => html`<${PlanProject} key=${p.id} p=${p} lanes=${lanes.filter((l) => l.project === p.id)} q=${q} multi=${projects.length > 1} />`)}
  </section>`;
}

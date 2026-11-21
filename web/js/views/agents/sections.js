// The sections of a lane's detail: the section frame, a key/value card, its pipelines, its live tool events, its
// plan item with check and step, and its log file reloaded while the lane works.
import { html, useState, useEffect } from '../../../vendor/preact-htm.js';
import { useStore, useFetch, onMessage, setUi } from '../../store.js';
import { ago, clockTime, go } from '../../common.js';
import { Md } from '../boards/md.js';
import { pipeName, laneIds, StageChip, glyph, sClass, startMs } from '../boards/pipe.js';
import { itemText } from './lanes.js';

/** The lane's log file, reloaded (debounced) while its lane reports activity. Plain text, never parsed as one JSON. */
export function useLaneLog(id, active) {
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

export function Sec({ title, n, children }) {
  return html`<section class="asec"><h4>${title}${n ? html` <span class="n">${n}</span>` : null}</h4>${children}</section>`;
}

export function KV({ rows }) {
  return html`<div class="lcard">${rows.filter((r) => r[1] != null && r[1] !== '').map(([k, v]) => html`<div class="lkv"><span class="lk">${k}</span><span class="lv">${v}</span></div>`)}</div>`;
}

export function LanePipelines({ id }) {
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

export function Events({ id }) {
  const s = useStore();
  const l = [];
  for (let i = s.events.length - 1; i >= 0 && l.length < 12; i--) if (s.events[i].lane === id && s.events[i].tool) l.push(s.events[i]);
  if (!l.length) return html`<p class="quiet sm">No activity yet</p>`;
  return html`<ul class="erows">${l.map((e) => html`<li key=${e.id}><span class="mut t">${clockTime(e.at)}</span><span class="tool">${e.tool}</span>
    <span class="what" title=${e.summary}>${e.summary}</span></li>`)}</ul>`;
}

export function PlanItem({ a }) {
  const [plan] = useFetch(a.project ? `/api/plan/${encodeURIComponent(a.project)}` : '', ['plan']);
  const step = plan && (plan.steps || []).find((x) => x.num === a.step);
  const it = step && (step.items || []).find((x) => x.text === a.item);
  return html`<div class="agoal"><${Md} text=${itemText(a.item)} /></div>
    ${it && it.check ? html`<p class="lcheck"><span class="mut">Check</span> <code>${it.check}</code></p>` : null}
    ${step ? html`<p class="mut sm lstep">Step ${step.num}${step.title ? ` · ${step.title}` : ''}</p>` : null}`;
}

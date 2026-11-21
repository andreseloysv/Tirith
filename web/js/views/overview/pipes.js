// The Overview's running pipelines: the Pipelines view's compact rows, each opening that view.
import { html } from '../../../vendor/preact-htm.js';
import { useFetch } from '../../store.js';
import { ICON, Icon, Span, ago, clockTime, go } from '../../common.js';
import { Svg, hit } from './bits.js';

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

export function Pipelines({ project, projects, q }) {
  const [list] = useFetch(`/api/pipelines?limit=30${project ? `&project=${encodeURIComponent(project)}` : ''}`, ['pipeline', 'run']);
  const run = (list || []).filter((p) => p && (p.status === 'running' || p.status === 'pending') && (!project || p.project === project) && hit(q, p.kind, p.project, p.lanes, p.id));
  if (project && !run.length) return null;
  const pr = (id) => projects.find((x) => x.id === id) || {};
  return html`<section><h2>Pipelines running <span class="n">${run.length}</span></h2>
    ${run.length ? html`<div class="ptable">${run.map((p) => html`<${PipeRow} key=${p.id} p=${p} color=${pr(p.project).color || '#9aa1ac'} name=${project ? '' : pr(p.project).name || p.project} />`)}</div>`
      : html`<p class="quiet">Nothing is running</p>`}</section>`;
}

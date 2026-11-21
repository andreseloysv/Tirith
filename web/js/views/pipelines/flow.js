// The selected run: its stages top to bottom on the zoomable dotted canvas, with the zoom bar and the total.
import { html } from '../../../vendor/preact-htm.js';
import { Span } from '../../common.js';
import { sClass } from '../boards/pipe.js';
import { total } from './runs.js';
import { StageNode } from './stage.js';
import { useZoom } from './zoom.js';

/** No stage ran: the merge is being prepared, or why the pipeline ended before its first stage. */
function none(p, lanes) {
  const notes = lanes.filter((l) => l && l.note).map((l) => l.note.split('\n')[0]);
  return html`<div class="node pnone"><span class="nt"><span class="tt">No stage ran</span><span class="ds">${sClass(p.status) === 'running' ? 'The merge is being prepared…'
          : notes[0] ? notes[0] : 'The pipeline ended before its first stage.'}</span></span></div>`;
}

export function Flow({ p, stage, onStage, lanes }) {
  const st = p.stages || [];
  const sig = st.map((x) => `${x.id}:${x.status}`).join(',');
  const [{ cv, zr, zs }, setZoom, step] = useZoom(p.id, sig);
  return html`<div class="canvas" ref=${cv}><div class="zoomer" ref=${zr}><div class="zstage" ref=${zs}>
      <div class="pflow">${st.length ? st.map((x, i) => html`${i ? html`<div class=${`conn${x.status !== 'pending' ? ' on' : ''}`}></div>` : null}
        <${StageNode} key=${x.id} st=${x} i=${i} n=${st.length} sel=${stage === x.id} onOpen=${onStage} />`)
        : none(p, lanes)}</div>
    </div></div></div>
    <div class="zbar"><button aria-label="Zoom out" onClick=${() => step(-0.1)}>−</button><span class="zv">100%</span>
      <button aria-label="Zoom in" onClick=${() => step(0.1)}>+</button><button class="fit" onClick=${() => setZoom({ fit: true, z: 1 })}>Fit</button>
      <span class="sepv"></span><span class="tot">Total: ${total(p) || html`<${Span} start=${p.started_at} />`}</span></div>`;
}

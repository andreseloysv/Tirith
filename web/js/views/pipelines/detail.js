// The selected run on the right: its head (state, name, project, commit, lanes, Retry when red), its flow, and the
// open stage's log in the drawer.
import { html, useState, useEffect } from '../../../vendor/preact-htm.js';
import { useStore, api, setUi } from '../../store.js';
import { ICON, Icon, dur, elapsed, clockTime, go } from '../../common.js';
import { label, sClass, glyph, pipeName, laneIds } from '../boards/pipe.js';
import { Flow } from './flow.js';
import { StageLog } from './stage.js';
import { t } from '../../i18n.js';

function head(p, projName, lanes, busy, retry, openLane) {
  const [kind, num] = pipeName(p), ps = sClass(p.status);
  return html`<div class="pinfo"><span class=${`pst s-${ps}`} title=${label(p.status)}>${glyph(p.status, 12)}</span>
      <b>${kind} <span class="mut">${num}</span></b><span class="pchip">${projName}</span>
      ${p.sha ? html`<code class="psha" title=${p.sha}>${p.sha.slice(0, 8)}</code>` : null}
      <span class="plns">${lanes.map((l) => html`<button key=${l.id} class="link plane" title=${l.id} onClick=${() => openLane(l.id)}>${l.item ? l.item.replace(/\s*\(check:.*$/, '') : l.id}</button>`)}</span>
      <span class="pdacts">${ps === 'failed' ? html`<button class=${`srun pact${busy ? ' busy' : ''}`} disabled=${busy} onClick=${retry}
        title=${t('Run the failed lanes or the pipeline again')}><${Icon} d=${ICON.retry} size=${12} w=${2} /><span>${busy ? t('Retrying…') : t('Retry')}</span></button>` : null}</span>
    </div>`;
}

function drawer(p, st, setStage) {
  const [kind, num] = pipeName(p);
  return html`<aside class="drawer" role="dialog" aria-label=${t('Log')}>
      <header><span>${t('Log · {stage}', { stage: st.stage })}</span><span class=${`rb ${sClass(st.status)}`}>${label(st.status)}</span>
        <button class="x" aria-label=${t('Close')} onClick=${() => setStage(0)}><${Icon} d=${ICON.close} size=${14} w=${2} /></button></header>
      <div class="dbody"><p class="mut sm plogh">${kind} ${num} · ${t('started {time}', { time: clockTime(st.started_at) })}${st.ended_at ? ` · ${t('took {time}', { time: dur(elapsed(st.started_at, st.ended_at)) })}` : ''}</p>
        <${StageLog} key=${st.id} st=${st} /></div></aside>`;
}

export function Detail({ p, projName, stage, setStage }) {
  const s = useStore();
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  useEffect(() => { setBusy(false); setErr(''); }, [p.id]);
  const lanes = laneIds(p).map((id) => s.lanes[id] || { id });
  const st = (p.stages || []).find((x) => x.id === stage);
  const retry = () => {
    setBusy(true); setErr('');
    api('POST', `/api/pipelines/${encodeURIComponent(p.id)}/retry`).catch((x) => { setBusy(false); setErr(x.message); });
  };
  const openLane = (id) => { setUi('openLane', id); go('agents', p.project || ''); };
  return html`<div class="stagearea">
    ${head(p, projName, lanes, busy, retry, openLane)}
    ${err ? html`<div class="ierr">${err}</div>` : null}
    <${Flow} p=${p} stage=${stage} onStage=${(id) => setStage(stage === id ? 0 : id)} lanes=${lanes} />
    ${st ? drawer(p, st, setStage) : null}
  </div>`;
}

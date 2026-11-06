// Pipelines view: the old page's .pv layout — the run list on the left (.runcard: name, project, progress bar,
// current or failing stage, stage chips, lanes, duration) and the selected run on the right as a vertical stage
// flow on the dotted canvas (.node.snode with the old stage icons, .conn, .zbar zoom and total), with the stage's
// log in the right drawer and Retry on red pipelines. Live through SSE "pipeline" and "run".
import { html, useState, useEffect, useLayoutEffect, useRef, useMemo } from '../../vendor/preact-htm.js';
import { useStore, useFetch, api, onMessage, setUi } from '../store.js';
import { ICON, Icon, Span, dur, elapsed, ago, clockTime, stageIcon, go } from '../common.js';
import { LABEL, sClass, glyph, pipeName, laneIds, startMs, StageChip, failedStage } from './boards/pipe.js';

const CAP = 20;
const GROUPS = [['running', 'Running'], ['failed', 'Failed'], ['passed', 'Passed'], ['other', 'Other']];
const group = (p) => { const s = sClass(p.status); return s === 'running' || s === 'failed' || s === 'passed' ? s : 'other'; };
const PLAY = html`<svg viewBox="0 0 12 12" width="9" height="9" aria-hidden="true"><path d="M3.2 2.1l6.2 3.9-6.2 3.9z" fill="currentColor" /></svg>`;

function pct(p) {
  const st = p.stages || [];
  if (sClass(p.status) !== 'running') return 100;
  if (!st.length) return 6;
  return Math.max(6, Math.round((100 * st.filter((x) => x.status === 'passed').length) / (st.length + 1)));
}
function total(p) { return p.ended_at ? dur(elapsed(p.started_at, p.ended_at)) : null; }

function RunCard({ p, sel, projName, onOpen }) {
  const s = sClass(p.status), [kind, num] = pipeName(p), f = failedStage(p);
  const cur = (p.stages || []).find((x) => x.status === 'running');
  const lanes = laneIds(p);
  return html`<button class=${`card runcard${sel ? ' sel' : ''}`} onClick=${() => onOpen(p.id)}>
    <span class="rhead"><b>${kind} <span class="mut">${num}</span></b><span class="sm mut">${projName}</span></span>
    <span class="bar"><i class=${s === 'failed' ? 'red' : s === 'running' ? 'blue' : s === 'passed' ? 'green' : ''} style=${`width:${pct(p)}%`}></i></span>
    ${s === 'running' ? html`<span class="rline"><span class="rcur">${cur ? cur.stage : (p.stages || []).length ? 'merging…' : 'starting…'}</span><span><${Span} start=${p.started_at} /></span></span>` : null}
    ${s === 'failed' ? html`<span class="rerr">${f ? html`${f.status === 'timeout' ? 'timed out at ' : 'failed at '}<b>${f.stage}</b>` : 'failed before any stage ran'}</span>` : null}
    ${(p.stages || []).length ? html`<span class="pmini">${p.stages.map((st) => html`<${StageChip} key=${st.id} st=${st} />`)}</span>` : null}
    <span class="rline rmeta mut"><span>${lanes.length ? `${lanes.length} lane${lanes.length === 1 ? '' : 's'}` : 'main'}${total(p) ? ` · ${total(p)}` : ''}</span>
      <span title=${clockTime(p.started_at)}>${ago(p.started_at)}${ago(p.started_at) === 'now' ? '' : ' ago'}</span></span>
  </button>`;
}

/** A stage's log file (plain text), reloaded while the stage runs. */
function StageLog({ st }) {
  const [text, setText] = useState(null);
  const running = st.status === 'running';
  useEffect(() => {
    let alive = true, t = 0;
    const load = () => fetch(`/api/runs/${st.id}/log`).then((r) => r.text()).then((x) => alive && setText(x), () => alive && setText(''));
    setText(null);
    load();
    const off = onMessage((m) => { if (m.t === 'run' && m.d && m.d.id === st.id) { clearTimeout(t); t = setTimeout(load, 300); } });
    return () => { alive = false; off(); clearTimeout(t); };
  }, [st.id, running]);
  const pre = useRef(null);
  useLayoutEffect(() => { if (pre.current) pre.current.scrollTop = pre.current.scrollHeight; }, [text]);
  if (text == null) return html`<p class="quiet">Loading the log…</p>`;
  const t = text.replace(/\x1b\[[0-9;]*[A-Za-z]/g, '').replace(/\s+$/, '');
  return t ? html`<pre class="log plog" tabindex="0" ref=${pre}>${t}</pre>` : html`<p class="quiet">No log output yet</p>`;
}

function StageNode({ st, i, n, sel, onOpen }) {
  const s = sClass(st.status), [d, c1, c2] = stageIcon(st.stage);
  const time = st.ended_at ? dur(elapsed(st.started_at, st.ended_at)) : null;
  return html`<button class=${`node snode s-${s}${sel ? ' sel' : ''}`} onClick=${() => onOpen(st.id)}>
    <span class=${`gico ${s}`} style=${`background:linear-gradient(135deg, ${c1}, ${c2})`}><${Icon} d=${d} size=${20} color="#fff" w=${1.9} /></span>
    <span class="nt"><span class="ov">Step ${i + 1} of ${n}</span><span class="tt">${st.stage}</span>
      <span class=${`ds pds s-${s}`}>${LABEL[st.status] || st.status}</span></span>
    ${st.started_at ? html`<span class="dpill">${PLAY}${time || html`<${Span} start=${st.started_at} />`}</span>` : null}
  </button>`;
}

/** The selected run: its stages top to bottom on the zoomable dotted canvas. */
function Flow({ p, stage, onStage, lanes }) {
  const st = p.stages || [];
  const cv = useRef(null), zr = useRef(null), zs = useRef(null);
  const [zoom, setZoom] = useState({ fit: true, z: 1 });
  const sig = st.map((x) => `${x.id}:${x.status}`).join(',');
  const apply = () => {
    const c = cv.current, s = zs.current, r = zr.current;
    if (!c || !s || !r) return;
    const w = s.offsetWidth, h = s.offsetHeight;
    const sc = zoom.fit ? Math.max(0.35, Math.min(1, (c.clientWidth - 48) / w, (c.clientHeight - 110) / h)) : zoom.z;
    if (c.dataset.sc === String(sc) && c.dataset.wh === `${w}x${h}`) return;
    s.style.transform = `scale(${sc})`;
    r.style.width = `${Math.round(w * sc)}px`; r.style.height = `${Math.round(h * sc)}px`;
    const zv = c.parentNode.querySelector('.zv');
    if (zv) zv.textContent = `${Math.round(sc * 100)}%`;
    c.dataset.sc = String(sc); c.dataset.wh = `${w}x${h}`;
  };
  const applyRef = useRef(apply);
  applyRef.current = apply;
  useLayoutEffect(apply, [zoom, p.id, sig]);
  useEffect(() => {
    let t = 0;
    const ro = new ResizeObserver(() => { cancelAnimationFrame(t); t = requestAnimationFrame(() => { if (cv.current) cv.current.dataset.sc = ''; applyRef.current(); }); });
    if (cv.current) ro.observe(cv.current);
    return () => { ro.disconnect(); cancelAnimationFrame(t); };
  }, []);
  const step = (k) => {
    const sc = Number(cv.current && cv.current.dataset.sc) || 1;
    setZoom({ fit: false, z: Math.max(0.35, Math.min(1.6, Math.round((sc + k) * 20) / 20)) });
  };
  const notes = lanes.filter((l) => l && l.note).map((l) => l.note.split('\n')[0]);
  return html`<div class="canvas" ref=${cv}><div class="zoomer" ref=${zr}><div class="zstage" ref=${zs}>
      <div class="pflow">${st.length ? st.map((x, i) => html`${i ? html`<div class=${`conn${x.status !== 'pending' ? ' on' : ''}`}></div>` : null}
        <${StageNode} key=${x.id} st=${x} i=${i} n=${st.length} sel=${stage === x.id} onOpen=${onStage} />`)
        : html`<div class="node pnone"><span class="nt"><span class="tt">No stage ran</span><span class="ds">${sClass(p.status) === 'running' ? 'The merge is being prepared…'
          : notes[0] ? notes[0] : 'The pipeline ended before its first stage.'}</span></span></div>`}</div>
    </div></div></div>
    <div class="zbar"><button aria-label="Zoom out" onClick=${() => step(-0.1)}>−</button><span class="zv">100%</span>
      <button aria-label="Zoom in" onClick=${() => step(0.1)}>+</button><button class="fit" onClick=${() => setZoom({ fit: true, z: 1 })}>Fit</button>
      <span class="sepv"></span><span class="tot">Total: ${total(p) || html`<${Span} start=${p.started_at} />`}</span></div>`;
}

function Detail({ p, projName, stage, setStage }) {
  const s = useStore();
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  useEffect(() => { setBusy(false); setErr(''); }, [p.id]);
  const lanes = laneIds(p).map((id) => s.lanes[id] || { id });
  const st = (p.stages || []).find((x) => x.id === stage);
  const [kind, num] = pipeName(p), ps = sClass(p.status);
  const retry = () => {
    setBusy(true); setErr('');
    api('POST', `/api/pipelines/${encodeURIComponent(p.id)}/retry`).catch((x) => { setBusy(false); setErr(x.message); });
  };
  const openLane = (id) => { setUi('openLane', id); go('agents', ''); };
  return html`<div class="stagearea">
    <div class="pinfo"><span class=${`pst s-${ps}`} title=${LABEL[p.status] || p.status}>${glyph(p.status, 12)}</span>
      <b>${kind} <span class="mut">${num}</span></b><span class="pchip">${projName}</span>
      ${p.sha ? html`<code class="psha" title=${p.sha}>${p.sha.slice(0, 8)}</code>` : null}
      <span class="plns">${lanes.map((l) => html`<button key=${l.id} class="link plane" title=${l.id} onClick=${() => openLane(l.id)}>${l.item ? l.item.replace(/\s*\(check:.*$/, '') : l.id}</button>`)}</span>
      <span class="pdacts">${ps === 'failed' ? html`<button class=${`srun pact${busy ? ' busy' : ''}`} disabled=${busy} onClick=${retry}
        title="Run the failed lanes or the pipeline again"><${Icon} d=${ICON.retry} size=${12} w=${2} /><span>${busy ? 'Retrying…' : 'Retry'}</span></button>` : null}</span>
    </div>
    ${err ? html`<div class="ierr">${err}</div>` : null}
    <${Flow} p=${p} stage=${stage} onStage=${(id) => setStage(stage === id ? 0 : id)} lanes=${lanes} />
    ${st ? html`<aside class="drawer" role="dialog" aria-label="Log">
      <header><span>Log · ${st.stage}</span><span class=${`rb ${sClass(st.status)}`}>${LABEL[st.status] || st.status}</span>
        <button class="x" aria-label="Close" onClick=${() => setStage(0)}><${Icon} d=${ICON.close} size=${14} w=${2} /></button></header>
      <div class="dbody"><p class="mut sm plogh">${kind} ${num} · started ${clockTime(st.started_at)}${st.ended_at ? ` · took ${dur(elapsed(st.started_at, st.ended_at))}` : ''}</p>
        <${StageLog} key=${st.id} st=${st} /></div></aside>` : null}
  </div>`;
}

export function Pipelines({ project }) {
  const s = useStore();
  const [list, , error] = useFetch('/api/pipelines?limit=50', ['pipeline', 'run']);
  const [sel, setSel] = useState(() => new URLSearchParams(location.search).get('pipeline') || '');
  const [stage, setStage] = useState(() => Number(new URLSearchParams(location.search).get('stage')) || 0);
  const [caps, setCaps] = useState({});
  useEffect(() => { if (s.openPipeline) { setSel(s.openPipeline); setStage(0); s.openPipeline = ''; } }, [s.openPipeline]);
  useEffect(() => {
    const key = (e) => { if (e.key === 'Escape' && stage && !/^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement.tagName)) setStage(0); };
    addEventListener('keydown', key);
    return () => removeEventListener('keydown', key);
  }, [stage]);

  const nameOf = (id) => ((s.projects.find((x) => x.id === id) || {}).name || id);
  const q = String(s.q || '').trim().toLowerCase();
  const runs = useMemo(() => (list || []).filter((p) => p && (!project || p.project === project))
    .filter((p) => !q || [p.id, p.kind, p.project, p.lanes, p.sha, p.status, ...(p.stages || []).map((x) => x.stage)].some((x) => String(x || '').toLowerCase().includes(q)))
    .sort((a, b) => startMs(b) - startMs(a)), [list, project, q]);

  if (!list) return html`<div class="pad"><p class="quiet">${error ? `Could not load the pipelines: ${error.message}` : 'Loading…'}</p></div>`;
  if (!runs.length) return html`<div class="pad"><p class="quiet">${q || project ? 'No pipelines match' : 'No pipelines yet: they run when a lane is merged'}</p></div>`;
  const cur = runs.find((p) => p.id === sel) || runs.find((p) => sClass(p.status) === 'running') || runs[0];
  const open = (id) => { setSel(id); setStage(0); };
  return html`<div class="pv">
    <div class="plist">${GROUPS.map(([g, title]) => {
      const l = runs.filter((p) => group(p) === g);
      if (!l.length) return null;
      const cap = caps[g] || CAP;
      return html`<h3>${title} <span class="n">${l.length}</span></h3>
        <div class="pcol">${l.slice(0, cap).map((p) => html`<${RunCard} key=${p.id} p=${p} sel=${p.id === cur.id} projName=${nameOf(p.project)} onOpen=${open} />`)}</div>
        ${l.length > cap ? html`<button class="link lshow" onClick=${() => setCaps({ ...caps, [g]: cap + CAP })}>Show ${Math.min(CAP, l.length - cap)} more</button>` : null}`;
    })}</div>
    <${Detail} p=${cur} projName=${nameOf(cur.project)} stage=${(cur.stages || []).some((x) => x.id === stage) ? stage : 0} setStage=${setStage} />
  </div>`;
}

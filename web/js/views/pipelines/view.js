// Pipelines view: the old page's .pv layout — the run list on the left and the selected run on the right as a
// vertical stage flow, with the stage's log in the right drawer and Retry on red pipelines. Live through SSE
// "pipeline" and "run".
import { html, useState, useEffect, useMemo } from '../../../vendor/preact-htm.js';
import { useStore, useFetch } from '../../store.js';
import { sClass, startMs } from '../boards/pipe.js';
import { runList } from './runs.js';
import { Detail } from './detail.js';

/** The runs shown: of `project` (or all), matching the search, newest first. */
function useRuns(list, project, q) {
  return useMemo(() => (list || []).filter((p) => p && (!project || p.project === project))
    .filter((p) => !q || [p.id, p.kind, p.project, p.lanes, p.sha, p.status, ...(p.stages || []).map((x) => x.stage)].some((x) => String(x || '').toLowerCase().includes(q)))
    .sort((a, b) => startMs(b) - startMs(a)), [list, project, q]);
}

/** Escape closes the open stage's log, unless the owner is typing. */
function useEscape(stage, setStage) {
  useEffect(() => {
    const key = (e) => { if (e.key === 'Escape' && stage && !/^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement.tagName)) setStage(0); };
    addEventListener('keydown', key);
    return () => removeEventListener('keydown', key);
  }, [stage]);
}

export function Pipelines({ project }) {
  const s = useStore();
  const [list, , error] = useFetch(`/api/pipelines?limit=50${project ? `&project=${encodeURIComponent(project)}` : ''}`, ['pipeline', 'run']);
  const [sel, setSel] = useState(() => new URLSearchParams(location.search).get('pipeline') || '');
  const [stage, setStage] = useState(() => Number(new URLSearchParams(location.search).get('stage')) || 0);
  const [caps, setCaps] = useState({});
  useEffect(() => { if (s.openPipeline) { setSel(s.openPipeline); setStage(0); s.openPipeline = ''; } }, [s.openPipeline]);
  useEscape(stage, setStage);

  const nameOf = (id) => ((s.projects.find((x) => x.id === id) || {}).name || id);
  const q = String(s.q || '').trim().toLowerCase();
  const runs = useRuns(list, project, q);

  if (!list) return html`<div class="pad"><p class="quiet">${error ? `Could not load the pipelines: ${error.message}` : 'Loading…'}</p></div>`;
  if (!runs.length) return html`<div class="pad"><p class="quiet">${q ? 'No pipelines match' : `No pipelines${project ? ' for this project' : ''} yet: they run when a lane is merged`}</p></div>`;
  const cur = runs.find((p) => p.id === sel) || runs.find((p) => sClass(p.status) === 'running') || runs[0];
  const open = (id) => { setSel(id); setStage(0); };
  return html`<div class="pv">
    ${runList({ runs, cur, caps, setCaps, nameOf, open })}
    <${Detail} p=${cur} projName=${nameOf(cur.project)} stage=${(cur.stages || []).some((x) => x.id === stage) ? stage : 0} setStage=${setStage} />
  </div>`;
}

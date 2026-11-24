// Agents view: the old page's kanban (.board of .col, one per lane status) or its compact list (.rows), and the
// right drawer (.drawer) with everything about one lane. Lanes come live from the store (SSE "lane").
import { html, useState, useEffect, useMemo } from '../../../vendor/preact-htm.js';
import { useStore } from '../../store.js';
import { projectColor } from '../../common.js';
import { cols, newest, isOld, questionOf, loadMode, saveMode } from './lanes.js';
import { Drawer, Column, ListRow } from './board.js';
import { t } from '../../i18n.js';

/** The lanes shown: of `project` (or all), matching the search, newest first. */
function useLanes(s, project, q) {
  return useMemo(() => Object.values(s.lanes)
    .filter((a) => !project || a.project === project)
    .filter((a) => !q || [a.item, a.id, a.agent, a.model, a.note, a.branch, a.status, a.question].some((x) => String(x || '').toLowerCase().includes(q)))
    .sort((x, y) => newest(y) - newest(x)), [s.updated, project, q]);
}

/** Each lane's last tool call, for the "now doing" line of a working card. */
function useDoing(s) {
  return useMemo(() => {
    const m = {};
    for (const e of s.events) if (e.lane && e.tool && e.kind === 'PreToolUse') m[e.lane] = e;
    return m;
  }, [s.events.length, s.events[s.events.length - 1]]);
}

function toolbar(mode, setMode) {
  return html`<div class="toolbar"><div class="seg" role="group" aria-label=${t('Layout')}>
    ${[['list', t('List')], ['kanban', t('Kanban')]].map(([m, label]) => html`<button class=${mode === m ? 'on' : ''} onClick=${() => { setMode(m); saveMode(m); }}>${label}</button>`)}</div></div>`;
}

function list(v, more) {
  const rank = (a) => (a.status === 'running' ? 0 : a.status === 'waiting' ? 1 : ['queued', 'merge-queued', 'merging'].includes(a.status) ? 2 : 3);
  const rows = v.vis.slice().sort((x, y) => rank(x) - rank(y) || newest(y) - newest(x));
  return html`<div class="listwrap"><ul class="rows card">${rows.slice(0, v.cap).map((a) => html`<${ListRow} key=${a.id} a=${a} open=${v.sel === a.id}
        onOpen=${v.setSel} color=${v.colorOf(a.project)} projName=${v.nameOf(a.project)} />`)}</ul>
      ${rows.length > v.cap ? html`<button class="link lshow" onClick=${() => v.setCap(v.cap + 60)}>${t('Show {n} more', { n: Math.min(60, rows.length - v.cap) })}</button>` : null} ${more}</div>`;
}

function kanban(v, more) {
  const { s, vis, cur, sel, setSel, colorOf, nameOf } = v;
  return [
    html`<div class="board lboard">${cols().filter((c) => !cur || vis.some((a) => c[3].includes(a.status))).map((c) => html`<${Column} key=${c[0]} col=${c} lanes=${vis.filter((a) => c[3].includes(a.status))}
      sel=${cur ? sel : ''} onOpen=${(id) => setSel(sel === id ? '' : id)} colorOf=${colorOf} nameOf=${nameOf} doing=${v.doing} qOf=${(a) => questionOf(s, a)} />`)}</div>`,
    more ? html`<div class="moreline">${more}</div>` : null,
    cur ? html`<${Drawer} a=${cur} projName=${nameOf(cur.project)} color=${colorOf(cur.project)} onClose=${() => setSel('')} />` : null,
  ];
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
  const all = useLanes(s, project, q);
  const doing = useDoing(s);
  const old = all.filter(isOld);
  const vis = earlier ? all : all.filter((a) => !isOld(a));
  const cur = sel && s.lanes[sel];

  const tb = toolbar(mode, setMode);
  const more = old.length ? html`<button class="link" onClick=${() => setEarlier(!earlier)}>${earlier ? t('Hide earlier') : t('Show {n} earlier', { n: old.length })}</button>` : null;

  if (!all.length) {
    return [tb, html`<div class="pad"><p class="quiet">${q ? t('No lanes match') : project ? t('No agents in this project yet: lanes appear here when the plan or the manager starts them') : t('No agents yet: lanes appear here when the plan or the manager starts them')}</p></div>`];
  }
  const v = { s, vis, cur, sel, setSel, cap, setCap, colorOf, nameOf, doing };
  if (mode === 'list') return [tb, list(v, more)];
  return [tb, ...kanban(v, more)];
}

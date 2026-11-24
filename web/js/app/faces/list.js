// The faces of the sidebar: every bot that shows, and the Project Manager — the agent of every project, one face
// for all of them (the boss of the helpers the Home lists). Each with its state, its unread answers and where it
// opens; in the order the owner dragged them.
import { useStore, useFetch } from '../../store.js';
import { useUnread } from '../../unread.js';
import { stateOf, keyOf, recall } from '../../views/bots.js';
import { lookOf } from '../../views/bots/face/face-map.js';
import { go } from '../../common.js';
import { pmProjects, lastProject } from './where.js';
import { sorted } from './order.js';

export const PM = 'manager';
const PM_FACE = { shape: 'hex', color: '#8e7cf0' };

/* the Project Manager: working where a project's chat has a turn or its helpers work; opens that project, else
   the one it worked on last */
function pmFace(role, s) {
  const now = pmProjects(s), last = lastProject(s);
  const target = now[0] || (last && last.id) || (s.projects[0] || {}).id || '';
  const face = (role.face && role.face.shape && role.face) || PM_FACE;
  return { id: PM, pm: true, key: '', name: role.name || 'Project Manager', shape: face.shape, color: face.color || PM_FACE.color, look: lookOf({ ...face, color: face.color || PM_FACE.color }),
    state: now.length ? 'work' : 'watch', unread: 0, open: () => go('overview', target) };
}

function botFace(b, s, unread) {
  const busy = !!s.busy[keyOf(b)];
  return { id: b.id, key: keyOf(b), name: b.name, shape: b.shape, color: b.color, look: lookOf(b), bot: b, busy,
    state: stateOf(b, busy), unread: unread.by[b.id] || 0, open: () => go('bots', b.id) };
}

/** `{faces, sel}`: the faces in the owner's order, and the one whose conversation is shown (none elsewhere). */
export function useFaces(r, order) {
  const s = useStore();
  const unread = useUnread();
  const [M] = useFetch('/api/models', ['models', 'bots']);
  const bots = unread.bots.filter((b) => b.show !== false).map((b) => botFace(b, s, unread));
  const role = M && M.roles && M.roles[PM];
  const all = role && s.projects.length ? bots.concat(pmFace(role, s)) : bots;
  const inProject = r.view === 'overview' && s.projects.some((p) => p.id === r.project);
  const sel = r.view === 'bots' ? r.project || recall() || (bots[0] || {}).id : inProject ? PM : '';
  const faces = sorted(all, order).map((f) => (f.id === sel ? { ...f, unread: 0 } : f));
  return { faces, sel };
}

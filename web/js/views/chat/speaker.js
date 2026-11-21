// Who speaks in a conversation — a project's Project Manager, a bot, or the manager of no project — with its
// name and face (from /api/models, the factory role) and what its face in the head shows.
import { useStore, useTalk, useFetch } from '../../store.js';
import { useUnread } from '../../unread.js';
import { projName } from './util.js';

const LABEL = { work: 'Working', watch: 'Waiting', sleep: 'Asleep' };

/** The head's face, as in Faces: working on a turn, else on watch (a bot with nothing on sleeps). */
function faceStateOf(busy, bot) {
  return busy ? 'work' : bot && !(bot.routines || []).some((r) => r.on !== false) && !(bot.routes || []).length ? 'sleep' : 'watch';
}

export function useSpeaker(project, who) {
  const s = useStore();
  const talk = useTalk(project);
  const name = projName(s.projects, project);
  const isProject = s.projects.some((p) => p.id === project);
  const role = isProject ? 'manager' : project;   /* the factory role that speaks in this conversation */
  const [M] = useFetch('/api/models', ['models', 'bots']);
  const unread = useUnread();   /* the bots and their unread answers, as the sidebar's Faces show them */
  const entry = (M && M.roles[role]) || null;
  const whoName = who || (entry && entry.name) || (isProject ? 'Project Manager' : 'Manager');
  const face = (entry && entry.face && entry.face.shape && entry.face) || (isProject ? { shape: 'hex', color: '#8e7cf0' } : null);
  const faceKey = face ? `${face.shape}:${face.color || ''}` : '';
  const busy = !!s.busy[project];
  /* with its unread answers */
  const bot = unread.bots.find((b) => (b.key || `bot-${b.id}`) === project) || null;
  const unseen = bot ? unread.by[bot.id] || 0 : 0;
  const faceState = faceStateOf(busy, bot);
  return { s, talk, name, isProject, role, whoName, face, faceKey, busy, bot, unseen, faceState, stateLabel: LABEL[faceState] };
}

// Who a question or a piece of work belongs to on the Home: a project's Project Manager (one face for every project,
// named with its project), a bot, or the Manager (no project). Names and faces come from the factory (/api/models).
import { useFetch, useStore } from '../../store.js';
import { go } from '../../common.js';

const PM_FACE = { shape: 'hex', color: '#8e7cf0' };
const GM_FACE = { shape: 'dome', color: '#3fb6a8' };
const faceOf = (role, d) => (role && role.face && role.face.shape ? { shape: role.face.shape, color: role.face.color || d.color } : d);

/** `who(key)` → {name, place, face, open()} for a conversation key: a project's id, `bot-<id>` or a system bot's key
    ('' and anything unknown: the Manager's). `open` shows its conversation. */
export function useWho() {
  const s = useStore();
  const [M] = useFetch('/api/models', ['models', 'bots']);
  const roles = (M && M.roles) || {};
  return (key) => {
    const p = s.projects.find((x) => x.id === key);
    if (p) return { name: (roles.manager && roles.manager.name) || 'Project Manager', place: p.name || p.id, face: faceOf(roles.manager, PM_FACE), open: () => go('overview', p.id) };
    const k = roles[key] ? key : 'manager-general', role = roles[k];
    const id = k.startsWith('bot-') ? k.slice(4) : k;
    return { name: (role && role.name) || 'Manager', place: '', face: faceOf(role, GM_FACE), open: () => go('bots', id) };
  };
}

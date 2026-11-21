// Rooms view: the owner's phone first, then each project's encrypted room, Join with a code from another Tirith,
// and the public rooms Tirith listens to. Data: GET /api/rooms, refetched on SSE "room" / "room-mention", every
// 15 s and when the window comes back.
import { html, useMemo, useEffect } from '../../../vendor/preact-htm.js';
import { useFetch, useStore } from '../../store.js';
import { PhoneCard, NoRoom } from './phone.js';
import { PrivateCard, JoinCard } from './private.js';
import { PublicCard } from './public.js';
import { agentOf } from './agent.js';

/** The QR on the screen must be the one that works: the rooms are also read on a clock and when the window comes
    back, in case the stream missed the moment a QR was used. */
function useReadAgain(reload) {
  useEffect(() => {
    const again = () => { if (!document.hidden) reload(); };
    const timer = setInterval(again, 15000);
    addEventListener('focus', again);
    document.addEventListener('visibilitychange', again);
    return () => { clearInterval(timer); removeEventListener('focus', again); document.removeEventListener('visibilitychange', again); };
  }, []);
}

/** `[private, public, phone]`: the rooms of `project` (or all) by title, and the room the phone connects through. */
function useRoomLists(data, rooms, project) {
  return useMemo(() => {
    const byTitle = (a, b) => String(a.title || a.id).localeCompare(String(b.title || b.id));
    const mine = rooms.filter((r) => !project || r.project === project).sort(byTitle);
    // the phone connects through the selected project's own room, else through the room of no project
    const own = rooms.filter((r) => r.kind === 'manager');
    return [mine.filter((r) => r.private), mine.filter((r) => !r.private), own.find((r) => r.project === (project || 'manager-general')) || (project ? null : own[0]) || null];
  }, [data, project]);
}

/** Inside a project: that project's rooms (its room talks to its manager); with none selected: all. */
export function Rooms({ project }) {
  const s = useStore();
  const [data, reload, err] = useFetch('/api/rooms', ['room', 'room-mention']);
  const [M] = useFetch('/api/models', ['models', 'bots']);
  const [bots] = useFetch('/api/bots', ['bots']);
  useReadAgain(reload);
  const rooms = (data && data.rooms) || [];
  const [priv, pub, phone] = useRoomLists(data, rooms, project);
  /* a room without a name of its own goes by its project's, as it does on the phone (the room of no project: All projects) */
  const titleOf = (r) => r.title || (s.projects.find((p) => p.id === r.project) || {}).name || (r.kind === 'manager' && !r.project ? 'All projects' : r.id);
  const agentFor = agentOf(s, M, bots);
  return html`<div class="msettings mrooms">
    ${err && !data ? html`<div class="ierr">Could not read the rooms: ${err.message}</div>` : null}
    ${phone ? html`<${PhoneCard} key=${phone.id} r=${phone} reload=${reload} title=${titleOf(phone)} agent=${agentFor(phone.project)} />` : data ? html`<${NoRoom} reload=${reload} project=${project} agent=${agentFor(project)} />` : null}
    ${priv.map((r) => html`<${PrivateCard} key=${r.id} r=${r} reload=${reload} title=${titleOf(r)} agent=${agentFor(r.project)} />`)}
    ${data ? html`<${JoinCard} reload=${reload} project=${project} />` : null}
    ${pub.length ? html`<h2 class="rsec">Public rooms <span class="n">${pub.length}</span></h2>
      ${pub.map((r) => html`<${PublicCard} key=${r.id} r=${r} reload=${reload} />`)}` : null}
  </div>`;
}

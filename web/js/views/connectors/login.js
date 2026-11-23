// A remote connector's sign-in for one bot from the page: the daemon begins it (POST /api/connectors/<id>/login),
// the owner's browser opens the server's sign-in in a new tab, and the page waits on its ticket until it ends.
import { api } from '../../store.js';

const WAIT_MS = 1500;
const MOST_MS = 10 * 60 * 1000;

/** Signs bot `bot` in to connector `id`; throws with why when it fails (the why holds no secret). */
export async function login(id, bot) {
  const tab = window.open('about:blank', '_blank'); // opened now, while the click still counts, so no popup blocker stops it
  let begun;
  try {
    begun = await api('POST', `/api/connectors/${encodeURIComponent(id)}/login`, { bot });
  } catch (e) {
    if (tab) tab.close();
    throw e;
  }
  if (tab) tab.location.href = begun.url; else window.open(begun.url, '_blank');
  const until = Date.now() + MOST_MS;
  while (Date.now() < until) {
    await new Promise((r) => setTimeout(r, WAIT_MS));
    const st = await api('GET', `/api/connectors/logins/${encodeURIComponent(begun.ticket)}`);
    if (st.status === 'done') return st;
    if (st.status === 'failed') throw new Error(`The sign-in failed: ${st.why || 'no reason given'}`);
  }
  throw new Error('The sign-in took over ten minutes: try Login again.');
}

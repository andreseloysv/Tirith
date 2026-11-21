// The Phone tab: one QR that opens the bot's chat on the owner's phone.
import { html, useState, useEffect, useRef } from '../../../vendor/preact-htm.js';
import { useFetch, api } from '../../store.js';
import { keyOf } from './state.js';

/** The rooms are read again on a clock and when the window comes back, so the QR on the screen is the one that works. */
function useFreshRooms(reload) {
  useEffect(() => {
    const again = () => { if (!document.hidden) reload(); };
    const timer = setInterval(again, 15000);
    addEventListener('focus', again);
    return () => { clearInterval(timer); removeEventListener('focus', again); };
  }, []);
}

export function Phone({ bot }) {
  const key = keyOf(bot);
  const [data, reload, err] = useFetch('/api/rooms', ['room']);
  const [fail, setFail] = useState('');
  const asked = useRef(0);
  const room = ((data && data.rooms) || []).find((r) => r.kind === 'manager' && r.project === key);
  useFreshRooms(reload);
  // No room yet, or its QR was used or is old: the next one is asked for by itself, at most every 10 s.
  useEffect(() => {
    if (!data || (room && room.invite) || Date.now() - asked.current < 10000) return;
    asked.current = Date.now();
    api('POST', '/api/rooms', { project: key, title: bot.name }).then(() => { setFail(''); reload(); }, (e) => setFail(e.message));
  });
  const src = room && room.invite ? `/api/rooms/${encodeURIComponent(room.id)}/qr.svg?kind=invite&v=${encodeURIComponent(room.invite)}` : '';
  const phones = room ? (room.devices || []).length : 0;
  return html`<div class="bphone">
    ${src ? html`<img class="bqr" src=${src} alt=${`QR code that opens ${bot.name} on your phone`} width="216" height="216" />`
      : html`<div class="bqr bqrwait">${fail || (err && !data) ? 'No QR right now' : 'Making the QR…'}</div>`}
    <p class="bscan">Scan it with your phone's camera to talk to ${bot.name} there.</p>
    <p class="bnote">It works once; the next one appears here by itself.${phones ? ` ${phones} phone${phones > 1 ? 's' : ''} already in this chat.` : ''}</p>
    ${fail ? html`<p class="berr">${fail}</p>` : null}
  </div>`;
}

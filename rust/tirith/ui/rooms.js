// rooms.js: the "Salas" section. Private rooms (phone pairing over agentsmessenger.com) and public rooms.
(() => {
  const box = T.section('rooms', 'Salas');
  const rooms = {};
  const short = k => T.esc(String(k || '').slice(0, 8));
  const css = document.createElement('style');
  css.textContent = `#rooms .room{border:1px solid var(--line);background:var(--panel);border-radius:10px;padding:9px 11px;margin-bottom:8px}
#rooms .row{display:flex;gap:6px;align-items:center;flex-wrap:wrap;margin-top:6px}#rooms .row input{flex:1;min-width:120px}
#rooms button{padding:2px 8px;font-size:12px}#rooms .qr{background:#fff;border-radius:6px;padding:4px;width:180px;height:180px;display:block;margin-top:6px}
#rooms .msg{font-size:12px;padding:2px 0;border-bottom:1px dashed var(--line);white-space:pre-wrap;word-wrap:break-word}
#rooms .msg.need{background:color-mix(in srgb,var(--acc) 25%,transparent)}#rooms .who{color:var(--mute)}
#rooms details summary{cursor:pointer;font-size:12px;color:var(--mute)}#rooms .err{color:var(--bad);font-size:12px}`;
  document.head.appendChild(css);
  box.innerHTML = `<form class="row" id="room-new"><input placeholder="Título de la nueva sala privada"><button class="pri">Nueva sala</button></form>
    <form class="row" id="room-join"><input placeholder="Código de otro manager (sealed-…:…:…)"><button>Unirse</button></form>
    <div id="room-list"></div>`;
  const shown = {}; // room id -> which QR is open ('link' | 'invite')

  function people(list, label) {
    if (!list.length) return '';
    return `<div class="quiet" style="font-size:12px">${label}: ${list.map(p => `${T.esc(p.name || '?')} <code>${short(p.key_id)}</code>${p.accepted === false ? ' (sin código)' : ''}`).join(', ')}</div>`;
  }
  function messages(r, n) {
    const need = new Set(r.needs_owner || []);
    return (r.messages || []).slice(-n).map(m => `<div class="msg${need.has(m.n) ? ' need' : ''}"><span class="who">${T.time(m.at)} ${T.esc(m.from)}${m.device ? ' 📱' : ''}:</span> ${T.esc(m.text)}</div>`).join('');
  }
  function qr(r, kind) {
    return shown[r.id] === kind ? `<img class="qr" alt="QR" src="/api/rooms/${encodeURIComponent(r.id)}/qr.svg?kind=${kind}&t=${Date.now()}">` : '';
  }
  function html(r) {
    const id = T.esc(r.id);
    const err = r.error ? `<div class="err">${T.esc(r.error)}</div>` : '';
    const head = `<div class="top"><b>${r.private ? '🔒 ' : ''}${T.esc(r.title || r.id)}</b> <span class="quiet" style="font-size:12px">${id}</span></div>${err}`;
    if (!r.private) {
      const need = (r.needs_owner || []).length ? `<div class="err">Te nombran y nadie respondió (${r.needs_owner.length})</div>` : '';
      return `${head}<div style="font-size:13px;margin-top:4px">${T.esc(r.summary || 'Sin mensajes todavía.')}</div>${need}
        ${people(r.participants || [], 'Participantes')}
        <details><summary>Últimos mensajes</summary>${messages(r, 40)}</details>
        <form class="row" data-say="${id}"><input placeholder="Escribir en la sala"><button>Enviar</button></form>`;
    }
    const last = (r.messages || []).slice(-1);
    const parts = [head, people(r.devices || [], 'Teléfonos'), people(r.managers || [], 'Managers')];
    if (r.kind === 'manager') {
      parts.push(`<div class="row"><span>Abrir en mi teléfono</span><button data-copy="${T.esc(r.link)}">Copiar</button><button data-qr="${id}" data-kind="link">QR</button></div>${qr(r, 'link')}`);
      parts.push(r.invite
        ? `<div class="row"><span>Conectar otro teléfono (un uso)</span><button data-qr="${id}" data-kind="invite">QR</button></div>${qr(r, 'invite')}`
        : `<div class="row"><span class="quiet">El último QR ya se usó</span><button data-newqr="${id}">Nuevo QR</button></div>`);
    } else parts.push('<div class="quiet" style="font-size:12px">Unida como manager invitado.</div>');
    if (r.braked) parts.push('<div class="err">Frenada: 30 mensajes seguidos de agentes. Vuelve cuando escribe un teléfono.</div>');
    if (last.length) parts.push(messages(r, 1));
    parts.push(`<div class="row">${r.kind === 'manager' ? `<button data-invite="${id}">Invitar a un manager</button>` : ''}<button data-leave="${id}">Salir</button></div>`);
    return parts.join('');
  }
  function render(r) {
    if (r.gone) { delete rooms[r.id]; document.getElementById('room-' + r.id)?.remove(); return; }
    rooms[r.id] = r;
    let el = document.getElementById('room-' + r.id);
    if (!el) { el = document.createElement('div'); el.id = 'room-' + r.id; el.className = 'room'; list().appendChild(el); }
    const typing = el.contains(document.activeElement) ? document.activeElement.value : null;
    el.innerHTML = html(r);
    if (typing) { const i = el.querySelector('input'); if (i) { i.value = typing; i.focus(); } }
    empty();
  }
  function empty() {
    const all = list();
    const none = !Object.keys(rooms).length;
    const note = document.getElementById('room-none');
    if (none && !note) all.insertAdjacentHTML('afterbegin', '<div class="quiet" id="room-none">Ninguna sala. Crea una para hablar con el manager desde tu teléfono, cifrado.</div>');
    if (!none && note) note.remove();
  }
  const list = () => document.getElementById('room-list');
  async function copy(text) {
    try { await navigator.clipboard.writeText(text); } catch { prompt('Copia:', text); }
  }
  async function load() {
    const got = await T.api('GET', '/api/rooms');
    list().innerHTML = '';
    Object.keys(rooms).forEach(k => delete rooms[k]);
    got.rooms.forEach(render);
    empty();
  }
  box.addEventListener('click', async e => {
    const d = e.target.dataset;
    if (d.copy) copy(d.copy);
    if (d.qr) { shown[d.qr] = shown[d.qr] === d.kind ? null : d.kind; render(rooms[d.qr]); }
    if (d.newqr) { shown[d.newqr] = 'invite'; render(await T.api('POST', `/api/rooms/${encodeURIComponent(d.newqr)}/qr`)); }
    if (d.invite) { const v = await T.api('POST', `/api/rooms/${encodeURIComponent(d.invite)}/invite`); copy(v.peer_code); e.target.textContent = 'Código copiado'; }
    if (d.leave && confirm('¿Salir de esta sala? Los teléfonos ya no podrán hablar con este manager en ella.')) await T.api('POST', `/api/rooms/${encodeURIComponent(d.leave)}/leave`);
  });
  box.addEventListener('submit', async e => {
    e.preventDefault();
    e.stopPropagation();
    const f = e.target, input = f.querySelector('input'), text = input.value.trim();
    if (f.id === 'room-new') { const r = await T.api('POST', '/api/rooms', {title: text}); input.value = ''; shown[r.id] = 'invite'; render(r); }
    else if (f.id === 'room-join' && text) { render(await T.api('POST', '/api/rooms/join', {code: text})); input.value = ''; }
    else if (f.dataset.say && text) { input.value = ''; await T.api('POST', `/api/rooms/${encodeURIComponent(f.dataset.say)}/say`, {text}); }
  });
  T.on('room', render);
  T.on('room-mention', m => { const r = rooms[m.room]; if (r) render(r); });
  load();
})();

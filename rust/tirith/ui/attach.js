// attach.js: files and images in the chat. Pick them with the clip, paste images, or drop files on the chat
// column; they wait as chips (removable) and are uploaded when the message is sent. Owner messages that list
// attached images show their thumbnails.
(() => {
  const form = document.getElementById('say'), text = document.getElementById('text'), left = document.getElementById('left');
  const IMG = /\.(png|jpe?g|gif|webp|svg)$/i;
  const pending = [];
  const css = document.createElement('style');
  css.textContent = `#att-chips{display:flex;flex-wrap:wrap;gap:6px;padding:0 16px}#att-chips:empty{display:none}
  .att-chip{display:flex;align-items:center;gap:5px;font-size:12px;border:1px solid var(--line);border-radius:8px;padding:3px 6px;background:var(--panel)}
  .att-chip img{width:34px;height:34px;object-fit:cover;border-radius:4px}.att-chip button{padding:0 6px;border:0}
  #left.att-drop{outline:2px dashed var(--acc);outline-offset:-6px}
  .att-thumbs{display:flex;flex-wrap:wrap;gap:6px;margin-top:6px}.att-thumbs img{max-width:160px;max-height:120px;border-radius:6px;cursor:zoom-in}
  .att-thumbs a{font-size:12px;color:inherit}`;
  document.head.appendChild(css);

  const chips = document.createElement('div'); chips.id = 'att-chips';
  form.parentNode.insertBefore(chips, form);
  const picker = document.createElement('input'); picker.type = 'file'; picker.multiple = true; picker.hidden = true;
  const clip = document.createElement('button'); clip.type = 'button'; clip.textContent = '📎'; clip.title = 'Adjuntar archivos';
  form.insertBefore(clip, form.querySelector('button.pri')); form.appendChild(picker);
  clip.addEventListener('click', () => picker.click());
  picker.addEventListener('change', () => { add(picker.files); picker.value = ''; });

  function add(files) {
    for (const f of files || []) {
      const name = f.name && f.name !== 'image.png' ? f.name : `pegado-${Date.now()}.${(f.type.split('/')[1] || 'png').replace('jpeg', 'jpg')}`;
      pending.push({file: f, name, thumb: f.type.startsWith('image/') ? URL.createObjectURL(f) : ''});
    }
    render();
  }
  function render() {
    chips.innerHTML = pending.map((p, i) => `<span class="att-chip">${p.thumb ? `<img src="${p.thumb}" alt="">` : '📄'}
      <span>${T.esc(p.name)}</span><button type="button" data-att-rm="${i}" title="Quitar">×</button></span>`).join('');
  }
  chips.addEventListener('click', e => {
    const i = e.target.dataset.attRm;
    if (i === undefined) return;
    const [p] = pending.splice(Number(i), 1);
    if (p && p.thumb) URL.revokeObjectURL(p.thumb);
    render();
  });

  text.addEventListener('paste', e => {
    const files = [...(e.clipboardData && e.clipboardData.files || [])];
    if (files.length) { e.preventDefault(); add(files); }
  });
  let depth = 0;
  left.addEventListener('dragenter', e => { if (e.dataTransfer.types.includes('Files')) { depth++; left.classList.add('att-drop'); } });
  left.addEventListener('dragleave', () => { if (--depth <= 0) { depth = 0; left.classList.remove('att-drop'); } });
  left.addEventListener('dragover', e => { if (e.dataTransfer.types.includes('Files')) e.preventDefault(); });
  left.addEventListener('drop', e => {
    depth = 0; left.classList.remove('att-drop');
    if (e.dataTransfer.files.length) { e.preventDefault(); add(e.dataTransfer.files); }
  });

  // The chat form: upload the pending files first, then send the message with their paths.
  let sending = false;
  form.addEventListener('submit', async e => {
    e.preventDefault(); e.stopPropagation();
    const msg = text.value.trim();
    if (sending || (!msg && !pending.length)) return;
    sending = true; clip.disabled = true;
    try {
      const paths = [];
      for (const p of pending) {
        const r = await fetch('/api/attachments?name=' + encodeURIComponent(p.name), {method: 'POST', body: p.file});
        const t = await r.text();
        if (!r.ok) { alert(`No se pudo subir ${p.name}: ${t}`); return; }
        paths.push(JSON.parse(t).path);
      }
      await T.api('POST', '/api/chat', {text: msg, attachments: paths});
      text.value = '';
      pending.splice(0).forEach(p => p.thumb && URL.revokeObjectURL(p.thumb));
      render();
    } finally { sending = false; clip.disabled = false; }
  });

  // Owner messages: the "Attached files" list becomes thumbnails (images) and links (other files).
  function decorate(el) {
    if (!el.classList || !el.classList.contains('owner') || el.dataset.att) return;
    const cut = el.textContent.indexOf('\n\nAttached files (');
    if (cut < 0) return;
    el.dataset.att = '1';
    const files = el.textContent.slice(cut).split('\n').filter(l => l.startsWith('- ')).map(l => l.slice(2));
    el.textContent = el.textContent.slice(0, cut);
    const box = document.createElement('div'); box.className = 'att-thumbs';
    box.innerHTML = files.map(path => {
      const m = path.match(/\/attachments\/([0-9a-f]+)\/([^/]+)$/);
      if (!m) return `<span>${T.esc(path)}</span>`;
      const url = `/api/attachments/${m[1]}/${encodeURIComponent(m[2])}`;
      return IMG.test(m[2]) ? `<a href="${url}" target="_blank"><img src="${url}" alt="${T.esc(m[2])}"></a>`
        : `<a href="${url}" target="_blank">📄 ${T.esc(m[2])}</a>`;
    }).join('');
    el.appendChild(box);
  }
  const chat = document.getElementById('chat');
  new MutationObserver(ms => ms.forEach(m => m.addedNodes.forEach(decorate))).observe(chat, {childList: true});
  chat.querySelectorAll('.m.owner').forEach(decorate);
})();

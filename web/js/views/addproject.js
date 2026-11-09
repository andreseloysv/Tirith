// The "+" next to Projects: a dialog that adds a folder as a project (POST /api/projects). The folder is picked from
// the git repositories next to the projects, chosen in macOS's folder picker (the daemon opens it) or typed; a dry
// run says before Add what adding it does (a git repository, a first plan) or why the folder cannot be a project.
import { html, useState, useEffect, useRef } from '../../vendor/preact-htm.js';
import { api, useFetch } from '../store.js';
import { go, route } from '../common.js';

const COLORS = ['#2f7d5b', '#3b6fd8', '#8a5cd0', '#d0663b', '#c2417c', '#b8860b', '#0e8aa0', '#64748b'];
const base = (p) => String(p || '').replace(/[\\/]+$/, '').split(/[\\/]/).pop() || '';
/** The daemon's message; an older daemon than the page answers "not found" to the projects API. */
const why = (e) => {
  const m = String((e && e.message) || e).trim();
  return /^not found$/i.test(m) ? 'The running Tirith is older than this page: quit and reopen Tirith, then add the project.' : m;
};

export function AddProject({ onClose }) {
  const [folder, setFolder] = useState('');
  const [name, setName] = useState('');
  const [color, setColor] = useState('');
  const [check, setCheck] = useState(null); // the dry run of `folder`: {r} or {err}
  const [busy, setBusy] = useState(''); // '', 'pick' or 'add'
  const [err, setErr] = useState('');
  const [done, setDone] = useState(null);
  const [found] = useFetch('/api/projects/folders', []);
  const input = useRef(null);
  const seq = useRef(0);
  const close = useRef(onClose);
  close.current = onClose;

  useEffect(() => {
    if (input.current) input.current.focus();
    const key = (e) => { if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); close.current(); } };
    addEventListener('keydown', key, true);
    return () => removeEventListener('keydown', key, true);
  }, []);
  useEffect(() => { // the dry run, once the folder stops changing
    const n = ++seq.current;
    setCheck(null);
    if (!folder.trim()) return undefined;
    const t = setTimeout(() => api('POST', '/api/projects', { folder, dry: true }).then(
      (r) => n === seq.current && setCheck({ r }), (e) => n === seq.current && setCheck({ err: why(e) })), 300);
    return () => clearTimeout(t);
  }, [folder]);

  const r = check && check.r;
  const q = folder.trim().toLowerCase();
  const list = (found || []).filter((f) => !q || `${f.name}\n${f.folder}`.toLowerCase().includes(q));
  const show = (id) => { go(route().view, id); close.current(); };
  const pick = async () => {
    setBusy('pick'); setErr('');
    try { const p = await api('POST', '/api/projects/pick'); if (p.folder) setFolder(p.folder); } catch (e) { setErr(why(e)); }
    setBusy('');
    if (input.current) input.current.focus();
  };
  const add = async () => {
    if (busy || !r || r.exists) return;
    setBusy('add'); setErr('');
    try {
      const res = await api('POST', '/api/projects', { folder, name, color });
      setDone(res);
      go(route().view, res.id);
    } catch (e) { setErr(why(e)); }
    setBusy('');
  };
  const enter = (e) => { if (e.key === 'Enter') { e.preventDefault(); add(); } };
  const todo = r && [r.git ? '' : 'create a git repository in it', r.plan ? '' : 'start its plan with a first step'].filter(Boolean).join(' and ');

  const body = done
    ? html`<p class="ap-p">${done.exists ? 'Already a project: ' : 'Added '}<b>${done.name || done.id}</b></p>
      <p class="ap-folder" title=${done.folder}>${done.folder}</p>
      ${(done.notes || []).map((n) => html`<p class="ap-note">${n}</p>`)}
      <div class="ap-act"><button type="button" class="ap-b pri" onClick=${() => close.current()}>Done</button></div>`
    : html`<label class="ap-l" for="ap-folder">Folder</label>
      <div class="ap-row"><input id="ap-folder" ref=${input} class="ap-in" placeholder="~/DEVELOP/my-app" autocomplete="off" spellcheck="false"
        value=${folder} onInput=${(e) => setFolder(e.target.value)} onKeyDown=${enter} />
        <button type="button" class="ap-b" disabled=${!!busy} onClick=${pick}>${busy === 'pick' ? 'Choosing…' : 'Choose…'}</button></div>
      ${r && r.exists ? html`<p class="ap-note">Already a project: <b>${r.name || r.id}</b> · <button type="button" class="alink" onClick=${() => show(r.id)}>Show it</button></p>`
        : r ? html`<p class="ap-note" title=${r.folder}>${todo ? `Tirith will ${todo}.` : 'Ready to add.'}</p>`
        : check && check.err ? html`<div class="ierr ap-err" role="alert">${check.err}</div>`
        : list.length ? html`<div class="ap-l">Git repositories next to your projects</div>
          <div class="ap-opts">${list.map((f) => html`<button type="button" class="ap-opt" key=${f.folder} title=${f.folder}
            onClick=${() => { setFolder(f.folder); if (input.current) input.current.focus(); }}><b>${f.name}</b><span>${f.folder}</span></button>`)}</div>`
        : null}
      ${r && r.exists ? null : html`<label class="ap-l" for="ap-name">Name</label>
      <input id="ap-name" class="ap-in" maxlength="80" autocomplete="off" placeholder=${(r && r.name) || base(folder) || 'Name'}
        value=${name} onInput=${(e) => setName(e.target.value)} onKeyDown=${enter} />
      <div class="ap-l" id="ap-color">Color</div>
      <div class="ap-sws" role="radiogroup" aria-labelledby="ap-color">${COLORS.map((c) => {
        const on = (color || (r && r.color)) === c;
        return html`<button type="button" class=${`ap-sw${on ? ' on' : ''}`} style=${`background:${c}`} role="radio" aria-checked=${on}
          aria-label=${`Color ${c}`} onClick=${() => setColor(c)}></button>`;
      })}</div>`}
      ${err ? html`<div class="ierr ap-err" role="alert">${err}</div>` : null}
      <div class="ap-act"><button type="button" class="ap-b" onClick=${() => close.current()}>Cancel</button>
        <button type="button" class="ap-b pri" disabled=${!!busy || !r || r.exists} onClick=${add}>${busy === 'add' ? 'Adding…' : 'Add'}</button></div>`;
  return html`<div class="ap-ov" role="dialog" aria-modal="true" aria-labelledby="ap-title"
    onClick=${(e) => { if (e.target === e.currentTarget && !busy) close.current(); }}>
    <div class="ap-card"><h3 id="ap-title">Add project</h3>${body}</div></div>`;
}

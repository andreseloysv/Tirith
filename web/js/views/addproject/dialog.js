// The "+" next to Projects: a dialog that adds a folder as a project (POST /api/projects). The folder is picked from
// the git repositories next to the projects, chosen in macOS's folder picker (the daemon opens it) or typed; a dry
// run says before Add what adding it does (a git repository, a first plan) or why the folder cannot be a project.
import { html, useState, useRef } from '../../../vendor/preact-htm.js';
import { useFetch } from '../../store.js';
import { useModal, useDoneFocus, useDryRun } from './effects.js';
import { actions } from './actions.js';
import { doneBody } from './done.js';
import { formBody } from './form.js';

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
  const doneBtn = useRef(null);
  const seq = useRef(0);
  const live = useRef(null);
  live.current = { onClose, busy };
  const close = () => live.current.onClose();

  useModal(input, live, close);
  useDoneFocus(done, doneBtn);
  useDryRun(folder, seq, setCheck);

  const r = check && check.r;
  const q = folder.trim().toLowerCase();
  const list = (found || []).filter((f) => !q || `${f.name}\n${f.folder}`.toLowerCase().includes(q));
  const d = { folder, setFolder, name, setName, color, setColor, check, r, busy, setBusy, err, setErr, setDone, list, input, close };
  const a = actions(d);

  const body = done ? doneBody(done, doneBtn, close) : formBody(d, a);
  return html`<div class="ap-ov" role="dialog" aria-modal="true" aria-labelledby="ap-title"
    onClick=${(e) => { if (e.target === e.currentTarget && !busy) close(); }}>
    <div class="ap-card"><h3 id="ap-title">Add project</h3>${body}</div></div>`;
}

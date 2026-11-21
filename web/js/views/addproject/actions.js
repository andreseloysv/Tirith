// What the dialog's buttons do: choose a folder in macOS's picker, add the project, show one that exists, Enter.
import { api } from '../../store.js';
import { go, route } from '../../common.js';
import { why } from './util.js';

/** `d` is the dialog's state: folder, name, color, busy, r, input, close and their setters. */
export function actions(d) {
  const show = (id) => { go(route().view, id); d.close(); };
  const pick = async () => {
    d.setBusy('pick'); d.setErr('');
    try { const p = await api('POST', '/api/projects/pick'); if (p.folder) d.setFolder(p.folder); } catch (e) { d.setErr(why(e)); }
    d.setBusy('');
    if (d.input.current) d.input.current.focus();
  };
  const add = async () => {
    if (d.busy || !d.r || d.r.exists) return;
    d.setBusy('add'); d.setErr('');
    try {
      const res = await api('POST', '/api/projects', { folder: d.folder, name: d.name, color: d.color });
      d.setDone(res);
      go(route().view, res.id);
    } catch (e) { d.setErr(why(e)); }
    d.setBusy('');
  };
  const enter = (e) => { if (e.key === 'Enter') { e.preventDefault(); add(); } };
  return { show, pick, add, enter };
}

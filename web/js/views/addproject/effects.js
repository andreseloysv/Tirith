// The dialog's effects: a modal over an inert page, the Done button keeping the focus, and the dry run of the folder.
import { useEffect } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { why } from './util.js';

/** A modal: the page behind is inert while it is open, and the focus goes back where it was. */
export function useModal(input, live, close) {
  useEffect(() => {
    const shell = document.getElementById('shell');
    const back = document.activeElement;
    if (shell) shell.inert = true;
    if (input.current) input.current.focus();
    const key = (e) => { if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); if (!live.current.busy) close(); } };
    addEventListener('keydown', key, true);
    return () => {
      removeEventListener('keydown', key, true);
      if (shell) shell.inert = false;
      if (back && back.focus) back.focus();
    };
  }, []);
}

/** After every render: the route change re-renders the page. */
export function useDoneFocus(done, doneBtn) {
  useEffect(() => { if (done && doneBtn.current && document.activeElement !== doneBtn.current) doneBtn.current.focus(); });
}

/** The dry run, once the folder stops changing. */
export function useDryRun(folder, seq, setCheck) {
  useEffect(() => {
    const n = ++seq.current;
    setCheck(null);
    if (!folder.trim()) return undefined;
    const t = setTimeout(() => api('POST', '/api/projects', { folder, dry: true }).then(
      (r) => n === seq.current && setCheck({ r }), (e) => n === seq.current && setCheck({ err: why(e) })), 300);
    return () => clearTimeout(t);
  }, [folder]);
}

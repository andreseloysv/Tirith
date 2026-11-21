// The files being attached to the next message: picked, pasted or dropped on the chat.
import { useState, useEffect } from '../../../vendor/preact-htm.js';

const PIC_MAX = 6, FILE_BYTES = 25 * 1024 * 1024;

/** The files that can be sent, and why the last one that cannot was left out. */
function accepted(files) {
  let bad = null;
  const ok = files.filter((f) => {
    if (!f.size) { bad = `${f.name || 'The file'} is empty (a folder cannot be attached).`; return false; }
    if (f.size > FILE_BYTES) { bad = `${f.name} is over 25 MB.`; return false; }
    return true;
  });
  return { ok, bad };
}
const entryOf = (f) => ({
  file: f, name: f.name || 'pasted.png', type: f.type, size: f.size,
  url: /^image\/(png|jpeg|gif|webp)$/.test(f.type) ? URL.createObjectURL(f) : null,
});

/** Files dragged over the chat: the drop target; dropped beside it they do nothing (no navigation). */
function useDrop(setDrag, addFiles) {
  useEffect(() => {
    let off = 0;
    const has = (e) => !!(e.dataTransfer && Array.from(e.dataTransfer.types || []).includes('Files'));
    const over = (e) => {
      if (!has(e)) return;
      e.preventDefault();
      clearTimeout(off);
      const on = !!(e.target.closest && e.target.closest('.mchat'));
      e.dataTransfer.dropEffect = on ? 'copy' : 'none';
      setDrag(on);
    };
    const leave = () => { clearTimeout(off); off = setTimeout(() => setDrag(false), 100); };
    const drop = (e) => {
      if (!has(e)) return;
      e.preventDefault();
      setDrag(false);
      if (e.target.closest && e.target.closest('.mchat')) addFiles(e.dataTransfer.files);
    };
    document.addEventListener('dragover', over);
    document.addEventListener('dragleave', leave);
    document.addEventListener('drop', drop);
    return () => { document.removeEventListener('dragover', over); document.removeEventListener('dragleave', leave); document.removeEventListener('drop', drop); };
  }, []);
}

/** pics: {file, name, type, size, url (pictures only)}; `setErr` says why a file was left out. */
export function useAttach(setErr) {
  const [pics, setPics] = useState([]);
  const [drag, setDrag] = useState(false);
  const addFiles = (list) => {
    const files = Array.from(list || []);
    if (!files.length) return;
    let { ok, bad } = accepted(files);
    setPics((cur) => {
      const room = PIC_MAX - cur.length;
      if (ok.length > room) bad = `At most ${PIC_MAX} files per message.`;
      return cur.concat(ok.slice(0, Math.max(0, room)).map(entryOf));
    });
    setErr(bad);
  };
  const unpic = (i) => setPics((cur) => cur.filter((_, k) => k !== i));
  useDrop(setDrag, addFiles);
  return { pics, setPics, drag, addFiles, unpic };
}

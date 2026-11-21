// Which projects of the menu are open and which were folded, remembered in localStorage.
import { useState } from '../../vendor/preact-htm.js';

const OPEN_KEY = 'tirith.nav.open';
const FOLDED_KEY = 'tirith.nav.folded';
function loadSet(key) { try { return new Set(JSON.parse(localStorage.getItem(key) || '[]')); } catch { return new Set(); } }
function saveSet(key, s) { try { localStorage.setItem(key, JSON.stringify([...s])); } catch { /* private window */ } }

/** `{open, folded, remember(id, want)}`: remember one project as open (`want`) or folded; the two sets stay
    disjoint, so a folded current project stays folded on arrival and an opened one stays open. */
export function useNavMemory() {
  const [open, setOpen] = useState(() => loadSet(OPEN_KEY));
  const [folded, setFolded] = useState(() => loadSet(FOLDED_KEY));
  const remember = (id, want) => {
    const o = new Set(open); if (want) o.add(id); else o.delete(id); setOpen(o); saveSet(OPEN_KEY, o);
    const f = new Set(folded); if (want) f.delete(id); else f.add(id); setFolded(f); saveSet(FOLDED_KEY, f);
  };
  return { open, folded, remember };
}

// The current route, redrawn when the hash changes.
import { useState, useEffect } from '../../vendor/preact-htm.js';
import { route } from '../common.js';

export function useRoute() {
  const [r, set] = useState(route());
  useEffect(() => {
    const on = () => set(route());
    addEventListener('hashchange', on);
    return () => removeEventListener('hashchange', on);
  }, []);
  return r;
}

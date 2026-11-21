// Small helpers of the chat: a component that repaints only when its props change, the draft kept in
// localStorage, a byte size in words and a project's name.
import { html, Component } from '../../../vendor/preact-htm.js';

/** A component that repaints only when `same(old props, new props)` says they differ. */
export function pure(Inner, same) {
  return class extends Component {
    shouldComponentUpdate(next) { return !same(this.props, next); }
    render(props) { return html`<${Inner} ...${props} />`; }
  };
}

export function store(key, val) {
  try {
    if (val === undefined) return localStorage.getItem(key) || '';
    if (val) localStorage.setItem(key, val); else localStorage.removeItem(key);
  } catch { /* private window: the draft just does not survive */ }
  return '';
}
export function size(n) { return n < 1024 ? n + ' B' : n < 1048576 ? Math.round(n / 1024) + ' KB' : (n / 1048576).toFixed(1) + ' MB'; }
export const projName = (projects, id) => (projects.find((p) => p.id === id) || {}).name || id;

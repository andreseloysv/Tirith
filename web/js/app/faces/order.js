// The faces' order in the sidebar, as the owner dragged it: kept per browser, like the menu's folds.
const KEY = 'tirith.faces.order';

export function loadOrder() {
  try { const v = JSON.parse(localStorage.getItem(KEY) || '[]'); return Array.isArray(v) ? v : []; } catch { return []; }
}
export function saveOrder(ids) {
  try { localStorage.setItem(KEY, JSON.stringify(ids)); } catch { /* private window: the order lasts this visit */ }
}

/** `list` in the saved order: the faces it names first, in its order, then the others (a new bot) as they came. */
export function sorted(list, order) {
  const at = new Map(order.map((id, i) => [id, i]));
  return list.map((f, i) => [f, at.has(f.id) ? at.get(f.id) : order.length + i]).sort((a, b) => a[1] - b[1]).map((x) => x[0]);
}

/** The ids with `id` moved to index `to`. */
export function moved(ids, id, to) {
  const rest = ids.filter((x) => x !== id);
  rest.splice(Math.max(0, Math.min(to, rest.length)), 0, id);
  return rest;
}

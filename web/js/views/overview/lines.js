// The tree's lines on the Home (tree.js), measured from where the faces are, so they stay right however the helpers
// lie. Wide, as in the owner's reference: a trunk down from the boss and, over each row of helpers, a bar with rounded
// ends that drops to each of them (the trunk runs down the middle, between the helpers of a full row). Narrow (the
// helpers in one column, 05-tree.css): a trunk down from the boss's face and a branch to each helper, the last one
// rounded.
import { useLayoutEffect, useState } from '../../../vendor/preact-htm.js';

const R = 10;    /* rounded corners */
const OVER = 16; /* how far above a row of faces its bar runs */

/** The wide tree: rows of helpers under bars (`boss`, `kids`: measured boxes, as linesOf reads them). */
export function rows(boss, kids) {
  const all = [];
  for (const k of kids) { const row = all.find((r) => Math.abs(r[0].top - k.top) < 4); if (row) row.push(k); else all.push([k]); }
  let d = '', end = boss.bottom;
  for (const row of all) {
    const y = row[0].top - OVER, xs = row.map((k) => k.x), l = Math.min(...xs), r = Math.max(...xs);
    end = y;
    if (r - l < 2 * R) { d += `M${l} ${y}V${row[0].top}`; continue; } /* one helper under the trunk: it goes on */
    d += `M${l} ${row[0].top}V${y + R}Q${l} ${y} ${l + R} ${y}H${r - R}Q${r} ${y} ${r} ${y + R}V${row[0].top}`;
    for (const k of row) if (k.x > l + 1 && k.x < r - 1) d += `M${k.x} ${y}V${k.top}`;
  }
  return `M${boss.x} ${boss.bottom + 4}V${end}${d}`;
}

/** The narrow tree: a comb, the trunk under the boss's face and a branch to the left of each helper's face. */
export function comb(face, kids) {
  const last = kids[kids.length - 1], x = face.x;
  let d = `M${x} ${face.bottom + 2}V${last.y - R}Q${x} ${last.y} ${x + R} ${last.y}H${last.left - 6}`;
  for (const k of kids.slice(0, -1)) d += `M${x} ${k.y}H${k.left - 6}`;
  return d;
}

/** The path, in the box's own pixels, through the boss (`.hm-tboss`) and the helpers' faces (`.hm-kid .hm-face`). */
export function linesOf(box) {
  const at = box.getBoundingClientRect();
  const rel = (el) => { const r = el.getBoundingClientRect(); return { x: r.left - at.left + r.width / 2, y: r.top - at.top + r.height / 2, top: r.top - at.top, bottom: r.bottom - at.top, left: r.left - at.left }; };
  const bossEl = box.querySelector('.hm-tboss'), list = box.querySelector('.hm-kids');
  const kids = [...box.querySelectorAll('.hm-kid .hm-face')].map(rel);
  if (!bossEl || !list || !kids.length) return '';
  return getComputedStyle(list).flexDirection === 'column' ? comb(rel(bossEl.querySelector('.hm-face')), kids) : rows(rel(bossEl), kids);
}

/** The path of the tree in `ref`, drawn again whenever `key` (its helpers) or the box's size changes. */
export function useLines(ref, key) {
  const [d, setD] = useState('');
  useLayoutEffect(() => {
    const box = ref.current;
    if (!box) return undefined;
    const draw = () => setD(linesOf(box));
    draw();
    const seen = typeof ResizeObserver === 'function' ? new ResizeObserver(draw) : null;
    if (seen) seen.observe(box);
    return () => { if (seen) seen.disconnect(); };
  }, [key]);
  return d;
}

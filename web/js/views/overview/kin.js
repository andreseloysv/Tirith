// The faces a boss's helpers wear on the Home's tree (tree.js): each a shape and a colour picked from its own id, the
// same every time it shows; never its boss's own shape or colour, and none twice among one boss's helpers while the
// lists have room. The app icon's star stays the owner's own: no helper wears it.
const hash = (s) => { let x = 2166136261; for (let i = 0; i < s.length; i++) x = Math.imul(x ^ s.charCodeAt(i), 16777619); return x >>> 0; };

/* the first pair, in this helper's own order of the lists, that `ok` takes */
function first(h, shapes, colors, ok) {
  for (let a = 0; a < shapes.length; a++) {
    for (let b = 0; b < colors.length; b++) {
      const shape = shapes[(h + a) % shapes.length], color = colors[((h >>> 8) + b) % colors.length];
      if (ok(shape, color)) return { shape, color };
    }
  }
  return null;
}

/** [{shape, color}] for the helpers `ids` (in their order) under `boss` ({shape, color}); `shapes` and `colors`: the
    lists the page draws (face.js), read as they are. */
export function helperFaces(ids, boss, shapes, colors) {
  const S = shapes.filter((s) => s !== 'hero');
  const usedS = new Set([boss.shape]), usedC = new Set([boss.color]), taken = new Set([`${boss.shape}|${boss.color}`]);
  const free = (s, c) => !taken.has(`${s}|${c}`);
  return ids.map((id) => {
    const h = hash(String(id));
    const f = first(h, S, colors, (s, c) => !usedS.has(s) && !usedC.has(c))
      || first(h, S, colors, (s, c) => free(s, c) && (!usedS.has(s) || !usedC.has(c)))
      || first(h, S, colors, free) || { shape: S[h % S.length], color: colors[h % colors.length] };
    usedS.add(f.shape); usedC.add(f.color); taken.add(`${f.shape}|${f.color}`);
    return f;
  });
}

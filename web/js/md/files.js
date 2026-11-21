// File references: a path in the text that the page confirms becomes a file link (a.fref).
import { esc } from './esc.js';
import { ctx } from './refs.js';

const SHAPE = /^(?:~\/|\.{1,2}\/|\/)?[\w.@+-]+(?:\/[\w.@+-]+)*\/?$/;
const NAME_EXT = /^(?:[\w@+-]*[A-Za-z][\w@+-]*(?:\.[\w-]+)*\.[A-Za-z][A-Za-z0-9]{0,9}|\.[A-Za-z][\w.-]*)$/;
export const PLAIN = /(^|[\s(\[*;])((?:~\/|\.{1,2}\/|\/)?[\w.@+-]+(?:\/[\w.@+-]+)*\/?(?::\d+){0,2}[.,:;!?]*)(?=$|[\s)\]*&])/g;
function looksLikePath(p) {
  if (!p || p.indexOf('://') >= 0 || p.charAt(0) === '-' || !SHAPE.test(p) || !/[A-Za-z]/.test(p)) return false;
  return p.replace(/^\/+|\/+$/g, '').indexOf('/') >= 0 || /\/$/.test(p) || /^(~\/|\.\.?\/|\/)/.test(p) || NAME_EXT.test(p);
}
/** "cli.py:42:7." -> {path: "cli.py", line: 42, col: 7, rest: "."}; null when it is not a path. */
export function splitRef(text) {
  const t = String(text).replace(/[.,:;!?]+$/, ''), m = /^(.+?)(?::(\d+)(?::(\d+))?)?$/.exec(t);
  if (!m || !looksLikePath(m[1])) return null;
  return { path: m[1], line: m[2] ? +m[2] : null, col: m[3] ? +m[3] : null, rest: String(text).slice(t.length) };
}
function normRef(p) { while (p.indexOf('./') === 0) p = p.slice(2); return p.replace(/\/+$/, '') || p; }
export function fileLink(ref, inner) {
  if (!ctx.refs || !ctx.refs.file || !ref || !ctx.refs.file(normRef(ref.path))) return null;
  const at = ref.path + (ref.line ? ':' + ref.line + (ref.col ? ':' + ref.col : '') : '');
  return `<a class="fref" href="#" data-act="fopen" data-project="${esc(ctx.refs.project || '')}" data-path="${esc(ref.path)}"` +
    (ref.line ? ` data-line="${ref.line}"` : '') + (ref.col ? ` data-col="${ref.col}"` : '') + ` title="Open ${esc(at)}">${inner}</a>`;
}

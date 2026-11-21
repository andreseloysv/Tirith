// The opts of the md() call being rendered ({file(path), project, lane(id)}), shared by the steps that link.

export const ctx = { refs: null };

export function withRefs(opts, fn) {
  const was = ctx.refs;
  ctx.refs = opts && (typeof opts.file === 'function' || typeof opts.lane === 'function') ? opts : null;
  try { return fn(); } finally { ctx.refs = was; }
}

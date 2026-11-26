// What a lane or a plan item spent (the daemon's `spent`: runs, tokens, dollars), as short text.

const k = (n) => (n >= 1e6 ? `${(n / 1e6).toFixed(1)}M` : n >= 1e3 ? `${(n / 1e3).toFixed(1)}k` : String(n || 0));

/** `$0.0123`, or `no price` when its runs have none; empty for something that has not run. */
export const dollars = (s) => (!s || !s.runs ? '' : s.cost_usd == null ? 'no price' : `$${s.cost_usd.toFixed(s.cost_usd < 1 ? 4 : 2)}`);

/** `$0.0123 · 4.2k in · 310 out`, for a title or a detail row. */
export const spentText = (s) => (!s || !s.runs ? '' : `${dollars(s)} · ${k(s.tokens_in + s.tokens_cache_read + s.tokens_cache_write)} in · ${k(s.tokens_out)} out · ${s.runs} run${s.runs === 1 ? '' : 's'}`);

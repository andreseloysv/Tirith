// The tokens a turn used, for the quiet line under a bot's answer (`Made` in react.js): the daemon keeps them on
// the turn (tokens_in, tokens_out and the cache's two), the same sum the Bots page counts per run.

const count = (v) => Math.max(0, Number(v) || 0);

/** What the model read in that turn (new text, and what its cache gave or took) and what it wrote. */
export function usedTokens(turn) {
  const read = count(turn.tokens_in) + count(turn.tokens_cache_read) + count(turn.tokens_cache_write);
  const written = count(turn.tokens_out);
  return { read, written, all: read + written };
}

/** A count in few characters: 950, 68k, 7.8M. */
export function shortCount(n) {
  if (n < 1000) return String(Math.round(n));
  return n < 999500 ? `${Math.round(n / 1000)}k` : `${Math.round(n / 100000) / 10}M`;
}

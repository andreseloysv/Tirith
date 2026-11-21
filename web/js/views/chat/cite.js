// A bot's memory citations in its chat: each [m<N>] (its own memory's note) or [s<N>] (the shared memory's) is a
// small raised number that shows the note's words on hover or tap, and is left out of what is copied. The words
// come from GET /api/bots/<id>/library; the stored message keeps its marks (Tirith counts the notes cited).
import { useMemo } from '../../../vendor/preact-htm.js';
import { useFetch } from '../../store.js';
import { esc } from '../../md.js';

const NOTE = /^\s*[-*]\s+\[([ms]\d+)\]\s*(.*)$/;
const MARK = /[ \t]*\[([ms])(\d+)\]/g;   /* with the spaces before it: the number sits on the word, as a footnote's */

/** The numbered notes of a library: {m1: 'words', s2: 'words', …}. */
export function notesOf(lib) {
  const notes = {};
  [lib && lib.memory, lib && lib.shared_memory].forEach((text) => String(text || '').split('\n').forEach((line) => {
    const n = NOTE.exec(line);
    if (n) notes[n[1]] = n[2].trim();
  }));
  return notes;
}

/** The notes of `bot` (useSpeaker's bot; null: no bot, no notes), kept current when its library changes. */
export function useNotes(bot) {
  const [lib] = useFetch(bot ? `/api/bots/${encodeURIComponent(bot.id)}/library` : '', ['library'], [bot && bot.id]);
  return useMemo(() => (bot ? notesOf(lib) : null), [lib, bot && bot.id]);
}

/** `html` (an answer's drawn Markdown) with its marks as raised numbers on the word before them, so a copy reads
    «Hecho.»; unchanged without notes (not a bot). */
export function cited(html, notes) {
  if (!notes) return html;
  return html.replace(MARK, (_, kind, n) => {
    const key = kind + n, words = notes[key];
    const label = `${kind === 's' ? 'Shared memory' : 'Memory'}, note ${n}: ${words || 'no longer in the memory'}`;
    return `<sup class="mc-cite${kind === 's' ? ' shared' : ''}" tabindex="0" role="note" aria-label="${esc(label)}" data-note="${esc(words || label)}">${n}</sup>`;
  });
}

// The log text as entries in file order: {kind, text, detail?, res?, sum?}. Plain lines between JSON lines are
// gathered into one output (or error) entry.
import { stripAnsi } from './text.js';
import { message, result } from './events.js';
import { t } from '../../../i18n.js';

/** One JSON line of the stream as its entries. */
function jsonLine(j, line, tools, out) {
  const type = j.type;
  if (type === 'system') {
    out.push({ kind: 'system', text: j.subtype === 'init' ? `${t('Session started')}${j.model ? ` · ${j.model}` : ''}` : t('System: {name}', { name: j.subtype || '' }), detail: j.session_id || '' });
  } else if (type === 'assistant' || type === 'user') {
    message(j, tools, out);
  } else if (type === 'result') {
    out.push(result(j));
  } else if (type === 'stream_event' || type === 'rate_limit_event') {
    // noise
  } else {
    out.push({ kind: 'system', text: type ? t('Event: {name}', { name: type }) : t('Event'), detail: line.slice(0, 300) });
  }
}

export function parseLog(text) {
  const out = [], tools = {};
  let plain = [];
  const flush = () => {
    const t = plain.join('\n').replace(/\s+$/, '');
    if (t) out.push({ kind: /\b(error|panic|fatal|Traceback)\b/i.test(t) && t.length < 600 ? 'error' : 'output', text: t });
    plain = [];
  };
  for (const raw of String(text || '').split('\n')) {
    const line = stripAnsi(raw);
    let j = null;
    if (line.startsWith('{')) { try { j = JSON.parse(line); } catch { j = null; } }
    if (!j || typeof j !== 'object') { plain.push(line); continue; }
    flush();
    jsonLine(j, line, tools, out);
  }
  flush();
  out.forEach((e, i) => { e.k = i; });
  return out;
}

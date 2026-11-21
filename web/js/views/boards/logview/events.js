// One stream-json line as log entries: a tool call, a tool's result (which also settles its test run), the
// final result with its usage, and the session's other events.
import { dur } from '../../../common.js';
import { TEST_RE, EDIT, READ, stripAnsi, resultText, short, testResult, counts } from './text.js';

export function toolUse(c) {
  const name = c.name || 'tool', s = short(c.input);
  if (name === 'Bash') return TEST_RE.test(s) ? { kind: 'test', text: s, res: 'running' } : { kind: 'command', text: s };
  if (EDIT.has(name)) {
    const ins = c.input && (c.input.content || c.input.new_string || '');
    const del = c.input && (c.input.old_string || '');
    const n = (x) => (x ? String(x).split('\n').length : 0);
    return { kind: 'edit', text: s, plus: n(ins), minus: n(del) };
  }
  if (name === 'Task' || name === 'Agent') return { kind: 'spawn', text: (c.input && (c.input.description || c.input.subagent_type)) || s };
  if (READ.has(name)) return { kind: 'read', text: `${name} ${s}` };
  return { kind: 'command', text: `${name}: ${s}`, tool: name };
}

export function toolResult(c, tools, out) {
  const r = stripAnsi(resultText(c.content)).replace(/\s+$/, '');
  const src = tools[c.tool_use_id];
  if (src && src.kind === 'test') { src.res = c.is_error ? 'failed' : testResult(r) || 'passed'; src.sum = counts(r); }
  if (c.is_error) out.push({ kind: 'error', text: r || 'Tool error' });
  else if (r && src && (src.kind === 'command' || src.kind === 'test')) out.push({ kind: 'output', text: r });
}

/** An assistant or user message: the assistant's text, its tool calls, and the results that come back. */
export function message(j, tools, out) {
  const t = j.type;
  const content = (j.message && j.message.content) || [];
  if (typeof content === 'string') { if (t === 'assistant') out.push({ kind: 'say', text: content }); return; }
  for (const c of content) {
    if (c.type === 'text' && c.text && t === 'assistant') out.push({ kind: 'say', text: c.text });
    else if (c.type === 'tool_use') {
      const e = toolUse(c);
      out.push(e);
      if (c.id) tools[c.id] = e;
    } else if (c.type === 'tool_result') toolResult(c, tools, out);
  }
}

export function result(j) {
  const u = j.usage || {}, bits = [];
  if (j.total_cost_usd != null) bits.push(`$${Number(j.total_cost_usd).toFixed(2)}`);
  if (u.input_tokens != null) bits.push(`${u.input_tokens} in · ${u.output_tokens || 0} out`);
  if (j.num_turns) bits.push(`${j.num_turns} turns`);
  if (j.duration_ms) bits.push(dur(j.duration_ms / 1000));
  return { kind: j.is_error ? 'error' : 'done', text: resultText(j.result) || (j.is_error ? 'Ended with an error' : 'Done'), usage: bits.join(' · ') };
}

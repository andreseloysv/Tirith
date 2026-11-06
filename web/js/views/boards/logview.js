// A lane's log, readable: the tail of its stream-json (or plain) output parsed into entries — the agent's text,
// commands, edits, reads, test runs, errors, results — newest first, one look per kind (port of the old
// logview.js; same classes: .logv .lchips .llist .le).
import { html, useState, useMemo } from '../../../vendor/preact-htm.js';
import { dur } from '../../common.js';
import { Md } from './md.js';

const FILTERS = [['all', 'All'], ['command', 'Commands'], ['edit', 'Edits'], ['error', 'Errors'], ['test', 'Tests']];
const KINDS = { command: ['command', 'test', 'output'], edit: ['edit'], error: ['error'], test: ['test'] };
const SHOW = 120;
const TEST_RE = /\b(test|tests|pytest|jest|vitest|mocha|cargo test|go test|gradlew\s+\S*test|npm (run )?test|ctest)\b/i;
const EDIT = new Set(['Edit', 'Write', 'MultiEdit', 'NotebookEdit']);
const READ = new Set(['Read', 'Grep', 'Glob', 'LS', 'WebFetch', 'WebSearch', 'TodoWrite']);

const ic = (d) => html`<svg class="lic" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"
  stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d=${d} /></svg>`;
const I = {
  file: 'M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z M14 3v5h5',
  flask: 'M9 3h6M10 3v6L4.5 19a1.6 1.6 0 0 0 1.4 2.4h12.2a1.6 1.6 0 0 0 1.4-2.4L14 9V3 M7.5 15h9',
  branch: 'M6 3a2 2 0 100 4 2 2 0 000-4z M6 17a2 2 0 100 4 2 2 0 000-4z M18 7a2 2 0 100 4 2 2 0 000-4z M6 7v10M18 11c0 4-6 3-12 6',
  flag: 'M5 21V4M5 4h11l-2 4 2 4H5',
};

const stripAnsi = (s) => String(s).replace(/\x1b\[[0-9;]*[A-Za-z]/g, '');
function resultText(c) {
  if (typeof c === 'string') return c;
  if (Array.isArray(c)) return c.map((x) => (typeof x === 'string' ? x : x.text || (x.type === 'image' ? '[image]' : ''))).join('\n');
  return c == null ? '' : JSON.stringify(c);
}
function short(input) {
  if (!input || typeof input !== 'object') return String(input || '');
  return input.command || input.file_path || input.path || input.pattern || input.url || input.query || input.description || input.prompt
    || Object.values(input).filter((v) => typeof v === 'string')[0] || JSON.stringify(input);
}
function testResult(out) {
  if (/\b(FAILED|failed|error\[|panicked|AssertionError)\b/.test(out) && !/\b0 failed\b/.test(out)) return 'failed';
  if (/\b(passed|ok\.|test result: ok|PASS)\b/.test(out)) return 'passed';
  return '';
}
function counts(out) {
  const p = /(\d+) passed/.exec(out), f = /(\d+) failed/.exec(out);
  return p || f ? `${p ? p[1] : 0} passed, ${f ? f[1] : 0} failed` : '';
}

/** The log text as entries in file order: {kind, text, detail?, res?, sum?} */
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
    const t = j.type;
    if (t === 'system') {
      out.push({ kind: 'system', text: j.subtype === 'init' ? `Session started${j.model ? ` · ${j.model}` : ''}` : `System: ${j.subtype || ''}`, detail: j.session_id || '' });
    } else if (t === 'assistant' || t === 'user') {
      const content = (j.message && j.message.content) || [];
      if (typeof content === 'string') { if (t === 'assistant') out.push({ kind: 'say', text: content }); continue; }
      for (const c of content) {
        if (c.type === 'text' && c.text && t === 'assistant') out.push({ kind: 'say', text: c.text });
        else if (c.type === 'tool_use') {
          const name = c.name || 'tool', s = short(c.input);
          let e;
          if (name === 'Bash') e = TEST_RE.test(s) ? { kind: 'test', text: s, res: 'running' } : { kind: 'command', text: s };
          else if (EDIT.has(name)) {
            const ins = c.input && (c.input.content || c.input.new_string || '');
            const del = c.input && (c.input.old_string || '');
            const n = (x) => (x ? String(x).split('\n').length : 0);
            e = { kind: 'edit', text: s, plus: n(ins), minus: n(del) };
          } else if (name === 'Task' || name === 'Agent') e = { kind: 'spawn', text: (c.input && (c.input.description || c.input.subagent_type)) || s };
          else if (READ.has(name)) e = { kind: 'read', text: `${name} ${s}` };
          else e = { kind: 'command', text: `${name}: ${s}`, tool: name };
          out.push(e);
          if (c.id) tools[c.id] = e;
        } else if (c.type === 'tool_result') {
          const r = stripAnsi(resultText(c.content)).replace(/\s+$/, '');
          const src = tools[c.tool_use_id];
          if (src && src.kind === 'test') { src.res = c.is_error ? 'failed' : testResult(r) || 'passed'; src.sum = counts(r); }
          if (c.is_error) out.push({ kind: 'error', text: r || 'Tool error' });
          else if (r && src && (src.kind === 'command' || src.kind === 'test')) out.push({ kind: 'output', text: r });
        }
      }
    } else if (t === 'result') {
      const u = j.usage || {}, bits = [];
      if (j.total_cost_usd != null) bits.push(`$${Number(j.total_cost_usd).toFixed(2)}`);
      if (u.input_tokens != null) bits.push(`${u.input_tokens} in · ${u.output_tokens || 0} out`);
      if (j.num_turns) bits.push(`${j.num_turns} turns`);
      if (j.duration_ms) bits.push(dur(j.duration_ms / 1000));
      out.push({ kind: j.is_error ? 'error' : 'done', text: resultText(j.result) || (j.is_error ? 'Ended with an error' : 'Done'), usage: bits.join(' · ') });
    } else if (t === 'stream_event' || t === 'rate_limit_event') {
      // noise
    } else {
      out.push({ kind: 'system', text: t ? `Event: ${t}` : 'Event', detail: line.slice(0, 300) });
    }
  }
  flush();
  out.forEach((e, i) => { e.k = i; });
  return out;
}

function More({ open, set }) {
  return html`<button class="link lmore" aria-expanded=${open} onClick=${() => set(!open)}>${open ? 'show less' : 'show more'}</button>`;
}

function Entry({ e }) {
  const [open, setOpen] = useState(false);
  const long = String(e.text).split('\n').length > 3 || e.text.length > 400;
  let body;
  switch (e.kind) {
    case 'command':
      body = html`<span class="lprompt">$</span><code class="lcmd" title=${e.text}>${e.text}</code>`; break;
    case 'output':
      body = html`<pre class=${`lout${open || !long ? ' open' : ''}`}>${e.text}</pre>${long ? html`<${More} open=${open} set=${setOpen} />` : null}`; break;
    case 'edit': {
      const i = e.text.lastIndexOf('/');
      body = html`${ic(I.file)}<span class="lpath" title=${e.text}>${i >= 0 ? html`<span class="mut">${e.text.slice(0, i + 1)}</span>` : null}<b>${e.text.slice(i + 1)}</b></span>
        ${e.plus || e.minus ? html`<span class="lcnt"><span class="plus">+${e.plus}</span> <span class="minus">−${e.minus}</span></span>` : null}`;
      break;
    }
    case 'read':
      body = html`<span class="lread" title=${e.text}>${e.text}</span>`; break;
    case 'error':
      body = html`<div class="lerr"><span class="lerrt">${open || !long ? e.text : e.text.split('\n').slice(0, 3).join('\n')}</span>${long ? html`<${More} open=${open} set=${setOpen} />` : null}</div>`; break;
    case 'test':
      body = html`${ic(I.flask)}<code class="lcmd" title=${e.text}>${e.text}</code>${e.sum ? html`<span class="mut lsum">${e.sum}</span>` : null}<span class=${`rb ${e.res}`}>${e.res}</span>`; break;
    case 'spawn':
      body = html`${ic(I.branch)}<span class="lspawn"><span class="mut">Started sub-agent</span> <b>${e.text}</b></span>`; break;
    case 'done':
      body = html`<div class="ldone"><div class="ldh">${ic(I.flag)}Final answer${e.usage ? html`<span class="mut lusage">${e.usage}</span>` : null}</div><div class="ldt"><${Md} text=${e.text} /></div></div>`; break;
    case 'system':
      body = html`<span class="lsys" title=${e.detail || ''}>${e.text}</span>`; break;
    default:
      body = html`<div class="lsay"><${Md} text=${e.text} /></div>`;
  }
  return html`<li class=${`le ${e.kind}${e.kind === 'test' ? ` ${e.res}` : ''}`}><span class="lt"></span><div class="lb">${body}</div></li>`;
}

/** The log viewer: filter chips, newest first, system events hidden behind a toggle, long logs capped. */
export function LogView({ text }) {
  const all = useMemo(() => parseLog(text).reverse(), [text]);
  const [f, setF] = useState('all');
  const [sys, setSys] = useState(false);
  const [limit, setLimit] = useState(SHOW);
  const hidden = all.filter((e) => e.kind === 'system').length;
  const shown = all.filter((e) => (e.kind === 'system' ? sys && f === 'all' : f === 'all' || (KINDS[f] || []).includes(e.kind)));
  return html`<div class="logv">
    <div class="lchips" role="group" aria-label="Filter log">${FILTERS.map(([k, label]) => {
      const n = k === 'all' ? 0 : all.filter((e) => KINDS[k].includes(e.kind)).length;
      return html`<button class=${`lchip${f === k ? ' on' : ''}`} aria-pressed=${f === k} onClick=${() => setF(k)}>${label}${n ? html` <b>${n}</b>` : null}</button>`;
    })}</div>
    ${shown.length ? html`<ul class="llist" tabindex="0">${shown.slice(0, limit).map((e) => html`<${Entry} key=${e.k} e=${e} />`)}</ul>` : html`<p class="quiet">${all.length ? 'Nothing matches this filter' : 'No log output yet'}</p>`}
    ${shown.length > limit ? html`<button class="link lsyst" onClick=${() => setLimit(limit + SHOW)}>Show ${Math.min(SHOW, shown.length - limit)} older entries</button>` : null}
    ${hidden && f === 'all' ? html`<button class="link lsyst" onClick=${() => setSys(!sys)}>${sys ? 'Hide ' : ''}${hidden} system event${hidden === 1 ? '' : 's'}${sys ? '' : ' hidden'}</button>` : null}
  </div>`;
}

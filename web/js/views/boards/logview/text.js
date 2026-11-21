// Reading a log's pieces: ANSI colors out, a tool result as text, a tool call in one line, a test run's verdict.
export const TEST_RE = /\b(test|tests|pytest|jest|vitest|mocha|cargo test|go test|gradlew\s+\S*test|npm (run )?test|ctest)\b/i;
export const EDIT = new Set(['Edit', 'Write', 'MultiEdit', 'NotebookEdit']);
export const READ = new Set(['Read', 'Grep', 'Glob', 'LS', 'WebFetch', 'WebSearch', 'TodoWrite']);

export const stripAnsi = (s) => String(s).replace(/\x1b\[[0-9;]*[A-Za-z]/g, '');
export function resultText(c) {
  if (typeof c === 'string') return c;
  if (Array.isArray(c)) return c.map((x) => (typeof x === 'string' ? x : x.text || (x.type === 'image' ? '[image]' : ''))).join('\n');
  return c == null ? '' : JSON.stringify(c);
}
export function short(input) {
  if (!input || typeof input !== 'object') return String(input || '');
  return input.command || input.file_path || input.path || input.pattern || input.url || input.query || input.description || input.prompt
    || Object.values(input).filter((v) => typeof v === 'string')[0] || JSON.stringify(input);
}
export function testResult(out) {
  if (/\b(FAILED|failed|error\[|panicked|AssertionError)\b/.test(out) && !/\b0 failed\b/.test(out)) return 'failed';
  if (/\b(passed|ok\.|test result: ok|PASS)\b/.test(out)) return 'passed';
  return '';
}
export function counts(out) {
  const p = /(\d+) passed/.exec(out), f = /(\d+) failed/.exec(out);
  return p || f ? `${p ? p[1] : 0} passed, ${f ? f[1] : 0} failed` : '';
}

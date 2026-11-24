// A stand-in for an agent, for tests that must not call a model (the Windows install check).
// With -p it is a lane: it commits one file in its folder and exits. Without, it is a manager that speaks the
// line protocol: an init line, then for each user line on stdin a tool_use, a text and a result.
import { execFileSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { createInterface } from 'node:readline';

const out = (o) => process.stdout.write(JSON.stringify(o) + '\n');
const args = process.argv.slice(2);

if (args.includes('-p')) {
  const git = (...a) => execFileSync('git', a, { stdio: 'inherit' });
  writeFileSync('fake-agent.txt', `done by the fake agent in lane ${process.env.TIRITH_LANE}\n`);
  git('add', 'fake-agent.txt');
  git('-c', 'user.name=fake', '-c', 'user.email=fake@localhost', 'commit', '-q', '-m', 'fake agent was here');
  console.log('fake-agent: committed');
} else {
  out({ type: 'system', subtype: 'init', session_id: 'fake-session' });
  createInterface({ input: process.stdin }).on('line', (line) => {
    let msg;
    try { msg = JSON.parse(line); } catch { return; }
    if (msg.type !== 'user') return;
    out({ type: 'assistant', message: { content: [{ type: 'tool_use', id: 't1', name: 'Bash', input: { command: 'tirith --version' } }] } });
    out({ type: 'assistant', message: { content: [{ type: 'text', text: 'TIRITH-OK' }] } });
    out({ type: 'result', subtype: 'success', result: 'TIRITH-OK', session_id: 'fake-session' });
  });
}

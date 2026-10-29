// The demo story as state: one request in the chat, a plan with two new steps, two agents, a rule that blocks one,
// claim check and merge, the health dot, one real decision. Built from fixtures/state.demo.json; every number the page
// shows comes from the rows below, so every scene is the real UI on realistic data (no screenshots, no mock-ups).
//
//   import { Story } from './story.mjs';
//   const story = new Story(dir);       // dir: a scratch folder; the page follows dir/state.js
//   story.set({ asked: true });         // cumulative: only the fields you pass change
//   story.write();                      // state.js + the steps files
//
// `web/demo/state.video.js` is the last beat of the story as a static file (node docs/demo/story.mjs --static).
import { readFileSync, writeFileSync, mkdirSync, renameSync } from 'node:fs';

// the page reads these files while they are rewritten: write beside, then rename
const put = (file, text) => { writeFileSync(file + '.tmp', text); renameSync(file + '.tmp', file); };
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '../..');
const iso = (ms) => new Date(ms).toISOString().replace(/\.\d+Z$/, 'Z');

export const REQUEST = 'Add a dark mode, and make the scan faster. Only ask me if you really need me.';
export const DECISION = 'Dark mode is merged and its tests pass. Ship it in v1.2 this week, or hold it for the 2.0 redesign?';

const TODO1 = [{ content: 'Add the plan items', status: 'in_progress' }, { content: 'Write a brief and start an agent per item', status: 'pending' }, { content: 'Merge what passes, report back', status: 'pending' }];
const TODO2 = [{ content: 'Add the plan items', status: 'completed' }, { content: 'Write a brief and start an agent per item', status: 'in_progress' }, { content: 'Merge what passes, report back', status: 'pending' }];
const BRIEF = [['+', '# Step 4: dark mode'], ['+', 'Goal: the app follows the system theme and has a toggle in Settings.'], ['+', 'Done when: `tirith run --project chromasol --stage test` passes'], ['+', 'and the theme tokens have no hard-coded colours left.'], ['+', 'Not allowed: editing the plan, starting helpers, skipping the commit checks.']];

// the manager's turn, one step at a time (the recorder reveals them while it films)
export const MANAGER = [
  { kind: 'text', summary: 'Two things: a dark mode and a faster scan. Reading the plan first.' },
  { kind: 'tool', name: 'Read', summary: 'Read docs/PLAN.md' },
  { kind: 'result', name: 'Read', summary: '# ChromaSol · steps 1-3 done, nothing open  … +31 lines' },
  { kind: 'todo', name: 'TodoWrite', summary: 'Task list', todos: TODO1 },
  { kind: 'tool', name: 'Bash', summary: 'Bash: tirith plan add --project chromasol --step 4 "Theme tokens for light and dark"' },
  { kind: 'result', name: 'Bash', summary: 'step 4 "Dark mode" · item added' },
  { kind: 'tool', name: 'Bash', summary: 'Bash: tirith plan add --project chromasol --step 5 "Reuse the pose task between photos"' },
  { kind: 'result', name: 'Bash', summary: 'step 5 "Faster scan" · item added' },
  { kind: 'todo', name: 'TodoWrite', summary: 'Task list', todos: TODO2 },
  { kind: 'tool', name: 'Write', summary: 'Write briefs/step-4-dark-mode.md', diff: { path: 'briefs/step-4-dark-mode.md', rows: BRIEF, more: 0 } },
  { kind: 'tool', name: 'Bash', summary: 'Bash: tirith lane run --project chromasol --step 4 --item "Theme tokens for light and dark"' },
  { kind: 'result', name: 'Bash', summary: 'lane started · Wren · step 4' },
  { kind: 'tool', name: 'Bash', summary: 'Bash: tirith lane run --project chromasol --step 5 --item "Reuse the pose task between photos"' },
  { kind: 'result', name: 'Bash', summary: 'lane started · Ivren · step 5' },
  { kind: 'text', summary: 'Both agents are working. I merge what passes and bring you only what is yours.' },
];
const RAW = 'tirith: raw-stage: running the tests raw is not allowed. Do this instead: tirith run --project chromasol --stage test';
export const WREN = [
  { kind: 'tool', name: 'Read', summary: 'Read app/src/main/java/ui/Theme.kt' },
  { kind: 'tool', name: 'Edit', summary: 'Edit app/src/main/java/ui/Theme.kt', diff: { path: 'app/src/main/java/ui/Theme.kt', rows: [['-', 'val Background = Color(0xFFFFFFFF)'], ['+', 'val BackgroundLight = Color(0xFFFFFFFF)'], ['+', 'val BackgroundDark = Color(0xFF121212)'], ['+', 'fun background(dark: Boolean) = if (dark) BackgroundDark else BackgroundLight']], more: 0 } },
  { kind: 'tool', name: 'Bash', summary: 'Bash: ./gradlew testDebugUnitTest' },
  { kind: 'result', name: 'Bash', summary: RAW, error: true },
  { kind: 'text', summary: 'Blocked: tests go through Tirith. Using the command it gave me.' },
  { kind: 'tool', name: 'Bash', summary: 'Bash: tirith run --project chromasol --stage test' },
  { kind: 'result', name: 'Bash', summary: 'chromasol#8 · test passed · 86 tests · 46s' },
  { kind: 'tool', name: 'Bash', summary: 'Bash: tirith plan report --status done --files 4 --tests 4' },
  { kind: 'result', name: 'Bash', summary: 'reported: done' },
];
export const IVREN = [
  { kind: 'tool', name: 'Read', summary: 'Read app/src/main/java/scan/PoseRunner.kt' },
  { kind: 'tool', name: 'Grep', summary: 'Grep "PoseLandmarker" in app/src' },
  { kind: 'tool', name: 'Edit', summary: 'Edit app/src/main/java/scan/PoseRunner.kt', diff: { path: 'app/src/main/java/scan/PoseRunner.kt', rows: [['-', 'val task = PoseLandmarker.create(ctx, opts)   // per photo'], ['+', 'private val task by lazy { PoseLandmarker.create(ctx, opts) }'], ['+', 'fun run(bitmap: Bitmap) = task.detect(bitmap)']], more: 0 } },
  { kind: 'tool', name: 'Bash', summary: 'Bash: tirith run --project chromasol --stage test' },
];

export class Story {
  constructor(dir) {
    this.dir = dir; mkdirSync(dir, { recursive: true });
    this.base = JSON.parse(readFileSync(join(root, 'fixtures/state.demo.json'), 'utf8'));
    this.o = { asked: false, mgr: 0, plan: 0, wren: 0, ivren: 0, lanes: false, violation: false, merged: false, health: 'green', decision: 'none', closed: false };
    this.t0 = Date.now();
  }
  set(o) {
    Object.assign(this.o, o);
    if (this.o.asked && !this.askAt) this.askAt = Date.now();
    if (this.o.lanes && !this.laneAt) this.laneAt = Date.now();
    return this;
  }

  // ---- the plan ---------------------------------------------------------------------------------------------------
  steps() {
    const o = this.o, it = (text, done) => ({ text, done: !!done, who: 'agent' });
    const done = (id, title, items) => ({ id: String(id), title, status: 'done', done: items.length, total: items.length, items: items.map((t) => it(t, true)), agents: [] });
    const out = [
      done(1, 'Scan a photo and find the colours', ['Person cutout', 'Colour engine', 'Result screen']),
      done(2, 'Save scans on the phone', ['Room database', 'History screen']),
      done(3, 'Store listing and signing', ['Release signing', 'Screenshots', 'Privacy text']),
    ];
    if (o.plan >= 1) {
      const merged = o.merged;
      const items4 = [it('Theme tokens for light and dark', merged), it('Follow the system theme', merged), it('Toggle in Settings', merged)];
      out.push({ id: '4', title: 'Dark mode', status: merged ? 'done' : o.lanes ? 'active' : 'ready', done: merged ? 3 : 0, total: 3, items: items4, agents: o.lanes && !merged ? ['wren'] : [] });
      const items5 = [it('Reuse the pose task between photos', false), it('Benchmark 200 photos on the phone', false)];
      out.push({ id: '5', title: 'Faster scan', status: o.lanes ? 'active' : 'ready', done: 0, total: 2, items: items5, agents: o.lanes ? ['ivren'] : [] });
    }
    return out;
  }
  project() {
    const p = JSON.parse(JSON.stringify(this.base.projects.find((x) => x.id === 'chromasol')));
    delete p.plans;
    Object.assign(p.setup, { gates_active: true, gates_problems: [], gates_fix: [], gates_summary: '', gates_actions: [], pending: [],
      protection: [{ agent: 'anthropic', ok: true, checked_at: iso(this.t0 - 60000) }, { agent: 'muse', ok: true, checked_at: iso(this.t0 - 60000) }] });
    for (const r of p.setup.rules) r.active = true;
    const steps = this.steps();
    p.roadmap = { file: '/Users/you/DEVELOP/ChromaSol/docs/PLAN.md', deadline: iso(this.t0 + 5 * 86400000 + 3600000 * 6), done: steps.reduce((n, s) => n + s.done, 0), total: steps.reduce((n, s) => n + s.total, 0), steps };
    p.counts = { agents_working: this.o.lanes ? 2 : 0, agents_total: this.o.lanes ? 2 : 0, pipelines_running: 0, flows_24h: 0 };
    return p;
  }

  // ---- agents ----------------------------------------------------------------------------------------------------
  agent(id, name, cls, feature, step, started, n, list, extra) {
    const tmpl = JSON.parse(JSON.stringify(this.base.agents.find((a) => a.id === 'w6-perf')));
    const now = Date.now(), laneFile = join(this.dir, id + '.steps.js');
    const done = !!(extra && extra.done);
    return Object.assign(tmpl, {
      id, name, epithet: extra.epithet, class: cls, rank: 'recruit', renown: 20, feature, goal: extra.goal, step, project: 'chromasol', parent: 'manager-s1',
      status: done ? 'done' : 'working', started_at: iso(started), ended_at: done ? iso(now - 4000) : null, elapsed_s: Math.round(((done ? now - 4000 : now) - started) / 1000),
      summary: extra.summary, log_tail: list.slice(0, n).filter((s) => s.kind !== 'text').map((s) => s.summary).slice(-3),
      files_changed: extra.files, insertions: extra.ins, deletions: extra.del, files: [], tools: [{ name: 'Bash', count: 4 }, { name: 'Edit', count: 2 }, { name: 'Read', count: 2 }],
      tests: n >= 7 && id === 'wren' ? [{ at: iso(now - 9000), command: 'tirith run --project chromasol --stage test', result: 'passed', failed: 0 }] : [], events: [], log_entries: [],
      session: 'chromasol-today', request_id: 'in-demo-1', turn_requests: [], active: !done, last_activity_at: iso(now - 1500), report: extra.report || null, review: extra.review,
      live: { steps: n, done, error: false, cost_usd: null, duration_s: done ? 83 : null, elapsed_s: Math.round((now - started) / 1000), detail: { path: laneFile, hash: id + n + done } },
    });
  }
  agents() {
    const o = this.o, t0 = this.t0, out = [];
    if (!o.lanes) return out;
    const merged = o.merged;
    out.push(this.agent('wren', 'Wren', 'glasswright', 'Dark mode', '4', this.laneAt - 6000, o.wren, WREN, {
      epithet: 'of the Quiet Palette', goal: 'Give ChromaSol a dark theme that follows the system and can be switched in Settings.',
      summary: merged ? 'Dark theme tokens and the Settings toggle are in; 86 tests pass.' : o.wren >= 4 ? 'The raw test run was blocked; it now runs the tests through Tirith.' : 'Splitting the colours into light and dark tokens in Theme.kt.',
      files: 4, ins: 118, del: 31, done: merged, report: merged ? 'Done: dark theme tokens, system follow, Settings toggle. 4 files, 4 tests added.' : null,
      review: merged ? { status: 'merged', at: iso(Date.now() - 3000), files: 4, passed: 86, run: 'chromasol#8', claims: { ok: true, mismatches: [] } } : undefined,
    }));
    out.push(this.agent('ivren', 'Ivren', 'swiftrider', 'Faster scan', '5', this.laneAt - 5000, o.ivren, IVREN, {
      epithet: 'of the Seventh Gate', goal: 'Reuse the pose task between photos so a 200-photo scan is fast on the phone.',
      summary: 'Moving the pose landmarker out of the per-photo loop so it is built once.', files: 2, ins: 41, del: 12,
    }));
    return out;
  }
  stepsFile(id, list, n) {
    const t = Date.now();
    const steps = list.slice(0, n).map((s, i) => Object.assign({ name: '', at: iso(t - (n - i) * 3000) }, s));
    if (steps.length && this.partial) steps[steps.length - 1].partial = steps[steps.length - 1].kind === 'text';
    put(join(this.dir, id + '.steps.js'), 'window.TIRITH_AGENT = ' + JSON.stringify({ id, steps }) + ';\n');
  }

  // ---- the whole state --------------------------------------------------------------------------------------------
  state() {
    const o = this.o, now = Date.now(), b = JSON.parse(JSON.stringify(this.base));
    const s = { version: 1, generated_at: iso(now), brain: b.brain, models: b.models, rules: b.rules, autopilot: { enabled: true, max_parallel: 4, started: [], paused_steps: [] }, launch: { token: 'demo-launch-token', available: true } };
    s.projects = [this.project()];
    s.sessions = [{ id: 'chromasol-today', title: 'ChromaSol today', started_at: iso(this.t0 - 3600000), ended_at: null, projects: ['chromasol'], counts: { agents: 2, pipelines: 1, flows: 0, notes: 0 } }];
    s.agents = this.agents();
    const old = (n, hrs, ok) => ({ run: 'chromasol#' + n, project: 'chromasol', pipeline: 'ci', status: ok ? 'passed' : 'failed', started_at: iso(now - hrs * 3600000), ended_at: iso(now - hrs * 3600000 + 52000), session: 'chromasol-today', stages: [
      { name: 'build', status: 'passed', started_at: iso(now - hrs * 3600000), ended_at: iso(now - hrs * 3600000 + 15000), duration_s: 15, log_tail: ['BUILD SUCCESSFUL in 15s'] },
      { name: 'test', status: ok ? 'passed' : 'failed', started_at: iso(now - hrs * 3600000 + 15000), ended_at: iso(now - hrs * 3600000 + 52000), duration_s: 37, log_tail: [ok ? 'tests 82 failures 0' : 'tests 82 failures 2'] }] });
    s.pipelines = [old(7, 3.2, true), old(6, 5.5, false), old(5, 8, true)];
    if (o.wren >= 7) {
      const t = (a) => iso(now - a * 1000);
      s.pipelines.push({ run: 'chromasol#8', project: 'chromasol', pipeline: 'ci', status: 'passed', started_at: t(60), ended_at: t(14), session: 'chromasol-today', stages: [
        { name: 'build', status: 'passed', started_at: t(60), ended_at: t(46), duration_s: 14, log_tail: ['BUILD SUCCESSFUL in 14s'] },
        { name: 'test', status: 'passed', started_at: t(46), ended_at: t(14), duration_s: 32, log_tail: ['tests 86 failures 0'] }] });
    }
    s.violations = o.violation ? [{ ts: iso(now - 6000), rule: 'raw-stage', agent: 'anthropic', lane: 'wren', project: 'chromasol', message: RAW, blocked: 'running the tests raw is not allowed', instead: 'tirith run --project chromasol --stage test' }] : [];
    s.rooms = []; s.flows = [];
    // the chat: the owner's request, the manager's live turn, then its answer
    const reqAt = this.askAt || now;
    const inbox = [];
    if (o.asked) inbox.push({ id: 'in-demo-1', at: iso(reqAt), kind: 'chat', project: 'chromasol', step: '', text: REQUEST, status: o.closed ? 'answered' : 'open', manager_pending: !o.closed, answers: [], lanes: [] });
    s.inbox = inbox;
    s.says = o.closed ? [{ id: 'say-1', at: iso(now - 2000), project: 'chromasol', text: 'Shipping dark mode in **v1.2**. The release build is queued; I will tell you when it is in the store track.', from: 'manager' }] : [];
    s.manager = { name: 'Aldric of the Tower', title: 'The Manager', activity: '', notes: [], presence: { mode: 'on_call', working: o.asked && !o.closed, lane: 'manager-s1', session: { lane: 'manager-s1', busy: o.asked && !o.closed, turns: 3, context_tokens: 38000, asks: 0 } }, turns: [] };
    if (o.asked && o.mgr > 0) {
      const lanes = o.lanes ? ['wren', 'ivren'] : [];
      const finished = o.closed || o.decision !== 'none';
      s.manager.turns = [{ id: 'manager-turn-aa11', name: 'Manager', status: finished ? 'done' : 'working', started_at: iso(reqAt + 1000), ended_at: finished ? iso(now - 3000) : null, elapsed_s: Math.round((now - reqAt) / 1000), parent: 'manager', turn_requests: ['in-demo-1'], lanes,
        live: { steps: o.mgr, done: finished, error: false, cost_usd: finished ? 0.21 : null, duration_s: finished ? 48 : null, elapsed_s: Math.round((now - reqAt) / 1000), detail: { path: join(this.dir, 'manager-turn-aa11.steps.js'), hash: 'm' + o.mgr } } }];
    }
    // the one real decision
    s.waiting_on_you = []; s.answered_waits = [];
    if (o.decision === 'open') s.waiting_on_you.push({ id: 'w-dark-1', project: 'chromasol', text: DECISION, since: iso(now - 4000), link: null, source: 'brain', session: 'chromasol-today', kind: 'decision', status: 'open', command: null,
      options: [{ label: 'Ship in v1.2', value: 'ship' }, { label: 'Hold for 2.0', value: 'hold' }] });
    s.manager_queue = [];
    if (o.health === 'amber') s.health = { level: 'amber', at: iso(now), checks: [
      { id: 'collector', label: 'Tirith itself', level: 'healing', text: 'New code landed on main. Tirith is reloading the collector.' },
      { id: 'manager', label: 'The manager', level: 'ok', text: 'The manager is answering normally.' },
      { id: 'autopilot', label: 'The autopilot', level: 'ok', text: 'The autopilot is running.' }] };
    else s.health = { level: 'green', at: iso(now), checks: [
      { id: 'collector', label: 'Tirith itself', level: 'ok', text: 'Tirith is collecting normally.' },
      { id: 'manager', label: 'The manager', level: 'ok', text: 'The manager is answering normally.' },
      { id: 'autopilot', label: 'The autopilot', level: 'ok', text: 'The autopilot is running.' },
      { id: 'disk', label: 'Disk', level: 'ok', text: '41 GB free.' }] };
    return s;
  }
  write() {
    this.partial = this.o.mgr > 0 && this.o.mgr < MANAGER.length;
    this.stepsFile('manager-turn-aa11', MANAGER, this.o.mgr);
    this.partial = false;
    this.stepsFile('wren', WREN, this.o.wren);
    this.stepsFile('ivren', IVREN, this.o.ivren);
    put(join(this.dir, 'state.js'), 'window.TIRITH_STATE = ' + JSON.stringify(this.state()) + ';');
  }
}

// node story.mjs --static  : web/demo/state.video.js, the last beat, for opening the page without the recorder
if (process.argv[2] === '--static') {
  const dir = join(root, 'web/demo'), s = new Story(dir);
  s.set({ asked: true, mgr: MANAGER.length, plan: 1, wren: WREN.length, ivren: IVREN.length, lanes: true, violation: true, merged: true, decision: 'open' });
  const st = s.state();
  for (const a of st.agents) a.live = null;   // the steps files live next to the recorder's state, not in the repo
  st.manager.turns[0].live = null;
  writeFileSync(join(dir, 'state.video.js'), 'window.TIRITH_STATE = ' + JSON.stringify(st, null, 1) + ';\n');
  console.log('wrote web/demo/state.video.js');
}

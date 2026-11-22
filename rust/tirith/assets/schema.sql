PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=5000;

-- Every lane, chat (turn) row, owner card, event and score carries its factory
-- stamp: `factory` is the factory version that made it (the main commit that
-- last changed factory/, plus a short hash of config.toml's overlay) and
-- `role_hash` a short hash of the setting of the role it ran with, so two
-- versions, models or prompts compare with one query (GROUP BY factory, role_hash).
CREATE TABLE IF NOT EXISTS lanes (id TEXT PRIMARY KEY, data TEXT NOT NULL,
    factory TEXT NOT NULL DEFAULT '', role_hash TEXT NOT NULL DEFAULT '');
CREATE TABLE IF NOT EXISTS events (id INTEGER PRIMARY KEY, at TEXT, lane TEXT, project TEXT,
    kind TEXT, tool TEXT, summary TEXT, session TEXT,
    factory TEXT NOT NULL DEFAULT '', role_hash TEXT NOT NULL DEFAULT '');
CREATE TABLE IF NOT EXISTS chat (id INTEGER PRIMARY KEY, at TEXT, role TEXT, text TEXT,
    factory TEXT NOT NULL DEFAULT '', role_hash TEXT NOT NULL DEFAULT '');
CREATE TABLE IF NOT EXISTS runs (id INTEGER PRIMARY KEY, lane TEXT, project TEXT, stage TEXT,
    status TEXT, started_at TEXT, ended_at TEXT, log TEXT);
CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value TEXT);
CREATE TABLE IF NOT EXISTS pipelines (id TEXT PRIMARY KEY, project TEXT, kind TEXT, lanes TEXT, sha TEXT,
    status TEXT, started_at TEXT, ended_at TEXT);
CREATE TABLE IF NOT EXISTS owner_items (id TEXT PRIMARY KEY, at TEXT, kind TEXT, question TEXT,
    status TEXT, answer TEXT, factory TEXT NOT NULL DEFAULT '', role_hash TEXT NOT NULL DEFAULT '');
CREATE TABLE IF NOT EXISTS scores (id INTEGER PRIMARY KEY, at TEXT, event INTEGER, eval TEXT,
    label TEXT, score REAL, why TEXT, factory TEXT NOT NULL DEFAULT '', role_hash TEXT NOT NULL DEFAULT '');
CREATE TABLE IF NOT EXISTS am_rooms (id TEXT PRIMARY KEY, data TEXT NOT NULL);
-- One row per pass of a turn role (the CEO, the Coach): its measures or trigger, its answer, its stamp and its role.
CREATE TABLE IF NOT EXISTS ceo_reviews (id INTEGER PRIMARY KEY, at TEXT, metric TEXT, text TEXT,
    factory TEXT NOT NULL DEFAULT '', role_hash TEXT NOT NULL DEFAULT '', role TEXT NOT NULL DEFAULT '');
CREATE TABLE IF NOT EXISTS net_blocks (rule TEXT PRIMARY KEY, at TEXT);
-- One row per turn of a chat or turn role (the project managers, the bots, the CEO, the judge; `runs` is the
-- pipelines'): its conversation, what started it (`trigger`: owner, room, schedule, run-now or event:<name>, and the
-- routine), its time, status, agent, model and effort, its tokens and cost, its tool calls (`steps`), the skills and
-- tools it used and the versions (git blob hashes) of its memory and skills (JSON), its answer, its event and its stamp;
-- its quality signals: the owner's reaction (1, -1), corrected (1 when owner-mood labelled the owner's next message a
-- correction or repeat), its own checks (passed, total, names), the numbered notes its answer cites (JSON) and, for
-- a routine's turn, the judge's turn-quality score of its routine.done (0 to 1); a run of a live experiment keeps
-- its `experiment` and its `variant` (A, the bot's main, or B, the branch).
CREATE TABLE IF NOT EXISTS turns (id INTEGER PRIMARY KEY, conv TEXT, role TEXT, "trigger" TEXT, routine TEXT,
    started_at TEXT, ended_at TEXT, ms INTEGER, status TEXT, agent TEXT, model TEXT, effort TEXT,
    tokens_in INTEGER, tokens_out INTEGER, tokens_cache_read INTEGER, tokens_cache_write INTEGER, cost_usd REAL,
    steps INTEGER, skills TEXT, tools TEXT, versions TEXT, answer TEXT, event INTEGER,
    factory TEXT NOT NULL DEFAULT '', role_hash TEXT NOT NULL DEFAULT '',
    reaction INTEGER, corrected INTEGER NOT NULL DEFAULT 0, checks_passed INTEGER, checks_total INTEGER,
    checks TEXT, cited TEXT, score REAL, experiment INTEGER, variant TEXT, why TEXT);
CREATE INDEX IF NOT EXISTS turns_conv ON turns (conv, id);
-- What was due (docs/DAEMON.md, `expected/`): one row for each occurrence of a routine or scheduled pass, written
-- before anything starts: when it was due (UTC), its task, waiting | running | ok | failed, why, and its turn.
CREATE TABLE IF NOT EXISTS expected_runs (id INTEGER PRIMARY KEY, conv TEXT, routine TEXT, title TEXT, "trigger" TEXT,
    kind TEXT, due TEXT, task TEXT, state TEXT, why TEXT, turn INTEGER, created_at TEXT, started_at TEXT, ended_at TEXT);
CREATE INDEX IF NOT EXISTS expected_runs_of ON expected_runs (conv, routine, due);
CREATE INDEX IF NOT EXISTS expected_runs_open ON expected_runs (state);
-- A turn from its start to its row in `turns`: what a restart finds here was interrupted and is kept as failed.
CREATE TABLE IF NOT EXISTS open_turns (conv TEXT, role TEXT, start_us INTEGER, "trigger" TEXT, routine TEXT,
    started_at TEXT, agent TEXT, model TEXT, effort TEXT, PRIMARY KEY (conv, role, start_us));
-- The lab (docs/BOTS.md, part 3): one row per experiment of the Coach on a bot's routine — the bot's key, the
-- routine, the branch of the bot's repository and its one commit (`change`), the hypothesis, the mode, its status
-- (running, adopted, discarded), its times, the numbers it was decided with (JSON) and the factory that decided it;
-- and one row per comparison of a B run with the A run before it: the letter B was shown under to the judge (`b_is`,
-- a or b, drawn at random), the event the judge read and B's verdict (win, lose or tie) with the judge's line.
CREATE TABLE IF NOT EXISTS experiments (id INTEGER PRIMARY KEY, bot TEXT, routine TEXT, branch TEXT, change TEXT,
    hypothesis TEXT, mode TEXT, status TEXT, created_at TEXT, ended_at TEXT, numbers TEXT,
    factory TEXT NOT NULL DEFAULT '');
CREATE TABLE IF NOT EXISTS lab_comparisons (id INTEGER PRIMARY KEY, experiment INTEGER, a_turn INTEGER,
    b_turn INTEGER, event INTEGER, b_is TEXT, verdict TEXT, why TEXT);

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
-- a routine's turn, the judge's turn-quality score of its routine.done (0 to 1).
CREATE TABLE IF NOT EXISTS turns (id INTEGER PRIMARY KEY, conv TEXT, role TEXT, "trigger" TEXT, routine TEXT,
    started_at TEXT, ended_at TEXT, ms INTEGER, status TEXT, agent TEXT, model TEXT, effort TEXT,
    tokens_in INTEGER, tokens_out INTEGER, tokens_cache_read INTEGER, tokens_cache_write INTEGER, cost_usd REAL,
    steps INTEGER, skills TEXT, tools TEXT, versions TEXT, answer TEXT, event INTEGER,
    factory TEXT NOT NULL DEFAULT '', role_hash TEXT NOT NULL DEFAULT '',
    reaction INTEGER, corrected INTEGER NOT NULL DEFAULT 0, checks_passed INTEGER, checks_total INTEGER,
    checks TEXT, cited TEXT, score REAL);
CREATE INDEX IF NOT EXISTS turns_conv ON turns (conv, id);

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

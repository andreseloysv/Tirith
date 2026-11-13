PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=5000;

CREATE TABLE IF NOT EXISTS lanes (id TEXT PRIMARY KEY, data TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS events (id INTEGER PRIMARY KEY, at TEXT, lane TEXT, project TEXT,
    kind TEXT, tool TEXT, summary TEXT, session TEXT);
CREATE TABLE IF NOT EXISTS chat (id INTEGER PRIMARY KEY, at TEXT, role TEXT, text TEXT);
CREATE TABLE IF NOT EXISTS runs (id INTEGER PRIMARY KEY, lane TEXT, project TEXT, stage TEXT,
    status TEXT, started_at TEXT, ended_at TEXT, log TEXT);
CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value TEXT);
CREATE TABLE IF NOT EXISTS pipelines (id TEXT PRIMARY KEY, project TEXT, kind TEXT, lanes TEXT, sha TEXT,
    status TEXT, started_at TEXT, ended_at TEXT);
CREATE TABLE IF NOT EXISTS owner_items (id TEXT PRIMARY KEY, at TEXT, kind TEXT, question TEXT,
    status TEXT, answer TEXT);

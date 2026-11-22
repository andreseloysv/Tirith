//! Opening the shared state at a Tirith home: its folders, its database and the lanes it kept.
use super::{migrate::migrate, state::Seen, App, Lane, Shared};
use crate::config::Config;
use rusqlite::Connection;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::{broadcast, Notify};

impl App {
    pub fn open_at(home: PathBuf) -> Shared {
        make_folders(&home);
        let db = open_db(&home);
        let lanes = load_lanes(&db);
        let cfg_mtime = std::fs::metadata(home.join("config.toml"))
            .and_then(|m| m.modified())
            .ok();
        let cfg = Arc::new(Config::load_at(&home));
        let cfg_mtime = (cfg_mtime, crate::factory::stamp(&cfg.raw));
        Arc::new(App {
            home,
            started: super::now_s(),
            cfg: RwLock::new(cfg.clone()),
            db: Mutex::new(db),
            lanes: Mutex::new(lanes),
            tx: broadcast::channel(1024).0,
            plan_dirty: Mutex::new(cfg.projects.iter().map(|p| p.id.clone()).collect()), // every project starts dirty: the first flush commits a plan a stopped daemon left uncommitted (flush skips a clean one)
            plan_lock: tokio::sync::Mutex::new(()),
            sched_wake: Notify::new(),
            merge_wake: Notify::new(),
            children: Mutex::new(HashSet::new()),
            seen: Seen {
                manifests: Mutex::new(HashMap::new()),
                cfg_mtime: Mutex::new(cfg_mtime),
            },
        })
    }
}

fn make_folders(home: &Path) {
    ["logs/lanes", "logs/runs", "briefs", "lanes"]
        .into_iter()
        .for_each(|dir| drop(std::fs::create_dir_all(home.join(dir))));
}

/// `tirith.sqlite3` with every table and column it needs, older names moved.
fn open_db(home: &Path) -> Connection {
    let db = Connection::open(home.join("tirith.sqlite3")).expect("open tirith.sqlite3");
    db.execute_batch(include_str!("../../assets/schema.sql"))
        .expect("create tables");
    add_later_columns(&db);
    crate::turns::add_columns(&db);
    migrate(&db);
    add_stamp_columns(&db);
    db
}

/// Columns added later (each ALTER fails once its column exists). A chat row and an owner card belong to a project; '' is none. A card may carry a waiting lane's question.
fn add_later_columns(db: &Connection) {
    [
        ("runs", "pipeline TEXT"),
        ("runs", "sha TEXT"),
        ("chat", "project TEXT NOT NULL DEFAULT ''"),
        ("owner_items", "project TEXT NOT NULL DEFAULT ''"),
        ("owner_items", "lane TEXT NOT NULL DEFAULT ''"),
        ("ceo_reviews", "role TEXT NOT NULL DEFAULT ''"),
    ]
    .into_iter()
    .for_each(|(table, col)| {
        drop(db.execute(&format!("ALTER TABLE {table} ADD COLUMN {col}"), []))
    });
}

/// Every stamped table carries its factory version and role hash.
fn add_stamp_columns(db: &Connection) {
    ["lanes", "events", "chat", "owner_items", "ceo_reviews"]
        .into_iter()
        .for_each(|t| {
            ["factory", "role_hash"].into_iter().for_each(|c| {
                drop(db.execute(
                    &format!("ALTER TABLE {t} ADD COLUMN {c} TEXT NOT NULL DEFAULT ''"),
                    [],
                ))
            })
        });
}

fn load_lanes(db: &Connection) -> BTreeMap<String, Lane> {
    db.prepare("SELECT data FROM lanes")
        .expect("read lanes")
        .query_map([], |r| r.get::<_, String>(0))
        .expect("read lanes")
        .flatten()
        .filter_map(|s| serde_json::from_str::<Lane>(&s).ok())
        .map(|l| (l.id.clone(), l))
        .collect()
}

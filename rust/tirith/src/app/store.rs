//! The store's writes: a row, a key's value, and keeping the event table bounded.
use super::App;
use rusqlite::params;

impl App {
    pub fn insert(&self, sql: &str, values: &[&dyn rusqlite::ToSql]) -> i64 {
        let db = self.db.lock().unwrap();
        let _ = db.execute(sql, values);
        db.last_insert_rowid()
    }

    pub fn kv(&self, key: &str) -> Option<String> {
        self.db
            .lock()
            .unwrap()
            .query_row("SELECT value FROM kv WHERE key = ?1", [key], |r| r.get(0))
            .ok()
    }

    pub fn set_kv(&self, key: &str, value: Option<&str>) {
        let db = self.db.lock().unwrap();
        let _ = value.map_or_else(
            || db.execute("DELETE FROM kv WHERE key = ?1", [key]),
            |v| {
                db.execute(
                    "INSERT OR REPLACE INTO kv (key, value) VALUES (?1, ?2)",
                    params![key, v],
                )
            },
        );
    }

    /// Keeps the event table bounded: the newest 50 000 rows.
    pub fn trim(&self) {
        let _ = self.db.lock().unwrap().execute(
            "DELETE FROM events WHERE id < (SELECT COALESCE(MAX(id), 0) - 50000 FROM events)",
            [],
        );
    }
}

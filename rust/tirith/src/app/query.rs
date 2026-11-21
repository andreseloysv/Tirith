//! The store's reads: rows mapped by a function, or as JSON objects.
use super::App;
use serde_json::{json, Value};

impl App {
    /// The rows of a query, each mapped by `f`; a query that fails gives none.
    pub fn query<T>(
        &self,
        sql: &str,
        args: impl rusqlite::Params,
        f: impl FnMut(&rusqlite::Row) -> rusqlite::Result<T>,
    ) -> Vec<T> {
        let db = self.db.lock().unwrap();
        let Ok(mut q) = db.prepare(sql) else {
            return vec![];
        };
        q.query_map(args, f)
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default()
    }

    /// The rows of a query as JSON objects named by `cols` (numbers and text; anything else is null).
    pub fn objects(&self, sql: &str, args: impl rusqlite::Params, cols: &[&str]) -> Vec<Value> {
        self.query(sql, args, |r| {
            cols.iter()
                .enumerate()
                .map(|pair| cell(pair, r))
                .collect::<rusqlite::Result<serde_json::Map<String, Value>>>()
                .map(Value::Object)
        })
    }

    /// The last `limit` rows of a table (with `project`, only that project's) as JSON objects, oldest first.
    pub fn recent(
        &self,
        table: &str,
        cols: &[&str],
        project: Option<&str>,
        limit: u32,
    ) -> Vec<Value> {
        let only = if project.is_some() {
            "WHERE project = ?1"
        } else {
            ""
        };
        self.objects(&format!("SELECT {} FROM (SELECT {} FROM {table} {only} ORDER BY id DESC LIMIT {limit}) ORDER BY id", cols.join(", "), cols.join(", ")), rusqlite::params_from_iter(project), cols)
    }
}

/// Column `i` of a row, named `c`: a number, a text, or null.
fn cell((i, c): (usize, &&str), r: &rusqlite::Row) -> rusqlite::Result<(String, Value)> {
    r.get_ref(i).map(|v| {
        (
            c.to_string(),
            v.as_i64()
                .map(|n| json!(n))
                .or_else(|_| v.as_str().map(|s| json!(s)))
                .unwrap_or(Value::Null),
        )
    })
}

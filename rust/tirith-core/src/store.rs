//! Reading and writing the small input files under TIRITH_HOME (`store.py`).
//!
//! Writes go through a temp file in the same folder and a rename, so a reader never sees half a file.

use serde_json::Value;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Read a JSON file; `None` when it is missing or broken.
pub fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

/// Write text via a temp file next to `path`, then rename it into place.
pub fn write_text_atomic(path: &Path, text: &str) -> io::Result<()> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp: PathBuf = parent.join(format!(
        "{name}.{}-{}.tmp",
        std::process::id(),
        TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Write JSON atomically; `compact` leaves out the indentation. The file ends with a newline.
pub fn write_json_atomic(path: &Path, payload: &Value, compact: bool) -> io::Result<()> {
    let mut text = if compact {
        serde_json::to_string(payload)
    } else {
        serde_json::to_string_pretty(payload)
    }
    .map_err(io::Error::other)?;
    text.push('\n');
    write_text_atomic(path, &text)
}

/// Append one JSON object as a line, creating the file (and its folder) if needed.
pub fn append_jsonl(path: &Path, record: &Value) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new().create(true).append(true).open(path)?;
    let mut line = serde_json::to_string(record).map_err(io::Error::other)?;
    line.push('\n');
    file.write_all(line.as_bytes())
}

/// Read a JSON-lines file: broken lines and non-objects are skipped, a missing file means no rows.
pub fn read_jsonl(path: &Path) -> Vec<Value> {
    let Ok(bytes) = fs::read(path) else { return Vec::new() };
    String::from_utf8_lossy(&bytes)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(Value::is_object)
        .collect()
}

/// All lane registrations (`<home>/lanes/*.json` with an `id`), sorted by start time then id.
pub fn read_lanes(home: &Path) -> Vec<Value> {
    let Ok(dir) = fs::read_dir(home.join("lanes")) else { return Vec::new() };
    let mut paths: Vec<PathBuf> = dir
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    let key = |lane: &Value, field: &str| lane.get(field).and_then(Value::as_str).unwrap_or("").to_string();
    let mut lanes: Vec<Value> = paths
        .iter()
        .filter_map(|p| read_json(p))
        .filter(|l| l.get("id").is_some_and(|id| id.as_str().map_or(!id.is_null(), |s| !s.is_empty())))
        .collect();
    lanes.sort_by_key(|l| (key(l, "started_at"), key(l, "id")));
    lanes
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn atomic_json_round_trip_leaves_no_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/x.json");
        write_json_atomic(&path, &json!({"a": [1, 2]}), false).unwrap();
        assert_eq!(read_json(&path), Some(json!({"a": [1, 2]})));
        assert!(fs::read_to_string(&path).unwrap().ends_with("}\n"));
        write_json_atomic(&path, &json!({"b": 1}), true).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"b\":1}\n");
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }

    #[test]
    fn broken_or_missing_json_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_json(&dir.path().join("nope.json")), None);
        fs::write(dir.path().join("bad.json"), "{oops").unwrap();
        assert_eq!(read_json(&dir.path().join("bad.json")), None);
    }

    #[test]
    fn jsonl_skips_broken_lines_and_non_objects() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("e.jsonl");
        append_jsonl(&path, &json!({"n": 1})).unwrap();
        fs::OpenOptions::new().append(true).open(&path).unwrap().write_all(b"junk\n[1]\n\n").unwrap();
        append_jsonl(&path, &json!({"n": 2})).unwrap();
        assert_eq!(read_jsonl(&path), vec![json!({"n": 1}), json!({"n": 2})]);
        assert!(read_jsonl(&dir.path().join("none.jsonl")).is_empty());
    }

    #[test]
    fn lanes_sort_by_start_then_id() {
        let dir = tempfile::tempdir().unwrap();
        let lanes = dir.path().join("lanes");
        fs::create_dir_all(&lanes).unwrap();
        for (id, at) in [("b", "2026-01-02"), ("a", "2026-01-02"), ("c", "2026-01-01")] {
            fs::write(lanes.join(format!("{id}.json")), json!({"id": id, "started_at": at}).to_string()).unwrap();
        }
        fs::write(lanes.join("noid.json"), "{}").unwrap();
        let ids: Vec<_> = read_lanes(dir.path()).iter().map(|l| l["id"].as_str().unwrap().to_string()).collect();
        assert_eq!(ids, ["c", "a", "b"]);
    }
}

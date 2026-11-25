//! A routine's own effort (and model): `routines.toml` in the bot's folder, a tracked file with a table per routine id (`[r1]`, `effort = "medium"`, optionally `model`). It holds for that routine's turns only, and a branch of the bot's repository can change it, so an experiment can test "this routine with medium effort instead of high".
use super::folder::folder;
use crate::app::Shared;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

/// The file, in the bot's folder.
pub const FILE: &str = "routines.toml";
/// (effort, model) a routine sets.
pub(super) type Tuning = (Option<String>, Option<String>);

/// The bots (by key) whose routine's run is on its way with a tuning.
static ACTIVE: Mutex<BTreeMap<String, Tuning>> = Mutex::new(BTreeMap::new());

/// The tuning of routine `routine` in the folder `dir`: none when the file, the table or both keys are missing.
pub fn read(dir: &Path, routine: &str) -> Option<Tuning> {
    let text = std::fs::read_to_string(dir.join(FILE)).ok()?;
    let table = text.parse::<toml::Table>().ok()?;
    let one = table.get(routine)?.as_table()?;
    let word = |k: &str| one.get(k)?.as_str().map(str::to_string);
    let tuning = (word("effort"), word("model"));
    (tuning.0.is_some() || tuning.1.is_some()).then_some(tuning)
}

/// Sets the tuning of routine `routine` in the folder `dir` (neither: its table goes); whether the file changed. The file's other tables and comments stay as they are.
pub(super) fn write(dir: &Path, routine: &str, tuning: &Tuning) -> Result<bool, String> {
    let path = dir.join(FILE);
    let was = std::fs::read_to_string(&path).unwrap_or_default();
    let parsed = was.parse::<toml_edit::DocumentMut>();
    let mut doc = parsed.map_err(|e| format!("{FILE}: {e}"))?;
    doc.remove(routine);
    let words = [("effort", &tuning.0), ("model", &tuning.1)];
    let mut table = toml_edit::Table::new();
    for (key, word) in words
        .into_iter()
        .filter_map(|(k, w)| Some((k, w.as_ref()?)))
    {
        table.insert(key, toml_edit::value(word.as_str()));
    }
    if !table.is_empty() {
        doc.insert(routine, toml_edit::Item::Table(table));
    }
    let now = doc.to_string();
    if now == was {
        return Ok(false);
    }
    let kept = std::fs::create_dir_all(dir).and_then(|_| std::fs::write(&path, now));
    kept.map(|_| true)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// A run of routine `routine` of bot `key` starts: its tuning (from the bot's folder as it is for this run, a B run's the branch's) applies to its turns, and a live session ends, so a fresh one starts with it.
pub fn tune(app: &Shared, key: &str, routine: &str) {
    let tuning = read(&folder(app, key), routine);
    let had = {
        let mut active = ACTIVE.lock().unwrap();
        let had = active.remove(key).is_some();
        tuning
            .iter()
            .for_each(|t| drop(active.insert(key.into(), t.clone())));
        had
    };
    let _ = (had || tuning.is_some()).then(|| crate::manager::end_session(key));
}

/// The run of bot `key` ended: its tuning goes, and its session ends, so the next turn starts a fresh one without it.
pub fn untune(_app: &Shared, key: &str) {
    let had = ACTIVE.lock().unwrap().remove(key).is_some();
    let _ = had.then(|| crate::manager::end_session(key));
}

/// The effort and model a turn of `role` runs with: the routine's own when its run is on its way (inside a replay or a shadow run, its experiment's routine, read from its worktree), else as given.
pub fn tuned(role: &str, effort: &mut Option<String>, model: &mut Option<String>) {
    let scoped = crate::lab::scope().and_then(|(dir, routine)| read(&dir, &routine));
    let active = ACTIVE.lock().unwrap().get(role).cloned();
    if let Some((e, m)) = scoped.or(active) {
        *effort = e.or(effort.take());
        *model = m.or(model.take());
    }
}

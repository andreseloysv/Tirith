//! Whether a Codex will hand its commands and patches to the process that has their limit. One that would not is never started under a limit: they would run in the process that holds its state.
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::sync::Mutex;
use std::time::SystemTime;

/// By which Codex is told where they run; what it is told there it obeys, or runs none at all.
pub(super) const URL: &str = "CODEX_EXEC_SERVER_URL";
/// A file that Codex reads before that word.
const CHOOSES: &str = "environments.toml";

type Built = (String, Option<SystemTime>, u64);
static KNOWN: Mutex<BTreeMap<Built, bool>> = Mutex::new(BTreeMap::new());

pub(super) fn capable(program: &str, state: &str) -> Result<(), String> {
    if !state.is_empty() && Path::new(state).join(CHOOSES).exists() {
        return Err(format!("{state}/{CHOOSES} chooses where its commands run, so no limit holds for them: move it away"));
    }
    let meta = std::fs::metadata(program).map_err(|e| format!("{program}: {e}"))?;
    let built = (program.to_string(), meta.modified().ok(), meta.len());
    let known = KNOWN
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&built)
        .copied();
    let obeys = known.unwrap_or_else(|| {
        let obeys = holds(program, URL.as_bytes());
        KNOWN
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(built, obeys);
        obeys
    });
    obeys.then_some(()).ok_or_else(|| {
        "this Codex cannot run its commands apart from its sign-in and state: update Codex".into()
    })
}

/// Whether `file` holds `word`.
fn holds(file: &str, word: &[u8]) -> bool {
    let Ok(mut file) = std::fs::File::open(file) else {
        return false;
    };
    let (mut kept, mut part) = (vec![], vec![0u8; 1 << 20]);
    while let Ok(got @ 1..) = file.read(&mut part) {
        kept.extend_from_slice(&part[..got]);
        let mut at = 0;
        while let Some(found) = kept[at..].iter().position(|b| *b == word[0]) {
            at += found;
            if kept[at..].starts_with(word) {
                return true;
            }
            at += 1;
        }
        let from = kept.len().saturating_sub(word.len() - 1);
        kept.drain(..from);
    }
    false
}

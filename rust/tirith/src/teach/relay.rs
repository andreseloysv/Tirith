//! The relay between a bot's `tirith teach do` and the extension: one command at a time per bot, held until the extension asks for it, and its answer handed back to the waiting command.
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::oneshot;
struct Pending {
    id: u64,
    cmd: Value,
    handed: bool,
    tx: oneshot::Sender<Value>,
}
static PENDING: Mutex<Option<HashMap<String, Pending>>> = Mutex::new(None);
fn with<T>(f: impl FnOnce(&mut HashMap<String, Pending>) -> T) -> T {
    let mut held = PENDING.lock().unwrap_or_else(|e| e.into_inner());
    f(held.get_or_insert_with(HashMap::new))
}
/// Hands `cmd` to the extension and waits up to `wait` for its answer.
pub(super) async fn ask(key: &str, mut cmd: Value, wait: Duration) -> Result<Value, String> {
    let id = rand::random::<u32>() as u64 + 1;
    cmd["id"] = json!(id);
    let (tx, rx) = oneshot::channel();
    let busy = with(|m| {
        let busy = m.contains_key(key);
        let p = Pending {
            id,
            cmd,
            handed: false,
            tx,
        };
        let _ = (!busy).then(|| m.insert(key.to_string(), p));
        busy
    });
    if busy {
        return Err("another step of this rehearsal is still running: wait for it".into());
    }
    let got = tokio::time::timeout(wait, rx).await;
    with(|m| {
        let _ = m
            .get(key)
            .is_some_and(|p| p.id == id)
            .then(|| m.remove(key));
    });
    match got {
        Ok(Ok(answer)) => Ok(answer),
        _ => Err("the extension did not answer: is the owner's Edge open, with Tirith's extension and this bot's link in it?".into()),
    }
}
/// The command waiting for the extension, if any, waiting up to `wait` for one (each is handed once).
pub(super) async fn next(key: &str, wait: Duration) -> Option<Value> {
    let until = tokio::time::Instant::now() + wait;
    loop {
        let cmd = with(|m| {
            let p = m.get_mut(key).filter(|p| !p.handed)?;
            p.handed = true;
            Some(p.cmd.clone())
        });
        if cmd.is_some() || tokio::time::Instant::now() >= until {
            return cmd;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
/// The extension's answer to command `id`; false when no command of that id waits.
pub(super) fn answer(key: &str, id: u64, answer: Value) -> bool {
    let p = with(|m| {
        let ours = m.get(key).is_some_and(|p| p.id == id);
        ours.then(|| m.remove(key)).flatten()
    });
    p.is_some_and(|p| p.tx.send(answer).is_ok())
}

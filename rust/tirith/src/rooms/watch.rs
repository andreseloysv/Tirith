//! One task per room listens to it, polling again and again, slower while the platform fails.
use super::{poll::poll, public, store::room, store::update, store::ROOMS};
use crate::app::Shared;
use std::collections::HashSet;
use std::sync::Mutex;
use std::time::{Duration, Instant};
/// The rooms that have their task.
static RUNNING: Mutex<Option<HashSet<String>>> = Mutex::new(None);
/// Starts the room's task, unless it runs already.
pub(super) fn watch(app: &Shared, id: &str) {
    let true = running(|ids| ids.insert(id.to_string())) else {
        return;
    };
    let (app, id) = (app.clone(), id.to_string());
    tokio::spawn(async move {
        listen(&app, &id).await;
        running(|ids| ids.remove(&id));
    });
}
fn running(f: impl FnOnce(&mut HashSet<String>) -> bool) -> bool {
    f(RUNNING.lock().unwrap().get_or_insert_with(HashSet::new))
}
async fn listen(app: &Shared, id: &str) {
    let mut backoff = 15;
    while let Some(r) = room(id) {
        if r.kind == "public" && !public::listed(app, id) {
            ROOMS.lock().unwrap().remove(id); // dropped from config.toml: keep the row, stop listening
            break;
        }
        let started = Instant::now();
        match poll(app, &r).await {
            Ok(()) => {
                backoff = 15;
                update(app, id, |r| r.error.clear());
                // A poll that came back at once was not held by the platform: do not spin on it.
                tokio::time::sleep(Duration::from_secs(
                    if started.elapsed() < Duration::from_secs(1) && r.cursor.is_some() {
                        2
                    } else {
                        0
                    },
                ))
                .await;
            }
            Err(e) => {
                update(app, id, |r| r.error = e);
                tokio::time::sleep(Duration::from_secs(backoff)).await;
                backoff = (backoff * 2).min(300);
            }
        }
    }
}

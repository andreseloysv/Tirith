//! The rooms' start, and the loop that keeps them as the config and the phones need them.
use super::{files::expire_files, outbox, provide::provide, public, store, turns::again};
use super::{store::ROOMS, watch::watch};
use crate::{app::Shared, manager};
use std::time::Duration;
use tokio::sync::Notify;
/// Wakes the loop that keeps the rooms as the config and the phones need them.
pub(super) static WAKE: Notify = Notify::const_new();
pub fn start(app: Shared) {
    store::load(&app);
    outbox::open(&app);
    super::unsent::again(&app);
    again(&app, None);
    tokio::spawn(keep(app));
}
async fn keep(app: Shared) {
    let (mut said, mut tick) = (String::new(), 0u32);
    loop {
        // At start and then every hour (120 rounds of at most 30 s) the attachments older than 24 h go.
        let _ = (tick % 120 == 0)
            .then(|| expire_files(&app.home.join("rooms/files"), Duration::from_secs(86400)));
        tick += 1;
        public::sync(&app);
        provide(&app, &mut said).await;
        manager::typing_heartbeats(); // about every 30 s: a busy turn's phones are shown it is still typing
        let ids: Vec<String> = ROOMS.lock().unwrap().keys().cloned().collect();
        ids.iter().for_each(|id| watch(&app, id));
        let _ = tokio::time::timeout(Duration::from_secs(30), WAKE.notified()).await;
    }
}

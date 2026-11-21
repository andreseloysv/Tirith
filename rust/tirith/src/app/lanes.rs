//! The lanes in memory, each change written to SQLite and shown on the page.
use super::{App, Lane};
use rusqlite::params;
use serde_json::json;

impl App {
    pub fn lane(&self, id: &str) -> Option<Lane> {
        self.lanes.lock().unwrap().get(id).cloned()
    }

    pub fn save_lane(&self, lane: &Lane) {
        let data = serde_json::to_string(lane).unwrap_or_default();
        self.insert("INSERT INTO lanes (id, data, factory, role_hash) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(id) DO UPDATE SET data = ?2, factory = ?3, role_hash = ?4", params![lane.id, data, lane.factory, lane.role_hash]);
        self.lanes
            .lock()
            .unwrap()
            .insert(lane.id.clone(), lane.clone());
        self.emit("lane", json!(lane));
    }

    /// Changes one lane in place (memory, SQLite, page) and returns the new value.
    pub fn update_lane(&self, id: &str, f: impl FnOnce(&mut Lane)) -> Option<Lane> {
        let mut lane = self.lane(id)?;
        f(&mut lane);
        self.save_lane(&lane);
        Some(lane)
    }
}

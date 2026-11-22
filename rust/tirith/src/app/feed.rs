//! The change feed the page listens to, and the event and chat rows it shows.
use super::{now_iso, App};
use rusqlite::params;
use serde_json::{json, Value};

impl App {
    pub fn emit(&self, t: &str, d: Value) {
        let _ = self.tx.send(json!({ "t": t, "d": d }).to_string());
    }

    /// One event row, shown on the page; its id, so an eval's score can keep it.
    pub fn event(
        &self,
        lane: &str,
        project: &str,
        kind: &str,
        tool: &str,
        summary: &str,
        session: &str,
    ) -> i64 {
        let at = now_iso();
        let summary = &crate::setup::Hidden::in_home(&self.home).hide(summary);
        let (factory, role_hash) = (
            self.cfg().factory.version.clone(),
            self.lane(lane).map(|l| l.role_hash).unwrap_or_default(),
        ); // a lane's event carries its lane's stamp
        let id = self.insert("INSERT INTO events (at, lane, project, kind, tool, summary, session, factory, role_hash) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)", params![at, lane, project, kind, tool, summary, session, factory, role_hash]);
        self.emit("event", json!({"id": id, "at": at, "lane": lane, "project": project, "kind": kind, "tool": tool, "summary": summary, "factory": factory, "role_hash": role_hash}));
        id
    }

    /// One row of a project's conversation with its manager (of no project: the manager-general bot's). A row a role wrote while working (its turn's text and tool calls, a turn role's answer under its name, a bot's routine) carries that role's stamp; every row carries the factory version.
    pub fn chat(&self, role: &str, text: &str, project: &str) {
        let project = self.conv(project); // a project or bot that is gone talks in the conversation of no project
        let at = now_iso();
        let by = if matches!(role, "manager" | "tool" | "routine") {
            self.chat_role(project)
        } else {
            role.to_string()
        };
        let (factory, role_hash) = self.stamp_for(&by);
        let id = self.insert("INSERT INTO chat (at, role, text, project, factory, role_hash) VALUES (?1,?2,?3,?4,?5,?6)", params![at, role, text, project, factory, role_hash]);
        self.emit("chat", json!({"id": id, "at": at, "role": role, "text": text, "project": project, "factory": factory, "role_hash": role_hash}));
    }
}

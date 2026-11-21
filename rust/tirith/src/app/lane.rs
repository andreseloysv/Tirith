//! One lane as the daemon keeps it in memory and in SQLite.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Lane {
    pub id: String,
    pub project: String,
    pub step: String,
    pub item: String,
    pub agent: String,
    pub model: Option<String>,
    /// queued, running, waiting (asked the manager), merge-queued, merging, waiting-disk (outside the queue), merged, failed, stopped
    pub status: String,
    /// Its worktree slot (`<tirith home>/lanes/<project>/slot-N`); emptied once the lane ends for good and its worktree is removed.
    pub workspace: String,
    pub branch: String,
    pub pid: Option<u32>,
    pub attempts: u32,
    /// The agent's last message (Stop hook) or Tirith's note.
    pub note: String,
    /// The agent session id seen in its hooks (a resume continues it).
    pub session: String,
    pub question: String,
    /// The message the next run starts with: resumes `session` when there is one, else ends the brief.
    pub resume: String,
    /// Agent aliases already used (out-of-credits fallback).
    pub tried: Vec<String>,
    /// Merged alone (its train was red).
    pub solo: bool,
    /// Its factory stamp: the factory version and the hash of the lane role's setting it was created with.
    pub factory: String,
    pub role_hash: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub merged_at: Option<String>,
    pub last_activity: u64,
}

impl Lane {
    pub fn active(&self) -> bool {
        matches!(
            self.status.as_str(),
            "queued" | "running" | "waiting" | "merge-queued" | "merging"
        )
    }
}

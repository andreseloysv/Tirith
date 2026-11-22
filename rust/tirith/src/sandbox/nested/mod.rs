//! macOS: Codex inside Tirith's sandbox. Its own sandbox cannot start there (macOS refuses any profile but the same one again: «sandbox_apply: Operation not permitted»), and no flag keeps both. So the limit its caller asked for becomes part of one profile: a `codex` on the sandboxed PATH reads what was asked (`asked.rs`, `settings.rs`), and the daemon, from outside, starts Codex under Tirith's profile plus that limit (`limits.rs`, `broker.rs`), its own sandbox off only there. Such a profile is never looser than Tirith's, and what runs under one gets no other. On Linux Codex's sandbox nests by itself: nothing here runs.
#[cfg(target_os = "macos")]
mod ask;
mod asked;
#[cfg(target_os = "macos")]
mod broker;
#[cfg(target_os = "macos")]
mod client;
#[cfg(target_os = "macos")]
mod ending;
mod flags;
mod layers;
mod limits;
mod mode;
#[cfg(target_os = "macos")]
mod place;
#[cfg(target_os = "macos")]
mod program;
mod roots;
mod settings;
#[cfg(target_os = "macos")]
mod spawn;
mod told;
mod values;

#[cfg(target_os = "macos")]
pub(super) use broker::start;
#[cfg(target_os = "macos")]
pub use client::{inside, launcher};

/// The word the wrapper calls the client by.
#[cfg(target_os = "macos")]
const WORD: &str = "nested-codex";

/// What the client asks the daemon for: a program as it is to run, and the limit on it. It names no profile: the daemon writes that.
#[derive(serde::Serialize, serde::Deserialize, Default, Debug, Clone)]
pub(super) struct Request {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub env: Vec<(String, String)>,
    /// Workspace-write (else read-only).
    pub write: bool,
    pub roots: Vec<String>,
    /// Codex's home, which the process itself must write.
    pub state: String,
    /// The file `-o` names.
    pub out: String,
    /// The commands may use the network (as Tirith's sandbox lets it).
    pub net: bool,
}

#[cfg(test)]
#[path = "../../../checks/nested.rs"]
mod tests;

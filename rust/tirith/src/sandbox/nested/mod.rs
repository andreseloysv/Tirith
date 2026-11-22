//! macOS: Codex inside Tirith's sandbox. Its own sandbox cannot start there (macOS refuses any profile but the same one again: «sandbox_apply: Operation not permitted»), and no flag keeps both. So the limit its caller asked for becomes part of one profile: a `codex` on the sandboxed PATH reads what was asked (`asked.rs`, `settings.rs`), and the daemon, from outside, starts Codex under Tirith's profile plus that limit (`limits.rs`, `broker.rs`), its own sandbox off only there; what Codex's process needs of it is that binary's alone, not its commands'. Such a profile is never looser than Tirith's, the broker serves only a caller under Tirith's, and what runs under one gets no other. On Linux Codex's sandbox nests by itself: nothing here runs.
#[cfg(target_os = "macos")]
mod around;
#[cfg(target_os = "macos")]
mod ask;
mod asked;
#[cfg(target_os = "macos")]
mod broker;
#[cfg(target_os = "macos")]
mod client;
mod commands;
mod doors;
#[cfg(target_os = "macos")]
mod ending;
mod flags;
mod here;
mod layers;
mod limits;
mod line;
#[cfg(target_os = "macos")]
mod listen;
mod mode;
#[cfg(target_os = "macos")]
mod other;
#[cfg(target_os = "macos")]
mod place;
mod profiles;
#[cfg(target_os = "macos")]
mod program;
mod roots;
mod sb;
mod services;
mod settings;
#[cfg(target_os = "macos")]
mod spawn;
mod state;
mod told;
mod values;
#[cfg(target_os = "macos")]
mod watch;
mod writes;

#[cfg(target_os = "macos")]
pub(super) use broker::start;
#[cfg(target_os = "macos")]
pub use client::{inside, launcher};

/// The word the wrapper calls the client by.
#[cfg(target_os = "macos")]
const WORD: &str = "nested-codex";
/// And Tirith itself, for a read-only role of its own.
#[cfg(target_os = "macos")]
const ROLE: &str = "nested-codex-role";

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
    /// Codex's home, of which the process itself must write some.
    pub state: String,
    /// The file `-o` names.
    pub out: String,
    /// The commands may use the network (as Tirith's sandbox lets it).
    pub net: bool,
    /// Tirith's own role: its commands keep the daemon, as that agent. No wider than what any confined caller has.
    #[serde(default)]
    pub agent: bool,
}

#[cfg(test)]
#[path = "../../../checks/nested.rs"]
mod tests;

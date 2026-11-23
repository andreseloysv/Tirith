//! macOS: Codex inside OpenBot's sandbox. Its own sandbox cannot start there (macOS refuses any profile but the same one again: «sandbox_apply: Operation not permitted»), and no flag keeps both. So the limit its caller asked for becomes part of the profile: a `codex` on the sandboxed PATH reads what was asked (`asked.rs`, `settings.rs`), and the daemon, from outside, starts Codex under OpenBot's profile plus that limit (`limits.rs`, `broker.rs`), its own sandbox off only there, as two processes (`pair.rs`): Codex's own, which alone has its state, and its `exec-server`, which alone runs the model's commands and patches. Such a profile is never looser than OpenBot's, the broker serves only a caller under OpenBot's, and what runs under one gets no other. On Linux Codex's sandbox nests by itself: nothing here runs.
#[cfg(target_os = "macos")]
mod answers;
#[cfg(target_os = "macos")]
mod around;
#[cfg(target_os = "macos")]
mod ask;
mod asked;
#[cfg(target_os = "macos")]
mod bridge;
#[cfg(target_os = "macos")]
mod broken;
#[cfg(target_os = "macos")]
mod broker;
#[cfg(target_os = "macos")]
mod capable;
#[cfg(target_os = "macos")]
mod carry;
#[cfg(target_os = "macos")]
mod client;
mod commands;
mod doors;
#[cfg(target_os = "macos")]
mod ending;
mod flags;
#[cfg(target_os = "macos")]
mod frames;
mod here;
mod layers;
mod limits;
mod line;
#[cfg(target_os = "macos")]
mod listen;
mod mode;
mod network;
#[cfg(target_os = "macos")]
mod notice;
#[cfg(target_os = "macos")]
mod other;
#[cfg(target_os = "macos")]
mod pair;
#[cfg(target_os = "macos")]
mod place;
mod profiles;
#[cfg(target_os = "macos")]
mod program;
mod request;
mod roots;
mod sb;
mod servers;
mod services;
mod settings;
#[cfg(target_os = "macos")]
mod spawn;
#[cfg(target_os = "macos")]
mod stalled;
mod state;
mod told;
#[cfg(target_os = "macos")]
mod too_big;
#[cfg(target_os = "macos")]
mod upgrade;
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
/// And OpenBot itself, for a read-only role of its own.
#[cfg(target_os = "macos")]
const ROLE: &str = "nested-codex-role";

use request::Request;

#[cfg(test)]
#[path = "../../../checks/nested.rs"]
mod tests;

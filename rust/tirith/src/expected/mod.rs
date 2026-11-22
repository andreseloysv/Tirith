//! What was due and how it ended: every run of a routine and every scheduled pass of a turn role is written down when it is due (table `expected_runs`), before anything starts, and ends in a row of `turns` whether it ran or not. One that failed, was lost or never started says why in that row, counts as a failure, and the owner hears it in one line sent by code, so no quota and no agent is needed for it. «∅» means "I looked and nothing is new", never "I could not" (`claims.rs`, `verdict.rs`).
mod admits;
mod calendar;
mod changed;
mod claims;
mod found;
mod gaps;
mod life;
mod lost;
mod missed;
mod notice;
mod overdue;
mod passes;
mod reasons;
mod restart;
mod store;
mod timed;
mod told;
mod verdict;
mod watch;
mod words;
pub use calendar::today;
pub use changed::changed;
pub use claims::{honest, quiet};
pub use found::known;
pub use gaps::late;
pub use life::{ended, orphaned, started, unhanded, unstarted};
pub use passes::{blocked, pass_due};
pub use restart::start;
pub use store::{expect, New};
pub use verdict::verdict;
pub use watch::tick;
pub use words::first;

/// The routine id of a turn role's scheduled pass.
pub const PASS: &str = "pass";
/// A pass as the owner and the numbers name it.
pub const PASS_TITLE: &str = "Pasada programada";

#[cfg(test)]
#[path = "../../checks/expected.rs"]
mod tests;

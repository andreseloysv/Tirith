//! The lab (docs/BOTS.md, part 3): the Coach's experiments on a bot's routine, each a branch of the bot's repository with one commit. In `live` mode the routine's real runs alternate A (the bot's own folder, its main) and B (the branch, in a worktree `<home>/lab/<bot>/<experiment>` that is the bot's folder for that turn); each B is judged blind against the A before it (eval `ab-compare`), and Tirith decides alone by the factory's `[lab]` rule: the branch is merged into the bot's main or deleted, and the bot's chat gets one line (`lab.adopted`, `lab.discarded`).
mod api;
mod compare;
mod decide;
mod git;
mod numbers;
mod rule;
mod seat;
mod start;
mod store;
mod turn;
pub use api::{cli, routes};
pub use compare::judged;
pub use decide::tick;
pub use seat::{fired, seat};
pub use turn::finished;
#[cfg(test)]
use {
    compare::{blind, verdict},
    decide::decide,
    rule::{Side, Tally},
    start::start,
    store::list,
};
#[cfg(test)]
#[path = "../../checks/lab.rs"]
mod tests;

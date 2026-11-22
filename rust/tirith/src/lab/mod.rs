//! The lab (docs/BOTS.md, part 3): the Coach's experiments on a bot's routine, each a branch of the bot's repository with one commit. In `live` mode the routine's real runs alternate A (the bot's own folder, its main) and B (the branch, in a worktree `<home>/lab/<bot>/<experiment>` that is the bot's folder for that turn); each B is judged blind against the A before it (eval `ab-compare`); in `replay` mode A and B do the routine's last captured runs again in throwaway worktrees, reaching their agent's model and nothing else (`reach.rs`), every outside read answered from the capture (`replay.rs`), and Tirith decides alone by the factory's `[lab]` rule: the branch is merged into the bot's main or deleted, and the bot's chat gets one line (`lab.adopted`, `lab.discarded`); in `shadow` mode B runs beside each real run of A, at the same time, in the branch's worktree, its reads live and every outward action refused (`shadow.rs`), and each pair is compared and decided the same way.
mod adopt;
mod answers;
mod api;
mod blind;
mod call_key;
mod compare;
mod decide;
mod due;
mod fixed;
mod fresh;
mod git;
mod interrupted;
mod numbers;
mod outside;
mod outward;
mod outward_browser;
mod outward_words;
mod reach;
mod recorded;
mod replay;
mod replay_runs;
mod replay_side;
mod replay_warm;
mod rule;
mod saved;
mod scope;
mod seat;
mod shadow;
mod shadow_open;
mod shadow_pair;
mod start;
mod store;
mod turn;
mod void;
pub use answers::{answer, in_replay};
pub use api::{cli, routes};
pub use compare::judged;
pub use decide::tick;
pub use fresh::session as fresh_session;
pub use interrupted::interrupted;
pub use reach::{reaches, replay_args};
pub use replay::{closed, proxy_caller};
pub use scope::scope;
pub use seat::{fired, released, seat};
pub use shadow_open::refused;
pub use turn::finished;
#[cfg(test)]
use {
    blind::{blind, verdict},
    decide::decide,
    rule::{Side, Tally},
    start::start,
    store::list,
};
#[cfg(test)]
#[path = "../../checks/replay.rs"]
mod replay_tests;
#[cfg(test)]
#[path = "../../checks/shadow.rs"]
mod shadow_tests;
#[cfg(test)]
#[path = "../../checks/lab.rs"]
mod tests;

//! The turn log: every turn of a chat or turn role (the project managers, the bots, the CEO, the judge) leaves a row in `turns` — who started it and why, its agent and model, its tokens and cost, its tools and skills, the versions of its memory and skills, its answer (docs/DAEMON.md). Where turns run and are handed over, the code only calls this module.
mod api;
mod live;
mod price;
mod record;
mod save;
mod tools;
mod usage;
mod versions;
pub use api::routes;
#[cfg(test)]
use api::turns;
pub use live::{abandon, begin, done, end, handed, line, on_event, trigger, turn_of};
pub use record::Turn;
#[cfg(test)]
use {price::cost, usage::Usage};
#[cfg(test)]
#[path = "../../checks/turns.rs"]
mod tests;

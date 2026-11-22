//! The turn log: every turn of a chat or turn role (the project managers, the bots, the CEO, the judge) leaves a row in `turns` — who started it and why, its agent and model, its tokens and cost, its tools and skills, the versions of its memory and skills, its answer (docs/DAEMON.md). Where turns run and are handed over, the code only calls this module.
mod api;
mod approval;
mod capture;
mod capture_fit;
mod capture_keep;
mod capture_told;
mod checks;
mod interrupted;
mod live;
mod loose;
mod price;
mod quality;
mod react;
mod record;
mod save;
mod signals;
mod tools;
mod usage;
mod versions;
pub use api::routes;
#[cfg(test)]
pub(crate) use api::turns as rows_of;
pub use approval::released;
#[cfg(test)]
pub use capture_keep::keep as keep_capture;
pub use capture_keep::{capture_of, captured, Captured};
pub use capture_told::told;
pub use checks::cli;
pub use interrupted::start;
pub use live::{abandon, end, handed, idle, line, on_event, take_trigger, trigger, By};
pub use loose::{done, turn_of};
pub use quality::scored;
pub use react::reacted;
pub use record::Turn;
pub use signals::{add_columns, judged, last};
#[cfg(test)]
use {api::turns, checks::request, signals::cited};
#[cfg(test)]
use {price::cost, usage::Usage};
#[cfg(test)]
#[path = "../../checks/approval.rs"]
mod approval_tests;
#[cfg(test)]
#[path = "../../checks/turns.rs"]
mod tests;

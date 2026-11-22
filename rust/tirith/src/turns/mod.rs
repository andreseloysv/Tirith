//! The turn log: every turn of a chat or turn role (the project managers, the bots, the CEO, the judge) leaves a row in `turns` — who started it and why, its agent and model, its tokens and cost, its tools and skills, the versions of its memory and skills, its answer (docs/DAEMON.md). Where turns run and are handed over, the code only calls this module.
mod api;
mod approval;
mod broke;
mod capture;
mod capture_fit;
mod capture_keep;
mod capture_told;
mod checks;
mod codex_log;
mod could_not;
mod evidence;
mod exec_items;
mod follow;
mod hashes;
mod interrupted;
mod live;
mod look_lines;
mod look_muse;
mod looks;
mod loose;
mod muse_log;
mod price;
mod proof;
mod quality;
mod react;
mod record;
mod said;
mod save;
mod settle;
mod shell;
mod signals;
mod skill_read;
mod spent;
mod tools;
mod usage;
mod usage_keys;
mod versions;
pub use api::routes;
#[cfg(test)]
pub(crate) use api::turns as rows_of;
pub use approval::{asked, released};
#[cfg(test)]
pub use capture_keep::keep as keep_capture;
pub use capture_keep::{capture_of, captured, Captured};
pub use capture_told::told;
pub use checks::cli;
pub use follow::follow;
pub use interrupted::start;
pub use live::{abandon, end, handed, idle, line, on_event, take_trigger, trigger, By};
pub use looks::Looks;
pub use loose::{done, turn_of};
pub use quality::{scored, EVAL as GRADE};
pub use react::reacted;
pub use record::Turn;
pub use signals::{add_columns, judged, last};
pub use spent::fresh;
pub use versions::prepare;
#[cfg(test)]
use {api::turns, checks::request, signals::cited};
#[cfg(test)]
use {price::cost, usage::Usage};
#[cfg(test)]
#[path = "../../checks/agents.rs"]
mod agents_tests;
#[cfg(test)]
#[path = "../../checks/approval.rs"]
mod approval_tests;
#[cfg(test)]
#[path = "../../checks/turns.rs"]
mod tests;

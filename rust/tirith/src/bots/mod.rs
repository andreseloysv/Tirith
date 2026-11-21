//! Bots: agents of their own — factory roles with `scope = "own"` (the owner's `bot-<id>` ones and the factory's system bots: CEO, judge, manager of no project) with `schedule` routes in `~/.tirith/config.toml` (written with toml_edit, keeping its comments; a change to a system bot is laid over the factory there), each with a memory and skills (docs/DAEMON.md). A routine's last run stays in kv: it is a stamp, not configuration. What makes a bot a CEO or a judge is only this configuration — its `[[routes]]` (which events it hears, as a turn or a note, on the phones or not), its `session` and its `access` — and its card on the page shows and changes all of it, saved here; routes config.toml writes to a role replace the factory's to that role (see factory.rs).
mod api;
mod card;
mod cli;
mod context;
mod folder;
mod fragment;
mod hook;
mod hook_api;
mod hook_event;
mod hook_fire;
mod layer;
mod library;
mod list;
mod migrate;
mod notes;
mod routine;
mod run;
mod schedule;
mod store;
mod sweep;
mod versioned;
mod written;
pub use api::routes;
/// The app's and the manager's checks read the list and save cards.
#[cfg(test)]
pub(crate) use card::save;
pub use cli::cli;
pub use context::context;
pub use folder::{folder, skills, writable, SKILLS};
pub use hook::{hook_from, webhook};
pub use list::{all, bot, key_for};
pub use migrate::start;
pub use run::tick;
pub use schedule::due;
pub use versioned::versioned;
#[cfg(test)]
#[path = "../../checks/bot_git.rs"]
mod git_tests;
#[cfg(test)]
#[path = "../../checks/bots.rs"]
mod tests;

//! Teaching a bot a task: the owner's Edge records the steps through NeoBot's extension (tools/teach/), the daemon keeps them with what the owner wrote in the bot's chat meanwhile, and at stop the bot is handed both to write a draft skill and ask its questions; the bot then rehearses the draft in the same Edge through the extension, and the owner approves it (docs/DAEMON.md).
mod api;
mod approvals;
mod approve;
mod caller;
mod cli;
mod clock;
mod cors;
mod door;
mod events;
mod ext_api;
mod hand;
mod mask;
mod names;
mod outward;
mod own_page;
mod record;
mod rehearsal;
mod rehearse_api;
mod relay;
mod run;
mod shots;
mod state;
mod steps;
mod url;
pub use api::routes;
pub use cli::cli;
pub use cors::ORIGIN;
pub use door::own_token;
pub use hand::heard;
pub use record::start;
#[cfg(test)]
#[path = "../../checks/teach_fixes.rs"]
mod fixes_tests;
#[cfg(test)]
#[path = "../../checks/rehearse.rs"]
mod rehearse_tests;
#[cfg(test)]
#[path = "../../checks/teach.rs"]
mod tests;
#[cfg(test)]
#[path = "../../checks/teach_words.rs"]
mod words_tests;

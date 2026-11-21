//! Teaching a bot a task: the owner's Edge records the steps through Tirith's extension (tools/teach/), the daemon keeps them with what the owner wrote in the bot's chat meanwhile, and at stop the bot is handed both to write a draft skill and ask its questions; the bot then rehearses the draft in the same Edge through the extension, and the owner approves it (docs/DAEMON.md).
mod api;
mod approve;
mod cli;
mod cors;
mod ext_api;
mod hand;
mod mask;
mod record;
mod rehearsal;
mod rehearse_api;
mod relay;
mod run;
mod shots;
mod state;
pub use api::routes;
pub use cli::cli;
pub use cors::ORIGIN;
pub use hand::heard;
#[cfg(test)]
#[path = "../../checks/rehearse.rs"]
mod rehearse_tests;
#[cfg(test)]
#[path = "../../checks/teach.rs"]
mod tests;

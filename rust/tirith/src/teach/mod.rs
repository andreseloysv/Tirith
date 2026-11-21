//! Teaching a bot a task: the owner's Edge records the steps through Tirith's extension (tools/teach/), the daemon keeps them with what the owner wrote in the bot's chat meanwhile, and at stop the bot is handed both to write a draft skill and ask its questions (docs/DAEMON.md).
mod api;
mod cors;
mod hand;
mod mask;
mod record;
mod state;
pub use api::routes;
pub use cors::ORIGIN;
pub use hand::heard;
#[cfg(test)]
#[path = "../../checks/teach.rs"]
mod tests;

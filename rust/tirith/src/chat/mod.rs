//! Clearing a conversation and giving it a longest life: `clear` drops its chat rows, forgets its saved agent session (kv `manager_session:*:<conversation>`) and ends its live one without cutting a running turn, so its bot starts fresh (memory.md, skills and routines stay). Its life is `chat_keep`, the owner's own words in `~/.tirith/config.toml` — on the bot's role or on the project, written with toml_edit: every minute the rows older than it go, and a sweep that dropped any forgets the saved session too, so the bot cannot remember what is gone. The phones hear both as one sealed line, `tirith-chat {"v":1,...}`, like `tirith-models-state`.

mod clear;
mod cli;
mod keep;
mod routes;
mod sweep;
mod words;
mod written;

pub use cli::cli;
pub use routes::routes;
pub use sweep::start;

#[cfg(test)]
#[path = "../../checks/chat.rs"]
mod tests;

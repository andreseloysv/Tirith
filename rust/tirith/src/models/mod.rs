//! Models per factory role and the fallback chain: the owner's override from the page, else `[roles.<role>]` of config.toml laid over the factory's role, else that role as the factory defines it.
mod browser;
mod browser_args;
mod chat_argv;
mod command;
mod credits;
mod flags;
mod gateway_args;
mod http;
mod lane_argv;
mod owners;
mod phone;
mod roles;
mod save;
mod start;
mod state;
mod updated;
mod validate;
pub use browser::own_browser;
pub use browser_args::browser_args_for;
pub use chat_argv::chat_argv;
pub use command::command;
pub use credits::{fallback, out_of_credits};
pub use gateway_args::gateway_args_for;
pub use http::{cli, routes};
pub use lane_argv::lane_argv;
pub use phone::phone_response;
pub use roles::{lane_role, role};
pub use start::start;
pub use state::state;
#[cfg(test)]
#[path = "../../checks/models.rs"]
mod tests;

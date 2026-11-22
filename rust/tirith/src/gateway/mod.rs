//! The connector gateway: `tirith mcp --bot <id>`, the one MCP server a bot's run is given (never a lane's or a manager's). The process the agent starts holds nothing: it proves the bot with its caller token and hands every message to the daemon, which keeps the bot's connectors open (local ones started in the bot's sandbox, remote ones reached through Tirith's proxy as the bot), puts their secrets only in their own process environment or auth headers, lists only what the connectors' verdict allows, asks that verdict again on every call and records each attempt (docs/DAEMON.md).
mod api;
mod auth;
mod call;
mod canon;
mod cli;
mod door;
mod given;
mod link;
mod list;
mod local;
mod names;
mod open;
mod record;
mod remote;
mod rpc;
mod session;
mod stdio;

pub use api::routes;
pub use cli::cli;

#[cfg(test)]
#[path = "../../checks/gateway_fakes.rs"]
mod fakes;
#[cfg(test)]
#[path = "../../checks/gateway.rs"]
mod tests;

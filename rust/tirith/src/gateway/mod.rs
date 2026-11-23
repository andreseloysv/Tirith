//! The connector gateway: `tirith mcp --bot <id>`, the one MCP server a bot's run is given (never a lane's or a manager's). The process the agent starts holds nothing: it proves the bot with its caller token and hands every message to the daemon, which keeps the bot's connectors open (local ones started in the bot's sandbox, remote ones reached through Tirith's proxy as the bot), puts their secrets only in their own process environment or auth headers, lists only what the connectors' verdict allows, asks that verdict again on every call, runs a call that asks first only on a one-use permit the owner's yes on its card granted, and records each attempt (docs/DAEMON.md).
mod api;
mod approve;
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
mod permit;
mod record;
mod remote;
mod renew;
mod rpc;
mod session;
mod stdio;
mod summary;

pub use api::routes;
pub use cli::cli;
pub use permit::{is_gateway_card, settled};

#[cfg(test)]
#[path = "../../checks/connector_access.rs"]
mod access_tests;
#[cfg(test)]
#[path = "../../checks/gateway_approval.rs"]
mod approval_tests;
#[cfg(test)]
#[path = "../../checks/gateway_fakes.rs"]
mod fakes;
#[cfg(test)]
#[path = "../../checks/connector_oauth.rs"]
mod oauth_tests;
#[cfg(test)]
#[path = "../../checks/connectors_page.rs"]
mod page_tests;
#[cfg(test)]
#[path = "../../checks/gateway.rs"]
mod tests;

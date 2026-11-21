//! Network sandbox: every process Tirith starts runs inside a macOS `sandbox-exec` profile that only allows loopback, with proxy variables pointing at the daemon's own proxy (`sandbox/proxy/`).
mod blocks;
mod check;
mod cli;
mod env;
mod jvm;
mod launch;
mod owner;
mod profile;
mod proxy;
mod routes;
mod rules;
mod safari;
mod start;
mod tags;
mod token;
mod tunnel;
mod wrap;
mod wrappers;

pub use check::check_command;
pub use cli::cli;
pub use profile::sandboxed;
pub use routes::routes;
pub use start::start;
pub use tags::tags;
pub use wrap::wrap;

#[cfg(test)]
#[path = "../../checks/sandbox.rs"]
mod tests;

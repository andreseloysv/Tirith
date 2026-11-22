//! Network sandbox: every process Tirith starts reaches the network only through the daemon's own proxy (`sandbox/proxy/`): a macOS `sandbox-exec` profile that only allows loopback, on Linux a network namespace of its own (`sandbox/linux/`).
mod blocks;
mod caller;
mod check;
mod cli;
mod confine;
mod env;
mod jvm;
mod launch;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod mac;
mod owner;
mod profile;
mod proxy;
mod quiet;
mod replay;
mod routes;
mod rules;
mod safari;
mod start;
mod tags;
mod token;
mod tunnel;
mod wrap;
mod wrappers;

pub use caller::confined;
pub use check::check_command;
pub use cli::cli;
pub use profile::sandboxed;
pub use quiet::launcher;
pub use replay::closes_network;
pub use routes::routes;
pub use start::start;
pub use tags::tags;
pub use wrap::wrap;

#[cfg(test)]
#[path = "../../checks/sandbox.rs"]
mod tests;

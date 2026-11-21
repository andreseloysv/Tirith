//! The factory file (`factory/factory.toml`, its prompts and its eval rubrics): agents, roles, limits, plan words, card kinds, measures and evals, read as data instead of Rust. Strict loading: an unknown key, a role on a missing agent or prompt, or an unknown measure makes a version invalid; the daemon keeps the last good one and reports the error. The repo's factory is embedded under the folder's as its fallback; `~/.tirith/config.toml` is laid over both key by key and wins.
mod agents;
mod current;
mod embedded;
mod evals;
mod extends;
mod keys;
mod lab;
mod layers;
mod limits;
mod lists;
mod loaded;
mod parse;
mod paths;
mod refs;
mod roles;
mod routes;
mod rowrefs;
mod stamp;
mod unknown;
mod validate;
mod values;
mod words;

pub use current::{current, error, load};
pub use lab::Lab;
pub(crate) use lists::EVENTS;
pub use loaded::Factory;
pub use parse::parse;
pub use routes::Route;
pub use stamp::stamp;
pub(crate) use values::merge;

#[cfg(test)]
pub(crate) use embedded::{embedded_prompts, TEXT};

#[cfg(test)]
#[path = "../../checks/factory.rs"]
mod tests;

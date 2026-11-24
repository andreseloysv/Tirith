//! Adding a project (the page's "+", `neobot project add`): git repository, commit guard, a plan with a first step, then a `[[projects]]` block appended to config.toml.

mod add;
mod add_plan;
mod block;
mod cli;
mod colors;
mod config_file;
mod folder;
mod folders;
mod pick;
mod project;
mod refusal;
mod register;
mod repo;
mod routes;

pub use cli::cli;
pub(crate) use colors::color_for;
#[cfg(test)]
pub(crate) use colors::COLORS;
pub(crate) use config_file::rewrite;
pub use routes::{list, routes};

#[cfg(test)]
#[path = "../../checks/projects.rs"]
mod tests;

//! `~/.tirith/config.toml` (projects, agent aliases, this machine's overlay on the factory) and each project's `tirith.toml` (plan file, checks, lane excludes, extra rules). Unknown keys are an error the owner sees, never silently ignored; yesterday's sections are read in memory under the factory's names, the file itself untouched.
mod file;
mod known;
mod load;
mod manifest;
mod migrate;
mod paths;
mod project;
mod table;

pub use file::Config;
pub use manifest::{Manifest, Stage};
#[cfg(test)]
pub use migrate::migrate;
pub use paths::{expand, home_dir, tirith_home};
pub use project::Project;
pub use table::get;

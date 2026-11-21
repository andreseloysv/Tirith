//! The commit guard: git hooks in every project that refuse a commit containing one of the owner's forbidden words (from config.toml only; `guard = false` opts a project out).
mod cli;
mod git_dir;
mod immutable;
mod install;
mod intact;
mod render;
mod routes;
mod start;
mod words;

pub use cli::cli;
pub use install::install;
pub use intact::intact;
pub use routes::routes;
pub use start::start;
pub use words::words;

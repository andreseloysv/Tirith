//! `[power] keep_awake` in config.toml: the Mac does not go to sleep while the daemon lives.
mod argv;
mod start;

pub use start::start;

#[cfg(test)]
#[path = "../../checks/power.rs"]
mod tests;

//! Tirith core: the pieces the CLI, the collector and the desktop app share.
//!
//! Port of the Python collector (`collector/tirith/`); see `docs/RUST-PORT.md` for the layout, the
//! parity harness and the conventions.

pub mod collect;
pub mod home;
pub mod lock;
pub mod manifest;
pub mod projects;
pub mod pytext;
pub mod roadmap;
pub mod state;
pub mod store;
pub mod timeutil;

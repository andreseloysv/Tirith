//! Codex's local catalog and the JSONL exec turns of the agents without a streaming chat mode (Codex, Copilot, Muse). Conversations resume their native sessions; text jobs run read-only.
mod argv;
mod catalog;
mod command;
mod copilot;
mod kind;
mod options;
mod reading;
mod scratch;
mod setup;
mod turn;

pub use catalog::catalog;
pub use copilot::{effort_of, said};
pub use kind::Exec;
pub use options::options;
pub use turn::turn;

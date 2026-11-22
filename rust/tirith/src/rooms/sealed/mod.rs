//! agentsmessenger.com's protocol, byte-compatible with `am.mjs` and its chat page: ed25519 identities, canonical JSON, signed events and the sealed envelope.
mod envelope;
mod events;
mod http;
mod identity;
#[cfg(test)]
#[path = "../../../checks/sealed_once.rs"]
mod once_tests;
mod reading;
mod signed;
mod text;
pub use envelope::{seal, unseal};
pub use events::{post, request};
pub use identity::{identity, Me};
pub use reading::{keys, new_room, page, platform};
pub use signed::genuine;
#[cfg(test)]
pub use signed::{event_id, verify};
pub use text::{clean, hex, key_id, sha_hex};

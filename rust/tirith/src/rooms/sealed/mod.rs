//! agentsmessenger.com's protocol, byte-compatible with `am.mjs` and its chat page: ed25519 identities, canonical JSON, signed events and the sealed envelope.
mod envelope;
mod events;
mod http;
mod identity;
mod reading;
mod signed;
mod text;
pub use envelope::{seal, unseal};
pub use events::post;
pub use identity::{identity, Me};
pub use reading::{keys, new_room, page, platform};
pub use signed::genuine;
pub use text::{clean, hex, key_id, sha_hex};

//! agentsmessenger.com rooms: public rooms Tirith watches, and private `sealed-` rooms where the owner talks to the managers from a phone, end-to-end encrypted. Only a phone paired with a one-use QR is the owner.
mod agents;
mod api;
mod cli;
mod create;
mod deliver;
mod files;
mod forget;
mod handle;
mod history;
mod hooked;
mod hooks;
mod join;
mod keeper;
mod listing;
mod outbox;
mod pair;
mod poll;
mod provide;
mod public;
mod record;
mod roster;
mod routes;
mod say;
pub(crate) mod sealed;
mod settings;
mod share;
mod show;
mod store;
mod svg;
mod tophones;
mod turns;
mod typing;
mod uncite;
mod unsent;
mod view;
mod watch;
mod words;
pub use api::forget;
pub use cli::cli;
pub use hooks::{hook_create, hook_delete};
pub use keeper::start;
pub use record::paired_phone;
use record::{Msg, Peer, Room, CODE_SECS, KEPT, PHONE};
pub use routes::routes;
pub use tophones::{phone_rooms, to_phones, to_phones_surely};
pub use turns::{again, begin, reply};
pub use typing::typing;
pub(crate) use typing::Typing;
#[cfg(test)]
#[path = "../../checks/citations.rs"]
mod citation_tests;
#[cfg(test)]
#[path = "../../checks/notices.rs"]
mod notice_tests;
#[cfg(test)]
#[path = "../../checks/status_room.rs"]
mod status_tests;
#[cfg(test)]
#[path = "../../checks/rooms.rs"]
mod tests;
#[cfg(test)]
#[path = "../../checks/webhooks.rs"]
mod webhook_tests;

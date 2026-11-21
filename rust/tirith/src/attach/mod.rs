//! Chat attachments: files and images the owner drops, pastes or picks in the chat, saved under `$TIRITH_HOME/attachments/<id>/<name>` so the manager can open them by path.
mod describe;
mod fetch;
mod kind;
mod name;
mod routes;
mod upload;

pub use describe::describe;
pub use kind::content_kind;
pub use routes::routes;

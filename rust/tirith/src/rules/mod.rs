//! Rules as code, checked in memory on every `PreToolUse`. A refusal says what to do instead.
mod call;
mod check;
mod confined;
mod hooks;
mod judge;
mod lane;
mod manifest;
mod normalize;
mod owned;
mod patterns;
mod role_at;
mod summary;

pub use check::check;
pub use normalize::{normalize, EDIT_TOOLS};
pub use role_at::role_at;
pub use summary::summary;

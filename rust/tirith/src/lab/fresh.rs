//! A run of a live or shadow experiment starts a session of its own, as the run it is compared with does: what a conversation gathered (and cached) would weigh on one side only.
use crate::app::Shared;

/// The next turn of bot `key` starts with no session to resume.
pub(super) fn session(app: &Shared, key: &str) {
    crate::manager::end_session(key);
    forget(app, key);
}

/// No session of bot `key` is resumed: the one a B run had was the branch's.
pub(super) fn forget(app: &Shared, key: &str) {
    let agent = crate::models::role(app, key).agent;
    app.set_kv(&format!("manager_session:{agent}:{key}"), None);
}

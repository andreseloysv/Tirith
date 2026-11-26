//! A turn made whole and comparable across the agents before its row is kept: its own share, fresh input apart from the cache, the model, and the ends of Muse's background commands.
use super::record::Turn;
use super::{
    codex_log,
    muse_log::{from_muse_session, since},
    spent::own_share,
    usage::Usage,
};
use crate::app::App;

/// `tokens_in` is always the fresh input: what the cache gave is `tokens_cache_read` alone, so no sum counts it twice.
pub(super) fn settle(app: &App, t: &mut Turn) {
    if !t.looks.waiting.is_empty() && !t.session.is_empty() {
        since(&t.session, t.start_us, |record| t.looks.take(record));
    }
    if !t.usage.seen && t.agent == "muse" && !t.session.is_empty() {
        t.usage = from_muse_session(&t.session, t.start_us);
    }
    own_share(app, t);
    let u = &mut t.usage;
    if u.input_has_cache {
        u.input = fresh(u);
        u.input_has_cache = false;
    }
    if t.model.is_empty() {
        t.model = std::mem::take(&mut u.model);
    }
    if t.model.is_empty() && t.agent == "codex" && !t.session.is_empty() {
        t.model = codex_log::model(&t.session).unwrap_or_default();
    }
}

/// The input no cache gave or took: where an agent's input holds its cache, it holds what was read from it and what was written to it (Copilot bills both apart, and so do its own `tokenDetails`).
pub fn fresh(u: &Usage) -> i64 {
    match u.input_has_cache {
        true => (u.input - u.cache_read - u.cache_write).max(0),
        false => u.input,
    }
}

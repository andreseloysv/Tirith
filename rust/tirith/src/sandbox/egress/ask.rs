//! `net = "ask"`: the first destination not on a role's list is refused and the owner is asked on an approval card; their yes puts it on the list (`permit.rs`).
use crate::app::Shared;
use rusqlite::params;

fn sha(role: &str, host: &str) -> String {
    format!("net:{role}|{host}")
}

/// The conversation the card goes to: a bot's own, a lane's project, else the general manager's.
fn conv(app: &Shared, who: &str) -> String {
    let who = who.strip_prefix("merge:").unwrap_or(who);
    match who.split_once('@') {
        Some((_, conv)) => app.conv(conv).to_string(),
        None => app
            .lane(who)
            .map_or("manager-general".into(), |l| l.project),
    }
}

/// Opens the card for `host` of `role` (none while one is open, none once the owner said no); its id when it asks now.
pub(super) fn open(app: &Shared, who: &str, role: &str, host: &str) -> Option<String> {
    let call = sha(role, host);
    let said_no =
        "SELECT COUNT(*) FROM connector_permits WHERE call_sha = ?1 AND status = 'refused'";
    if app
        .query(said_no, [&call], |r| r.get::<_, i64>(0))
        .first()
        .is_some_and(|n| *n > 0)
    {
        return None;
    }
    let question = format!("{role} wants to connect to {host}. Allow it? (yes adds it to its list of allowed destinations)");
    crate::gateway::ask_permit(app, &conv(app, who), &call, &question).ok()
}

/// A card of a destination ended: the owner's yes puts it on its role's list.
pub fn settled(app: &Shared, card: &str) {
    let sql = "SELECT call_sha FROM connector_permits WHERE card = ?1 AND status = 'granted' AND call_sha LIKE 'net:%'";
    let call = app.query(sql, params![card], |r| r.get::<_, String>(0));
    for (role, host) in call
        .iter()
        .filter_map(|c| c.strip_prefix("net:")?.split_once('|'))
    {
        let _ = super::permit::allow(app, Some(role), host);
    }
}

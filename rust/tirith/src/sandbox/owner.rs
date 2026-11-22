//! Who a sandboxed process belongs to.
use crate::app::Shared;

/// The lane and project an event of `who` belongs to ("merge:<lane>" is the lane's; "manager" stays itself).
pub fn owner(app: &Shared, who: &str) -> (String, String) {
    let name = label(who);
    let project = app.lane(&name).map(|l| l.project).unwrap_or_default();
    (name, project)
}

/// `who` as the events name it: a role without its conversation, a replay by its experiment.
fn label(who: &str) -> String {
    let who = who.strip_prefix("merge:").unwrap_or(who);
    match who.strip_prefix("replay:") {
        Some(rest) => format!("replay:{}", rest.split(':').next().unwrap_or("")),
        None => who
            .split_once('@')
            .map_or(who, |(role, _)| role)
            .to_string(),
    }
}

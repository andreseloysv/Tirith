//! Who a sandboxed process belongs to.
use crate::app::Shared;

/// The lane and project an event of `who` belongs to ("merge:<lane>" is the lane's; "manager" stays itself).
pub fn owner(app: &Shared, who: &str) -> (String, String) {
    (
        who.strip_prefix("merge:").unwrap_or(who).to_string(),
        app.lane(who.strip_prefix("merge:").unwrap_or(who))
            .map(|l| l.project)
            .unwrap_or_default(),
    )
}

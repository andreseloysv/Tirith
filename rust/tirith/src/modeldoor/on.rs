//! Whether the door is on, and the day's budgets: `[door] enabled` (on by default in server mode), `[door] budget_usd` for the machine and `door_budget` in `[roles.<role>]` for a role, in dollars a day.
use crate::app::Shared;
use crate::config::get;

pub fn enabled(app: &Shared) -> bool {
    get::<bool>(&app.cfg().raw, &["door", "enabled"])
        .unwrap_or_else(|| crate::sandbox::server_mode(app))
}

fn dollars(v: Option<toml::Value>) -> Option<f64> {
    v.and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
}

/// The machine's daily budget, when there is one.
pub fn machine(app: &Shared) -> Option<f64> {
    dollars(get::<toml::Value>(&app.cfg().raw, &["door", "budget_usd"]))
}

/// The role's daily budget, when it has one.
pub fn role(app: &Shared, role: &str) -> Option<f64> {
    dollars(app.cfg().factory.role::<toml::Value>(role, "door_budget"))
}

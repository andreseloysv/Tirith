//! Reading a factory's routes: who hears an event, and the timed passes.
use super::loaded::Factory;
use super::values::{rows, str_of};

/// One `[[routes]]` entry read for one event: (to, mode, chat, phone, after_minutes).
pub type Route = (String, String, bool, bool, Option<u64>);

/// One `schedule` route with a time of day: (to, at, days).
pub type Schedule = (String, String, Vec<u64>);

fn flag(r: &toml::Value, k: &str) -> bool {
    r.get(k).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn count(v: &toml::Value) -> Option<u64> {
    v.as_integer().and_then(|n| u64::try_from(n).ok())
}

impl Factory {
    /// Every `schedule` route that names a time of day and no task: (to, at, days). A route with a task is a routine, fired from config.toml by bots.rs; one of the factory's own (the CEO's daily pass) by whoever runs its role.
    pub fn schedules(&self) -> Vec<Schedule> {
        rows(&self.raw, "routes")
            .filter(|r| {
                str_of(r, "on") == "schedule"
                    && !str_of(r, "at").is_empty()
                    && str_of(r, "task").is_empty()
                    && r.get("enabled").and_then(|v| v.as_bool()) != Some(false)
            })
            .map(|r| {
                (
                    str_of(r, "to").into(),
                    str_of(r, "at").into(),
                    r.get("days")
                        .and_then(|d| d.as_array())
                        .into_iter()
                        .flatten()
                        .filter_map(count)
                        .collect(),
                )
            })
            .collect()
    }

    /// Who hears `on`: (to, mode, chat, phone, after_minutes) of every `[[routes]]` on it.
    pub fn routes_on(&self, on: &str) -> Vec<Route> {
        rows(&self.raw, "routes")
            .filter(|r| str_of(r, "on") == on)
            .map(|r| {
                (
                    str_of(r, "to").into(),
                    str_of(r, "mode").into(),
                    flag(r, "chat"),
                    flag(r, "phone"),
                    r.get("after_minutes").and_then(count),
                )
            })
            .collect()
    }
}

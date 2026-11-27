//! `neobot costs [--project P] [--since DATE] [--by item|lane|run|issue|model|day|project|bot] [--format csv|json]`.
use crate::client::{call, call_json, Flag};

/// The command, or none for another module's.
pub fn cli(pos: &[&str], flag: Flag) -> Option<Result<(), String>> {
    (pos == ["costs"]).then(|| {
        let arg = |name: &str| flag(name).unwrap_or_default();
        let (project, since, by) = (arg("--project"), arg("--since"), arg("--by"));
        let query = format!("?project={project}&since={since}&by={by}");
        match arg("--format").as_str() {
            "csv" => {
                call("GET", &format!("/api/costs{query}&format=csv"), None).map(|t| print!("{t}"))
            }
            _ => call_json("GET", &format!("/api/costs{query}"), None)
                .map(|v| println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default())),
        }
    })
}

//! One tool call against every rule, in order: the first refusal wins.
use super::{call::Call, confined, hooks, lane, manifest, owned};
use crate::app::{Lane, Shared};
use crate::config::Project;
use serde_json::Value;

pub fn check(
    app: &Shared,
    tool: &str,
    input: &Value,
    lane: Option<&Lane>,
    project: Option<&Project>,
    role: Option<&str>,
) -> Option<String> {
    let cfg = app.cfg();
    let call = Call::new(tool, input);
    let first = hooks::skipped(&call)
        .or_else(|| hooks::waiting(&call, lane.is_some(), role))
        .or_else(|| confined::refused(app, &call, role))
        .or_else(|| owned::guarded(app, &cfg, &call, role));
    if first.is_some() {
        return first;
    }
    let manifest = project.map(|p| (p, app.manifest(&p.repo())));
    manifest
        .as_ref()
        .and_then(|(p, m)| manifest::plan_written(&call, p, m))
        .or_else(|| lane.and_then(|l| lane::refused(app, &cfg, &call, l)))
        .or_else(|| manifest.and_then(|(_, m)| manifest::broken(m, &call, lane.is_some())))
}

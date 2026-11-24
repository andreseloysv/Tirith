//! The rules a project's tirith.toml brings: its plan is written only by NeoBot, and its own `[[rules]]`.
use super::call::Call;
use crate::config::{Manifest, Project};
use regex::Regex;

/// A write to the project's plan: an edit of it, or a shell command that redirects, tees, edits in place, moves, copies, deletes or restores it.
pub(super) fn plan_written(call: &Call, p: &Project, m: &Manifest) -> Option<String> {
    let plan = format!("/{}", m.roadmap.trim_start_matches("./"));
    let escaped = regex::escape(m.roadmap.trim_start_matches("./"));
    let writes = call.bash && Regex::new(&format!(r"(>>?\s*|\btee\s+(-a\s+)?|\b(sed|perl)\s+-[a-zA-Z]*i\S*\s[^;&|]*|\b(mv|cp|rm|truncate)\s[^;&|]*|\bgit\s+(checkout|restore)\s[^;&|]*)\S*{escaped}")).map(|r| r.is_match(call.cmd)).unwrap_or(false);
    let edits = call.edit && call.files.iter().any(|f| f.ends_with(&plan));
    (edits || writes).then(|| {
        format!(
            "The plan is written only by NeoBot. Add an item with: neobot plan add {} <step> \"<text>\"",
            p.id
        )
    })
}

/// The first of the project's own rules the call breaks, with its message and id.
pub(super) fn broken(m: Manifest, call: &Call, in_lane: bool) -> Option<String> {
    m.rules.into_iter().find_map(|r| {
        let applies = !(r.lanes_only && !in_lane)
            && (r.tools.is_empty() || r.tools.iter().any(|t| t == call.tool));
        (applies
            && Regex::new(&r.pattern).is_ok_and(|re| {
                if call.bash {
                    re.is_match(call.cmd)
                } else {
                    call.files.iter().any(|f| re.is_match(f))
                }
            }))
        .then(|| format!("{} ({})", r.message, r.id))
    })
}

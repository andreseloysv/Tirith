//! The one line the owner gets of a run that did not work, written by code and kept until the phones have it.
use super::store::Expected;
use super::words::{short, when};
use crate::app::Shared;

/// The line of a run that ran and failed, or (`partial`, the judge scored it above 0) did a part of its task.
pub(super) fn failed_line(run: &Expected, why: &str, partial: bool) -> String {
    format!(
        "✗ «{}» {} ({}): {}",
        short(&run.title, 70),
        if partial { "a medias" } else { "falló" },
        when(&run.due),
        short(why, 220)
    )
}

/// A run ran and failed (or did a part: `partial`).
pub(super) fn failed(app: &Shared, run: &Expected, why: &str, partial: bool) {
    tell(app, &run.conv, &failed_line(run, why, partial));
}

/// Runs of one routine never ran (several days a daemon was down are one line).
pub(super) fn lost(app: &Shared, runs: &[Expected], why: &str) {
    let (Some(first), Some(last)) = (runs.first(), runs.last()) else {
        return;
    };
    let which = if runs.len() == 1 {
        format!("({})", when(&first.due))
    } else {
        format!(
            "{} veces ({} a {})",
            runs.len(),
            when(&first.due),
            when(&last.due)
        )
    };
    let title = short(&first.title, 70);
    let line = format!("✗ «{title}» no se hizo {which}: {}", short(why, 220));
    tell(app, &first.conv, &line);
}

fn tell(app: &Shared, conv: &str, line: &str) {
    app.event("", conv, "routine.failed", "", line, "");
    crate::manager::notify(app, conv, line);
}

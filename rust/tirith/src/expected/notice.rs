//! The one line the owner gets of a run that did not work, written by code and kept until the phones have it.
use super::store::Expected;
use super::words::{short, when};
use crate::app::Shared;

/// A run ran and failed.
pub(super) fn failed(app: &Shared, run: &Expected, why: &str) {
    let line = format!(
        "✗ «{}» falló ({}): {}",
        short(&run.title, 70),
        when(&run.due),
        short(why, 220)
    );
    tell(app, &run.conv, &line);
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

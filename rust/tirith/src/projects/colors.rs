//! The colours a project is shown with.
use crate::config::Project;

pub(crate) const COLORS: [&str; 8] = [
    "#2f7d5b", "#3b6fd8", "#8a5cd0", "#d0663b", "#c2417c", "#b8860b", "#0e8aa0", "#64748b",
];

/// The colour a project is shown with: its own `[[projects]]` colour, else a stable one from `COLORS` (its id hashed, so a project added by hand without a colour gets the same one every time).
pub(crate) fn color_for(p: &Project) -> String {
    p.color
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map_or_else(|| hashed(&p.id), str::to_string)
}

/// The colour of `COLORS` that an id hashes to.
fn hashed(id: &str) -> String {
    COLORS[id
        .bytes()
        .fold(0usize, |a, b| a.wrapping_mul(31).wrapping_add(b as usize))
        % COLORS.len()]
    .into()
}

//! What the skill lists under «Qué requiere aprobación»: the click a rehearsal must stop at because the skill names it.
use super::names::fold;
/// The skill's section of what needs the owner's yes.
fn section(skill: &str) -> String {
    const TITLE: &str = "## Qué requiere aprobación";
    let Some(at) = skill.find(TITLE) else {
        return String::new();
    };
    let rest = &skill[at + TITLE.len()..];
    rest[..rest.find("\n## ").unwrap_or(rest.len())].to_string()
}
/// The pieces of a line between «», "" or ``.
fn quoted(line: &str) -> Vec<String> {
    let mut out = vec![];
    for (open, close) in [('«', '»'), ('"', '"'), ('`', '`')] {
        let mut rest = line;
        while let Some((_, after)) = rest.split_once(open) {
            let Some((inside, tail)) = after.split_once(close) else {
                break;
            };
            out.push(inside.to_string());
            rest = tail;
        }
    }
    out
}
/// The line of the skill's «Qué requiere aprobación» that names this click's `target`: a quoted piece of it that is the target (or, for a target of four letters or more, holds it or is held by it); else, for such a target, the line itself holding it.
pub(super) fn listed(skill: &str, target: &str) -> Option<String> {
    let want = fold(target.trim()).to_lowercase();
    let same = |piece: &str| {
        let piece = fold(piece.trim()).to_lowercase();
        let long = want.chars().count() >= 4 && piece.chars().count() >= 3;
        piece == want || long && (want.contains(&piece) || piece.contains(&want))
    };
    (want.chars().count() >= 3).then_some(())?; // a name of one or two letters stops nothing
    section(skill)
        .lines()
        .map(str::trim)
        .find(|l| {
            quoted(l).iter().any(|q| same(q))
                || (want.chars().count() >= 4 && fold(l).to_lowercase().contains(&want))
        })
        .map(str::to_string)
}

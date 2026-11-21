//! The rules about the turn itself: commit checks are never skipped, and a chat's turn never waits.
use super::call::Call;
use super::patterns::{FOLLOW, GUARD, QUOTED, WAIT, WRITES};

/// Skipping the commit checks: `--no-verify`, `commit -n`, `core.hooksPath`, or writing `.git/hooks`.
pub(super) fn skipped(call: &Call) -> Option<String> {
    let cmd = call.cmd;
    let hooks_write = call.bash
        && cmd.contains(".git/hooks")
        && WRITES.is_match(&cmd.replace("2>/dev/null", "").replace(">/dev/null", ""));
    let skips = (call.bash && GUARD.is_match(cmd))
        || hooks_write
        || (call.edit && call.files.iter().any(|f| f.contains("/.git/hooks/")));
    skips.then(|| "Commit checks are never skipped (--no-verify, commit -n, core.hooksPath, .git/hooks). Fix what the check reports and commit again.".into())
}

/// A chat role (not a lane) that sleeps or follows something inside its turn.
pub(super) fn waiting(call: &Call, in_lane: bool, role: Option<&str>) -> Option<String> {
    let waits = !in_lane
        && role.is_some()
        && ((call.bash
            && (WAIT.is_match(call.cmd) || FOLLOW.is_match(&QUOTED.replace_all(call.cmd, " "))))
            || call.tool == "clocksleep");
    waits.then(|| "no esperes dentro del turno: contesta y termina; Tirith te avisa cuando una lane entre, falle o pregunte".into())
}

//! The rules about the turn itself: commit checks are never skipped, NeoBot's hooks stay, and a chat's turn never waits.
use super::call::Call;
use super::patterns::{loop_sleeps, FOLLOW, GUARD, QUOTED, UNHOOK, WAIT, WRITES};

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
            && (WAIT.is_match(call.cmd)
                || loop_sleeps(call.cmd)
                || FOLLOW.is_match(&QUOTED.replace_all(call.cmd, " "))))
            || call.tool == "clocksleep");
    waits.then(|| "do not wait inside the turn: answer and finish; NeoBot tells you when a lane lands, fails or asks".into())
}

/// `neobot hooks remove` (quoted text taken out first, so a message may name it): the hooks are how these rules hear a call, so only the owner takes them out.
pub(super) fn unhooked(call: &Call) -> Option<String> {
    let removes = call.bash && UNHOOK.is_match(&QUOTED.replace_all(call.cmd, " "));
    removes.then(|| "NeoBot's hooks are how its rules hear you: only the owner removes them, never an agent.".into())
}

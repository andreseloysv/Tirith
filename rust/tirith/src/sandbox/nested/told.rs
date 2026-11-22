//! What is said of a limit kept in one profile.
use super::mode::Mode;

/// One line for whoever called, so no limit changes in silence.
pub(super) fn said(mode: Mode, net: bool, unread: bool) -> &'static str {
    match (unread, mode, net) {
        (true, ..) => "its permission profile cannot be kept inside Tirith's sandbox, so it runs read-only, in one profile with Tirith's (give --sandbox to choose)",
        (_, Mode::Write, true) => "workspace-write with network is kept in one profile with Tirith's; unlike Codex's own, its commands can also write Codex's state folder",
        (_, Mode::Write, _) => "workspace-write is kept in one profile with Tirith's; unlike Codex's own, its commands can also write Codex's state folder and reach Tirith's own ports",
        _ => "read-only is kept in one profile with Tirith's; unlike Codex's own, its commands can also write Codex's state folder and reach Tirith's own ports",
    }
}

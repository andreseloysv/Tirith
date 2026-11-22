//! The three limits Codex's sandbox knows.

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Mode {
    ReadOnly,
    Write,
    Full,
}

impl Mode {
    pub(super) fn named(name: &str) -> Option<Mode> {
        match name {
            "read-only" => Some(Mode::ReadOnly),
            "workspace-write" => Some(Mode::Write),
            "danger-full-access" => Some(Mode::Full),
            _ => None,
        }
    }
}

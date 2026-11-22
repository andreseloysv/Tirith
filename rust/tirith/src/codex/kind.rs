//! The agents whose turns run as exec turns, one process a turn, printing JSON lines: Codex, GitHub Copilot and Muse.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Exec {
    Codex,
    Copilot,
    Muse,
}

impl Exec {
    /// The exec agent of an alias; none for one that speaks stream-json.
    pub fn of(alias: &str) -> Option<Exec> {
        match alias {
            "codex" => Some(Exec::Codex),
            "copilot" => Some(Exec::Copilot),
            "muse" => Some(Exec::Muse),
            _ => None,
        }
    }

    pub fn alias(self) -> &'static str {
        match self {
            Exec::Codex => "codex",
            Exec::Copilot => "copilot",
            Exec::Muse => "muse",
        }
    }

    /// Its name in errors.
    pub fn name(self) -> &'static str {
        match self {
            Exec::Codex => "Codex",
            Exec::Copilot => "Copilot",
            Exec::Muse => "the muse agent",
        }
    }

    /// How a failed turn of it opens in the chat.
    pub fn failed(self) -> &'static str {
        match self {
            Exec::Codex => "Codex",
            Exec::Copilot => "Copilot",
            Exec::Muse => "The muse agent failed",
        }
    }

    /// Codex and Copilot say their messages as they work; Muse answers once, at the end of its turn.
    pub fn speaks(self) -> bool {
        self != Exec::Muse
    }
}

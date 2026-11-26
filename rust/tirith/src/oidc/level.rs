//! What a person may do, from the least to the most: the three app roles NeoBot knows.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Reads the page, the bots, the runs and the costs; changes nothing.
    Viewer,
    /// Also starts bot runs and tasks, and answers the cards that are not the owner's or an admin's.
    Member,
    /// Also configures bots, routines, connectors and net permits, and approves outward actions.
    Admin,
}

impl Level {
    pub fn name(self) -> &'static str {
        match self {
            Level::Viewer => "viewer",
            Level::Member => "member",
            Level::Admin => "admin",
        }
    }

    pub fn parse(name: &str) -> Option<Level> {
        [Level::Viewer, Level::Member, Level::Admin]
            .into_iter()
            .find(|l| l.name() == name.trim().to_lowercase())
    }
}

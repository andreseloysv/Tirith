//! The daemon's two ports as a limit's profile knows them.
use super::Request;

/// Where an agent's commands find the daemon, and where the limited ones find the rules' hook alone (server/hook_door.rs).
#[derive(Clone, Copy)]
pub(super) struct Doors {
    pub main: u16,
    pub rules: u16,
}

impl Doors {
    /// The port `r`'s commands reach: a role's is the daemon's, anyone else's the hook's.
    pub(super) fn open_to(self, r: &Request) -> u16 {
        if r.agent {
            self.main
        } else {
            self.rules
        }
    }

    /// `r`'s environment says that port, so the hook, and anything that asks Tirith, goes there.
    pub(super) fn told(self, r: &mut Request) {
        const PORT: &str = "TIRITH_PORT";
        r.env.retain(|(name, _)| name != PORT);
        r.env.push((PORT.into(), self.open_to(r).to_string()));
    }
}

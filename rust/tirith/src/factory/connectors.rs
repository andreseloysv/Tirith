//! A factory version's connectors, and the ones a bot is granted.
use super::loaded::Factory;
use crate::connectors::{grant_of, Connector, Grant};

#[allow(dead_code)] // the gateway (plan step 38) calls them
impl Factory {
    /// The connector with this id.
    pub fn connector(&self, id: &str) -> Option<&Connector> {
        self.connectors.iter().find(|c| c.id == id)
    }

    /// The connectors granted to the bot whose role is `role` (`bot-<id>`), each with its grant; none for any other role.
    pub fn granted(&self, role: &str) -> Vec<(&Connector, Grant)> {
        let grants = self.raw["roles"]
            .get(role)
            .filter(|_| role.starts_with("bot-"))
            .and_then(|r| r.get("connectors"))
            .and_then(|c| c.as_table());
        grants
            .into_iter()
            .flatten()
            .filter_map(|(id, g)| Some((self.connector(id)?, grant_of(g.as_table()?))))
            .collect()
    }
}

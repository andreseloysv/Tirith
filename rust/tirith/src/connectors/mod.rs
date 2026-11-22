//! Connectors: MCP servers defined once in `~/.tirith/config.toml` (`[[connectors]]`, outside every repo), and what each bot may do with one (`[roles.bot-<id>.connectors.<connector>]`, its grant). A bot is granted a connector only when that table exists; the grant can add `tools`, `deny` and `approval` but only narrows the connector. One pure verdict decides both what a bot is listed and what a call does (docs/DAEMON.md).
// The gateway (plan step 38) is the first caller of the verdict and of the factory's grants; until it lands only the checks call them.
#![allow(dead_code, unused_imports)]
mod check;
mod glob;
mod grants;
mod read;
mod row;
mod verdict;

pub use check::checked;
pub use read::grant as grant_of;
pub use verdict::{listed, refusal, verdict, Verdict};

/// One `[[connectors]]` row: a local server (`command`, an argv) or a remote one (`url`), its rules, and the names (never the values) of the secrets it is given.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Connector {
    pub id: String,
    pub command: Vec<String>,
    pub url: String,
    pub blocked: bool,
    pub deny: Vec<String>,
    pub approval: Vec<String>,
    pub secrets: Vec<String>,
}

/// One bot's grant of a connector: an empty one is the connector as defined; `tools`, when set, is all it may use.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Grant {
    pub tools: Option<Vec<String>>,
    pub deny: Vec<String>,
    pub approval: Vec<String>,
}

#[cfg(test)]
#[path = "../../checks/connectors.rs"]
mod tests;

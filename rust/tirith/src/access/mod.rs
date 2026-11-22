//! Connectors on the Access page, the owner's alone: the gateway's calls beside the network destinations (newest first, by bot and by connector: time, bot, `connector.tool`, verdict), and Block / Unblock of a connector for one bot or for all. One bot's block takes its `[roles.bot-<id>.connectors.<connector>]` table out of config.toml and remembers it, so its unblock lays it back exactly; all bots' sets `blocked = true` on the `[[connectors]]` row. Both are written by the config writer with toml_edit (every comment stays) and reload the config, so the next call is refused with no restart (docs/DAEMON.md).
mod block;
mod calls;
mod change;
mod kept;
mod place;
mod routes;

pub use kept::blocked_for;
pub use routes::routes;

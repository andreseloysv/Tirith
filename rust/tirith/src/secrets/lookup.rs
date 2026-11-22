//! The daemon's own lookup of one value, by bot and name, and its own keeping of a connector's sign-in. No route and no command leads here; only the connector gateway (`gateway/`) and the sign-in (`oauth/`) ask, and a value goes in nothing but its connector's own process or auth headers.
use super::store::{alone, Store};
use super::{index::Held, names};
use crate::app::Shared;

/// The value bot `bot` (its page id) has as `name`, from the system's store or the sealed file. It blocks.
pub fn value(app: &Shared, bot: &str, name: &str) -> Result<Option<String>, String> {
    Store::of(app).value(bot, name)
}

/// The daemon keeps `value` as bot `bot`'s `name`: a sign-in's reserved names, which no route sets. It blocks.
pub fn keep(app: &Shared, bot: &str, name: &str, value: &str) -> Result<(), String> {
    Store::of(app).set(bot, name, value)
}

impl Store<'_> {
    /// The value bot `bot` has as `name`. It blocks: off the async threads (`spawn_blocking`).
    pub fn value(&self, bot: &str, name: &str) -> Result<Option<String>, String> {
        names::secret(name)?;
        let _one = alone();
        self.exists(bot)?;
        let held = Held::read(self.app, bot)?;
        let Some(place) = held.names.get(name) else {
            return Ok(None);
        };
        self.at(*place)
            .get(&names::service(bot), &held.account(name))
    }
}

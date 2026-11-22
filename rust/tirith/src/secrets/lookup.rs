//! The daemon's own lookup of one value, by bot and name. No route and no command leads here, and nothing puts a value in a process's environment.
use super::store::{alone, Store};
use super::{index::Held, names};

impl Store<'_> {
    /// The value bot `bot` has as `name`. It blocks: off the async threads (`spawn_blocking`).
    #[cfg_attr(not(test), allow(dead_code))] // what hands a secret to its use is another item's
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

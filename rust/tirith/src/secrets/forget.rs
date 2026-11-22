//! Removal: one value, or all of a bot's with its role.
use super::backend::Backend;
use super::store::{alone, Store};
use super::{index::Held, index::Place, names};
use crate::app::Shared;

impl Store<'_> {
    pub fn remove(&self, bot: &str, name: &str) -> Result<(), String> {
        names::bot(bot)?;
        names::secret(name)?;
        let _one = alone();
        let mut held = Held::read(self.app, bot)?;
        let place = held.names.get(name).copied();
        let place = place.ok_or_else(|| format!("bot '{bot}' has no secret {name}"))?;
        let (service, account) = (names::service(bot), held.account(name));
        if place == Place::Os {
            self.os.remove(&service, &account)?;
        }
        self.file.remove(&service, &account)?; // also what a move to the OS store left
        held.names.remove(name);
        held.save(self.app, bot)
    }

    /// Removes every value of `bot`, then runs `role`. A value that cannot go keeps the role, so the owner can try again; none is set in between.
    pub fn forgotten<T>(
        &self,
        bot: &str,
        role: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let _one = alone();
        let mut held = Held::read(self.app, bot)?;
        let service = names::service(bot);
        for (name, place) in held.names.clone() {
            if place == Place::Os {
                self.os.remove(&service, &held.account(&name))?;
            }
        }
        self.file.clear(&service)?;
        held.names.clear();
        held.save(self.app, bot)?;
        role()
    }
}

/// `role` takes the bot out of config.toml. It blocks: off the async threads (`spawn_blocking`).
pub fn forgotten<T>(
    app: &Shared,
    bot: &str,
    role: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    Store::of(app).forgotten(bot, role)
}

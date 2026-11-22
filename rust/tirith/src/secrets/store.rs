//! The store, used one call at a time: a value set, and the names. A name is listed before its value is written, so a removal always finds it.
use super::backend::{system, Backend};
use super::{fallback::Fallback, index::Held, index::Place, names};
use crate::app::Shared;
use std::sync::{Mutex, MutexGuard};

static ONE: Mutex<()> = Mutex::new(());

pub(super) fn alone() -> MutexGuard<'static, ()> {
    ONE.lock().unwrap_or_else(|e| e.into_inner())
}

pub(super) struct Store<'a> {
    pub app: &'a Shared,
    pub os: &'a dyn Backend,
    pub file: Fallback,
}

impl<'a> Store<'a> {
    pub fn of(app: &'a Shared) -> Store<'a> {
        Store::on(app, system())
    }

    pub fn on(app: &'a Shared, os: &'a dyn Backend) -> Store<'a> {
        let file = Fallback::at(&app.home);
        Store { app, os, file }
    }

    pub fn at(&self, place: Place) -> &dyn Backend {
        match place {
            Place::Os => self.os,
            Place::File => &self.file,
        }
    }

    pub(super) fn exists(&self, bot: &str) -> Result<(), String> {
        names::bot(bot)?;
        crate::bots::key_for(self.app, bot)
            .map(drop)
            .ok_or_else(|| format!("no bot '{bot}'"))
    }

    /// The OS store when there is one. A value that is there never moves down to the file.
    pub fn set(&self, bot: &str, name: &str, value: &str) -> Result<(), String> {
        names::secret(name)?;
        names::value(value)?;
        let _one = alone();
        self.exists(bot)?;
        let mut held = Held::read(self.app, bot)?;
        let was = held.names.get(name).copied();
        let place = if was == Some(Place::Os) || self.os.available() {
            Place::Os
        } else {
            Place::File
        };
        if was.is_none() {
            held.names.insert(name.into(), place);
            held.save(self.app, bot)?;
        }
        let (service, account) = (names::service(bot), held.account(name));
        if let Err(why) = self.at(place).set(&service, &account, value) {
            if was.is_none() {
                held.names.remove(name);
                held.save(self.app, bot)?;
            }
            return Err(why);
        }
        if was == Some(Place::File) && place == Place::Os {
            held.names.insert(name.into(), place);
            held.save(self.app, bot)?;
            self.file.remove(&service, &account)?;
        }
        Ok(())
    }

    pub fn list(&self, bot: &str) -> Result<Vec<String>, String> {
        names::bot(bot)?;
        let _one = alone();
        let held = Held::read(self.app, bot)?;
        if held.names.is_empty() {
            self.exists(bot)?;
        }
        Ok(held.names.into_keys().collect())
    }
}

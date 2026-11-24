//! Which names a bot has and where each value lies (kv `secrets:<bot>`), never a value: listing reads nothing else.
use crate::app::App;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Place {
    Os,
    File,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Held {
    /// This home's own mark in the accounts: the OS store is one for every NeoBot home of a user, and all of them have a bot `ceo`.
    tag: String,
    pub names: BTreeMap<String, Place>,
}

impl Held {
    pub fn read(app: &App, bot: &str) -> Result<Held, String> {
        let Some(text) = app.kv(&key(bot)) else {
            return Ok(Held {
                tag: format!("{:08x}", rand::random::<u32>()),
                names: BTreeMap::new(),
            });
        };
        serde_json::from_str(&text).map_err(|_| "the list of this bot's secrets is damaged".into())
    }

    pub fn account(&self, name: &str) -> String {
        format!("{name}@{}", self.tag)
    }

    /// Written and read back: a value whose name was lost could never be removed with its bot.
    pub fn save(&self, app: &App, bot: &str) -> Result<(), String> {
        let text = serde_json::to_string(self).map_err(|e| e.to_string())?;
        let text = (!self.names.is_empty()).then_some(text);
        app.set_kv(&key(bot), text.as_deref());
        (app.kv(&key(bot)) == text)
            .then_some(())
            .ok_or_else(|| "the list of this bot's secrets could not be saved".into())
    }
}

fn key(bot: &str) -> String {
    format!("secrets:{bot}")
}

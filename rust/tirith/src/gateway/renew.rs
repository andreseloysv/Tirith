//! A signed-in remote connector's renewal: when its server refuses the bearer (401), the daemon renews the bot's sign-in (`oauth::bearer`) and the connection sends the new one from then on, marked sensitive like every secret header.
use crate::app::Shared;
use reqwest::header::HeaderValue;
use std::sync::Mutex;

pub struct Renew {
    app: Shared,
    bot: String,
    id: String,
    now: Mutex<Option<HeaderValue>>,
}

impl Renew {
    /// The renewal of bot `bot`'s sign-in to connector `id`.
    pub fn of(app: &Shared, bot: &str, id: &str) -> Renew {
        let (app, bot, id) = (app.clone(), bot.into(), id.into());
        Renew {
            app,
            bot,
            id,
            now: Mutex::new(None),
        }
    }

    /// The bearer header renewed last, if one was.
    pub fn header(&self) -> Option<HeaderValue> {
        self.now.lock().unwrap().clone()
    }

    /// A new bearer, renewed by the daemon.
    pub async fn again(&self) -> Result<(), String> {
        let bearer = crate::oauth::bearer(&self.app, &self.bot, &self.id, true).await?;
        let bearer = bearer.ok_or("its sign-in is gone")?;
        let mut value = HeaderValue::try_from(format!("Bearer {bearer}"))
            .map_err(|_| "a renewed token is no header value".to_string())?;
        value.set_sensitive(true);
        *self.now.lock().unwrap() = Some(value);
        Ok(())
    }
}

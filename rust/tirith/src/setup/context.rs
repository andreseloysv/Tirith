//! What every step of the guide works with: this home, this binary, the owner's terminal.
use super::{ask::Ask, catalog::Known};
use crate::config::{expand, home_dir, tirith_home, Config};
use std::path::PathBuf;

pub struct Context {
    pub home: PathBuf,
    pub exe: PathBuf,
    pub ask: Ask,
}

impl Context {
    pub fn new(yes: bool) -> Result<Context, String> {
        let home = tirith_home();
        let home = std::path::absolute(&home).unwrap_or(home);
        std::fs::create_dir_all(&home).map_err(|e| format!("{}: {e}", home.display()))?;
        // Tokens, chats and keys live here: on a server other users are never let in.
        crate::os::set_mode(&home, 0o700).map_err(|e| format!("{}: {e}", home.display()))?;
        if std::env::var_os(super::as_user::VIA).is_some() {
            // install.sh's account: the agents' own files and transcripts are in its home
            let account = home_dir();
            crate::os::set_mode(&account, 0o700)
                .map_err(|e| format!("{}: {e}", account.display()))?;
        }
        Ok(Context {
            exe: crate::app::exe().ok_or("cannot tell where the tirith binary is")?,
            home,
            ask: Ask::new(yes),
        })
    }

    /// For the page's setup of `alias`: no terminal, what programs print goes to the page.
    pub fn page(alias: &str) -> Result<Context, String> {
        Ok(Context {
            ask: Ask::page(alias),
            ..Context::new(true)?
        })
    }

    pub fn cfg(&self) -> Config {
        Config::load_at(&self.home)
    }

    /// The home when hooks must name it (it is not the usual one).
    pub fn named_home(&self) -> Option<&std::path::Path> {
        (self.home != home_dir().join(".tirith")).then_some(self.home.as_path())
    }

    /// An agent's hooks file: config.toml's, else the catalog's.
    pub fn settings(&self, known: &Known) -> Option<PathBuf> {
        let set = self.cfg().factory.agent::<String>(&known.alias, "settings");
        set.or_else(|| Some(known.settings.clone()))
            .filter(|s| !s.is_empty())
            .map(|s| expand(&s))
    }
}

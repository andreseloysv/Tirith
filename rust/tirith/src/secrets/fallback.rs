//! Where no OS store exists: one sealed file per bot in `<home>/bot-secrets/`, named by a hash of its service.
use super::{backend::Backend, key, seal};
use crate::rooms::sealed::sha_hex;
use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

type Values = BTreeMap<String, String>;

pub(super) struct Fallback {
    dir: PathBuf,
}

impl Fallback {
    pub fn at(home: &Path) -> Fallback {
        Fallback {
            dir: home.join("bot-secrets"),
        }
    }

    pub(super) fn file(&self, service: &str) -> PathBuf {
        self.dir.join(name(service))
    }

    /// All of one bot's values go, also when they no longer open.
    pub fn clear(&self, service: &str) -> Result<(), String> {
        let file = self.file(service);
        match std::fs::remove_file(&file) {
            Err(e) if e.kind() != ErrorKind::NotFound => Err(format!("{}: {e}", file.display())),
            _ => Ok(()),
        }
    }

    fn values(&self, service: &str) -> Result<Values, String> {
        let file = self.file(service);
        let sealed = match std::fs::read(&file) {
            Ok(sealed) => sealed,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Values::new()),
            Err(e) => return Err(format!("{}: {e}", file.display())),
        };
        key::read(&self.dir)?
            .and_then(|key| seal::open(&key, service, &sealed))
            .and_then(|plain| serde_json::from_slice(&plain).ok())
            .ok_or_else(|| format!("{} does not open with this daemon's key", file.display()))
    }

    fn keep(&self, service: &str, values: &Values) -> Result<(), String> {
        if values.is_empty() {
            return self.clear(service);
        }
        let plain = serde_json::to_vec(values).map_err(|e| e.to_string())?;
        let sealed = seal::seal(&key::made(&self.dir)?, service, &plain)?;
        key::write(&self.dir, &name(service), &sealed)
    }
}

fn name(service: &str) -> String {
    format!("{}.sealed", sha_hex(service.as_bytes()))
}

impl Backend for Fallback {
    fn available(&self) -> bool {
        true
    }

    fn set(&self, service: &str, account: &str, value: &str) -> Result<(), String> {
        let mut values = self.values(service)?;
        values.insert(account.into(), value.into());
        self.keep(service, &values)
    }

    fn get(&self, service: &str, account: &str) -> Result<Option<String>, String> {
        Ok(self.values(service)?.remove(account))
    }

    fn remove(&self, service: &str, account: &str) -> Result<(), String> {
        let mut values = self.values(service)?;
        match values.remove(account) {
            Some(_) => self.keep(service, &values),
            None => Ok(()),
        }
    }
}

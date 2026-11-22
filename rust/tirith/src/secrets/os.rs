//! The OS's credential store, called in this process so that no value is ever in a command's argv: macOS's login Keychain (which lets the program that made an item read it), Windows Credential Manager, Linux's Secret Service. These calls block and may wait for the owner.
use super::backend::Backend;
use keyring::{Entry, Error};

/// What is looked up to learn whether a store answers; it is never written.
const PROBE: &str = "tirith.bot";

pub(super) struct Os;

fn failed(e: Error) -> String {
    format!("the system's credential store: {e}")
}

fn entry(service: &str, account: &str) -> Result<Entry, String> {
    Entry::new(service, account).map_err(failed)
}

impl Backend for Os {
    /// A store that answers "not there" exists; a locked one too, so nothing meant for it falls back in silence.
    fn available(&self) -> bool {
        Entry::new(PROBE, PROBE)
            .is_ok_and(|e| matches!(e.get_secret(), Ok(_) | Err(Error::NoEntry)))
    }

    fn set(&self, service: &str, account: &str, value: &str) -> Result<(), String> {
        entry(service, account)?.set_password(value).map_err(failed)
    }

    fn get(&self, service: &str, account: &str) -> Result<Option<String>, String> {
        match entry(service, account)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(Error::NoEntry) => Ok(None),
            Err(e) => Err(failed(e)),
        }
    }

    fn remove(&self, service: &str, account: &str) -> Result<(), String> {
        let entry = entry(service, account)?;
        match entry.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => {}
            Err(e) => return Err(failed(e)),
        }
        // the Keychain's delete does not say when it failed
        match entry.get_secret() {
            Err(Error::NoEntry) => Ok(()),
            Ok(_) => Err("the system's credential store did not delete the value".into()),
            Err(e) => Err(failed(e)),
        }
    }
}

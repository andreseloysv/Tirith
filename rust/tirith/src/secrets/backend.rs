//! The one shape of a place that keeps values: the OS's credential store, the sealed fallback, or the tests' fake.

pub(super) trait Backend: Sync {
    /// Whether this place exists for this process.
    fn available(&self) -> bool;
    fn set(&self, service: &str, account: &str, value: &str) -> Result<(), String>;
    fn get(&self, service: &str, account: &str) -> Result<Option<String>, String>;
    /// A value that is not there is already removed.
    fn remove(&self, service: &str, account: &str) -> Result<(), String>;
}

#[cfg(not(test))]
pub(super) fn system() -> &'static dyn Backend {
    &super::os::Os
}

/// No test reaches the owner's own credential store.
#[cfg(test)]
pub(super) fn system() -> &'static dyn Backend {
    &super::tests::FAKE
}

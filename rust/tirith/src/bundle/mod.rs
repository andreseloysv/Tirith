//! What the binary carries so that it runs with no checkout beside it (build.rs), and the checkout a development build still reads.
mod checkout;
mod files;
mod launcher;
mod zip;

pub use checkout::checkout;
pub use files::{below, file};
pub use launcher::launcher;
pub use zip::zip;

/// The version, the commit and the target this binary was built for.
pub const VERSION: &str = env!("TIRITH_VERSION");

#[cfg(test)]
#[path = "../../checks/bundle.rs"]
mod tests;

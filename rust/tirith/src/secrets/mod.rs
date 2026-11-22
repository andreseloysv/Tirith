//! A bot's secrets: the owner sets, lists (names only) and removes them, and the daemon alone looks a value up, by bot and name. No route answers a value.
mod api;
mod backend;
mod cli;
mod fallback;
mod forget;
mod index;
mod input;
mod key;
mod lookup;
mod names;
#[cfg(not(test))]
mod os;
mod seal;
mod store;

pub use api::{route, routes};
pub use cli::cli;
pub use forget::forgotten;
pub use lookup::value;

#[cfg(test)]
#[path = "../../checks/secrets.rs"]
mod tests;

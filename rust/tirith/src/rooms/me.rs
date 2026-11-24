//! This OpenBot's identity in the rooms, for what it signs besides messages (a bot package).
use super::sealed::{identity, Me};
use super::settings::my_name;
use crate::app::Shared;
/// The one key of this OpenBot: the messenger's, made on first use.
pub fn me(app: &Shared) -> Result<Me, String> {
    identity(&app.home, &my_name(app))
}

//! What the config says about the rooms: the platform, the owner's names, this manager's name and the public rooms.
use super::sealed;
use crate::{app::Shared, config::get as cfg_get};
use serde::Deserialize;
pub(super) fn base(app: &Shared) -> String {
    cfg_get::<String>(&app.cfg().raw, &["messenger", "url"])
        .unwrap_or_else(|| "https://agentsmessenger.com".into())
        .trim_end_matches('/')
        .to_string()
}
pub(super) fn owners(app: &Shared) -> Vec<String> {
    cfg_get::<Vec<String>>(&app.cfg().raw, &["owner", "names"])
        .unwrap_or_default()
        .iter()
        .map(|n| sealed::clean(n))
        .collect()
}
/// The manager's name in rooms (its key is `<home>/am/<name>.json`): `[messenger] name`, else `tirith-manager`.
pub(super) fn my_name(app: &Shared) -> String {
    Some(sealed::clean(
        &cfg_get::<String>(&app.cfg().raw, &["messenger", "name"]).unwrap_or_default(),
    ))
    .filter(|n| !n.is_empty())
    .unwrap_or_else(|| "tirith-manager".into())
}
/// A public room listed in config.toml (`[[rooms]]`).
#[derive(Deserialize, Default)]
#[serde(default)]
pub(super) struct PublicCfg {
    pub(super) id: String,
    pub(super) project: String,
    pub(super) title: String,
}
pub(super) fn public_rooms(app: &Shared) -> Vec<PublicCfg> {
    cfg_get::<Vec<PublicCfg>>(&app.cfg().raw, &["rooms"]).unwrap_or_default()
}
/// This machine's name, as the phones are told when they connect.
pub(super) fn host() -> String {
    let mut buf = [0u8; 256];
    unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
    String::from_utf8_lossy(&buf[..buf.iter().position(|b| *b == 0).unwrap_or(0)]).into_owned()
}

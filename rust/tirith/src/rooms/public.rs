//! The public rooms Tirith watches, as config.toml lists them.
use super::settings::{my_name, public_rooms};
use super::store::{room, save, update};
use super::Room;
use crate::app::Shared;
/// Every public room in config.toml has its record, with the title and project the config gives it.
pub(super) fn sync(app: &Shared) {
    for p in public_rooms(app).into_iter().filter(|p| !p.id.is_empty()) {
        let title = if p.title.is_empty() {
            p.id.clone()
        } else {
            p.title.clone()
        };
        if room(&p.id).is_none() {
            save(
                app,
                &Room {
                    id: p.id.clone(),
                    title,
                    project: p.project,
                    kind: "public".into(),
                    me: my_name(app),
                    ..Default::default()
                },
            );
        } else {
            update(app, &p.id, |r| (r.title, r.project) = (title, p.project));
        }
    }
}
/// A public room still listed in config.toml.
pub(super) fn listed(app: &Shared, id: &str) -> bool {
    public_rooms(app).iter().any(|p| p.id == id)
}

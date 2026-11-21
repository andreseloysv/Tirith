//! A phone paired in one room is the owner's in all of them: every project, and what belongs to none, gets a room; each of those rooms gets every phone (`share`); and the phones are told the list (`listing`), which the chat page shows as one chat per project. So one QR opens every project, also those added later.
use super::{create::create, listing::label, share::share, store::ROOMS, Room};
use crate::app::Shared;
/// `said` is the last failure reported, so a platform that is away is said once, not every half minute.
pub(super) async fn provide(app: &Shared, said: &mut String) {
    let failed = make_missing(app).await;
    let _ = (!failed.is_empty() && failed != *said).then(|| {
        app.event(
            "",
            "",
            "error",
            "rooms",
            &format!("no room for the phones yet: {failed}"),
            "",
        )
    });
    *said = failed;
    share(app);
}
fn mine() -> Vec<Room> {
    ROOMS
        .lock()
        .unwrap()
        .values()
        .filter(|r| r.kind == "manager")
        .cloned()
        .collect()
}
/// The conversations that get a room: every project, and the roles that carry their own (shown) conversation.
fn conversations(app: &Shared) -> Vec<String> {
    let cfg = app.cfg();
    cfg.projects
        .iter()
        .map(|p| p.id.clone())
        .chain(cfg.factory.roles().into_iter().filter(|r| {
            cfg.factory.role::<String>(r, "scope").as_deref() == Some("own")
                && cfg.factory.role::<bool>(r, "show") != Some(false)
        }))
        .collect()
}
/// Makes the rooms missing for the phones; the last failure, or empty.
async fn make_missing(app: &Shared) -> String {
    let mut failed = String::new();
    let phone = mine().iter().any(|r| !r.devices.is_empty()); // without a phone there is nothing to make
    for id in conversations(app)
        .iter()
        .filter(|id| phone && !mine().iter().any(|r| app.conv(&r.project) == **id))
    {
        // No code: nobody scans this one, the phones are added below. A project that gets none now gets it next time.
        failed = create(app, &label(app, id), id, String::new())
            .await
            .err()
            .unwrap_or(failed);
    }
    failed
}

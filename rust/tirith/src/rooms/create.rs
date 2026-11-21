//! A private room this manager makes on the platform for a conversation, where its phones are paired.
use super::settings::{base, my_name};
use super::{keeper::WAKE, record::Codes, sealed, store::save, watch::watch, Room};
use crate::app::{now_s, Shared};
use serde_json::json;
pub(super) async fn create(
    app: &Shared,
    title: &str,
    project: &str,
    invite: String,
) -> Result<Room, String> {
    let base = base(app);
    let me = sealed::identity(&app.home, &my_name(app))?;
    let id = sealed::new_room(&base).await?;
    sealed::post(&base, &me, "announce", &id, json!("joined")).await?;
    let r = Room {
        id: id.clone(),
        title: clean_title(title),
        project: app.conv(project).into(),
        kind: "manager".into(),
        me: me.name.clone(),
        me_id: me.id.clone(),
        codes: Codes {
            invite,
            invite_at: now_s(),
            ..Default::default()
        },
        ..Default::default()
    };
    save(app, &r);
    watch(app, &id);
    WAKE.notify_one(); // the owner's phones are added to it
    Ok(r)
}
/// The title as the phones may show it: no controls, no markup, 80 characters at most.
fn clean_title(title: &str) -> String {
    let title: String = title
        .chars()
        .filter(|c| !c.is_control() && *c != '<' && *c != '>')
        .take(80)
        .collect();
    title.trim().to_string()
}

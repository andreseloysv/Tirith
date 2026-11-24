// A failed run's one line reaches a phone: only a room that a phone joined counts as the phones'.
use super::store::{save, ROOMS};
use super::{phone_rooms, tophones, Peer, Room};

#[test]
fn a_room_no_phone_joined_takes_no_notice_from_the_phones() {
    let home = std::env::temp_dir().join(format!("tirith-notices-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        home.join("config.toml"),
        "[roles.bot-aviso]\nextends = \"manager\"\nscope = \"own\"\nsession = \"chat\"\nname = \"Avisa\"\n",
    )
    .unwrap();
    let app = crate::app::App::open_at(home.clone());
    let room = |id: &str, project: &str, devices: Vec<Peer>| Room {
        id: id.into(),
        project: project.into(),
        kind: "manager".into(),
        devices,
        ..Default::default()
    };
    let phone = Peer {
        public: "notices-phone".into(),
        ..Default::default()
    };
    save(&app, &room("sealed-notice-empty", "bot-aviso", vec![]));
    save(&app, &room("sealed-notice-general", "", vec![phone]));
    assert_eq!(
        app.conv("bot-aviso"),
        "bot-aviso",
        "the bot's own conversation"
    );
    let own = phone_rooms(&app, "bot-aviso");
    // what `manager::notify` does with it
    let to = if own.is_empty() {
        app.conv("")
    } else {
        "bot-aviso"
    };
    let sent = tophones::lines(&app, to, "✗ «Riego» no se hizo: NeoBot estaba parado");
    for id in ["sealed-notice-empty", "sealed-notice-general"] {
        ROOMS.lock().unwrap().remove(id);
    }
    let _ = std::fs::remove_dir_all(&home);
    assert!(own.is_empty(), "no phone is in it: {own:?}");
    assert!(
        sent.iter().any(|(id, _, _)| id == "sealed-notice-general"),
        "the line falls back to the phone that can hear it: {sent:?}"
    );
}

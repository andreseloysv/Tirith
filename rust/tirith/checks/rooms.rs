use super::*;
#[test]
fn room_attachment_saved_as_file() {
    let home = std::env::temp_dir().join(format!("tirith-att-{}", std::process::id()));
    let png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
    let out = save_image_at(&home, &json!({"mime": "image/png", "data": png}).to_string(), "sealed-abc").unwrap();
    let path = out.strip_prefix("[imagen: ").unwrap().split(']').next().unwrap();
    assert!(!out.contains(png) && path.starts_with(home.join("rooms/files/sealed-abc").to_str().unwrap()) && path.ends_with(".png"), "{out}");
    assert!(std::fs::read(path).unwrap().starts_with(&[0x89, 0x50, 0x4e, 0x47]));
    assert!(save_image_at(&home, "{\"mime\":\"image/png\",\"data\":\"AAAA\"}", "r").is_none());
}
#[test]
fn typing_progress_for_a_room_turn_and_none_without_rooms() {
    // A room turn, with no network: the control is emitted at its start, with its first tool and at its end, for its rooms alone.
    let mut typing = Typing::default();
    assert_eq!(typing.start(vec!["sealed-turn".into()]), Some((vec!["sealed-turn".to_string()], json!({"v": 1, "typing": 1}))), "typing on at the start");
    assert_eq!(typing.start(vec!["sealed-turn".into()]), None, "a second start while it runs says nothing again");
    assert_eq!(typing.line("Read: src/main.rs", 100), Some((vec!["sealed-turn".to_string()], json!({"v": 1, "typing": 1, "line": "Read: src/main.rs"}))), "the first tool line goes at once");
    assert_eq!(typing.line("Read: src/main.rs", 106), None, "the same line twice in a row is dropped");
    assert_eq!(typing.line("Bash: run the tests", 102), None, "at most one line every 5 s");
    assert_eq!(typing.line("Bash: run the tests", 105).map(|s| s.1), Some(json!({"v": 1, "typing": 1, "line": "Bash: run the tests"})), "the next line after 5 s goes");
    let long = typing.line(&"x".repeat(300), 111).unwrap().1;
    assert_eq!(long["line"].as_str().unwrap().chars().count(), 120, "120 chars at most: {long}");
    assert_eq!(typing.stop(), Some((vec!["sealed-turn".to_string()], json!({"v": 1, "typing": 0}))), "typing off at the end");
    assert_eq!(typing.stop(), None, "and only once");
    assert_eq!(typing.line("Read: late.rs", 200), None, "no line after the turn ended");
    // A turn with no rooms (only the page) emits nothing at all.
    let mut quiet = Typing::default();
    assert_eq!(quiet.start(vec![]), None, "no rooms, no typing on");
    assert_eq!(quiet.line("Read: src/main.rs", 100), None, "no rooms, no tool line");
    assert_eq!(quiet.stop(), None, "no rooms, no typing off");
}
#[test]
fn typing_control_is_queued_for_the_turns_room_alone() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    assert!(OUT.set(tx).is_ok(), "the rooms' sender is this test's alone");
    let room = |id: &str, project: &str| Room { id: id.into(), project: project.into(), kind: "manager".into(), devices: vec![Peer { public: "phone-key".into(), ..Default::default() }], ..Default::default() };
    ROOMS.lock().unwrap().extend([room("sealed-typing-a", "typingproj"), room("sealed-typing-b", "otherproj")].map(|r| (r.id.clone(), r)));
    let home = std::env::temp_dir().join(format!("tirith-typing-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("config.toml"), format!("[[projects]]\nid = \"typingproj\"\nrepo = \"{}\"\n[[projects]]\nid = \"otherproj\"\nrepo = \"{}\"\n", home.display(), home.display())).unwrap();
    let app = crate::app::App::open_at(home.clone());
    assert_eq!(phone_rooms(&app, "typingproj"), ["sealed-typing-a"], "a conversation's phone rooms are its own alone");
    typing(Some((vec!["sealed-typing-a".into()], json!({"v": 1, "typing": 1}))));
    typing(Some((vec!["sealed-typing-a".into()], json!({"v": 1, "typing": 1, "line": "Read: a.rs"}))));
    typing(Some((vec!["sealed-typing-a".into()], json!({"v": 1, "typing": 0}))));
    typing(None);
    let sent: Vec<(String, String, Vec<String>)> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    let bodies: Vec<Value> = sent.iter().map(|(_, text, _)| serde_json::from_str(text.strip_prefix("tirith-chat ").unwrap()).unwrap()).collect();
    assert!(sent.iter().all(|(id, _, readers)| id == "sealed-typing-a" && readers == &["phone-key".to_string()]), "queued for the turn's room alone, sealed for its phone: {sent:?}");
    assert_eq!(bodies, [json!({"v": 1, "typing": 1}), json!({"v": 1, "typing": 1, "line": "Read: a.rs"}), json!({"v": 1, "typing": 0})], "start, first tool and end, in order");
    ["sealed-typing-a", "sealed-typing-b"].iter().for_each(|id| drop(ROOMS.lock().unwrap().remove(*id)));
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}
#[test]
fn room_files_expire_after_24h() {
    let (root, then) = (std::env::temp_dir().join(format!("tirith-exp-{}", std::process::id())), std::time::SystemTime::now() - Duration::from_secs(90000));
    let (old, new) = (root.join("a/old.png"), root.join("b/new.png"));
    for f in [&old, &new] {
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, b"x").unwrap();
    }
    std::fs::File::options().write(true).open(&old).unwrap().set_modified(then).unwrap();
    expire_files(&root, Duration::from_secs(86400));
    assert!(!old.exists() && !root.join("a").exists() && new.exists());
}
#[test]
fn rooms_list_carries_faces() {
    let toml = "[roles.bot-abc123.face]
shape = \"drop\"
color = \"#22d3ee\"
[roles.manager.face]
shape = \"hex\"
color = \"#8e7cf0\"
[roles.manager-general.face]
shape = \"dome\"
color = \"#3fb6a8\"
[roles.worker]
name = \"Worker\"
[roles.partial.face]
shape = \"burst\"
[roles.blank.face]
shape = \"\"
color = \"\"
";
    let raw: toml::Table = toml.parse().unwrap();
    let proj = |id: &str, color: Option<&str>| crate::config::Project { id: id.into(), color: color.map(Into::into), ..Default::default() };
    assert_eq!(listed_room(&raw, "sealed-bot", "Noticias", "bot-abc123", None), json!({"r": "sealed-bot", "t": "Noticias", "f": {"shape": "drop", "color": "#22d3ee"}}), "a bot room carries the bot's face");
    assert_eq!(listed_room(&raw, "sealed-proj", "My project", "manager", None), json!({"r": "sealed-proj", "t": "My project", "f": {"shape": "hex", "color": "#8e7cf0"}}), "with no project, the Project Manager's face");
    let (zoo, rela) = (proj("zoo", Some("#2f7d5b")), proj("rela", Some("#d0663b")));
    assert_eq!(
        listed_room(&raw, "sealed-zoo", "Zoo", "manager", Some(&zoo)),
        json!({"r": "sealed-zoo", "t": "Zoo", "f": {"shape": "hex", "color": "#2f7d5b"}}),
        "a project room keeps the Project Manager's shape in the project's own colour"
    );
    assert_eq!(listed_room(&raw, "sealed-rela", "Rela", "manager", Some(&rela))["f"], json!({"shape": "hex", "color": "#d0663b"}), "a second project comes out in its own distinct colour, not the first's");
    let plain = proj("handmade", None);
    let stable = listed_room(&raw, "sealed-hand", "Handmade", "manager", Some(&plain));
    assert_eq!(stable["f"], json!({"shape": "hex", "color": "#c2417c"}), "a project with no colour gets its stable palette colour");
    assert!(crate::projects::COLORS.contains(&stable["f"]["color"].as_str().unwrap()), "the stable colour is one of the palette: {stable}");
    assert_eq!(listed_room(&raw, "sealed-hand", "Handmade", "manager", Some(&plain)), stable, "the same project gets the same stable colour in every list");
    assert_eq!(listed_room(&raw, "sealed-hand", "Handmade", "manager", Some(&proj("handmade", Some(""))))["f"], stable["f"], "a project added by hand with an empty colour gets the same stable colour and does not break the list");
    assert_eq!(listed_room(&raw, "sealed-all", "All projects", "manager-general", None), json!({"r": "sealed-all", "t": "All projects", "f": {"shape": "dome", "color": "#3fb6a8"}}), "All projects carries the Manager's face");
    assert_eq!(listed_room(&raw, "sealed-x", "X", "worker", None)["f"], json!({"shape": "round", "color": "#3b82f6"}), "a role with no face gets round blue");
    assert_eq!(listed_room(&raw, "sealed-x", "X", "no-such-role", None)["f"], json!({"shape": "round", "color": "#3b82f6"}), "an unknown role gets round blue");
    assert_eq!(listed_room(&raw, "sealed-x", "X", "partial", None)["f"], json!({"shape": "burst", "color": "#3b82f6"}), "a missing colour alone defaults");
    assert_eq!(listed_room(&raw, "sealed-x", "X", "blank", None)["f"], json!({"shape": "round", "color": "#3b82f6"}), "an empty face defaults");
    let list = json!([listed_room(&raw, "sealed-bot", "Noticias", "bot-abc123", None), listed_room(&raw, "sealed-zoo", "Zoo", "manager", Some(&zoo)), listed_room(&raw, "sealed-all", "All projects", "manager-general", None)]).to_string();
    assert!(list.contains("\"f\":{\"color\":\"#22d3ee\",\"shape\":\"drop\"}") || list.contains("\"f\":{\"shape\":\"drop\",\"color\":\"#22d3ee\"}"), "the list the phones get carries f: {list}");
    assert!(list.contains("#2f7d5b") && list.contains("#3fb6a8"), "the list carries the project's colour and All projects' own: {list}");
}

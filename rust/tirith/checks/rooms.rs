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
    assert_eq!(listed_room(&raw, "sealed-bot", "Noticias", "bot-abc123"), json!({"r": "sealed-bot", "t": "Noticias", "f": {"shape": "drop", "color": "#22d3ee"}}), "a bot room carries the bot's face");
    assert_eq!(listed_room(&raw, "sealed-proj", "My project", "manager"), json!({"r": "sealed-proj", "t": "My project", "f": {"shape": "hex", "color": "#8e7cf0"}}), "a project room carries the Project Manager's face");
    assert_eq!(listed_room(&raw, "sealed-all", "All projects", "manager-general"), json!({"r": "sealed-all", "t": "All projects", "f": {"shape": "dome", "color": "#3fb6a8"}}), "All projects carries the Manager's face");
    assert_eq!(listed_room(&raw, "sealed-x", "X", "worker")["f"], json!({"shape": "round", "color": "#3b82f6"}), "a role with no face gets round blue");
    assert_eq!(listed_room(&raw, "sealed-x", "X", "no-such-role")["f"], json!({"shape": "round", "color": "#3b82f6"}), "an unknown role gets round blue");
    assert_eq!(listed_room(&raw, "sealed-x", "X", "partial")["f"], json!({"shape": "burst", "color": "#3b82f6"}), "a missing colour alone defaults");
    assert_eq!(listed_room(&raw, "sealed-x", "X", "blank")["f"], json!({"shape": "round", "color": "#3b82f6"}), "an empty face defaults");
    let list = json!([listed_room(&raw, "sealed-bot", "Noticias", "bot-abc123"), listed_room(&raw, "sealed-all", "All projects", "manager-general")]).to_string();
    assert!(list.contains("\"f\":{\"color\":\"#22d3ee\",\"shape\":\"drop\"}") || list.contains("\"f\":{\"shape\":\"drop\",\"color\":\"#22d3ee\"}"), "the list the phones get carries f: {list}");
}

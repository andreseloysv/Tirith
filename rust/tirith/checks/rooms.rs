use super::*;
#[test]
fn room_attachment_saved_as_file() {
    let home = std::env::temp_dir().join(format!("tirith-att-{}", std::process::id()));
    std::env::set_var("TIRITH_HOME", &home);
    let png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
    let out = save_image(&json!({"mime": "image/png", "data": png}).to_string(), "sealed-abc").unwrap();
    let path = out.strip_prefix("[imagen: ").unwrap().split(']').next().unwrap();
    assert!(!out.contains(png) && path.starts_with(home.join("rooms/files/sealed-abc").to_str().unwrap()) && path.ends_with(".png"), "{out}");
    assert!(std::fs::read(path).unwrap().starts_with(&[0x89, 0x50, 0x4e, 0x47]));
    assert!(save_image("{\"mime\":\"image/png\",\"data\":\"AAAA\"}", "r").is_none());
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

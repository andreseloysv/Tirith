use super::super::{catalog, hook_lines::heard, hooks, rehook};
use super::{recorded, remove, RECORD};
use serde_json::json;
use std::path::{Path, PathBuf};

fn dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tirith-unhook-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn read(file: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
}

#[test]
fn an_uninstall_takes_out_only_neobots_hooks() {
    let d = dir("file");
    let file = d.join("settings.json");
    let ours = |event: &str| json!({"hooks": [{"type": "command", "command": format!("C:/Users/me/AppData/Local/NeoBot/neobot.exe hook --agent x --event {event}"), "timeout": 10}]});
    let own = json!({"model": "mine", "hooks": {
        "PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "my-own-check"}]}, ours("PreToolUse")],
        "Stop": [ours("Stop")],
        "PostToolUse": [{"hooks": [{"type": "http", "url": "http://127.0.0.1:7717/hook/PostToolUse"}]}],
        "Notification": []}});
    std::fs::write(&file, own.to_string()).unwrap();
    assert!(hooks::remove(&file).unwrap());
    let now = read(&file);
    assert_eq!(
        now,
        json!({"model": "mine", "hooks": {
            "PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "my-own-check"}]}],
            "Notification": []}}),
        "the owner's own hook and their empty list stay; events that held only NeoBot's go"
    );
    assert!(!hooks::remove(&file).unwrap(), "nothing left to take out");
    assert!(
        !hooks::remove(&d.join("none.json")).unwrap(),
        "no file, nothing to do"
    );
    std::fs::write(&file, "{ not json").unwrap();
    assert!(!hooks::remove(&file).unwrap());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "{ not json");
    std::fs::remove_dir_all(&d).unwrap();
}

#[test]
fn hooks_an_uninstall_took_out_come_back_at_the_next_start() {
    let home = dir("home");
    let (muse, copilot) = (home.join("muse.json"), home.join("copilot.json"));
    let quoted = |p: &Path| toml::Value::String(p.display().to_string()).to_string();
    let config = format!(
        "[agents.muse]\nsettings = {}\n[agents.copilot]\nsettings = {}\n[agents.codex]\nsettings = {}\n",
        quoted(&muse),
        quoted(&copilot),
        quoted(&home.join("never-written.json"))
    );
    std::fs::write(home.join("config.toml"), config).unwrap();
    let old = Path::new("/old/place/neobot");
    for (alias, file) in [("muse", &muse), ("copilot", &copilot)] {
        let known = catalog::known(alias).unwrap();
        assert!(hooks::install(&known, file, old, Some(&home)).unwrap());
    }
    assert!(heard(&std::fs::read_to_string(&muse).unwrap()));
    assert_eq!(remove(&home).unwrap(), ["copilot", "muse"]);
    for file in [&muse, &copilot] {
        let text = std::fs::read_to_string(file).unwrap();
        assert!(!heard(&text) && !text.contains("neobot"), "{text}");
    }
    assert_eq!(read(&muse)["schema_version"], 1, "its own keys stay");
    assert_eq!(
        remove(&home).unwrap(),
        ["copilot", "muse"],
        "a second uninstall still knows who waits"
    );
    let new = Path::new("/new/place/neobot");
    assert_eq!(rehook::restore(&home, new), ["copilot", "muse"]);
    let text = std::fs::read_to_string(&muse).unwrap();
    assert!(
        heard(&text) && text.contains("/new/place/neobot hook --agent muse --event PreToolUse")
    );
    assert!(read(&copilot)["hooks"]["PreToolUse"][0]["bash"]
        .as_str()
        .unwrap()
        .starts_with("/new/place/neobot hook --agent copilot"));
    assert!(!home.join(RECORD).exists() && recorded(&home).is_empty());
    assert!(
        rehook::restore(&home, new).is_empty(),
        "nothing waits any more"
    );
    std::fs::remove_dir_all(&home).unwrap();
}

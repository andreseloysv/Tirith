use serde_json::json;
use std::path::Path;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[tokio::test]
async fn bot_folders_are_git_repos() {
    let home = std::env::temp_dir().join(format!("tirith-bot-git-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    // A bot on a fake stream-json agent whose turn adds a note to its memory.md.
    let agent = home.join("fake-agent.sh");
    std::fs::write(&agent, r#"#!/bin/sh
echo '{"type":"system","subtype":"init","session_id":"fake-git"}'
while read -r line; do
  echo "- the owner likes short answers" >> memory.md
  echo '{"type":"assistant","message":{"content":[{"type":"text","text":"anotado"}]}}'
  echo '{"type":"result","subtype":"success","is_error":false,"usage":{"input_tokens":1,"output_tokens":1}}'
done
"#).unwrap();
    std::fs::set_permissions(&agent, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    std::fs::write(
        home.join("config.toml"),
        format!(
            "[agents.fake]\ncommand = \"{}\"\nchat_args = []\n[network]\nsandbox = false\n",
            agent.display()
        ),
    )
    .unwrap();
    let app = crate::app::App::open_at(home.clone());
    let bot = super::save(&app, json!({"name": "Notas", "shape": "drop", "color": "#22d3ee", "session": "chat", "access": "write"})).unwrap();
    let (id, key) = (
        bot["id"].as_str().unwrap().to_string(),
        format!("bot-{}", bot["id"].as_str().unwrap()),
    );
    app.set_kv(&format!("models:{key}"), Some(r#"{"agent":"fake"}"#));
    // Its own skill, and a shared one linked into its skills folder.
    let dir = home.join("manager").join(&key);
    let shared = home.join("bots/skills/compartida");
    std::fs::create_dir_all(dir.join(".agents/skills/propia")).unwrap();
    std::fs::create_dir_all(&shared).unwrap();
    std::fs::write(dir.join(".agents/skills/propia/SKILL.md"), "# propia\n").unwrap();
    std::fs::write(shared.join("SKILL.md"), "# compartida\n").unwrap();
    std::os::unix::fs::symlink(&shared, dir.join(".agents/skills/compartida")).unwrap();
    std::fs::write(dir.join("memory.md"), "# Memory\n").unwrap();
    std::fs::write(dir.join("notes.txt"), "a working file\n").unwrap();

    crate::manager::say(&app, &key, "apunta que me gustan las respuestas cortas", "").unwrap();
    let mut log = String::new();
    let mut row = json!(null);
    for _ in 0..300 {
        let rows = crate::turns::rows_of(&app, &key, 10);
        row = rows
            .into_iter()
            .find(|r| r["role"] == key.as_str())
            .unwrap_or(json!(null));
        log = git(&dir, &["log", "--format=%s"]);
        if !row.is_null() && log.contains("turn ") {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    crate::manager::end_session(&key);
    // The turn started from a committed state, and, having changed memory.md, left a commit named after it.
    assert_eq!(
        log.trim(),
        format!("turn {}\nbefore a turn", row["id"]),
        "{row}"
    );
    let files = git(&dir, &["ls-files"]);
    assert_eq!(
        files.lines().collect::<Vec<_>>(),
        [".agents/skills/propia/SKILL.md", ".gitignore", "memory.md"],
        "only the memory and its own skills; not the link to the shared skill nor its working files"
    );
    assert!(git(&dir, &["show", "HEAD:memory.md"]).contains("short answers"));
    // The turn's version of memory.md is the one it ran with, the blob committed as it started.
    assert_eq!(
        row["versions"]["memory.md"].as_str().map(str::to_string),
        Some(
            git(&dir, &["rev-parse", "HEAD~1:memory.md"])
                .trim()
                .to_string()
        ),
        "{row}"
    );
    // The shared folder is a repository of its own, with the shared skill.
    let bots = home.join("bots");
    assert_eq!(
        git(&bots, &["ls-files"]).lines().collect::<Vec<_>>(),
        [".gitignore", "skills/compartida/SKILL.md"]
    );
    let before = git(&bots, &["log", "--format=%s"]);

    // A change from the Library is committed as «library»; one that changes nothing commits nothing.
    super::library::learn(
        &app,
        &id,
        &json!({"memory": true, "shared": true, "text": "- the owner writes in Spanish\n"}),
        false,
    )
    .unwrap();
    assert_eq!(git(&bots, &["log", "-1", "--format=%s"]).trim(), "library");
    assert_eq!(
        git(&bots, &["log", "--format=%s"]).lines().count(),
        before.lines().count() + 1
    );
    super::library::learn(
        &app,
        &id,
        &json!({"memory": true, "shared": true, "text": "- the owner writes in Spanish\n"}),
        false,
    )
    .unwrap();
    assert_eq!(
        git(&bots, &["log", "--format=%s"]).lines().count(),
        before.lines().count() + 1
    );
    super::library::learn(
        &app,
        &id,
        &json!({"name": "nueva", "text": "# nueva\n"}),
        false,
    )
    .unwrap();
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]).trim(), "library");
    assert!(git(&dir, &["ls-files"]).contains(".agents/skills/nueva/SKILL.md"));
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

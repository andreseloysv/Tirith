use super::{catalog, hook_lines, hooks, opts};
use serde_json::json;
use std::path::Path;

fn dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("tirith-setup-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_guide_knows_four_agents_and_a_release_can_add_to_one() {
    let all = catalog::read(include_str!("../assets/agents.toml"), "").unwrap();
    let names: Vec<&str> = all.iter().map(|k| k.alias.as_str()).collect();
    assert_eq!(names, ["anthropic", "codex", "copilot", "muse"]);
    let more = "[anthropic]\nbin = \"agent-x\"\n";
    let all = catalog::read(include_str!("../assets/agents.toml"), more).unwrap();
    assert_eq!(
        (all[0].bin.as_str(), all[0].check_model.as_str()),
        ("agent-x", "haiku")
    );
}

#[test]
fn options_without_a_value_never_eat_the_next_word() {
    let args = |s: &str| s.split(' ').map(String::from).collect::<Vec<_>>();
    let o = opts::parse(&args(
        "--server install --agents codex,muse --port=7801 --yes",
    ))
    .unwrap();
    assert!(o.server && o.yes && !o.boot);
    assert_eq!((o.words, o.port), (vec!["install".to_string()], Some(7801)));
    assert_eq!(o.agents.unwrap(), ["codex", "muse"]);
    assert!(opts::parse(&args("--sever")).is_err() && opts::parse(&args("--port 80")).is_err());
}

#[test]
fn hooks_keep_the_owners_own_and_never_pile_up() {
    let d = dir("hooks");
    let file = d.join("settings.json");
    let own = json!({"model": "mine", "hooks": {
        "PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "my-own-check"}]},
                       {"hooks": [{"type": "command", "command": "tirith hook --agent x --event PreToolUse"}]}],
        "Notification": [{"hooks": [{"type": "command", "command": "bell"}]}]}});
    std::fs::write(&file, own.to_string()).unwrap();
    let known = catalog::Known {
        alias: "x".into(),
        hooks: "groups".into(),
        ..Default::default()
    };
    let (exe, home) = (
        Path::new("/opt/My Tools/tirith"),
        Path::new("/srv/tirith home"),
    );
    assert!(hooks::install(&known, &file, exe, Some(home)).unwrap());
    assert!(
        !hooks::install(&known, &file, exe, Some(home)).unwrap(),
        "a second run changes nothing"
    );
    let text = std::fs::read_to_string(&file).unwrap();
    let now: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(now["model"], "mine");
    assert_eq!(
        now["hooks"]["Notification"][0]["hooks"][0]["command"],
        "bell"
    );
    let pre = now["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(pre.len(), 2);
    assert_eq!(pre[0]["hooks"][0]["command"], "my-own-check");
    assert_eq!(
        pre[1]["hooks"][0]["command"],
        "'/opt/My Tools/tirith' hook --agent x --event PreToolUse --home '/srv/tirith home'"
    );
    assert!(hook_lines::heard(&text));
    assert!(!hook_lines::heard(&own_without_ours().to_string()));
    assert!(!hook_lines::ours(&json!({"command": "not-tirith hook"})));
    let kept = std::fs::read_to_string(d.join("settings.json.before-tirith")).unwrap();
    assert!(kept.contains("my-own-check"));
    std::fs::write(&file, "{ not json").unwrap();
    assert!(hooks::install(&known, &file, exe, None).is_err());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "{ not json");
    std::fs::remove_dir_all(&d).unwrap();
}

#[test]
fn root_finds_the_service_user_by_its_exact_name() {
    let passwd = "root:x:0:0:root:/root:/bin/bash\ntirith2:x:1002:1002::/home/no:/bin/sh\ntirith:x:1001:1001::/home/tirith:/bin/sh\n";
    let got = super::as_user::entry(passwd);
    assert_eq!(got, Some(("1001".to_string(), "/home/tirith".to_string())));
    assert_eq!(super::as_user::entry("root:x:0:0::/root:/bin/sh\n"), None);
}

fn own_without_ours() -> serde_json::Value {
    json!({"hooks": {"PreToolUse": [{"hooks": [{"type": "command", "command": "my-own-check"}]}]}})
}

#[test]
fn roles_of_agents_not_connected_move_to_the_primary() {
    let d = dir("roles");
    std::fs::write(
        d.join("config.toml"),
        "# mine\n[roles.judge]\nmodel = \"other-agents-model\"\n",
    )
    .unwrap();
    let moved = super::configure::roles(&d, &["codex".to_string()], "codex").unwrap();
    assert!(
        moved.iter().any(|(r, was)| r == "judge" && was == "muse"),
        "{moved:?}"
    );
    let cfg = crate::config::Config::load_at(&d);
    for role in ["manager", "worker", "judge"] {
        assert_eq!(
            cfg.factory.role::<String>(role, "agent").as_deref(),
            Some("codex"),
            "{role}"
        );
    }
    let text = std::fs::read_to_string(d.join("config.toml")).unwrap();
    assert!(
        text.contains("# mine") && !text.contains("other-agents-model"),
        "{text}"
    );
    assert_eq!(cfg.error, None);
    std::fs::remove_dir_all(&d).unwrap();
}

#[test]
fn a_token_is_kept_for_that_agent_alone_and_private() {
    use std::os::unix::fs::PermissionsExt;
    let d = dir("auth");
    super::auth::keep(&d, "copilot", "T", "abc_123").unwrap();
    assert_eq!(
        super::auth::env(&d, "copilot"),
        [("T".to_string(), "abc_123".to_string())]
    );
    assert!(super::auth::env(&d, "codex").is_empty());
    let mode = std::fs::metadata(d.join("auth/copilot.env"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    assert!(super::auth::keep(&d, "copilot", "T", "two words").is_err());
    std::fs::remove_dir_all(&d).unwrap();
}

#[test]
fn a_pasted_token_loses_the_terminals_key_codes() {
    let clean = super::secret::cleaned;
    for typed in [
        "\x1b[Atok_A-1",
        "\x1b[200~tok_A-1\x1b[201~",
        "\u{feff}tok_A-1\r\n",
        "\x1bOBtok_A-1 ",
        "tok_A-1",
    ] {
        assert_eq!(clean(typed), "tok_A-1", "{typed:?}");
    }
    assert_eq!(clean("two words"), "two words"); // still refused as a token, not glued
}

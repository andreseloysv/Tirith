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
    let muse = (all[3].check_model.as_str(), all[3].check_effort.as_str());
    assert_eq!(
        muse,
        ("muse-ember-1.0", "low"),
        "its default model is its dearest"
    );
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

#[test]
fn a_failed_check_says_what_the_agent_said() {
    let log = "starting\n  • Error: 401 Unauthorized\n  token lacks the Copilot Requests permission\nbye\n";
    assert_eq!(
        super::why::picked(log),
        "Error: 401 Unauthorized | token lacks the Copilot Requests permission"
    );
    assert_eq!(super::why::picked("one\ntwo\n\n"), "two");
}

#[test]
fn who_installed_it_may_reach_the_page_through_a_tunnel() {
    let home = dir("trusted");
    let file = home.join("config.toml");
    std::fs::write(&file, "[server]\nport = 7800\n").unwrap();
    super::trusted::add(&home, "deploy").unwrap();
    super::trusted::add(&home, "deploy").unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert_eq!(text.matches("deploy").count(), 1, "{text}");
    assert!(text.contains("port = 7800"), "{text}");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn an_enterprises_host_is_kept_with_the_token_and_never_shown() {
    let home = dir("host");
    let (host, token) = (
        "Https://Made-Up-Corp.ghe.com/",
        "github_pat_madeupmadeup1234",
    );
    let kept = super::host::cleaned(host).unwrap();
    assert_eq!(kept, "made-up-corp.ghe.com");
    super::auth::keep(&home, "copilot", "COPILOT_GH_HOST", &kept).unwrap();
    super::auth::keep(&home, "copilot", "COPILOT_GITHUB_TOKEN", token).unwrap();
    super::auth::keep(&home, "copilot", "COPILOT_GITHUB_TOKEN", token).unwrap();
    let vars = super::auth::env(&home, "copilot");
    assert_eq!(vars.len(), 2, "the token keeps the host, and itself once");
    let hidden = super::private::Hidden::in_home(&home);
    let line =
        format!("GET https://api.MADE-UP-CORP.ghe.com/user as ana_made-up-corp with {token}");
    let shown = hidden.hide(&line);
    assert!(
        !shown.to_lowercase().contains("made-up-corp") && !shown.contains(token),
        "{shown}"
    );
    let public = [("COPILOT_GH_HOST".to_string(), "github.com".to_string())];
    assert!(
        super::private::Hidden::of(&public).none(),
        "github.com is no secret"
    );
    for bad in ["two words.com", "nodots", "a.com/path", "$(id).com"] {
        assert!(super::host::cleaned(bad).is_err(), "{bad}");
    }
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn copilot_is_the_agent_with_a_host() {
    let known = super::catalog::known("copilot").unwrap();
    assert_eq!(
        (known.host_env.as_str(), known.host_flag.as_str()),
        ("COPILOT_GH_HOST", "--host")
    );
    assert!(super::catalog::known("codex").unwrap().host_env.is_empty());
}

#[test]
fn a_token_never_follows_to_another_host() {
    use super::host::changed;
    assert!(
        changed(None, "name.ghe.com"),
        "a token with no host named was github.com's"
    );
    assert!(changed(Some("name.ghe.com"), "github.com"));
    assert!(!changed(None, "github.com") && !changed(Some("name.ghe.com"), "name.ghe.com"));
    let home = dir("forget");
    super::auth::keep(
        &home,
        "copilot",
        "COPILOT_GITHUB_TOKEN",
        "github_pat_madeupmadeup",
    )
    .unwrap();
    super::auth::keep(&home, "copilot", "COPILOT_GH_HOST", "name.ghe.com").unwrap();
    assert!(super::auth::forget(&home, "copilot", "COPILOT_GITHUB_TOKEN").unwrap());
    let left = super::auth::env(&home, "copilot");
    assert_eq!(
        left,
        [("COPILOT_GH_HOST".to_string(), "name.ghe.com".to_string())]
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn roots_file_is_handed_over_on_stdin() {
    let d = dir("root-put");
    let to = d.join("app.tirith.daemon.plist");
    // `env` stands in for sudo: the same shell gets the same text, as this user
    let put = super::root::write(Path::new("/usr/bin/env"), &to, "<plist>the job</plist>\n");
    assert!(put, "written");
    assert_eq!(
        std::fs::read_to_string(&to).unwrap(),
        "<plist>the job</plist>\n"
    );
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        std::fs::metadata(&to).unwrap().permissions().mode() & 0o777,
        0o644
    );
    assert_eq!(std::fs::read_dir(&d).unwrap().count(), 1, "no draft stays");
    assert!(
        !super::root::write(Path::new("/usr/bin/env"), &to, ""),
        "an empty file never replaces one"
    );
    let found = super::root::program("sh").unwrap();
    assert!(
        ["/usr/bin/sh", "/bin/sh"]
            .iter()
            .any(|p| found == Path::new(p)),
        "a system one"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn muse_chat_turn_is_an_exec_run() {
    let a = super::muse_argv(
        "muse",
        false,
        (Some("m"), Some("high")),
        "/w",
        Some("s-1"),
        "/w/p.txt",
        &[],
    );
    let has = |x: &str| a.iter().any(|y| y == x);
    assert!(
        a[1] == "exec" && has("--json") && has("--yolo") && has("--prompt-file"),
        "{a:?}"
    );
    assert!(
        has("--model") && has("--reasoning-effort") && has("--session-id"),
        "{a:?}"
    );
    // The failure this exists for: the other agent's chat flags must never reach this binary.
    assert!(
        !has("-p")
            && !has("--input-format")
            && !has("--output-format")
            && !has("--append-system-prompt"),
        "{a:?}"
    );
    assert!(!has("--disable-write"), "{a:?}");
}

/// An app at a fresh home whose config puts Muse in place, with this `[browser] use` ("" = the section is absent, NeoBot's own browser).
fn browser_home(tag: &str, use_mine: bool) -> crate::app::Shared {
    let dir = std::env::temp_dir().join(format!("tirith-browser-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let browser = if use_mine {
        "[browser]\nuse = \"mine\"\n"
    } else {
        ""
    };
    std::fs::write(
        dir.join("config.toml"),
        format!("# the owner's\n[agents.muse]\ncommand = \"muse\"\n[agents.codex]\ncommand = \"codex\"\n[agents.anthropic]\ncommand = \"anthropic-agent\"\n{browser}[[projects]]\nid = \"p1\"\nrepo = \"/tmp\"\n"),
    )
    .unwrap();
    let app = crate::app::App::open_at(dir);
    assert_eq!(
        app.cfg().error,
        None,
        "config with [browser] must not read as unknown keys"
    );
    app
}

/// The `tirith_browser` server of a Muse argv: its JSON after `-c mcpServers.tirith_browser=`.
fn browser_of(argv: &[String]) -> serde_json::Value {
    let at = argv
        .iter()
        .position(|a| a == "-c")
        .unwrap_or_else(|| panic!("no -c in {argv:?}"));
    let json = argv[at + 1]
        .strip_prefix("mcpServers.tirith_browser=")
        .unwrap_or_else(|| panic!("not the browser server: {:?}", argv[at + 1]));
    serde_json::from_str(json).unwrap()
}

#[test]
fn muse_runs_get_the_browser_mcp() {
    // Default (use = "tirith"): a lane's argv carries the launcher with --isolated; a bot's turn argv carries it bare (the launcher then opens the persistent profile).
    let app = browser_home("tirith", false);
    let lane = crate::models::lane_argv(
        &app,
        Some("muse"),
        None,
        std::path::Path::new("/tmp/brief.md"),
        std::path::Path::new("/tmp"),
        None,
    )
    .unwrap();
    let server = browser_of(&lane.argv);
    assert!(
        server["command"]
            .as_str()
            .is_some_and(|c| c.ends_with("node")),
        "node runs it: {server}"
    );
    assert!(
        server["args"][0]
            .as_str()
            .is_some_and(|a| a.ends_with("tools/browser/launch.mjs")),
        "the repo's launcher: {server}"
    );
    assert_eq!(
        server["args"][1], "--isolated",
        "a lane never shares or keeps a profile: {server}"
    );
    assert_eq!(
        server["mode"], "optional",
        "a browser that cannot start ends no turn (Muse's default is required): {server}"
    );
    assert!(
        server["required"].is_null(),
        "never both: Muse then drops every server: {server}"
    );
    assert_eq!(
        server["env"]["TIRITH_HOME"], "${TIRITH_HOME}",
        "the whole-value variable both spawns set (Muse refuses a literal env value): {server}"
    );
    let bot = super::muse_argv(
        "muse",
        false,
        (None, None),
        "/w",
        None,
        "/w/p.txt",
        &crate::models::browser_args_for(&app, "muse", false),
    );
    let server = browser_of(&bot);
    assert_eq!(
        server["args"].as_array().map(|a| a.len()),
        Some(1),
        "a bot gets the persistent profile, no mode flag: {server}"
    );
    // While a live session holds that profile (its SingletonLock names this very process), a bot falls back to --isolated instead of failing.
    let profile = app.home.join("browser/profile");
    std::fs::create_dir_all(&profile).unwrap();
    std::os::unix::fs::symlink(
        format!("somehost-{}", std::process::id()),
        profile.join("SingletonLock"),
    )
    .unwrap();
    let server = browser_of(&crate::models::browser_args_for(&app, "muse", false));
    assert_eq!(
        server["args"][1], "--isolated",
        "a held profile means isolated, not an error: {server}"
    );
    // use = "mine": a bot connects to the owner's Edge (--extension); a lane still never touches the owner's browser.
    let app = browser_home("mine", true);
    let bot = super::muse_argv(
        "muse",
        false,
        (None, None),
        "/w",
        None,
        "/w/p.txt",
        &crate::models::browser_args_for(&app, "muse", false),
    );
    let server = browser_of(&bot);
    assert_eq!(
        server["args"][1], "--extension",
        "a bot with use = \"mine\" drives the owner's Edge: {server}"
    );
    let lane = crate::models::lane_argv(
        &app,
        Some("muse"),
        None,
        std::path::Path::new("/tmp/brief.md"),
        std::path::Path::new("/tmp"),
        None,
    )
    .unwrap();
    let server = browser_of(&lane.argv);
    assert_eq!(
        server["args"][1], "--isolated",
        "a lane is isolated even with use = \"mine\": {server}"
    );
    assert!(
        !lane.argv.iter().any(|a| a.contains("--extension")),
        "no lane ever gets the owner's browser: {:?}",
        lane.argv
    );
}

/// The `tirith_browser` of a Codex argv: the TOML value after `-c mcp_servers.tirith_browser=`, parsed as Codex parses it.
fn codex_browser_of(argv: &[String]) -> toml::Value {
    let pair = argv
        .windows(2)
        .find(|w| w[0] == "-c" && w[1].starts_with("mcp_servers.tirith_browser="))
        .unwrap_or_else(|| panic!("no tirith_browser in {argv:?}"));
    assert!(
        argv.windows(2).any(|w| w[0] == "-c"
            && w[1] == "mcp_servers.node_repl={command=\"node_repl\",enabled=false}"),
        "the desktop-app browser is off: {argv:?}"
    );
    format!(
        "v = {}",
        pair[1].strip_prefix("mcp_servers.tirith_browser=").unwrap()
    )
    .parse::<toml::Table>()
    .unwrap()["v"]
        .clone()
}

/// The `tirith_browser` server of an anthropic argv: the JSON of its `--mcp-config=…`.
fn anthropic_browser_of(argv: &[String]) -> serde_json::Value {
    assert!(
        argv.iter()
            .any(|a| a == "--allowedTools=mcp__tirith_browser__*"),
        "its tools are allowed: {argv:?}"
    );
    let json = argv
        .iter()
        .find_map(|a| a.strip_prefix("--mcp-config="))
        .unwrap_or_else(|| panic!("no --mcp-config in {argv:?}"));
    serde_json::from_str::<serde_json::Value>(json).unwrap()["mcpServers"]["tirith_browser"].clone()
}

#[test]
fn browser_mcp_for_every_agent() {
    let lane = |app: &crate::app::Shared, alias: &str| {
        crate::models::lane_argv(
            app,
            Some(alias),
            None,
            std::path::Path::new("/tmp/brief.md"),
            std::path::Path::new("/tmp"),
            None,
        )
        .unwrap()
        .argv
    };
    // Muse, as before: a lane isolated, a bot on the persistent profile.
    let app = browser_home("every", false);
    assert_eq!(browser_of(&lane(&app, "muse"))["args"][1], "--isolated");
    let bot = super::muse_argv(
        "muse",
        false,
        (None, None),
        "/w",
        None,
        "/w/p.txt",
        &crate::models::browser_args_for(&app, "muse", false),
    );
    assert_eq!(
        browser_of(&bot)["args"].as_array().map(|a| a.len()),
        Some(1)
    );
    // Codex, with no ~/.codex profile anywhere in its config: a lane carries the launcher with --isolated in the -c TOML, a bot's turn carries it bare (persistent profile), and no --profile rides along.
    let argv = lane(&app, "codex");
    let server = codex_browser_of(&argv);
    assert!(
        server["command"]
            .as_str()
            .is_some_and(|c| c.ends_with("node")),
        "node runs it: {server}"
    );
    assert!(
        server["args"][0]
            .as_str()
            .is_some_and(|a| a.ends_with("tools/browser/launch.mjs")),
        "the repo's launcher: {server}"
    );
    assert_eq!(
        server["args"][1].as_str(),
        Some("--isolated"),
        "a lane never shares or keeps a profile: {server}"
    );
    assert_eq!(
        server["env"]["TIRITH_HOME"].as_str(),
        Some(app.home.display().to_string().as_str()),
        "its home rides in the server env: {server}"
    );
    assert!(
        !argv.iter().any(|a| a == "--profile"),
        "no profile is needed or passed: {argv:?}"
    );
    let cfg = app.cfg();
    let bot = super::codex_argv(
        "codex",
        false,
        (None, None),
        &cfg,
        "be a bot",
        Some("t-1"),
        &crate::models::browser_args_for(&app, "codex", false),
    );
    let server = codex_browser_of(&bot);
    assert_eq!(
        server["args"].as_array().map(|a| a.len()),
        Some(1),
        "a bot gets the persistent profile, no mode flag: {server}"
    );
    assert!(
        bot.iter()
            .position(|a| a.starts_with("mcp_servers."))
            .unwrap()
            < bot.iter().position(|a| a == "resume").unwrap(),
        "the -c pair configures exec, not resume: {bot:?}"
    );
    // The anthropic agent: a lane isolated, a bot's chat session on the persistent profile, the browser tools allowed both times.
    let server = anthropic_browser_of(&lane(&app, "anthropic"));
    assert_eq!(
        server["args"][1], "--isolated",
        "a lane is isolated: {server}"
    );
    assert!(
        server["command"]
            .as_str()
            .is_some_and(|c| c.ends_with("node"))
            && server["args"][0]
                .as_str()
                .is_some_and(|a| a.ends_with("tools/browser/launch.mjs")),
        "the same launcher: {server}"
    );
    assert_eq!(
        server["env"]["TIRITH_HOME"],
        app.home.display().to_string(),
        "its home rides in the server env: {server}"
    );
    let bot = crate::models::chat_argv(&app, "manager", None)
        .unwrap()
        .argv; // the manager role runs on the anthropic agent
    let server = anthropic_browser_of(&bot);
    assert_eq!(
        server["args"].as_array().map(|a| a.len()),
        Some(1),
        "a bot gets the persistent profile, no mode flag: {server}"
    );
    // use = "mine": bots of Codex and anthropic drive the owner's Edge; lanes of either still never do.
    let app = browser_home("everymine", true);
    let cfg = app.cfg();
    let bot = super::codex_argv(
        "codex",
        false,
        (None, None),
        &cfg,
        "be a bot",
        None,
        &crate::models::browser_args_for(&app, "codex", false),
    );
    assert_eq!(
        codex_browser_of(&bot)["args"][1].as_str(),
        Some("--extension"),
        "a Codex bot with use = \"mine\" drives the owner's Edge"
    );
    let bot = crate::models::chat_argv(&app, "manager", None)
        .unwrap()
        .argv;
    assert_eq!(
        anthropic_browser_of(&bot)["args"][1],
        "--extension",
        "an anthropic bot with use = \"mine\" drives the owner's Edge"
    );
    for alias in ["codex", "anthropic"] {
        let argv = lane(&app, alias);
        assert!(
            !argv.iter().any(|a| a.contains("--extension")),
            "no lane ever gets the owner's browser: {argv:?}"
        );
    }
    assert_eq!(
        codex_browser_of(&lane(&app, "codex"))["args"][1].as_str(),
        Some("--isolated")
    );
    assert_eq!(
        anthropic_browser_of(&lane(&app, "anthropic"))["args"][1],
        "--isolated"
    );
}

/// An app like `browser_home` whose config turns the network sandbox off, so its agents run outside it.
fn browser_home_unsandboxed(tag: &str) -> crate::app::Shared {
    let dir = std::env::temp_dir().join(format!("tirith-browser-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("config.toml"),
        "# the owner's\n[agents.muse]\ncommand = \"muse\"\n[agents.codex]\ncommand = \"codex\"\n[agents.anthropic]\ncommand = \"anthropic-agent\"\n[network]\nsandbox = false\n[[projects]]\nid = \"p1\"\nrepo = \"/tmp\"\n",
    )
    .unwrap();
    let app = crate::app::App::open_at(dir);
    assert_eq!(
        app.cfg().error,
        None,
        "config with [network] must not read as unknown keys"
    );
    app
}

#[test]
#[ignore = "a test of a sandbox: not run, by the owner's order"]
fn browser_mcp_env_reaches_the_sandbox() {
    let lane = |app: &crate::app::Shared, alias: &str| {
        crate::models::lane_argv(
            app,
            Some(alias),
            None,
            std::path::Path::new("/tmp/brief.md"),
            std::path::Path::new("/tmp"),
            None,
        )
        .unwrap()
        .argv
    };
    let proxy = ["HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY", "NO_PROXY"];
    // Inside the sandbox (the default config): every agent's server env names TIRITH_SANDBOXED and the four proxy variables, forwarded by name — Muse and anthropic as a whole-value ${VAR}, Codex in env_vars — so the launcher passes --no-sandbox and Edge leaves through NeoBot's proxy.
    let app = browser_home("sandboxed", false);
    let env = browser_of(&lane(&app, "muse"))["env"].clone();
    assert_eq!(
        env["TIRITH_SANDBOXED"], "${TIRITH_SANDBOXED}",
        "muse takes the flag from its own env: {env}"
    );
    for k in proxy {
        assert_eq!(
            env[k],
            format!("${{{k}}}"),
            "muse forwards {k} by name: {env}"
        );
    }
    let server = codex_browser_of(&lane(&app, "codex"));
    assert_eq!(
        server["env"]["TIRITH_SANDBOXED"].as_str(),
        Some("1"),
        "codex gets the flag as a value: {server}"
    );
    assert_eq!(
        server["env_vars"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>(),
        proxy.to_vec(),
        "codex forwards the proxies by name: {server}"
    );
    let env = anthropic_browser_of(&lane(&app, "anthropic"))["env"].clone();
    assert_eq!(
        env["TIRITH_SANDBOXED"], "1",
        "anthropic gets the flag as a value: {env}"
    );
    for k in proxy {
        assert_eq!(
            env[k],
            format!("${{{k}}}"),
            "anthropic forwards {k} by name: {env}"
        );
    }
    // Outside the sandbox ([network] sandbox = false): no sandbox flag and no proxy forwarding, for any of the three agents.
    let app = browser_home_unsandboxed("outside");
    let env = browser_of(&lane(&app, "muse"))["env"].clone();
    assert!(
        env.get("TIRITH_SANDBOXED").is_none() && proxy.iter().all(|k| env.get(*k).is_none()),
        "muse outside the sandbox names neither: {env}"
    );
    let server = codex_browser_of(&lane(&app, "codex"));
    assert!(
        server["env"]
            .as_table()
            .is_some_and(|t| !t.contains_key("TIRITH_SANDBOXED"))
            && server
                .as_table()
                .is_some_and(|t| !t.contains_key("env_vars")),
        "codex outside the sandbox names neither: {server}"
    );
    let env = anthropic_browser_of(&lane(&app, "anthropic"))["env"].clone();
    assert!(
        env.get("TIRITH_SANDBOXED").is_none() && proxy.iter().all(|k| env.get(*k).is_none()),
        "anthropic outside the sandbox names neither: {env}"
    );
}

#[test]
fn muse_read_only_turn_is_read_only_and_fresh_without_a_session() {
    let a = super::muse_argv("muse", true, (None, None), "/w", None, "/w/p.txt", &[]);
    let has = |x: &str| a.iter().any(|y| y == x);
    assert!(has("--disable-write"), "{a:?}");
    assert!(
        !has("--session-id") && !has("--model") && !has("--reasoning-effort"),
        "{a:?}"
    );
}

#[test]
fn copilot_chat_turn_is_an_exec_run() {
    use super::super::{copilot::copilot_argv, kind::Exec, scratch::Scratch};
    let usage = std::path::Path::new("/w/u.json");
    let a = copilot_argv(
        "copilot",
        true,
        (Some("m"), Some("low")),
        Some("s-1"),
        usage,
        &[],
    );
    let has = |x: &str| a.iter().any(|y| y == x);
    assert!(
        has("--output-format") && has("--no-ask-user") && has("--usage-output-file"),
        "{a:?}"
    );
    assert!(
        has("--resume=s-1") && has("--deny-tool=write") && has("--model"),
        "{a:?}"
    );
    assert!(
        !has("--input-format") && !has("--append-system-prompt") && !has("-p"),
        "{a:?}"
    );
    assert_eq!(Exec::of("copilot"), Some(Exec::Copilot));
    assert_eq!(Exec::of("anthropic"), None);
    // two turns of one role at the same time never share a file
    let (mut s, dir) = (Scratch::default(), std::env::temp_dir());
    assert_ne!(s.path(&dir, "prompt.txt"), s.path(&dir, "prompt.txt"));
}

#[test]
fn a_github_token_of_the_new_kind_is_redacted() {
    let shown = crate::app::redact_secrets("sent github_pat_11ABCDEFG0madeupmadeupmadeup to it");
    assert!(!shown.contains("madeup"), "{shown}");
}

#[test]
fn copilots_auto_is_never_told_an_effort() {
    let told = |model| crate::codex::effort_of(model, Some("low"));
    assert_eq!(
        told(Some("auto")),
        None,
        "a free plan's only model refuses one"
    );
    assert_eq!(told(None), None, "none named is auto there");
    assert_eq!(told(Some("gpt-5.6-luna")), Some("low"));
}

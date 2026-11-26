use super::{profile::PROFILE, safari::script::safari_script};

#[test]
#[ignore = "a test of a sandbox: not run, by the owner's order"]
fn safari_commands_only_for_chat_roles() {
    // A fresh home with one of the owner's bots (a chat role extending the manager, as config.toml writes them).
    let dir = std::env::temp_dir().join(format!("tirith-safari-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("config.toml"), "[roles.bot-test]\nextends = \"manager\"\nscope = \"own\"\nname = \"Test\"\nprompt = [\"prompts/bot.md\"]\n").unwrap();
    let app = crate::app::App::open_at(dir);
    assert_eq!(app.cfg().error, None, "the bot's role must read as valid");
    // Nobody sends Apple Events from inside the sandbox any more (macOS refuses them there anyway).
    assert!(
        !PROFILE.contains("Safari") && PROFILE.contains("(deny appleevent-send)"),
        "the profile has no Safari exception:\n{PROFILE}"
    );
    // A lane gets an error, whatever role it names; so do the CEO, a lane worker, the judge and no role at all.
    for (lane, role) in [
        ("a-lane-id", "manager"),
        ("a-lane-id", "bot-test"),
        ("", "ceo"),
        ("", "worker"),
        ("", "judge"),
        ("", ""),
    ] {
        let err = safari_script(&app, lane, role, "read", "").unwrap_err();
        assert!(
            err.contains("managers and bots"),
            "lane={lane:?} role={role:?} is refused as not a chat role: {err}"
        );
    }
    // For a bot (and the managers), the fixed AppleScript the daemon would run, without opening Safari.
    for role in ["bot-test", "manager", "manager-general"] {
        let read = safari_script(&app, "", role, "read", "").unwrap();
        assert_eq!(read, "tell application \"Safari\" to return (URL of current tab of front window) & linefeed & (name of current tab of front window) & linefeed & (text of current tab of front window)", "{role} reads the current tab");
        let tab = safari_script(&app, "", role, "read", "https://mail.example/").unwrap();
        assert!(tab.contains("if (URL of t) starts with \"https://mail.example/\" then return (URL of t) & linefeed & (name of t) & linefeed & (text of t)"), "{role} reads the tab whose URL starts with the one given:\n{tab}");
        let open = safari_script(&app, "", role, "open", "https://example.com/x").unwrap();
        assert_eq!(open, "tell application \"Safari\"\nactivate\nopen location \"https://example.com/x\"\nend tell", "{role} opens a page");
    }
    // Only a URL rides in, escaped: a quote in it cannot end the string and start the agent's own script.
    let evil = safari_script(
        &app,
        "",
        "bot-test",
        "open",
        "https://x/\" & (do shell script \"id\") & \"",
    )
    .unwrap();
    assert!(
        evil.contains("open location \"https://x/\\\" & (do shell script \\\"id\\\") & \\\"\""),
        "the URL is escaped into the fixed script:\n{evil}"
    );
    // And the command's own shape is checked.
    assert!(
        safari_script(&app, "", "bot-test", "open", "").is_err(),
        "open needs a URL"
    );
    assert!(
        safari_script(&app, "", "bot-test", "eval", "1").is_err(),
        "only open and read exist"
    );
    assert!(
        safari_script(&app, "", "bot-test", "read", "https://x/\n").is_err(),
        "no control characters in a URL"
    );
}

#[test]
#[ignore = "a test of a sandbox: not run, by the owner's order"]
fn the_proxy_is_no_way_round_to_the_daemon() {
    let own = |at: &str| super::proxy::own_door(&[at.parse().unwrap()], 7717);
    for at in [
        "127.0.0.1:7717",
        "127.9.9.9:7717",
        "0.0.0.0:7717",
        "[::1]:7717",
        "[::ffff:127.0.0.1]:7717",
    ] {
        assert!(own(at), "{at}");
    }
    assert!(!own("127.0.0.1:7718") && !own("93.184.216.34:7717"));
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "a test of a sandbox: not run, by the owner's order"]
fn the_kernel_tells_a_confined_caller_from_the_owner() {
    let me = std::process::id() as libc::pid_t;
    if super::mac::ours(me) {
        return; // the tests themselves run in NeoBot's sandbox, and sandboxes do not nest
    }
    let mut child = std::process::Command::new(super::profile::SANDBOX_EXEC)
        .args(["-p", PROFILE, "/bin/sleep", "5"])
        .spawn()
        .unwrap();
    let pid = child.id() as libc::pid_t;
    let seen = (0..100).any(|_| {
        std::thread::sleep(std::time::Duration::from_millis(20));
        super::mac::ours(pid)
    });
    let _ = (child.kill(), child.wait());
    assert!(seen, "a process under the profile");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let to = listener.local_addr().unwrap();
    let here = std::net::TcpStream::connect(to).unwrap();
    let from = here.local_addr().unwrap();
    let began = std::time::Instant::now();
    assert!(
        !super::confined(from, to),
        "this test is no confined caller"
    );
    eprintln!("who holds a connection: {:?}", began.elapsed());
    assert!(
        super::confined("127.0.0.1:1".parse().unwrap(), to),
        "a caller nobody holds is never the owner"
    );
}

/// Where the daemon's code starts git itself; elsewhere it must ask `sandbox::git`.
fn bare_git(dir: &std::path::Path, found: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            bare_git(&path, found);
        } else if path.extension().is_some_and(|e| e == "rs") && !path.ends_with("sandbox/git.rs") {
            let text = std::fs::read_to_string(&path).unwrap();
            if text.contains(concat!("Command::new(", "\"git\")")) {
                found.push(path.display().to_string());
            }
        }
    }
}

#[test]
#[ignore = "a test of a sandbox: not run, by the owner's order"]
fn the_daemon_starts_git_only_through_the_sandbox() {
    let mut found = vec![];
    bare_git(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut found,
    );
    assert!(
        found.is_empty(),
        "an agent's repository would run code outside the sandbox: {found:?}"
    );
}

#[test]
fn the_profile_hides_every_auth_file_and_the_folder_they_are_made_in() {
    let home = std::env::temp_dir().join(format!("tirith-hide-{}", std::process::id()));
    std::fs::create_dir_all(home.join("auth")).unwrap();
    std::fs::write(
        home.join("auth/copilot.env"),
        "COPILOT_PROVIDER_API_KEY=x\n",
    )
    .unwrap();
    let kept = crate::app::kept(&home);
    assert!(kept.contains(&home.join("auth/copilot.env")), "{kept:?}");
    let rules = super::hidden::rules(&kept) + &super::hidden::folder(&home.join("auth"));
    assert!(
        rules.contains("(literal") && rules.contains("copilot.env"),
        "{rules}"
    );
    assert!(
        rules.contains("(deny file-read* file-write* file-link file-clone (subpath"),
        "{rules}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

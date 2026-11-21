use super::{profile::PROFILE, safari::script::safari_script};

#[test]
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

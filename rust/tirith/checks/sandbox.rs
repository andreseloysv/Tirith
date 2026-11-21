use super::*;

#[test]
fn chat_roles_may_script_safari() {
    // A fresh home with one of the owner's bots (a chat role extending the manager, as config.toml writes them).
    let dir = std::env::temp_dir().join(format!("tirith-safari-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("config.toml"), "[roles.bot-test]\nextends = \"manager\"\nscope = \"own\"\nname = \"Test\"\nprompt = [\"prompts/bot.md\"]\n").unwrap();
    let app = crate::app::App::open_at(dir);
    assert_eq!(app.cfg().error, None, "the bot's role must read as valid");
    let allow = "(allow appleevent-send (appleevent-destination \"com.apple.Safari\"))";
    for who in ["bot-test", "manager", "manager-general"] {
        let p = profile(&app, who);
        assert!(p.contains(allow), "{who} is a chat role and may script Safari:\n{p}");
        assert_eq!(p.matches("appleevent-destination").count(), 1, "{who} may reach Safari alone:\n{p}");
        assert!(p.find("(deny appleevent-send)") < p.find(allow), "{who}: the Safari allow must come after the general deny:\n{p}");
    }
    for who in ["a-lane-id", "merge:a-lane-id", "ceo", "judge", "worker"] {
        let p = profile(&app, who);
        assert!(!p.contains("Safari") && p.contains("(deny appleevent-send)"), "{who} may not send Apple Events:\n{p}");
    }
}

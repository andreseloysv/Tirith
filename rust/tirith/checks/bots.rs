use chrono::{Local, TimeZone};
use serde_json::json;

fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> chrono::DateTime<Local> {
    Local.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
}

#[test]
fn days_from_words_and_lists() {
    assert_eq!(super::days(&json!("weekdays")), Some(vec![1, 2, 3, 4, 5]));
    assert_eq!(super::days(&json!("daily")), Some(vec![1, 2, 3, 4, 5, 6, 7]));
    assert_eq!(super::days(&json!("weekends")), Some(vec![6, 7]));
    assert_eq!(super::days(&json!("Mon, wed,FRI")), Some(vec![1, 3, 5]));
    assert_eq!(super::days(&json!([2, 9, 4])), Some(vec![2, 4]));
    assert_eq!(super::days(&json!(null)), None);
}

#[test]
fn a_routine_is_due_once_a_day_after_its_time() {
    // 2026-11-23 is a Monday.
    let last = |t: chrono::DateTime<Local>| t.with_timezone(&chrono::Utc).format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let r = |l: &str, on: bool| json!({"at": "08:00", "days": [1, 2, 3, 4, 5], "on": on, "last": l});
    let fri = last(at(2026, 11, 20, 9, 0));
    assert!(!super::due(&r(&fri, true), at(2026, 11, 23, 7, 59)), "before its time");
    assert!(super::due(&r(&fri, true), at(2026, 11, 23, 8, 0)), "at its time");
    assert!(super::due(&r(&fri, true), at(2026, 11, 23, 13, 0)), "later the same day, when the daemon was down at 8");
    assert!(!super::due(&r(&last(at(2026, 11, 23, 8, 0)), true), at(2026, 11, 23, 8, 1)), "it ran today");
    assert!(!super::due(&r(&fri, false), at(2026, 11, 23, 9, 0)), "switched off");
    assert!(!super::due(&r(&fri, true), at(2026, 11, 22, 9, 0)), "Sunday is not one of its days");
    assert!(!super::due(&r(&last(at(2026, 11, 23, 9, 30)), true), at(2026, 11, 23, 10, 0)), "made after its time today: tomorrow");
    assert!(super::due(&json!({"at": "08:00", "days": [1]}), at(2026, 11, 23, 8, 0)), "never ran");
}

#[test]
fn bots_live_in_config_as_own_roles_with_schedule_routes() {
    // A bot in the page's shape, with a switched-off routine: written into a config.toml with the owner's comments.
    let bots = json!([{"id": "abc123", "name": "Noticias", "shape": "drop", "color": "#22d3ee", "routines": [{"id": "r111111", "title": "News", "prompt": "Summarise the news", "days": [1, 2, 3, 4, 5], "at": "08:00", "on": false}]}]);
    let text = super::written("# the owner's own comment\n[server]\nport = 7799 # keep this too\n", bots.as_array().unwrap()).unwrap();
    assert!(text.contains("# the owner's own comment") && text.contains("# keep this too"), "the file's comments stay:\n{text}");
    assert!(text.contains("[roles.bot-abc123]") && text.contains("scope = \"own\"") && text.contains("Noticias"), "the bot is a role:\n{text}");
    assert!(text.contains("on = \"schedule\"") && text.contains("to = \"bot-abc123\"") && text.contains("Summarise the news") && text.contains("enabled = false"), "the routine is a schedule route:\n{text}");
    // Read back as `all` does, with the routine's last run from its stamp.
    let read = |text: &str| super::bots_of(&serde_json::to_value(&text.parse::<toml::Table>().unwrap()).unwrap(), &|id, rid| if (id, rid) == ("abc123", "r111111") { "2026-11-20T08:00:00Z".into() } else { String::new() });
    let bots = read(&text);
    assert_eq!(bots.len(), 1);
    assert_eq!((bots[0]["name"].as_str(), bots[0]["shape"].as_str(), bots[0]["color"].as_str()), (Some("Noticias"), Some("drop"), Some("#22d3ee")));
    assert_eq!((bots[0]["routines"][0]["title"].as_str(), bots[0]["routines"][0]["on"].as_bool(), bots[0]["routines"][0]["last"].as_str()), (Some("News"), Some(false), Some("2026-11-20T08:00:00Z")));
    // The factory resolves the role over that config: a chat session of its own, the bot prompt with its name, its schedule routed to it.
    let prompts: std::collections::BTreeMap<String, String> = [
        ("manager", include_str!("../../../factory/prompts/manager.md")),
        ("one-project", include_str!("../../../factory/prompts/one-project.md")),
        ("no-project", include_str!("../../../factory/prompts/no-project.md")),
        ("brief", include_str!("../../../factory/prompts/brief.md")),
        ("reviewer", include_str!("../../../factory/prompts/reviewer.md")),
        ("bot", include_str!("../../../factory/prompts/bot.md")),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    let f = crate::factory::parse(include_str!("../../../factory/factory.toml"), prompts, &text.parse::<toml::Table>().unwrap()).unwrap();
    assert_eq!((f.role::<String>("bot-abc123", "session"), f.role::<String>("bot-abc123", "scope")), (Some("chat".into()), Some("own".into())));
    assert!(f.prompts_of("bot-abc123").starts_with("You are Noticias"), "its prompt carries its name");
    assert!(f.routes_on("schedule").iter().any(|r| r.0 == "bot-abc123"), "its routine routes to it");
    // Written again with a rename and the routine on: still one role and one route, the comments still there; written with no bots, both go.
    let mut renamed = bots.clone();
    (renamed[0]["name"], renamed[0]["routines"][0]["on"]) = (json!("Renamed"), json!(true));
    let text = super::written(&text, &renamed).unwrap();
    assert_eq!((text.matches("[roles.bot-abc123]").count(), text.matches("to = \"bot-abc123\"").count()), (1, 1), "no duplicates:\n{text}");
    assert!(text.contains("# the owner's own comment") && read(&text)[0]["name"] == "Renamed" && read(&text)[0]["routines"][0]["on"] == true);
    let text = super::written(&text, &[]).unwrap();
    assert!(read(&text).is_empty() && !text.contains("bot-abc123") && text.contains("# the owner's own comment"), "a deleted bot goes, with its routes:\n{text}");
}

#[test]
fn bots_write_only_in_their_own_folders_and_the_shared_skills() {
    let home = std::path::Path::new("/Users/o/.tirith");
    for ok in ["manager/bot-ab12cd/memory.md", "manager/bot-ab12cd/.agents/skills/jobs/SKILL.md", "manager/bot-ab12cd/.codex/skills/jobs/SKILL.md", "bots/skills/report/SKILL.md", "manager/bot-ab12cd/.seen"] {
        assert!(super::writable(home, &home.join(ok)), "{ok}");
    }
    for no in ["manager/tirith/notes.md", "manager/memory.md", "config.toml", "tirith.sqlite3", "bots/list.json", "manager/bot-ab12cd/.codex/config.toml", "manager/bot-ab12cd/.muse/settings.json", "manager/bot-ab12cd/notes/.cfg/x"] {
        assert!(!super::writable(home, &home.join(no)), "{no}");
    }
}

#[test]
fn skills_list_the_folders_with_a_skill_and_the_shared_links_only_when_asked() {
    let root = std::env::temp_dir().join(format!("tirith-skills-{}", std::process::id()));
    let (own, common) = (root.join("bot").join(super::SKILLS), root.join("shared"));
    for (dir, text) in [(own.join("jobs"), "own"), (common.join("report"), "shared")] {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), text).unwrap();
    }
    std::fs::create_dir_all(own.join("half-written")).unwrap(); // no SKILL.md: not a skill
    std::os::unix::fs::symlink(common.join("report"), own.join("report")).unwrap();
    let names = |links| super::skills(&own, links).into_iter().map(|s| s.0).collect::<Vec<_>>();
    assert_eq!(names(false), ["jobs"]);
    assert_eq!(names(true), ["jobs", "report"]);
    assert_eq!(super::skills(&own, true)[1].1, "shared");
    let _ = std::fs::remove_dir_all(&root);
}

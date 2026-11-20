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

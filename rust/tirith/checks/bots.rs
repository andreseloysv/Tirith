use chrono::{Local, TimeZone};
use serde_json::json;

fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> chrono::DateTime<Local> {
    Local.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
}

#[test]
fn days_from_words_and_lists() {
    assert_eq!(
        super::schedule::days(&json!("weekdays")),
        Some(vec![1, 2, 3, 4, 5])
    );
    assert_eq!(
        super::schedule::days(&json!("daily")),
        Some(vec![1, 2, 3, 4, 5, 6, 7])
    );
    assert_eq!(super::schedule::days(&json!("weekends")), Some(vec![6, 7]));
    assert_eq!(
        super::schedule::days(&json!("Mon, wed,FRI")),
        Some(vec![1, 3, 5])
    );
    assert_eq!(super::schedule::days(&json!([2, 9, 4])), Some(vec![2, 4]));
    assert_eq!(super::schedule::days(&json!(null)), None);
}

#[test]
fn a_routine_is_due_once_a_day_after_its_time() {
    // 2026-11-23 is a Monday.
    let last = |t: chrono::DateTime<Local>| {
        t.with_timezone(&chrono::Utc)
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string()
    };
    let r =
        |l: &str, on: bool| json!({"at": "08:00", "days": [1, 2, 3, 4, 5], "on": on, "last": l});
    let fri = last(at(2026, 11, 20, 9, 0));
    assert!(
        !super::due(&r(&fri, true), at(2026, 11, 23, 7, 59)),
        "before its time"
    );
    assert!(
        super::due(&r(&fri, true), at(2026, 11, 23, 8, 0)),
        "at its time"
    );
    assert!(
        super::due(&r(&fri, true), at(2026, 11, 23, 13, 0)),
        "later the same day, when the daemon was down at 8"
    );
    assert!(
        !super::due(
            &r(&last(at(2026, 11, 23, 8, 0)), true),
            at(2026, 11, 23, 8, 1)
        ),
        "it ran today"
    );
    assert!(
        !super::due(&r(&fri, false), at(2026, 11, 23, 9, 0)),
        "switched off"
    );
    assert!(
        !super::due(&r(&fri, true), at(2026, 11, 22, 9, 0)),
        "Sunday is not one of its days"
    );
    assert!(
        !super::due(
            &r(&last(at(2026, 11, 23, 9, 30)), true),
            at(2026, 11, 23, 10, 0)
        ),
        "made after its time today: tomorrow"
    );
    assert!(
        super::due(&json!({"at": "08:00", "days": [1]}), at(2026, 11, 23, 8, 0)),
        "never ran"
    );
}

#[test]
fn routines_catch_up_once() {
    let stamp = |t: chrono::DateTime<Local>| t.with_timezone(&chrono::Utc).to_rfc3339();
    let r = |catch_up: &str, days: &[u64], last: &str| json!({"at": "08:00", "days": days, "catch_up": catch_up, "last": last});
    let (daily, ran) = ([1, 2, 3, 4, 5, 6, 7], stamp(at(2026, 11, 18, 8, 0)));
    // Every 30 s from `back` to `until`, as the scheduler does: each run with the delay it tells.
    let runs = |mut r: serde_json::Value, back, until| {
        let (mut now, mut ran): (chrono::DateTime<Local>, Vec<_>) = (back, vec![]);
        while now <= until {
            if super::due(&r, now) {
                ran.push((now, super::schedule::late(&r, now)));
                r["last"] = json!(stamp(now));
            }
            now += chrono::TimeDelta::seconds(30);
        }
        ran
    };

    // Down two days, back on Friday at 10:15: one run, saying how late; with skip, none.
    let (back, night) = (at(2026, 11, 20, 10, 15), at(2026, 11, 20, 23, 59));
    let late = ", 2 h 15 min late (it was for Fri 2026-11-20 08:00)".to_string();
    assert_eq!(
        runs(r("once", &daily, &ran), back, night),
        [(back, late.clone())]
    );
    assert!(runs(r("skip", &daily, &ran), back, night).is_empty());
    let mut unset = r("once", &daily, &ran);
    unset.as_object_mut().unwrap().remove("catch_up");
    assert_eq!(
        runs(unset, back, night),
        [(back, late)],
        "once is the default"
    );

    // Back before that day's time: the missed run at once, then the day's own; with skip only the day's own.
    let (early, eight) = (at(2026, 11, 20, 6, 0), at(2026, 11, 20, 8, 0));
    let missed = ", 22 h 0 min late (it was for Thu 2026-11-19 08:00)".to_string();
    assert_eq!(
        runs(r("once", &daily, &ran), early, night),
        [(early, missed), (eight, String::new())]
    );
    assert_eq!(
        runs(r("skip", &daily, &ran), early, night),
        [(eight, String::new())]
    );

    // A Monday routine missed is caught up on a day that is not its own, even weeks after its last run.
    let old = stamp(at(2026, 11, 2, 8, 0));
    let wed = at(2026, 11, 18, 10, 0);
    assert_eq!(
        runs(r("once", &[1], &old), wed, night),
        [(
            wed,
            ", 2 d 2 h late (it was for Mon 2026-11-16 08:00)".to_string()
        )]
    );
    assert!(runs(r("skip", &[1], &old), wed, night).is_empty());

    // Nothing is missed when none of its days passed, it is off, or it never ran and today's time has not come.
    let monday = stamp(at(2026, 11, 16, 8, 0));
    assert!(!super::due(&r("once", &[1], &monday), wed));
    assert!(!super::due(&r("once", &daily, ""), early));
    let mut off = r("once", &daily, &ran);
    off["on"] = json!(false);
    assert!(!super::due(&off, back));

    // With skip, a tick a little behind is still on time.
    assert!(super::due(&r("skip", &daily, &ran), at(2026, 11, 20, 8, 5)));
    assert!(!super::due(
        &r("skip", &daily, &ran),
        at(2026, 11, 20, 8, 6)
    ));
}

#[test]
fn catch_up_lives_in_config() {
    let saved = |catch_up: &str| {
        let bots = json!([{"id": "abc123", "name": "N", "shape": "drop", "color": "#22d3ee", "routines": [{"id": "r1", "title": "T", "prompt": "P", "days": [1], "at": "08:00", "catch_up": catch_up}]}]);
        let text = super::written::written("", bots.as_array().unwrap()).unwrap();
        let table = text.parse::<toml::Table>().unwrap();
        let read = super::list::bots_of(&json!(table), &|_, _| String::new());
        assert_eq!(read[0]["routines"][0]["catch_up"], catch_up, "{text}");
        text
    };
    assert!(
        !saved("once").contains("catch_up"),
        "the default is not written"
    );
    let parse = |text: &str| {
        crate::factory::parse(
            crate::factory::TEXT,
            crate::factory::embedded_prompts(),
            &text.parse().unwrap(),
        )
    };
    let text = saved("skip");
    let f = parse(&text).unwrap();
    assert!(f
        .schedules()
        .iter()
        .any(|s| s.0 == "bot-abc123" && s.3 == "skip"));
    let bad = parse(&text.replace("\"skip\"", "\"twice\"")).err();
    assert!(bad.unwrap_or_default().contains("catch_up"));
}

#[test]
fn bots_live_in_config_as_own_roles_with_schedule_routes() {
    // A bot in the page's shape, with a switched-off routine: written into a config.toml with the owner's comments.
    let bots = json!([{"id": "abc123", "name": "Noticias", "shape": "drop", "color": "#22d3ee", "routines": [{"id": "r111111", "title": "News", "prompt": "Summarise the news", "days": [1, 2, 3, 4, 5], "at": "08:00", "on": false}]}]);
    let text = super::written::written(
        "# the owner's own comment\n[server]\nport = 7799 # keep this too\n",
        bots.as_array().unwrap(),
    )
    .unwrap();
    assert!(
        text.contains("# the owner's own comment") && text.contains("# keep this too"),
        "the file's comments stay:\n{text}"
    );
    assert!(
        text.contains("[roles.bot-abc123]")
            && text.contains("scope = \"own\"")
            && text.contains("Noticias"),
        "the bot is a role:\n{text}"
    );
    assert!(
        text.contains("on = \"schedule\"")
            && text.contains("to = \"bot-abc123\"")
            && text.contains("Summarise the news")
            && text.contains("enabled = false"),
        "the routine is a schedule route:\n{text}"
    );
    // Read back as `all` does, with the routine's last run from its stamp.
    let read = |text: &str| {
        super::list::bots_of(
            &serde_json::to_value(&text.parse::<toml::Table>().unwrap()).unwrap(),
            &|id, rid| {
                if (id, rid) == ("abc123", "r111111") {
                    "2026-11-20T08:00:00Z".into()
                } else {
                    String::new()
                }
            },
        )
    };
    let bots = read(&text);
    assert_eq!(bots.len(), 1);
    assert_eq!(
        (
            bots[0]["name"].as_str(),
            bots[0]["shape"].as_str(),
            bots[0]["color"].as_str()
        ),
        (Some("Noticias"), Some("drop"), Some("#22d3ee"))
    );
    assert_eq!(
        (
            bots[0]["routines"][0]["title"].as_str(),
            bots[0]["routines"][0]["on"].as_bool(),
            bots[0]["routines"][0]["last"].as_str()
        ),
        (Some("News"), Some(false), Some("2026-11-20T08:00:00Z"))
    );
    // The factory resolves the role over that config: a chat session of its own, the bot prompt with its name, its schedule routed to it.
    let prompts: std::collections::BTreeMap<String, String> = [
        (
            "manager",
            include_str!("../../../factory/prompts/manager.md"),
        ),
        (
            "one-project",
            include_str!("../../../factory/prompts/one-project.md"),
        ),
        (
            "no-project",
            include_str!("../../../factory/prompts/no-project.md"),
        ),
        ("brief", include_str!("../../../factory/prompts/brief.md")),
        ("ceo", include_str!("../../../factory/prompts/ceo.md")),
        ("coach", include_str!("../../../factory/prompts/coach.md")),
        ("bot", include_str!("../../../factory/prompts/bot.md")),
        (
            "owner-mood",
            include_str!("../../../factory/evals/owner-mood.md"),
        ),
        (
            "card-needed",
            include_str!("../../../factory/evals/card-needed.md"),
        ),
        (
            "turn-quality",
            include_str!("../../../factory/evals/turn-quality.md"),
        ),
        (
            "unapproved",
            include_str!("../../../factory/evals/unapproved.md"),
        ),
        (
            "ab-compare",
            include_str!("../../../factory/evals/ab-compare.md"),
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    let f = crate::factory::parse(
        include_str!("../../../factory/factory.toml"),
        prompts,
        &text.parse::<toml::Table>().unwrap(),
    )
    .unwrap();
    assert_eq!(
        (
            f.role::<String>("bot-abc123", "session"),
            f.role::<String>("bot-abc123", "scope")
        ),
        (Some("chat".into()), Some("own".into()))
    );
    assert!(
        f.prompts_of("bot-abc123").starts_with("You are Noticias"),
        "its prompt carries its name"
    );
    assert!(
        f.routes_on("schedule").iter().any(|r| r.0 == "bot-abc123"),
        "its routine routes to it"
    );
    // Written again with a rename and the routine on: still one role and one route, the comments still there; written with no bots, both go.
    let mut renamed = bots.clone();
    (renamed[0]["name"], renamed[0]["routines"][0]["on"]) = (json!("Renamed"), json!(true));
    let text = super::written::written(&text, &renamed).unwrap();
    assert_eq!(
        (
            text.matches("[roles.bot-abc123]").count(),
            text.matches("to = \"bot-abc123\"").count()
        ),
        (1, 1),
        "no duplicates:\n{text}"
    );
    assert!(
        text.contains("# the owner's own comment")
            && read(&text)[0]["name"] == "Renamed"
            && read(&text)[0]["routines"][0]["on"] == true
    );
    let text = super::written::written(&text, &[]).unwrap();
    assert!(
        read(&text).is_empty()
            && !text.contains("bot-abc123")
            && text.contains("# the owner's own comment"),
        "a deleted bot goes, with its routes:\n{text}"
    );
}

#[test]
fn bots_write_only_in_their_own_folders_and_the_shared_skills() {
    let home = std::path::Path::new("/Users/o/.tirith");
    for ok in [
        "manager/bot-ab12cd/memory.md",
        "manager/bot-ab12cd/.agents/skills/jobs/SKILL.md",
        "manager/bot-ab12cd/.codex/skills/jobs/SKILL.md",
        "bots/skills/report/SKILL.md",
        "manager/bot-ab12cd/.seen",
    ] {
        assert!(super::writable(home, &home.join(ok)), "{ok}");
    }
    for no in [
        "manager/tirith/notes.md",
        "manager/memory.md",
        "config.toml",
        "tirith.sqlite3",
        "bots/list.json",
        "manager/bot-ab12cd/.codex/config.toml",
        "manager/bot-ab12cd/.muse/settings.json",
        "manager/bot-ab12cd/notes/.cfg/x",
    ] {
        assert!(!super::writable(home, &home.join(no)), "{no}");
    }
}

#[test]
fn skills_list_the_folders_with_a_skill_and_the_shared_links_only_when_asked() {
    let root = std::env::temp_dir().join(format!("tirith-skills-{}", std::process::id()));
    let (own, common) = (
        root.join("bot").join(super::folder::SKILLS),
        root.join("shared"),
    );
    for (dir, text) in [(own.join("jobs"), "own"), (common.join("report"), "shared")] {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), text).unwrap();
    }
    std::fs::create_dir_all(own.join("half-written")).unwrap(); // no SKILL.md: not a skill
    std::os::unix::fs::symlink(common.join("report"), own.join("report")).unwrap();
    let names = |links| {
        super::folder::skills(&own, links)
            .into_iter()
            .map(|s| s.0)
            .collect::<Vec<_>>()
    };
    assert_eq!(names(false), ["jobs"]);
    assert_eq!(names(true), ["jobs", "report"]);
    assert_eq!(super::folder::skills(&own, true)[1].1, "shared");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn bots_written_at_the_end() {
    // A config.toml with several [[projects]], a bot's role, face and route between them, and an empty [roles] header at the end.
    let old = "# top comment\n[server]\nport = 7799\n\n[[projects]]\nname = \"a\" # keep\n\n[roles.bot-abc123]\nextends = \"manager\"\nscope = \"own\"\n\n[roles.bot-abc123.face]\nshape = \"drop\"\n\n[[routes]]\non = \"schedule\"\nto = \"bot-abc123\"\n\n[[projects]]\nname = \"b\"\n\n[roles]\n";
    let bots = json!([{"id": "abc123", "name": "N", "shape": "drop", "color": "#22d3ee", "routines": [{"id": "r1", "title": "T", "prompt": "P", "days": [1], "at": "08:00", "on": true}]}]);
    let text = super::written::written(old, bots.as_array().unwrap()).unwrap();
    let at = |s: &str| {
        text.find(s)
            .unwrap_or_else(|| panic!("{s} missing:\n{text}"))
    };
    assert!(
        at("[roles.bot-abc123]") > at("name = \"b\"")
            && at("[roles.bot-abc123.face]") > at("[roles.bot-abc123]")
            && at("[[routes]]") > at("[roles.bot-abc123.face]"),
        "bots after the owner's sections:\n{text}"
    );
    assert_eq!(
        (
            text.matches("[roles]").count(),
            text.matches("[[routes]]").count(),
            text.matches("[roles.bot-abc123]").count()
        ),
        (0, 1, 1),
        "no empty header:\n{text}"
    );
    assert!(
        text.contains("# top comment") && text.contains("name = \"a\" # keep"),
        "comments stay:\n{text}"
    );
    assert_eq!(
        super::written::written(&text, bots.as_array().unwrap()).unwrap(),
        text,
        "writing again changes nothing"
    );
    assert!(!super::written::written(&text, &[])
        .unwrap()
        .contains("bot-abc123"));
}

#[test]
fn a_system_bot_change_is_a_layer_in_config() {
    // The CEO renamed and re-faced on the page; the judge untouched; an owner's bot as ever.
    let old = "# keep\n[roles.ceo]\nmodel = \"keep-me\" # and this\n";
    let bots = json!([
        {"id": "ceo", "key": "ceo", "system": true, "dirty": true, "name": "Boss", "shape": "star", "color": "#e5484d", "show": true, "session": "chat", "access": "", "routes": [{"on": "lane.failed", "to": "ceo", "mode": "note", "chat": false, "phone": true}], "routines": []},
        {"id": "judge", "key": "judge", "system": true, "name": "Judge", "shape": "hex", "color": "#8c8c8c", "show": false, "routines": []},
        {"id": "abc123", "name": "N", "shape": "drop", "color": "#22d3ee", "routines": []}
    ]);
    let text = super::written::written(old, bots.as_array().unwrap()).unwrap();
    assert!(
        text.contains("model = \"keep-me\" # and this") && text.contains("# keep"),
        "the CEO's other config keys and comments stay:\n{text}"
    );
    assert!(
        text.contains("Boss") && text.contains("[roles.bot-abc123]"),
        "the change and the owner's bot are written:\n{text}"
    );
    assert!(
        !text.contains("judge"),
        "an untouched system bot leaves no layer:\n{text}"
    );
    assert_eq!(
        text.matches("[roles.ceo]").count(),
        1,
        "one CEO table, no duplicate:\n{text}"
    );
    // The layer wins over the factory, key by key, and system folders are writable like a bot's.
    let prompts: std::collections::BTreeMap<String, String> = [
        (
            "manager",
            include_str!("../../../factory/prompts/manager.md"),
        ),
        (
            "one-project",
            include_str!("../../../factory/prompts/one-project.md"),
        ),
        (
            "no-project",
            include_str!("../../../factory/prompts/no-project.md"),
        ),
        ("brief", include_str!("../../../factory/prompts/brief.md")),
        ("ceo", include_str!("../../../factory/prompts/ceo.md")),
        ("coach", include_str!("../../../factory/prompts/coach.md")),
        ("bot", include_str!("../../../factory/prompts/bot.md")),
        (
            "owner-mood",
            include_str!("../../../factory/evals/owner-mood.md"),
        ),
        (
            "card-needed",
            include_str!("../../../factory/evals/card-needed.md"),
        ),
        (
            "turn-quality",
            include_str!("../../../factory/evals/turn-quality.md"),
        ),
        (
            "unapproved",
            include_str!("../../../factory/evals/unapproved.md"),
        ),
        (
            "ab-compare",
            include_str!("../../../factory/evals/ab-compare.md"),
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    let f = crate::factory::parse(
        include_str!("../../../factory/factory.toml"),
        prompts,
        &text.parse::<toml::Table>().unwrap(),
    )
    .unwrap();
    assert_eq!(f.role::<String>("ceo", "name").as_deref(), Some("Boss"));
    assert_eq!(
        (
            f.role::<String>("ceo", "session").as_deref(),
            f.role::<String>("ceo", "access").as_deref()
        ),
        (Some("chat"), Some("")),
        "its session and access are the layer's, not the factory's turn/read"
    );
    assert_eq!(
        f.routes_on("lane.failed"),
        vec![
            ("manager".to_string(), "turn".to_string(), true, true, None),
            ("ceo".to_string(), "note".to_string(), false, true, None)
        ],
        "the changed CEO hears lane.failed as written"
    );
    assert!(
        f.routes_on("metric.red").iter().all(|r| r.0 != "ceo"),
        "and the factory's routes to it are replaced, not kept beside the layer's"
    );
    assert_eq!(
        f.role::<toml::Table>("ceo", "face")
            .and_then(|t| t.get("shape").and_then(|v| v.as_str()).map(String::from))
            .as_deref(),
        Some("star")
    );
    // With no [roles] in the file at all, the layer is a plain table at the end (an inline `roles = {…}` would make a later [roles.bot-…] header unparseable).
    let text =
        super::written::written("[server]\nport = 7799\n", &bots.as_array().unwrap()[..1]).unwrap();
    assert!(
        text.contains("[roles.ceo]")
            && !text.contains("roles = {")
            && text.parse::<toml::Table>().is_ok(),
        "a real table:\n{text}"
    );
    let home = std::path::Path::new("/Users/o/.tirith");
    assert!(
        super::writable(home, &home.join("manager/ceo/memory.md"))
            && super::writable(
                home,
                &home.join("manager/manager-general/.agents/skills/x/SKILL.md")
            )
    );
}

#[test]
fn saving_a_bot_keeps_a_pass_time_the_owner_wrote() {
    // A schedule route with no task is a turn role's pass, not one of its routines: the page has no row to write it back from.
    let old = "[[routes]]\non = \"schedule\"\nto = \"ceo\"\nmode = \"turn\"\ndays = [1, 3, 5]\nat = \"16:30\"\n\n[[routes]]\non = \"schedule\"\nto = \"ceo\"\nid = \"r1\"\ntitle = \"Regar\"\ntask = \"water\"\ndays = [2]\nat = \"08:00\"\n";
    let bots = json!([
        {"id": "ceo", "key": "ceo", "system": true, "name": "CEO", "routines": []},
        {"id": "abc123", "name": "N", "shape": "drop", "color": "#22d3ee", "routines": [{"id": "r2", "title": "Mirar", "prompt": "look", "days": [1], "at": "09:00"}]}
    ]);
    let text = super::written::written(old, bots.as_array().unwrap()).unwrap();
    assert!(
        text.contains("16:30"),
        "the CEO's pass time stays, or that pass is silently never due again:\n{text}"
    );
    assert!(
        !text.contains("08:00") && text.contains("09:00"),
        "its routines are still written again from the page:\n{text}"
    );
    let again = super::written::written(&text, bots.as_array().unwrap()).unwrap();
    assert_eq!(again.matches("16:30").count(), 1, "kept once:\n{again}");
}

const TITLES: [&str; 7] = [
    "Cuándo usarla",
    "Entradas y accesos",
    "Pasos",
    "Cómo validar",
    "Qué devuelve",
    "Qué requiere aprobación",
    "Si algo falla",
];

#[test]
fn skill_template_in_the_factory() {
    let template = include_str!("../../../factory/skill.md");
    assert!(
        template.starts_with("---\nname: {name}\ndescription: "),
        "its header:\n{template}"
    );
    let lines: Vec<&str> = template.lines().collect();
    for t in TITLES {
        let at = lines
            .iter()
            .position(|l| *l == format!("## {t}"))
            .unwrap_or_else(|| panic!("the section «{t}»:\n{template}"));
        assert!(
            lines
                .get(at + 1)
                .is_some_and(|l| !l.trim().is_empty() && !l.starts_with('#')),
            "a line says what goes in «{t}»"
        );
    }
    assert_eq!(lines.iter().filter(|l| l.starts_with("## ")).count(), 7);
    // A bot's prompt gets the template's text where bot.md has {skill_template}; the template keeps its own {name}.
    let text = "[roles.bot-x]\nextends = \"manager\"\nscope = \"own\"\nname = \"Ana\"\nprompt = [\"prompts/bot.md\"]\n";
    let f = crate::factory::parse(
        crate::factory::TEXT,
        crate::factory::embedded_prompts(),
        &text.parse().unwrap(),
    )
    .unwrap();
    let prompt = f.prompts_of("bot-x");
    assert!(
        include_str!("../../../factory/prompts/bot.md").contains("{skill_template}"),
        "bot.md asks for the template"
    );
    assert!(
        !prompt.contains("{skill_template}") && prompt.contains(template.trim()),
        "the bot's prompt holds the template:\n{prompt}"
    );
    assert!(prompt.contains("You are Ana"));
}

#[test]
fn notes_are_numbered_once() {
    let text = "# Owner\n- likes short answers\n- [m4] only remote jobs\n  * indented note\n\nSome prose.\n---\n**bold**\n-\n";
    let once = super::notes::numbered(text, 'm');
    assert_eq!(
        once,
        "# Owner\n- [m5] likes short answers\n- [m4] only remote jobs\n  * [m6] indented note\n\nSome prose.\n---\n**bold**\n-\n"
    );
    assert_eq!(
        super::notes::numbered(&once, 'm'),
        once,
        "numbering twice changes nothing"
    );
    assert_eq!(
        super::notes::numbered("- a\n- b", 's'),
        "- [s1] a\n- [s2] b"
    );
    assert_eq!(super::notes::numbered("", 'm'), "");
}

#[tokio::test]
async fn shared_memory_comes_first() {
    let home = std::env::temp_dir().join(format!("tirith-shared-memory-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(home.join("bots")).unwrap();
    let app = crate::app::App::open_at(home.clone());
    let card = json!({"name": "Notas", "shape": "drop", "color": "#22d3ee", "session": "chat", "access": "read", "routes": []});
    let bot = super::save(&app, card).unwrap();
    let key = format!("bot-{}", bot["id"].as_str().unwrap());
    std::fs::write(
        home.join("bots/memory.md"),
        "# Owner\n- name: Ana\n- time zone: Europe/Madrid\n",
    )
    .unwrap();
    let own = super::folder::folder(&app, &key).join("memory.md");
    std::fs::create_dir_all(own.parent().unwrap()).unwrap();
    std::fs::write(&own, "- [m2] only remote jobs\n- answer in Spanish\n").unwrap();
    let ctx = super::context(&app, &key).expect("a bot's context");
    let (shared, mine) = (
        ctx.find("[tirith] Shared memory.md as it is now:\n# Owner\n- [s1] name: Ana\n- [s2] time zone: Europe/Madrid")
            .expect(&ctx),
        ctx.find("[tirith] Your memory.md as it is now:\n- [m2] only remote jobs\n- [m3] answer in Spanish")
            .expect(&ctx),
    );
    assert!(shared < mine, "the shared memory comes first:\n{ctx}");
    // Saved numbered, and handed again unchanged.
    assert!(std::fs::read_to_string(&own)
        .unwrap()
        .contains("- [m3] answer in Spanish"));
    assert!(std::fs::read_to_string(home.join("bots/memory.md"))
        .unwrap()
        .contains("- [s2] time zone"));
    assert_eq!(super::context(&app, &key).unwrap(), ctx);
    // The Library shows and edits the shared memory, and gives the template.
    let lib = super::library::library(&app, bot["id"].as_str().unwrap()).unwrap();
    assert!(lib["shared_memory"]
        .as_str()
        .unwrap()
        .contains("[s1] name: Ana"));
    assert!(lib["template"]
        .as_str()
        .unwrap()
        .contains("## Cómo validar"));
    super::library::learn(
        &app,
        bot["id"].as_str().unwrap(),
        &json!({"memory": true, "shared": true, "text": "- language: es\n"}),
        false,
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(home.join("bots/memory.md")).unwrap(),
        "- language: es\n"
    );
    assert_eq!(
        std::fs::read_to_string(&own).unwrap(),
        "- [m2] only remote jobs\n- [m3] answer in Spanish\n"
    );
    let _ = std::fs::remove_dir_all(&home);
}

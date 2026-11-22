use chrono::{Local, TimeZone};
use serde_json::json;

/// A skill's `## ` sections: each title with the text under it.
fn sections(text: &str) -> Vec<(&str, &str)> {
    text.split("\n## ")
        .skip(1)
        .map(|part| part.split_once('\n').unwrap_or((part, "")))
        .map(|(title, body)| (title, body.trim()))
        .collect()
}

#[test]
fn bot_templates_are_complete() {
    let all = crate::factory::embedded_templates();
    let mut on_disk: Vec<String> =
        std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../factory/bots"))
            .unwrap()
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
    on_disk.sort();
    assert_eq!(
        all.keys().cloned().collect::<Vec<_>>(),
        on_disk,
        "every template of the folder is built in"
    );
    assert_eq!(
        on_disk,
        ["resumen-correo", "revisar-pr", "vigilar-web"],
        "the three templates"
    );

    let template = sections(include_str!("../../../factory/skill.md"));
    assert_eq!(template.len(), 7);
    let faces = include_str!("../../../web/js/views/bots/face/shapes2d.js");
    let mut bots = vec![];
    for (id, (bot, skill)) in &all {
        let t = super::template::parse(id, bot, skill).unwrap();

        let (header, _) = skill
            .strip_prefix("---\n")
            .and_then(|x| x.split_once("\n---\n"))
            .unwrap_or_else(|| panic!("{id}: no header"));
        let lines: Vec<&str> = header.lines().collect();
        assert_eq!(lines.len(), 2, "{id}: a name and a description, no draft");
        assert_eq!(lines[0], format!("name: {id}"));
        assert!(
            lines[1].len() > "description: ".len() && lines[1].starts_with("description: "),
            "{id}: a description"
        );
        let own = sections(skill);
        assert_eq!(
            own.iter().map(|x| x.0).collect::<Vec<_>>(),
            template.iter().map(|x| x.0).collect::<Vec<_>>(),
            "{id}: the seven sections, in the template's order"
        );
        for ((title, body), (_, hint)) in own.iter().zip(&template) {
            assert!(
                !body.is_empty() && body != hint,
                "{id}: «{title}» has its own text"
            );
        }

        assert!(!t.start.is_empty(), "{id}: says what it needs first");
        assert!(
            t.name.chars().count() <= 40,
            "{id}: a name the card can hold"
        );
        assert!(
            faces.contains(&format!("\n  {}: [", t.shape)),
            "{id}: a shape the page draws"
        );
        assert!(
            faces.contains(&format!("'{}'", t.color)),
            "{id}: a colour of the page"
        );

        let r = &t.routine;
        let prompt = r["prompt"].as_str().unwrap();
        assert!(prompt.contains(id.as_str()), "{id}: uses its skill");
        if r["trigger"] == "webhook" {
            let event = super::hook_event::event_of(&super::hook_event::sample()).unwrap();
            assert!(
                super::hook_event::matches(r["when"].as_str().unwrap(), &event),
                "{id}: a pull request opened starts it"
            );
            assert!(
                !prompt.contains(['{', '}']),
                "{id}: what comes from outside stays out of the task"
            );
        } else {
            // 2026-11-23 is a Monday.
            let late = Local.with_ymd_and_hms(2026, 11, 23, 23, 59, 0).unwrap();
            assert!(super::due(r, late), "{id}: its routine comes due");
        }
        let mut routine = r.clone();
        routine["id"] = json!("r1");
        bots.push(json!({"id": id, "name": t.name, "shape": t.shape, "color": t.color, "routines": [routine]}));
    }

    let of = |id: &str| bots.iter().find(|b| b["id"] == id).unwrap();
    let mail = of("resumen-correo");
    assert_eq!(mail["name"], "Resumen del correo a las 07:00");
    assert_eq!(mail["routines"][0]["at"], "07:00");
    assert_eq!(
        of("vigilar-web")["name"],
        "Vigilar una web y avisar si cambia"
    );
    let pr = of("revisar-pr");
    assert_eq!(pr["name"], "Revisar los PR nuevos");
    assert_eq!(pr["routines"][0]["when"], "github:pull_request.opened");

    // Written as bots, the factory takes them.
    let text = super::written::written("", &bots).unwrap();
    crate::factory::parse(
        crate::factory::TEXT,
        crate::factory::embedded_prompts(),
        &text.parse().unwrap(),
    )
    .unwrap();
}

#[tokio::test]
async fn a_template_makes_a_bot_with_its_skill_and_routine() {
    let home = std::env::temp_dir().join(format!("tirith-bot-template-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let app = crate::app::App::open_at(home.clone());
    for (id, (_, skill)) in crate::factory::embedded_templates() {
        let made = super::from_template::create(&app, &id).unwrap();
        let key = made["key"].as_str().unwrap();
        let kept = super::bot(&app, key).expect("the bot is kept");
        assert_eq!(kept["name"], made["name"]);
        assert_eq!(kept["shape"], made["shape"]);
        let r = &kept["routines"][0];
        assert_eq!(r["prompt"], made["routines"][0]["prompt"]);
        assert!(
            !r["last"].as_str().unwrap().is_empty() && !super::due(r, Local::now()),
            "{id}: it does not run for a time already past today"
        );
        assert_eq!(
            std::fs::read_to_string(super::skill_file(&app, key, &id)).unwrap(),
            skill
        );
    }
    let before = super::all(&app).len();
    for bad in ["no-such-template", "../resumen-correo", ""] {
        assert!(super::from_template::create(&app, bad).is_err(), "{bad}");
    }
    assert_eq!(super::all(&app).len(), before, "nothing made for those");
    let _ = std::fs::remove_dir_all(&home);
}

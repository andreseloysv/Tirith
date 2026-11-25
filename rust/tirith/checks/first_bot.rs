use super::{start, welcome, welcomed};
use serde_json::Value;

fn app(name: &str, config: &str) -> crate::app::Shared {
    let home = std::env::temp_dir().join(format!("tirith-first-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("config.toml"), config).unwrap();
    crate::app::App::open_at(home)
}

fn own(app: &crate::app::Shared) -> Vec<Value> {
    let mut bots = super::super::all(app);
    bots.retain(|b| b["system"] != true);
    bots
}

#[tokio::test]
async fn a_new_home_starts_with_one_bot_that_owes_its_welcome() {
    let app = app("new", "[network]\nsandbox = false\n");
    start(&app);
    let bots = own(&app);
    assert_eq!(bots.len(), 1, "{bots:?}");
    let look = (&bots[0]["name"], &bots[0]["shape"], &bots[0]["color"]);
    assert_eq!(look, (&"NeoBot".into(), &"blob".into(), &"#8fd14f".into()));
    let id = bots[0]["id"].as_str().unwrap().to_string();
    assert_eq!(welcome(&app), Some(id.clone()));
    start(&app);
    assert_eq!(own(&app).len(), 1, "a second start makes no second bot");
    welcomed(&app, "another");
    assert_eq!(
        welcome(&app),
        Some(id.clone()),
        "only its own welcome counts"
    );
    welcomed(&app, &id);
    assert_eq!(welcome(&app), None);
    // Deleted by the owner, it does not come back.
    super::super::store::with(&app, |bots| {
        bots.retain(|b| b["id"] != id.as_str());
        Ok(())
    })
    .unwrap();
    start(&app);
    assert!(own(&app).is_empty(), "the owner's choice stays");
    let _ = std::fs::remove_dir_all(&app.home);
}

#[tokio::test]
async fn an_older_home_keeps_what_it_has() {
    let with_project = "[network]\nsandbox = false\n[[projects]]\nid = \"p1\"\nrepo = \"/tmp\"\n";
    let with_bot = "[network]\nsandbox = false\n[roles.bot-abc123]\nextends = \"manager\"\nscope = \"own\"\nname = \"Mine\"\n";
    for (name, config, bots) in [("project", with_project, 0), ("bot", with_bot, 1)] {
        let app = app(name, config);
        start(&app);
        assert_eq!(own(&app).len(), bots, "{name}");
        assert_eq!(welcome(&app), None, "{name}: nobody owes a welcome");
        let _ = std::fs::remove_dir_all(&app.home);
    }
}

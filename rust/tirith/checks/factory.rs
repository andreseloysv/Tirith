use super::*;

fn overlay(text: &str) -> toml::Table {
    text.parse().unwrap()
}

#[test]
fn embedded_factory_has_todays_values() {
    let f = Factory::default();
    assert_eq!(f.agent::<String>("anthropic", "effort_flag").as_deref(), Some("--effort"));
    assert_eq!(f.agent::<Vec<String>>("codex", "lane_args"), Some(vec!["exec".into(), "--json".into(), "--dangerously-bypass-approvals-and-sandbox".into(), "Read the brief at {brief} and do it completely in {workspace}.".into()]));
    assert_eq!(f.agent::<Vec<String>>("muse", "resume_args"), Some(vec![]));
    assert_eq!(f.agent::<Vec<String>>("muse", "chat_args"), None); // exec: no stream-json chat
    assert!(f.agent::<Vec<String>>("anthropic", "chat_args").unwrap().iter().any(|a| a == "stream-json"));
    assert_eq!(f.agent::<Vec<String>>("whatever", "lane_args"), f.agent("default", "lane_args")); // an unknown alias runs with the default
    assert_eq!(f.limits.max_load, 2.0);
    assert_eq!((f.limits.merge_train, f.limits.merge_backlog, f.limits.low_disk_gb, f.limits.batch_seconds), (5, 3, 5.0, 2));
    assert_eq!(f.limits.max_parallel, None);
    assert_eq!(f.words("plan", "role_prefixes"), ["manager de guardia:", "ceo:"]);
    assert_eq!(f.words("cards", "kinds").len(), 5);
    assert_eq!(f.metric("merged"), ("En main (24 h)".to_string(), Some(10.0), true));
    assert_eq!(f.metric("interruptions"), ("Interrupciones por ítem (24 h)".to_string(), Some(1.0), false));
    assert_eq!(f.metric("needless_cards"), ("Tarjetas innecesarias (%)".to_string(), Some(25.0), false));
    assert_eq!(f.metric("failed").1, None);
    assert!(!f.metric("idle").2);
}

#[test]
fn default_routes_reproduce_today() {
    // Every call the daemon made when who-hears-what was fixed in Rust, as (to, mode, chat, phone, after_minutes):
    // `notify` was turn+chat+phone, `inform` note+chat+phone, `tell` a turn with no line of its own whose answer
    // reached the phones like every manager message; owner and room messages start turns whose answers do.
    // Quieter phones (factory step 4): merge.landed, main.green and deploy.done keep chat+note, without phone.
    let f = Factory::default();
    let turn = |to: &str, chat: bool| vec![(to.to_string(), "turn".to_string(), chat, true, None)];
    let note = |chat: bool, phone: bool| vec![("manager".to_string(), "note".to_string(), chat, phone, None)];
    let table: Vec<(&str, Vec<Route>)> = vec![
        ("owner.message", turn("manager", false)),
        ("room.message", turn("manager", false)),
        ("lane.asked", turn("manager", true)),
        ("lane.failed", turn("manager", true)),
        ("lane.not_started", turn("manager", true)),
        ("lane.long", turn("manager", true)),
        ("merge.blocked", turn("manager", true)),
        ("merge.landed", note(true, false)),
        ("main.red", turn("manager", true)),
        ("main.green", note(true, false)),
        ("plan.empty", turn("manager", false)),
        ("deploy.done", note(true, false)),
        ("deploy.failed", turn("manager", true)),
        ("schedule", turn("manager", false)),
        ("review.done", turn("lead", false)),
        ("factory.invalid", turn("lead", true)),
        ("card.opened", vec![]),
        ("card.answered", vec![]),
        ("card.closed", vec![]),
    ];
    for (on, want) in table {
        assert_eq!(f.routes_on(on), want, "routes on {on}");
    }
    // A red measure wakes the lead at once and, still red after 15 minutes, the improver.
    assert_eq!(f.routes_on("metric.red"), vec![("lead".to_string(), "turn".to_string(), true, true, None), ("improver".to_string(), "turn".to_string(), false, false, Some(15))]);
    // A route on a missing event, to a missing role, or with an unknown key or mode makes the version invalid.
    let prompts = embedded_prompts();
    let bad = |text: &str| parse(text, prompts.clone(), &toml::Table::new()).err().unwrap_or_default();
    assert!(bad("[[routes]]\non = \"no.such.event\"\nto = \"manager\"\nmode = \"turn\"\n").contains("missing event"));
    assert!(bad("[[routes]]\non = \"lane.failed\"\nto = \"ghost\"\nmode = \"turn\"\n").contains("missing event, role or mode"));
    assert!(bad("[[routes]]\non = \"lane.failed\"\nto = \"manager\"\nmode = \"shout\"\n").contains("missing event, role or mode"));
    assert!(bad("[[routes]]\non = \"lane.failed\"\nto = \"manager\"\nmode = \"turn\"\nbogus = 1\n").contains("unknown key routes.bogus"));
    // Routes of config.toml (a bot's routines) stand next to the factory's, never instead of them.
    let o = overlay("[[routes]]\non = \"schedule\"\nto = \"manager\"\nmode = \"turn\"\ntask = \"news\"\n");
    let f = parse(TEXT, prompts, &o).unwrap();
    assert_eq!(f.routes_on("schedule").len(), 2);
    assert_eq!(f.routes_on("lane.failed").len(), 1);
}

#[test]
fn roles_extend_and_prompts_resolve() {
    let f = Factory::default();
    assert_eq!(f.role::<String>("lead", "agent").as_deref(), Some("anthropic")); // extends manager
    assert_eq!(f.role::<u64>("improver", "max_per_day"), Some(6));
    assert_eq!(f.role::<String>("worker", "session").as_deref(), Some("lane"));
    assert!(f.prompts_of("manager").starts_with("You are the manager inside Tirith"));
    assert!(f.prompts_of("manager").contains("only about the project {id}"));
    assert!(f.prompts_of("lead").contains("not about one project"));
    // The improver closes the loop: by-version measures, at most one factory change as a step 17 item with its lane, and the revert rule.
    let improver = f.prompt("improver");
    assert!(improver.contains("You are the improver") && improver.contains("at most one change to `factory/`"), "one change at most:\n{improver}");
    assert!(improver.contains("tirith plan add tirith 17") && improver.contains("tirith lane start tirith 17"), "as a step 17 item with its lane started");
    assert!(improver.contains("first 10 merged items, or 24 h") && improver.contains("satisfaction") && improver.contains("interruptions") && improver.contains("first try"), "the revert window and its three measures");
    assert!(improver.contains("revert the factory change"), "a worse version is reverted by its next pass");
    assert!(f.prompt("brief").contains("# Lane {id}"));
}

#[test]
fn factory_roles_carry_their_shape() {
    let f = Factory::default();
    assert_eq!(f.roles(), ["improver", "judge", "lead", "manager", "worker"]);
    let shape = |r: &str| (f.role::<String>(r, "session"), f.role::<String>(r, "scope"), f.role::<String>(r, "runs_on"));
    assert_eq!(shape("manager"), (Some("chat".into()), Some("project".into()), Some("local".into())));
    assert_eq!(shape("lead"), (Some("chat".into()), Some("global".into()), Some("local".into()))); // runs_on and idle come through extends
    assert_eq!(shape("worker"), (Some("lane".into()), Some("project".into()), Some("local".into())));
    assert_eq!(shape("improver"), (Some("turn".into()), Some("global".into()), Some("local".into())));
    assert_eq!(shape("judge"), (Some("turn".into()), Some("global".into()), Some("local".into())));
    assert_eq!(
        (f.role::<String>("judge", "agent").as_deref(), f.role::<String>("judge", "effort").as_deref(), f.role::<String>("judge", "access").as_deref()),
        (Some("muse"), Some("low"), Some("read")),
        "the judge: small text jobs on a cheap, fast model, read-only"
    );
    assert!(f.prompts_of("judge").is_empty(), "no prompt of its own: each eval's rubric is its instructions");
    assert_eq!(f.role::<String>("improver", "access").as_deref(), Some("read"));
    assert_eq!(f.role::<u64>("manager", "idle_minutes"), Some(30));
    assert_eq!(f.role::<u64>("lead", "idle_minutes"), Some(30));
    // Code finds roles by shape, never by a fixed list of names.
    assert_eq!(f.find_role("lane", None).as_deref(), Some("worker"));
    assert_eq!(f.find_role("chat", Some("project")).as_deref(), Some("manager"));
    assert_eq!(f.find_role("chat", Some("global")).as_deref(), Some("lead"));
    assert_eq!(f.find_role("turn", None).as_deref(), Some("improver"));
    assert_eq!(f.find_role("chat", Some("own")), None);
}

#[test]
fn overlay_wins_key_by_key() {
    let o = overlay("[limits]\nmax_load = 4.5\n[metrics.merged]\ntarget = 3\n[agents.anthropic]\ncommand = \"agent-a-test\"\n[roles.worker]\nagent = \"muse\"\n");
    let f = parse(TEXT, embedded_prompts(), &o).unwrap();
    assert_eq!(f.limits.max_load, 4.5);
    assert_eq!(f.limits.merge_train, 5); // untouched keys keep the factory's value
    assert_eq!(f.metric("merged").1, Some(3.0));
    assert_eq!(f.metric("merged").0, "En main (24 h)");
    assert_eq!(f.role::<String>("worker", "agent").as_deref(), Some("muse"));
    assert_eq!(f.command("anthropic").as_deref(), Some("agent-a-test"));
    assert_eq!(f.command("codex").as_deref(), Some("codex")); // no config line needed, as before
}

#[test]
fn invalid_factories_are_refused() {
    let prompts = embedded_prompts();
    let bad = |text: &str| parse(text, prompts.clone(), &toml::Table::new()).err().unwrap_or_default();
    assert!(bad("[bogus]\nx = 1\n").contains("unknown section"), "unknown section");
    assert!(bad("[limits]\nbogus = 1\n").contains("unknown field"), "unknown key in limits (deny_unknown_fields)");
    assert!(bad("[agents.anthropic]\nbogus = 1\n").contains("unknown key agents.anthropic.bogus"));
    assert!(bad("[roles.x]\nagent = \"ghost\"\nsession = \"chat\"\nscope = \"global\"\n").contains("missing agent"));
    assert!(bad("[roles.x]\nagent = \"anthropic\"\nsession = \"chat\"\nscope = \"global\"\nprompt = [\"prompts/nope.md\"]\n").contains("missing prompt"));
    assert!(bad("[metrics.nope]\nlabel = \"x\"\n").contains("unknown measure"));
    assert!(bad("[agents.x]\nprotocol = \"smoke-signals\"\n").contains("unknown protocol"));
    assert!(bad("[roles.x]\nextends = \"ghost\"\n").contains("extends a missing role"));
    assert!(bad("[roles.x]\nagent = \"anthropic\"\nsession = \"turn\"\nscope = \"global\"\nruns_on = \"the-moon\"\n").contains("runs_on 'the-moon'"));
    assert!(bad("[roles.x]\nagent = \"anthropic\"\nsession = \"turn\"\nscope = \"global\"\naccess = \"root\"\n").contains("access 'root'"));
    assert!(bad("[roles.x]\nagent = \"anthropic\"\n").contains("unknown session ''"));
    // An unknown key arriving through config.toml's overlay is refused too.
    let o = overlay("[limits]\nbogus = 1\n");
    assert!(parse(TEXT, prompts.clone(), &o).is_err());
    let o = overlay("[roles.worker]\nbogus = 1\n");
    assert!(parse(TEXT, prompts, &o).is_err());
}

#[test]
fn config_sections_move_to_their_new_names() {
    let mut t: toml::Table = "[models.workers]\nagent = \"muse\"\nmodel = \"m1\"\n[models]\nchoices = [\"m1\"]\n[lanes]\nmax_load = 3.5\nmax_parallel = 7\n[merge]\npush = false\nstage_timeout_minutes = 9\n[plan]\ncommit_seconds = 33\n[ceo]\nmax_per_day = 2\nenabled = true\n[ceo.targets]\nmerged_per_day = 4\n[roles.worker]\neffort = \"high\"\n[roles.reviewer]\neffort = \"low\"\n".parse().unwrap();
    assert!(crate::config::migrate(&mut t));
    assert!(!crate::config::migrate(&mut t)); // once is enough: nothing left to move
    let g = |p: &[&str]| crate::config::get::<toml::Value>(&t, p);
    assert_eq!(g(&["roles", "worker", "agent"]).and_then(|v| v.as_str().map(String::from)), Some("muse".into()));
    assert_eq!(g(&["roles", "worker", "effort"]).and_then(|v| v.as_str().map(String::from)), Some("high".into())); // the new name already written wins and keeps its other keys
    assert_eq!(g(&["limits", "max_load"]).and_then(|v| v.as_float()), Some(3.5));
    assert_eq!(g(&["limits", "max_parallel"]).and_then(|v| v.as_integer()), Some(7));
    assert_eq!(g(&["limits", "stage_timeout_minutes"]).and_then(|v| v.as_integer()), Some(9));
    assert_eq!(g(&["limits", "plan_commit_seconds"]).and_then(|v| v.as_integer()), Some(33));
    assert_eq!(g(&["roles", "improver", "max_per_day"]).and_then(|v| v.as_integer()), Some(2));
    assert_eq!(g(&["roles", "improver", "effort"]).and_then(|v| v.as_str().map(String::from)), Some("low".into())); // yesterday's reviewer is today's improver, its keys moved with it
    assert!(g(&["roles", "reviewer"]).is_none());
    assert_eq!(g(&["metrics", "merged", "target"]).and_then(|v| v.as_integer()), Some(4));
    assert!(t.get("lanes").is_none() && t.get("plan").is_none());
    assert_eq!(g(&["merge", "push"]).and_then(|v| v.as_bool()), Some(false)); // what stays, stays
    assert!(g(&["models", "choices"]).is_some());
    assert!(g(&["ceo", "enabled"]).is_some()); // a dead key stays put, so the unknown-key error names it
}

#[test]
fn stamp_is_factory_commit_plus_overlay_hash() {
    let base = Factory::default();
    let (commit, hash) = base.version.split_once('+').expect("version is <commit>+<hash>");
    assert!(!commit.is_empty(), "the main commit that last changed factory/ (or `embedded`): {}", base.version);
    assert_eq!(hash.len(), 8);
    assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()), "short hash of the overlay: {hash}");
    // The same overlay stamps the same; a key config.toml lays over the factory changes the hash, not the commit.
    let same = parse(TEXT, embedded_prompts(), &toml::Table::new()).unwrap();
    assert_eq!(same.version, base.version);
    let over = parse(TEXT, embedded_prompts(), &overlay("[limits]\nmax_load = 4.5\n")).unwrap();
    assert_eq!(over.version.split_once('+').unwrap().0, commit);
    assert_ne!(over.version, base.version);
    // What is no factory section (the projects) does not restamp the factory.
    let projects = parse(TEXT, embedded_prompts(), &overlay("[[projects]]\nid = \"x\"\n")).unwrap();
    assert_eq!(projects.version, base.version);
}

#[test]
fn stamp_moves_when_main_moves() {
    // The reload stamp covers main itself (its reflog and ref), so a landed factory change applies with no restart.
    let root = std::env::temp_dir().join(format!("tirith-factory-stamp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for d in ["factory/prompts", "factory/evals", ".git/logs", ".git/refs/heads"] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    std::fs::write(root.join("factory/factory.toml"), "").unwrap();
    std::fs::write(root.join(".git/logs/HEAD"), "old\n").unwrap();
    let over = overlay(&format!("factory = {:?}", root.join("factory").display().to_string()));
    let before = stamp(&over).expect("a stamp");
    let touched = std::process::Command::new("touch").args(["-t", "203001010000"]).arg(root.join(".git/logs/HEAD")).status().unwrap();
    assert!(touched.success());
    let after = stamp(&over).expect("a stamp after main moved");
    assert!(after > before, "main moving moves the stamp");
    assert!(after.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() > 1_893_000_000, "the stamp is main's reflog time (2030 in the test)");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn role_hash_tracks_agent_model_and_prompt() {
    let f = Factory::default();
    let worker = f.role_hash("worker", "");
    assert_eq!(worker.len(), 8);
    assert_ne!(worker, f.role_hash("manager", ""), "each role its own hash");
    assert_eq!(f.role_hash("owner", ""), "", "no factory role, no hash");
    assert_ne!(worker, f.role_hash("worker", "{\"model\":\"other\"}"), "the page's model override is folded in");
    // Another agent for the role (config.toml over the factory) is another hash.
    let muse = parse(TEXT, embedded_prompts(), &overlay("[roles.worker]\nagent = \"muse\"\n")).unwrap();
    assert_ne!(worker, muse.role_hash("worker", ""));
    // Another prompt text is another hash.
    let mut prompts = embedded_prompts();
    prompts.insert("brief".to_string(), "a different brief".to_string());
    let briefed = parse(TEXT, prompts, &toml::Table::new()).unwrap();
    assert_ne!(worker, briefed.role_hash("worker", ""));
    assert_eq!(f.role_hash("improver", ""), briefed.role_hash("improver", ""), "an untouched role keeps its hash");
}

#[test]
fn evals_judge_the_owners_mood_and_whether_a_card_was_needed() {
    let f = Factory::default();
    assert_eq!(f.evals_on("owner.message"), vec![("owner-mood".to_string(), "judge".to_string(), "owner-mood".to_string(), 1.0)]);
    assert_eq!(f.evals_on("card.closed"), vec![("card-needed".to_string(), "judge".to_string(), "card-needed".to_string(), 1.0)]);
    assert_eq!(f.evals_on("card.answered"), f.evals_on("card.closed"), "an answered card is judged too");
    assert!(f.evals_on("merge.landed").is_empty());
    // The rubrics name every label the measures count.
    for label in ["approval", "neutral", "correction", "repeat", "noise"] {
        assert!(f.prompt("owner-mood").contains(label), "owner-mood rubric names {label}");
    }
    for label in ["needed", "alone"] {
        assert!(f.prompt("card-needed").contains(label), "card-needed rubric names {label}");
    }
    // Satisfaction is a measure like any other: label, target and side in the factory.
    assert_eq!(f.metric("satisfaction"), ("Satisfacción del dueño (24 h)".to_string(), Some(0.0), true));
    // An eval on a missing event, with a missing judge or rubric, a sample outside 0 to 1, or an unknown key is invalid.
    let prompts = embedded_prompts();
    let bad = |text: &str| parse(text, prompts.clone(), &toml::Table::new()).err().unwrap_or_default();
    assert!(bad("[[evals]]\nid = \"x\"\non = \"no.such.event\"\njudge = \"judge\"\nrubric = \"evals/owner-mood.md\"\n").contains("missing event, judge role or rubric"));
    assert!(bad("[[evals]]\nid = \"x\"\non = \"owner.message\"\njudge = \"ghost\"\nrubric = \"evals/owner-mood.md\"\n").contains("missing event, judge role or rubric"));
    assert!(bad("[[evals]]\nid = \"x\"\non = \"owner.message\"\njudge = \"judge\"\nrubric = \"evals/nope.md\"\n").contains("missing event, judge role or rubric"));
    assert!(bad("[[evals]]\nid = \"x\"\non = \"owner.message\"\njudge = \"judge\"\nrubric = \"evals/owner-mood.md\"\nsample = 2.0\n").contains("sample outside 0 to 1"));
    assert!(bad("[[evals]]\nid = \"x\"\non = \"owner.message\"\njudge = \"judge\"\nrubric = \"evals/owner-mood.md\"\nbogus = 1\n").contains("unknown key evals.bogus"));
    // An eval arriving through config.toml's overlay replaces the factory's evals and restamps the factory.
    let o = overlay("[[evals]]\nid = \"owner-mood\"\non = \"owner.message\"\njudge = \"judge\"\nrubric = \"evals/owner-mood.md\"\nsample = 0.5\n");
    let f = parse(TEXT, prompts, &o).unwrap();
    assert_eq!(f.evals_on("owner.message")[0].3, 0.5);
    assert!(f.evals_on("card.closed").is_empty());
    assert_ne!(f.version, Factory::default().version);
}

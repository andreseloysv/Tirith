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
    assert_eq!(f.metric("failed").1, None);
    assert!(!f.metric("idle").2);
}

#[test]
fn roles_extend_and_prompts_resolve() {
    let f = Factory::default();
    assert_eq!(f.role::<String>("lead", "agent").as_deref(), Some("anthropic")); // extends manager
    assert_eq!(f.role::<u64>("ceo", "max_per_day"), Some(6)); // today's name for the reviewer
    assert_eq!(f.role::<String>("workers", "session").as_deref(), Some("lane"));
    assert!(f.prompts_of("manager").starts_with("You are the manager inside Tirith"));
    assert!(f.prompts_of("manager").contains("only about the project {id}"));
    assert!(f.prompts_of("lead").contains("not about one project"));
    assert!(f.prompt("reviewer").contains("CEO review"));
    assert!(f.prompt("brief").contains("# Lane {id}"));
}

#[test]
fn overlay_wins_key_by_key() {
    let o = overlay("[limits]\nmax_load = 4.5\n[metrics.merged]\ntarget = 3\n[agents.anthropic]\ncommand = \"agent-a-test\"\n[roles.worker]\nagent = \"muse\"\n");
    let f = parse(TEXT, embedded_prompts(), &o).unwrap();
    assert_eq!(f.limits.max_load, 4.5);
    assert_eq!(f.limits.merge_train, 5); // untouched keys keep the factory's value
    assert_eq!(f.metric("merged").1, Some(3.0));
    assert_eq!(f.metric("merged").0, "En main (24 h)");
    assert_eq!(f.role::<String>("workers", "agent").as_deref(), Some("muse"));
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
    // An unknown key arriving through config.toml's overlay is refused too.
    let o = overlay("[limits]\nbogus = 1\n");
    assert!(parse(TEXT, prompts.clone(), &o).is_err());
    let o = overlay("[roles.worker]\nbogus = 1\n");
    assert!(parse(TEXT, prompts, &o).is_err());
}

#[test]
fn config_sections_move_to_their_new_names() {
    let mut t: toml::Table = "[models.workers]\nagent = \"muse\"\nmodel = \"m1\"\n[models]\nchoices = [\"m1\"]\n[lanes]\nmax_load = 3.5\nmax_parallel = 7\n[merge]\npush = false\nstage_timeout_minutes = 9\n[plan]\ncommit_seconds = 33\n[ceo]\nmax_per_day = 2\nenabled = true\n[ceo.targets]\nmerged_per_day = 4\n[roles.worker]\neffort = \"high\"\n".parse().unwrap();
    assert!(crate::config::migrate(&mut t));
    assert!(!crate::config::migrate(&mut t)); // once is enough: nothing left to move
    let g = |p: &[&str]| crate::config::get::<toml::Value>(&t, p);
    assert_eq!(g(&["roles", "worker", "agent"]).and_then(|v| v.as_str().map(String::from)), Some("muse".into()));
    assert_eq!(g(&["roles", "worker", "effort"]).and_then(|v| v.as_str().map(String::from)), Some("high".into())); // the new name already written wins and keeps its other keys
    assert_eq!(g(&["limits", "max_load"]).and_then(|v| v.as_float()), Some(3.5));
    assert_eq!(g(&["limits", "max_parallel"]).and_then(|v| v.as_integer()), Some(7));
    assert_eq!(g(&["limits", "stage_timeout_minutes"]).and_then(|v| v.as_integer()), Some(9));
    assert_eq!(g(&["limits", "plan_commit_seconds"]).and_then(|v| v.as_integer()), Some(33));
    assert_eq!(g(&["roles", "reviewer", "max_per_day"]).and_then(|v| v.as_integer()), Some(2));
    assert_eq!(g(&["metrics", "merged", "target"]).and_then(|v| v.as_integer()), Some(4));
    assert!(t.get("lanes").is_none() && t.get("plan").is_none());
    assert_eq!(g(&["merge", "push"]).and_then(|v| v.as_bool()), Some(false)); // what stays, stays
    assert!(g(&["models", "choices"]).is_some());
    assert!(g(&["ceo", "enabled"]).is_some()); // a dead key stays put, so the unknown-key error names it
}

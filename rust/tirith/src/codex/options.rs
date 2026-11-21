//! The options every Codex command line gets from config.toml: its profile and the reasoning effort.

pub fn options(cfg: &crate::config::Config, argv: &mut Vec<String>, effort: Option<&str>) {
    crate::config::get::<String>(&cfg.raw, &["agents", "codex", "profile"])
        .into_iter()
        .for_each(|profile| drop(argv.splice(1..1, ["--profile".into(), profile])));
    if let Some(e) = effort {
        // Replace only this config key; unrelated -c overrides must survive.
        argv.retain(|a| !a.starts_with("--config=model_reasoning_effort="));
        for i in (1..argv.len()).rev() {
            let _ = (argv[i].starts_with("model_reasoning_effort=")
                && matches!(argv[i - 1].as_str(), "-c" | "--config"))
            .then(|| argv.drain(i - 1..=i));
        }
        argv.push(format!(
            "--config=model_reasoning_effort={}",
            toml::Value::String(e.into())
        ));
    }
}

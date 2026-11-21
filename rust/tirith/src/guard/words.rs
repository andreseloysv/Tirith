//! A project's forbidden words, from config.toml.
use crate::app::Shared;
use crate::config::Project;

pub fn words(app: &Shared, project: &Project) -> Vec<String> {
    let cfg = app.cfg();
    let entry = cfg
        .raw
        .get("projects")
        .and_then(|p| p.as_array())
        .and_then(|ps| {
            ps.iter()
                .find(|p| p.get("id").and_then(|i| i.as_str()) == Some(project.id.as_str()))
                .cloned()
        })
        .unwrap_or(toml::Value::Table(Default::default()));
    let Some(()) = (entry.get("guard").and_then(|g| g.as_bool()) != Some(false)).then_some(())
    else {
        return vec![];
    };
    let list = |v: Option<&toml::Value>| -> Option<Vec<String>> { v?.clone().try_into().ok() };
    list(entry.get("guard_words"))
        .or_else(|| list(cfg.raw.get("guard").and_then(|g| g.get("words"))))
        .unwrap_or_default()
        .into_iter()
        .filter(|w| !w.trim().is_empty())
        .collect()
}

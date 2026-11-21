//! Changing a plan's lines under the writer lock.
use super::item::Step;
use super::lookup::{path, same};
use crate::app::Shared;
use serde_json::json;

/// Applies `edit` to the plan's lines under the writer lock, then queues the commit.
pub(super) async fn change(
    app: &Shared,
    project: &str,
    edit: impl FnOnce(&mut Vec<String>, &[Step]) -> Result<(), String>,
) -> Result<(), String> {
    let _guard = app.plan_lock.lock().await;
    let file = path(app, project)?;
    let text = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    edit(&mut lines, &super::parse(&text))?;
    std::fs::write(&file, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
    app.plan_dirty.lock().unwrap().insert(project.to_string());
    app.emit("plan", json!({ "project": project }));
    Ok(())
}

/// Takes an open item out of the plan (its lane was deleted).
pub async fn drop_item(app: &Shared, project: &str, step: &str, item: &str) -> Result<(), String> {
    change(app, project, |lines, steps| {
        let it = steps
            .iter()
            .filter(|s| s.num == step)
            .flat_map(|s| &s.items)
            .find(|i| !i.done && same(&i.text, item))
            .ok_or("no such open item")?;
        lines.remove(it.line);
        Ok(())
    })
    .await
}

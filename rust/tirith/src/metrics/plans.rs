//! The plans as a CEO pass reads them: each project's steps with their open and done items, its autopilot and its main.
use crate::app::Shared;
use crate::plan;
/// One project's steps, each with how many items are open and done.
fn steps(app: &Shared, project: &str) -> String {
    plan::read(app, project)
        .unwrap_or_default()
        .iter()
        .map(|st| {
            format!(
                "- step {}. {}: {} open, {} done\n",
                st.num,
                st.title,
                st.items.iter().filter(|i| !i.done).count(),
                st.items.iter().filter(|i| i.done).count()
            )
        })
        .collect()
}
fn plan_of(app: &Shared, p: &crate::config::Project) -> String {
    let steps = steps(app, &p.id);
    format!(
        "## {} (repo {}, autopilot {}, main {})\n{steps}",
        p.id,
        p.repo().display(),
        if crate::lanes::autopilot_on(app, &p.id) {
            "on"
        } else {
            "off"
        },
        app.kv(&format!("main_red:{}", p.id))
            .as_deref()
            .unwrap_or("green")
    )
}
pub(super) fn plans(app: &Shared) -> String {
    let cfg = app.cfg();
    cfg.projects.iter().map(|p| plan_of(app, p)).collect()
}

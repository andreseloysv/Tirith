//! Which conversation and which factory role a row belongs to, and the stamp it carries.
use super::App;

impl App {
    /// The conversation of `project` (a project or a bot), or the manager of no project's (`manager-general`, a system bot) when there is no such project (any more).
    pub fn conv<'a>(&self, project: &'a str) -> &'a str {
        if self.cfg().project(project).is_some() || crate::bots::bot(self, project).is_some() {
            project
        } else {
            "manager-general"
        }
    }

    /// The factory role of a conversation: a bot's own key (a system bot's too), else a project's `session = "chat"`, `scope = "project"` role; of no project, the manager of no project.
    pub fn chat_role(&self, project: &str) -> String {
        let Some(()) = crate::bots::bot(self, project).is_none().then_some(()) else {
            return project.to_string();
        };
        self.cfg().project(project).map_or_else(
            || "manager-general".into(),
            |_| {
                self.cfg()
                    .factory
                    .find_role("chat", Some("project"))
                    .unwrap_or_else(|| "manager".into())
            },
        )
    }

    /// The stamp a row made now by `role` carries: (the factory version, a hash of the role's setting with the page's model override for it folded in). A name that is no factory role (the owner, a room, NeoBot itself) keeps an empty hash.
    pub fn stamp_for(&self, role: &str) -> (String, String) {
        let cfg = self.cfg();
        (
            cfg.factory.version.clone(),
            cfg.factory.role_hash(
                role,
                &self.kv(&format!("models:{role}")).unwrap_or_default(),
            ),
        )
    }
}

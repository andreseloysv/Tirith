//! Reading a factory's roles: their keys, who runs a session, and the prompts they run with.
use super::embedded::embedded_prompts;
use super::loaded::Factory;
use super::values::stem;
use crate::config::get;
use serde::de::DeserializeOwned;

impl Factory {
    /// One key of a role, by its factory name.
    pub fn role<T: DeserializeOwned>(&self, name: &str, key: &str) -> Option<T> {
        get(&self.raw, &["roles", name, key])
    }

    /// False when the owner switched the judge off for the conversation `conv` (`judge = false` in its bot's role): its turns are not graded, which saves their cost. On by default, and always for a system role.
    pub fn judged(&self, conv: &str) -> bool {
        !conv.starts_with("bot-") || self.role::<bool>(conv, "judge") != Some(false)
    }

    /// Every role of the factory, in name order.
    pub fn roles(&self) -> Vec<String> {
        self.raw["roles"]
            .as_table()
            .into_iter()
            .flatten()
            .map(|(n, _)| n.clone())
            .collect()
    }

    /// The first role (in name order) with this `session` and, when given, this `scope`: how code finds who runs a lane, a project's chat or a turn without naming a role. A role with no prompt of its own (the evals' judge, whose instructions are each rubric) never runs standalone, so it is never found here.
    pub fn find_role(&self, session: &str, scope: Option<&str>) -> Option<String> {
        self.roles().into_iter().find(|n| {
            self.role::<String>(n, "session").as_deref() == Some(session)
                && scope.is_none_or(|s| self.role::<String>(n, "scope").as_deref() == Some(s))
                && !self.prompts_of(n).is_empty()
        })
    }

    /// One prompt or rubric by file stem; the embedded one when this version lacks it.
    pub fn prompt(&self, name: &str) -> String {
        self.prompts
            .get(name)
            .cloned()
            .unwrap_or_else(|| embedded_prompts().get(name).cloned().unwrap_or_default())
    }

    /// A role's prompt files, one after another, with `{name}` its factory name (a bot's, from config.toml) and `{skill_template}` the text of skill.md (after `{name}`: the template keeps its own).
    pub fn prompts_of(&self, role: &str) -> String {
        self.role::<Vec<String>>(role, "prompt")
            .unwrap_or_default()
            .iter()
            .map(|p| self.prompt(&stem(p)))
            .collect::<Vec<_>>()
            .join("\n")
            .replace(
                "{name}",
                &self
                    .role::<String>(role, "name")
                    .unwrap_or_else(|| role.to_string()),
            )
            .replace("{skill_template}", self.prompt("skill").trim())
    }
}

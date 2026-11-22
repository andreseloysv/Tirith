//! What one exec turn is: which agent runs it, in which folder, and how its role's factory shape sets it up.
use super::kind::Exec;
use crate::app::Shared;
use std::path::PathBuf;

/// The agent that runs a turn, and the name its errors give it.
pub(super) struct Agent {
    pub kind: Exec,
    pub name: &'static str,
    pub command: String,
}

/// A role's turn: its agent, its shape (read-only, a chat) and where it runs.
pub(super) struct Setup {
    pub agent: Agent,
    pub read_only: bool,
    pub chat: bool,
    /// The log and process name: "manager" for a chat role, the role otherwise.
    pub who: String,
    pub dir: PathBuf,
}

impl Agent {
    fn of(app: &Shared, kind: Exec) -> Result<Agent, String> {
        let command = app.cfg().factory.command(kind.alias()).ok_or_else(|| {
            format!(
                "no agent alias '{}' in config.toml: connect it with tirith connect {}",
                kind.alias(),
                kind.alias()
            )
        })?;
        Ok(Agent {
            kind,
            name: kind.name(),
            command,
        })
    }
}

impl Setup {
    pub fn new(app: &Shared, role: &str, project: &str, agent: &str) -> Result<Setup, String> {
        let cfg = app.cfg();
        // The role's factory shape, not its name, decides: `access = "read"` runs read-only; a chat role (a bot's key counts as one) keeps the chats' folder and the longer turn.
        let read_only = cfg.factory.role::<String>(role, "access").as_deref() == Some("read");
        let chat = cfg
            .factory
            .role::<String>(role, "session")
            .is_none_or(|s| s == "chat");
        let who = if chat { "manager" } else { role };
        let kind = Exec::of(agent).ok_or(format!("agent '{agent}' runs no exec turns"))?;
        let agent = Agent::of(app, kind)?;
        let dir = if cfg.factory.role::<String>(role, "scope").as_deref() == Some("own") && !chat {
            app.home.join("manager").join(role)
        } else {
            app.home.join(who).join(project)
        };
        let dir = crate::lab::seat(dir); // a B run of a live experiment runs in its branch's worktree
        Ok(Setup {
            agent,
            read_only,
            chat,
            who: who.to_string(),
            dir,
        })
    }

    /// How long the turn may take: a chat's turn is longer.
    pub fn limit(&self) -> std::time::Duration {
        std::time::Duration::from_secs(if self.chat { 1800 } else { 600 })
    }
}

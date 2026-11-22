//! The one verdict on a bot's use of a connector's tool: listing and calling both ask it, so a tool that is listed is one a call may reach.
use super::glob::any;
use super::{Connector, Grant};

/// What happens to one tool, in the order it is decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The connector is `blocked`, for every bot.
    Blocked,
    /// The connector's `deny` matches the tool.
    Denied,
    /// The bot's own `deny` matches the tool.
    BotDenied,
    /// The bot's `tools` is set and does not match the tool.
    OutOfScope,
    /// The connector's `approval` matches the tool.
    Approval,
    /// The bot's own `approval` matches the tool.
    BotApproval,
    Run,
}

impl Verdict {
    /// The word a record uses.
    pub fn word(self) -> &'static str {
        match self {
            Verdict::Blocked => "refused by block",
            Verdict::Denied | Verdict::BotDenied => "refused by deny",
            Verdict::OutOfScope => "refused by scope",
            Verdict::Approval | Verdict::BotApproval => "approval required",
            Verdict::Run => "ran",
        }
    }
    /// A refused tool is neither listed nor called.
    pub fn refused(self) -> bool {
        !matches!(
            self,
            Verdict::Approval | Verdict::BotApproval | Verdict::Run
        )
    }
}

/// The verdict on `tool` of connector `c` for a bot with grant `g`: connector blocked; connector deny; bot deny; outside the bot's tools when it set them; connector approval; bot approval; otherwise run. The bot's rules come after the connector's, so they can only narrow it.
pub fn verdict(c: &Connector, g: &Grant, tool: &str) -> Verdict {
    if c.blocked {
        Verdict::Blocked
    } else if any(&c.deny, tool) {
        Verdict::Denied
    } else if any(&g.deny, tool) {
        Verdict::BotDenied
    } else if g.tools.as_ref().is_some_and(|t| !any(t, tool)) {
        Verdict::OutOfScope
    } else if any(&c.approval, tool) {
        Verdict::Approval
    } else if any(&g.approval, tool) {
        Verdict::BotApproval
    } else {
        Verdict::Run
    }
}

/// The tools of `catalog` the bot is shown, each with its verdict (an approval one too: calling it asks).
pub fn listed(c: &Connector, g: &Grant, catalog: &[String]) -> Vec<(String, Verdict)> {
    catalog
        .iter()
        .map(|t| (t.clone(), verdict(c, g, t)))
        .filter(|(_, v)| !v.refused())
        .collect()
}

/// Why a call of `tool` is refused, naming the tools the bot may use; None when it is not refused. Only the connector's id and tool names are said, never its secrets or command.
pub fn refusal(c: &Connector, g: &Grant, tool: &str, catalog: &[String]) -> Option<String> {
    let v = verdict(c, g, tool);
    v.refused().then(|| {
        let may: Vec<String> = listed(c, g, catalog).into_iter().map(|(t, _)| t).collect();
        let may = if may.is_empty() {
            "none".to_string()
        } else {
            may.join(", ")
        };
        format!("{}.{tool}: {}; this bot may use: {may}", c.id, v.word())
    })
}

//! From a token alone to its agent: the lane, the role with its project, and the bot.
use super::who::{sent, Agent};
use crate::app::Shared;
use axum::http::HeaderMap;
use std::ops::Not;

/// The agent whose token the request carries.
pub(super) fn agent(app: &Shared, headers: &HeaderMap) -> Option<Agent> {
    let who = crate::sandbox::who_of(app, sent(headers)?)?;
    who.starts_with("door:").not().then_some(())?; // the door's token opens the door alone
                                                   // `replay:<experiment>:<who>` is that who, its network closed (`lab/replay.rs`).
    let own = who
        .strip_prefix("replay:")
        .and_then(|rest| rest.split_once(':'))
        .map_or(who.as_str(), |(_, own)| own);
    Some(match own.split_once('@') {
        Some((role, project)) => in_role(app, role, project),
        None if own.starts_with("merge:") || own == "git" => Agent {
            name: own.into(),
            ..Default::default()
        },
        None => Agent {
            name: own.into(),
            lane: own.into(),
            project: app.lane(own).map(|l| l.project).unwrap_or_default(),
            ..Default::default()
        },
    })
}

/// A session of `role` on conversation `project` (`manager::spawn_chat`). Every chat runs as the manager, so a bot's is told by its conversation; a turn's by its role.
fn in_role(app: &Shared, role: &str, project: &str) -> Agent {
    let key = if role == "manager" {
        app.conv(project).to_string()
    } else {
        role.to_string()
    };
    let bot = crate::bots::bot(app, &key).map(|_| key).unwrap_or_default();
    Agent {
        name: role.into(),
        role: role.into(),
        project: project.into(),
        bot,
        ..Default::default()
    }
}

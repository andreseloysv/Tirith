//! One turn while it runs: who runs it and why, and what its lines say (its usage, tools, skills and text).
use super::tools::{calls, skill_of};
use super::usage::Usage;
use crate::app::{now_iso, Shared};
use serde_json::{json, Map, Value};
use std::time::Instant;

/// One turn of a chat or turn role, from its start to its end.
pub struct Turn {
    pub conv: String,
    pub role: String,
    pub trigger: String,
    pub routine: String,
    pub started_at: String,
    pub started: Instant,
    /// Its start in microseconds since the epoch, as Muse's records count time.
    pub start_us: i64,
    pub agent: String,
    pub model: String,
    pub effort: String,
    pub usage: Usage,
    pub steps: i64,
    pub skills: Vec<String>,
    pub tools: Vec<Value>,
    pub text: String,
    /// Its agent's session (Codex's thread).
    pub session: String,
    /// A turn of its own (a turn role's): its whole session.
    pub loose: bool,
    /// The versions of its memory and skills as they were when it started.
    pub began: Map<String, Value>,
    /// Some line of its agent came during it.
    pub seen: bool,
    /// Its own validation (`tirith turn checks`): passed, total and the names of the checks.
    pub checks: Option<(i64, i64, String)>,
    /// How its looks ended.
    pub looks: super::Looks,
    /// What it read: its start and each tool call with its result (`<home>/captures`).
    pub capture: super::capture::Capture,
}

impl Turn {
    /// A turn of `role` in conversation `conv` starts now, started by `trigger` (and routine `routine`).
    pub fn new(app: &Shared, conv: &str, role: &str, (trigger, routine): (String, String)) -> Turn {
        let setting = crate::models::role(app, role);
        let mut turn = Turn {
            conv: conv.into(),
            role: role.into(),
            trigger,
            routine,
            started_at: now_iso(),
            started: Instant::now(),
            start_us: chrono::Utc::now().timestamp_micros(),
            agent: setting.agent,
            model: setting.model.filter(|m| m != "auto").unwrap_or_default(), // auto: its lines name the one it chose
            effort: setting.effort.unwrap_or_default(),
            usage: Usage::default(),
            steps: 0,
            skills: vec![],
            tools: vec![],
            text: String::new(),
            session: String::new(),
            loose: false,
            began: Map::new(),
            seen: false,
            checks: None,
            looks: Default::default(),
            capture: super::capture::Capture::of(conv),
        };
        turn.began = super::versions::at_start(app, &turn);
        turn
    }

    /// Takes one line of its agent's output.
    pub fn line(&mut self, msg: &Value) {
        self.seen = true;
        self.usage.take(msg);
        self.capture.take(msg);
        self.looks.take(msg);
        for call in calls(msg) {
            self.steps += 1;
            let skill = skill_of(&call).filter(|s| !self.skills.contains(s));
            self.skills.extend(skill);
            let short = json!({"name": call.name, "summary": call.summary});
            let _ = (self.tools.len() < 50).then(|| self.tools.push(short));
        }
        for words in super::said::words(msg) {
            self.text = format!("{}\n\n{words}", self.text).trim().to_string();
        }
        if let Some(model) = super::said::model(msg).filter(|_| self.model.is_empty()) {
            self.model = model.into();
        }
        if let Some(id) = super::said::session(msg) {
            self.session = id.into();
        }
    }
}

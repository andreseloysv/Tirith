//! One turn while it runs: who runs it and why, and what its lines say (its usage, tools, skills and text).
use super::tools::{calls, skill_of};
use super::usage::Usage;
use crate::app::{now_iso, Shared};
use serde_json::{json, Value};
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
    /// Muse's session id, for its usage when its output carries none.
    pub session: String,
    /// Some line of its agent came during it.
    pub seen: bool,
    /// Its own validation (`tirith turn checks`): passed, total and the names of the checks.
    pub checks: Option<(i64, i64, String)>,
}

impl Turn {
    /// A turn of `role` in conversation `conv` starts now, started by `trigger` (and routine `routine`).
    pub fn new(app: &Shared, conv: &str, role: &str, (trigger, routine): (String, String)) -> Turn {
        let setting = crate::models::role(app, role);
        Turn {
            conv: conv.into(),
            role: role.into(),
            trigger,
            routine,
            started_at: now_iso(),
            started: Instant::now(),
            start_us: chrono::Utc::now().timestamp_micros(),
            agent: setting.agent,
            model: setting.model.unwrap_or_default(),
            effort: setting.effort.unwrap_or_default(),
            usage: Usage::default(),
            steps: 0,
            skills: vec![],
            tools: vec![],
            text: String::new(),
            session: String::new(),
            seen: false,
            checks: None,
        }
    }

    /// Takes one line of its agent's output.
    pub fn line(&mut self, msg: &Value) {
        self.seen = true;
        self.usage.take(msg);
        for call in calls(msg) {
            self.steps += 1;
            let skill = skill_of(&call).filter(|s| !self.skills.contains(s));
            self.skills.extend(skill);
            let short = json!({"name": call.name, "summary": call.summary});
            let _ = (self.tools.len() < 50).then(|| self.tools.push(short));
        }
        self.text_of(msg);
        let model = msg["model"].as_str().filter(|_| msg["type"] == "system");
        let model = model.or_else(|| msg["payload"]["event"]["model"].as_str());
        let _ = (self.model.is_empty()).then(|| self.model = model.unwrap_or("").into());
        let session = msg["stream"]["id"]
            .as_str()
            .filter(|_| msg["stream"]["kind"] == "session");
        session.into_iter().for_each(|id| self.session = id.into());
    }

    /// The answer's words: Anthropic's text blocks and Codex's messages (Muse's answer is handed over at its end).
    fn text_of(&mut self, msg: &Value) {
        let mut said = |t: &str| self.text = format!("{}\n\n{t}", self.text).trim().to_string();
        if msg["type"] == "assistant" {
            let blocks = msg["message"]["content"].as_array().cloned();
            blocks
                .into_iter()
                .flatten()
                .filter(|b| b["type"] == "text")
                .for_each(|b| said(b["text"].as_str().unwrap_or("")));
        } else if msg["type"] == "item.completed" && msg["item"]["type"] == "agent_message" {
            said(msg["item"]["text"].as_str().unwrap_or(""));
        }
    }
}

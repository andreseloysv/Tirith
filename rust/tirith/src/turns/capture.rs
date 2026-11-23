//! What a turn read, as its agent saw it: the message that started it (without the memory OpenBot lays before it) and, in order, each tool call with its input and its result — Anthropic's `tool_result` blocks in its `user` lines, Muse's `tool.result`, Codex's and Copilot's calls — up to 256 KB a turn.
use super::capture_fit::{fitted, text_of};
use super::capture_told::taken;
use serde_json::{json, Value};

/// The most a turn's capture keeps, in bytes of its JSON.
pub const LIMIT: usize = 256 * 1024;

/// One turn's capture while it runs.
#[derive(Default)]
pub struct Capture {
    pub start: String,
    pub calls: Vec<Value>,
    bytes: usize,
    pub full: bool,
}

impl Capture {
    /// The capture of a turn of `conv` that starts now, with what was handed to it.
    pub fn of(conv: &str) -> Capture {
        let start = taken(conv);
        let start: String = start.join("\n\n").chars().take(LIMIT / 4).collect();
        let bytes = start.len();
        Capture {
            start,
            bytes,
            ..Default::default()
        }
    }

    /// Takes one line of the agent's output.
    pub fn take(&mut self, msg: &Value) {
        let blocks = msg["message"]["content"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        match msg["payload_type"].as_str().or(msg["type"].as_str()) {
            Some("assistant") => blocks
                .iter()
                .filter(|b| b["type"] == "tool_use")
                .for_each(|b| {
                    let call = json!({"tool": b["name"], "id": b["id"], "input": b["input"]});
                    self.add(call, None)
                }),
            Some("user") => blocks
                .iter()
                .filter(|b| b["type"] == "tool_result")
                .for_each(|b| self.result(&b["tool_use_id"], text_of(&b["content"]))),
            Some("tool.result") => {
                let (call, text) = super::exec_items::muse(&msg["payload"]);
                self.add(call, Some(text))
            }
            _ => self.exec(msg),
        }
    }

    /// A line of Codex or Copilot: a call that starts, or one's result.
    fn exec(&mut self, msg: &Value) {
        if let Some((id, tool, input)) = super::exec_items::started(msg) {
            self.add(json!({"tool": tool, "id": id, "input": input}), None);
        }
        if let Some((id, text)) = super::exec_items::ended(msg) {
            self.result(&id, text);
        }
    }

    /// One call (with its result when it came with it); none once the capture is full.
    fn add(&mut self, call: Value, result: Option<String>) {
        let Some(()) = (!self.full).then_some(()) else {
            return;
        };
        let room = LIMIT.saturating_sub(self.bytes);
        let (call, cut) = match result {
            Some(r) => fitted(&call, &r, room),
            None => (Some(call).filter(|c| c.to_string().len() <= room), false),
        };
        self.full |= cut || call.is_none();
        let Some(call) = call else { return };
        self.bytes += call.to_string().len();
        self.calls.push(call);
    }

    /// The result of Anthropic's call `id`, as much of it as the capture still has room for.
    fn result(&mut self, id: &Value, text: String) {
        let open = |c: &&mut Value| c["id"] == *id && c.get("result").is_none();
        let Some(call) = self.calls.iter_mut().rev().find(open) else {
            return;
        };
        let before = call.to_string().len();
        let (with, cut) = fitted(call, &text, LIMIT.saturating_sub(self.bytes) + before);
        self.full |= cut || with.is_none();
        let Some(with) = with else { return };
        self.bytes = self.bytes - before + with.to_string().len();
        *call = with;
    }
}

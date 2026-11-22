//! Whether a turn's calls worked: how each ended and what error it printed, in any agent's lines; and the agent's own error.
use super::capture_fit::text_of;
use super::look_lines::{blocks, whole, Ended};
use super::look_muse::Said;
use serde_json::Value;
use std::collections::BTreeMap;

/// A turn's calls as they ended: the last try of each counts.
#[derive(Default)]
pub struct Looks {
    open: BTreeMap<String, String>,
    pub(super) ended: Vec<Ended>,
    /// Commands whose end has not come: they went on in the background.
    pub(super) waiting: Vec<String>,
    /// What the agent said of its own failure, where that is not its answer.
    pub error: String,
}

impl Looks {
    /// Takes one line of the agent's output, or one record of its session log.
    pub(crate) fn take(&mut self, msg: &Value) {
        let data = &msg["data"];
        match whole(msg) {
            Some(Said::Ended(ended)) => return self.ended(ended),
            Some(Said::Waits(what)) => return self.waiting.push(what),
            None => {}
        }
        match msg["type"].as_str().unwrap_or("") {
            "assistant" => blocks(msg, "tool_use").for_each(|b| {
                let name = b["name"].as_str().unwrap_or("");
                self.starts(&b["id"], crate::rules::summary(name, &b["input"]));
            }),
            "user" => blocks(msg, "tool_result").for_each(|b| {
                let said = text_of(&b["content"]);
                self.ends(&b["tool_use_id"], b["is_error"] == true, &said)
            }),
            "tool.execution_start" => {
                let name = data["toolName"].as_str().unwrap_or("");
                let what = crate::rules::summary(name, &data["arguments"]);
                self.starts(&data["toolCallId"], what);
            }
            "tool.execution_complete" => {
                let said = [
                    text_of(&data["result"]["content"]),
                    text_of(&data["error"]["message"]),
                ];
                let failed = data["success"] == false;
                self.ends(&data["toolCallId"], failed, &said.join("\n"))
            }
            "result" if msg["is_error"] == true => {
                self.error = msg["result"].as_str().unwrap_or("").into();
            }
            _ => {}
        }
    }

    fn ended(&mut self, ended: Ended) {
        if let Some(at) = self.waiting.iter().position(|what| *what == ended.what) {
            self.waiting.remove(at);
        }
        self.ended.push(ended);
    }

    fn starts(&mut self, id: &Value, what: String) {
        self.open.insert(id.to_string(), what);
    }

    fn ends(&mut self, id: &Value, failed: bool, said: &str) {
        if let Some(what) = self.open.remove(&id.to_string()) {
            self.ended(Ended::of(what, failed, said));
        }
    }
}

//! Whether a turn's looks worked: each tool call's outcome in any agent's lines, and the agent's own error.
use super::look_lines::{blocks, muse};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::LazyLock;

/// A call that looks at something outside.
static OUTSIDE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(?i)safari|curl|wget|fetch|web_?search|browser|playwright|navigate|mcp__|https?://",
    )
    .unwrap()
});

/// A turn's calls as they ended, each by what it did: the last try of each counts.
#[derive(Default)]
pub struct Looks {
    open: BTreeMap<String, String>,
    ended: Vec<(String, bool)>,
    /// What the agent said of its own failure, where that is not its answer.
    pub error: String,
}

impl Looks {
    /// Takes one line of the agent's output.
    pub(crate) fn take(&mut self, msg: &Value) {
        let (data, item) = (&msg["data"], &msg["item"]);
        match msg["payload_type"]
            .as_str()
            .or(msg["type"].as_str())
            .unwrap_or("")
        {
            "assistant" => blocks(msg, "tool_use").for_each(|b| {
                let name = b["name"].as_str().unwrap_or("");
                self.starts(&b["id"], crate::rules::summary(name, &b["input"]));
            }),
            "user" => blocks(msg, "tool_result")
                .for_each(|b| self.ends(&b["tool_use_id"], b["is_error"] == true)),
            "tool.execution_start" => {
                let name = data["toolName"].as_str().unwrap_or("");
                self.starts(
                    &data["toolCallId"],
                    crate::rules::summary(name, &data["arguments"]),
                );
            }
            "tool.execution_complete" => self.ends(&data["toolCallId"], data["success"] == false),
            "item.completed" if item["type"] == "command_execution" => {
                let failed = item["status"] == "failed"
                    || item["exit_code"].as_i64().is_some_and(|c| c != 0);
                self.ended
                    .push((item["command"].as_str().unwrap_or("").into(), failed));
            }
            "item.completed" if item["type"] == "mcp_tool_call" => {
                let part = |k: &str| item[k].as_str().unwrap_or("").to_string();
                let failed = item["status"] == "failed" || !item["error"].is_null();
                self.ended
                    .push((format!("mcp__{}__{}", part("server"), part("tool")), failed));
            }
            "tool.result" => self.ended.push(muse(&msg["payload"])),
            "result" if msg["is_error"] == true => {
                self.error = msg["result"].as_str().unwrap_or("").into();
            }
            _ => {}
        }
    }

    fn starts(&mut self, id: &Value, what: String) {
        self.open.insert(id.to_string(), what);
    }

    fn ends(&mut self, id: &Value, failed: bool) {
        if let Some(what) = self.open.remove(&id.to_string()) {
            self.ended.push((what, failed));
        }
    }

    /// The look that did not work: one outside that failed and never worked after, or the last call when none worked at all.
    pub fn blind(&self) -> Option<&str> {
        let retried =
            |at: usize, what: &str| self.ended[at + 1..].iter().any(|(w, f)| w == what && !f);
        let outside =
            self.ended.iter().enumerate().find(|(at, (what, failed))| {
                *failed && OUTSIDE.is_match(what) && !retried(*at, what)
            });
        let none_worked = || {
            self.ended
                .last()
                .filter(|_| self.ended.iter().all(|(_, f)| *f))
        };
        outside
            .map(|(_, (what, _))| what.as_str())
            .or_else(|| none_worked().map(|(what, _)| what.as_str()))
    }
}

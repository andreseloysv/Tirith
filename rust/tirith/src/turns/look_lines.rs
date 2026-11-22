//! How a tool call ended, as any agent's lines say it.
use super::broke::{broke, end, silent, Broke};
use super::capture_fit::text_of;
use super::look_muse::{self, Said};
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

/// A search: its status 1 with nothing printed is «no match», which is no failure (what it searched is the judge's to doubt).
static MATCHING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(^|[|;&:(]\s*)([ef]?grep|rg|diff)\b[^|;&]*$").unwrap());

/// One ended call: what it did, whether it failed, the error it printed and the last thing it said.
#[derive(Clone, Debug)]
pub(super) struct Ended {
    pub what: String,
    pub failed: bool,
    pub broke: Option<Broke>,
    pub end: String,
}

impl Ended {
    pub(super) fn of(what: String, failed: bool, output: &str) -> Ended {
        let failed = failed && !(MATCHING.is_match(&what) && silent(output));
        let broke = broke(output, failed);
        Ended {
            what,
            failed,
            broke,
            end: end(output),
        }
    }
}

/// The content blocks of one kind in a message.
pub(super) fn blocks<'a>(msg: &'a Value, kind: &'a str) -> impl Iterator<Item = &'a Value> + 'a {
    let all = msg["message"]["content"].as_array().into_iter().flatten();
    all.filter(move |b| b["type"] == kind)
}

/// A call that says all of itself on one line: Codex's and Muse's.
pub(super) fn whole(msg: &Value) -> Option<Said> {
    let item = &msg["item"];
    match msg["payload_type"].as_str().or(msg["type"].as_str())? {
        "item.completed" if item["type"] == "command_execution" => {
            let code = item["exit_code"].as_i64();
            let failed = item["status"] == "failed" || code.is_some_and(|c| c != 0);
            let what = item["command"].as_str().unwrap_or("").to_string();
            let said = text_of(&item["aggregated_output"]);
            Some(Said::Ended(Ended::of(what, failed, &said)))
        }
        "item.completed" if item["type"] == "mcp_tool_call" => {
            let part = |k: &str| item[k].as_str().unwrap_or("").to_string();
            let failed = item["status"] == "failed" || !item["error"].is_null();
            let said = [
                text_of(&item["result"]["content"]),
                text_of(&item["error"]["message"]),
            ];
            let what = format!("mcp__{}__{}", part("server"), part("tool"));
            Some(Said::Ended(Ended::of(what, failed, &said.join("\n"))))
        }
        "tool.result" => Some(look_muse::result(&msg["payload"])),
        "runtime.session" => look_muse::record(&msg["payload"]).map(Said::Ended),
        _ => None,
    }
}

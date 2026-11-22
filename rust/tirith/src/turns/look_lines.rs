//! How a tool call ended, as any agent's lines say it.
use super::broke::{broke, silent, Broke};
use super::capture_fit::text_of;
use super::look_muse::{self, Said};
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

/// A local search: its status 1 with nothing printed is «no match», which is no failure.
static MATCHING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(^|[|;&:(]\s*)([ef]?grep|rg|diff)\b[^|;&]*$").unwrap());

/// One ended call: what it did, whether it failed, and the error it printed.
#[derive(Clone, Debug)]
pub(super) struct Ended {
    pub what: String,
    pub failed: bool,
    pub broke: Option<Broke>,
}

impl Ended {
    pub(super) fn of(what: String, failed: bool, output: &str) -> Ended {
        // of a look outside the silence may be the failure itself (`curl -s … | grep`)
        let none_matched = MATCHING.is_match(&what) && !super::proof::outside(&what);
        let failed = failed && !(none_matched && silent(output));
        let broke = broke(output, failed);
        Ended {
            what,
            failed,
            broke,
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

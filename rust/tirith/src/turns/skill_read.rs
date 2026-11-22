//! The skill a call read. A path that only stands in what came back, in a prompt handed on, or in a file being written is no read of it.
use super::tools::Call;
use serde_json::Value;
use std::sync::LazyLock;

static SKILL_MD: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"skills/([A-Za-z0-9_.:-]+)/SKILL\.md").unwrap());
/// A quoted argument, or a file a command's output goes to.
static ASIDE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#""(?:\\.|[^"\\])*"|'[^']*'|(?:>{1,2}|\btee(?:\s+-a)?\s)\s*[^\s;&|<>]+"#)
        .unwrap()
});

/// The skill `call` read: Anthropic's `Skill`, Muse's `read_skill`, a file tool on `skills/<name>/SKILL.md`, or a command with that file as an argument.
pub fn skill_of(call: &Call) -> Option<String> {
    let named = |v: &Value| {
        ["skill", "name"]
            .iter()
            .find_map(|k| v[*k].as_str().map(String::from))
    };
    let by_tool = match call.name.as_str() {
        "Skill" => named(&call.input),
        "read_skill" => [&call.input, &call.input["input"], &call.input["arguments"]]
            .into_iter()
            .find_map(named),
        _ => None,
    };
    by_tool.or_else(|| {
        SKILL_MD
            .captures(&looked_at(call))
            .map(|c| c[1].to_string())
    })
}

/// What names the files a call reads: a command's arguments, a reading tool's input (Muse's is kept with its result).
fn looked_at(call: &Call) -> String {
    if let Some(command) = field(call, "command") {
        return arguments(&command);
    }
    let name = call.name.to_lowercase();
    let reads = ["read", "view", "skill"].iter().any(|w| name.contains(w));
    if reads {
        let path = ["file_path", "path"].iter().find_map(|k| field(call, k));
        path.unwrap_or_else(|| call.input.to_string())
    } else {
        String::new()
    }
}

/// One of a call's arguments; Muse's are in the JSON of its result, beside the output.
fn field(call: &Call, key: &str) -> Option<String> {
    let direct = call.input[key].as_str().map(String::from);
    direct.or_else(|| {
        let result: Value = serde_json::from_str(call.input["text"].as_str()?).ok()?;
        result[key].as_str().map(String::from)
    })
}

/// A command without where it writes and without its quoted prose; a quoted path stays.
fn arguments(command: &str) -> String {
    let kept = ASIDE.replace_all(command, |found: &regex::Captures| {
        let word = found[0].trim_matches(['"', '\'']);
        let path = word.starts_with(['/', '.', '~', '$']) || !word.contains(char::is_whitespace);
        let written = found[0].starts_with(['>', 't']);
        let path = path && !written && word.ends_with("SKILL.md");
        if path {
            word.to_string()
        } else {
            " ".to_string()
        }
    });
    kept.into_owned()
}

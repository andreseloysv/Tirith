//! MCP servers that are programs: Codex's own process takes none once it is told where commands run (it has no environment of its own then), so each runs where the commands do, under their limit, from a whole path.
use super::layers::Layers;
use std::path::Path;

/// The settings that put each there. They go last on the line: a caller's inline table of a server would replace them.
pub(super) fn words(set: &Layers, dir: &Path) -> Vec<String> {
    let mut words = vec![];
    for name in set.stdio() {
        let plain = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
        if !name.chars().all(plain) {
            continue; // Codex takes no such name
        }
        let key = |field: &str| format!("mcp_servers.{name}.{field}");
        let asked = set.get(&key("cwd"));
        let cwd = asked.as_ref().and_then(|v| v.as_str()).unwrap_or("");
        let cwd = if cwd.is_empty() {
            dir.to_path_buf()
        } else {
            dir.join(cwd)
        };
        let cwd = toml::Value::String(cwd.display().to_string());
        words.extend([
            "-c".to_string(),
            format!("{}=\"remote\"", key("environment_id")),
            "-c".to_string(),
            format!("{}={cwd}", key("cwd")),
        ]);
    }
    words
}

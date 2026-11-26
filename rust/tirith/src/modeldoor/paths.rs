//! The only paths the door forwards: a model provider's own, never a file, an account or an admin call.

/// Whether `method` may call `path` (the part after the base URL's own path).
pub fn known(method: &str, path: &str) -> bool {
    let path = path.trim_end_matches('/');
    let rest = path
        .strip_prefix("/v1")
        .or_else(|| path.strip_prefix("/openai/v1"))
        .unwrap_or(path);
    let parts: Vec<&str> = rest.trim_start_matches('/').split('/').collect();
    let post = matches!(
        parts.as_slice(),
        ["messages"]
            | ["messages", "count_tokens"]
            | ["chat", "completions"]
            | ["completions"]
            | ["responses"]
            | ["embeddings"]
    );
    let get = matches!(
        parts.as_slice(),
        ["models"] | ["models", _] | ["responses", _]
    );
    let azure = matches!(
        path.trim_start_matches('/')
            .split('/')
            .collect::<Vec<_>>()
            .as_slice(),
        [
            "openai",
            "deployments",
            _,
            "chat",
            "completions" | "responses"
        ] | [
            "openai",
            "deployments",
            _,
            "completions" | "embeddings" | "responses"
        ]
    );
    let clean = !path.contains("..") && !path.contains("//");
    clean
        && match method {
            "POST" => post || azure,
            "GET" => get,
            _ => false,
        }
}

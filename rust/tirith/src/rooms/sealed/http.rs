//! Requests to the platform.
use serde_json::Value;
use std::sync::OnceLock;
use std::time::Duration;
pub(super) fn http() -> &'static reqwest::Client {
    static C: OnceLock<reqwest::Client> = OnceLock::new();
    C.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("tirith-room/2")
            .build()
            .unwrap_or_default()
    })
}
/// One request; the JSON answer on 2xx, `(status, message)` otherwise (status 0: unreachable).
pub(super) async fn api(
    base: &str,
    path: &str,
    body: Option<&Value>,
    secs: u64,
) -> Result<Value, (u16, String)> {
    let url = format!("{base}{path}");
    let req = body.map_or_else(|| http().get(&url), |b| http().post(&url).json(b));
    let res = req
        .timeout(Duration::from_secs(secs))
        .send()
        .await
        .map_err(|e| (0, format!("cannot reach {base}: {e}")))?;
    let status = res.status().as_u16();
    let v: Value = res.json().await.unwrap_or(Value::Null);
    let Some(()) = (200..300).contains(&status).then_some(()) else {
        return Err((
            status,
            v["error"].as_str().map_or(format!("HTTP {status}"), |m| {
                if status >= 500 {
                    format!("HTTP {status}: {m}")
                } else {
                    m.into()
                }
            }),
        ));
    };
    Ok(v)
}

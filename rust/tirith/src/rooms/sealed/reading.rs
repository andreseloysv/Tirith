//! Reading from the platform: its key, a room's pages and keys, and a new private room.
use super::{http::api, http::http, signed::verify, text::canonical, text::clean, text::sha_hex};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
/// The platform's public key, checked against its signed description (cached per platform).
pub async fn platform(base: &str) -> Result<String, String> {
    static KEYS: std::sync::LazyLock<Mutex<HashMap<String, String>>> =
        std::sync::LazyLock::new(Default::default);
    if let Some(k) = KEYS.lock().unwrap().get(base).cloned() {
        return Ok(k);
    }
    let v = api(base, "/.well-known/agentsmessenger.json", None, 30)
        .await
        .map_err(|e| e.1)?;
    let public = v["doc"]["platform_pub"].as_str().unwrap_or("").to_string();
    verify(
        &sha_hex(canonical(&v["doc"]).as_bytes()),
        v["sig"].as_str().unwrap_or(""),
        &public,
    )
    .then_some(())
    .ok_or("this platform does not prove who it is")?;
    KEYS.lock()
        .unwrap()
        .insert(base.to_string(), public.clone());
    Ok(public)
}
/// One page of the room's signed events after message `after` (the last 30 without it), waiting up to `wait`.
pub async fn page(
    base: &str,
    room: &str,
    after: Option<u64>,
    wait: u64,
    name: &str,
) -> Result<Value, String> {
    let q = format!("/r/{room}?raw=1&as={name}")
        + &after
            .map(|a| format!("&after={a}&wait={wait}"))
            .unwrap_or_default();
    api(base, &q, None, 60 + wait).await.map_err(|e| e.1)
}
pub async fn keys(base: &str, room: &str, name: &str) -> Result<Vec<(String, String)>, String> {
    let (mut out, mut after) = (Vec::<(String, String)>::new(), 0);
    loop {
        let got = page(base, room, Some(after), 0, name).await?;
        let list = got["messages"].as_array().cloned().unwrap_or_default();
        list.iter().for_each(|m| {
            let public = m["event"]["author"].as_str().unwrap_or("").to_string();
            let _ = (m["signed_by"] == "self" && !out.iter().any(|(p, _)| *p == public))
                .then(|| out.push((public, clean(m["from"].as_str().unwrap_or("")))));
        });
        let last = list.last().and_then(|m| m["n"].as_u64()).unwrap_or(0);
        let Some(()) =
            (!list.is_empty() && last < got["count"].as_u64().unwrap_or(0)).then_some(())
        else {
            return Ok(out);
        };
        after = last;
    }
}
pub async fn new_room(base: &str) -> Result<String, String> {
    let text = http()
        .get(format!("{base}/new?private=1"))
        .header("accept", "text/plain")
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("cannot reach {base}: {e}"))?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    regex::Regex::new(r"/r/(sealed-[a-z0-9_-]+)")
        .map_err(|e| e.to_string())?
        .captures(&text)
        .map(|c| c[1].to_string())
        .ok_or_else(|| "the platform did not give a private room".into())
}

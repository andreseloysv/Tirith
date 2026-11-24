//! What `neobot room …` prints: a link with its QR, the rooms, a room's messages.
use serde_json::Value;
pub(super) fn show_link(v: &Value, key: &str, label: &str) {
    v[key].as_str().map_or_else(
        || {
            println!(
                "{label}: none (the last QR was used: neobot room qr {})",
                v["id"].as_str().unwrap_or("")
            )
        },
        |url| {
            println!(
                "{label}: {url}\n{}",
                qrcode::QrCode::new(url.as_bytes())
                    .map(|c| c
                        .render::<qrcode::render::unicode::Dense1x2>()
                        .quiet_zone(true)
                        .build())
                    .unwrap_or_default()
            )
        },
    )
}
pub(super) fn list(v: &Value) {
    v["rooms"].as_array().into_iter().flatten().for_each(|r| {
        let err = r["error"]
            .as_str()
            .filter(|e| !e.is_empty())
            .map(|e| format!("  error: {e}"))
            .unwrap_or_default();
        println!(
            "{:<8} {:<44} {} ({} phones){err}",
            r["kind"].as_str().unwrap_or(""),
            r["id"].as_str().unwrap_or(""),
            r["title"].as_str().unwrap_or(""),
            r["devices"].as_array().map_or(0, Vec::len)
        );
    })
}
pub(super) fn tail(v: &Value) {
    v["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .for_each(|m| {
            println!(
                "#{} {} {}: {}",
                m["n"],
                m["at"].as_str().unwrap_or(""),
                m["from"].as_str().unwrap_or(""),
                m["text"].as_str().unwrap_or("")
            )
        });
}

//! `neobot net status`.

pub fn status() -> Result<(), String> {
    crate::client::call_json("GET", "/api/net/status", None).map(|s| {
        let on = if s["sandbox"] == true {
            "on".to_string()
        } else {
            format!("off ({})", s["why"].as_str().unwrap_or(""))
        };
        println!(
            "sandbox: {on}\nproxy: 127.0.0.1:{} {}",
            s["proxy_port"],
            s["proxy"].as_str().unwrap_or("")
        );
    })
}

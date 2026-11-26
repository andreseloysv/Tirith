//! The redirect URI of a native app (RFC 8252): a port of this computer, picked at random, that the browser is sent
//! back to with the code. It answers the one request that carries `code` (or `error`) and `state`, and nothing else.
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

pub struct Redirect {
    listener: TcpListener,
}

impl Redirect {
    pub fn open() -> Result<Redirect, String> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        Ok(Redirect { listener })
    }

    /// The URI to register at the provider as `http://localhost` (it ignores the port) and to send.
    pub fn uri(&self) -> String {
        let port = self.listener.local_addr().map(|a| a.port()).unwrap_or(0);
        format!("http://localhost:{port}")
    }

    /// The code, once the browser comes back with `state`; an error the provider reported, or a timeout.
    pub fn code(&self, state: &str, wait: Duration) -> Result<String, String> {
        let end = Instant::now() + wait;
        while Instant::now() < end {
            let Ok((mut conn, _)) = self.listener.accept() else {
                std::thread::sleep(Duration::from_millis(50));
                continue;
            };
            let _ = conn.set_nonblocking(false);
            let _ = conn.set_read_timeout(Some(Duration::from_secs(2)));
            let mut buf = [0u8; 4096];
            let n = conn.read(&mut buf).unwrap_or(0);
            let line = String::from_utf8_lossy(&buf[..n]);
            let target = line.split_whitespace().nth(1).unwrap_or("/");
            let url = reqwest::Url::parse(&format!("http://localhost{target}")).ok();
            let q = |k: &str| {
                url.as_ref().and_then(|u| {
                    u.query_pairs()
                        .find(|(n, _)| n == k)
                        .map(|(_, v)| v.to_string())
                })
            };
            let mut done = |text: &str| {
                let body = format!("<!doctype html><meta charset=utf-8><title>NeoBot</title><p style=\"font:16px system-ui;margin:20vh auto;max-width:28rem\">{text}");
                let head = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close";
                let _ = conn.write_all(
                    format!("{head}\r\nContent-Length: {}\r\n\r\n{body}", body.len()).as_bytes(),
                );
            };
            if q("state").as_deref() != Some(state) || (q("code").is_none() && q("error").is_none())
            {
                done("Nothing to do here.");
                continue; // a favicon, a probe, a stale tab: not the answer
            }
            if let Some(e) = q("error") {
                done("Sign-in was refused. You can close this tab.");
                return Err(format!(
                    "{e}: {}",
                    q("error_description").unwrap_or_default()
                ));
            }
            done("You are signed in to NeoBot. You can close this tab.");
            return q("code").ok_or_else(|| "no code".into());
        }
        Err("sign-in timed out".into())
    }
}

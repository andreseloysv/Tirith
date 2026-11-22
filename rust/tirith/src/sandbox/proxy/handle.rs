//! One caller of the proxy, from its request head to the end of the connection.
use super::{
    answer::answer,
    auth::caller,
    forward::forward,
    request::{host_port, Request},
    upstream::upstream,
};
use crate::app::Shared;
use std::time::Duration;
use tokio::{io::AsyncReadExt, net::TcpStream};

pub async fn handle(app: Shared, mut c: TcpStream) -> Result<(), String> {
    let Some((buf, end)) = read_head(&mut c).await? else {
        return Ok(());
    };
    let head = String::from_utf8_lossy(&buf[..end]).to_string();
    let req = Request::parse(&head);
    let Some(who) = caller(&app, &req.headers) else {
        return answer(
            &mut c,
            "407 Proxy Authentication Required",
            "Proxy-Authenticate: Basic realm=\"tirith\"\r\n",
            "Tirith's proxy needs the credentials of HTTPS_PROXY.\n",
        )
        .await;
    };
    let Some((hostport, path)) = req.destination() else {
        return answer(
            &mut c,
            "400 Bad Request",
            "",
            "Only CONNECT and http:// requests go through Tirith's proxy.\n",
        )
        .await;
    };
    let Some((host, port)) = host_port(hostport, if req.connect() { 443 } else { 80 }) else {
        return answer(&mut c, "400 Bad Request", "", "No destination host.\n").await;
    };
    let Some(mut up) = upstream(&app, &mut c, &who, &host, port).await? else {
        return Ok(());
    };
    forward(&req, &path, &mut c, &mut up, &buf[end..]).await
}

/// The bytes read and where the head ends; `None` when the caller closed first or the head is too large (answered).
async fn read_head(c: &mut TcpStream) -> Result<Option<(Vec<u8>, usize)>, String> {
    let mut buf = Vec::with_capacity(2048);
    loop {
        let mut chunk = [0u8; 4096];
        let n = tokio::time::timeout(Duration::from_secs(30), c.read(&mut chunk))
            .await
            .map_err(|_| "slow client")?
            .map_err(|e| e.to_string())?;
        let Some(()) = (n > 0).then_some(()) else {
            return Ok(None);
        };
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            return Ok(Some((buf, i + 4)));
        }
        let Some(()) = (buf.len() <= 65536).then_some(()) else {
            answer(c, "431 Request Header Fields Too Large", "", "").await?;
            return Ok(None);
        };
    }
}

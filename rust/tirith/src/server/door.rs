//! The daemon's door. It is loopback only, and on Linux, where a server has more users, a connection comes in only from this Tirith's own user, root, or one of `[server] users` in config.toml (who signs in and tunnels to the page).
use crate::app::Shared;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const WHY: &str =
    "this Tirith serves its own user, root and the [server] users of its config.toml\n";

pub struct Door {
    listener: TcpListener,
    app: Shared,
}

pub fn door(listener: TcpListener, app: Shared) -> Door {
    Door { listener, app }
}

impl axum::serve::Listener for Door {
    type Io = TcpStream;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (TcpStream, SocketAddr) {
        loop {
            match self.listener.accept().await {
                Ok((stream, from)) if super::peer::welcome(&self.app, &stream, from) => {
                    return (stream, from)
                }
                Ok((stream, _)) => drop(tokio::spawn(refuse(stream))),
                Err(_) => tokio::time::sleep(Duration::from_millis(100)).await, // out of descriptors for a moment
            }
        }
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }
}

/// Says why and closes; what the caller sent is read and dropped, so that it can read the answer.
async fn refuse(mut stream: TcpStream) {
    let head = "HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain\r\nConnection: close";
    let said = format!("{head}\r\nContent-Length: {}\r\n\r\n{WHY}", WHY.len());
    let _ = stream.write_all(said.as_bytes()).await;
    let _ = stream.shutdown().await;
    let mut unread = [0u8; 4096];
    let drain = async { while matches!(stream.read(&mut unread).await, Ok(n) if n > 0) {} };
    let _ = tokio::time::timeout(Duration::from_secs(2), drain).await;
}

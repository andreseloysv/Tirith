//! The provider's answer on its way back: passed on as it comes, counted by what it says it used.
use super::record::{keep, Call as Row};
use super::tally::Tally;
use axum::body::{Body, Bytes};
use axum::response::{IntoResponse, Response};
use std::time::Instant;
use tokio_stream::wrappers::ReceiverStream;

/// `answer` as the caller gets it (its status and headers as they are, its body chunk by chunk); the call is kept when it ends, or when the caller leaves.
pub(super) fn back(
    app: &crate::app::Shared,
    answer: reqwest::Response,
    mut row: Row,
    started: Instant,
) -> Response {
    row.status = answer.status().as_u16();
    let content = answer.headers().get("content-type");
    let stream = content.is_some_and(|t| t.as_bytes().starts_with(b"text/event-stream"));
    let mut out = Response::builder().status(answer.status());
    let hop = ["connection", "transfer-encoding", "content-length"];
    for (name, value) in answer
        .headers()
        .iter()
        .filter(|(n, _)| !hop.contains(&n.as_str()))
    {
        out = out.header(name.clone(), value.clone());
    }
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, std::io::Error>>(16);
    let app = app.clone();
    tokio::spawn(async move {
        let (mut answer, mut tally) = (answer, Tally::default());
        while let Ok(Some(chunk)) = answer.chunk().await {
            tally.feed(&chunk, stream);
            if tx.send(Ok(chunk)).await.is_err() {
                break; // the caller left: nothing more is read from the provider
            }
        }
        tally.end();
        row.ms = started.elapsed().as_millis() as i64;
        keep(&app, &row, &tally);
    });
    out.body(Body::from_stream(ReceiverStream::new(rx)))
        .unwrap_or_else(|e| e.to_string().into_response())
}

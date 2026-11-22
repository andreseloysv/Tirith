//! The page is the owner's by the owner's token: `tirith open` gets a link with it that works once (never the token, which a command line or a history would show), and the link leaves the browser a cookie derived from the token.
use crate::app::{App, Shared};
use crate::rooms::sealed::{hex, sha_hex};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const LINK_LIVES: Duration = Duration::from_secs(600); // long enough to open an SSH tunnel
static LINKS: Mutex<Vec<(String, Instant)>> = Mutex::new(Vec::new());

pub(super) fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/page", post(made))
        .route("/auth/{code}", get(enter))
}

/// A link's row: it is this Tirith's alone.
fn row(app: &App, code: &str) -> String {
    sha_hex(format!("{}\n{code}", app.keys.owner).as_bytes())
}

/// `<name>=<value>` of this Tirith's cookie. Every port of 127.0.0.1 shares cookies, so the name is this home's.
pub(super) fn cookie(app: &App) -> String {
    let of = |what: &str| sha_hex(format!("{what}\n{}", app.keys.owner).as_bytes());
    format!("tirith-{}={}", &of("name")[..8], of("page"))
}

pub(super) fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0, |d, (x, y)| d | (x ^ y)) == 0
}

/// Whether the request is the opened page's: its cookie, and for the API not another site's request (another port is one) nor an address somebody had the browser visit.
pub(super) fn opened(app: &App, path: &str, h: &HeaderMap) -> bool {
    let header = |name: &str| h.get(name).and_then(|v| v.to_str().ok());
    let own = cookie(app);
    let kept = h
        .get_all("cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .any(|pair| same(pair.trim(), &own));
    if !kept || !path.starts_with("/api/") {
        return kept;
    }
    let ours = header("sec-fetch-site").is_none_or(|s| s == "same-origin" || s == "none");
    let visited = header("sec-fetch-dest") == Some("document");
    ours && (!visited || path.starts_with("/api/attachments/"))
}

/// The one route that opens the page: its code is its token.
pub(super) fn link(method: &Method, path: &str) -> bool {
    let code = path.strip_prefix("/auth/").unwrap_or("");
    method == Method::GET && !code.is_empty() && !code.contains('/')
}

async fn made(State(app): State<Shared>) -> Json<Value> {
    let code = hex(&rand::random::<[u8; 24]>());
    let mut links = LINKS.lock().unwrap();
    links.retain(|(_, at)| at.elapsed() < LINK_LIVES);
    links.push((row(&app, &code), Instant::now()));
    let port = app.cfg().port;
    Json(json!({"url": format!("http://127.0.0.1:{port}/auth/{code}")}))
}

async fn enter(State(app): State<Shared>, Path(code): Path<String>) -> Response {
    let wanted = row(&app, &code);
    let known = {
        let mut links = LINKS.lock().unwrap();
        links.retain(|(_, at)| at.elapsed() < LINK_LIVES);
        let at = links.iter().position(|(link, _)| same(link, &wanted));
        at.map(|at| links.remove(at)).is_some()
    };
    if !known {
        let why = "this link was used or is too old: run tirith open again";
        return (StatusCode::FORBIDDEN, why).into_response();
    }
    let kept = format!(
        "{}; Path=/; Max-Age=34560000; HttpOnly; SameSite=Lax",
        cookie(&app)
    );
    // Not a redirect: the code leaves the history with it.
    let on = "<!doctype html><title>Tirith</title><script>location.replace('/')</script>";
    ([("set-cookie", kept)], Html(on)).into_response()
}

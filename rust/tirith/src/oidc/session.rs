//! What both ways into a session share: the proven token's person is kept, a session opens for them and the
//! sign-in is an event; a refusal is an event too.
use super::verify::{self, Claims};
use super::{people, Settings};
use crate::app::{now_iso, Shared};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub(super) fn refuse(app: &Shared, code: StatusCode, why: &str) -> Response {
    app.event(
        "",
        "",
        "auth",
        "refused",
        &format!("sign-in refused: {why}"),
        "",
    );
    (code, Json(json!({ "error": why }))).into_response()
}

/// A proven token's person is kept, a session opens for them and the sign-in is an event: what both ways in share.
pub(super) async fn start_session(
    app: &Shared,
    s: &Settings,
    token: &str,
) -> Result<(Claims, String), String> {
    let claims = verify::check(s, token).await?;
    people::upsert(app, &claims);
    let session = people::open_session(app, &claims.oid, s.hours);
    let said = format!(
        "{} ({}) signed in at {}",
        claims.name,
        claims.level.name(),
        now_iso()
    );
    app.event("", "", "auth", "login", &said, "");
    Ok((claims, session))
}

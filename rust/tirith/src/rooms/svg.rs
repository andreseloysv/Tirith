//! A room's link as a QR picture for the page: the one-use pairing QR (`?kind=invite`) or the permanent link.
use super::{store::known, view::view};
use crate::app::Shared;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::collections::HashMap;
pub(super) async fn api_svg(
    State(app): State<Shared>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let Ok(r) = known(&id) else {
        return (StatusCode::NOT_FOUND, "no such room").into_response();
    };
    let kind = if q.get("kind").map(String::as_str) == Some("invite") {
        "invite"
    } else {
        "link"
    };
    let Some(url) = view(&app, &r)[kind].as_str().map(String::from) else {
        return (StatusCode::NOT_FOUND, "no such link").into_response();
    };
    qrcode::QrCode::new(url.as_bytes()).map_or_else(
        |e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        |code| {
            (
                [
                    ("content-type", "image/svg+xml"),
                    ("cache-control", "no-store"),
                ],
                svg(code),
            )
                .into_response()
        },
    )
}
fn svg(c: qrcode::QrCode) -> String {
    c.render::<qrcode::render::svg::Color>()
        .min_dimensions(220, 220)
        .dark_color(qrcode::render::svg::Color("#000"))
        .light_color(qrcode::render::svg::Color("#fff"))
        .build()
}

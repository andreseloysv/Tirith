//! What a JSON API handler answers.
use serde_json::Value;

pub type R = Result<axum::Json<Value>, (axum::http::StatusCode, String)>;

pub fn bad(e: String) -> (axum::http::StatusCode, String) {
    (axum::http::StatusCode::BAD_REQUEST, e)
}

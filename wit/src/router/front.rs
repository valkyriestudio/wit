use axum::{Router, response::Redirect, routing::get};

pub(crate) fn router() -> Router {
    Router::new().route("/", get(|| async { Redirect::temporary("/git") }))
}

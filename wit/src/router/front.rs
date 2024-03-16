use axum::{response::Redirect, routing::get, Router};

pub(crate) fn router() -> Router {
    Router::new().route("/", get(|| async { Redirect::temporary("/git") }))
}

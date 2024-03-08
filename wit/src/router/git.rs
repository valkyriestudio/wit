use axum::{response::Html, routing::get, Router};

pub(crate) fn router() -> Router {
    Router::new().route("/", get(browse))
}

async fn browse() -> Html<&'static str> {
    Html("<p>Hello, World!</p>")
}

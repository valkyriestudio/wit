use axum::Router;
use tower_http::services::ServeDir;

pub(crate) fn router() -> Router {
    Router::new().nest_service("/assets", ServeDir::new("assets"))
}

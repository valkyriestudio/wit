use axum::{http::StatusCode, routing::get, Router};

pub(crate) fn router() -> Router {
    Router::new().route("/healthz", get(health))
}

async fn health() -> StatusCode {
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::response::IntoResponse;

    #[tokio::test]
    async fn test_health() {
        let response = health().await.into_response();

        assert!(response.status() == StatusCode::OK);
    }
}

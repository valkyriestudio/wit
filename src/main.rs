use axum::{http::StatusCode, routing::get, Router};
use mimalloc::MiMalloc;
use std::net::SocketAddr;
use tower_http::{
  trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
  LatencyUnit,
};
use tracing::Level;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn create_app() -> Router {
  let app = Router::new()
    .route("/", get(health))
    .layer(
      TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().include_headers(true))
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(
          DefaultOnResponse::new()
            .level(Level::INFO)
            .latency_unit(LatencyUnit::Micros),
        ),
    )
    .route("/healthz", get(health));

  app
}

#[tokio::main]
async fn main() {
  env_logger::init();

  let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
  let app = create_app();

  tracing::info!("listening on {}", addr);

  let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
  axum::serve(listener, app).await.unwrap();
}

pub async fn health() -> StatusCode {
  StatusCode::OK
}

#[cfg(test)]
mod tests {
  use axum::{http::StatusCode, response::IntoResponse};

  #[tokio::test]
  async fn test_health() {
    let response = crate::health().await.into_response();

    assert!(response.status() == StatusCode::OK);
  }
}

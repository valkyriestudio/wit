mod git;
mod health_check;

use std::{iter::once, time::Duration};

use axum::{http::header, Router};
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    cors::CorsLayer,
    request_id::MakeRequestUuid,
    sensitive_headers::{SetSensitiveRequestHeadersLayer, SetSensitiveResponseHeadersLayer},
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    LatencyUnit, ServiceBuilderExt,
};

pub(crate) fn create_app() -> Router {
    Router::new()
        .nest("/git", git::router())
        .layer(
            ServiceBuilder::new()
                .layer(CatchPanicLayer::new())
                .layer(SetSensitiveRequestHeadersLayer::new([
                    header::AUTHORIZATION,
                    header::COOKIE,
                    header::PROXY_AUTHORIZATION,
                ]))
                .set_x_request_id(MakeRequestUuid)
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(DefaultMakeSpan::new().include_headers(true))
                        .on_response(
                            DefaultOnResponse::new()
                                .include_headers(true)
                                .latency_unit(LatencyUnit::Micros),
                        ),
                )
                .propagate_x_request_id()
                .layer(SetSensitiveResponseHeadersLayer::new(once(
                    header::SET_COOKIE,
                )))
                .layer(CompressionLayer::new())
                .layer(CorsLayer::permissive())
                .layer(TimeoutLayer::new(Duration::from_secs(30))),
        )
        .merge(health_check::router())
}

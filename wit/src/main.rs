mod router;
mod service;

use std::net::{IpAddr, Ipv4Addr};

use axum::serve::{Listener, ListenerExt};
use mimalloc::MiMalloc;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let level = if cfg!(debug_assertions) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(level.into())
                .from_env_lossy(),
        )
        .init();

    let bind_address = std::env::var("WIT_BIND_ADDRESS")
        .map(|s| {
            s.parse::<IpAddr>().unwrap_or_else(|_| {
                tracing::error!("invalid ip address {s:?}");
                std::process::exit(1);
            })
        })
        .unwrap_or(Ipv4Addr::UNSPECIFIED.into());

    let port = std::env::var("WIT_PORT")
        .map(|s| {
            s.parse::<u16>().unwrap_or_else(|_| {
                tracing::error!("invalid port number {s:?}");
                std::process::exit(1);
            })
        })
        .unwrap_or(3000);

    let app = router::create_app();

    let listener = tokio::net::TcpListener::bind((bind_address, port))
        .await?
        .tap_io(|tcp_stream| {
            if let Err(err) = tcp_stream.set_nodelay(true) {
                tracing::warn!("failed to set TCP_NODELAY on incoming connection: {err:?}");
            }
        });

    tracing::info!("listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}

use std::sync::Arc;
use std::net::SocketAddr;

use axum::Router;
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

use project_white::server::{self, session::SessionStore};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let store = Arc::new(RwLock::new(SessionStore::new()));

    // Spawn garbage collection task (runs every 60 seconds).
    let gc_store = store.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            let removed = gc_store.write().await.garbage_collect();
            if removed > 0 {
                tracing::info!("garbage collected {} expired sessions", removed);
            }
        }
    });

    let app = Router::new()
        .route("/health", axum::routing::get(server::handlers::health))
        .merge(server::router(store));

    let addr: SocketAddr = std::env::var("PW_SERVER_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".into())
        .parse()
        .expect("invalid PW_SERVER_ADDR");

    tracing::info!("signaling server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

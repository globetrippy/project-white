use std::sync::Arc;
use std::net::SocketAddr;

use axum::{
    Router,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
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
        // Install scripts (embedded at compile time)
        .route("/install.sh", get(install_sh))
        .route("/install.ps1", get(install_ps1))
        // CLI binary download (served from the container filesystem)
        .route("/download/*path", get(download_binary))
        .merge(server::router(store));

    let addr_str = std::env::var("PW_SERVER_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".into());

    let addr: SocketAddr = match addr_str.parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: invalid PW_SERVER_ADDR '{}' — {}", addr_str, e);
            std::process::exit(1);
        }
    };

    tracing::info!("signaling server starting on {}", addr);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("error: failed to bind to {} — {}", addr, e);
            std::process::exit(1);
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("error: server stopped — {}", e);
        std::process::exit(1);
    }
}

// ─── Download & Install Handlers ────────────────────────────────

/// Serve the Unix install script (embedded from project root).
async fn install_sh() -> impl IntoResponse {
    (
        [("content-type", "text/x-shellscript")],
        include_str!("../../install.sh"),
    )
}

/// Serve the Windows install script (embedded from project root).
async fn install_ps1() -> impl IntoResponse {
    (
        [("content-type", "text/powershell")],
        include_str!("../../install.ps1"),
    )
}

/// Serve the pre-built `pw` CLI binary.
///
/// In the Docker image, the `pw` binary is built alongside the server
/// and placed at `/usr/local/bin/pw-cli`. This handler reads and serves it.
async fn download_binary() -> Result<impl IntoResponse, StatusCode> {
    let data = tokio::fs::read("/usr/local/bin/pw-cli")
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok((
        [
            ("content-type", "application/octet-stream"),
            ("content-disposition", "attachment; filename=\"pw\""),
        ],
        data,
    ))
}

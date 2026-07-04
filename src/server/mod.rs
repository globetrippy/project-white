pub mod codec;
pub mod handlers;
pub mod session;

use std::sync::Arc;

use axum::Router;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

use session::SessionStore;

pub fn router(store: Arc<RwLock<SessionStore>>) -> Router {
    Router::new()
        .route("/api/v1/session", axum::routing::post(handlers::create_session))
        .route(
            "/api/v1/session/{code}/join",
            axum::routing::post(handlers::join_session),
        )
        .route(
            "/api/v1/session/{code}/poll",
            axum::routing::get(handlers::poll_session),
        )
        .route(
            "/api/v1/session/{code}/approve",
            axum::routing::post(handlers::approve_session),
        )
        .route(
            "/api/v1/session/{code}",
            axum::routing::delete(handlers::delete_session),
        )
        .layer(CorsLayer::permissive())
        .with_state(store)
}

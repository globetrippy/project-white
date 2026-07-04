use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::RwLock;

use super::session::{PeerInfo, SessionStatus, SessionStore};

// ─── Shared State ──────────────────────────────────────────

pub type SharedStore = Arc<RwLock<SessionStore>>;

// ─── Request Types ─────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub public_key: String,
    pub addr: String,
}

#[derive(Deserialize)]
pub struct JoinSessionRequest {
    pub public_key: String,
    pub addr: String,
}

#[derive(Deserialize)]
pub struct ApproveRequest {
    pub sender_token: String,
}

// ─── Response Helper ───────────────────────────────────────

type ApiResponse = (StatusCode, Json<serde_json::Value>);

fn ok_json(v: serde_json::Value) -> ApiResponse {
    (StatusCode::OK, Json(v))
}

fn created_json(v: serde_json::Value) -> ApiResponse {
    (StatusCode::CREATED, Json(v))
}

fn err(status: StatusCode, msg: &str) -> ApiResponse {
    (status, Json(json!({ "error": msg })))
}

// ─── Handlers ──────────────────────────────────────────────

/// POST /api/v1/session
pub async fn create_session(
    State(store): State<SharedStore>,
    Json(req): Json<CreateSessionRequest>,
) -> ApiResponse {
    let mut store = store.write().await;

    let peer = PeerInfo {
        public_key: req.public_key,
        addr: req.addr,
    };
    let code = store.insert(peer);

    let session_id = store
        .get(&code)
        .map(|s| s.id.to_string())
        .unwrap_or_default();

    created_json(json!({ "code": code, "session_id": session_id }))
}

/// POST /api/v1/session/{code}/join
pub async fn join_session(
    State(store): State<SharedStore>,
    Path(code): Path<String>,
    Json(req): Json<JoinSessionRequest>,
) -> ApiResponse {
    let mut store = store.write().await;

    if !store.exists(&code) {
        return err(StatusCode::NOT_FOUND, "session not found");
    }

    if !store.check_rate_limit(&code) {
        return err(
            StatusCode::TOO_MANY_REQUESTS,
            "too many join attempts",
        );
    }

    let session = match store.get_mut(&code) {
        Some(s) => s,
        None => return err(StatusCode::NOT_FOUND, "session not found"),
    };

    if session.receiver.is_some() {
        return err(
            StatusCode::CONFLICT,
            "session already has a receiver",
        );
    }

    let receiver_fingerprint = compute_fingerprint(&req.public_key);

    session.receiver = Some(PeerInfo {
        public_key: req.public_key,
        addr: req.addr,
    });
    session.receiver_fingerprint = Some(receiver_fingerprint.clone());
    session.status = SessionStatus::AwaitingApproval;

    ok_json(json!({
        "sender_public_key": session.sender.public_key,
        "sender_addr": session.sender.addr,
        "receiver_fingerprint": receiver_fingerprint,
    }))
}

/// GET /api/v1/session/{code}/poll
pub async fn poll_session(
    State(store): State<SharedStore>,
    Path(code): Path<String>,
) -> ApiResponse {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(30);

    loop {
        {
            let store = store.read().await;
            let session = match store.get(&code) {
                Some(s) => s,
                None => return err(StatusCode::NOT_FOUND, "session not found"),
            };

            if let Some(ref receiver) = session.receiver {
                if session.status == SessionStatus::Connected {
                    return ok_json(json!({
                        "receiver_public_key": receiver.public_key,
                        "receiver_addr": receiver.addr,
                        "receiver_fingerprint": session.receiver_fingerprint,
                    }));
                }
            }
        }

        if tokio::time::Instant::now() >= deadline {
            return (StatusCode::NO_CONTENT, Json(json!(null)));
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}

/// POST /api/v1/session/{code}/approve
pub async fn approve_session(
    State(store): State<SharedStore>,
    Path(code): Path<String>,
    Json(_req): Json<ApproveRequest>,
) -> ApiResponse {
    let mut store = store.write().await;

    let session = match store.get_mut(&code) {
        Some(s) => s,
        None => return err(StatusCode::NOT_FOUND, "session not found"),
    };

    if session.receiver.is_none() {
        return err(StatusCode::BAD_REQUEST, "no receiver has joined yet");
    }

    session.status = SessionStatus::Connected;
    ok_json(json!({ "status": "approved" }))
}

/// DELETE /api/v1/session/{code}
pub async fn delete_session(
    State(store): State<SharedStore>,
    Path(code): Path<String>,
) -> ApiResponse {
    let mut store = store.write().await;

    if store.remove(&code).is_some() {
        (StatusCode::NO_CONTENT, Json(json!({})))
    } else {
        err(StatusCode::NOT_FOUND, "session not found")
    }
}

/// GET /health
pub async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

// ─── Helpers ───────────────────────────────────────────────

fn compute_fingerprint(public_key_b64: &str) -> String {
    let hash = blake3::hash(public_key_b64.as_bytes());
    hex::encode_upper(&hash.as_bytes()[..4])
}

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::codec;

// ─── Session Status ────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    /// Sender created the session, waiting for receiver to join.
    Pending,
    /// Receiver joined, waiting for sender to approve.
    AwaitingApproval,
    /// Sender approved, both peers can proceed to direct connection.
    Connected,
    /// Transfer complete (or session expired).
    Complete,
}

// ─── Peer Info ─────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerInfo {
    /// X25519 public key (base64-encoded).
    pub public_key: String,
    /// Observed IP:port from the server's perspective.
    pub addr: String,
}

// ─── Session ───────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Session {
    /// Internal UUID (not exposed to users).
    pub id: Uuid,
    /// Human-readable session code (base58, 8 chars).
    pub code: String,
    /// Session creation time.
    pub created_at: DateTime<Utc>,
    /// Session expiration time (defaults to 10 minutes after creation).
    pub expires_at: DateTime<Utc>,
    /// Sender's connection info (set at creation).
    pub sender: PeerInfo,
    /// Receiver's connection info (set when they join).
    pub receiver: Option<PeerInfo>,
    /// Current session status.
    pub status: SessionStatus,
    /// Receiver's public key fingerprint (for sender approval).
    pub receiver_fingerprint: Option<String>,
}

impl Session {
    pub fn new(code: String, sender: PeerInfo) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            code,
            created_at: now,
            expires_at: now + chrono::Duration::minutes(10),
            sender,
            receiver: None,
            status: SessionStatus::Pending,
            receiver_fingerprint: None,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn is_full(&self) -> bool {
        self.receiver.is_some()
    }
}

// ─── Session Store ─────────────────────────────────────────

/// In-memory session storage.
///
/// Sessions are stored in a `HashMap<String, Session>` keyed by
/// the 8-character session code.
///
/// A background task periodically removes expired sessions.
pub struct SessionStore {
    sessions: HashMap<String, Session>,
    /// Maximum join attempts per session code per minute.
    rate_limit_per_minute: usize,
    /// Track join attempts: (code, timestamp of last attempt)
    join_attempts: HashMap<String, Vec<DateTime<Utc>>>,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            rate_limit_per_minute: 10,
            join_attempts: HashMap::new(),
        }
    }
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a new session and return the generated code.
    pub fn insert(&mut self, sender: PeerInfo) -> String {
        let code = codec::generate_session_code();
        let session = Session::new(code.clone(), sender);
        self.sessions.insert(code.clone(), session);
        code
    }

    /// Get a session by code.
    pub fn get(&self, code: &str) -> Option<&Session> {
        self.sessions.get(code).filter(|s| !s.is_expired())
    }

    /// Get a mutable session by code.
    pub fn get_mut(&mut self, code: &str) -> Option<&mut Session> {
        if let Some(s) = self.sessions.get(code) {
            if s.is_expired() {
                self.sessions.remove(code);
                return None;
            }
        }
        self.sessions.get_mut(code)
    }

    /// Check if a session code exists and is not expired.
    pub fn exists(&self, code: &str) -> bool {
        self.sessions
            .get(code)
            .is_some_and(|s| !s.is_expired())
    }

    /// Check rate limits for join attempts.
    /// Returns true if the attempt is allowed.
    pub fn check_rate_limit(&mut self, code: &str) -> bool {
        let now = Utc::now();
        let attempts = self.join_attempts.entry(code.to_string()).or_default();

        // Remove attempts older than 1 minute.
        let cutoff = now - chrono::Duration::minutes(1);
        attempts.retain(|t| *t > cutoff);

        if attempts.len() >= self.rate_limit_per_minute {
            return false;
        }

        attempts.push(now);
        true
    }

    /// Remove a session by code.
    pub fn remove(&mut self, code: &str) -> Option<Session> {
        self.sessions.remove(code)
    }

    /// Remove all expired sessions.
    /// Returns the number of sessions removed.
    pub fn garbage_collect(&mut self) -> usize {
        let before = self.sessions.len();
        self.sessions.retain(|_, s| !s.is_expired());
        before - self.sessions.len()
    }

    /// Session count (non-expired).
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_sender() -> PeerInfo {
        PeerInfo {
            public_key: "AQsFqGi4o9q7qJ5QxLqH3Q==".into(),
            addr: "192.168.1.1:45000".into(),
        }
    }

    #[test]
    fn test_insert_and_get() {
        let mut store = SessionStore::new();
        let code = store.insert(test_sender());
        assert_eq!(code.len(), 8);
        assert!(store.exists(&code));
    }

    #[test]
    fn test_get_nonexistent() {
        let store = SessionStore::new();
        assert!(!store.exists("nonexistent"));
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn test_remove() {
        let mut store = SessionStore::new();
        let code = store.insert(test_sender());
        assert!(store.remove(&code).is_some());
        assert!(!store.exists(&code));
    }

    #[test]
    fn test_garbage_collect() {
        let mut store = SessionStore::new();
        let code = store.insert(test_sender());
        // Expire the session manually
        if let Some(s) = store.sessions.get_mut(&code) {
            s.expires_at = Utc::now() - chrono::Duration::minutes(1);
        }
        assert_eq!(store.garbage_collect(), 1);
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_is_full() {
        let mut store = SessionStore::new();
        let code = store.insert(test_sender());
        assert!(!store.get(&code).unwrap().is_full());

        let session = store.get_mut(&code).unwrap();
        session.receiver = Some(PeerInfo {
            public_key: "BQsFqGi4o9q7qJ5QxLqH3Q==".into(),
            addr: "10.0.0.1:48000".into(),
        });
        assert!(session.is_full());
    }

    #[test]
    fn test_rate_limit() {
        let mut store = SessionStore::new();
        let code = store.insert(test_sender());

        // 10 attempts should succeed.
        for _ in 0..10 {
            assert!(store.check_rate_limit(&code));
        }
        // 11th should fail.
        assert!(!store.check_rate_limit(&code));
    }

    #[test]
    fn test_session_expiry() {
        let mut store = SessionStore::new();
        let code = store.insert(test_sender());
        assert!(store.exists(&code));

        // Manually expire.
        if let Some(s) = store.sessions.get_mut(&code) {
            s.expires_at = Utc::now() - chrono::Duration::seconds(1);
        }
        assert!(!store.exists(&code));
    }
}

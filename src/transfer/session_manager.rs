use thiserror::Error;

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub code: String,
    pub session_id: String,
}

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub public_key: String,
    pub addr: String,
}

#[derive(Debug, Clone)]
pub struct PollResult {
    pub status: String,
    pub receiver: Option<PeerInfo>,
    pub receiver_fingerprint: Option<String>,
}

pub struct SessionManager {
    client: reqwest::Client,
    server_url: String,
}

impl SessionManager {
    pub fn new(server_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            server_url: server_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn create_session(
        &self,
        public_key: &str,
        addr: &str,
    ) -> Result<SessionInfo, SessionError> {
        let resp = self
            .client
            .post(format!("{}/api/v1/session", self.server_url))
            .json(&serde_json::json!({
                "public_key": public_key,
                "addr": addr,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(SessionError::Api(format!("create failed: {}", text)));
        }

        let body: serde_json::Value = resp.json().await?;
        Ok(SessionInfo {
            code: body["code"]
                .as_str()
                .ok_or(SessionError::MissingField("code"))?
                .to_string(),
            session_id: body["session_id"]
                .as_str()
                .ok_or(SessionError::MissingField("session_id"))?
                .to_string(),
        })
    }

    pub async fn join_session(
        &self,
        code: &str,
        public_key: &str,
        addr: &str,
    ) -> Result<PeerInfo, SessionError> {
        let resp = self
            .client
            .post(format!("{}/api/v1/session/{}/join", self.server_url, code))
            .json(&serde_json::json!({
                "public_key": public_key,
                "addr": addr,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(SessionError::Api(format!("join failed: {}", text)));
        }

        let body: serde_json::Value = resp.json().await?;
        Ok(PeerInfo {
            public_key: body["sender_public_key"]
                .as_str()
                .ok_or(SessionError::MissingField("sender_public_key"))?
                .to_string(),
            addr: body["sender_addr"]
                .as_str()
                .ok_or(SessionError::MissingField("sender_addr"))?
                .to_string(),
        })
    }

    pub async fn poll_session(&self, code: &str) -> Result<Option<PollResult>, SessionError> {
        let resp = self
            .client
            .get(format!("{}/api/v1/session/{}/poll", self.server_url, code))
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NO_CONTENT {
            return Ok(None);
        }

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(SessionError::Api(format!("poll failed: {}", text)));
        }

        let body: serde_json::Value = resp.json().await?;

        let status = body["status"]
            .as_str()
            .unwrap_or("connected")
            .to_string();

        let receiver = body.get("receiver_public_key").and_then(|k| {
            k.as_str().map(|pk| PeerInfo {
                public_key: pk.to_string(),
                addr: body["receiver_addr"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
            })
        });

        Ok(Some(PollResult {
            status,
            receiver,
            receiver_fingerprint: body["receiver_fingerprint"]
                .as_str()
                .map(|s| s.to_string()),
        }))
    }

    pub async fn approve_session(&self, code: &str, sender_token: &str) -> Result<(), SessionError> {
        let resp = self
            .client
            .post(format!(
                "{}/api/v1/session/{}/approve",
                self.server_url, code
            ))
            .json(&serde_json::json!({
                "sender_token": sender_token,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(SessionError::Api(format!("approve failed: {}", text)));
        }

        Ok(())
    }

    pub async fn delete_session(&self, code: &str) -> Result<(), SessionError> {
        let resp = self
            .client
            .delete(format!("{}/api/v1/session/{}", self.server_url, code))
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(SessionError::Api(format!("delete failed: {}", text)));
        }

        Ok(())
    }

    pub async fn wait_for_receiver(
        &self,
        code: &str,
        timeout_secs: u64,
    ) -> Result<PollResult, SessionError> {
        let start = std::time::Instant::now();
        loop {
            if start.elapsed().as_secs() > timeout_secs {
                return Err(SessionError::Timeout);
            }

            match self.poll_session(code).await? {
                Some(result) if result.receiver.is_some() => return Ok(result),
                _ => {}
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    pub async fn wait_for_approval(
        &self,
        code: &str,
        timeout_secs: u64,
    ) -> Result<(), SessionError> {
        let start = std::time::Instant::now();
        loop {
            if start.elapsed().as_secs() > timeout_secs {
                return Err(SessionError::Timeout);
            }

            if let Some(result) = self.poll_session(code).await? {
                if result.status == "connected" {
                    return Ok(());
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }
}

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error: {0}")]
    Api(String),

    #[error("missing field in response: {0}")]
    MissingField(&'static str),

    #[error("timeout waiting for peer")]
    Timeout,
}

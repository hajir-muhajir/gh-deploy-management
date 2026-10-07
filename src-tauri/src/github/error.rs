//! Error type returned to the frontend, and the mapping from HTTP responses.
//!
//! Messages are deliberately free of credential material.

use reqwest::{Response, StatusCode};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub kind: &'static str,
    pub message: String,
}

impl ApiError {
    pub(crate) fn new(kind: &'static str, message: impl Into<String>) -> Self {
        Self { kind, message: message.into() }
    }

    pub fn network(err: reqwest::Error) -> Self {
        // Deliberately not `err.to_string()` on the whole chain: a reqwest error
        // can embed the request URL, and our URLs are safe, but the header set is
        // not something we want stringified by accident.
        Self::new(
            "network",
            if err.is_timeout() { "Request to GitHub timed out" } else { "Cannot reach GitHub" },
        )
    }

    /// Raised when an API call is attempted before a token has been saved.
    pub fn not_connected() -> Self {
        Self::new("notConnected", "No GitHub token saved yet")
    }

    pub fn keyring(err: keyring::Error) -> Self {
        Self::new("keyring", format!("Credential store error: {err}"))
    }
}

/// GitHub puts the human-readable reason in a JSON `message` field.
pub(super) async fn error_from(response: Response) -> ApiError {
    let status = response.status();
    let remaining = header_str(&response, "x-ratelimit-remaining");
    let message = response
        .json::<serde_json::Value>()
        .await
        .ok()
        .and_then(|body| body.get("message").and_then(|m| m.as_str()).map(String::from))
        .unwrap_or_else(|| status.to_string());

    match status {
        StatusCode::UNAUTHORIZED => ApiError::new("invalidToken", message),
        StatusCode::NOT_FOUND => ApiError::new("notFound", message),
        StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS if remaining.as_deref() == Some("0") => {
            ApiError::new("rateLimited", message)
        }
        StatusCode::FORBIDDEN => ApiError::new("forbidden", message),
        // GitHub explains exactly what it disliked — "Required input 'tag' not
        // provided", "No ref found for: …" — so the message is the whole point.
        StatusCode::UNPROCESSABLE_ENTITY => ApiError::new("invalid", message),
        _ => ApiError::new("network", message),
    }
}

pub(super) fn header_str(response: &Response, name: &str) -> Option<String> {
    response.headers().get(name)?.to_str().ok().map(String::from)
}

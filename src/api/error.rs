use std::fmt;

#[derive(Debug)]
pub enum ApiError {
    Network(String),
    Http(u16, String),
    Parse(String),
    Timeout,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Network(msg) => write!(f, "Network error: {}", msg),
            ApiError::Http(status, msg) => write!(f, "HTTP error {}: {}", status, msg),
            ApiError::Parse(msg) => write!(f, "Parse error: {}", msg),
            ApiError::Timeout => write!(f, "Request timeout"),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            ApiError::Timeout
        } else if let Some(status) = err.status() {
            ApiError::Http(status.as_u16(), err.to_string())
        } else if err.is_connect() || err.is_request() {
            ApiError::Network(err.to_string())
        } else {
            ApiError::Network(err.to_string())
        }
    }
}

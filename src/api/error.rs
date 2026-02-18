use std::fmt;

#[derive(Debug)]
pub enum ApiError {
    Network(String),
    Http(u16, String),
    Parse(String),
    Timeout,
}

impl ApiError {
    /// Get a user-friendly error message with helpful suggestions
    pub fn user_message(&self) -> String {
        match self {
            ApiError::Network(msg) => {
                format!(
                    "Network error: {}\n\nSuggestions:\n  • Check your internet connection\n  • Verify CircleCI service status",
                    msg
                )
            }
            ApiError::Http(status, msg) => match status {
                401 | 403 => {
                    format!(
                        "Authentication failed (HTTP {})\n\nSuggestions:\n  • Check CIRCLECI_TOKEN in .env file\n  • Verify token has required permissions\n  • Generate a new token at https://app.circleci.com/settings/user/tokens",
                        status
                    )
                }
                404 => {
                    "Resource not found (HTTP 404)\n\nSuggestions:\n  • Verify PROJECT_SLUG is correct in .env file\n  • Check pipeline/workflow/job ID\n  • Ensure you have access to this project"
                        .to_string()
                }
                429 => {
                    "Rate limit exceeded (HTTP 429)\n\nSuggestions:\n  • Wait a few minutes before retrying\n  • Reduce polling frequency"
                        .to_string()
                }
                500..=599 => {
                    format!(
                        "CircleCI service error (HTTP {})\n\nSuggestions:\n  • Check CircleCI service status\n  • Try again in a few minutes",
                        status
                    )
                }
                _ => format!("HTTP error {}: {}", status, msg),
            },
            ApiError::Parse(msg) => {
                format!(
                    "Failed to parse response: {}\n\nSuggestions:\n  • API response format may have changed\n  • Check for CircleCI API updates",
                    msg
                )
            }
            ApiError::Timeout => {
                "Request timed out\n\nSuggestions:\n  • Check your internet connection\n  • Try again\n  • CircleCI service may be slow"
                    .to_string()
            }
        }
    }
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

//! Configuration loading for CircleCI TUI
//!
//! This module handles loading configuration from environment variables
//! using the .env file.

use anyhow::{Context, Result};
use std::env;

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// CircleCI API token
    pub circle_token: String,
    /// Project slug in format "gh/owner/repo" or "bb/owner/repo"
    pub project_slug: String,
}

impl Config {
    /// Load configuration from .env file
    ///
    /// This method loads the .env file and reads the following variables:
    /// - `CIRCLECI_TOKEN`: CircleCI API token (required)
    /// - `PROJECT_SLUG`: Project slug (required)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The .env file cannot be loaded
    /// - Required environment variables are missing
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use circleci_tui_rs::config::Config;
    ///
    /// let config = Config::load().expect("Failed to load config");
    /// println!("Token: {}", config.circle_token);
    /// println!("Project: {}", config.project_slug);
    /// ```
    pub fn load() -> Result<Self> {
        // Load .env file if it exists (don't error if it doesn't)
        let _ = dotenvy::dotenv();

        let circle_token = env::var("CIRCLECI_TOKEN")
            .context("CIRCLECI_TOKEN environment variable not set. Please set it in .env file or environment.")?;

        let project_slug = env::var("PROJECT_SLUG").context(
            "PROJECT_SLUG environment variable not set. Please set it in .env file or environment.",
        )?;

        // Validate that neither field is empty
        if circle_token.trim().is_empty() {
            anyhow::bail!("CIRCLECI_TOKEN cannot be empty");
        }

        if project_slug.trim().is_empty() {
            anyhow::bail!("PROJECT_SLUG cannot be empty");
        }

        Ok(Self {
            circle_token,
            project_slug,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = Config {
            circle_token: "test_token".to_string(),
            project_slug: "gh/owner/repo".to_string(),
        };

        assert_eq!(config.circle_token, "test_token");
        assert_eq!(config.project_slug, "gh/owner/repo");
    }

    #[test]
    fn test_config_clone() {
        let config = Config {
            circle_token: "test_token".to_string(),
            project_slug: "gh/owner/repo".to_string(),
        };

        let cloned = config.clone();
        assert_eq!(config.circle_token, cloned.circle_token);
        assert_eq!(config.project_slug, cloned.project_slug);
    }
}

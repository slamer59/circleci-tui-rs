//! Configuration loading for CircleCI TUI
//!
//! This module handles loading configuration from environment variables
//! using `.env.local` and `.env` files.

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
    /// Load configuration from environment and dotenv files
    ///
    /// This method loads `.env.local` before `.env`, so local values take
    /// precedence. Variables already present in the process environment take
    /// precedence over both files.
    ///
    /// It reads the following variables:
    /// - `CIRCLECI_TOKEN`: CircleCI API token (required)
    /// - `PROJECT_SLUG`: Project slug (required)
    ///
    /// # Errors
    ///
    /// Returns an error if:
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
        // dotenvy preserves existing variables, so loading the local file first
        // gives us: process environment > .env.local > .env.
        let _ = dotenvy::from_filename(".env.local");
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

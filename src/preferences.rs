//! User Preferences Management
//!
//! This module provides persistent user preferences using the `confy` crate with YAML format.
//! Preferences are stored in the standard configuration directory for the platform:
//! - Linux: `~/.config/circleci-tui/preferences.yml`
//! - macOS: `~/Library/Application Support/circleci-tui/preferences.yml`
//! - Windows: `%APPDATA%\circleci-tui\preferences.yml`

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Main preferences structure containing all user settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Version number for future migrations
    pub version: u32,

    /// Cached CircleCI user information
    pub user: Option<CachedUser>,

    /// Pipeline filter preferences
    pub pipeline_filters: PipelineFilterPrefs,

    /// First-run detection flag
    pub first_run: bool,
}

/// Cached user information from CircleCI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedUser {
    /// Username from CircleCI (e.g., "johndoe")
    pub login: String,

    /// Full name (e.g., "John Doe")
    pub name: Option<String>,

    /// Timestamp when this user info was cached
    pub cached_at: DateTime<Utc>,
}

/// Pipeline filter preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineFilterPrefs {
    /// Owner filter index: 0="All", 1="Mine"
    pub owner_index: usize,

    /// Branch filter: None="All", Some(branch_name) for specific branch
    pub branch: Option<String>,

    /// Date filter index (application-specific)
    pub date_index: usize,

    /// Status filter index (application-specific)
    pub status_index: usize,

    /// Search query text
    pub search_text: String,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            version: 1,
            user: None,
            pipeline_filters: PipelineFilterPrefs::default(),
            first_run: true,
        }
    }
}

impl Default for PipelineFilterPrefs {
    fn default() -> Self {
        Self {
            owner_index: 0,      // "All"
            branch: None,        // "All"
            date_index: 0,       // First option
            status_index: 0,     // First option
            search_text: String::new(),
        }
    }
}

impl CachedUser {
    /// Creates a new CachedUser with the current timestamp
    pub fn new(login: String, name: Option<String>) -> Self {
        Self {
            login,
            name,
            cached_at: Utc::now(),
        }
    }

    /// Checks if the cached user info is stale (older than 24 hours)
    pub fn is_stale(&self) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(self.cached_at);
        age > Duration::hours(24)
    }
}

/// Manager for loading, saving, and accessing user preferences
pub struct PreferencesManager {
    preferences: UserPreferences,
    app_name: &'static str,
    config_name: &'static str,
}

impl PreferencesManager {
    /// Application name for confy
    const APP_NAME: &'static str = "circleci-tui";

    /// Configuration file name
    const CONFIG_NAME: &'static str = "preferences";

    /// Loads preferences from disk or creates defaults if not found
    pub fn load() -> Result<Self> {
        let preferences = match confy::load(Self::APP_NAME, Self::CONFIG_NAME) {
            Ok(prefs) => prefs,
            Err(e) => {
                // Log warning about corrupted preferences
                eprintln!(
                    "Warning: Failed to load preferences ({}), using defaults",
                    e
                );

                // Return default preferences
                UserPreferences::default()
            }
        };

        Ok(Self {
            preferences,
            app_name: Self::APP_NAME,
            config_name: Self::CONFIG_NAME,
        })
    }

    /// Saves current preferences to disk
    pub fn save(&self) -> Result<()> {
        confy::store(self.app_name, self.config_name, &self.preferences)
            .context("Failed to save preferences to disk")?;
        Ok(())
    }

    /// Gets an immutable reference to the preferences
    pub fn get_preferences(&self) -> &UserPreferences {
        &self.preferences
    }

    /// Gets a mutable reference to the preferences
    pub fn get_preferences_mut(&mut self) -> &mut UserPreferences {
        &mut self.preferences
    }

    /// Checks if the cached user information is stale (older than 24 hours)
    /// Returns true if there's no cached user or if the cache is stale
    pub fn is_user_cache_stale(&self) -> bool {
        match &self.preferences.user {
            None => true,
            Some(user) => user.is_stale(),
        }
    }

    /// Updates the cached user information with current timestamp
    pub fn update_user_cache(&mut self, login: String, name: Option<String>) {
        self.preferences.user = Some(CachedUser::new(login, name));
    }

    /// Clears the first-run flag
    pub fn clear_first_run(&mut self) {
        self.preferences.first_run = false;
    }

    /// Gets the configuration file path (for display/debugging purposes)
    pub fn get_config_path(&self) -> Result<std::path::PathBuf> {
        let config_dir = confy::get_configuration_file_path(self.app_name, self.config_name)
            .context("Failed to get configuration file path")?;
        Ok(config_dir)
    }

    /// Resets all preferences to defaults (useful for testing or user-requested reset)
    pub fn reset_to_defaults(&mut self) -> Result<()> {
        self.preferences = UserPreferences::default();
        self.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_preferences() {
        let prefs = UserPreferences::default();
        assert_eq!(prefs.version, 1);
        assert!(prefs.user.is_none());
        assert!(prefs.first_run);
        assert_eq!(prefs.pipeline_filters.owner_index, 0);
        assert_eq!(prefs.pipeline_filters.branch, None);
        assert_eq!(prefs.pipeline_filters.search_text, "");
    }

    #[test]
    fn test_cached_user_creation() {
        let user = CachedUser::new("testuser".to_string(), Some("Test User".to_string()));
        assert_eq!(user.login, "testuser");
        assert_eq!(user.name, Some("Test User".to_string()));
        assert!(!user.is_stale()); // Just created, should not be stale
    }

    #[test]
    fn test_cached_user_staleness() {
        let mut user = CachedUser::new("testuser".to_string(), None);

        // Just created - not stale
        assert!(!user.is_stale());

        // Simulate 25 hours ago
        user.cached_at = Utc::now() - Duration::hours(25);
        assert!(user.is_stale());

        // Simulate 23 hours ago - not stale yet
        user.cached_at = Utc::now() - Duration::hours(23);
        assert!(!user.is_stale());
    }

    #[test]
    fn test_preferences_manager_user_cache() {
        let mut manager = PreferencesManager {
            preferences: UserPreferences::default(),
            app_name: PreferencesManager::APP_NAME,
            config_name: PreferencesManager::CONFIG_NAME,
        };

        // No user cached - should be stale
        assert!(manager.is_user_cache_stale());

        // Update cache
        manager.update_user_cache("testuser".to_string(), Some("Test User".to_string()));
        assert!(!manager.is_user_cache_stale());

        // Check cached values
        let prefs = manager.get_preferences();
        assert!(prefs.user.is_some());
        assert_eq!(prefs.user.as_ref().unwrap().login, "testuser");
    }

    #[test]
    fn test_first_run_flag() {
        let mut manager = PreferencesManager {
            preferences: UserPreferences::default(),
            app_name: PreferencesManager::APP_NAME,
            config_name: PreferencesManager::CONFIG_NAME,
        };

        assert!(manager.get_preferences().first_run);
        manager.clear_first_run();
        assert!(!manager.get_preferences().first_run);
    }
}

//! Example demonstrating the preferences system
//!
//! This example shows how to:
//! - Load preferences (creates defaults if not found)
//! - Update user cache
//! - Save preferences
//! - Check cache staleness

use anyhow::Result;
use circleci_tui_rs::preferences::PreferencesManager;

fn main() -> Result<()> {
    println!("=== CircleCI TUI Preferences Demo ===\n");

    // Load preferences
    println!("Loading preferences...");
    let mut manager = PreferencesManager::load()?;

    // Show config file location
    if let Ok(path) = manager.get_config_path() {
        println!("Config file: {}\n", path.display());
    }

    // Show current preferences
    let prefs = manager.get_preferences();
    println!("Current preferences:");
    println!("  Version: {}", prefs.version);
    println!("  First run: {}", prefs.first_run);
    println!("  User cached: {}", prefs.user.is_some());

    if let Some(user) = &prefs.user {
        println!("    Login: {}", user.login);
        println!("    Name: {:?}", user.name);
        println!("    Cached at: {}", user.cached_at);
        println!("    Is stale: {}", user.is_stale());
    }

    println!("\nPipeline filters:");
    println!("  Owner index: {}", prefs.pipeline_filters.owner_index);
    println!("  Branch: {:?}", prefs.pipeline_filters.branch);
    println!("  Date index: {}", prefs.pipeline_filters.date_index);
    println!("  Status index: {}", prefs.pipeline_filters.status_index);
    println!("  Search text: '{}'", prefs.pipeline_filters.search_text);

    // Check if user cache is stale
    println!("\nUser cache stale: {}", manager.is_user_cache_stale());

    // Example: Update user cache
    println!("\nUpdating user cache...");
    manager.update_user_cache("demo-user".to_string(), Some("Demo User".to_string()));

    // Example: Update filter preferences
    println!("Updating filter preferences...");
    {
        let prefs_mut = manager.get_preferences_mut();
        prefs_mut.pipeline_filters.owner_index = 1; // "Mine"
        prefs_mut.pipeline_filters.branch = Some("main".to_string());
        prefs_mut.pipeline_filters.search_text = "test".to_string();
    }

    // Clear first run flag
    manager.clear_first_run();

    // Save preferences
    println!("Saving preferences...");
    manager.save()?;

    // Reload to verify
    println!("\nReloading preferences to verify...");
    let manager_reloaded = PreferencesManager::load()?;
    let prefs_reloaded = manager_reloaded.get_preferences();

    println!("Verified:");
    println!("  First run: {}", prefs_reloaded.first_run);
    println!("  User: {}", prefs_reloaded.user.as_ref().unwrap().login);
    println!(
        "  Owner index: {}",
        prefs_reloaded.pipeline_filters.owner_index
    );
    println!(
        "  Branch: {:?}",
        prefs_reloaded.pipeline_filters.branch
    );
    println!(
        "  Search text: '{}'",
        prefs_reloaded.pipeline_filters.search_text
    );

    println!("\n✓ Preferences system working correctly!");

    Ok(())
}

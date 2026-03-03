# Preferences System Integration Guide

## Overview

The preferences system (`src/preferences.rs`) provides persistent user settings using the `confy` crate with YAML format. Preferences are automatically saved to the platform's standard configuration directory.

## Storage Location

Preferences are stored in:
- **Linux**: `~/.config/circleci-tui/preferences.yml`
- **macOS**: `~/Library/Application Support/circleci-tui/preferences.yml`
- **Windows**: `%APPDATA%\circleci-tui\preferences.yml`

## Data Structures

### UserPreferences

The main preferences structure containing all user settings:

```rust
pub struct UserPreferences {
    pub version: u32,                          // For future migrations
    pub user: Option<CachedUser>,              // Cached CircleCI user
    pub pipeline_filters: PipelineFilterPrefs, // Filter state
    pub first_run: bool,                       // First-run detection
}
```

### CachedUser

Cached CircleCI user information:

```rust
pub struct CachedUser {
    pub login: String,                               // Username from CircleCI
    pub name: Option<String>,                        // Full name
    pub cached_at: chrono::DateTime<chrono::Utc>,   // Cache timestamp
}
```

### PipelineFilterPrefs

Pipeline filter preferences:

```rust
pub struct PipelineFilterPrefs {
    pub owner_index: usize,      // 0="All", 1="Mine"
    pub branch: Option<String>,  // Saved branch name or None="All"
    pub date_index: usize,       // Date filter index
    pub status_index: usize,     // Status filter index
    pub search_text: String,     // Search query
}
```

## PreferencesManager API

### Loading Preferences

```rust
use circleci_tui_rs::preferences::PreferencesManager;

// Load preferences (creates defaults if not found)
let manager = PreferencesManager::load()?;

// Access preferences (immutable)
let prefs = manager.get_preferences();
println!("First run: {}", prefs.first_run);
```

### Updating Preferences

```rust
// Get mutable access
let prefs_mut = manager.get_preferences_mut();

// Update filter preferences
prefs_mut.pipeline_filters.owner_index = 1; // "Mine"
prefs_mut.pipeline_filters.branch = Some("main".to_string());
prefs_mut.pipeline_filters.search_text = "test".to_string();
```

### User Cache Management

```rust
// Check if user cache is stale (>24 hours old)
if manager.is_user_cache_stale() {
    // Fetch user from API...
    manager.update_user_cache("username".to_string(), Some("Full Name".to_string()));
}

// Access cached user
if let Some(user) = &manager.get_preferences().user {
    println!("User: {} ({})", user.name.as_deref().unwrap_or(""), user.login);
}
```

### Saving Preferences

```rust
// Save preferences to disk
manager.save()?;
```

### Other Operations

```rust
// Clear first-run flag
manager.clear_first_run();

// Get config file path (for display/debugging)
let path = manager.get_config_path()?;
println!("Config: {}", path.display());

// Reset to defaults
manager.reset_to_defaults()?;
```

## Integration into App

### 1. Add PreferencesManager to App State

Update `src/app/mod.rs`:

```rust
use crate::preferences::PreferencesManager;

pub struct App {
    // ... existing fields ...
    preferences: PreferencesManager,
}

impl App {
    pub fn new(config: Config) -> Result<Self> {
        // Load preferences
        let mut preferences = PreferencesManager::load()?;

        // Check if first run
        if preferences.get_preferences().first_run {
            // Show welcome screen or tutorial
            preferences.clear_first_run();
        }

        Ok(Self {
            // ... existing fields ...
            preferences,
        })
    }
}
```

### 2. Restore Filter State on Startup

```rust
impl App {
    pub fn new(config: Config) -> Result<Self> {
        let mut preferences = PreferencesManager::load()?;

        // Restore filter state
        let filter_prefs = &preferences.get_preferences().pipeline_filters;

        let mut app = Self {
            // ... initialize fields ...
            preferences,
        };

        // Apply saved filters to pipelines screen
        app.pipelines_screen.set_owner_filter_index(filter_prefs.owner_index);
        if let Some(branch) = &filter_prefs.branch {
            app.pipelines_screen.set_branch_filter(Some(branch.clone()));
        }
        app.pipelines_screen.set_search_text(&filter_prefs.search_text);

        Ok(app)
    }
}
```

### 3. Save Filter State on Changes

```rust
impl App {
    pub fn handle_filter_change(&mut self) -> Result<()> {
        // Get current filter state from pipelines screen
        let owner_idx = self.pipelines_screen.get_owner_filter_index();
        let branch = self.pipelines_screen.get_branch_filter();
        let search = self.pipelines_screen.get_search_text();

        // Update preferences
        let prefs = self.preferences.get_preferences_mut();
        prefs.pipeline_filters.owner_index = owner_idx;
        prefs.pipeline_filters.branch = branch;
        prefs.pipeline_filters.search_text = search.to_string();

        // Save to disk
        self.preferences.save()?;

        Ok(())
    }
}
```

### 4. User Cache Management

```rust
impl App {
    pub async fn load_user_if_needed(&mut self) -> Result<()> {
        // Check if cache is stale
        if self.preferences.is_user_cache_stale() {
            // Fetch user from API
            let user = self.client.get_current_user().await?;

            // Update cache
            self.preferences.update_user_cache(
                user.login.clone(),
                user.name.clone()
            );

            // Save preferences
            self.preferences.save()?;
        }

        Ok(())
    }

    pub fn get_current_username(&self) -> Option<&str> {
        self.preferences
            .get_preferences()
            .user
            .as_ref()
            .map(|u| u.login.as_str())
    }
}
```

### 5. Save Preferences on Exit

Update `src/main.rs`:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // ... setup code ...

    // Run app
    let result = run_app(&mut app, &mut terminal).await;

    // Save preferences before exit
    if let Err(e) = app.save_preferences() {
        eprintln!("Warning: Failed to save preferences: {}", e);
    }

    // ... cleanup code ...

    result
}
```

Add to `src/app/mod.rs`:

```rust
impl App {
    pub fn save_preferences(&mut self) -> Result<()> {
        self.preferences.save()
    }
}
```

## Error Handling

The preferences system handles errors gracefully:

- **Corrupted config file**: Automatically resets to defaults and logs a warning
- **Missing config file**: Creates new file with default values
- **Permission errors**: Returns error via `anyhow::Result`

## Testing

Run the example to test the system:

```bash
cargo run --example preferences_demo
```

Run unit tests:

```bash
cargo test preferences::
```

## Migration Support

The `version` field supports future migrations. Example:

```rust
impl PreferencesManager {
    pub fn load() -> Result<Self> {
        let mut prefs: UserPreferences = confy::load(Self::APP_NAME, Self::CONFIG_NAME)?;

        // Migrate old versions
        if prefs.version < 2 {
            // Perform v1 -> v2 migration
            prefs.version = 2;
        }

        Ok(Self { preferences: prefs, ... })
    }
}
```

## Best Practices

1. **Save on Changes**: Save preferences immediately after user changes (filters, settings)
2. **Load on Startup**: Load preferences early in app initialization
3. **Save on Exit**: Always save preferences before application exit
4. **Handle Errors**: Log preference errors but don't crash the app
5. **Cache Staleness**: Refresh user cache periodically (24-hour default)
6. **First Run**: Use the `first_run` flag to show tutorials or setup wizards

## YAML Format Example

```yaml
version: 1
user:
  login: demo-user
  name: Demo User
  cached_at: 2026-03-03T11:01:48.280965508Z
pipeline_filters:
  owner_index: 1
  branch: main
  date_index: 0
  status_index: 0
  search_text: test
first_run: false
```

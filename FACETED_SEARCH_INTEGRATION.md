# Faceted Search Integration - Pipeline Screen

## Overview

This document describes the integration of the generic `FacetedSearchBar` widget into the pipeline screen for filtering pipelines.

## Files Modified

### 1. `/home/pedot/projects/circleci-tui-rs/src/ui/widgets/faceted_search.rs`

**Added Features:**
- **`update_facet_options()` method**: Allows dynamic updating of facet options (e.g., when new pipelines are loaded with different branches)
- **Preserves current selection**: When options are updated, the current selection is preserved if it exists in the new options
- **Test coverage**: Added comprehensive tests for the new functionality

**Key Methods:**
```rust
pub fn update_facet_options(&mut self, facet_idx: usize, new_options: Vec<String>) -> bool
```

### 2. `/home/pedot/projects/circleci-tui-rs/src/ui/screens/pipelines.rs`

**Filter Configuration (Matching Python Implementation):**

1. **Owner Filter (Facet 0)**
   - Icon: ☍
   - Options: "All pipelines", "Mine"
   - Default: "All pipelines" (index 0)
   - Filters pipelines by commit author

2. **Branch Filter (Facet 1)**
   - Icon: ⎇
   - Options: "All" + unique branches from pipeline data
   - Default: "All" (index 0)
   - Dynamically updated when pipelines are loaded
   - Exact branch name matching

3. **Date Filter (Facet 2)**
   - Icon: 📅
   - Options: "Last 24 hours", "Last 7 days", "Last 30 days", "Last 90 days", "All time"
   - Default: "All time" (index 4)
   - Filters based on pipeline `created_at` timestamp
   - Uses chrono::Duration for date calculations

4. **Status Filter (Facet 3)**
   - Icon: ●
   - Options: "All", "success", "failed", "running", "pending"
   - Default: "All" (index 0)
   - Filters by pipeline state

**Key Changes:**

1. **Dynamic Branch Updates:**
```rust
pub fn set_pipelines(&mut self, pipelines: Vec<Pipeline>) {
    self.pipelines = pipelines;
    self.update_branch_filter(); // Update branch options
    self.apply_filters();
}

fn update_branch_filter(&mut self) {
    let branches = self.get_unique_branches();
    let mut branch_options = vec!["All".to_string()];
    branch_options.extend(branches);
    self.faceted_search.update_facet_options(1, branch_options);
}
```

2. **Enhanced Filter Logic:**
```rust
pub fn apply_filters(&mut self) {
    // Get filter values
    let owner_filter = self.faceted_search.get_filter_value(0).unwrap_or("All pipelines");
    let branch_filter = self.faceted_search.get_filter_value(1).unwrap_or("All");
    let date_filter = self.faceted_search.get_filter_value(2).unwrap_or("All time");
    let status_filter = self.faceted_search.get_filter_value(3).unwrap_or("All");

    // Apply all filters with proper date calculations
    // ...
}
```

3. **UI Enhancements:**
   - Header shows active filter count: "CircleCI Pipelines - project (2 filters active)"
   - Footer displays context-aware keyboard shortcuts
   - Filter bar renders using `faceted_search.render(f, area)`

## Keyboard Navigation

### When Filter Bar is NOT Active:
- **↑/↓**: Navigate pipeline list
- **⏎**: Open selected pipeline
- **/**: Activate filter mode
- **r**: Refresh pipeline list
- **Esc**: Clear all filters
- **?**: Show help
- **q**: Quit

### When Filter Bar is Active:
- **←/→ or Tab**: Switch between filter buttons
- **↑/↓**: Navigate dropdown options (when open)
- **⏎ or Space**: Open dropdown / Confirm selection
- **Esc**: Close dropdown or exit filter mode

## Filter Behavior

### Owner Filter
- **"All pipelines"**: Shows all pipelines regardless of author
- **"Mine"**: Shows only pipelines by the current user (mock: Alice or Bob)

### Branch Filter
- **"All"**: Shows pipelines from all branches
- **Specific branch**: Shows only pipelines from that exact branch
- **Dynamic**: Options update automatically when new pipelines are loaded

### Date Filter
- **"All time"**: No date filtering
- **"Last 24 hours"**: Pipelines created in the last 24 hours
- **"Last 7 days"**: Pipelines created in the last 7 days
- **"Last 30 days"**: Pipelines created in the last 30 days
- **"Last 90 days"**: Pipelines created in the last 90 days

### Status Filter
- **"All"**: Shows pipelines with any status
- **Specific status**: Shows only pipelines with that state (success, failed, running, pending)

## Visual States

The filter buttons have four visual states:

1. **Inactive**: Dim text (FG_DIM)
2. **Focused**: Bright text (FG_BRIGHT) + Bold
3. **Pressed**: Dark background + Bright text (dropdown open)
4. **Filtered**: Accent color (non-default selection)

## Implementation Notes

1. **State Management:**
   - `filter_active` flag controls input routing
   - `faceted_search` field holds the widget state
   - `filtered_pipelines` contains the result after applying filters

2. **Performance:**
   - Filters are applied on every change
   - Branch options are updated only when pipelines change
   - Current selection is preserved when updating options

3. **Mock Data:**
   - "Mine" filter uses hardcoded user names (Alice, Bob) for mock data
   - Date filtering uses actual chrono calculations
   - Branch filtering supports real branch names from pipeline data

## Testing

All filter functionality is covered by tests in:
- `src/ui/widgets/faceted_search.rs` - Widget-level tests
- `src/ui/screens/pipelines.rs` - Screen-level tests

Run tests with:
```bash
cargo test
```

## Future Enhancements

1. **Real User Authentication**: Replace mock "Mine" filter with actual CircleCI user API
2. **Filter Persistence**: Save filter state between sessions
3. **Search Text Input**: Add free-text search alongside faceted filters
4. **Filter Presets**: Allow saving and loading filter combinations
5. **Export Filters**: Generate CircleCI API URLs based on current filters

## Comparison with Python Implementation

The Rust implementation now closely matches the Python version:

| Feature | Python | Rust | Status |
|---------|--------|------|--------|
| Owner Filter | ✓ | ✓ | Complete |
| Branch Filter | ✓ | ✓ | Complete |
| Date Filter | ✓ | ✓ | Complete |
| Status Filter | ✗ (jobs only) | ✓ | Enhanced |
| Dynamic Branches | ✓ | ✓ | Complete |
| Filter Count Display | ✓ | ✓ | Complete |
| Keyboard Navigation | ✓ | ✓ | Complete |

## Related Files

- `/home/pedot/projects/circleci-tui-rs/src/ui/widgets/faceted_search.rs` - Generic widget
- `/home/pedot/projects/circleci-tui-rs/src/ui/screens/pipelines.rs` - Pipeline screen
- `/home/pedot/projects/circleci-tui/src/cci/screens/pipelines.py` - Python reference
- `/home/pedot/projects/circleci-tui/src/cci/widgets/filter_panel.py` - Python filter panel

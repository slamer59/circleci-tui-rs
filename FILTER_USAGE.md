# How to Use Filters in CircleCI TUI (Current Implementation)

## Pipelines Screen Filters

### Accessing Filters

**Option 1: Via Text Search → Tab**
1. Press `/` to activate text search input
2. Type to search pipelines (optional)
3. Press `Tab` to move to filter buttons
4. Now you can use the dropdowns

**Option 2: Direct (if implemented)**
- Press a dedicated filter key (e.g., `f`) to jump directly to filters

### Using Filter Dropdowns

Once on the filter buttons (after pressing `/` then `Tab`):

1. **Navigate between filters:**
   - `←/→` or `Tab` - Switch between ☍ ⎇ 📅 ●

2. **Open dropdown:**
   - `Space` or `Enter` - Opens the options menu

3. **Navigate options:**
   - `↑/↓` - Move through dropdown options
   - `Enter` - Select and close
   - `Esc` - Close without changing

4. **Exit filter mode:**
   - `Esc` - Returns to normal navigation

### Current Filter Options

- **☍ Owner**: "All pipelines" / "Mine"
- **⎇ Branch**: "All" + dynamic list of branches
- **📅 Date**: "Last 24 hours" / "Last 7 days" / "Last 30 days" / "All time"
- **● Status**: "All" / "success" / "failed" / "running" / "pending"

## Quick Test Steps

1. Launch app: `cargo run`
2. Press `/` (activates search)
3. Press `Tab` (moves to filter buttons - should see first button highlighted)
4. Press `Space` (dropdown should appear below button)
5. Press `↓` (navigate dropdown)
6. Press `Enter` (select option)

If dropdown doesn't appear after step 4, there may be a rendering issue.

## Troubleshooting

### Dropdown not appearing?
- Make sure you pressed Tab after `/` to reach filter buttons
- Check terminal has enough height (need ~10 lines for dropdown)
- Try pressing Space/Enter again

### Can't see which button is focused?
- Focused button should be underlined
- Try pressing Tab to cycle and watch for visual changes

### Filters not applying?
- Filters apply immediately when you select an option
- Check if pipeline list changes after selection
- Press Esc to clear all filters and try again

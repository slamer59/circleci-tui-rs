# How to Test Filters in CircleCI TUI

## Build and Run

```bash
cd /home/pedot/projects/circleci-tui-rs
source ~/.cargo/env
cargo build --release
cargo run --release
```

## Testing Pipeline Filters

### 1. Navigate to Pipelines Screen
- Launch the app
- You should see the pipeline list

### 2. Activate Filters
- Press `/` key to activate filter mode
- You'll see 4 filter buttons at the top:
  - ☍ (Owner): "All pipelines" / "Mine"
  - ⎇ (Branch): "All" + list of branches
  - 📅 (Date): "Last 24 hours" / "Last 7 days" / etc.
  - ● (Status): "All" / "success" / "failed" / "running" / "pending"

### 3. Navigate Filters
- Use `←/→` or `Tab` to switch between filters
- Use `↑/↓` to navigate dropdown options
- Press `Space` or `Enter` to open dropdown/confirm selection
- Press `Esc` to close dropdown or exit filter mode

### 4. Test Each Filter
- **Owner Filter**: Select "Mine" - should show only your pipelines
- **Branch Filter**: Select a specific branch - should filter by branch
- **Date Filter**: Select "Last 24 hours" - should show only recent pipelines
- **Status Filter**: Select "success" - should show only successful pipelines

### 5. Verify Filter Count
- Header should show "(X filters active)" when filters are applied
- Press `Esc` to clear all filters

## Testing Job Filters

### 1. Open Pipeline Detail
- Navigate to a pipeline and press `Enter`
- You should see workflows on the left, jobs on the right

### 2. Activate Job Filters
- Press `f` key to toggle filter mode
- You'll see checkboxes and a duration dropdown

### 3. Test Status Checkboxes
- Use `←/→` or `Tab` to navigate checkboxes
- Press `Space` to toggle each checkbox
- Test combinations:
  - Uncheck "Success" - successful jobs should disappear
  - Check only "Failed" - should show only failed jobs
  - Check only "Running" - should show only running jobs

### 4. Test Duration Filter
- Navigate to the "⏱ All durations" button
- Press `Space` or `Enter` to open dropdown
- Select different duration ranges:
  - "Quick (< 1min)" - jobs under 60 seconds
  - "Short (1-5min)" - jobs 1-5 minutes
  - "Long (15-30min)" - jobs 15-30 minutes
- Press `Enter` to apply

### 5. Verify Combined Filters
- Try combining status and duration filters
- Example: "Failed" + "Long" = only long-running failed jobs

### 6. Exit Filter Mode
- Press `f` or `Esc` to exit filter mode
- Filters remain active but can no longer be edited

## Expected Behavior

### Visual Feedback
- **Inactive filter**: Dim gray text
- **Focused filter**: Bright + bold + underline
- **Active/filtered filter**: Accent color (magenta) + bold
- **Dropdown open**: List appears below button

### Filter Persistence
- Filters remain active when navigating away and back
- Filter state is preserved during the session

### Performance
- Filtering should be instant (< 50ms)
- UI should remain responsive during filtering

## Troubleshooting

### Filters Not Working?
1. Check that cargo build completed without errors
2. Verify you're in filter mode (press `/` or `f`)
3. Check that you have data to filter (pipelines/jobs loaded)

### Dropdown Not Opening?
1. Make sure filter is focused (use Tab to navigate)
2. Press Space or Enter to open
3. Check that dropdown isn't already open

### No Visual Feedback?
1. Verify terminal supports colors
2. Check terminal size (minimum 80x24)
3. Try resizing terminal window

## Debug Mode

To see debug output:
```bash
cargo run 2>&1 | tee debug.log
```

This will show:
- Filter state changes
- Filter application timing
- Active filter counts

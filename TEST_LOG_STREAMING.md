# Log Streaming Test Guide

## Changes Made

### 1. Fixed Spinner Animation (src/ui/widgets/log_modal.rs)
- **Issue**: Spinner only animated when `is_loading` was true, but `is_loading` was set to false after first log load
- **Fix**: Changed `render()` method to always call `advance_spinner()` on each render, regardless of loading state
- **Result**: Spinner now animates smoothly during both initial load AND streaming updates

```rust
// Before:
if self.is_loading {
    self.advance_spinner();
}

// After:
// Always advance spinner animation on each render for smooth animation
self.advance_spinner();
```

### 2. Improved Event Loop Timing (src/main.rs)
- **Issue**: 100ms poll interval was too slow for smooth spinner animation
- **Fix**: Reduced poll interval from 100ms to 50ms
- **Result**: Spinner updates ~20 times per second for smoother animation

```rust
// Before:
if event::poll(Duration::from_millis(100))? {

// After:
if event::poll(Duration::from_millis(50))? {
```

### 3. Enhanced Log Streaming Debug Output (src/main.rs)
- **Added**: Debug messages for log refresh events
- **Purpose**: Help verify that streaming is working correctly
- **Output**: Logs show when streaming jobs trigger refresh, success/failure

```rust
eprintln!("[DEBUG] Refreshing logs for streaming job #{}", job_number);
// ... after load ...
eprintln!("[DEBUG] Successfully refreshed logs for job #{}", job_number);
```

### 4. Improved Auto-Scroll Logic (src/ui/widgets/log_modal.rs)
- **Issue**: Auto-scroll logic didn't account for streaming updates
- **Fix**: Enhanced `set_logs()` to track previous line count and conditionally auto-scroll
- **Result**: Auto-scrolls to bottom on initial load OR if streaming and user hasn't manually scrolled up

```rust
// Auto-scroll to bottom if:
// 1. This is the initial load (prev_lines was 1)
// 2. Auto-scroll is enabled (job is streaming and user hasn't scrolled up)
if prev_lines <= 1 || self.auto_scroll {
    self.scroll_to_bottom();
    eprintln!("[DEBUG] Auto-scrolled to bottom");
}
```

### 5. Added Job Status Update Support (src/ui/widgets/log_modal.rs)
- **Added**: `update_job()` method to update job info when refreshing logs
- **Purpose**: Detect when a running job completes and stop streaming
- **Result**: Can detect job state transitions (running → success/failed)

```rust
pub fn update_job(&mut self, job: Job) {
    let was_streaming = self.is_streaming;
    self.job = job.clone();
    self.is_streaming = job.is_running();

    // If job was streaming but is no longer running, stop auto-scroll
    if was_streaming && !self.is_streaming {
        eprintln!("[DEBUG] Job #{} transitioned from running to {}",
                 job.job_number, job.status);
        self.auto_scroll = false;
    }
}
```

## How Log Streaming Works

1. **Initial Load**: When you open a job log modal, the app loads logs immediately
2. **Streaming Detection**: If the job status is "running", `is_streaming` is set to true
3. **Auto-Refresh**: Every 2 seconds, the event loop checks `should_refresh()` for streaming jobs
4. **Visual Indicators**:
   - Animated spinner (always running when modal is visible)
   - "● Live" badge in header for streaming jobs
   - "Updated: Xs ago" timestamp showing time since last refresh
5. **Auto-Scroll**: Logs automatically scroll to bottom for streaming jobs (unless user manually scrolls up)

## Testing Instructions

### Test 1: Spinner Animation
1. Build and run the app: `cargo run`
2. Navigate to any pipeline and open a job log modal
3. **Verify**: The spinner should animate smoothly (⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏)
4. **Expected**: ~20 updates per second, smooth animation

### Test 2: Completed Job
1. Open logs for a completed job (status: success/failed)
2. **Verify**:
   - Spinner shows briefly during initial load
   - No "● Live" badge in header
   - No "Updated: Xs ago" timestamp
   - Logs don't refresh automatically

### Test 3: Running Job (requires real running job)
1. Trigger a long-running job in CircleCI (e.g., sleep 60)
2. Open the TUI and navigate to the running job
3. Open the job log modal
4. **Verify**:
   - "● Live" badge appears in header (cyan/accent color)
   - "Updated: Xs ago" timestamp appears and increments
   - Logs refresh every 2 seconds
   - New log lines appear automatically
   - Spinner continues to animate
   - Logs auto-scroll to bottom when new lines arrive

### Test 4: Manual Scroll During Streaming
1. Open logs for a running job
2. Scroll up using arrow keys or Page Up
3. **Verify**:
   - Auto-scroll is disabled (logs don't jump to bottom)
   - Logs still refresh in background
   - "Updated: Xs ago" timestamp still updates
4. Scroll to bottom using End key
5. **Verify**: Auto-scroll re-enables

### Test 5: Job Completion During Streaming
1. Open logs for a running job
2. Wait for the job to complete
3. **Expected** (future enhancement):
   - "● Live" badge disappears
   - Auto-scroll stops
   - Status updates to final state
   - *(Note: Currently requires manual refresh)*

## Debug Output

When running the app, you'll see debug output in the terminal:

```
[DEBUG] Opening log modal for job #123 (test-job)
[DEBUG] Set pending_log_load = Some(123)
[DEBUG] Triggering log load for job #123
[DEBUG] Setting logs: 45 lines
[DEBUG] is_loading set to false, prev_lines=1, new_lines=45
[DEBUG] Auto-scrolled to bottom
[DEBUG] Successfully loaded logs for job #123

# For streaming jobs, every 2 seconds:
[DEBUG] Refreshing logs for streaming job #123
[DEBUG] Setting logs: 67 lines
[DEBUG] is_loading set to false, prev_lines=45, new_lines=67
[DEBUG] Auto-scrolled to bottom
[DEBUG] Successfully refreshed logs for job #123
```

## Known Limitations

1. **Job Status Updates**: The app doesn't currently fetch updated job metadata when refreshing logs, so it won't detect when a job completes until you close and reopen the modal
2. **Network Errors**: If a log refresh fails, it's logged to stderr but doesn't show an error modal
3. **API Rate Limiting**: Refreshing every 2 seconds could hit rate limits on very active jobs (consider increasing to 3-5 seconds if needed)

## Future Enhancements

1. Fetch updated job metadata on each log refresh to detect completion
2. Add configurable refresh interval (2-10 seconds)
3. Show brief notification when new logs arrive
4. Add a manual refresh button (F5 or Ctrl+R)
5. Show error modal if log refresh fails
6. Add bandwidth optimization (only fetch new lines, not entire log)

## Performance Notes

- **Event Loop**: 50ms poll = 20Hz refresh rate for UI
- **Log Refresh**: 2 second interval for streaming jobs
- **Spinner**: 10 frames × 20Hz = 200ms per complete rotation
- **Memory**: All logs stored in memory (could be issue for very large logs)

# Load More Jobs Pagination Implementation

## Summary

Implemented "Load More" pagination functionality for jobs in the workflow screen, allowing users to load additional pages of jobs from the CircleCI API.

## Changes Made

### 1. App State (`src/app.rs`)

#### Added new field to App struct:
```rust
/// Pending load more jobs (workflow_id)
pub pending_load_more_jobs: Option<String>,
```

#### Updated App::new() to initialize the field:
```rust
pending_load_more_jobs: None,
```

#### Updated LoadMoreJobs action handler (lines 250-262):
```rust
PipelineDetailAction::LoadMoreJobs => {
    // Trigger load more jobs for the current workflow
    if let Some(detail) = &self.pipeline_detail_screen {
        if !detail.workflows.is_empty() {
            let workflow_id = detail.workflows[detail.selected_workflow_index].id.clone();
            // Set loading state and trigger async load
            if let Some(d) = &mut self.pipeline_detail_screen {
                d.loading_more_jobs = true;
            }
            self.pending_load_more_jobs = Some(workflow_id);
        }
    }
}
```

#### Added new method to check for pending load more:
```rust
/// Check if more jobs need to be loaded (pagination)
///
/// Returns the workflow_id if more jobs need to be loaded.
pub fn should_load_more_jobs(&self) -> Option<String> {
    self.pending_load_more_jobs.clone()
}
```

### 2. Main Event Loop (`src/main.rs`)

Added load more jobs check to the main async event loop (lines 76-84):
```rust
// Check if we need to load more jobs (pagination)
if let Some(workflow_id) = app.should_load_more_jobs() {
    // Clear the pending flag
    app.pending_load_more_jobs = None;
    // Load more jobs
    if let Err(e) = app.load_more_jobs(&workflow_id).await {
        eprintln!("Error loading more jobs: {}", e);
    }
}
```

## Existing Infrastructure (Already Implemented)

The following components were already in place and didn't require changes:

### UI Components (`src/ui/screens/pipeline_detail.rs`)
- ✅ "Load More" button rendering (lines 984-1008)
- ✅ Keyboard handler for 'l' key (lines 358-365)
- ✅ `can_load_more()` method (lines 209-212)
- ✅ `append_jobs()` method (lines 191-207)
- ✅ `loading_more_jobs` state field
- ✅ `next_page_token` field
- ✅ Pagination info display in footer (lines 1077-1081)

### API Client (`src/api/client.rs`)
- ✅ `get_jobs_page()` method with pagination support
- ✅ Returns `(Vec<Job>, Option<String>)` tuple with next page token

### App Methods (`src/app.rs`)
- ✅ `load_more_jobs()` async method (lines 479-510)
- ✅ Calls `api_client.get_jobs_page()` with token
- ✅ Calls `detail.append_jobs()` to add new jobs

## How It Works

1. **User presses 'l' key** when focused on jobs panel
2. **UI handler** (`pipeline_detail.rs`) checks `can_load_more()` and returns `PipelineDetailAction::LoadMoreJobs`
3. **App event handler** (`app.rs`) sets `loading_more_jobs = true` and stores workflow_id in `pending_load_more_jobs`
4. **Main event loop** (`main.rs`) detects pending load, clears the flag, and calls `app.load_more_jobs()` asynchronously
5. **API call** fetches next page using the stored `next_page_token`
6. **Jobs appended** to existing list and UI updates with new jobs
7. **Loading state cleared** and new `next_page_token` stored for next page

## Testing Instructions

### Prerequisites
- A workflow with more than 10 jobs (CircleCI returns 10 jobs per page by default)
- The Rust CircleCI TUI running

### Test Steps
1. Navigate to a pipeline with multiple workflows
2. Select a workflow with many jobs (>10)
3. Focus on the jobs panel (Tab key)
4. Look for "[Load More Jobs]" button at the bottom of the job list
5. Press 'l' key to load more jobs
6. Verify:
   - Loading indicator appears briefly
   - New jobs are appended to the list
   - Selection is preserved
   - Pagination info updates (e.g., "Showing 20 of 30+")
7. Repeat pressing 'l' to load all pages until no more jobs available

### Expected Behavior
- ✅ "Load More" button only appears when `next_page_token` exists
- ✅ Pressing 'l' triggers async job loading
- ✅ Loading state shows "Loading more jobs..." indicator
- ✅ New jobs append to existing list (not replace)
- ✅ Job selection/scroll position preserved
- ✅ Footer shows '[l] Load More' when pagination available
- ✅ Button disappears when all jobs loaded

## Key Design Decisions

### Async Pattern
- Uses the same async loading pattern as workflows and job logs
- Non-blocking: UI remains responsive during API calls
- Error handling logs to stderr but doesn't crash the app

### State Management
- `loading_more_jobs` separate from `loading_jobs` to allow different UI states
- `pending_load_more_jobs` triggers loading on next event loop iteration
- Token stored in `PipelineDetailScreen` state for persistence

### User Experience
- Keyboard shortcut 'l' (for "load") when focused on jobs panel
- Footer dynamically shows shortcut only when pagination available
- Pagination info shows current count vs total (e.g., "20 of 50+")
- Loading indicator replaces "Load More" button during fetch

## References

### Python Implementation
The Python version uses a similar pattern:
- `/home/pedot/projects/circleci-tui/src/cci/screens/workflow.py` (lines 253-331)
- Shows "▼ Load More Jobs ▼" button
- Appends jobs on load
- Updates pagination info widget

### Related Files Modified
1. `/home/pedot/projects/circleci-tui-rs/src/app.rs`
2. `/home/pedot/projects/circleci-tui-rs/src/main.rs`

### Files Using Pagination (No Changes Needed)
1. `/home/pedot/projects/circleci-tui-rs/src/ui/screens/pipeline_detail.rs`
2. `/home/pedot/projects/circleci-tui-rs/src/api/client.rs`

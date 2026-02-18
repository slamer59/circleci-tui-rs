# Load More Jobs - Testing Guide

## Quick Start

### Build the Application
```bash
cd /home/pedot/projects/circleci-tui-rs
cargo build --release
```

### Run the Application
```bash
cargo run
```

## Finding a Workflow with Pagination

### Option 1: Look for Workflows with Many Jobs
CircleCI returns 10 jobs per page by default, so you need a workflow with >10 jobs.

Common scenarios that create many jobs:
- **Matrix builds** - testing across multiple platforms/versions
- **Parallel test splits** - running tests in parallel
- **Multi-environment deployments** - staging, production, etc.
- **Monorepo builds** - building multiple packages/services

### Option 2: Check Your Recent Pipelines
1. Navigate to CircleCI web UI
2. Find a pipeline with a workflow that has >10 jobs
3. Note the pipeline number
4. Use that pipeline in the TUI

### Option 3: Trigger a Large Build (if you have access)
If you control the repository, you can temporarily create a workflow with many jobs:

```yaml
version: 2.1

workflows:
  test-pagination:
    jobs:
      - test-1
      - test-2
      - test-3
      # ... add more jobs (15-20 recommended for testing)
      - test-20

jobs:
  test-1:
    docker:
      - image: cimg/base:stable
    steps:
      - run: echo "Job 1"
  # Repeat for each job
```

## Step-by-Step Testing

### 1. Launch the TUI
```bash
./target/release/circleci-tui-rs
# or
cargo run
```

### 2. Navigate to Pipeline Detail
```
[↑↓]      Navigate through pipelines
[Enter]   Select a pipeline to view details
```

You should see:
```
┌─────────────────────────────────────────────────────────┐
│ WORKFLOWS          │ JOBS › workflow-name                │
│                    │                                     │
│ ▶ ● build-and-test │ ● 09:15  job-1      2m 30s     ●  │
│   ● deploy-staging │      Job status message...          │
│                    │ ● 09:17  job-2      1m 15s     ●  │
│                    │      Job status message...          │
│                    │ ...                                 │
│                    │ [Load More Jobs] (10 of 25+)       │
└─────────────────────────────────────────────────────────┘
```

### 3. Focus on Jobs Panel
```
[Tab]     Switch focus to jobs panel
```

When focused, the jobs panel border will be highlighted.

### 4. Trigger Load More
```
[l]       Load more jobs (when pagination available)
```

### 5. Observe the Behavior

#### Before Loading
```
│ [Load More Jobs] (Showing 10 of 20+) │
```

#### During Loading
```
│ ● Loading more jobs...               │
```

#### After Loading
```
│ ● 09:15  job-11     3m 10s     ●    │
│      Job status message...          │
│ [Load More Jobs] (Showing 20 of 30+)│
```

#### All Jobs Loaded
```
│ (All 25 jobs loaded)                │
```

### 6. Verify Key Behaviors

#### ✅ Jobs Append (Don't Replace)
- First load shows jobs 1-10
- Second load shows jobs 1-20
- Third load shows jobs 1-30
- etc.

#### ✅ Selection Preserved
- Select job #5
- Press 'l' to load more
- Job #5 should still be selected
- Scroll position maintained

#### ✅ Pagination Info Updates
- Shows current count / total
- Updates after each load
- Shows "+" if more pages exist
- Shows exact count when all loaded

#### ✅ Footer Updates
- Shows `[l] Load More` when pagination available
- Hides shortcut when all jobs loaded

#### ✅ Loading State
- Button changes to "Loading more jobs..."
- Cannot trigger load again while loading
- Button reappears after load completes

## Visual Indicators

### Load More Button States

#### Available to Load
```
[Load More Jobs] (Showing 10 of 20+)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
Cyan, bold - press 'l' to load
```

#### Loading in Progress
```
● Loading more jobs...
^^^^^^^^^^^^^^^^^^^^^^^
Yellow/Running color
```

#### All Jobs Loaded
```
(All 25 jobs loaded)
^^^^^^^^^^^^^^^^^^^^
No button - pagination complete
```

### Footer Shortcuts

#### Pagination Available (Jobs Panel Focused)
```
[↑↓] Nav  [Tab] Switch  [⏎] View Logs  [f] Toggle Filters  [l] Load More  [?] Help  [Esc] Back
                                                             ^^^^^^^^^^^^^
```

#### No Pagination (All Jobs Loaded)
```
[↑↓] Nav  [Tab] Switch  [⏎] View Logs  [f] Toggle Filters  [?] Help  [Esc] Back
                                                             (no load more shortcut)
```

## Troubleshooting

### Issue: "Load More" Button Never Appears
**Possible Causes:**
- Workflow has ≤10 jobs (no pagination needed)
- API didn't return `next_page_token`
- Jobs are filtered out (try disabling filters)

**Solution:**
1. Check total job count in CircleCI web UI
2. Disable all filters (press 'f' and enable all status types)
3. Try a different workflow with more jobs

### Issue: Pressing 'l' Does Nothing
**Possible Causes:**
- Jobs panel not focused (focus is on workflows panel)
- No pagination available (all jobs loaded)
- Loading already in progress

**Solution:**
1. Press `[Tab]` to focus jobs panel (border should be highlighted)
2. Verify "Load More" button is visible
3. Wait for current load to complete

### Issue: Jobs List is Empty After Load
**Possible Causes:**
- API error (check stderr output)
- Network timeout
- Invalid authentication

**Solution:**
1. Check console for error messages: `eprintln!("Error loading more jobs: ...")`
2. Verify CircleCI token is valid
3. Check network connectivity
4. Try refreshing the workflow (Esc, then navigate back)

### Issue: Selection Jumps After Loading
**Possible Causes:**
- Bug in selection preservation logic
- Filtered jobs changed during load

**Solution:**
- This should not happen - file a bug report if observed
- Current implementation preserves `selected_job_index`

## Debug Output

The application logs to stderr, so you can monitor the loading process:

```bash
cargo run 2>&1 | tee debug.log
```

Look for messages like:
```
Error loading more jobs: <error details>
```

## Expected Performance

### Load Times
- **Local network:** 100-500ms per page
- **Remote network:** 500-2000ms per page
- **Slow connection:** 2-5 seconds per page

### UI Responsiveness
- UI should remain responsive during load
- No freezing or blocking
- Smooth animations and updates

## Test Scenarios

### Scenario 1: Single Page Workflow
**Setup:** Workflow with 5-10 jobs
**Expected:** No "Load More" button appears
**Verify:** All jobs loaded immediately

### Scenario 2: Two Page Workflow
**Setup:** Workflow with 15-20 jobs
**Actions:**
1. Open workflow (10 jobs loaded)
2. Press 'l' (remaining jobs loaded)
3. No more pagination

**Verify:**
- First load: "10 of 20+"
- Second load: "All 20 jobs loaded"

### Scenario 3: Many Pages Workflow
**Setup:** Workflow with 50+ jobs
**Actions:**
1. Load page 1 (10 jobs)
2. Load page 2 (20 jobs)
3. Load page 3 (30 jobs)
4. Continue until all loaded

**Verify:**
- Pagination info updates correctly
- No duplicate jobs
- Selection preserved across loads

### Scenario 4: Load While Filtering
**Setup:** Workflow with 20+ jobs, mixed statuses
**Actions:**
1. Enable only "failed" filter
2. Trigger load more
3. Verify filtered jobs append correctly

**Verify:**
- Only failed jobs shown (across all pages)
- Pagination info accounts for filtering
- Load more fetches all jobs (filter applied after)

### Scenario 5: Switch Workflows During Load
**Setup:** Two workflows with many jobs
**Actions:**
1. Start loading jobs for workflow A
2. Immediately switch to workflow B

**Expected:**
- Load cancels or completes in background
- Workflow B loads fresh (no stale data)
- No crashes or errors

## Validation Criteria

### ✅ Core Functionality
- [ ] "Load More" button appears for paginated workflows
- [ ] Pressing 'l' triggers job loading
- [ ] New jobs append to existing list
- [ ] Pagination token updates correctly
- [ ] All jobs eventually loaded

### ✅ User Experience
- [ ] Loading indicator shows during fetch
- [ ] UI remains responsive
- [ ] Selection preserved after load
- [ ] Scroll position maintained
- [ ] Pagination info accurate

### ✅ Edge Cases
- [ ] Works with filters enabled
- [ ] Handles API errors gracefully
- [ ] No duplicate jobs loaded
- [ ] Correct behavior with 0 jobs
- [ ] Correct behavior with exactly 10 jobs

### ✅ Performance
- [ ] No UI freezing during load
- [ ] Acceptable load times (<3s typical)
- [ ] Memory usage reasonable
- [ ] No memory leaks after multiple loads

## Success Criteria

The implementation is successful if:

1. ✅ Users can load all jobs for any workflow
2. ✅ UI provides clear feedback during loading
3. ✅ Job list state is preserved correctly
4. ✅ Keyboard shortcuts work as documented
5. ✅ Error handling prevents crashes

## Next Steps After Testing

If testing reveals issues:
1. Check stderr for error messages
2. Verify API responses (add debug logging)
3. Test with different workflows
4. Compare behavior with Python version
5. File bug reports with reproduction steps

If testing succeeds:
1. ✅ Mark task as complete
2. Update documentation
3. Consider optional enhancements
4. Deploy to users

---

**Happy Testing! 🧪🚀**

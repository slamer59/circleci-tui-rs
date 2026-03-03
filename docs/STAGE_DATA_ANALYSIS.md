# Stage Data Analysis

## Executive Summary

After analyzing the codebase, I've determined that:

1. **Pipeline model does NOT contain workflow/stage data** - only metadata
2. **Workflows must be fetched separately** via `get_workflows(pipeline_id)`
3. **Real stage indicators are achievable** but require additional API calls
4. **Recommendation: Use mock stages for now**, add real data as future enhancement

---

## 1. Current Pipeline Model Data

### Data Structure
From `/home/pedot/projects/circleci-tui-rs/src/api/models.rs` (lines 5-14):

```rust
pub struct Pipeline {
    pub id: String,
    pub number: u32,
    pub state: String,              // Overall pipeline state
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub vcs: VcsInfo,               // Git/branch info
    pub trigger: TriggerInfo,       // How it was triggered
    pub project_slug: String,
}
```

### What's Available in Pipeline Response

The pipeline model includes:
- Pipeline-level state (success/failed/running/pending)
- VCS metadata (branch, revision, commit message, author)
- Trigger information (webhook/scheduled/api)
- Timestamps

### What's NOT Available in Pipeline Response

The pipeline model does NOT include:
- Workflow list or summary
- Stage information
- Job counts or statuses
- Per-workflow status breakdown

**Current workaround**: Line 884 in pipelines.rs shows hardcoded data:
```rust
let summary = format!(
    "          3 workflows • 24 jobs • {} ({})",
    pipeline.state, duration
);
```

---

## 2. Workflow Data (Separate API Call)

### Workflow Model
From `/home/pedot/projects/circleci-tui-rs/src/api/models.rs` (lines 32-39):

```rust
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub status: String,             // success/failed/running/pending/blocked/canceled
    pub created_at: DateTime<Utc>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub pipeline_id: String,
}
```

### API Call Required
From `/home/pedot/projects/circleci-tui-rs/src/api/client.rs` (lines 353-387):

```rust
pub async fn get_workflows(&self, pipeline_id: &str) -> Result<Vec<Workflow>, ApiError>
```

**API Endpoint**: `GET /api/v2/pipeline/{pipeline_id}/workflow`

### What You Get

For each workflow:
- Workflow name (e.g., "build-and-test", "integration-tests", "deploy-staging")
- Status (mapped: success/failed/running/pending/blocked/canceled)
- Duration (via created_at and stopped_at timestamps)

**Status mapping** (lines 19-34 in client.rs):
```rust
fn map_status(status: &str) -> String {
    match status {
        "created" => "pending",
        "running" => "running",
        "not_run" => "blocked",
        "success" => "success",
        "failed" => "failed",
        "error" => "failed",
        "failing" => "failed",
        "on_hold" => "blocked",
        "canceled" => "blocked",
        "unauthorized" => "blocked",
        _ => "unknown",
    }
    .to_string()
}
```

---

## 3. Can We Show Real Stage Status?

### Yes - Here's How

**Option A: Fetch workflows for each pipeline in view**

```rust
// In pipelines screen (parallel fetching)
for pipeline in visible_pipelines {
    let workflows = client.get_workflows(&pipeline.id).await?;

    // Compute stage icons
    let stage_status = workflows.iter().map(|w| {
        match w.status.as_str() {
            "success" => "✓",
            "failed" => "✗",
            "running" => "⊙",
            "pending" | "blocked" => "⊘",
            _ => "?",
        }
    }).collect::<String>();

    // Display: ✓✓⊘⊘ or ✓✗⊘⊘
}
```

**Option B: Fetch once on pipeline selection**
- Only fetch workflows when user selects a pipeline
- Cache results in pipeline detail screen
- Show indicator that "stage info loading..."

**Option C: Batch API calls**
- Fetch workflows for top 10 visible pipelines
- Update UI as data arrives
- Show loading indicator for pending data

---

## 4. Performance Considerations

### API Call Overhead

**Current**: 1 API call for pipelines list
```
GET /api/v2/project/{project_slug}/pipeline
```

**With real stages**: N+1 API calls
```
GET /api/v2/project/{project_slug}/pipeline          # 1 call
GET /api/v2/pipeline/{id_1}/workflow                 # 1 per pipeline
GET /api/v2/pipeline/{id_2}/workflow
...
GET /api/v2/pipeline/{id_N}/workflow
```

### Performance Impact

| Scenario | API Calls | Estimated Time | UX Impact |
|----------|-----------|----------------|-----------|
| Mock stages | 1 | ~200ms | Instant |
| Real stages (10 pipelines) | 11 | ~2-3 seconds | Noticeable delay |
| Real stages (50 pipelines) | 51 | ~10-15 seconds | Unacceptable |

### Mitigation Strategies

1. **Progressive loading**: Show pipelines immediately, fetch workflows in background
2. **Lazy loading**: Only fetch workflows for visible pipelines (viewport)
3. **Caching**: Cache workflow data for 30-60 seconds
4. **Parallel requests**: Use `tokio::join!` or `futures::stream` for concurrent fetching

---

## 5. Mock Data Implementation

### Current Approach (Recommended for Now)

The codebase already has mock workflow data that demonstrates the structure:

From `/home/pedot/projects/circleci-tui-rs/src/api/models.rs` (lines 273-336):

```rust
pub fn mock_workflows(pipeline_id: &str) -> Vec<Workflow> {
    vec![
        Workflow {
            id: format!("wf-{}-001", pipeline_id),
            name: "build-and-test".to_string(),
            status: "success".to_string(),
            // ...
        },
        Workflow {
            id: format!("wf-{}-002", pipeline_id),
            name: "integration-tests".to_string(),
            status: "success".to_string(),
            // ...
        },
        Workflow {
            id: format!("wf-{}-003", pipeline_id),
            name: "deploy-staging".to_string(),
            status: "success".to_string(),
            // ...
        },
    ]
}
```

### Mock Stage Algorithm

For displaying stage indicators without API calls:

```rust
fn compute_mock_stages(pipeline: &Pipeline) -> String {
    match pipeline.state.as_str() {
        "success" => "✓✓✓✓",
        "failed" => "✓✓✗⊘",
        "running" => "✓✓⊙⊘",
        "pending" => "⊙⊘⊘⊘",
        _ => "✓✓✓✓",
    }
}
```

**Pros:**
- Zero API overhead
- Instant display
- Reasonable approximation of reality

**Cons:**
- Not accurate (just based on overall pipeline state)
- Doesn't reflect actual workflow names/count
- Users can't trust the specific stage status

---

## 6. Data Hierarchy

### How It All Fits Together

```
Pipeline (1 API call)
├── id: "pipe-001"
├── state: "success"
└── workflows: NOT INCLUDED (requires separate call)

Workflow (1 API call per pipeline)
├── id: "wf-pipe-001-001"
├── name: "build-and-test"
├── status: "success"
└── jobs: NOT INCLUDED (requires separate call)

Job (1 API call per workflow)
├── id: "job-wf-001-001"
├── name: "unit-tests"
├── status: "success"
└── steps: NOT INCLUDED (requires separate v1.1 API call)

JobStep (v1.1 API - different endpoint)
├── name: "Run Tests"
├── status: "success"
└── actions: [...]
```

**Lesson**: CircleCI API is intentionally hierarchical to avoid massive payloads.

---

## 7. Recommendation

### Short Term (Current Phase)

**Keep mocked stages** for performance and simplicity:

```rust
// In pipelines.rs line ~883
let summary = format!(
    "          {} • {} • {} ({})",
    compute_mock_stages(&pipeline),  // ✓✓✓✓ or ✓✗⊘⊘
    "3 workflows • 24 jobs",          // Still mock for now
    pipeline.state,
    duration
);
```

**Why?**
1. Instant UI rendering
2. No API overhead for browsing pipelines
3. Users primarily care about overall pipeline state (which is real)
4. Detail screen already shows real workflow data

### Medium Term (Future Enhancement)

**Add real stage data** with progressive loading:

```rust
pub struct PipelinesScreen {
    // Existing fields...
    workflow_cache: HashMap<String, Vec<Workflow>>,  // pipeline_id -> workflows
    loading_workflows: HashSet<String>,              // pipelines being fetched
}

impl PipelinesScreen {
    pub async fn fetch_workflows_for_visible(&mut self, client: &CircleCIClient) {
        let visible = self.get_visible_pipeline_ids();

        for pipeline_id in visible {
            if !self.workflow_cache.contains_key(pipeline_id)
                && !self.loading_workflows.contains(pipeline_id) {

                self.loading_workflows.insert(pipeline_id.clone());

                // Spawn async task
                tokio::spawn(async move {
                    let workflows = client.get_workflows(pipeline_id).await;
                    // Send back via channel...
                });
            }
        }
    }

    fn render_stage_indicators(&self, pipeline: &Pipeline) -> String {
        if let Some(workflows) = self.workflow_cache.get(&pipeline.id) {
            // Real data
            workflows.iter().map(|w| match w.status.as_str() {
                "success" => "✓",
                "failed" => "✗",
                "running" => "⊙",
                _ => "⊘",
            }).collect()
        } else {
            // Fallback to mock while loading
            compute_mock_stages(pipeline)
        }
    }
}
```

**Benefits:**
- Progressive enhancement
- No blocking on initial load
- Real data appears as it arrives

---

## 8. Implementation Path

### Phase 1: Keep Current Mock (Recommended)
- ✓ Already implemented
- ✓ Fast and reliable
- ✓ Good enough for MVP

### Phase 2: Add Cache Infrastructure
```rust
// Add to pipelines screen
workflow_cache: HashMap<String, (Vec<Workflow>, Instant)>,  // with TTL
```

### Phase 3: Lazy Loading
```rust
// On scroll or refresh
- Check which pipelines are visible
- Fetch workflows for visible pipelines not in cache
- Update cache asynchronously
- Re-render when data arrives
```

### Phase 4: Polish
```rust
// Show loading indicator
- "✓✓..." (loading)
- Animate spinner for running workflows
- Add tooltips showing workflow names
```

---

## 9. Code Snippets

### Example: Real Stage Status Fetching

```rust
use std::collections::HashMap;
use tokio::sync::mpsc;

pub struct PipelinesScreen {
    pipelines: Vec<Pipeline>,
    workflow_cache: HashMap<String, Vec<Workflow>>,
}

impl PipelinesScreen {
    pub fn render_pipeline_row(&self, pipeline: &Pipeline) -> String {
        let stage_icons = if let Some(workflows) = self.workflow_cache.get(&pipeline.id) {
            // Real data available
            workflows
                .iter()
                .map(|w| match w.status.as_str() {
                    "success" => "✓",
                    "failed" => "✗",
                    "running" => "⊙",
                    "pending" | "blocked" | "canceled" => "⊘",
                    _ => "?",
                })
                .collect::<String>()
        } else {
            // Fallback to mock
            match pipeline.state.as_str() {
                "success" => "✓✓✓✓",
                "failed" => "✓✓✗⊘",
                "running" => "✓✓⊙⊘",
                "pending" => "⊙⊘⊘⊘",
                _ => "✓✓✓✓",
            }
        };

        format!(
            "          {} • {} • {} ({})",
            stage_icons,
            format!("{} workflows",
                self.workflow_cache.get(&pipeline.id)
                    .map(|w| w.len())
                    .unwrap_or(3)),
            pipeline.state,
            duration
        )
    }

    pub async fn start_workflow_fetching(
        &mut self,
        client: Arc<CircleCIClient>,
        tx: mpsc::Sender<(String, Vec<Workflow>)>,
    ) {
        for pipeline in &self.pipelines {
            if self.workflow_cache.contains_key(&pipeline.id) {
                continue; // Already cached
            }

            let pipeline_id = pipeline.id.clone();
            let client = Arc::clone(&client);
            let tx = tx.clone();

            tokio::spawn(async move {
                if let Ok(workflows) = client.get_workflows(&pipeline_id).await {
                    let _ = tx.send((pipeline_id, workflows)).await;
                }
            });
        }
    }
}
```

### Example: Status to Icon Mapping

```rust
pub fn workflow_status_to_icon(status: &str) -> &'static str {
    match status {
        "success" => "✓",
        "failed" | "error" | "failing" => "✗",
        "running" => "⊙",
        "pending" | "created" => "⊘",
        "blocked" | "canceled" | "on_hold" | "not_run" => "⊘",
        _ => "?",
    }
}

pub fn workflows_to_stage_string(workflows: &[Workflow]) -> String {
    if workflows.is_empty() {
        return "⊘".to_string();
    }

    workflows
        .iter()
        .map(|w| workflow_status_to_icon(&w.status))
        .collect()
}
```

---

## 10. Testing Strategy

### Mock Testing (Current)
```bash
cargo run
# All pipelines show stage indicators immediately
# Indicators are approximations based on pipeline state
```

### Real Data Testing (Future)
```bash
# Set CIRCLECI_TOKEN and PROJECT_SLUG
export CIRCLECI_TOKEN="your-token"
export PROJECT_SLUG="gh/org/repo"

cargo run
# Observe:
# - Initial render shows mock stages
# - After 1-2 seconds, real stages appear
# - Console logs API calls
```

### Performance Testing
```bash
# Measure API call latency
time curl -H "Circle-Token: $TOKEN" \
  "https://circleci.com/api/v2/pipeline/{id}/workflow"

# Typical: 100-300ms per call
# 10 pipelines = 1-3 seconds total (if parallel)
```

---

## Summary Table

| Aspect | Current State | Real Data Available? | API Call Required |
|--------|---------------|---------------------|-------------------|
| Pipeline list | Real | Yes | 1 call (already done) |
| Pipeline state | Real | Yes | Included in list |
| Stage count | Mock (3) | Yes | +1 per pipeline |
| Stage status | Mock (✓✓✓✓) | Yes | +1 per pipeline |
| Workflow names | Not shown | Yes | +1 per pipeline |
| Job count | Mock (24) | Yes | +1 per workflow |

**Key Insight**: Getting real stage data requires N additional API calls (one per pipeline). This is why mock data is currently used for performance.

---

## Conclusion

1. **Pipeline model** contains metadata only, no workflow/stage summary
2. **Workflow data** is available via `get_workflows(pipeline_id)` but requires N API calls
3. **Real stage status** is technically achievable with proper async/caching architecture
4. **Recommendation**: Keep mock stages for now, add real data as Phase 2 enhancement
5. **Current approach** strikes the right balance between UX and performance

The detail screen already shows real workflow data (when user drills down), so the most important data is accessible. Stage indicators on the pipeline list are "nice to have" but not critical for functionality.

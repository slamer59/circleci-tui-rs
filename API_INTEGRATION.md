# CircleCI API Integration Implementation

## Overview

This document describes the real CircleCI API integration that replaces the mock data system.

## Implementation Summary

### 1. API Client (`/home/pedot/projects/circleci-tui-rs/src/api/client.rs`)

**CircleCIClient** - Main API client struct:
```rust
pub struct CircleCIClient {
    client: reqwest::Client,
    base_url: String,
    project_slug: String,
}
```

**Key Features:**
- HTTP client with 30-second timeout
- Authentication via `Circle-Token` header
- Base URL: `https://circleci.com/api/v2`
- Pagination support for large result sets

**Implemented Methods:**

#### `new(token: String, project_slug: String) -> Result<Self, ApiError>`
Creates a new API client with authentication headers.

#### `get_pipelines(&self, limit: usize) -> Result<Vec<Pipeline>, ApiError>`
- **Endpoint:** `GET /project/{project-slug}/pipeline`
- **Features:**
  - Fetches up to `limit` pipelines (default: 50)
  - Automatic pagination using `next_page_token`
  - Maps CircleCI status to internal status (created→pending, not_run→blocked, etc.)
  - Extracts commit info from nested `vcs.commit` structure
  - Extracts author from `trigger.actor.login`
  - Truncates SHA to 7 characters

#### `get_workflows(&self, pipeline_id: &str) -> Result<Vec<Workflow>, ApiError>`
- **Endpoint:** `GET /pipeline/{pipeline-id}/workflow`
- **Features:**
  - Fetches all workflows for a pipeline
  - Maps status values
  - Includes created_at and stopped_at timestamps for duration calculation

#### `get_jobs(&self, workflow_id: &str) -> Result<Vec<Job>, ApiError>`
- **Endpoint:** `GET /workflow/{workflow-id}/job`
- **Features:**
  - Fetches all jobs for a workflow
  - Calculates duration from started_at/stopped_at timestamps
  - Maps status values
  - Extracts executor type

### 2. Status Mapping

CircleCI API statuses are mapped to internal status values:

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
}
```

### 3. API Response Structures

Internal structs for deserializing API responses:

**PipelineResponse:**
```rust
struct PipelineResponse {
    items: Vec<PipelineItem>,
    next_page_token: Option<String>,
}

struct PipelineItem {
    id: String,
    number: u32,
    state: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    vcs: Option<VcsResponse>,
    trigger: Option<TriggerResponse>,
    project_slug: String,
}
```

**WorkflowResponse:**
```rust
struct WorkflowResponse {
    items: Vec<WorkflowItem>,
    next_page_token: Option<String>,
}

struct WorkflowItem {
    id: String,
    name: String,
    status: String,
    created_at: DateTime<Utc>,
    stopped_at: Option<DateTime<Utc>>,
    pipeline_id: String,
}
```

**JobResponse:**
```rust
struct JobResponse {
    items: Vec<JobItem>,
    next_page_token: Option<String>,
}

struct JobItem {
    id: String,
    name: String,
    status: String,
    job_number: u32,
    started_at: Option<DateTime<Utc>>,
    stopped_at: Option<DateTime<Utc>>,
    executor_type: Option<String>,
}
```

### 4. Error Handling (`/home/pedot/projects/circleci-tui-rs/src/api/error.rs`)

**ApiError enum:**
```rust
pub enum ApiError {
    Network(String),      // Connection errors
    Http(u16, String),    // HTTP error responses
    Parse(String),        // JSON parsing errors
    Timeout,              // Request timeout
}
```

**Features:**
- Automatic conversion from `reqwest::Error`
- Distinguishes timeout, HTTP status, and network errors
- Implements `std::error::Error` trait

### 5. App Integration (`/home/pedot/projects/circleci-tui-rs/src/app.rs`)

**Updated App struct:**
```rust
pub struct App {
    // ... existing fields ...
    pub api_client: Arc<CircleCIClient>,
    pub is_loading: bool,
}
```

**New async methods:**

#### `load_pipelines(&mut self) -> Result<()>`
Fetches pipelines from API and updates PipelineScreen:
```rust
pub async fn load_pipelines(&mut self) -> Result<()> {
    self.is_loading = true;
    let pipelines = self.api_client.get_pipelines(50).await?;
    self.pipeline_screen.set_pipelines(pipelines);
    self.is_loading = false;
    Ok(())
}
```

#### `load_workflows(&mut self, pipeline_id: &str) -> Result<()>`
Fetches workflows for a pipeline and updates PipelineDetailScreen.

#### `load_jobs(&mut self, workflow_id: &str) -> Result<()>`
Fetches jobs for a workflow and updates PipelineDetailScreen.

### 6. Screen Updates

**PipelineScreen** (`/home/pedot/projects/circleci-tui-rs/src/ui/screens/pipelines.rs`):
```rust
pub fn set_pipelines(&mut self, pipelines: Vec<Pipeline>) {
    self.pipelines = pipelines;
    self.apply_filters();
}
```

**PipelineDetailScreen** (`/home/pedot/projects/circleci-tui-rs/src/ui/screens/pipeline_detail.rs`):
```rust
pub fn set_workflows(&mut self, workflows: Vec<Workflow>) {
    self.workflows = workflows;
    // Update selection state
}

pub fn set_jobs(&mut self, jobs: Vec<Job>) {
    self.jobs = jobs;
    // Update selection state
}
```

### 7. Main Entry Point (`/home/pedot/projects/circleci-tui-rs/src/main.rs`)

**Updated to use async:**
```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::load()?;

    // Create app with API client
    let mut app = App::new(config)?;

    // Load initial data
    app.load_pipelines().await?;

    // Run event loop
    app.run(&mut terminal)?;

    Ok(())
}
```

## Configuration

The API client requires environment variables:

```bash
CIRCLECI_TOKEN=your_circle_ci_token_here
PROJECT_SLUG=gh/owner/repo
```

## Testing

Run `cargo check` to verify compilation:
```bash
cargo check
```

All code compiles successfully with only unused code warnings (from mock data functions).

## API Documentation Reference

Based on CircleCI API v2 documentation:
- Pipelines: https://circleci.com/docs/api/v2/index.html#operation/listPipelinesForProject
- Workflows: https://circleci.com/docs/api/v2/index.html#operation/listWorkflowsByPipelineId
- Jobs: https://circleci.com/docs/api/v2/index.html#operation/listWorkflowJobs

## Future Enhancements

Potential improvements for future implementation:

1. **Caching:** Add response caching to reduce API calls
2. **Background Refresh:** Implement periodic background polling
3. **Job Steps:** Add support for v1.1 API to fetch job steps/logs
4. **Webhook Support:** Add workflow rerun functionality
5. **Rate Limiting:** Add rate limit handling and backoff
6. **Error UI:** Show API errors in the UI instead of exiting
7. **Loading States:** Show loading spinners during API calls
8. **Offline Mode:** Fall back to cached data when offline

## Comparison with Python Implementation

The Rust implementation follows the same pattern as the Python version (`/home/pedot/projects/circleci-tui/src/cci/api.py`):

| Feature | Python | Rust |
|---------|--------|------|
| HTTP Client | httpx.AsyncClient | reqwest::Client |
| Timeout | 30s | 30s |
| Authentication | Circle-Token header | Circle-Token header |
| Base URL | https://circleci.com/api/v2 | https://circleci.com/api/v2 |
| Pagination | Automatic with next_page_token | Automatic with next_page_token |
| Status Mapping | STATUS_MAP dict | map_status() function |
| Duration Calc | calculate_duration() | Calculated in get_jobs() |

Both implementations provide identical functionality and API compatibility.

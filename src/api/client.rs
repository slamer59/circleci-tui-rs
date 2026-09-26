use super::error::ApiError;
use super::models::{
    ExecutorInfo, Job, JobStep, Pipeline, StepAction, TriggerInfo, VcsInfo, Workflow,
};
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT};
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

/// CircleCI API v2 client
pub struct CircleCIClient {
    client: reqwest::Client,
    base_url: String,
    project_slug: String,
}

/// Status mapping from CircleCI API to internal status
fn map_status(status: &str) -> String {
    match status {
        "created" => "pending",
        "errored" => "failed",
        "setup-pending" | "setup" | "pending" => "pending",
        "running" => "running",
        "not_run" => "blocked",
        "success" => "success",
        "failed" => "failed",
        "error" => "failed",
        "failing" => "failed",
        "on_hold" => "blocked",
        "canceled" => "blocked",
        "unauthorized" => "blocked",
        _ => "pending",
    }
    .to_string()
}

// API Response structures for deserialization

#[derive(Debug, Deserialize)]
pub struct MeResponse {
    pub login: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PipelineResponse {
    items: Vec<PipelineItem>,
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
struct VcsResponse {
    branch: Option<String>,
    revision: Option<String>,
    commit: Option<CommitResponse>,
}

#[derive(Debug, Deserialize)]
struct CommitResponse {
    subject: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TriggerResponse {
    #[serde(rename = "type")]
    trigger_type: String,
    actor: Option<ActorResponse>,
}

#[derive(Debug, Deserialize)]
struct ActorResponse {
    login: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WorkflowResponse {
    items: Vec<WorkflowItem>,
}

#[derive(Debug, Deserialize)]
struct WorkflowItem {
    id: String,
    name: String,
    status: String,
    created_at: DateTime<Utc>,
    stopped_at: Option<DateTime<Utc>>,
    pipeline_id: String,
}

#[derive(Debug, Deserialize)]
struct JobResponse {
    items: Vec<JobItem>,
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JobItem {
    id: String,
    name: String,
    status: String,
    job_number: u32,
    started_at: Option<DateTime<Utc>>,
    stopped_at: Option<DateTime<Utc>>,
    #[serde(rename = "type")]
    executor_type: Option<String>,
}

/// Messages sent during step-based log streaming
#[derive(Debug)]
pub enum StepStreamMessage {
    /// Step metadata discovered - (name, status) pairs for all steps
    StepsDiscovered(Vec<(String, String)>),
    /// Logs fetched for a specific step
    StepLogsFetched {
        step_index: usize,
        logs: Vec<String>,
    },
}

impl CircleCIClient {
    /// Get current user information
    ///
    /// # Returns
    /// User information including login and optional name
    pub async fn get_me(&self) -> Result<MeResponse, ApiError> {
        let url = format!("{}/me", self.base_url);

        // Make API request
        let response = self.client.get(&url).send().await?;

        // Check for errors
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::Http(status, body));
        }

        // Parse response
        let me_response: MeResponse = response
            .json()
            .await
            .map_err(|e| ApiError::Parse(format!("Failed to parse user info: {}", e)))?;

        Ok(me_response)
    }

    /// Create a new CircleCI API client
    ///
    /// # Arguments
    /// * `token` - CircleCI API token
    /// * `project_slug` - Project slug in format "gh/org/repo" or "bb/org/repo"
    pub fn new(token: String, project_slug: String) -> Result<Self, ApiError> {
        // Build headers
        let mut headers = HeaderMap::new();
        headers.insert(
            "Circle-Token",
            HeaderValue::from_str(&token)
                .map_err(|e| ApiError::Network(format!("Invalid token: {}", e)))?,
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        // Build reqwest client with timeout
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ApiError::Network(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self {
            client,
            base_url: "https://circleci.com/api/v2".to_string(),
            project_slug,
        })
    }

    /// Get pipelines for the project with optional filters (server-side filtering)
    ///
    /// # Arguments
    /// * `limit` - Maximum number of pipelines to fetch
    /// * `branch` - Optional branch name to filter by
    /// * `mine` - If true, only return pipelines triggered by the authenticated user
    ///
    /// # Returns
    /// A vector of pipelines matching the filters
    pub async fn get_pipelines_filtered(
        &self,
        limit: usize,
        branch: Option<&str>,
        mine: bool,
    ) -> Result<Vec<Pipeline>, ApiError> {
        let mut all_pipelines = Vec::new();
        let mut next_page_token: Option<String> = None;

        // Keep fetching pages until we have enough pipelines
        while all_pipelines.len() < limit {
            let base_url = format!("{}/project/{}/pipeline", self.base_url, self.project_slug);
            let mut query_params = Vec::new();

            // Add branch filter
            if let Some(branch_name) = branch {
                query_params.push(format!("branch={}", branch_name));
            }

            // Add mine filter
            if mine {
                query_params.push("mine=true".to_string());
            }

            // Add page token if we have one
            if let Some(token) = &next_page_token {
                query_params.push(format!("page-token={}", token));
            }

            // Build full URL with query params
            let url = if !query_params.is_empty() {
                format!("{}?{}", base_url, query_params.join("&"))
            } else {
                base_url
            };

            // Make API request
            let response = self.client.get(&url).send().await?;

            // Check for errors
            if !response.status().is_success() {
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();
                return Err(ApiError::Http(status, body));
            }

            // Parse response
            let pipeline_response: PipelineResponse = response
                .json()
                .await
                .map_err(|e| ApiError::Parse(format!("Failed to parse pipelines: {}", e)))?;

            // Convert API items to our Pipeline model
            for item in pipeline_response.items {
                let vcs = item.vcs.as_ref();
                let commit = vcs.and_then(|v| v.commit.as_ref());
                let trigger = item.trigger.as_ref();
                let actor = trigger.and_then(|t| t.actor.as_ref());

                all_pipelines.push(Pipeline {
                    id: item.id,
                    number: item.number,
                    state: map_status(&item.state),
                    created_at: item.created_at,
                    updated_at: item.updated_at,
                    vcs: VcsInfo {
                        branch: vcs
                            .and_then(|v| v.branch.clone())
                            .unwrap_or_else(|| "unknown".to_string()),
                        revision: vcs
                            .and_then(|v| v.revision.as_ref())
                            .map(|r| {
                                // Take first 7 characters for short SHA
                                if r.len() > 7 {
                                    r[..7].to_string()
                                } else {
                                    r.clone()
                                }
                            })
                            .unwrap_or_else(|| "unknown".to_string()),
                        commit_subject: commit
                            .and_then(|c| c.subject.clone())
                            .unwrap_or_else(|| "No commit message".to_string()),
                        commit_author_name: actor
                            .and_then(|a| a.login.clone())
                            .unwrap_or_else(|| "unknown".to_string()),
                        commit_timestamp: item.created_at,
                    },
                    trigger: TriggerInfo {
                        trigger_type: trigger
                            .map(|t| t.trigger_type.clone())
                            .unwrap_or_else(|| "unknown".to_string()),
                    },
                    project_slug: item.project_slug,
                });

                if all_pipelines.len() >= limit {
                    break;
                }
            }

            // Check if there's a next page
            next_page_token = pipeline_response.next_page_token;
            if next_page_token.is_none() {
                break;
            }
        }

        Ok(all_pipelines)
    }

    /// Fetch workflows for a pipeline
    ///
    /// # Arguments
    /// * `pipeline_id` - Pipeline ID
    ///
    /// # Returns
    /// List of workflows for the pipeline
    pub async fn get_workflows(&self, pipeline_id: &str) -> Result<Vec<Workflow>, ApiError> {
        let url = format!("{}/pipeline/{}/workflow", self.base_url, pipeline_id);

        // Make API request
        let response = self.client.get(&url).send().await?;

        // Check for errors
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::Http(status, body));
        }

        // Parse response
        let workflow_response: WorkflowResponse = response
            .json()
            .await
            .map_err(|e| ApiError::Parse(format!("Failed to parse workflows: {}", e)))?;

        // Convert API items to our Workflow model
        let workflows = workflow_response
            .items
            .into_iter()
            .map(|item| Workflow {
                id: item.id,
                name: item.name,
                status: item.status,
                created_at: item.created_at,
                stopped_at: item.stopped_at,
                pipeline_id: item.pipeline_id,
            })
            .collect();

        Ok(workflows)
    }

    /// Fetch jobs for a workflow
    ///
    /// # Arguments
    /// * `workflow_id` - Workflow ID
    ///
    /// # Returns
    /// Tuple of (jobs, next_page_token)
    pub async fn get_jobs(
        &self,
        workflow_id: &str,
    ) -> Result<(Vec<Job>, Option<String>), ApiError> {
        self.get_jobs_page(workflow_id, None).await
    }

    /// Fetch jobs for a workflow with pagination support
    ///
    /// # Arguments
    /// * `workflow_id` - Workflow ID
    /// * `page_token` - Optional page token for pagination
    ///
    /// # Returns
    /// Tuple of (jobs, next_page_token)
    pub async fn get_jobs_page(
        &self,
        workflow_id: &str,
        page_token: Option<&str>,
    ) -> Result<(Vec<Job>, Option<String>), ApiError> {
        let mut url = format!("{}/workflow/{}/job", self.base_url, workflow_id);

        // Add page token if provided
        if let Some(token) = page_token {
            url = format!("{}?page-token={}", url, token);
        }

        // Make API request
        let response = self.client.get(&url).send().await?;

        // Check for errors
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::Http(status, body));
        }

        // Parse response
        let job_response: JobResponse = response
            .json()
            .await
            .map_err(|e| ApiError::Parse(format!("Failed to parse jobs: {}", e)))?;

        // Convert API items to our Job model
        let jobs = job_response
            .items
            .into_iter()
            .map(|item| {
                // Calculate duration if both timestamps are available
                let duration =
                    if let (Some(started), Some(stopped)) = (item.started_at, item.stopped_at) {
                        let delta = stopped.signed_duration_since(started);
                        Some(delta.num_seconds() as u32)
                    } else {
                        None
                    };

                Job {
                    id: item.id,
                    name: item.name,
                    status: item.status,
                    job_number: item.job_number,
                    workflow_id: workflow_id.to_string(),
                    started_at: item.started_at,
                    stopped_at: item.stopped_at,
                    duration,
                    executor: ExecutorInfo {
                        executor_type: item.executor_type.unwrap_or_else(|| "unknown".to_string()),
                    },
                }
            })
            .collect();

        Ok((jobs, job_response.next_page_token))
    }

    /// Fetch steps for a job using v1.1 API
    ///
    /// The v2 API doesn't include step information, so we need to use the v1.1 API.
    ///
    /// # Arguments
    /// * `job_number` - Job number
    ///
    /// # Returns
    /// List of job steps with their actions
    pub async fn get_job_steps(&self, job_number: u32) -> Result<Vec<JobStep>, ApiError> {
        // Convert project slug from v2 format (gh/org/repo) to v1.1 format (github/org/repo)
        let project_v1 = self.project_slug.replace("gh/", "github/");
        let url = format!(
            "https://circleci.com/api/v1.1/project/{}/{}",
            project_v1, job_number
        );

        // Make API request with the same auth token
        let response = self.client.get(&url).send().await?;

        // Check for errors
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::Http(status, body));
        }

        // Parse response as generic JSON first
        let json: Value = response
            .json()
            .await
            .map_err(|e| ApiError::Parse(format!("Failed to parse job details: {}", e)))?;

        // Extract steps array
        let steps_array = json
            .get("steps")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ApiError::Parse("No steps field in response".to_string()))?;

        // Parse steps
        let mut steps = Vec::new();
        for step_value in steps_array {
            let step_name = step_value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown step")
                .to_string();

            let empty_vec = Vec::new();
            let actions_array = step_value
                .get("actions")
                .and_then(|v| v.as_array())
                .unwrap_or(&empty_vec);

            let mut actions = Vec::new();
            for (index, action_value) in actions_array.iter().enumerate() {
                let action_name = action_value
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown action")
                    .to_string();

                let status = action_value
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let output_url = action_value
                    .get("output_url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                actions.push(StepAction {
                    name: action_name,
                    status,
                    output_url,
                    index,
                });
            }

            // Determine step status from actions
            let step_status = if actions.iter().any(|a| a.status == "failed") {
                "failed".to_string()
            } else if actions.iter().any(|a| a.status == "running") {
                "running".to_string()
            } else if actions.iter().all(|a| a.status == "success") {
                "success".to_string()
            } else {
                "pending".to_string()
            };

            steps.push(JobStep {
                name: step_name,
                status: step_status,
                actions,
            });
        }

        Ok(steps)
    }

    /// Fetch log output from a URL
    ///
    /// CircleCI returns logs as a JSON array of log entries.
    ///
    /// # Arguments
    /// * `output_url` - URL to fetch logs from (typically an S3 URL)
    ///
    /// # Returns
    /// List of log lines
    pub async fn fetch_log_output_pub(&self, output_url: &str) -> Result<Vec<String>, ApiError> {
        self.fetch_log_output(output_url).await
    }

    async fn fetch_log_output(&self, output_url: &str) -> Result<Vec<String>, ApiError> {
        // Fetch log output (no auth needed for S3 URLs)
        let response = reqwest::get(output_url)
            .await
            .map_err(|e| ApiError::Network(format!("Failed to fetch log output: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            return Err(ApiError::Http(
                status,
                "Failed to fetch log output".to_string(),
            ));
        }

        let text = response
            .text()
            .await
            .map_err(|e| ApiError::Parse(format!("Failed to read log output: {}", e)))?;

        // Try to parse as JSON array
        if let Ok(log_entries) = serde_json::from_str::<Vec<Value>>(&text) {
            let mut lines = Vec::new();
            for entry in log_entries {
                if let Some(message) = entry.get("message").and_then(|v| v.as_str()) {
                    // Split message by newlines and add each line
                    for line in message.lines() {
                        lines.push(line.to_string());
                    }
                }
            }
            Ok(lines)
        } else {
            // Fallback: treat as plain text
            Ok(text.lines().map(|s| s.to_string()).collect())
        }
    }

    /// Stream job logs by fetching all available logs
    ///
    /// For running jobs, this should be called periodically to get updates.
    ///
    /// # Arguments
    /// * `job_number` - Job number to stream logs for
    ///
    /// # Returns
    /// List of formatted log lines with timestamps
    pub async fn stream_job_log(&self, job_number: u32) -> Result<Vec<String>, ApiError> {
        self.stream_job_log_with_progress(job_number, None).await
    }

    /// Fetch all logs for a job with optional progress reporting
    ///
    /// When a progress sender is provided, it sends `(current_step, total_steps, step_name)`
    /// after each step is fetched, enabling progress bar display in the UI.
    pub async fn stream_job_log_with_progress(
        &self,
        job_number: u32,
        progress_tx: Option<tokio::sync::mpsc::UnboundedSender<(usize, usize, String)>>,
    ) -> Result<Vec<String>, ApiError> {
        let steps = self.get_job_steps(job_number).await?;
        let mut all_logs = Vec::new();

        // Count total actions with logs for progress tracking
        let total_actions: usize = steps
            .iter()
            .flat_map(|s| &s.actions)
            .filter(|a| a.output_url.is_some())
            .count();

        // Check if any action has logs
        let has_any_logs = total_actions > 0;

        if !has_any_logs {
            // No logs available yet
            all_logs.push("Job starting...".to_string());
            all_logs.push(String::new());
            all_logs.push("No logs available yet. This usually means:".to_string());
            all_logs.push("• The job is still being prepared".to_string());
            all_logs.push("• CircleCI is spinning up the environment".to_string());
            all_logs.push(String::new());
            all_logs.push("Logs will appear here once the job starts running.".to_string());
            return Ok(all_logs);
        }

        let mut fetched_count = 0usize;

        for (step_idx, step) in steps.iter().enumerate() {
            // Add separator between steps (not before first step)
            if step_idx > 0 {
                all_logs.push(String::new());
            }

            for action in &step.actions {
                // Add action name as clean header (like terminal)
                all_logs.push(action.name.clone());

                // Fetch and add log output if available
                if let Some(output_url) = &action.output_url {
                    match self.fetch_log_output(output_url).await {
                        Ok(lines) => {
                            // Add raw log lines (no indentation)
                            for line in lines {
                                if !line.is_empty() {
                                    all_logs.push(line);
                                }
                            }
                        }
                        Err(e) => {
                            all_logs.push(format!("[Error fetching logs: {}]", e));
                        }
                    }

                    fetched_count += 1;
                    // Report progress
                    if let Some(tx) = &progress_tx {
                        let _ = tx.send((fetched_count, total_actions, action.name.clone()));
                    }
                } else if action.status == "running" {
                    all_logs.push("[Waiting for output...]".to_string());
                } else if action.status == "pending" {
                    all_logs.push("[Pending...]".to_string());
                }

                all_logs.push(String::new());
            }
        }
        Ok(all_logs)
    }

    /// Fetch job steps metadata, then stream per-step logs via channels.
    ///
    /// Sends:
    /// 1. `StepDiscovered` messages with step metadata (name, status) for each step
    /// 2. `StepLogsFetched` messages with logs for each step that has an output_url
    /// 3. Returns Ok(()) when complete
    ///
    /// This allows the UI to show the step list immediately while logs load progressively.
    pub async fn stream_job_steps_and_logs(
        &self,
        job_number: u32,
        step_tx: tokio::sync::mpsc::UnboundedSender<StepStreamMessage>,
    ) -> Result<(), ApiError> {
        let steps = self.get_job_steps(job_number).await?;

        // First, send all step metadata so UI can show the step list immediately
        let step_infos: Vec<(String, String)> = steps
            .iter()
            .map(|s| (s.name.clone(), s.status.clone()))
            .collect();
        let _ = step_tx.send(StepStreamMessage::StepsDiscovered(step_infos));

        // Check if any action has logs
        let has_any_logs = steps
            .iter()
            .flat_map(|s| &s.actions)
            .any(|a| a.output_url.is_some());

        if !has_any_logs {
            // No logs available yet - send empty logs for all steps
            for (idx, _) in steps.iter().enumerate() {
                let _ = step_tx.send(StepStreamMessage::StepLogsFetched {
                    step_index: idx,
                    logs: vec!["No logs available yet.".to_string()],
                });
            }
            return Ok(());
        }

        // Then fetch logs for each step's actions
        for (step_idx, step) in steps.iter().enumerate() {
            let mut step_logs = Vec::new();

            for action in &step.actions {
                if let Some(output_url) = &action.output_url {
                    match self.fetch_log_output(output_url).await {
                        Ok(lines) => {
                            // If step has multiple actions, add action name as header
                            if step.actions.len() > 1 {
                                step_logs.push(action.name.clone());
                            }
                            for line in lines {
                                if !line.is_empty() {
                                    step_logs.push(line);
                                }
                            }
                        }
                        Err(e) => {
                            step_logs.push(format!("[Error fetching logs: {}]", e));
                        }
                    }
                } else if action.status == "running" {
                    step_logs.push("[Waiting for output...]".to_string());
                } else if action.status == "pending" {
                    step_logs.push("[Pending...]".to_string());
                }
            }

            let _ = step_tx.send(StepStreamMessage::StepLogsFetched {
                step_index: step_idx,
                logs: step_logs,
            });
        }

        Ok(())
    }
}

impl Default for CircleCIClient {
    fn default() -> Self {
        // For default, create a client with empty values
        // This is mainly for testing purposes
        Self::new("".to_string(), "".to_string()).unwrap_or_else(|_| Self {
            client: reqwest::Client::new(),
            base_url: "https://circleci.com/api/v2".to_string(),
            project_slug: "".to_string(),
        })
    }
}
#[cfg(test)]
mod api_tests {
    //! HTTP-level tests against a mockito server.
    //! Moved here from tests/integration_test.rs and tests/workflow_test.rs: the client's
    //! `base_url` is private, so mock-server injection needs in-module access (the
    //! `new_with_base_url` test seam was removed in b7738e3).

    mod integration {
        use crate::api::client::CircleCIClient;
        use crate::api::error::ApiError;
        use mockito::{Server, ServerGuard};

        /// Helper function to create a mock CircleCI server
        async fn create_mock_server() -> ServerGuard {
            Server::new_async().await
        }

        /// Helper function to create a test client pointing at the mock server
        fn create_test_client(server: &ServerGuard) -> CircleCIClient {
            let mut client = CircleCIClient::new(
                "test-token".to_string(),
                "gh/test-org/test-repo".to_string(),
            )
            .expect("Failed to create test client");
            client.base_url = server.url();
            client
        }

        /// Helper to create a valid pipeline response JSON
        fn mock_pipeline_response() -> serde_json::Value {
            serde_json::json!({
                "items": [
                    {
                        "id": "pipe-001",
                        "number": 123,
                        "state": "success",
                        "created_at": "2024-01-15T10:30:00Z",
                        "updated_at": "2024-01-15T10:45:00Z",
                        "project_slug": "gh/test-org/test-repo",
                        "vcs": {
                            "branch": "main",
                            "revision": "abc123def456",
                            "commit": {
                                "subject": "feat: add new feature"
                            }
                        },
                        "trigger": {
                            "type": "webhook",
                            "actor": {
                                "login": "testuser"
                            }
                        }
                    },
                    {
                        "id": "pipe-002",
                        "number": 124,
                        "state": "running",
                        "created_at": "2024-01-15T11:00:00Z",
                        "updated_at": "2024-01-15T11:05:00Z",
                        "project_slug": "gh/test-org/test-repo",
                        "vcs": {
                            "branch": "feature-branch",
                            "revision": "def456ghi789",
                            "commit": {
                                "subject": "fix: bug fix"
                            }
                        },
                        "trigger": {
                            "type": "webhook",
                            "actor": {
                                "login": "developer"
                            }
                        }
                    }
                ],
                "next_page_token": null
            })
        }

        /// Helper to create a workflow response JSON
        fn mock_workflow_response() -> serde_json::Value {
            serde_json::json!({
                "items": [
                    {
                        "id": "wf-001",
                        "name": "build-and-test",
                        "status": "success",
                        "created_at": "2024-01-15T10:30:00Z",
                        "stopped_at": "2024-01-15T10:45:00Z",
                        "pipeline_id": "pipe-001"
                    },
                    {
                        "id": "wf-002",
                        "name": "deploy",
                        "status": "running",
                        "created_at": "2024-01-15T10:45:00Z",
                        "stopped_at": null,
                        "pipeline_id": "pipe-001"
                    }
                ],
                "next_page_token": null
            })
        }

        /// Helper to create a job response JSON
        fn mock_job_response() -> serde_json::Value {
            serde_json::json!({
                "items": [
                    {
                        "id": "job-001",
                        "name": "test",
                        "status": "success",
                        "job_number": 1001,
                        "started_at": "2024-01-15T10:30:00Z",
                        "stopped_at": "2024-01-15T10:35:00Z",
                        "type": "docker"
                    },
                    {
                        "id": "job-002",
                        "name": "build",
                        "status": "running",
                        "job_number": 1002,
                        "started_at": "2024-01-15T10:35:00Z",
                        "stopped_at": null,
                        "type": "machine"
                    }
                ],
                "next_page_token": null
            })
        }

        // ============================================================================
        // SUCCESS CASE TESTS
        // ============================================================================

        #[tokio::test]
        async fn test_get_pipelines_success() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let mock = server
                .mock("GET", "/project/gh/test-org/test-repo/pipeline")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(mock_pipeline_response().to_string())
                .create();

            let result = client.get_pipelines_filtered(10, None, false).await;
            mock.assert();

            assert!(result.is_ok());
            let pipelines = result.unwrap();
            assert_eq!(pipelines.len(), 2);
            assert_eq!(pipelines[0].id, "pipe-001");
            assert_eq!(pipelines[0].number, 123);
            assert_eq!(pipelines[0].state, "success");
            assert_eq!(pipelines[0].vcs.branch, "main");
            assert_eq!(pipelines[1].id, "pipe-002");
        }

        #[tokio::test]
        async fn test_get_workflows_success() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let mock = server
                .mock("GET", "/pipeline/pipe-001/workflow")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(mock_workflow_response().to_string())
                .create();

            let result = client.get_workflows("pipe-001").await;
            mock.assert();

            assert!(result.is_ok());
            let workflows = result.unwrap();
            assert_eq!(workflows.len(), 2);
            assert_eq!(workflows[0].id, "wf-001");
            assert_eq!(workflows[0].name, "build-and-test");
            assert_eq!(workflows[0].status, "success");
            assert_eq!(workflows[1].id, "wf-002");
        }

        #[tokio::test]
        async fn test_get_jobs_success() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let mock = server
                .mock("GET", "/workflow/wf-001/job")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(mock_job_response().to_string())
                .create();

            let result = client.get_jobs("wf-001").await;
            mock.assert();

            assert!(result.is_ok());
            let (jobs, next_page_token) = result.unwrap();
            assert_eq!(jobs.len(), 2);
            assert_eq!(jobs[0].id, "job-001");
            assert_eq!(jobs[0].name, "test");
            assert_eq!(jobs[0].status, "success");
            assert_eq!(jobs[0].job_number, 1001);
            assert!(next_page_token.is_none());
        }

        // ============================================================================
        // ERROR CASE TESTS - HTTP STATUS CODES
        // ============================================================================

        #[tokio::test]
        async fn test_get_pipelines_404_not_found() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let mock = server
                .mock("GET", "/project/gh/test-org/test-repo/pipeline")
                .with_status(404)
                .with_body("Project not found")
                .create();

            let result = client.get_pipelines_filtered(10, None, false).await;
            mock.assert();

            assert!(result.is_err());
            match result.unwrap_err() {
                ApiError::Http(status, msg) => {
                    assert_eq!(status, 404);
                    assert_eq!(msg, "Project not found");
                }
                _ => panic!("Expected Http error"),
            }
        }

        #[tokio::test]
        async fn test_get_workflows_403_forbidden() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let mock = server
                .mock("GET", "/pipeline/pipe-001/workflow")
                .with_status(403)
                .with_body("Forbidden")
                .create();

            let result = client.get_workflows("pipe-001").await;
            mock.assert();

            assert!(result.is_err());
            match result.unwrap_err() {
                ApiError::Http(status, msg) => {
                    assert_eq!(status, 403);
                    assert_eq!(msg, "Forbidden");
                }
                _ => panic!("Expected Http error"),
            }
        }

        #[tokio::test]
        async fn test_get_jobs_429_rate_limit() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let mock = server
                .mock("GET", "/workflow/wf-001/job")
                .with_status(429)
                .with_body("Rate limit exceeded")
                .create();

            let result = client.get_jobs("wf-001").await;
            mock.assert();

            assert!(result.is_err());
            match result.unwrap_err() {
                ApiError::Http(status, msg) => {
                    assert_eq!(status, 429);
                    assert_eq!(msg, "Rate limit exceeded");
                }
                _ => panic!("Expected Http error"),
            }
        }

        #[tokio::test]
        async fn test_get_pipelines_500_server_error() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let mock = server
                .mock("GET", "/project/gh/test-org/test-repo/pipeline")
                .with_status(500)
                .with_body("Internal server error")
                .create();

            let result = client.get_pipelines_filtered(10, None, false).await;
            mock.assert();

            assert!(result.is_err());
            match result.unwrap_err() {
                ApiError::Http(status, msg) => {
                    assert_eq!(status, 500);
                    assert_eq!(msg, "Internal server error");
                }
                _ => panic!("Expected Http error"),
            }
        }

        // ============================================================================
        // ERROR CASE TESTS - MALFORMED JSON
        // ============================================================================

        #[tokio::test]
        async fn test_get_pipelines_malformed_json() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let mock = server
                .mock("GET", "/project/gh/test-org/test-repo/pipeline")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body("{invalid json}")
                .create();

            let result = client.get_pipelines_filtered(10, None, false).await;
            mock.assert();

            assert!(result.is_err());
            match result.unwrap_err() {
                ApiError::Parse(msg) => {
                    assert!(msg.contains("Failed to parse pipelines"));
                }
                _ => panic!("Expected Parse error"),
            }
        }

        #[tokio::test]
        async fn test_get_workflows_malformed_json() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let mock = server
                .mock("GET", "/pipeline/pipe-001/workflow")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body("not json at all")
                .create();

            let result = client.get_workflows("pipe-001").await;
            mock.assert();

            assert!(result.is_err());
            match result.unwrap_err() {
                ApiError::Parse(msg) => {
                    assert!(msg.contains("Failed to parse workflows"));
                }
                _ => panic!("Expected Parse error"),
            }
        }

        #[tokio::test]
        async fn test_get_jobs_malformed_json() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let mock = server
                .mock("GET", "/workflow/wf-001/job")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body("[1, 2, 3]")
                .create();

            let result = client.get_jobs("wf-001").await;
            mock.assert();

            assert!(result.is_err());
            match result.unwrap_err() {
                ApiError::Parse(msg) => {
                    assert!(msg.contains("Failed to parse jobs"));
                }
                _ => panic!("Expected Parse error"),
            }
        }

        // ============================================================================
        // EDGE CASE TESTS
        // ============================================================================

        #[tokio::test]
        async fn test_get_pipelines_empty_response() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let mock = server
                .mock("GET", "/project/gh/test-org/test-repo/pipeline")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(r#"{"items": [], "next_page_token": null}"#)
                .create();

            let result = client.get_pipelines_filtered(10, None, false).await;
            mock.assert();

            assert!(result.is_ok());
            let pipelines = result.unwrap();
            assert_eq!(pipelines.len(), 0);
        }

        #[tokio::test]
        async fn test_get_workflows_empty_response() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let mock = server
                .mock("GET", "/pipeline/pipe-001/workflow")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(r#"{"items": [], "next_page_token": null}"#)
                .create();

            let result = client.get_workflows("pipe-001").await;
            mock.assert();

            assert!(result.is_ok());
            let workflows = result.unwrap();
            assert_eq!(workflows.len(), 0);
        }

        #[tokio::test]
        async fn test_get_jobs_empty_response() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let mock = server
                .mock("GET", "/workflow/wf-001/job")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(r#"{"items": [], "next_page_token": null}"#)
                .create();

            let result = client.get_jobs("wf-001").await;
            mock.assert();

            assert!(result.is_ok());
            let (jobs, _) = result.unwrap();
            assert_eq!(jobs.len(), 0);
        }

        #[tokio::test]
        async fn test_get_pipelines_with_pagination() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            // First page
            let first_page = serde_json::json!({
                "items": [
                    {
                        "id": "pipe-001",
                        "number": 123,
                        "state": "success",
                        "created_at": "2024-01-15T10:30:00Z",
                        "updated_at": "2024-01-15T10:45:00Z",
                        "project_slug": "gh/test-org/test-repo",
                        "vcs": {
                            "branch": "main",
                            "revision": "abc123",
                            "commit": {"subject": "Test"}
                        },
                        "trigger": {
                            "type": "webhook",
                            "actor": {"login": "user"}
                        }
                    }
                ],
                "next_page_token": "page2"
            });

            // Second page
            let second_page = serde_json::json!({
                "items": [
                    {
                        "id": "pipe-002",
                        "number": 124,
                        "state": "running",
                        "created_at": "2024-01-15T11:00:00Z",
                        "updated_at": "2024-01-15T11:05:00Z",
                        "project_slug": "gh/test-org/test-repo",
                        "vcs": {
                            "branch": "feature",
                            "revision": "def456",
                            "commit": {"subject": "Feature"}
                        },
                        "trigger": {
                            "type": "webhook",
                            "actor": {"login": "dev"}
                        }
                    }
                ],
                "next_page_token": null
            });

            let mock1 = server
                .mock("GET", "/project/gh/test-org/test-repo/pipeline")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(first_page.to_string())
                .create();

            let mock2 = server
                .mock("GET", "/project/gh/test-org/test-repo/pipeline")
                .match_query(mockito::Matcher::UrlEncoded(
                    "page-token".into(),
                    "page2".into(),
                ))
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(second_page.to_string())
                .create();

            let result = client.get_pipelines_filtered(10, None, false).await;
            mock1.assert();
            mock2.assert();

            assert!(result.is_ok());
            let pipelines = result.unwrap();
            assert_eq!(pipelines.len(), 2);
            assert_eq!(pipelines[0].id, "pipe-001");
            assert_eq!(pipelines[1].id, "pipe-002");
        }

        #[tokio::test]
        async fn test_get_jobs_with_pagination_token() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let response_with_token = serde_json::json!({
                "items": [
                    {
                        "id": "job-001",
                        "name": "test",
                        "status": "success",
                        "job_number": 1001,
                        "started_at": "2024-01-15T10:30:00Z",
                        "stopped_at": "2024-01-15T10:35:00Z",
                        "type": "docker"
                    }
                ],
                "next_page_token": "next-page-123"
            });

            let mock = server
                .mock("GET", "/workflow/wf-001/job")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(response_with_token.to_string())
                .create();

            let result = client.get_jobs("wf-001").await;
            mock.assert();

            assert!(result.is_ok());
            let (jobs, next_page_token) = result.unwrap();
            assert_eq!(jobs.len(), 1);
            assert_eq!(next_page_token, Some("next-page-123".to_string()));
        }

        #[tokio::test]
        async fn test_pipelines_with_missing_optional_fields() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let minimal_pipeline = serde_json::json!({
                "items": [
                    {
                        "id": "pipe-001",
                        "number": 123,
                        "state": "success",
                        "created_at": "2024-01-15T10:30:00Z",
                        "updated_at": "2024-01-15T10:45:00Z",
                        "project_slug": "gh/test-org/test-repo"
                        // vcs and trigger are optional
                    }
                ],
                "next_page_token": null
            });

            let mock = server
                .mock("GET", "/project/gh/test-org/test-repo/pipeline")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(minimal_pipeline.to_string())
                .create();

            let result = client.get_pipelines_filtered(10, None, false).await;
            mock.assert();

            assert!(result.is_ok());
            let pipelines = result.unwrap();
            assert_eq!(pipelines.len(), 1);
            assert_eq!(pipelines[0].vcs.branch, "unknown");
            assert_eq!(pipelines[0].vcs.commit_subject, "No commit message");
        }

        #[tokio::test]
        async fn test_status_mapping() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let response = serde_json::json!({
                "items": [
                    {
                        "id": "job-001",
                        "name": "created-job",
                        "status": "created",
                        "job_number": 1001,
                        "type": "docker"
                    },
                    {
                        "id": "job-002",
                        "name": "failing-job",
                        "status": "failing",
                        "job_number": 1002,
                        "type": "docker"
                    },
                    {
                        "id": "job-003",
                        "name": "on-hold-job",
                        "status": "on_hold",
                        "job_number": 1003,
                        "type": "docker"
                    }
                ],
                "next_page_token": null
            });

            let mock = server
                .mock("GET", "/workflow/wf-001/job")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(response.to_string())
                .create();

            let result = client.get_jobs("wf-001").await;
            mock.assert();

            assert!(result.is_ok());
            let (jobs, _) = result.unwrap();
            assert_eq!(jobs.len(), 3);
            // Jobs keep CircleCI's raw status; normalization happens at the UI layer
            // (see theme::get_status_color / api::models). map_status only maps pipeline state.
            assert_eq!(jobs[0].status, "created");
            assert_eq!(jobs[1].status, "failing");
            assert_eq!(jobs[2].status, "on_hold");
        }

        #[tokio::test]
        async fn test_jobs_duration_calculation() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let response = serde_json::json!({
                "items": [
                    {
                        "id": "job-001",
                        "name": "completed-job",
                        "status": "success",
                        "job_number": 1001,
                        "started_at": "2024-01-15T10:30:00Z",
                        "stopped_at": "2024-01-15T10:32:00Z", // 2 minutes = 120 seconds
                        "type": "docker"
                    },
                    {
                        "id": "job-002",
                        "name": "running-job",
                        "status": "running",
                        "job_number": 1002,
                        "started_at": "2024-01-15T10:30:00Z",
                        "stopped_at": null, // Still running
                        "type": "docker"
                    }
                ],
                "next_page_token": null
            });

            let mock = server
                .mock("GET", "/workflow/wf-001/job")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(response.to_string())
                .create();

            let result = client.get_jobs("wf-001").await;
            mock.assert();

            assert!(result.is_ok());
            let (jobs, _) = result.unwrap();
            assert_eq!(jobs.len(), 2);
            assert_eq!(jobs[0].duration, Some(120)); // 2 minutes
            assert_eq!(jobs[1].duration, None); // Still running
        }

        #[tokio::test]
        async fn test_pipelines_revision_shortening() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            let response = serde_json::json!({
                "items": [
                    {
                        "id": "pipe-001",
                        "number": 123,
                        "state": "success",
                        "created_at": "2024-01-15T10:30:00Z",
                        "updated_at": "2024-01-15T10:45:00Z",
                        "project_slug": "gh/test-org/test-repo",
                        "vcs": {
                            "branch": "main",
                            "revision": "abc123def456ghi789jkl012", // Long SHA
                            "commit": {"subject": "Test"}
                        },
                        "trigger": {
                            "type": "webhook",
                            "actor": {"login": "user"}
                        }
                    }
                ],
                "next_page_token": null
            });

            let mock = server
                .mock("GET", "/project/gh/test-org/test-repo/pipeline")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(response.to_string())
                .create();

            let result = client.get_pipelines_filtered(10, None, false).await;
            mock.assert();

            assert!(result.is_ok());
            let pipelines = result.unwrap();
            assert_eq!(pipelines.len(), 1);
            assert_eq!(pipelines[0].vcs.revision, "abc123d"); // First 7 characters
        }
    }

    mod workflow {
        use crate::api::client::CircleCIClient;
        use mockito::{Server, ServerGuard};

        /// Helper function to create a mock CircleCI server
        async fn create_mock_server() -> ServerGuard {
            Server::new_async().await
        }

        /// Helper function to create a test client pointing at the mock server
        fn create_test_client(server: &ServerGuard) -> CircleCIClient {
            let mut client = CircleCIClient::new(
                "test-token".to_string(),
                "gh/test-org/test-repo".to_string(),
            )
            .expect("Failed to create test client");
            client.base_url = server.url();
            client
        }

        /// End-to-end test: Complete pipeline → workflows → jobs flow
        ///
        /// This test simulates the full user journey:
        /// 1. Fetch pipelines
        /// 2. Select a pipeline and fetch its workflows
        /// 3. Select a workflow and fetch its jobs
        #[tokio::test]
        async fn test_complete_pipeline_to_jobs_flow() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            // Step 1: Mock GET pipelines
            let pipelines_response = serde_json::json!({
                "items": [
                    {
                        "id": "pipe-123",
                        "number": 1234,
                        "state": "success",
                        "created_at": "2024-01-15T10:00:00Z",
                        "updated_at": "2024-01-15T10:30:00Z",
                        "project_slug": "gh/test-org/test-repo",
                        "vcs": {
                            "branch": "main",
                            "revision": "abc123def456",
                            "commit": {
                                "subject": "feat: add new feature"
                            }
                        },
                        "trigger": {
                            "type": "webhook",
                            "actor": {
                                "login": "alice"
                            }
                        }
                    },
                    {
                        "id": "pipe-124",
                        "number": 1235,
                        "state": "running",
                        "created_at": "2024-01-15T11:00:00Z",
                        "updated_at": "2024-01-15T11:15:00Z",
                        "project_slug": "gh/test-org/test-repo",
                        "vcs": {
                            "branch": "feature-branch",
                            "revision": "def456ghi789",
                            "commit": {
                                "subject": "fix: bug fix"
                            }
                        },
                        "trigger": {
                            "type": "webhook",
                            "actor": {
                                "login": "bob"
                            }
                        }
                    }
                ],
                "next_page_token": null
            });

            let pipelines_mock = server
                .mock("GET", "/project/gh/test-org/test-repo/pipeline")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(pipelines_response.to_string())
                .create();

            // Step 2: Mock GET workflows for the first pipeline
            let workflows_response = serde_json::json!({
                "items": [
                    {
                        "id": "wf-001",
                        "name": "build-and-test",
                        "status": "success",
                        "created_at": "2024-01-15T10:00:00Z",
                        "stopped_at": "2024-01-15T10:20:00Z",
                        "pipeline_id": "pipe-123"
                    },
                    {
                        "id": "wf-002",
                        "name": "deploy-staging",
                        "status": "success",
                        "created_at": "2024-01-15T10:20:00Z",
                        "stopped_at": "2024-01-15T10:30:00Z",
                        "pipeline_id": "pipe-123"
                    }
                ],
                "next_page_token": null
            });

            let workflows_mock = server
                .mock("GET", "/pipeline/pipe-123/workflow")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(workflows_response.to_string())
                .create();

            // Step 3: Mock GET jobs for the first workflow
            let jobs_response = serde_json::json!({
                "items": [
                    {
                        "id": "job-001",
                        "name": "checkout",
                        "status": "success",
                        "job_number": 5001,
                        "started_at": "2024-01-15T10:00:00Z",
                        "stopped_at": "2024-01-15T10:01:00Z",
                        "type": "docker"
                    },
                    {
                        "id": "job-002",
                        "name": "test",
                        "status": "success",
                        "job_number": 5002,
                        "started_at": "2024-01-15T10:01:00Z",
                        "stopped_at": "2024-01-15T10:05:00Z",
                        "type": "docker"
                    },
                    {
                        "id": "job-003",
                        "name": "build",
                        "status": "success",
                        "job_number": 5003,
                        "started_at": "2024-01-15T10:05:00Z",
                        "stopped_at": "2024-01-15T10:15:00Z",
                        "type": "machine"
                    }
                ],
                "next_page_token": null
            });

            let jobs_mock = server
                .mock("GET", "/workflow/wf-001/job")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(jobs_response.to_string())
                .create();

            // Execute the full workflow
            // Step 1: Get pipelines
            let pipelines = client
                .get_pipelines_filtered(10, None, false)
                .await
                .expect("Failed to get pipelines");
            pipelines_mock.assert();

            assert_eq!(pipelines.len(), 2);
            assert_eq!(pipelines[0].id, "pipe-123");
            assert_eq!(pipelines[0].number, 1234);
            assert_eq!(pipelines[0].state, "success");
            assert_eq!(pipelines[0].vcs.branch, "main");
            assert_eq!(pipelines[0].vcs.revision, "abc123d"); // Shortened SHA

            // Step 2: Get workflows for the first pipeline
            let selected_pipeline = &pipelines[0];
            let workflows = client
                .get_workflows(&selected_pipeline.id)
                .await
                .expect("Failed to get workflows");
            workflows_mock.assert();

            assert_eq!(workflows.len(), 2);
            assert_eq!(workflows[0].id, "wf-001");
            assert_eq!(workflows[0].name, "build-and-test");
            assert_eq!(workflows[0].status, "success");
            assert_eq!(workflows[0].pipeline_id, "pipe-123");

            // Step 3: Get jobs for the first workflow
            let selected_workflow = &workflows[0];
            let (jobs, next_page_token) = client
                .get_jobs(&selected_workflow.id)
                .await
                .expect("Failed to get jobs");
            jobs_mock.assert();

            assert_eq!(jobs.len(), 3);
            assert!(next_page_token.is_none());

            // Verify job details
            assert_eq!(jobs[0].id, "job-001");
            assert_eq!(jobs[0].name, "checkout");
            assert_eq!(jobs[0].status, "success");
            assert_eq!(jobs[0].job_number, 5001);
            assert_eq!(jobs[0].duration, Some(60)); // 1 minute

            assert_eq!(jobs[1].id, "job-002");
            assert_eq!(jobs[1].name, "test");
            assert_eq!(jobs[1].duration, Some(240)); // 4 minutes

            assert_eq!(jobs[2].id, "job-003");
            assert_eq!(jobs[2].name, "build");
            assert_eq!(jobs[2].duration, Some(600)); // 10 minutes
            assert_eq!(jobs[2].executor.executor_type, "machine");
        }

        /// Test the complete flow with a failed pipeline
        ///
        /// This verifies that the system correctly handles failed pipelines
        /// and workflows, propagating status information through the chain.
        #[tokio::test]
        async fn test_complete_flow_with_failed_pipeline() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            // Mock a failed pipeline
            let pipelines_response = serde_json::json!({
                "items": [
                    {
                        "id": "pipe-failed",
                        "number": 2000,
                        "state": "failed",
                        "created_at": "2024-01-15T12:00:00Z",
                        "updated_at": "2024-01-15T12:15:00Z",
                        "project_slug": "gh/test-org/test-repo",
                        "vcs": {
                            "branch": "feature-broken",
                            "revision": "bad123commit",
                            "commit": {
                                "subject": "fix: attempted bug fix"
                            }
                        },
                        "trigger": {
                            "type": "webhook",
                            "actor": {
                                "login": "charlie"
                            }
                        }
                    }
                ],
                "next_page_token": null
            });

            let pipelines_mock = server
                .mock("GET", "/project/gh/test-org/test-repo/pipeline")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(pipelines_response.to_string())
                .create();

            // Mock workflow with mixed statuses
            let workflows_response = serde_json::json!({
                "items": [
                    {
                        "id": "wf-failed-001",
                        "name": "build-and-test",
                        "status": "failed",
                        "created_at": "2024-01-15T12:00:00Z",
                        "stopped_at": "2024-01-15T12:15:00Z",
                        "pipeline_id": "pipe-failed"
                    }
                ],
                "next_page_token": null
            });

            let workflows_mock = server
                .mock("GET", "/pipeline/pipe-failed/workflow")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(workflows_response.to_string())
                .create();

            // Mock jobs where one failed
            let jobs_response = serde_json::json!({
                "items": [
                    {
                        "id": "job-pass-001",
                        "name": "checkout",
                        "status": "success",
                        "job_number": 6001,
                        "started_at": "2024-01-15T12:00:00Z",
                        "stopped_at": "2024-01-15T12:01:00Z",
                        "type": "docker"
                    },
                    {
                        "id": "job-fail-002",
                        "name": "test",
                        "status": "failed",
                        "job_number": 6002,
                        "started_at": "2024-01-15T12:01:00Z",
                        "stopped_at": "2024-01-15T12:10:00Z",
                        "type": "docker"
                    },
                    {
                        "id": "job-canceled-003",
                        "name": "build",
                        "status": "canceled",
                        "job_number": 6003,
                        "started_at": null,
                        "stopped_at": null,
                        "type": "machine"
                    }
                ],
                "next_page_token": null
            });

            let jobs_mock = server
                .mock("GET", "/workflow/wf-failed-001/job")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(jobs_response.to_string())
                .create();

            // Execute flow
            let pipelines = client.get_pipelines_filtered(10, None, false).await.unwrap();
            pipelines_mock.assert();
            assert_eq!(pipelines[0].state, "failed");

            let workflows = client.get_workflows(&pipelines[0].id).await.unwrap();
            workflows_mock.assert();
            assert_eq!(workflows[0].status, "failed");

            let (jobs, _) = client.get_jobs(&workflows[0].id).await.unwrap();
            jobs_mock.assert();

            assert_eq!(jobs.len(), 3);
            assert_eq!(jobs[0].status, "success");
            assert_eq!(jobs[1].status, "failed");
            assert_eq!(jobs[2].status, "canceled"); // raw CircleCI status, mapped at display time
            assert_eq!(jobs[2].duration, None); // Job never ran
        }

        /// Test pagination across the entire flow
        ///
        /// This verifies that pagination works correctly when fetching
        /// multiple pages of pipelines and jobs.
        #[tokio::test]
        async fn test_complete_flow_with_pagination() {
            let mut server = create_mock_server().await;
            let client = create_test_client(&server);

            // First page of pipelines
            let page1_response = serde_json::json!({
                "items": [
                    {
                        "id": "pipe-001",
                        "number": 100,
                        "state": "success",
                        "created_at": "2024-01-15T10:00:00Z",
                        "updated_at": "2024-01-15T10:30:00Z",
                        "project_slug": "gh/test-org/test-repo",
                        "vcs": {
                            "branch": "main",
                            "revision": "aaa111",
                            "commit": {"subject": "First commit"}
                        },
                        "trigger": {
                            "type": "webhook",
                            "actor": {"login": "user1"}
                        }
                    }
                ],
                "next_page_token": "token-page2"
            });

            // Second page of pipelines
            let page2_response = serde_json::json!({
                "items": [
                    {
                        "id": "pipe-002",
                        "number": 101,
                        "state": "running",
                        "created_at": "2024-01-15T11:00:00Z",
                        "updated_at": "2024-01-15T11:30:00Z",
                        "project_slug": "gh/test-org/test-repo",
                        "vcs": {
                            "branch": "develop",
                            "revision": "bbb222",
                            "commit": {"subject": "Second commit"}
                        },
                        "trigger": {
                            "type": "webhook",
                            "actor": {"login": "user2"}
                        }
                    }
                ],
                "next_page_token": null
            });

            let page1_mock = server
                .mock("GET", "/project/gh/test-org/test-repo/pipeline")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(page1_response.to_string())
                .create();

            let page2_mock = server
                .mock("GET", "/project/gh/test-org/test-repo/pipeline")
                .match_query(mockito::Matcher::UrlEncoded(
                    "page-token".into(),
                    "token-page2".into(),
                ))
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(page2_response.to_string())
                .create();

            // Mock workflows for the first pipeline
            let workflows_response = serde_json::json!({
                "items": [
                    {
                        "id": "wf-001",
                        "name": "ci",
                        "status": "success",
                        "created_at": "2024-01-15T10:00:00Z",
                        "stopped_at": "2024-01-15T10:30:00Z",
                        "pipeline_id": "pipe-001"
                    }
                ],
                "next_page_token": null
            });

            let workflows_mock = server
                .mock("GET", "/pipeline/pipe-001/workflow")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(workflows_response.to_string())
                .create();

            // Mock jobs with pagination
            let jobs_page1 = serde_json::json!({
                "items": [
                    {
                        "id": "job-001",
                        "name": "test-1",
                        "status": "success",
                        "job_number": 1001,
                        "started_at": "2024-01-15T10:00:00Z",
                        "stopped_at": "2024-01-15T10:05:00Z",
                        "type": "docker"
                    }
                ],
                "next_page_token": "jobs-page2"
            });

            let jobs_page2 = serde_json::json!({
                "items": [
                    {
                        "id": "job-002",
                        "name": "test-2",
                        "status": "success",
                        "job_number": 1002,
                        "started_at": "2024-01-15T10:05:00Z",
                        "stopped_at": "2024-01-15T10:10:00Z",
                        "type": "docker"
                    }
                ],
                "next_page_token": null
            });

            let jobs_page1_mock = server
                .mock("GET", "/workflow/wf-001/job")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(jobs_page1.to_string())
                .create();

            let jobs_page2_mock = server
                .mock("GET", "/workflow/wf-001/job")
                .match_query(mockito::Matcher::UrlEncoded(
                    "page-token".into(),
                    "jobs-page2".into(),
                ))
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(jobs_page2.to_string())
                .create();

            // Execute flow
            let pipelines = client.get_pipelines_filtered(10, None, false).await.unwrap();
            page1_mock.assert();
            page2_mock.assert();
            assert_eq!(pipelines.len(), 2);

            let workflows = client.get_workflows(&pipelines[0].id).await.unwrap();
            workflows_mock.assert();
            assert_eq!(workflows.len(), 1);

            // Get first page of jobs
            let (jobs_page1, token1) = client.get_jobs(&workflows[0].id).await.unwrap();
            jobs_page1_mock.assert();
            assert_eq!(jobs_page1.len(), 1);
            assert_eq!(token1, Some("jobs-page2".to_string()));

            // Get second page of jobs
            let (jobs_page2, token2) = client
                .get_jobs_page(&workflows[0].id, Some("jobs-page2"))
                .await
                .unwrap();
            jobs_page2_mock.assert();
            assert_eq!(jobs_page2.len(), 1);
            assert!(token2.is_none());
        }
    }
}

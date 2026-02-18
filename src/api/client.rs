use super::error::ApiError;
use super::models::{ExecutorInfo, Job, Pipeline, TriggerInfo, VcsInfo, Workflow};
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT};
use serde::Deserialize;
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

// API Response structures for deserialization

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
    body: Option<String>,
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
    avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WorkflowResponse {
    items: Vec<WorkflowItem>,
    next_page_token: Option<String>,
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

impl CircleCIClient {
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

    /// Fetch pipelines for the project
    ///
    /// # Arguments
    /// * `limit` - Maximum number of pipelines to fetch (default: 50)
    ///
    /// # Returns
    /// List of pipelines, most recent first
    pub async fn get_pipelines(&self, limit: usize) -> Result<Vec<Pipeline>, ApiError> {
        let mut all_pipelines = Vec::new();
        let mut next_page_token: Option<String> = None;

        // Keep fetching pages until we have enough pipelines
        while all_pipelines.len() < limit {
            let mut url = format!("{}/project/{}/pipeline", self.base_url, self.project_slug);

            // Add page token if we have one
            if let Some(token) = &next_page_token {
                url = format!("{}?page-token={}", url, token);
            }

            // Make API request
            let response = self
                .client
                .get(&url)
                .send()
                .await?;

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
                status: map_status(&item.status),
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
    /// List of jobs for the workflow
    pub async fn get_jobs(&self, workflow_id: &str) -> Result<Vec<Job>, ApiError> {
        let url = format!("{}/workflow/{}/job", self.base_url, workflow_id);

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
                let duration = if let (Some(started), Some(stopped)) =
                    (item.started_at, item.stopped_at)
                {
                    let delta = stopped.signed_duration_since(started);
                    Some(delta.num_seconds() as u32)
                } else {
                    None
                };

                Job {
                    id: item.id,
                    name: item.name,
                    status: map_status(&item.status),
                    job_number: item.job_number,
                    workflow_id: workflow_id.to_string(),
                    started_at: item.started_at,
                    stopped_at: item.stopped_at,
                    duration,
                    executor: ExecutorInfo {
                        executor_type: item
                            .executor_type
                            .unwrap_or_else(|| "unknown".to_string()),
                    },
                }
            })
            .collect();

        Ok(jobs)
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

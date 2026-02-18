use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub number: u32,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub vcs: VcsInfo,
    pub trigger: TriggerInfo,
    pub project_slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcsInfo {
    pub branch: String,
    pub revision: String,
    pub commit_subject: String,
    pub commit_author_name: String,
    pub commit_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerInfo {
    #[serde(rename = "type")]
    pub trigger_type: String, // webhook, scheduled, api
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub pipeline_id: String,
}

impl Workflow {
    pub fn duration_formatted(&self) -> String {
        if let Some(stopped) = self.stopped_at {
            let duration = stopped.signed_duration_since(self.created_at);
            let secs = duration.num_seconds();
            if secs < 60 {
                format!("{}s", secs)
            } else if secs < 3600 {
                format!("{}m {}s", secs / 60, secs % 60)
            } else {
                format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
            }
        } else {
            "running...".to_string()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub status: String,
    pub job_number: u32,
    pub workflow_id: String, // job belongs to workflow
    pub started_at: Option<DateTime<Utc>>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub duration: Option<u32>, // in seconds
    pub executor: ExecutorInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutorInfo {
    #[serde(rename = "type")]
    pub executor_type: String, // docker, machine
}

impl Job {
    pub fn duration_formatted(&self) -> String {
        if let Some(secs) = self.duration {
            if secs < 60 {
                format!("{}s", secs)
            } else if secs < 3600 {
                format!("{}m {}s", secs / 60, secs % 60)
            } else {
                format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
            }
        } else if self.started_at.is_some() {
            "running...".to_string()
        } else {
            "pending".to_string()
        }
    }

    /// Check if this job is currently running
    pub fn is_running(&self) -> bool {
        self.status == "running" && self.stopped_at.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStep {
    pub name: String,
    pub status: String,
    pub actions: Vec<StepAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepAction {
    pub name: String,
    pub status: String,
    pub output_url: Option<String>,
    pub index: usize,
    #[serde(skip)]
    pub log_output: Vec<String>,
}

pub mod mock_data {
    use super::*;
    use chrono::Duration;

    pub fn mock_pipelines() -> Vec<Pipeline> {
        let now = Utc::now();

        vec![
            Pipeline {
                id: "pipe-001".to_string(),
                number: 1234,
                state: "success".to_string(),
                created_at: now - Duration::hours(2),
                updated_at: now - Duration::hours(1) - Duration::minutes(45),
                vcs: VcsInfo {
                    branch: "main".to_string(),
                    revision: "a1b2c3d".to_string(),
                    commit_subject: "feat: add webhook retry logic".to_string(),
                    commit_author_name: "alice".to_string(),
                    commit_timestamp: now - Duration::hours(2) - Duration::minutes(5),
                },
                trigger: TriggerInfo {
                    trigger_type: "webhook".to_string(),
                },
                project_slug: "gh/acme/api-service".to_string(),
            },
            Pipeline {
                id: "pipe-002".to_string(),
                number: 1235,
                state: "running".to_string(),
                created_at: now - Duration::minutes(45),
                updated_at: now - Duration::minutes(5),
                vcs: VcsInfo {
                    branch: "feat/oauth".to_string(),
                    revision: "e4f5g6h".to_string(),
                    commit_subject: "fix: rate limiter edge case".to_string(),
                    commit_author_name: "bob".to_string(),
                    commit_timestamp: now - Duration::minutes(50),
                },
                trigger: TriggerInfo {
                    trigger_type: "webhook".to_string(),
                },
                project_slug: "gh/acme/api-service".to_string(),
            },
            Pipeline {
                id: "pipe-003".to_string(),
                number: 1236,
                state: "failed".to_string(),
                created_at: now - Duration::hours(4),
                updated_at: now - Duration::hours(3) - Duration::minutes(50),
                vcs: VcsInfo {
                    branch: "fix/memory-leak".to_string(),
                    revision: "i7j8k9l".to_string(),
                    commit_subject: "fix: memory leak in connection pool".to_string(),
                    commit_author_name: "charlie".to_string(),
                    commit_timestamp: now - Duration::hours(4) - Duration::minutes(10),
                },
                trigger: TriggerInfo {
                    trigger_type: "webhook".to_string(),
                },
                project_slug: "gh/acme/api-service".to_string(),
            },
            Pipeline {
                id: "pipe-004".to_string(),
                number: 1237,
                state: "success".to_string(),
                created_at: now - Duration::hours(6),
                updated_at: now - Duration::hours(5) - Duration::minutes(52),
                vcs: VcsInfo {
                    branch: "main".to_string(),
                    revision: "m0n1o2p".to_string(),
                    commit_subject: "chore: bump deps".to_string(),
                    commit_author_name: "dave".to_string(),
                    commit_timestamp: now - Duration::hours(6) - Duration::minutes(2),
                },
                trigger: TriggerInfo {
                    trigger_type: "scheduled".to_string(),
                },
                project_slug: "gh/acme/api-service".to_string(),
            },
            Pipeline {
                id: "pipe-005".to_string(),
                number: 1238,
                state: "pending".to_string(),
                created_at: now - Duration::minutes(2),
                updated_at: now - Duration::minutes(2),
                vcs: VcsInfo {
                    branch: "feat/metrics".to_string(),
                    revision: "q3r4s5t".to_string(),
                    commit_subject: "feat: add prometheus metrics".to_string(),
                    commit_author_name: "alice".to_string(),
                    commit_timestamp: now - Duration::minutes(5),
                },
                trigger: TriggerInfo {
                    trigger_type: "api".to_string(),
                },
                project_slug: "gh/acme/api-service".to_string(),
            },
            Pipeline {
                id: "pipe-006".to_string(),
                number: 1239,
                state: "success".to_string(),
                created_at: now - Duration::hours(8),
                updated_at: now - Duration::hours(7) - Duration::minutes(54),
                vcs: VcsInfo {
                    branch: "main".to_string(),
                    revision: "u6v7w8x".to_string(),
                    commit_subject: "docs: update API documentation".to_string(),
                    commit_author_name: "bob".to_string(),
                    commit_timestamp: now - Duration::hours(8) - Duration::minutes(3),
                },
                trigger: TriggerInfo {
                    trigger_type: "webhook".to_string(),
                },
                project_slug: "gh/acme/api-service".to_string(),
            },
            Pipeline {
                id: "pipe-007".to_string(),
                number: 1240,
                state: "failed".to_string(),
                created_at: now - Duration::hours(10),
                updated_at: now - Duration::hours(9) - Duration::minutes(48),
                vcs: VcsInfo {
                    branch: "fix/database-conn".to_string(),
                    revision: "y9z0a1b".to_string(),
                    commit_subject: "fix: database connection timeout".to_string(),
                    commit_author_name: "charlie".to_string(),
                    commit_timestamp: now - Duration::hours(10) - Duration::minutes(8),
                },
                trigger: TriggerInfo {
                    trigger_type: "webhook".to_string(),
                },
                project_slug: "gh/acme/api-service".to_string(),
            },
            Pipeline {
                id: "pipe-008".to_string(),
                number: 1241,
                state: "success".to_string(),
                created_at: now - Duration::hours(12),
                updated_at: now - Duration::hours(11) - Duration::minutes(56),
                vcs: VcsInfo {
                    branch: "main".to_string(),
                    revision: "c2d3e4f".to_string(),
                    commit_subject: "perf: optimize query performance".to_string(),
                    commit_author_name: "dave".to_string(),
                    commit_timestamp: now - Duration::hours(12) - Duration::minutes(4),
                },
                trigger: TriggerInfo {
                    trigger_type: "webhook".to_string(),
                },
                project_slug: "gh/acme/api-service".to_string(),
            },
        ]
    }

    pub fn mock_workflows(pipeline_id: &str) -> Vec<Workflow> {
        let now = Utc::now();

        vec![
            Workflow {
                id: format!("wf-{}-001", pipeline_id),
                name: "build-and-test".to_string(),
                status: if pipeline_id.contains("002") || pipeline_id.contains("005") {
                    "running".to_string()
                } else if pipeline_id.contains("003") || pipeline_id.contains("007") {
                    "failed".to_string()
                } else {
                    "success".to_string()
                },
                created_at: now - Duration::minutes(30),
                stopped_at: if pipeline_id.contains("002") || pipeline_id.contains("005") {
                    None
                } else {
                    Some(now - Duration::minutes(25))
                },
                pipeline_id: pipeline_id.to_string(),
            },
            Workflow {
                id: format!("wf-{}-002", pipeline_id),
                name: "integration-tests".to_string(),
                status: if pipeline_id.contains("002") {
                    "blocked".to_string()
                } else if pipeline_id.contains("005") {
                    "pending".to_string()
                } else if pipeline_id.contains("003") || pipeline_id.contains("007") {
                    "success".to_string()
                } else {
                    "success".to_string()
                },
                created_at: now - Duration::minutes(25),
                stopped_at: if pipeline_id.contains("002") || pipeline_id.contains("005") {
                    None
                } else {
                    Some(now - Duration::minutes(18))
                },
                pipeline_id: pipeline_id.to_string(),
            },
            Workflow {
                id: format!("wf-{}-003", pipeline_id),
                name: "deploy-staging".to_string(),
                status: if pipeline_id.contains("002") || pipeline_id.contains("005") {
                    "pending".to_string()
                } else if pipeline_id.contains("003") || pipeline_id.contains("007") {
                    "canceled".to_string()
                } else {
                    "success".to_string()
                },
                created_at: now - Duration::minutes(18),
                stopped_at: if pipeline_id.contains("002") || pipeline_id.contains("005") {
                    None
                } else if pipeline_id.contains("003") || pipeline_id.contains("007") {
                    Some(now - Duration::minutes(18))
                } else {
                    Some(now - Duration::minutes(12))
                },
                pipeline_id: pipeline_id.to_string(),
            },
        ]
    }

    pub fn mock_jobs(workflow_id: &str) -> Vec<Job> {
        let now = Utc::now();
        let is_running = workflow_id.contains("002");
        let is_failed = workflow_id.contains("003") || workflow_id.contains("007");

        vec![
            Job {
                id: format!("job-{}-001", workflow_id),
                name: "checkout".to_string(),
                status: "success".to_string(),
                job_number: 1,
                workflow_id: workflow_id.to_string(),
                started_at: Some(now - Duration::minutes(28)),
                stopped_at: Some(now - Duration::minutes(27) - Duration::seconds(45)),
                duration: Some(15),
                executor: ExecutorInfo {
                    executor_type: "docker".to_string(),
                },
            },
            Job {
                id: format!("job-{}-002", workflow_id),
                name: "install-deps".to_string(),
                status: "success".to_string(),
                job_number: 2,
                workflow_id: workflow_id.to_string(),
                started_at: Some(now - Duration::minutes(27) - Duration::seconds(45)),
                stopped_at: Some(now - Duration::minutes(26) - Duration::seconds(15)),
                duration: Some(90),
                executor: ExecutorInfo {
                    executor_type: "docker".to_string(),
                },
            },
            Job {
                id: format!("job-{}-003", workflow_id),
                name: "lint".to_string(),
                status: "success".to_string(),
                job_number: 3,
                workflow_id: workflow_id.to_string(),
                started_at: Some(now - Duration::minutes(26) - Duration::seconds(15)),
                stopped_at: Some(now - Duration::minutes(25) - Duration::seconds(45)),
                duration: Some(30),
                executor: ExecutorInfo {
                    executor_type: "docker".to_string(),
                },
            },
            Job {
                id: format!("job-{}-004", workflow_id),
                name: "unit-tests".to_string(),
                status: if is_running {
                    "running".to_string()
                } else if is_failed {
                    "failed".to_string()
                } else {
                    "success".to_string()
                },
                job_number: 4,
                workflow_id: workflow_id.to_string(),
                started_at: Some(now - Duration::minutes(25) - Duration::seconds(45)),
                stopped_at: if is_running {
                    None
                } else {
                    Some(now - Duration::minutes(23) - Duration::seconds(30))
                },
                duration: if is_running { None } else { Some(135) },
                executor: ExecutorInfo {
                    executor_type: "docker".to_string(),
                },
            },
            Job {
                id: format!("job-{}-005", workflow_id),
                name: "build".to_string(),
                status: if is_running {
                    "blocked".to_string()
                } else if is_failed {
                    "canceled".to_string()
                } else {
                    "success".to_string()
                },
                job_number: 5,
                workflow_id: workflow_id.to_string(),
                started_at: if is_running || is_failed {
                    None
                } else {
                    Some(now - Duration::minutes(23) - Duration::seconds(30))
                },
                stopped_at: if is_running || is_failed {
                    None
                } else {
                    Some(now - Duration::minutes(21))
                },
                duration: if is_running || is_failed {
                    None
                } else {
                    Some(150)
                },
                executor: ExecutorInfo {
                    executor_type: "machine".to_string(),
                },
            },
            Job {
                id: format!("job-{}-006", workflow_id),
                name: "e2e-tests".to_string(),
                status: if is_running || is_failed {
                    "pending".to_string()
                } else {
                    "success".to_string()
                },
                job_number: 6,
                workflow_id: workflow_id.to_string(),
                started_at: if is_running || is_failed {
                    None
                } else {
                    Some(now - Duration::minutes(21))
                },
                stopped_at: if is_running || is_failed {
                    None
                } else {
                    Some(now - Duration::minutes(18))
                },
                duration: if is_running || is_failed {
                    None
                } else {
                    Some(180)
                },
                executor: ExecutorInfo {
                    executor_type: "machine".to_string(),
                },
            },
            Job {
                id: format!("job-{}-007", workflow_id),
                name: "security-scan".to_string(),
                status: if is_running || is_failed {
                    "pending".to_string()
                } else {
                    "success".to_string()
                },
                job_number: 7,
                workflow_id: workflow_id.to_string(),
                started_at: if is_running || is_failed {
                    None
                } else {
                    Some(now - Duration::minutes(18))
                },
                stopped_at: if is_running || is_failed {
                    None
                } else {
                    Some(now - Duration::minutes(16) - Duration::seconds(30))
                },
                duration: if is_running || is_failed {
                    None
                } else {
                    Some(90)
                },
                executor: ExecutorInfo {
                    executor_type: "docker".to_string(),
                },
            },
            Job {
                id: format!("job-{}-008", workflow_id),
                name: "upload-artifacts".to_string(),
                status: if is_running || is_failed {
                    "pending".to_string()
                } else {
                    "success".to_string()
                },
                job_number: 8,
                workflow_id: workflow_id.to_string(),
                started_at: if is_running || is_failed {
                    None
                } else {
                    Some(now - Duration::minutes(16) - Duration::seconds(30))
                },
                stopped_at: if is_running || is_failed {
                    None
                } else {
                    Some(now - Duration::minutes(15))
                },
                duration: if is_running || is_failed {
                    None
                } else {
                    Some(90)
                },
                executor: ExecutorInfo {
                    executor_type: "docker".to_string(),
                },
            },
        ]
    }
}

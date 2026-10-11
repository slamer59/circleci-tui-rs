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

impl Pipeline {
    /// Calculate pipeline duration from workflow execution times
    /// Returns the span from earliest workflow start to latest workflow completion
    pub fn calculate_duration_from_workflows(&self, workflows: Option<&Vec<Workflow>>) -> String {
        match workflows {
            None => {
                // No workflows loaded yet
                "--".to_string()
            }
            Some(wfs) if wfs.is_empty() => {
                // Pipeline has no workflows
                "--".to_string()
            }
            Some(wfs) => {
                let has_running = wfs.iter().any(|w| w.status == "running");
                let all_inactive = wfs.iter().all(|w| {
                    w.status == "not_run"
                        || w.status == "canceled"
                        || w.status == "cancelled"
                        || w.status == "on_hold"
                });

                if all_inactive {
                    return "--".to_string();
                }

                // Find earliest created_at (when first workflow started)
                let earliest_start = wfs.iter().map(|w| w.created_at).min();

                // Find latest stopped_at (when last workflow completed)
                let latest_stop = wfs.iter().filter_map(|w| w.stopped_at).max();

                match (earliest_start, latest_stop) {
                    (Some(start), Some(stop)) => {
                        let duration = stop.signed_duration_since(start);
                        let secs = duration.num_seconds().max(0);

                        if secs < 60 {
                            format!("{}s", secs)
                        } else if secs < 3600 {
                            let mins = secs / 60;
                            let remaining_secs = secs % 60;
                            if remaining_secs > 0 {
                                format!("{}m {}s", mins, remaining_secs)
                            } else {
                                format!("{}m", mins)
                            }
                        } else {
                            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
                        }
                    }
                    (Some(_start), None) if has_running => "...".to_string(),
                    _ => "--".to_string(),
                }
            }
        }
    }
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

    /// Sample workflows for a pipeline, used by `--demo`.
    ///
    /// Ids are derived from the pipeline so the demo stays self-consistent.
    pub fn mock_workflows(pipeline_id: &str) -> Vec<Workflow> {
        let now = Utc::now();
        let wf = |suffix: &str| format!("{}-{}", pipeline_id, suffix);

        vec![
            Workflow {
                id: wf("wf-build"),
                name: "build-and-test".to_string(),
                status: "success".to_string(),
                created_at: now - Duration::hours(2),
                stopped_at: Some(now - Duration::hours(1) - Duration::minutes(46)),
                pipeline_id: pipeline_id.to_string(),
            },
            Workflow {
                id: wf("wf-staging"),
                name: "deploy-staging".to_string(),
                status: "success".to_string(),
                created_at: now - Duration::hours(1) - Duration::minutes(45),
                stopped_at: Some(now - Duration::hours(1) - Duration::minutes(31)),
                pipeline_id: pipeline_id.to_string(),
            },
            Workflow {
                id: wf("wf-production"),
                name: "deploy-production".to_string(),
                status: "failed".to_string(),
                created_at: now - Duration::hours(1) - Duration::minutes(30),
                stopped_at: Some(now - Duration::hours(1) - Duration::minutes(12)),
                pipeline_id: pipeline_id.to_string(),
            },
        ]
    }

    /// Sample jobs for a workflow, used by `--demo`.
    pub fn mock_jobs(workflow_id: &str) -> Vec<Job> {
        let now = Utc::now();
        let job = |number: u32, name: &str, status: &str, started_mins: i64, secs: u32| Job {
            id: format!("{}-job-{}", workflow_id, number),
            name: name.to_string(),
            status: status.to_string(),
            job_number: number,
            workflow_id: workflow_id.to_string(),
            started_at: Some(now - Duration::minutes(started_mins)),
            stopped_at: Some(now - Duration::minutes(started_mins) + Duration::seconds(secs as i64)),
            duration: Some(secs),
            executor: ExecutorInfo {
                executor_type: "docker".to_string(),
            },
        };

        // Numbers are stable per workflow so the list reads like a real run.
        let base = workflow_id.bytes().map(|b| b as u32).sum::<u32>() % 900 + 100;
        let on_production = workflow_id.ends_with("wf-production");
        let on_staging = workflow_id.ends_with("wf-staging");

        if on_staging {
            vec![
                job(base, "publish-image", "success", 105, 96),
                job(base + 1, "deploy-staging", "success", 98, 42),
                job(base + 2, "smoke-tests", "success", 95, 28),
            ]
        } else if on_production {
            vec![
                job(base, "approve-release", "success", 90, 15),
                job(base + 1, "deploy-production", "failed", 88, 37),
                job(base + 2, "verify-release", "canceled", 87, 0),
            ]
        } else {
            vec![
                job(base, "checkout", "success", 120, 8),
                job(base + 1, "lint", "success", 119, 24),
                job(base + 2, "unit-tests", "success", 118, 133),
                job(base + 3, "integration-tests", "success", 115, 261),
                job(base + 4, "build-image", "success", 110, 78),
            ]
        }
    }
}

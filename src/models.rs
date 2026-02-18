/// Data models for CircleCI entities
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub branch: String,
    pub commit_msg: String,
    pub author: String,
    pub status: String,
    pub sha: String,
    pub trigger: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub status: String,
    pub duration: String,
    pub jobs_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub status: String,
    pub duration: String,
    pub executor: String,
    pub step: String,
    #[serde(default)]
    pub job_number: Option<i32>,
    #[serde(default)]
    pub ssh_enabled: bool,
}

use super::error::ApiError;
use super::models::{Job, Pipeline, Workflow};

pub struct CircleCIClient {
    // Fields will be added in later phases
    // For now, this is just a stub
}

impl CircleCIClient {
    pub fn new() -> Self {
        CircleCIClient {}
    }

    pub async fn get_pipelines(&self) -> Result<Vec<Pipeline>, ApiError> {
        // For Phase 2, return mock data
        Ok(super::models::mock_data::mock_pipelines())
    }

    pub async fn get_workflows(&self, pipeline_id: &str) -> Result<Vec<Workflow>, ApiError> {
        // For Phase 2, return mock data
        Ok(super::models::mock_data::mock_workflows(pipeline_id))
    }

    pub async fn get_jobs(&self, workflow_id: &str) -> Result<Vec<Job>, ApiError> {
        // For Phase 2, return mock data
        Ok(super::models::mock_data::mock_jobs(workflow_id))
    }
}

impl Default for CircleCIClient {
    fn default() -> Self {
        Self::new()
    }
}

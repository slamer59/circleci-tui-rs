use crate::api::client::CircleCIClient;
use crate::api::error::ApiError;
use crate::api::models::Job;
use crate::cache::log_cache::{CacheStatus, LogCacheManager};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

/// Result from a prefetch operation
pub struct PrefetchResult {
    pub job_number: u32,
    pub result: Result<Vec<String>, ApiError>,
}

/// Coordinates parallel prefetch operations for job logs
pub struct PrefetchCoordinator {
    active_tasks: HashMap<u32, JoinHandle<()>>,
    result_rx: UnboundedReceiver<PrefetchResult>,
    result_tx: UnboundedSender<PrefetchResult>,
    cache_manager: Arc<LogCacheManager>,
    api_client: Arc<CircleCIClient>,
}

impl PrefetchCoordinator {
    /// Create a new prefetch coordinator
    pub fn new(cache_manager: Arc<LogCacheManager>, api_client: Arc<CircleCIClient>) -> Self {
        let (result_tx, result_rx) = unbounded_channel();

        Self {
            active_tasks: HashMap::new(),
            result_rx,
            result_tx,
            cache_manager,
            api_client,
        }
    }

    /// Prefetch logs for a list of jobs
    pub fn prefetch_jobs(&mut self, job_numbers: Vec<u32>, jobs: Vec<Job>) {
        for job_number in job_numbers {
            // Skip if already prefetching
            if self.active_tasks.contains_key(&job_number) {
                continue;
            }

            // Find job details
            let job = jobs.iter().find(|j| j.job_number == job_number);

            let job_status = job
                .map(|j| j.status.clone())
                .unwrap_or_else(|| "unknown".to_string());

            // Spawn prefetch task
            let cache_manager = Arc::clone(&self.cache_manager);
            let api_client = Arc::clone(&self.api_client);
            let result_tx = self.result_tx.clone();

            let handle = tokio::spawn(async move {
                prefetch_worker(job_number, job_status, cache_manager, api_client, result_tx).await;
            });

            self.active_tasks.insert(job_number, handle);
        }
    }

    /// Cancel prefetch for jobs no longer in view
    pub fn cancel_jobs(&mut self, job_numbers: Vec<u32>) {
        for job_number in job_numbers {
            if let Some(handle) = self.active_tasks.remove(&job_number) {
                handle.abort();
            }
        }
    }

    /// Poll for completed prefetch results (non-blocking)
    pub fn poll_results(&mut self) -> Vec<PrefetchResult> {
        let mut results = Vec::new();

        while let Ok(result) = self.result_rx.try_recv() {
            // Remove from active tasks
            self.active_tasks.remove(&result.job_number);
            results.push(result);
        }

        results
    }
}

/// Worker function that fetches logs for a single job
async fn prefetch_worker(
    job_number: u32,
    job_status: String,
    cache_manager: Arc<LogCacheManager>,
    api_client: Arc<CircleCIClient>,
    result_tx: UnboundedSender<PrefetchResult>,
) {
    // Skip if job not started yet
    if job_status == "pending" || job_status == "blocked" {
        return;
    }

    // Check cache (skip for running jobs)
    if job_status != "running" {
        if let Ok(CacheStatus::Valid(entry)) = cache_manager.get(job_number) {
            let _ = result_tx.send(PrefetchResult {
                job_number,
                result: Ok(entry.logs),
            });
            return;
        }
    }

    // Fetch from API
    let result = api_client.stream_job_log(job_number).await;

    // Cache if successful and not running
    if let Ok(ref logs) = result {
        if job_status != "running" {
            let _ = cache_manager.put(job_number, logs.clone(), job_status.clone());
        }
    }

    let _ = result_tx.send(PrefetchResult { job_number, result });
}

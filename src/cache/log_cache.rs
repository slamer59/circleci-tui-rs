use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone)]
pub struct LogCacheManager {
    cache_dir: PathBuf,
}

pub struct CacheEntry {
    pub logs: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CacheMetadata {
    pub cached_at: DateTime<Utc>,
    pub job_status: String,
    pub log_hash: String,
}

pub enum CacheStatus {
    Valid(CacheEntry),
    Stale,
    Missing,
}

impl LogCacheManager {
    /// Create a new LogCacheManager with cache directory at ~/.cache/circleci-tui/logs/
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .context("Failed to determine cache directory")?
            .join("circleci-tui")
            .join("logs");

        // Create cache directory if it doesn't exist
        fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;

        Ok(Self { cache_dir })
    }

    /// Get cached logs for a job if valid
    pub fn get(&self, job_number: u32) -> Result<CacheStatus> {
        let log_path = self.cache_dir.join(format!("{}.log", job_number));
        let meta_path = self.cache_dir.join(format!("{}.meta", job_number));

        // Check if both files exist
        if !log_path.exists() || !meta_path.exists() {
            return Ok(CacheStatus::Missing);
        }

        // Read metadata
        let meta_content =
            fs::read_to_string(&meta_path).context("Failed to read metadata file")?;
        let metadata: CacheMetadata =
            serde_json::from_str(&meta_content).context("Failed to parse metadata")?;

        // Check if cache is still valid
        if !self.is_cache_valid(&metadata) {
            return Ok(CacheStatus::Stale);
        }

        // Read log file
        let log_content = fs::read_to_string(&log_path).context("Failed to read log file")?;
        let logs: Vec<String> = log_content.lines().map(|s| s.to_string()).collect();

        Ok(CacheStatus::Valid(CacheEntry { logs }))
    }

    /// Store logs to disk cache
    pub fn put(&self, job_number: u32, logs: Vec<String>, job_status: String) -> Result<()> {
        let log_path = self.cache_dir.join(format!("{}.log", job_number));
        let meta_path = self.cache_dir.join(format!("{}.meta", job_number));

        // Calculate simple hash (just content length for now)
        let log_hash = logs.len().to_string();

        // Write log file
        let log_content = logs.join("\n");
        fs::write(&log_path, log_content).context("Failed to write log file")?;

        // Write metadata file
        let metadata = CacheMetadata {
            cached_at: Utc::now(),
            job_status: job_status.clone(),
            log_hash,
        };
        let meta_content =
            serde_json::to_string_pretty(&metadata).context("Failed to serialize metadata")?;
        fs::write(&meta_path, meta_content).context("Failed to write metadata file")?;

        Ok(())
    }

    /// Clean up cache entries older than 15 days
    pub fn cleanup_old_entries(&self) -> Result<()> {
        let now = Utc::now();
        let max_age_days = 15;

        let entries = fs::read_dir(&self.cache_dir).context("Failed to read cache directory")?;

        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            // Only check .meta files
            if path.extension().and_then(|s| s.to_str()) != Some("meta") {
                continue;
            }

            // Read metadata to check age
            if let Ok(meta_content) = fs::read_to_string(&path) {
                if let Ok(metadata) = serde_json::from_str::<CacheMetadata>(&meta_content) {
                    let age_days = now.signed_duration_since(metadata.cached_at).num_days();

                    if age_days > max_age_days {
                        // Delete both .meta and .log files
                        let job_number = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .and_then(|s| s.parse::<u32>().ok());

                        if let Some(job_num) = job_number {
                            let log_path = self.cache_dir.join(format!("{}.log", job_num));
                            let _ = fs::remove_file(&path); // .meta
                            let _ = fs::remove_file(&log_path); // .log
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if cache entry is still valid
    fn is_cache_valid(&self, metadata: &CacheMetadata) -> bool {
        // Running jobs should always be refetched
        if metadata.job_status == "running" {
            return false;
        }

        // Check age (15 days)
        let now = Utc::now();
        let age_days = now.signed_duration_since(metadata.cached_at).num_days();
        age_days <= 15
    }
}

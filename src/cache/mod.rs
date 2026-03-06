pub mod log_cache;
pub mod prefetch;

pub use log_cache::{LogCacheManager, CacheEntry, CacheMetadata, CacheStatus};
pub use prefetch::{PrefetchCoordinator, PrefetchResult, PREFETCH_AHEAD, PREFETCH_BEHIND, CACHE_MAX_AGE_DAYS};

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::SearchResult;
use crate::installer::hash_simple;

/// Cached search results for a single (source, query) pair.
#[derive(Serialize, Deserialize)]
struct CachedResults {
    query: String,
    source_name: String,
    timestamp: u64,
    results: Vec<SearchResult>,
}

/// File-based search result cache, keyed by (source_name, query).
pub struct SearchCache {
    cache_dir: PathBuf,
}

impl SearchCache {
    /// Create a new cache using the platform data directory.
    /// Returns `None` if the data directory cannot be determined.
    pub fn new() -> Option<Self> {
        let dir = dirs::data_dir()?.join("ion/search_cache");
        Some(Self { cache_dir: dir })
    }

    /// Look up cached results. Returns `None` on miss or if older than `max_age_secs`.
    pub fn get(
        &self,
        source_name: &str,
        query: &str,
        max_age_secs: u64,
    ) -> Option<Vec<SearchResult>> {
        let path = self.cache_path(source_name, query);
        let data = std::fs::read_to_string(&path).ok()?;
        let cached: CachedResults = serde_json::from_str(&data).ok()?;

        let now = now_secs();
        if now.saturating_sub(cached.timestamp) > max_age_secs {
            log::debug!(
                "cache expired for source={source_name} query={query:?} (age={}s, max={max_age_secs}s)",
                now.saturating_sub(cached.timestamp)
            );
            return None;
        }

        log::debug!(
            "cache hit for source={source_name} query={query:?} ({} results)",
            cached.results.len()
        );
        Some(cached.results)
    }

    /// Store results in the cache.
    pub fn put(&self, source_name: &str, query: &str, results: &[SearchResult]) {
        if let Err(e) = std::fs::create_dir_all(&self.cache_dir) {
            log::debug!("failed to create cache dir: {e}");
            return;
        }

        let cached = CachedResults {
            query: query.to_string(),
            source_name: source_name.to_string(),
            timestamp: now_secs(),
            results: results.to_vec(),
        };

        let path = self.cache_path(source_name, query);
        match serde_json::to_string(&cached) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    log::debug!("failed to write cache file: {e}");
                }
            }
            Err(e) => log::debug!("failed to serialize cache: {e}"),
        }
    }

    fn cache_path(&self, source_name: &str, query: &str) -> PathBuf {
        let key = format!("{source_name}:{query}");
        let hash = hash_simple(&key);
        self.cache_dir.join(format!("{hash:016x}.json"))
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cache = SearchCache {
            cache_dir: dir.path().to_path_buf(),
        };

        let results = vec![SearchResult::new("test-skill", "A test", "owner/repo", "github")];
        cache.put("github", "test", &results);

        let cached = cache.get("github", "test", 3600).unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].name, "test-skill");
    }

    #[test]
    fn cache_miss_on_different_query() {
        let dir = tempfile::tempdir().unwrap();
        let cache = SearchCache {
            cache_dir: dir.path().to_path_buf(),
        };

        let results = vec![SearchResult::new("a", "", "", "test")];
        cache.put("src", "hello", &results);

        assert!(cache.get("src", "world", 3600).is_none());
    }

    #[test]
    fn cache_expired() {
        let dir = tempfile::tempdir().unwrap();
        let cache = SearchCache {
            cache_dir: dir.path().to_path_buf(),
        };

        let results = vec![SearchResult::new("a", "", "", "test")];
        cache.put("src", "q", &results);

        // max_age_secs=0 means always expired
        assert!(cache.get("src", "q", 0).is_none());
    }

    #[test]
    fn cache_no_dir_does_not_panic() {
        let cache = SearchCache {
            cache_dir: PathBuf::from("/nonexistent/path/search_cache"),
        };
        // put should silently fail
        cache.put("src", "q", &[]);
        // get should return None
        assert!(cache.get("src", "q", 3600).is_none());
    }
}

//! Cache module for CodeHUD core
//!
//! This module provides intelligent caching capabilities that must exactly
//! match the Python caching behavior for zero degradation compatibility.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use std::path::Path;

/// Cache key for storing analysis results
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    pub extractor_name: String,
    pub content_hash: String,
    pub config_hash: String,
    pub version: String,
}

impl CacheKey {
    /// Create a new cache key
    pub fn new(extractor_name: String, content_hash: String, config_hash: String, version: String) -> Self {
        Self {
            extractor_name,
            content_hash,
            config_hash,
            version,
        }
    }

    /// Generate cache key as string for storage
    pub fn as_string(&self) -> String {
        format!("{}:{}:{}:{}", 
            self.extractor_name, 
            self.content_hash, 
            self.config_hash, 
            self.version
        )
    }
}

/// Cache entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub data: T,
    pub created_at: SystemTime,
    pub last_accessed: SystemTime,
    pub access_count: usize,
    pub dependencies: Vec<String>,
}

impl<T> CacheEntry<T> {
    /// Create a new cache entry
    pub fn new(data: T) -> Self {
        let now = SystemTime::now();
        Self {
            data,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            dependencies: Vec::new(),
        }
    }

    /// Update access statistics
    pub fn touch(&mut self) {
        self.last_accessed = SystemTime::now();
        self.access_count += 1;
    }

    /// Check if cache entry is stale
    pub fn is_stale(&self, max_age: Duration) -> bool {
        self.created_at.elapsed().unwrap_or(Duration::MAX) > max_age
    }
}

/// Smart cache implementation that matches Python behavior
pub struct SmartCache {
    cache_dir: std::path::PathBuf,
    max_size: usize,
    max_age: Duration,
    current_size: usize,
}

impl SmartCache {
    /// Create a new smart cache
    pub fn new(cache_dir: std::path::PathBuf, max_size: usize, max_age: Duration) -> crate::Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;
        
        Ok(Self {
            cache_dir,
            max_size,
            max_age,
            current_size: 0,
        })
    }

    /// Store data in cache
    pub fn store<T>(&mut self, key: &CacheKey, data: T) -> crate::Result<()>
    where
        T: Serialize,
    {
        let entry = CacheEntry::new(data);
        let cache_file = self.cache_dir.join(format!("{}.cache", key.as_string()));
        
        let serialized = bincode::serialize(&entry)
            .map_err(|e| crate::Error::Cache(format!("Failed to serialize cache entry: {}", e)))?;
            
        std::fs::write(&cache_file, &serialized)?;
        
        // Update cache size tracking
        self.current_size += serialized.len();
        
        // Evict old entries if necessary
        if self.current_size > self.max_size {
            self.evict_old_entries()?;
        }
        
        Ok(())
    }

    /// Retrieve data from cache
    pub fn retrieve<T>(&mut self, key: &CacheKey) -> crate::Result<Option<T>>
    where
        T: for<'de> Deserialize<'de> + Serialize,
    {
        let cache_file = self.cache_dir.join(format!("{}.cache", key.as_string()));
        
        if !cache_file.exists() {
            return Ok(None);
        }
        
        let data = std::fs::read(&cache_file)?;
        let mut entry: CacheEntry<T> = bincode::deserialize(&data)
            .map_err(|e| crate::Error::Cache(format!("Failed to deserialize cache entry: {}", e)))?;
            
        // Check if entry is stale
        if entry.is_stale(self.max_age) {
            std::fs::remove_file(cache_file)?;
            return Ok(None);
        }
        
        // Update access statistics
        entry.touch();
        
        // Write back updated entry
        let serialized = bincode::serialize(&entry)
            .map_err(|e| crate::Error::Cache(format!("Failed to serialize updated cache entry: {}", e)))?;
        std::fs::write(self.cache_dir.join(format!("{}.cache", key.as_string())), serialized)?;
        
        Ok(Some(entry.data))
    }

    /// Invalidate cache entries that depend on the given file
    pub fn invalidate_dependencies(&mut self, file_path: &Path) -> crate::Result<()> {
        let path_str = file_path.to_string_lossy().to_string();
        
        for entry in std::fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("cache") {
                // Check if this cache file depends on the changed file
                if let Ok(data) = std::fs::read(&path) {
                    if let Ok(cache_entry) = bincode::deserialize::<CacheEntry<serde_json::Value>>(&data) {
                        if cache_entry.dependencies.contains(&path_str) {
                            std::fs::remove_file(path)?;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Evict old cache entries to free up space
    fn evict_old_entries(&mut self) -> crate::Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(&self.cache_dir)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()?.to_str()? == "cache" {
                    let metadata = entry.metadata().ok()?;
                    let modified = metadata.modified().ok()?;
                    Some((path, modified, metadata.len()))
                } else {
                    None
                }
            })
            .collect();
            
        // Sort by last modified time (oldest first)
        entries.sort_by_key(|(_, modified, _)| *modified);
        
        // Remove oldest entries until we're under the size limit
        for (path, _, size) in entries {
            if self.current_size <= self.max_size * 3 / 4 {  // Keep 75% of max size
                break;
            }
            
            std::fs::remove_file(path)?;
            self.current_size = self.current_size.saturating_sub(size as usize);
        }
        
        Ok(())
    }

    /// Clear all cache entries
    pub fn clear(&mut self) -> crate::Result<()> {
        for entry in std::fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("cache") {
                std::fs::remove_file(path)?;
            }
        }
        
        self.current_size = 0;
        Ok(())
    }

    /// Get cache statistics
    pub fn get_statistics(&self) -> CacheStatistics {
        let mut stats = CacheStatistics::default();
        
        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(metadata) = entry.metadata() {
                        if entry.path().extension().and_then(|s| s.to_str()) == Some("cache") {
                            stats.total_entries += 1;
                            stats.total_size += metadata.len() as usize;
                            
                            // Check if stale
                            if let Ok(modified) = metadata.modified() {
                                if modified.elapsed().unwrap_or(Duration::MAX) > self.max_age {
                                    stats.stale_entries += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        stats.cache_hit_ratio = if stats.total_accesses > 0 {
            stats.cache_hits as f64 / stats.total_accesses as f64
        } else {
            0.0
        };
        
        stats
    }
}

/// Statistics about cache performance
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CacheStatistics {
    pub total_entries: usize,
    pub stale_entries: usize,
    pub total_size: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub total_accesses: usize,
    pub cache_hit_ratio: f64,
}

/// Tool cache for external tool results (matches Python ToolCache behavior)
pub struct ToolCache<T> {
    cache: HashMap<String, CacheEntry<T>>,
    max_entries: usize,
    max_age: Duration,
}

impl<T> ToolCache<T>
where
    T: Clone + Serialize + for<'de> Deserialize<'de>,
{
    /// Create a new tool cache
    pub fn new(max_entries: usize, max_age: Duration) -> Self {
        Self {
            cache: HashMap::new(),
            max_entries,
            max_age,
        }
    }

    /// Store result in cache
    pub fn store(&mut self, key: String, data: T) {
        let entry = CacheEntry::new(data);
        self.cache.insert(key, entry);
        
        // Evict entries if we exceed max size
        if self.cache.len() > self.max_entries {
            self.evict_lru();
        }
    }

    /// Retrieve result from cache
    pub fn retrieve(&mut self, key: &str) -> Option<T> {
        let entry = self.cache.get_mut(key)?;
        
        // Check if stale
        if entry.is_stale(self.max_age) {
            self.cache.remove(key);
            return None;
        }
        
        entry.touch();
        Some(entry.data.clone())
    }

    /// Remove stale entries
    pub fn cleanup_stale(&mut self) {
        let stale_keys: Vec<_> = self.cache
            .iter()
            .filter(|(_, entry)| entry.is_stale(self.max_age))
            .map(|(key, _)| key.clone())
            .collect();
            
        for key in stale_keys {
            self.cache.remove(&key);
        }
    }

    /// Evict least recently used entry
    fn evict_lru(&mut self) {
        if let Some((oldest_key, _)) = self.cache
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed) 
        {
            let oldest_key = oldest_key.clone();
            self.cache.remove(&oldest_key);
        }
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_key() {
        let key = CacheKey::new(
            "test_extractor".to_string(),
            "hash123".to_string(),
            "config456".to_string(),
            "1.0.0".to_string(),
        );
        
        assert_eq!(key.as_string(), "test_extractor:hash123:config456:1.0.0");
    }

    #[test]
    fn test_cache_entry() {
        let mut entry = CacheEntry::new("test_data".to_string());
        assert_eq!(entry.access_count, 0);
        
        entry.touch();
        assert_eq!(entry.access_count, 1);
    }

    #[test]
    fn test_smart_cache() -> crate::Result<()> {
        let temp_dir = tempdir().unwrap();
        let mut cache = SmartCache::new(
            temp_dir.path().to_path_buf(), 
            1024 * 1024,  // 1MB
            Duration::from_secs(3600),  // 1 hour
        )?;
        
        let key = CacheKey::new(
            "test".to_string(),
            "hash".to_string(),
            "config".to_string(),
            "1.0".to_string(),
        );
        
        // Store and retrieve
        cache.store(&key, "test_data".to_string())?;
        let result: Option<String> = cache.retrieve(&key)?;
        assert_eq!(result, Some("test_data".to_string()));
        
        Ok(())
    }

    #[test]
    fn test_tool_cache() {
        let mut cache = ToolCache::new(10, Duration::from_secs(60));
        
        cache.store("key1".to_string(), "value1".to_string());
        assert_eq!(cache.retrieve("key1"), Some("value1".to_string()));
        assert_eq!(cache.retrieve("nonexistent"), None);
    }
}
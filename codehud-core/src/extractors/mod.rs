//! Data extractors for CodeHUD analysis
//!
//! This module provides the 11+ data extractors that match the Python
//! implementation exactly for zero degradation compatibility.

use crate::Result;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

/// Base trait for all CodeHUD data extractors
/// 
/// Matches the Python BaseDataExtractor abstract class exactly
pub trait BaseDataExtractor {
    /// Extract raw data for this analysis type
    fn extract_data(&self) -> Result<HashMap<String, serde_json::Value>>;
    
    /// Get extractor type name
    fn extractor_type(&self) -> &'static str;
    
    /// Get codebase path
    fn codebase_path(&self) -> &Path;
    
    /// Get extraction timestamp
    fn extraction_timestamp(&self) -> DateTime<Utc>;
    
    /// Get common metadata for all extractors
    fn get_metadata(&self) -> ExtractorMetadata {
        ExtractorMetadata {
            extractor_type: self.extractor_type().to_string(),
            codebase_path: self.codebase_path().to_string_lossy().to_string(),
            extraction_timestamp: self.extraction_timestamp(),
            codebase_name: self.codebase_path()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        }
    }
    
    /// Extract data and include metadata (Python extract_with_metadata equivalent)
    fn extract_with_metadata(&self) -> ExtractorResult {
        match self.extract_data() {
            Ok(data) => ExtractorResult {
                metadata: self.get_metadata(),
                data,
                status: ExtractionStatus::Success,
                error_message: None,
            },
            Err(e) => {
                tracing::error!("Data extraction failed for {}: {}", self.extractor_type(), e);
                ExtractorResult {
                    metadata: self.get_metadata(),
                    data: HashMap::new(),
                    status: ExtractionStatus::Error,
                    error_message: Some(e.to_string()),
                }
            }
        }
    }
    
    /// Get source files matching given extensions (Python _get_source_files equivalent)
    fn get_source_files(&self, extensions: Option<&[&str]>) -> Result<Vec<PathBuf>> {
        let default_extensions = [".py", ".js", ".ts", ".java", ".cpp", ".c", ".rs", ".go"];
        let extensions = extensions.unwrap_or(&default_extensions);
        
        let excluded_dirs: HashSet<&str> = [
            ".git", "__pycache__", "node_modules", ".pytest_cache",
            "venv", "env", ".venv", "build", "dist", ".tox",
            ".codehud_backups", ".codehud_analysis"
        ].into_iter().collect();
        
        let mut source_files = Vec::new();
        let codebase_path = self.codebase_path();
        
        for entry in walkdir::WalkDir::new(codebase_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            // Skip if any parent directory is excluded
            if path.components().any(|component| {
                if let std::path::Component::Normal(name) = component {
                    excluded_dirs.contains(name.to_str().unwrap_or(""))
                } else {
                    false
                }
            }) {
                continue;
            }
            
            // Check file extension
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                let ext_with_dot = format!(".{}", ext);
                if extensions.contains(&ext_with_dot.as_str()) {
                    source_files.push(path.to_path_buf());
                }
            }
        }
        
        Ok(source_files)
    }
    
    /// Calculate basic metrics for a file (Python _calculate_file_metrics equivalent)
    fn calculate_file_metrics(&self, file_path: &Path) -> FileMetrics {
        match std::fs::read_to_string(file_path) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let non_empty_lines: Vec<&str> = lines.iter()
                    .filter(|line| !line.trim().is_empty())
                    .copied()
                    .collect();
                
                let metadata = std::fs::metadata(file_path);
                let (file_size, last_modified) = match metadata {
                    Ok(meta) => {
                        let modified = meta.modified()
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        let datetime: DateTime<Utc> = modified.into();
                        (meta.len(), Some(datetime))
                    }
                    Err(_) => (0, None),
                };
                
                FileMetrics {
                    path: file_path.strip_prefix(self.codebase_path())
                        .unwrap_or(file_path)
                        .to_string_lossy()
                        .to_string(),
                    total_lines: lines.len(),
                    code_lines: non_empty_lines.len(),
                    file_size_bytes: file_size,
                    extension: file_path.extension()
                        .and_then(|s| s.to_str())
                        .map(|s| format!(".{}", s)),
                    last_modified,
                    error: None,
                }
            }
            Err(e) => {
                tracing::warn!("Could not calculate metrics for {:?}: {}", file_path, e);
                FileMetrics {
                    path: file_path.strip_prefix(self.codebase_path())
                        .unwrap_or(file_path)
                        .to_string_lossy()
                        .to_string(),
                    total_lines: 0,
                    code_lines: 0,
                    file_size_bytes: 0,
                    extension: None,
                    last_modified: None,
                    error: Some(e.to_string()),
                }
            }
        }
    }
}

/// Result of data extraction with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorResult {
    pub metadata: ExtractorMetadata,
    pub data: HashMap<String, serde_json::Value>,
    pub status: ExtractionStatus,
    pub error_message: Option<String>,
}

/// Metadata for data extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorMetadata {
    pub extractor_type: String,
    pub codebase_path: String,
    pub extraction_timestamp: DateTime<Utc>,
    pub codebase_name: String,
}

/// Extraction status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractionStatus {
    Success,
    Error,
}

/// File metrics structure matching Python implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetrics {
    pub path: String,
    pub total_lines: usize,
    pub code_lines: usize,
    pub file_size_bytes: u64,
    pub extension: Option<String>,
    pub last_modified: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Get Python files excluding backup and cache directories
/// Static method equivalent from Python BaseDataExtractor
pub fn get_python_files(codebase_path: &Path) -> Result<Vec<PathBuf>> {
    let excluded_dirs: HashSet<&str> = [
        ".git", "__pycache__", "node_modules", ".pytest_cache",
        "venv", "env", ".venv", "build", "dist", ".tox",
        ".codehud_backups", ".codehud_analysis"
    ].into_iter().collect();
    
    let mut python_files = Vec::new();
    
    for entry in walkdir::WalkDir::new(codebase_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        
        // Skip if any parent directory is excluded
        if path.components().any(|component| {
            if let std::path::Component::Normal(name) = component {
                excluded_dirs.contains(name.to_str().unwrap_or(""))
            } else {
                false
            }
        }) {
            continue;
        }
        
        // Check for .py extension
        if path.extension().and_then(|s| s.to_str()) == Some("py") {
            python_files.push(path.to_path_buf());
        }
    }
    
    Ok(python_files)
}

// Module declarations for individual extractors
pub mod topology;
pub mod dependencies; 
pub mod evolution;
pub mod flow;
pub mod issues;
pub mod orphaned_files;
pub mod performance;
pub mod quality;
pub mod security;
pub mod testing;
pub mod runtime_profiler;

// Re-export the main extractors for convenience
pub use topology::TopologyExtractor;
pub use dependencies::DependenciesExtractor;
pub use evolution::EvolutionExtractor;
pub use flow::FlowExtractor;
pub use issues::IssuesExtractor;
pub use orphaned_files::OrphanedFilesExtractor;
pub use performance::PerformanceExtractor;
pub use quality::QualityExtractor;
pub use security::SecurityExtractor;
pub use testing::TestingExtractor;
pub use runtime_profiler::RuntimeProfiler;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_get_python_files() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        
        // Create some test files
        std::fs::write(temp_path.join("test.py"), "print('hello')").unwrap();
        std::fs::write(temp_path.join("main.rs"), "fn main() {}").unwrap();
        
        let python_files = get_python_files(temp_path).unwrap();
        assert_eq!(python_files.len(), 1);
        assert!(python_files[0].file_name().unwrap() == "test.py");
    }

    #[test]
    fn test_excluded_directories() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        
        // Create excluded directory with Python file
        let pycache_dir = temp_path.join("__pycache__");
        std::fs::create_dir(&pycache_dir).unwrap();
        std::fs::write(pycache_dir.join("cached.py"), "# cached").unwrap();
        
        // Create regular Python file
        std::fs::write(temp_path.join("regular.py"), "# regular").unwrap();
        
        let python_files = get_python_files(temp_path).unwrap();
        assert_eq!(python_files.len(), 1);
        assert!(python_files[0].file_name().unwrap() == "regular.py");
    }
}
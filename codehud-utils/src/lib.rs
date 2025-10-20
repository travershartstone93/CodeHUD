//! CodeHUD Utilities - Python-Compatible Utility Functions
//!
//! This crate provides utility functions that match Python behavior exactly
//! to ensure zero degradation in file operations, string processing,
//! configuration handling, and logging.

//#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

pub mod file;
pub mod string;
pub mod config;
pub mod logging;

/// Re-export commonly used utilities
pub use file::{
    safe_path_join, normalize_path, find_project_root, 
    create_backup, copy_with_backup
};
pub use string::{
    safe_truncate, normalize_whitespace, extract_function_names, 
    calculate_similarity
};
pub use config::{load_config, merge_configs, validate_config};

/// Result type used throughout CodeHUD utilities
pub type Result<T> = std::result::Result<T, UtilError>;

/// Error types for utility operations
#[derive(Debug, thiserror::Error)]
pub enum UtilError {
    /// I/O operation failed
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// String processing error
    #[error("String processing error: {0}")]
    StringProcessing(String),

    /// Path operation error
    #[error("Path operation error: {0}")]
    PathOperation(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Common exclusion patterns that match Python behavior exactly
pub fn default_exclusion_patterns() -> std::collections::HashSet<String> {
    let mut patterns = std::collections::HashSet::new();
    
    // Version control
    patterns.insert(".git".to_string());
    patterns.insert(".svn".to_string());
    patterns.insert(".hg".to_string());
    
    // Build artifacts
    patterns.insert("__pycache__".to_string());
    patterns.insert("*.pyc".to_string());
    patterns.insert("*.pyo".to_string());
    patterns.insert("build".to_string());
    patterns.insert("dist".to_string());
    
    // Virtual environments
    patterns.insert("venv".to_string());
    patterns.insert("env".to_string());
    patterns.insert(".venv".to_string());
    patterns.insert(".env".to_string());
    
    // IDE and editor files
    patterns.insert(".vscode".to_string());
    patterns.insert(".idea".to_string());
    patterns.insert("*.swp".to_string());
    patterns.insert("*.swo".to_string());
    patterns.insert("*~".to_string());
    
    // Test artifacts
    patterns.insert(".pytest_cache".to_string());
    patterns.insert(".tox".to_string());
    patterns.insert(".coverage".to_string());
    patterns.insert("htmlcov".to_string());
    
    // Package managers
    patterns.insert("node_modules".to_string());
    patterns.insert("target".to_string()); // Rust
    
    // CodeHUD specific
    patterns.insert(".codehud_backups".to_string());
    patterns.insert(".codehud_analysis".to_string());
    
    patterns
}

/// File extensions for different programming languages (matches Python logic)
pub fn supported_language_extensions() -> std::collections::HashMap<String, Vec<String>> {
    let mut extensions = std::collections::HashMap::new();
    
    extensions.insert("python".to_string(), vec![
        "py".to_string(), "pyx".to_string(), "pyi".to_string()
    ]);
    
    extensions.insert("javascript".to_string(), vec![
        "js".to_string(), "jsx".to_string(), "mjs".to_string()
    ]);
    
    extensions.insert("typescript".to_string(), vec![
        "ts".to_string(), "tsx".to_string()
    ]);
    
    extensions.insert("rust".to_string(), vec![
        "rs".to_string()
    ]);
    
    extensions.insert("java".to_string(), vec![
        "java".to_string()
    ]);
    
    extensions.insert("cpp".to_string(), vec![
        "cpp".to_string(), "cxx".to_string(), "cc".to_string(), 
        "c".to_string(), "h".to_string(), "hpp".to_string(), "hxx".to_string()
    ]);
    
    extensions.insert("go".to_string(), vec![
        "go".to_string()
    ]);
    
    extensions.insert("ruby".to_string(), vec![
        "rb".to_string()
    ]);
    
    extensions
}

/// Detect programming language from file extension
pub fn detect_language(file_path: &std::path::Path) -> Option<String> {
    let extension = file_path.extension()?.to_str()?.to_lowercase();
    let extensions_map = supported_language_extensions();
    
    for (language, exts) in extensions_map {
        if exts.contains(&extension) {
            return Some(language);
        }
    }
    
    None
}

/// Check if a path should be excluded based on patterns
pub fn should_exclude_path(path: &std::path::Path, patterns: &std::collections::HashSet<String>) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();
    
    for pattern in patterns {
        if pattern.starts_with("*.") {
            // Handle glob patterns like *.pyc
            let ext = &pattern[2..];
            if let Some(file_ext) = path.extension().and_then(|e| e.to_str()) {
                if file_ext.to_lowercase() == ext {
                    return true;
                }
            }
        } else if path_str.contains(&pattern.to_lowercase()) {
            return true;
        }
    }
    
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_language_detection() {
        assert_eq!(detect_language(&PathBuf::from("test.py")), Some("python".to_string()));
        assert_eq!(detect_language(&PathBuf::from("test.js")), Some("javascript".to_string()));
        assert_eq!(detect_language(&PathBuf::from("test.rs")), Some("rust".to_string()));
        assert_eq!(detect_language(&PathBuf::from("test.unknown")), None);
    }

    #[test]
    fn test_exclusion_patterns() {
        let patterns = default_exclusion_patterns();
        
        assert!(should_exclude_path(&PathBuf::from(".git/config"), &patterns));
        assert!(should_exclude_path(&PathBuf::from("src/__pycache__/test.pyc"), &patterns));
        assert!(should_exclude_path(&PathBuf::from("venv/lib/python"), &patterns));
        assert!(!should_exclude_path(&PathBuf::from("src/main.py"), &patterns));
    }

    #[test]
    fn test_glob_patterns() {
        let mut patterns = std::collections::HashSet::new();
        patterns.insert("*.pyc".to_string());
        patterns.insert("*.tmp".to_string());
        
        assert!(should_exclude_path(&PathBuf::from("test.pyc"), &patterns));
        assert!(should_exclude_path(&PathBuf::from("cache.tmp"), &patterns));
        assert!(!should_exclude_path(&PathBuf::from("test.py"), &patterns));
    }
}
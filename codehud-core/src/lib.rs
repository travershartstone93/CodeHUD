//! CodeHUD Core - Analysis Engine and Data Structures
//!
//! This crate provides the core analysis engine for CodeHUD, including:
//! - Data models and semantic structures
//! - Analysis extractors and algorithms  
//! - Graph analysis and pattern detection
//! - Caching and performance optimization
//!
//! This is a zero-degradation Rust translation of the Python CodeHUD core,
//! designed to maintain 100% compatibility while achieving 60%+ performance improvements.

//#![deny(missing_docs)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]
#![allow(
    clippy::multiple_crate_versions,  // Common in large dependency trees
    clippy::module_name_repetitions,  // Often necessary for clarity
)]

pub mod constants;
pub mod models;
pub mod extractors;
pub mod external_tools;
pub mod graph;
pub mod pattern;
pub mod cache;
pub mod query_engine;
pub mod analysis;

// Re-export commonly used types for convenience
pub use constants::{
    complexity_thresholds::{ComplexityThresholds, CyclomaticComplexityThresholds},
    health_score_thresholds::{HealthScoreThresholds, QualityThresholds},
};

pub use models::{
    view_types::ViewType,
    semantic_nodes::{FunctionSignature, ClassDefinition, SemanticNode, GraphBundle},
};

// Re-export Pipeline for easy access (commented out to avoid name conflict)
// pub use Pipeline;

/// Result type used throughout CodeHUD core
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for CodeHUD core operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// I/O operation failed
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Parsing error
    #[error("Parse error: {0}")]
    Parse(String),

    /// Analysis error
    #[error("Analysis error: {0}")]
    Analysis(String),

    /// Cache error
    #[error("Cache error: {0}")]
    Cache(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// External tool error
    #[error("External tool error: {tool}: {message}")]
    ExternalTool { tool: String, message: String },

    /// Timeout error
    #[error("Operation timed out after {seconds} seconds")]
    Timeout { seconds: u64 },

    /// Utility error
    #[error("Utility error: {0}")]
    Util(#[from] codehud_utils::UtilError),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Analysis pipeline types matching Python implementation exactly
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum Pipeline {
    /// Direct analysis pipeline (fast, modern approach)
    Direct,
    /// Legacy analysis pipeline (compatibility with older Python behavior)
    Legacy,
    /// Hybrid pipeline (combines direct and legacy modes)
    Hybrid,
}

impl std::fmt::Display for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Pipeline::Direct => write!(f, "direct"),
            Pipeline::Legacy => write!(f, "legacy"),
            Pipeline::Hybrid => write!(f, "hybrid"),
        }
    }
}

impl std::str::FromStr for Pipeline {
    type Err = String;
    
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "direct" => Ok(Pipeline::Direct),
            "legacy" => Ok(Pipeline::Legacy),
            "hybrid" => Ok(Pipeline::Hybrid),
            _ => Err(format!("Invalid pipeline type: '{s}'. Valid options: direct, legacy, hybrid")),
        }
    }
}

/// Global configuration for CodeHUD core
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoreConfig {
    /// Maximum number of files to analyze
    pub max_files: usize,
    /// Enable parallel processing
    pub parallel_processing: bool,
    /// Number of worker threads
    pub max_workers: usize,
    /// Enable caching
    pub enable_caching: bool,
    /// Cache directory
    pub cache_dir: Option<std::path::PathBuf>,
    /// File extensions to analyze
    pub file_extensions: Option<std::collections::HashSet<String>>,
    /// Patterns to exclude
    pub exclude_patterns: Option<std::collections::HashSet<String>>,
    /// Default analysis pipeline to use
    pub default_pipeline: Pipeline,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            max_files: 1000,
            parallel_processing: true,
            max_workers: std::thread::available_parallelism().map(|p| p.get()).unwrap_or(4),
            enable_caching: true,
            cache_dir: None,
            file_extensions: None,
            exclude_patterns: Some({
                let mut set = std::collections::HashSet::new();
                set.insert(".git".to_string());
                set.insert("__pycache__".to_string());
                set.insert("node_modules".to_string());
                set.insert(".pytest_cache".to_string());
                set.insert("venv".to_string());
                set.insert("env".to_string());
                set.insert(".venv".to_string());
                set.insert("build".to_string());
                set.insert("dist".to_string());
                set.insert(".tox".to_string());
                set.insert(".codehud_backups".to_string());
                set.insert(".codehud_analysis".to_string());
                set
            }),
            default_pipeline: Pipeline::Direct,
        }
    }
}
//! CodeHUD Transform - Code Transformation and Refactoring Engine
//!
//! This crate provides comprehensive code transformation capabilities, implementing
//! all transformation types from the Python codebase with zero degradation.
//!
//! Key features:
//! - LibCST-equivalent concrete syntax tree transformations
//! - Magic number extraction and architectural refactoring
//! - Property-based and search-based test generation
//! - Git-integrated rollback system
//! - Batch transformations with formatting preservation

#![warn(clippy::all, clippy::pedantic)]

pub mod engine;
pub mod transformers;
pub mod libcst;
pub mod rollback;
pub mod types;

// Re-export main types for convenience
pub use engine::{TransformationEngine, TransformationHistory};
pub use transformers::Transformer;
pub use types::{TransformationType, TransformationResult, TransformationSuggestion};
pub use rollback::{RollbackSystem, GitBackupIntegration};

/// Result type for transformation operations
pub type Result<T> = std::result::Result<T, TransformError>;

/// Error types for transformation operations
#[derive(Debug, thiserror::Error)]
pub enum TransformError {
    /// I/O operation failed
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Parsing error
    #[error("Parse error: {0}")]
    Parse(String),

    /// Transformation error
    #[error("Transformation error: {0}")]
    Transform(String),

    /// Git operation error
    #[error("Git error: {0}")]
    Git(String),

    /// External tool error
    #[error("External tool error: {tool}: {message}")]
    ExternalTool { tool: String, message: String },

    /// Rollback error
    #[error("Rollback error: {0}")]
    Rollback(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// CodeHUD core error
    #[error("Core error: {0}")]
    Core(#[from] codehud_core::Error),

    /// Utility error
    #[error("Utility error: {0}")]
    Util(#[from] codehud_utils::UtilError),

    /// Tree-sitter language error
    #[error("Language error: {0}")]
    Language(String),

    /// Tree-sitter query error
    #[error("Query error: {0}")]
    Query(String),

    /// UTF-8 encoding error
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    /// Regex error
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
}

impl From<tree_sitter::LanguageError> for TransformError {
    fn from(err: tree_sitter::LanguageError) -> Self {
        TransformError::Language(format!("{:?}", err))
    }
}

impl From<tree_sitter::QueryError> for TransformError {
    fn from(err: tree_sitter::QueryError) -> Self {
        TransformError::Query(format!("{:?}", err))
    }
}
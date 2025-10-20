//! Transformer implementations
//!
//! This module contains all transformer implementations matching Python behavior

use crate::{
    types::{TransformationSuggestion, TransformationResult, TransformationType},
    Result,
};
use async_trait::async_trait;
use codehud_core::models::AnalysisResult;
use std::collections::HashMap;

pub mod magic_numbers;
pub mod architectural;
pub mod batch;
pub mod multi_language;
pub mod test_generation;
pub mod unused_args;

// Re-export all transformers - matching Python exactly
pub use magic_numbers::MagicNumberTransformer;
pub use architectural::ArchitecturalRefactorer;
pub use batch::BatchTransformer;
pub use multi_language::MultiLanguageTransformer;
pub use test_generation::TestGenerationTransformer;
pub use unused_args::UnusedArgumentRemover;

/// Base transformer trait - matching Python BaseTransformer ABC exactly
/// Only abstract methods are required; concrete methods are provided as default implementations
#[async_trait]
pub trait Transformer: Send + Sync {
    /// Analyze code and suggest transformation opportunities
    /// Matches Python: analyze_transformation_opportunities(self, analysis_result: AnalysisResult) -> list[TransformationSuggestion]
    async fn analyze_transformation_opportunities(
        &self, 
        analysis_result: &AnalysisResult
    ) -> Result<Vec<TransformationSuggestion>>;
    
    /// Apply a specific transformation
    /// Matches Python: apply_transformation(self, suggestion: TransformationSuggestion, codebase_path: Path) -> TransformationResult
    async fn apply_transformation(
        &self,
        suggestion: &TransformationSuggestion,
        codebase_path: &std::path::Path
    ) -> Result<TransformationResult>;
    
    /// Validate that transformation was successful
    /// Matches Python: validate_transformation(self, result: TransformationResult, codebase_path: Path) -> dict[str, Any]
    async fn validate_transformation(
        &self,
        result: &TransformationResult,
        codebase_path: &std::path::Path
    ) -> Result<HashMap<String, serde_json::Value>>;
    
    // Concrete methods with default implementations (matching Python BaseTransformer)
    
    /// Create a git commit backup before transformation
    /// Matches Python: create_backup(self, codebase_path: Path) -> Optional[str]
    async fn create_backup(&self, codebase_path: &std::path::Path) -> Result<Option<String>> {
        // Default implementation - delegates to GitBackupIntegration
        use crate::rollback::GitBackupIntegration;
        use crate::engine::EngineConfig;
        
        let config = EngineConfig::default();
        let git_integration = GitBackupIntegration::new(&config)?;
        
        // Create backup using session ID similar to Python
        let session_id = "transformer_backup";
        git_integration.create_session_backup(session_id).map(Some).or_else(|e| {
            eprintln!("Warning: Failed to create git backup: {}", e);
            Ok(None)
        })
    }
    
    /// Rollback transformation using git
    /// Matches Python: rollback_transformation(self, backup_commit: str, codebase_path: Path) -> bool
    async fn rollback_transformation(
        &self,
        backup_commit: &str,
        _codebase_path: &std::path::Path
    ) -> Result<bool> {
        // Default implementation - delegates to GitBackupIntegration
        use crate::rollback::GitBackupIntegration;
        use crate::engine::EngineConfig;
        
        let config = EngineConfig::default();
        let git_integration = GitBackupIntegration::new(&config)?;
        
        git_integration.restore_to_commit(backup_commit).map(|_| true).or_else(|e| {
            eprintln!("Error: Failed to rollback transformation: {}", e);
            Ok(false)
        })
    }
}


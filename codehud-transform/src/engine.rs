//! Central Transformation Engine
//!
//! This module implements the main transformation orchestrator that coordinates
//! all transformation types, history tracking, and rollback functionality.

use crate::{
    types::{TransformationType, TransformationResult, TransformationSuggestion},
    transformers::Transformer,
    rollback::{RollbackSystem, GitBackupIntegration},
    Result, TransformError,
};
use codehud_core::models::AnalysisResult;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Central transformation orchestrator matching Python exactly
pub struct TransformationEngine {
    /// All available transformers
    transformers: HashMap<TransformationType, Box<dyn Transformer>>,
    /// Transformation history
    history: TransformationHistory,
    /// Rollback system
    rollback_system: RollbackSystem,
    /// Git integration for backups
    git_integration: GitBackupIntegration,
    /// Engine configuration
    config: EngineConfig,
}

/// Configuration for the transformation engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    /// Maximum number of concurrent transformations
    pub max_concurrent_transforms: usize,
    /// Whether to create Git backups by default
    pub enable_git_backup: bool,
    /// Default timeout for transformations (seconds)
    pub default_timeout_seconds: u64,
    /// Whether to validate transformations after applying
    pub validate_after_transform: bool,
    /// Whether to run tests after transformations
    pub run_tests_after_transform: bool,
    /// Directory for storing backups
    pub backup_directory: Option<PathBuf>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_concurrent_transforms: 4,
            enable_git_backup: true,
            default_timeout_seconds: 300, // 5 minutes
            validate_after_transform: true,
            run_tests_after_transform: false,
            backup_directory: None,
        }
    }
}

/// Transformation history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationHistory {
    /// All transformation sessions
    sessions: Vec<TransformationSession>,
    /// Current session if active
    current_session: Option<TransformationSession>,
}

/// A single transformation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationSession {
    /// Unique session identifier
    pub session_id: String,
    /// Session start time
    pub start_time: DateTime<Utc>,
    /// Session end time
    pub end_time: Option<DateTime<Utc>>,
    /// Transformations applied in this session
    pub transformations: Vec<TransformationRecord>,
    /// Session description
    pub description: String,
    /// Git commit hash for this session
    pub git_commit: Option<String>,
    /// Session status
    pub status: SessionStatus,
}

/// Status of a transformation session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    /// Session is currently active
    Active,
    /// Session completed successfully
    Completed,
    /// Session failed
    Failed,
    /// Session was rolled back
    RolledBack,
}

/// Record of a single transformation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationRecord {
    /// Unique transformation identifier
    pub transformation_id: String,
    /// Type of transformation
    pub transformation_type: TransformationType,
    /// Input file path
    pub file_path: String,
    /// Timestamp of transformation
    pub timestamp: DateTime<Utc>,
    /// Transformation result
    pub result: TransformationResult,
    /// Whether this transformation was rolled back
    pub rolled_back: bool,
}

impl TransformationEngine {
    /// Create a new transformation engine
    pub fn new(config: EngineConfig) -> Result<Self> {
        let mut engine = Self {
            transformers: HashMap::new(),
            history: TransformationHistory {
                sessions: Vec::new(),
                current_session: None,
            },
            rollback_system: RollbackSystem::new(&config)?,
            git_integration: GitBackupIntegration::new(&config)?,
            config,
        };

        // Register all default transformers
        engine.register_default_transformers()?;

        Ok(engine)
    }

    /// Register a transformer for a specific type
    pub fn register_transformer(
        &mut self,
        transform_type: TransformationType,
        transformer: Box<dyn Transformer>,
    ) {
        self.transformers.insert(transform_type, transformer);
    }

    /// Start a new transformation session
    pub fn start_session(&mut self, description: String) -> Result<String> {
        if self.history.current_session.is_some() {
            return Err(TransformError::Config(
                "Cannot start new session while another is active".to_string(),
            ));
        }

        let session_id = Uuid::new_v4().to_string();
        let session = TransformationSession {
            session_id: session_id.clone(),
            start_time: Utc::now(),
            end_time: None,
            transformations: Vec::new(),
            description,
            git_commit: None,
            status: SessionStatus::Active,
        };

        self.history.current_session = Some(session);

        // Create Git backup if enabled
        if self.config.enable_git_backup {
            let commit_hash = self.git_integration.create_session_backup(&session_id)?;
            if let Some(ref mut session) = self.history.current_session {
                session.git_commit = Some(commit_hash);
            }
        }

        Ok(session_id)
    }

    /// End the current transformation session
    pub fn end_session(&mut self) -> Result<()> {
        if let Some(mut session) = self.history.current_session.take() {
            session.end_time = Some(Utc::now());
            session.status = if session.transformations.iter().any(|t| t.result.status != crate::types::TransformationStatus::Completed) {
                SessionStatus::Failed
            } else {
                SessionStatus::Completed
            };

            self.history.sessions.push(session);
        }

        Ok(())
    }

    /// Analyze codebase and get transformation opportunities
    pub async fn analyze_transformation_opportunities(
        &self,
        analysis_result: &AnalysisResult,
    ) -> Result<Vec<TransformationSuggestion>> {
        let mut all_suggestions = Vec::new();
        
        for (transform_type, transformer) in &self.transformers {
            match transformer.analyze_transformation_opportunities(analysis_result).await {
                Ok(mut suggestions) => {
                    all_suggestions.append(&mut suggestions);
                }
                Err(e) => {
                    eprintln!("Warning: Transformer {:?} failed analysis: {}", transform_type, e);
                }
            }
        }
        
        Ok(all_suggestions)
    }
    
    /// Apply a transformation suggestion
    pub async fn apply_transformation(
        &mut self,
        suggestion: &TransformationSuggestion,
        codebase_path: &Path,
    ) -> Result<TransformationResult> {
        // Ensure we have an active session
        if self.history.current_session.is_none() {
            self.start_session("Auto-generated session".to_string())?;
        }

        // Get the appropriate transformer
        let transformer = self.transformers.get(&suggestion.transformation_type)
            .ok_or_else(|| TransformError::Config(
                format!("No transformer registered for type: {:?}", suggestion.transformation_type)
            ))?;

        // Create backup if enabled
        let backup_commit = if self.config.enable_git_backup {
            let session_id = if let Some(ref session) = self.history.current_session {
                &session.session_id
            } else {
                "default"
            };
            self.git_integration.create_session_backup(session_id).ok()
        } else {
            None
        };

        // Apply transformation
        let start_time = std::time::Instant::now();
        let mut result = transformer.apply_transformation(suggestion, codebase_path).await?;
        let _execution_time = start_time.elapsed();

        // Add backup commit to result
        if let Some(commit) = backup_commit {
            result.backup_commit = Some(commit);
        }

        // Validate result if enabled
        if self.config.validate_after_transform {
            let validation_results = transformer.validate_transformation(&result, codebase_path).await?;
            let validation_success = validation_results.get("overall_success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !validation_success {
                return Err(TransformError::Validation("Transformation validation failed".to_string()));
            }
        }

        // Record transformation in history
        let transformation_id = Uuid::new_v4().to_string();
        let record = TransformationRecord {
            transformation_id: transformation_id.clone(),
            transformation_type: suggestion.transformation_type,
            file_path: codebase_path.to_string_lossy().to_string(),
            timestamp: Utc::now(),
            result: result.clone(),
            rolled_back: false,
        };

        if let Some(ref mut session) = self.history.current_session {
            session.transformations.push(record);
        }

        Ok(result)
    }

    /// Apply multiple transformation suggestions in batch
    pub async fn apply_transformations_batch(
        &mut self,
        suggestions: Vec<TransformationSuggestion>,
        codebase_path: &Path,
    ) -> Result<Vec<TransformationResult>> {
        let mut results = Vec::new();

        for suggestion in suggestions {
            match self.apply_transformation(&suggestion, codebase_path).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    // On error, rollback all successful transformations in this batch
                    self.rollback_current_session().await?;
                    return Err(e);
                }
            }
        }

        Ok(results)
    }

    /// Rollback the current session
    pub async fn rollback_current_session(&mut self) -> Result<()> {
        // Collect transformation IDs that need to be rolled back
        let transformation_ids: Vec<String> = if let Some(ref session) = self.history.current_session {
            session.transformations
                .iter()
                .filter(|t| !t.rolled_back)
                .map(|t| t.transformation_id.clone())
                .collect()
        } else {
            Vec::new()
        };

        // Now rollback each transformation
        for transformation_id in transformation_ids {
            self.rollback_transformation(&transformation_id).await?;

            // Mark as rolled back in the session
            if let Some(ref mut session) = self.history.current_session {
                if let Some(transformation) = session.transformations
                    .iter_mut()
                    .find(|t| t.transformation_id == transformation_id) {
                    transformation.rolled_back = true;
                }
            }
        }

        // Update session status
        if let Some(ref mut session) = self.history.current_session {
            session.status = SessionStatus::RolledBack;
        }

        Ok(())
    }

    /// Rollback a specific transformation
    pub async fn rollback_transformation(&mut self, transformation_id: &str) -> Result<()> {
        // Find the transformation record
        let transformation = self.find_transformation_record(transformation_id)
            .ok_or_else(|| TransformError::Rollback(
                format!("Transformation not found: {}", transformation_id)
            ))?;

        // Use backup commit to restore
        if let Some(ref backup_commit) = transformation.result.backup_commit {
            self.git_integration.restore_to_commit(backup_commit)?;
        } else {
            return Err(TransformError::Rollback(
                "No backup commit available for rollback".to_string(),
            ));
        }

        Ok(())
    }

    /// Get transformation history
    pub fn get_history(&self) -> &TransformationHistory {
        &self.history
    }

    /// Get current session
    pub fn get_current_session(&self) -> Option<&TransformationSession> {
        self.history.current_session.as_ref()
    }

    /// Check if a transformer is available
    pub fn has_transformer(&self, transform_type: TransformationType) -> bool {
        self.transformers.contains_key(&transform_type)
    }

    /// Get list of available transformation types
    pub fn available_transformations(&self) -> Vec<TransformationType> {
        self.transformers.keys().copied().collect()
    }

    /// Register all default transformers - matching Python exactly
    fn register_default_transformers(&mut self) -> Result<()> {
        // Import and register transformers exactly as Python does
        use crate::transformers::{
            magic_numbers::MagicNumberTransformer,
            architectural::ArchitecturalRefactorer,
            batch::BatchTransformer,
            multi_language::MultiLanguageTransformer,
            test_generation::TestGenerationTransformer,
            unused_args::UnusedArgumentRemover,
        };

        // Magic Number Extractor -> MAGIC_NUMBERS
        self.register_transformer(
            TransformationType::MagicNumbers,
            Box::new(MagicNumberTransformer::new()?),
        );

        // Architectural Refactorer -> ARCHITECTURAL_REFACTOR
        self.register_transformer(
            TransformationType::ArchitecturalRefactor,
            Box::new(ArchitecturalRefactorer::new()?),
        );

        // Batch Transformer (includes Bowler-based large-scale refactoring) -> BATCH_TRANSFORM
        self.register_transformer(
            TransformationType::BatchTransform,
            Box::new(BatchTransformer::new()?),
        );

        // Multi-Language Transformer -> MULTI_LANGUAGE
        self.register_transformer(
            TransformationType::MultiLanguage,
            Box::new(MultiLanguageTransformer::new()?),
        );

        // Test Generator -> TEST_GENERATION
        self.register_transformer(
            TransformationType::TestGeneration,
            Box::new(TestGenerationTransformer::new()?),
        );

        // Code Cleanup (unused arguments, imports, etc.) -> CODE_CLEANUP
        self.register_transformer(
            TransformationType::CodeCleanup,
            Box::new(UnusedArgumentRemover::new()?),
        );

        Ok(())
    }

    /// Find transformation record by ID
    fn find_transformation_record(&self, transformation_id: &str) -> Option<&TransformationRecord> {
        // Search in current session
        if let Some(ref session) = self.history.current_session {
            if let Some(record) = session.transformations.iter()
                .find(|t| t.transformation_id == transformation_id) {
                return Some(record);
            }
        }

        // Search in completed sessions
        for session in &self.history.sessions {
            if let Some(record) = session.transformations.iter()
                .find(|t| t.transformation_id == transformation_id) {
                return Some(record);
            }
        }

        None
    }

    /// Validate transformation result
    async fn validate_transformation(&self, result: &TransformationResult) -> Result<()> {
        // Basic validation - check status
        if result.status != crate::types::TransformationStatus::Completed {
            return Err(TransformError::Transform(
                "Transformation did not complete successfully".to_string(),
            ));
        }

        if result.error_message.is_some() {
            return Err(TransformError::Transform(
                result.error_message.as_ref().unwrap().clone(),
            ));
        }

        Ok(())
    }

    /// Run post-transformation tests
    async fn run_post_transformation_tests(&self, _file_path: &str) -> Result<()> {
        // TODO: Implement test execution
        // This would run relevant tests after transformation to ensure nothing broke
        Ok(())
    }
}

impl TransformationHistory {
    /// Get statistics about transformation history
    pub fn get_statistics(&self) -> HistoryStatistics {
        let mut stats = HistoryStatistics::default();

        let all_sessions = self.sessions.iter()
            .chain(self.current_session.iter());

        for session in all_sessions {
            stats.total_sessions += 1;
            match session.status {
                SessionStatus::Completed => stats.successful_sessions += 1,
                SessionStatus::Failed => stats.failed_sessions += 1,
                SessionStatus::RolledBack => stats.rolled_back_sessions += 1,
                SessionStatus::Active => stats.active_sessions += 1,
            }

            for transformation in &session.transformations {
                stats.total_transformations += 1;
                if transformation.result.status == crate::types::TransformationStatus::Completed {
                    stats.successful_transformations += 1;
                } else {
                    stats.failed_transformations += 1;
                }

                if transformation.rolled_back {
                    stats.rolled_back_transformations += 1;
                }

                // Update transformation type counts
                *stats.transformations_by_type
                    .entry(transformation.transformation_type)
                    .or_insert(0) += 1;
            }
        }

        stats
    }
}

/// Statistics about transformation history
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HistoryStatistics {
    pub total_sessions: usize,
    pub successful_sessions: usize,
    pub failed_sessions: usize,
    pub rolled_back_sessions: usize,
    pub active_sessions: usize,
    pub total_transformations: usize,
    pub successful_transformations: usize,
    pub failed_transformations: usize,
    pub rolled_back_transformations: usize,
    pub transformations_by_type: HashMap<TransformationType, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TransformConfig;

    #[test]
    fn test_engine_creation() {
        let config = EngineConfig::default();
        let engine = TransformationEngine::new(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_session_management() {
        let config = EngineConfig::default();
        let mut engine = TransformationEngine::new(config).unwrap();

        let session_id = engine.start_session("Test session".to_string()).unwrap();
        assert!(engine.get_current_session().is_some());
        assert_eq!(engine.get_current_session().unwrap().session_id, session_id);

        engine.end_session().unwrap();
        assert!(engine.get_current_session().is_none());
    }

    #[test]
    fn test_history_statistics() {
        let history = TransformationHistory {
            sessions: vec![],
            current_session: None,
        };
        
        let stats = history.get_statistics();
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.total_transformations, 0);
    }
}
//! Transformation Types and Core Data Structures
//!
//! This module defines all transformation types and supporting data structures
//! exactly matching the Python implementation for zero degradation.

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use codehud_core::models::AnalysisResult;

/// All transformation types implemented exactly as in Python
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransformationType {
    /// Extract magic numbers to constants
    MagicNumbers,
    /// Large-scale architectural changes
    ArchitecturalRefactor,
    /// Batch transformations with formatting
    BatchTransform,
    /// Cross-language transformations via Comby
    MultiLanguage,
    /// Property-based and search-based test creation
    TestGeneration,
    /// Code cleanup and dead code elimination
    CodeCleanup,
}

impl TransformationType {
    /// Get all transformation types
    pub fn all() -> Vec<TransformationType> {
        vec![
            TransformationType::MagicNumbers,
            TransformationType::ArchitecturalRefactor,
            TransformationType::BatchTransform,
            TransformationType::MultiLanguage,
            TransformationType::TestGeneration,
            TransformationType::CodeCleanup,
        ]
    }

    /// Get string representation matching Python
    pub fn as_str(&self) -> &'static str {
        match self {
            TransformationType::MagicNumbers => "magic_numbers",
            TransformationType::ArchitecturalRefactor => "architectural_refactor",
            TransformationType::BatchTransform => "batch_transform",
            TransformationType::MultiLanguage => "multi_language",
            TransformationType::TestGeneration => "test_generation",
            TransformationType::CodeCleanup => "code_cleanup",
        }
    }

    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            TransformationType::MagicNumbers => "Magic Number Extraction",
            TransformationType::ArchitecturalRefactor => "Architectural Refactoring",
            TransformationType::BatchTransform => "Batch Transformations",
            TransformationType::MultiLanguage => "Multi-Language Transformations",
            TransformationType::TestGeneration => "Test Generation",
            TransformationType::CodeCleanup => "Dead Code Elimination",
        }
    }

    /// Check if transformation supports dry run
    pub fn supports_dry_run(&self) -> bool {
        match self {
            TransformationType::MagicNumbers => true,
            TransformationType::ArchitecturalRefactor => true,
            TransformationType::BatchTransform => true,
            TransformationType::MultiLanguage => true,
            TransformationType::TestGeneration => false, // Creates new files
            TransformationType::CodeCleanup => true,
        }
    }

    /// Check if transformation supports rollback
    pub fn supports_rollback(&self) -> bool {
        // All transformations support rollback via Git integration
        true
    }
}

/// Status of a transformation operation - matching Python exactly
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransformationStatus {
    /// Transformation is pending execution
    Pending,
    /// Transformation is currently in progress
    InProgress,
    /// Transformation completed successfully
    Completed,
    /// Transformation failed
    Failed,
    /// Transformation was rolled back
    RolledBack,
}

impl TransformationStatus {
    /// Get string representation matching Python
    pub fn as_str(&self) -> &'static str {
        match self {
            TransformationStatus::Pending => "pending",
            TransformationStatus::InProgress => "in_progress",
            TransformationStatus::Completed => "completed",
            TransformationStatus::Failed => "failed",
            TransformationStatus::RolledBack => "rolled_back",
        }
    }
}

/// A suggested transformation operation - matching Python dataclass exactly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationSuggestion {
    /// Type of transformation suggested
    pub transformation_type: TransformationType,
    /// Human-readable description of the transformation
    pub description: String,
    /// List of files that would be affected
    pub affected_files: Vec<String>,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
    /// Estimated impact description
    pub estimated_impact: String,
    /// Prerequisites that must be met
    pub prerequisites: Vec<String>,
    /// Additional metadata for the transformation
    pub metadata: HashMap<String, serde_json::Value>,
}

impl std::fmt::Display for TransformationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for TransformationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "magic_numbers" => Ok(TransformationType::MagicNumbers),
            "architectural_refactor" => Ok(TransformationType::ArchitecturalRefactor),
            "batch_transform" => Ok(TransformationType::BatchTransform),
            "multi_language" => Ok(TransformationType::MultiLanguage),
            "test_generation" => Ok(TransformationType::TestGeneration),
            "code_cleanup" => Ok(TransformationType::CodeCleanup),
            _ => Err(format!("Unknown transformation type: {}", s)),
        }
    }
}

/// Complexity estimate for transformations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityEstimate {
    /// Estimated execution time in seconds
    pub estimated_duration_seconds: f64,
    /// Number of files that will be modified
    pub files_to_modify: usize,
    /// Number of individual transformations
    pub transformation_count: usize,
    /// Risk level for the transformation
    pub risk_level: RiskLevel,
    /// Confidence in the estimate (0.0 to 1.0)
    pub confidence: f64,
    /// Estimated lines of code affected
    pub lines_affected: usize,
    /// Whether manual review is recommended
    pub requires_manual_review: bool,
}

/// Risk levels for transformations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Safe transformation with minimal risk
    Low,
    /// Moderate risk, should be reviewed
    Medium,
    /// High risk, requires careful review
    High,
    /// Critical risk, expert review required
    Critical,
}

impl RiskLevel {
    /// Get display color for UI
    pub fn display_color(&self) -> &'static str {
        match self {
            RiskLevel::Low => "green",
            RiskLevel::Medium => "yellow",
            RiskLevel::High => "orange",
            RiskLevel::Critical => "red",
        }
    }

    /// Get risk description
    pub fn description(&self) -> &'static str {
        match self {
            RiskLevel::Low => "Safe transformation with minimal risk of breaking changes",
            RiskLevel::Medium => "Moderate risk - review recommended before applying",
            RiskLevel::High => "High risk - careful review and testing required",
            RiskLevel::Critical => "Critical risk - expert review and comprehensive testing required",
        }
    }
}

/// Input data for transformations
#[derive(Debug, Clone)]
pub struct TransformInput {
    /// Source code to transform
    pub source_code: String,
    /// File path for context
    pub file_path: String,
    /// Language of the source code
    pub language: String,
    /// Additional configuration
    pub config: TransformConfig,
    /// Analysis context from CodeHUD
    pub analysis_context: Option<AnalysisContext>,
}

/// Configuration for transformations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformConfig {
    /// Whether to perform dry run only
    pub dry_run: bool,
    /// Whether to create backup before transformation
    pub create_backup: bool,
    /// Custom transformation parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Target style guide (e.g., "pep8", "google", "microsoft")
    pub style_guide: Option<String>,
    /// Maximum complexity allowed after transformation
    pub max_complexity: Option<f64>,
    /// Whether to preserve comments
    pub preserve_comments: bool,
    /// Whether to preserve formatting
    pub preserve_formatting: bool,
}

impl Default for TransformConfig {
    fn default() -> Self {
        Self {
            dry_run: false,
            create_backup: true,
            parameters: HashMap::new(),
            style_guide: None,
            max_complexity: None,
            preserve_comments: true,
            preserve_formatting: true,
        }
    }
}

/// Analysis context from CodeHUD core
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisContext {
    /// Function signatures in the file
    pub functions: Vec<FunctionInfo>,
    /// Class definitions in the file
    pub classes: Vec<ClassInfo>,
    /// Import statements
    pub imports: Vec<String>,
    /// Complexity metrics
    pub complexity_metrics: HashMap<String, f64>,
    /// Identified issues
    pub issues: Vec<IssueInfo>,
    /// Dependencies
    pub dependencies: Vec<String>,
}

/// Function information for transformation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    /// Function name
    pub name: String,
    /// Line number where function starts
    pub start_line: usize,
    /// Line number where function ends
    pub end_line: usize,
    /// Function parameters
    pub parameters: Vec<String>,
    /// Return type annotation if available
    pub return_type: Option<String>,
    /// Cyclomatic complexity
    pub complexity: f64,
    /// Whether function is used elsewhere
    pub is_used: bool,
}

/// Class information for transformation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    /// Class name
    pub name: String,
    /// Line number where class starts
    pub start_line: usize,
    /// Line number where class ends
    pub end_line: usize,
    /// Base classes
    pub base_classes: Vec<String>,
    /// Method names
    pub methods: Vec<String>,
    /// Number of lines of code
    pub lines_of_code: usize,
}

/// Issue information for transformation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueInfo {
    /// Issue type (e.g., "magic_number", "unused_variable")
    pub issue_type: String,
    /// Line number
    pub line_number: usize,
    /// Column number
    pub column_number: Option<usize>,
    /// Issue description
    pub description: String,
    /// Severity level
    pub severity: String,
    /// Suggested fix if available
    pub suggested_fix: Option<String>,
}

/// Result of a transformation operation - matching Python dataclass exactly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationResult {
    /// Type of transformation that was applied
    pub transformation_type: TransformationType,
    /// Current status of the transformation
    pub status: TransformationStatus,
    /// List of files that were modified
    pub files_modified: Vec<String>,
    /// Git commit hash for backup
    pub backup_commit: Option<String>,
    /// Validation results with details
    pub validation_results: HashMap<String, serde_json::Value>,
    /// Error message if transformation failed
    pub error_message: Option<String>,
    /// Whether rollback is available for this transformation
    pub rollback_available: bool,
}

/// Backup information for rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    /// Backup identifier
    pub backup_id: String,
    /// Git commit hash if using Git
    pub git_commit: Option<String>,
    /// Backup directory path
    pub backup_path: String,
    /// Timestamp of backup
    pub timestamp: DateTime<Utc>,
    /// Files included in backup
    pub files: Vec<String>,
}

/// Statistics about transformation execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationStatistics {
    /// Total lines processed
    pub lines_processed: usize,
    /// Lines modified
    pub lines_modified: usize,
    /// Lines added
    pub lines_added: usize,
    /// Lines removed
    pub lines_removed: usize,
    /// Number of transformations applied
    pub transformations_applied: usize,
    /// Complexity before transformation
    pub complexity_before: Option<f64>,
    /// Complexity after transformation
    pub complexity_after: Option<f64>,
    /// Issues fixed
    pub issues_fixed: usize,
    /// New issues introduced
    pub issues_introduced: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transformation_type_conversion() {
        for transform_type in TransformationType::all() {
            let as_string = transform_type.as_str();
            let parsed: TransformationType = as_string.parse().unwrap();
            assert_eq!(transform_type, parsed);
        }
    }

    #[test]
    fn test_transformation_type_properties() {
        assert!(TransformationType::MagicNumbers.supports_dry_run());
        assert!(TransformationType::ArchitecturalRefactor.supports_rollback());
        assert!(!TransformationType::TestGeneration.supports_dry_run());
    }

    #[test]
    fn test_risk_level_properties() {
        assert_eq!(RiskLevel::Low.display_color(), "green");
        assert_eq!(RiskLevel::Critical.display_color(), "red");
        assert!(RiskLevel::High.description().contains("High risk"));
    }

    #[test]
    fn test_transform_config_default() {
        let config = TransformConfig::default();
        assert!(!config.dry_run);
        assert!(config.create_backup);
        assert!(config.preserve_comments);
        assert!(config.preserve_formatting);
    }
}
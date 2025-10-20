//! Batch Transformer
//!
//! This module implements batch transformation capabilities for processing
//! multiple files with consistent formatting and style preservation.

use crate::{
    types::{TransformationSuggestion, TransformationResult, TransformationType, TransformationStatus, RiskLevel, TransformInput, TransformationStatistics},
    transformers::Transformer,
    Result, TransformError,
};
use codehud_core::models::AnalysisResult;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde_json::Value;

/// Batch transformer for processing multiple files
pub struct BatchTransformer {
    /// Configuration for batch processing
    config: BatchConfig,
    /// Statistics from batch operations
    statistics: BatchStatistics,
}

/// Configuration for batch transformations
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum number of files to process concurrently
    pub max_concurrent_files: usize,
    /// Whether to stop on first error
    pub fail_fast: bool,
    /// Whether to create backup of all files before processing
    pub backup_all_files: bool,
    /// Whether to validate all files after transformation
    pub validate_after_transform: bool,
    /// Maximum file size to process (in bytes)
    pub max_file_size: usize,
    /// File patterns to include
    pub include_patterns: Vec<String>,
    /// File patterns to exclude
    pub exclude_patterns: Vec<String>,
    /// Whether to preserve directory structure
    pub preserve_directory_structure: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_concurrent_files: 4,
            fail_fast: false,
            backup_all_files: true,
            validate_after_transform: true,
            max_file_size: 10 * 1024 * 1024, // 10MB
            include_patterns: vec!["**/*.py".to_string(), "**/*.js".to_string(), "**/*.ts".to_string()],
            exclude_patterns: vec!["**/node_modules/**".to_string(), "**/__pycache__/**".to_string()],
            preserve_directory_structure: true,
        }
    }
}

/// Statistics for batch operations
#[derive(Debug, Clone, Default)]
pub struct BatchStatistics {
    /// Total files processed
    pub files_processed: usize,
    /// Files successfully transformed
    pub files_succeeded: usize,
    /// Files that failed transformation
    pub files_failed: usize,
    /// Files skipped due to filters
    pub files_skipped: usize,
    /// Total processing time in milliseconds
    pub total_time_ms: u64,
    /// Average time per file in milliseconds
    pub average_time_per_file_ms: f64,
    /// Total lines processed
    pub total_lines_processed: usize,
    /// Total lines modified
    pub total_lines_modified: usize,
    /// Errors encountered
    pub errors: Vec<BatchError>,
    /// Warnings generated
    pub warnings: Vec<String>,
}

/// Batch processing error
#[derive(Debug, Clone)]
pub struct BatchError {
    /// File path where error occurred
    pub file_path: String,
    /// Error message
    pub error_message: String,
    /// Error type
    pub error_type: String,
    /// Line number if applicable
    pub line_number: Option<usize>,
}

/// Batch operation specification
#[derive(Debug, Clone)]
pub struct BatchOperation {
    /// Files to process
    pub files: Vec<BatchFile>,
    /// Transformations to apply
    pub transformations: Vec<BatchTransformation>,
    /// Configuration for this batch
    pub config: BatchConfig,
}

/// File in a batch operation
#[derive(Debug, Clone)]
pub struct BatchFile {
    /// Path to the file
    pub path: PathBuf,
    /// Language of the file (detected or specified)
    pub language: String,
    /// File size in bytes
    pub size_bytes: usize,
    /// Last modified timestamp
    pub last_modified: std::time::SystemTime,
    /// Whether this file should be processed
    pub should_process: bool,
    /// Reason for skipping if not processed
    pub skip_reason: Option<String>,
}

/// Transformation to apply in batch
#[derive(Debug, Clone)]
pub struct BatchTransformation {
    /// Type of transformation
    pub transformation_type: crate::types::TransformationType,
    /// Parameters for this transformation
    pub parameters: HashMap<String, Value>,
    /// Whether this transformation should be applied to all files
    pub apply_to_all: bool,
    /// File patterns this transformation applies to
    pub file_patterns: Vec<String>,
}

/// Result of batch processing
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// Overall success status
    pub success: bool,
    /// Results for individual files
    pub file_results: HashMap<String, TransformationResult>,
    /// Overall statistics
    pub statistics: BatchStatistics,
    /// Files that were created during batch processing
    pub created_files: Vec<String>,
    /// Files that were modified
    pub modified_files: Vec<String>,
    /// Backup information
    pub backup_info: Option<BatchBackupInfo>,
}

/// Backup information for batch operations
#[derive(Debug, Clone)]
pub struct BatchBackupInfo {
    /// Backup identifier
    pub backup_id: String,
    /// Backup directory
    pub backup_directory: PathBuf,
    /// Files included in backup
    pub backed_up_files: Vec<String>,
    /// Timestamp of backup
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl BatchTransformer {
    /// Create new batch transformer
    pub fn new() -> Result<Self> {
        Ok(Self {
            config: BatchConfig::default(),
            statistics: BatchStatistics::default(),
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: BatchConfig) -> Self {
        Self {
            config,
            statistics: BatchStatistics::default(),
        }
    }

    /// Process a batch of files (DEPRECATED - not part of Python-matching interface)
    #[allow(dead_code)]
    async fn _deprecated_process_batch(&mut self, operation: BatchOperation) -> Result<BatchResult> {
        let start_time = std::time::Instant::now();
        
        let mut file_results = HashMap::new();
        let mut statistics = BatchStatistics::default();
        let mut created_files = Vec::new();
        let mut modified_files = Vec::new();

        // Create backup if configured
        let backup_info = if operation.config.backup_all_files {
            Some(self.create_batch_backup(&operation.files).await?)
        } else {
            None
        };

        // Process files
        for file in &operation.files {
            if !file.should_process {
                statistics.files_skipped += 1;
                if let Some(reason) = &file.skip_reason {
                    statistics.warnings.push(format!("Skipped {}: {}", file.path.display(), reason));
                }
                continue;
            }

            statistics.files_processed += 1;

            match self._deprecated_process_single_file(file, &operation.transformations).await {
                Ok(result) => {
                    statistics.files_succeeded += 1;

                    // Update overall statistics from validation_results
                    if let Some(stats_value) = result.validation_results.get("statistics") {
                        if let Some(lines_processed) = stats_value.get("lines_processed").and_then(|v| v.as_u64()) {
                            statistics.total_lines_processed += lines_processed as usize;
                        }
                        if let Some(lines_modified) = stats_value.get("lines_modified").and_then(|v| v.as_u64()) {
                            statistics.total_lines_modified += lines_modified as usize;
                        }
                    }

                    // Track created and modified files from validation_results
                    if let Some(created) = result.validation_results.get("created_files").and_then(|v| v.as_array()) {
                        for file_val in created {
                            if let Some(file_str) = file_val.as_str() {
                                created_files.push(file_str.to_string());
                            }
                        }
                    }
                    modified_files.extend(result.files_modified.clone());

                    file_results.insert(file.path.to_string_lossy().to_string(), result);
                }
                Err(e) => {
                    statistics.files_failed += 1;
                    statistics.errors.push(BatchError {
                        file_path: file.path.to_string_lossy().to_string(),
                        error_message: e.to_string(),
                        error_type: "transformation_error".to_string(),
                        line_number: None,
                    });

                    if operation.config.fail_fast {
                        break;
                    }
                }
            }
        }

        // Calculate timing statistics
        let total_time = start_time.elapsed();
        statistics.total_time_ms = total_time.as_millis() as u64;
        statistics.average_time_per_file_ms = if statistics.files_processed > 0 {
            statistics.total_time_ms as f64 / statistics.files_processed as f64
        } else {
            0.0
        };

        let success = statistics.files_failed == 0 || !operation.config.fail_fast;

        Ok(BatchResult {
            success,
            file_results,
            statistics,
            created_files,
            modified_files,
            backup_info,
        })
    }

    /// Process a single file with specified transformations (DEPRECATED)
    #[allow(dead_code)]
    async fn _deprecated_process_single_file(
        &self,
        file: &BatchFile,
        transformations: &[BatchTransformation],
    ) -> Result<TransformationResult> {
        // Read file content
        let content = std::fs::read_to_string(&file.path)?;

        // Create transform input
        let input = TransformInput {
            source_code: content,
            file_path: file.path.to_string_lossy().to_string(),
            language: file.language.clone(),
            config: crate::types::TransformConfig::default(),
            analysis_context: None,
        };

        // Apply each transformation sequentially
        let mut current_input = input;
        let mut transformed_code = current_input.source_code.clone();
        let mut cumulative_statistics = TransformationStatistics {
            lines_processed: current_input.source_code.lines().count(),
            lines_modified: 0,
            lines_added: 0,
            lines_removed: 0,
            transformations_applied: 0,
            complexity_before: None,
            complexity_after: None,
            issues_fixed: 0,
            issues_introduced: 0,
        };
        let mut cumulative_modified_files: Vec<String> = vec![];
        let mut cumulative_created_files: Vec<String> = vec![];
        let mut cumulative_errors: Vec<String> = vec![];
        let mut cumulative_warnings: Vec<String> = vec![];

        let mut transformation_success = true;

        for transformation in transformations {
            // Check if transformation applies to this file
            if !self.transformation_applies_to_file(transformation, file) {
                continue;
            }

            // Apply transformation parameters
            current_input.config.parameters = transformation.parameters.clone();

            // Get appropriate transformer and apply
            let result = self._deprecated_apply_transformation_to_input(&current_input, transformation).await?;

            if result.status == crate::types::TransformationStatus::Completed {
                // Store the transformed code from validation results if available
                if let Some(code_value) = result.validation_results.get("transformed_code") {
                    if let Some(new_code) = code_value.as_str() {
                        current_input.source_code = new_code.to_string();
                        transformed_code = new_code.to_string();
                    }
                }

                // Extract statistics from validation results
                if let Some(stats_value) = result.validation_results.get("statistics") {
                    if let Some(lines_modified) = stats_value.get("lines_modified").and_then(|v| v.as_u64()) {
                        cumulative_statistics.lines_modified += lines_modified as usize;
                    }
                    if let Some(lines_added) = stats_value.get("lines_added").and_then(|v| v.as_u64()) {
                        cumulative_statistics.lines_added += lines_added as usize;
                    }
                    if let Some(lines_removed) = stats_value.get("lines_removed").and_then(|v| v.as_u64()) {
                        cumulative_statistics.lines_removed += lines_removed as usize;
                    }
                    if let Some(transformations_applied) = stats_value.get("transformations_applied").and_then(|v| v.as_u64()) {
                        cumulative_statistics.transformations_applied += transformations_applied as usize;
                    }
                    if let Some(issues_fixed) = stats_value.get("issues_fixed").and_then(|v| v.as_u64()) {
                        cumulative_statistics.issues_fixed += issues_fixed as usize;
                    }
                }

                // Merge file lists
                cumulative_modified_files.extend(result.files_modified.clone());

                // Extract warnings from validation results
                if let Some(warnings_value) = result.validation_results.get("warnings") {
                    if let Some(warnings_array) = warnings_value.as_array() {
                        for warning in warnings_array {
                            if let Some(warning_str) = warning.as_str() {
                                cumulative_warnings.push(warning_str.to_string());
                            }
                        }
                    }
                }
            } else {
                transformation_success = false;
                if let Some(error_msg) = &result.error_message {
                    cumulative_errors.push(error_msg.clone());
                }
            }
        }

        // Write transformed content back to file if not dry run
        if transformation_success && !current_input.config.dry_run {
            std::fs::write(&file.path, &transformed_code)?;
            if !cumulative_modified_files.contains(&file.path.to_string_lossy().to_string()) {
                cumulative_modified_files.push(file.path.to_string_lossy().to_string());
            }
        }

        // Build the result with the new structure
        let mut validation_results = HashMap::new();
        validation_results.insert("statistics".to_string(),
            serde_json::to_value(&cumulative_statistics)
                .map_err(|e| TransformError::Transform(format!("Failed to serialize statistics: {}", e)))?);
        validation_results.insert("transformed_code".to_string(), serde_json::json!(transformed_code));
        validation_results.insert("created_files".to_string(), serde_json::json!(cumulative_created_files));
        validation_results.insert("warnings".to_string(), serde_json::json!(cumulative_warnings));
        validation_results.insert("errors".to_string(), serde_json::json!(cumulative_errors));

        Ok(TransformationResult {
            transformation_type: TransformationType::BatchTransform,
            status: if transformation_success {
                crate::types::TransformationStatus::Completed
            } else {
                crate::types::TransformationStatus::Failed
            },
            files_modified: cumulative_modified_files,
            backup_commit: None,
            validation_results,
            error_message: if cumulative_errors.is_empty() {
                None
            } else {
                Some(cumulative_errors.join("; "))
            },
            rollback_available: false,
        })
    }

    /// Check if transformation applies to a file
    fn transformation_applies_to_file(&self, transformation: &BatchTransformation, file: &BatchFile) -> bool {
        if transformation.apply_to_all {
            return true;
        }

        // Check file patterns
        for pattern in &transformation.file_patterns {
            if self.matches_pattern(&file.path, pattern) {
                return true;
            }
        }

        false
    }

    /// Check if file path matches a pattern
    fn matches_pattern(&self, path: &Path, pattern: &str) -> bool {
        // Simplified pattern matching - in a real implementation would use glob crate
        let path_str = path.to_string_lossy();
        let pattern_str = pattern.replace("**", ".*").replace("*", "[^/]*");
        
        if let Ok(regex) = regex::Regex::new(&pattern_str) {
            regex.is_match(&path_str)
        } else {
            false
        }
    }

    /// Apply a transformation to input (DEPRECATED)
    #[allow(dead_code)]
    async fn _deprecated_apply_transformation_to_input(
        &self,
        input: &TransformInput,
        transformation: &BatchTransformation,
    ) -> Result<TransformationResult> {
        // This would delegate to the appropriate transformer based on transformation type
        // For now, return a placeholder result
        Ok(TransformationResult {
            transformation_type: crate::types::TransformationType::BatchTransform,
            status: crate::types::TransformationStatus::Completed,
            files_modified: vec![],
            backup_commit: None,
            validation_results: {
                let mut results = HashMap::new();
                results.insert("lines_processed".to_string(), serde_json::json!(input.source_code.lines().count()));
                results.insert("success".to_string(), serde_json::json!(true));
                results
            },
            error_message: None,
            rollback_available: false,
        })
    }

    /// Create backup for batch operation
    async fn create_batch_backup(&self, files: &[BatchFile]) -> Result<BatchBackupInfo> {
        let backup_id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now();
        let backup_directory = std::env::temp_dir().join(format!("codehud_batch_backup_{}", backup_id));

        // Create backup directory
        std::fs::create_dir_all(&backup_directory)?;

        let mut backed_up_files = Vec::new();

        for file in files {
            if file.should_process && file.path.exists() {
                let relative_path = file.path.file_name()
                    .ok_or_else(|| TransformError::Config("Invalid file path".to_string()))?;
                
                let backup_path = backup_directory.join(relative_path);
                
                // Create parent directories if needed
                if let Some(parent) = backup_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                // Copy file to backup location
                std::fs::copy(&file.path, &backup_path)?;
                backed_up_files.push(file.path.to_string_lossy().to_string());
            }
        }

        Ok(BatchBackupInfo {
            backup_id,
            backup_directory,
            backed_up_files,
            timestamp,
        })
    }

    /// Scan directory for files to process
    pub fn scan_directory(&self, directory: &Path, config: &BatchConfig) -> Result<Vec<BatchFile>> {
        let mut files = Vec::new();
        
        if !directory.exists() {
            return Err(TransformError::Config(format!("Directory does not exist: {}", directory.display())));
        }

        self.scan_directory_recursive(directory, &mut files, config)?;
        
        Ok(files)
    }

    /// Recursively scan directory
    fn scan_directory_recursive(&self, dir: &Path, files: &mut Vec<BatchFile>, config: &BatchConfig) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let file = self.analyze_file(&path, config)?;
                files.push(file);
            } else if path.is_dir() && config.preserve_directory_structure {
                self.scan_directory_recursive(&path, files, config)?;
            }
        }

        Ok(())
    }

    /// Analyze a file to determine if it should be processed
    fn analyze_file(&self, path: &Path, config: &BatchConfig) -> Result<BatchFile> {
        let metadata = std::fs::metadata(path)?;
        let size_bytes = metadata.len() as usize;
        let last_modified = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);

        // Detect language from file extension
        let language = self.detect_language(path);

        // Check if file should be processed
        let (should_process, skip_reason) = self.should_process_file(path, size_bytes, &language, config);

        Ok(BatchFile {
            path: path.to_path_buf(),
            language,
            size_bytes,
            last_modified,
            should_process,
            skip_reason,
        })
    }

    /// Detect programming language from file extension
    fn detect_language(&self, path: &Path) -> String {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("py") => "python".to_string(),
            Some("js") => "javascript".to_string(),
            Some("ts") => "typescript".to_string(),
            Some("jsx") => "javascript".to_string(),
            Some("tsx") => "typescript".to_string(),
            Some("rs") => "rust".to_string(),
            Some("java") => "java".to_string(),
            Some("cpp") | Some("cxx") | Some("cc") => "cpp".to_string(),
            Some("c") => "c".to_string(),
            Some("h") | Some("hpp") => "c".to_string(),
            Some("go") => "go".to_string(),
            Some("rb") => "ruby".to_string(),
            Some("php") => "php".to_string(),
            _ => "unknown".to_string(),
        }
    }

    /// Check if file should be processed
    fn should_process_file(&self, path: &Path, size_bytes: usize, language: &str, config: &BatchConfig) -> (bool, Option<String>) {
        // Check file size
        if size_bytes > config.max_file_size {
            return (false, Some(format!("File too large: {} bytes", size_bytes)));
        }

        // Check language support
        if language == "unknown" {
            return (false, Some("Unknown file type".to_string()));
        }

        // Check include patterns
        let matches_include = config.include_patterns.is_empty() || 
            config.include_patterns.iter().any(|pattern| self.matches_pattern(path, pattern));

        if !matches_include {
            return (false, Some("Does not match include patterns".to_string()));
        }

        // Check exclude patterns
        let matches_exclude = config.exclude_patterns.iter().any(|pattern| self.matches_pattern(path, pattern));

        if matches_exclude {
            return (false, Some("Matches exclude pattern".to_string()));
        }

        (true, None)
    }

    /// Get processing statistics
    pub fn get_statistics(&self) -> &BatchStatistics {
        &self.statistics
    }

    /// Reset statistics
    pub fn reset_statistics(&mut self) {
        self.statistics = BatchStatistics::default();
    }
    
    /// Apply a specific pattern transformation to source code
    fn apply_pattern_to_code(&self, source_code: &str, pattern_name: &str) -> Result<String> {
        // Simple pattern-based transformations
        // In a real implementation, this would use more sophisticated AST manipulation
        match pattern_name {
            "format_modernization" => {
                // Example: improve format! usage consistency
                let improved = source_code.replace("print!", "println!");
                Ok(improved)
            }
            "import_organization" => {
                // Example: organize use statements (very basic)
                Ok(source_code.to_string()) // No change for now - would need proper parsing
            }
            "error_handling_improvement" => {
                // Example: suggest alternatives to unwrap() (very basic)
                let improved = source_code.replace(".unwrap()", ".expect(\"TODO: handle error properly\")");
                Ok(improved)
            }
            _ => {
                // Unknown pattern - return unchanged
                Ok(source_code.to_string())
            }
        }
    }
}

#[async_trait]
impl Transformer for BatchTransformer {
    /// Analyze code and suggest batch transformation opportunities
    async fn analyze_transformation_opportunities(
        &self,
        analysis_result: &AnalysisResult,
    ) -> Result<Vec<TransformationSuggestion>> {
        let mut suggestions = Vec::new();
        
        // Look for patterns that could benefit from large-scale batch transformations
        // This matches the Python BatchTransformer which looks for deprecated patterns
        if let Some(parsed_files) = &analysis_result.parsed_files {
            let mut pattern_counts: HashMap<String, usize> = HashMap::new();
            let mut affected_files = Vec::new();
            
            // Count patterns across all files
            for file_data in parsed_files {
                if let Some(file_path) = file_data.get("file_path").and_then(|v| v.as_str()) {
                    if let Some(source_code) = file_data.get("source_code").and_then(|v| v.as_str()) {
                        affected_files.push(file_path.to_string());
                        
                        // Look for deprecated string formatting patterns (like Python %)
                        if source_code.contains("format!(") || source_code.contains("println!(") {
                            *pattern_counts.entry("format_modernization".to_string()).or_insert(0) += 1;
                        }
                        
                        // Look for import reorganization opportunities
                        if source_code.contains("use ") {
                            *pattern_counts.entry("import_organization".to_string()).or_insert(0) += 1;
                        }
                        
                        // Look for code consistency patterns
                        if source_code.contains("unwrap()") {
                            *pattern_counts.entry("error_handling_improvement".to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
            
            // Create suggestions for patterns found in multiple files (batch-worthy)
            for (pattern_name, count) in pattern_counts {
                if count > 5 { // Only suggest if found in multiple files
                    let description = match pattern_name.as_str() {
                        "format_modernization" => format!("Modernize formatting patterns across {} files", count),
                        "import_organization" => format!("Reorganize imports across {} files", count),
                        "error_handling_improvement" => format!("Improve error handling patterns across {} files", count),
                        _ => format!("Apply {} pattern across {} files", pattern_name, count),
                    };
                    
                    let suggestion = TransformationSuggestion {
                        transformation_type: TransformationType::BatchTransform,
                        description,
                        affected_files: affected_files.clone(),
                        confidence: if count > 10 { 0.9 } else { 0.7 },
                        estimated_impact: format!("Large-scale change affecting {} files with {} instances", affected_files.len(), count),
                        prerequisites: vec![
                            "Comprehensive test suite".to_string(),
                            "Clean git working directory".to_string(),
                            "Code review for large-scale changes".to_string(),
                            "Staged deployment plan".to_string(),
                        ],
                        metadata: {
                            let mut metadata = HashMap::new();
                            metadata.insert("pattern_name".to_string(), serde_json::json!(pattern_name));
                            metadata.insert("pattern_count".to_string(), serde_json::json!(count));
                            metadata.insert("files_affected".to_string(), serde_json::json!(affected_files.len()));
                            metadata.insert("risk_level".to_string(), serde_json::json!("medium"));
                            metadata
                        },
                    };
                    suggestions.push(suggestion);
                }
            }
        }
        
        Ok(suggestions)
    }
    
    /// Apply batch transformation
    async fn apply_transformation(
        &self,
        suggestion: &TransformationSuggestion,
        codebase_path: &std::path::Path,
    ) -> Result<TransformationResult> {
        let pattern_name = suggestion.metadata.get("pattern_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        let mut files_modified = Vec::new();
        
        // Apply the batch transformation pattern to all affected files
        for file_path in &suggestion.affected_files {
            let full_path = codebase_path.join(file_path);
            if let Ok(source_code) = std::fs::read_to_string(&full_path) {
                let transformed_code = self.apply_pattern_to_code(&source_code, pattern_name)?;
                
                // Only write if the code actually changed
                if transformed_code != source_code {
                    std::fs::write(&full_path, &transformed_code)?;
                    files_modified.push(file_path.clone());
                }
            }
        }
        
        let files_count = files_modified.len();
        Ok(TransformationResult {
            transformation_type: TransformationType::BatchTransform,
            status: TransformationStatus::Completed,
            files_modified,
            backup_commit: None, // Will be set by engine if backup is created
            validation_results: {
                let mut results = HashMap::new();
                results.insert("pattern_applied".to_string(), serde_json::json!(pattern_name));
                results.insert("files_processed".to_string(), serde_json::json!(suggestion.affected_files.len()));
                results.insert("files_modified".to_string(), serde_json::json!(files_count));
                results
            },
            error_message: None,
            rollback_available: true,
        })
    }
    
    /// Validate batch transformation results
    /// Returns detailed validation results as dict[str, Any] matching Python
    async fn validate_transformation(
        &self,
        result: &TransformationResult,
        codebase_path: &std::path::Path
    ) -> Result<HashMap<String, serde_json::Value>> {
        use std::collections::HashMap;
        use serde_json::json;
        
        let mut validation_results = HashMap::new();
        
        // Basic validation - check if transformation completed successfully
        let is_successful = result.status == TransformationStatus::Completed;
        validation_results.insert("success".to_string(), json!(is_successful));
        validation_results.insert("status".to_string(), json!(result.status.as_str()));
        
        // Validate that all modified files exist and are accessible
        let mut files_valid = true;
        let mut inaccessible_files = Vec::new();
        
        for file_path in &result.files_modified {
            let full_path = codebase_path.join(file_path);
            if !full_path.exists() {
                files_valid = false;
                inaccessible_files.push(format!("File not found: {}", file_path));
                continue;
            }
            
            // Try to read the file to ensure it's accessible
            if std::fs::read_to_string(&full_path).is_err() {
                files_valid = false;
                inaccessible_files.push(format!("Cannot read file: {}", file_path));
            }
        }
        
        validation_results.insert("files_valid".to_string(), json!(files_valid));
        validation_results.insert("files_modified_count".to_string(), json!(result.files_modified.len()));
        
        if !inaccessible_files.is_empty() {
            validation_results.insert("inaccessible_files".to_string(), json!(inaccessible_files));
        }
        
        // Additional validation: check that batch operation didn't introduce syntax errors
        // This could be enhanced with language-specific parsing
        validation_results.insert("batch_operation_type".to_string(), json!("pattern_based"));
        
        Ok(validation_results)
    }

}

impl Clone for BatchTransformer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            statistics: BatchStatistics::default(), // Don't clone statistics
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_transformer_creation() {
        let transformer = BatchTransformer::new();
        assert!(transformer.is_ok());
    }

    #[test]
    fn test_language_detection() {
        let transformer = BatchTransformer::new().unwrap();
        assert_eq!(transformer.detect_language(Path::new("test.py")), "python");
        assert_eq!(transformer.detect_language(Path::new("test.js")), "javascript");
        assert_eq!(transformer.detect_language(Path::new("test.ts")), "typescript");
    }

    #[test]
    fn test_pattern_matching() {
        let transformer = BatchTransformer::new().unwrap();
        assert!(transformer.matches_pattern(Path::new("src/main.py"), "**/*.py"));
        assert!(!transformer.matches_pattern(Path::new("src/main.js"), "**/*.py"));
    }
}
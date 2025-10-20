//! Multi-Language Transformer
//!
//! This module implements cross-language transformations using Comby-style
//! pattern matching and replacement, enabling transformations across different
//! programming languages with consistent patterns.

use crate::{
    types::{TransformationSuggestion, TransformationResult, TransformationType, TransformationStatus, RiskLevel},
    transformers::Transformer,
    Result, TransformError,
};
use codehud_core::models::AnalysisResult;
use async_trait::async_trait;
use regex::Regex;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Multi-language transformation pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLanguagePattern {
    /// Pattern name/identifier
    pub name: String,
    /// Comby-style pattern string
    pub pattern: String,
    /// Example before transformation
    pub example_before: String,
    /// Example after transformation
    pub example_after: String,
    /// Languages this pattern supports
    pub supported_languages: Vec<String>,
    /// Confidence in this pattern (0.0 to 1.0)
    pub confidence: f64,
}

/// Configuration for multi-language transformations
#[derive(Debug, Clone)]
pub struct MultiLanguageConfig {
    /// Whether to preserve formatting
    pub preserve_formatting: bool,
    /// Maximum number of replacements per file
    pub max_replacements_per_file: usize,
    /// Whether to apply language-specific rules
    pub apply_language_rules: bool,
}

impl Default for MultiLanguageConfig {
    fn default() -> Self {
        Self {
            preserve_formatting: true,
            max_replacements_per_file: 1000,
            apply_language_rules: true,
        }
    }
}

/// Multi-language transformer for cross-language transformations
#[derive(Debug)]
pub struct MultiLanguageTransformer {
    /// Transformation patterns
    patterns: Vec<MultiLanguagePattern>,
    /// Configuration
    config: MultiLanguageConfig,
}

impl MultiLanguageTransformer {
    /// Create new multi-language transformer
    pub fn new() -> Result<Self> {
        Ok(Self {
            patterns: Self::create_default_patterns(),
            config: MultiLanguageConfig::default(),
        })
    }
    
    /// Create with custom patterns
    pub fn with_patterns(patterns: Vec<MultiLanguagePattern>) -> Self {
        Self {
            patterns,
            config: MultiLanguageConfig::default(),
        }
    }
    
    /// Create default transformation patterns
    fn create_default_patterns() -> Vec<MultiLanguagePattern> {
        vec![
            MultiLanguagePattern {
                name: "documentation_standardization".to_string(),
                pattern: "TODO".to_string(),
                example_before: "// TODO: fix this".to_string(),
                example_after: "// TODO(username): fix this by YYYY-MM-DD".to_string(),
                supported_languages: vec!["python".to_string(), "rust".to_string(), "javascript".to_string()],
                confidence: 0.9,
            },
            MultiLanguagePattern {
                name: "naming_convention_consistency".to_string(),
                pattern: "_".to_string(),
                example_before: "snake_case".to_string(),
                example_after: "camelCase".to_string(),
                supported_languages: vec!["javascript".to_string(), "typescript".to_string()],
                confidence: 0.7,
            },
        ]
    }
    
    /// Detect language from source code and file path
    pub fn detect_language(&self, source_code: &str, file_path: Option<&str>) -> String {
        if let Some(path) = file_path {
            // Detect by file extension
            if path.ends_with(".py") {
                return "python".to_string();
            } else if path.ends_with(".rs") {
                return "rust".to_string();
            } else if path.ends_with(".js") {
                return "javascript".to_string();
            } else if path.ends_with(".ts") {
                return "typescript".to_string();
            }
        }
        
        // Detect by code patterns
        if source_code.contains("def ") || source_code.contains("import ") {
            "python".to_string()
        } else if source_code.contains("fn ") || source_code.contains("use ") {
            "rust".to_string()
        } else if source_code.contains("function ") || source_code.contains("const ") {
            "javascript".to_string()
        } else {
            "unknown".to_string()
        }
    }
    
    /// Apply a language-specific pattern to source code
    fn apply_language_pattern(&self, source_code: &str, pattern_name: &str, language: &str) -> Result<String> {
        match pattern_name {
            "documentation_standardization" => {
                // Simple TODO enhancement
                let enhanced = source_code.replace("TODO", "TODO(reviewer)");
                Ok(enhanced)
            }
            "naming_convention_consistency" => {
                // Basic naming convention adjustments - would be more sophisticated in real implementation
                Ok(source_code.to_string()) // No change for now
            }
            _ => Ok(source_code.to_string()),
        }
    }
}

#[async_trait]
impl Transformer for MultiLanguageTransformer {
    /// Analyze code for multi-language transformation opportunities
    async fn analyze_transformation_opportunities(
        &self,
        analysis_result: &AnalysisResult,
    ) -> Result<Vec<TransformationSuggestion>> {
        let mut suggestions = Vec::new();
        
        // Look for cross-language patterns and opportunities
        if let Some(parsed_files) = &analysis_result.parsed_files {
            let mut language_patterns: HashMap<String, usize> = HashMap::new();
            let mut affected_files = Vec::new();
            
            for file_data in parsed_files {
                if let Some(file_path) = file_data.get("file_path").and_then(|v| v.as_str()) {
                    if let Some(source_code) = file_data.get("source_code").and_then(|v| v.as_str()) {
                        let language = self.detect_language(source_code, Some(file_path));
                        affected_files.push(file_path.to_string());
                        
                        // Look for cross-language transformation opportunities
                        if source_code.contains("TODO") || source_code.contains("FIXME") {
                            *language_patterns.entry("documentation_standardization".to_string()).or_insert(0) += 1;
                        }
                        
                        // Look for naming convention patterns
                        if source_code.contains("_") {
                            *language_patterns.entry("naming_convention_consistency".to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
            
            // Create suggestions for patterns found across languages
            for (pattern_name, count) in language_patterns {
                if count > 3 { // Only suggest if found in multiple files
                    let suggestion = TransformationSuggestion {
                        transformation_type: TransformationType::MultiLanguage,
                        description: format!("Apply {} pattern across {} files", pattern_name.replace('_', " "), count),
                        affected_files: affected_files.clone(),
                        confidence: 0.8,
                        estimated_impact: format!("Cross-language consistency improvement affecting {} files", count),
                        prerequisites: vec![
                            "Cross-language testing".to_string(),
                            "Style guide verification".to_string(),
                        ],
                        metadata: {
                            let mut metadata = HashMap::new();
                            metadata.insert("pattern_name".to_string(), serde_json::json!(pattern_name));
                            metadata.insert("pattern_count".to_string(), serde_json::json!(count));
                            metadata
                        },
                    };
                    suggestions.push(suggestion);
                }
            }
        }
        
        Ok(suggestions)
    }
    
    /// Apply multi-language transformation
    async fn apply_transformation(
        &self,
        suggestion: &TransformationSuggestion,
        codebase_path: &std::path::Path,
    ) -> Result<TransformationResult> {
        let pattern_name = suggestion.metadata.get("pattern_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        let mut files_modified = Vec::new();
        
        // Apply transformation to affected files
        for file_path in &suggestion.affected_files {
            let full_path = codebase_path.join(file_path);
            if let Ok(source_code) = std::fs::read_to_string(&full_path) {
                let language = self.detect_language(&source_code, Some(file_path));
                let transformed_code = self.apply_language_pattern(&source_code, pattern_name, &language)?;
                
                if transformed_code != source_code {
                    std::fs::write(&full_path, &transformed_code)?;
                    files_modified.push(file_path.clone());
                }
            }
        }
        
        Ok(TransformationResult {
            transformation_type: TransformationType::MultiLanguage,
            status: TransformationStatus::Completed,
            files_modified,
            backup_commit: None,
            validation_results: {
                let mut results = HashMap::new();
                results.insert("pattern_applied".to_string(), serde_json::json!(pattern_name));
                results.insert("files_processed".to_string(), serde_json::json!(suggestion.affected_files.len()));
                results
            },
            error_message: None,
            rollback_available: true,
        })
    }
    
    /// Validate multi-language transformation
    async fn validate_transformation(&self, result: &TransformationResult, codebase_path: &std::path::Path) -> Result<HashMap<String, serde_json::Value>> {
        let mut validation_results = HashMap::new();
        
        // Validate transformation status
        validation_results.insert("status_valid".to_string(), serde_json::json!(result.status == TransformationStatus::Completed));
        validation_results.insert("error_free".to_string(), serde_json::json!(result.error_message.is_none()));
        
        // Validate files were actually modified
        let mut files_exist = true;
        let mut files_readable = true;
        
        for file_path in &result.files_modified {
            let full_path = codebase_path.join(file_path);
            if !full_path.exists() {
                files_exist = false;
                break;
            }
            
            if std::fs::read_to_string(&full_path).is_err() {
                files_readable = false;
                break;
            }
        }
        
        validation_results.insert("files_exist".to_string(), serde_json::json!(files_exist));
        validation_results.insert("files_readable".to_string(), serde_json::json!(files_readable));
        validation_results.insert("files_modified_count".to_string(), serde_json::json!(result.files_modified.len()));
        
        // Overall validation success
        let overall_success = result.status == TransformationStatus::Completed 
            && result.error_message.is_none() 
            && files_exist 
            && files_readable;
        validation_results.insert("overall_success".to_string(), serde_json::json!(overall_success));
        
        Ok(validation_results)
    }
    
}
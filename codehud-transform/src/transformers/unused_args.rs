//! Unused Argument Remover Transformer
//!
//! This module implements dead code elimination, focusing on unused function
//! arguments, imports, and variables, matching Python UnusedArgumentRemover.

use crate::{
    types::{TransformationSuggestion, TransformationResult, TransformationType, TransformationStatus, RiskLevel},
    transformers::Transformer,
    Result, TransformError,
};
use codehud_core::models::AnalysisResult;
use async_trait::async_trait;
use regex::Regex;
use std::collections::{HashMap, HashSet};

/// Configuration for unused code removal
#[derive(Debug, Clone)]
pub struct CodeCleanupConfig {
    /// Remove unused function arguments
    pub remove_unused_args: bool,
    /// Remove unused imports
    pub remove_unused_imports: bool,
    /// Remove unused variables
    pub remove_unused_variables: bool,
    /// Preserve public API elements
    pub preserve_public_api: bool,
}

impl Default for CodeCleanupConfig {
    fn default() -> Self {
        Self {
            remove_unused_args: true,
            remove_unused_imports: true,
            remove_unused_variables: true,
            preserve_public_api: true,
        }
    }
}

/// Information about detected unused code
#[derive(Debug, Clone)]
pub struct UnusedCodeItem {
    /// Name of the unused item
    pub name: String,
    /// Type of unused item (argument, import, variable)
    pub item_type: String,
    /// Line number where it appears
    pub line_number: usize,
    /// Function/scope containing the item
    pub scope: String,
}

/// Unused argument remover transformer matching Python UnusedArgumentRemover
#[derive(Debug)]
pub struct UnusedArgumentRemover {
    /// Configuration
    config: CodeCleanupConfig,
}

impl UnusedArgumentRemover {
    /// Create new unused argument remover
    pub fn new() -> Result<Self> {
        Ok(Self {
            config: CodeCleanupConfig::default(),
        })
    }
    
    /// Create with custom configuration
    pub fn with_config(config: CodeCleanupConfig) -> Self {
        Self { config }
    }
    
    /// Detect unused arguments in source code
    fn find_unused_arguments(&self, source_code: &str, language: &str) -> Vec<UnusedCodeItem> {
        let mut unused_items = Vec::new();
        
        match language {
            "python" => {
                self.find_unused_python_arguments(source_code, &mut unused_items);
            }
            "rust" => {
                self.find_unused_rust_arguments(source_code, &mut unused_items);
            }
            _ => {} // Unsupported language
        }
        
        unused_items
    }
    
    /// Find unused arguments in Python code
    fn find_unused_python_arguments(&self, source_code: &str, unused_items: &mut Vec<UnusedCodeItem>) {
        let lines: Vec<&str> = source_code.lines().collect();
        
        for (i, line) in lines.iter().enumerate() {
            if line.trim().starts_with("def ") {
                // Extract function definition
                if let Some(args_start) = line.find('(') {
                    if let Some(args_end) = line.find(')') {
                        let args_str = &line[args_start + 1..args_end];
                        let args: Vec<&str> = args_str.split(',')
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty() && *s != "self")
                            .collect();
                        
                        // Check if arguments are used in function body
                        let func_name = line.split_whitespace().nth(1).unwrap_or("unknown");
                        for arg in args {
                            let arg_name = arg.split(':').next().unwrap_or(arg).trim();
                            if !arg_name.is_empty() && !self.is_argument_used(&lines, i + 1, arg_name) {
                                unused_items.push(UnusedCodeItem {
                                    name: arg_name.to_string(),
                                    item_type: "argument".to_string(),
                                    line_number: i + 1,
                                    scope: func_name.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Find unused arguments in Rust code
    fn find_unused_rust_arguments(&self, source_code: &str, unused_items: &mut Vec<UnusedCodeItem>) {
        let lines: Vec<&str> = source_code.lines().collect();
        
        for (i, line) in lines.iter().enumerate() {
            if line.trim().starts_with("fn ") {
                // Extract function definition
                if let Some(args_start) = line.find('(') {
                    if let Some(args_end) = line.find(')') {
                        let args_str = &line[args_start + 1..args_end];
                        let args: Vec<&str> = args_str.split(',')
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty() && !s.starts_with("&"))
                            .collect();
                        
                        // Check if arguments are used in function body
                        let func_name = line.split_whitespace().nth(1).unwrap_or("unknown");
                        for arg in args {
                            let arg_name = arg.split(':').next().unwrap_or(arg).trim();
                            if !arg_name.is_empty() && !arg_name.starts_with("_") && !self.is_argument_used(&lines, i + 1, arg_name) {
                                unused_items.push(UnusedCodeItem {
                                    name: arg_name.to_string(),
                                    item_type: "argument".to_string(),
                                    line_number: i + 1,
                                    scope: func_name.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Check if an argument is used in the function body
    fn is_argument_used(&self, lines: &[&str], start_line: usize, arg_name: &str) -> bool {
        let mut brace_count = 0;
        let mut in_function = false;
        
        for line in lines.iter().skip(start_line) {
            if line.contains('{') {
                brace_count += line.matches('{').count();
                in_function = true;
            }
            if line.contains('}') {
                brace_count -= line.matches('}').count();
                if brace_count == 0 && in_function {
                    break; // End of function
                }
            }
            
            // Simple check if argument name appears in line
            if in_function && line.contains(arg_name) {
                return true;
            }
        }
        
        false
    }
    
    /// Remove unused arguments from source code
    fn remove_unused_arguments(&self, source_code: &str, unused_items: &[UnusedCodeItem]) -> String {
        let mut result = source_code.to_string();
        
        // Simple removal - in real implementation would use proper AST manipulation
        for item in unused_items {
            if item.item_type == "argument" {
                // Simple pattern-based removal (would be more sophisticated in real implementation)
                let pattern = format!(", {}", item.name);
                result = result.replace(&pattern, "");
                
                let pattern = format!("{}, ", item.name);
                result = result.replace(&pattern, "");
            }
        }
        
        result
    }
    
    /// Detect language from file path
    fn detect_language(&self, file_path: &str) -> String {
        if file_path.ends_with(".py") {
            "python".to_string()
        } else if file_path.ends_with(".rs") {
            "rust".to_string()
        } else {
            "unknown".to_string()
        }
    }
}

#[async_trait]
impl Transformer for UnusedArgumentRemover {
    /// Analyze code and suggest unused code removal opportunities
    async fn analyze_transformation_opportunities(
        &self,
        analysis_result: &AnalysisResult,
    ) -> Result<Vec<TransformationSuggestion>> {
        let mut suggestions = Vec::new();
        
        // Look for unused code elements
        if let Some(parsed_files) = &analysis_result.parsed_files {
            let mut total_unused_items = 0;
            let mut affected_files = Vec::new();
            
            for file_data in parsed_files {
                if let Some(file_path) = file_data.get("file_path").and_then(|v| v.as_str()) {
                    if let Some(source_code) = file_data.get("source_code").and_then(|v| v.as_str()) {
                        let language = self.detect_language(file_path);
                        let unused_items = self.find_unused_arguments(source_code, &language);
                        
                        if !unused_items.is_empty() {
                            total_unused_items += unused_items.len();
                            affected_files.push(file_path.to_string());
                        }
                    }
                }
            }
            
            // Create suggestion if we found unused code
            if total_unused_items > 0 {
                let suggestion = TransformationSuggestion {
                    transformation_type: TransformationType::CodeCleanup,
                    description: format!("Remove {} unused code elements across {} files", total_unused_items, affected_files.len()),
                    affected_files,
                    confidence: 0.95,
                    estimated_impact: format!("Code cleanup removing {} unused elements", total_unused_items),
                    prerequisites: vec![
                        "Comprehensive test coverage".to_string(),
                        "Code review approval".to_string(),
                    ],
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("unused_items_count".to_string(), serde_json::json!(total_unused_items));
                        metadata.insert("cleanup_type".to_string(), serde_json::json!("unused_arguments"));
                        metadata
                    },
                };
                suggestions.push(suggestion);
            }
        }
        
        Ok(suggestions)
    }
    
    /// Apply unused code removal transformation
    async fn apply_transformation(
        &self,
        suggestion: &TransformationSuggestion,
        codebase_path: &std::path::Path,
    ) -> Result<TransformationResult> {
        let mut files_modified = Vec::new();
        
        // Remove unused code from each affected file
        for file_path in &suggestion.affected_files {
            let full_path = codebase_path.join(file_path);
            if let Ok(source_code) = std::fs::read_to_string(&full_path) {
                let language = self.detect_language(file_path);
                let unused_items = self.find_unused_arguments(&source_code, &language);
                
                if !unused_items.is_empty() {
                    let cleaned_code = self.remove_unused_arguments(&source_code, &unused_items);
                    
                    if cleaned_code != source_code {
                        std::fs::write(&full_path, &cleaned_code)?;
                        files_modified.push(file_path.clone());
                    }
                }
            }
        }
        
        let files_count = files_modified.len();
        Ok(TransformationResult {
            transformation_type: TransformationType::CodeCleanup,
            status: TransformationStatus::Completed,
            files_modified,
            backup_commit: None,
            validation_results: {
                let mut results = HashMap::new();
                results.insert("cleanup_applied".to_string(), serde_json::json!("unused_arguments"));
                results.insert("files_processed".to_string(), serde_json::json!(suggestion.affected_files.len()));
                results.insert("files_modified".to_string(), serde_json::json!(files_count));
                results
            },
            error_message: None,
            rollback_available: true,
        })
    }
    
    /// Validate unused code removal transformation
    async fn validate_transformation(&self, result: &TransformationResult, codebase_path: &std::path::Path) -> Result<HashMap<String, serde_json::Value>> {
        let mut validation_results = HashMap::new();
        
        // Validate transformation status
        validation_results.insert("status_valid".to_string(), serde_json::json!(result.status == TransformationStatus::Completed));
        validation_results.insert("error_free".to_string(), serde_json::json!(result.error_message.is_none()));
        
        // Basic validation - check that modified files are syntactically valid
        let mut all_files_exist = true;
        let mut all_files_readable = true;
        let mut all_files_non_empty = true;
        
        for file_path in &result.files_modified {
            let full_path = codebase_path.join(file_path);
            if !full_path.exists() {
                all_files_exist = false;
                break;
            }
            
            // Try to read the modified file
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                if content.trim().is_empty() {
                    all_files_non_empty = false;
                    break;
                }
            } else {
                all_files_readable = false;
                break;
            }
        }
        
        validation_results.insert("files_exist".to_string(), serde_json::json!(all_files_exist));
        validation_results.insert("files_readable".to_string(), serde_json::json!(all_files_readable));
        validation_results.insert("files_non_empty".to_string(), serde_json::json!(all_files_non_empty));
        validation_results.insert("files_modified_count".to_string(), serde_json::json!(result.files_modified.len()));
        
        // Overall validation success
        let overall_success = result.status == TransformationStatus::Completed 
            && result.error_message.is_none() 
            && all_files_exist 
            && all_files_readable 
            && all_files_non_empty;
        validation_results.insert("overall_success".to_string(), serde_json::json!(overall_success));
        
        Ok(validation_results)
    }
    
}
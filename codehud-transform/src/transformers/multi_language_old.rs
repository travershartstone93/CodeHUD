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
use std::process::{Command, Stdio};

/// Multi-language transformer using Comby-style patterns
pub struct MultiLanguageTransformer {
    /// Configuration for multi-language transformations
    config: MultiLanguageConfig,
    /// Pattern library for different languages
    pattern_library: PatternLibrary,
    /// Language detection engine
    language_detector: LanguageDetector,
}

/// Configuration for multi-language transformations
#[derive(Debug, Clone)]
pub struct MultiLanguageConfig {
    /// Whether to use external Comby tool
    pub use_external_comby: bool,
    /// Path to Comby executable
    pub comby_path: Option<String>,
    /// Maximum file size to process
    pub max_file_size: usize,
    /// Whether to validate syntax after transformation
    pub validate_syntax: bool,
    /// Whether to preserve formatting
    pub preserve_formatting: bool,
    /// Language-specific settings
    pub language_settings: HashMap<String, LanguageSettings>,
}

impl Default for MultiLanguageConfig {
    fn default() -> Self {
        let mut language_settings = HashMap::new();
        
        // Python settings
        language_settings.insert("python".to_string(), LanguageSettings {
            indent_style: IndentStyle::Spaces(4),
            line_ending: LineEnding::Unix,
            max_line_length: 88,
            preserve_comments: true,
            syntax_checker: Some("python -m py_compile".to_string()),
        });
        
        // JavaScript settings
        language_settings.insert("javascript".to_string(), LanguageSettings {
            indent_style: IndentStyle::Spaces(2),
            line_ending: LineEnding::Unix,
            max_line_length: 80,
            preserve_comments: true,
            syntax_checker: Some("node --check".to_string()),
        });

        Self {
            use_external_comby: false, // Default to internal implementation
            comby_path: None,
            max_file_size: 10 * 1024 * 1024, // 10MB
            validate_syntax: true,
            preserve_formatting: true,
            language_settings,
        }
    }
}

/// Language-specific settings
#[derive(Debug, Clone)]
pub struct LanguageSettings {
    /// Indentation style
    pub indent_style: IndentStyle,
    /// Line ending style
    pub line_ending: LineEnding,
    /// Maximum line length
    pub max_line_length: usize,
    /// Whether to preserve comments
    pub preserve_comments: bool,
    /// Command to check syntax
    pub syntax_checker: Option<String>,
}

/// Indentation style
#[derive(Debug, Clone)]
pub enum IndentStyle {
    /// Spaces with specified count
    Spaces(usize),
    /// Tabs
    Tabs,
}

/// Line ending style
#[derive(Debug, Clone)]
pub enum LineEnding {
    /// Unix style (\n)
    Unix,
    /// Windows style (\r\n)
    Windows,
    /// Mac style (\r)
    Mac,
}

/// Pattern library for cross-language transformations
#[derive(Debug, Clone)]
pub struct PatternLibrary {
    /// Patterns organized by category
    patterns: HashMap<String, Vec<TransformPattern>>,
}

/// Cross-language transformation pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformPattern {
    /// Pattern name
    pub name: String,
    /// Description of what this pattern does
    pub description: String,
    /// Languages this pattern applies to
    pub languages: Vec<String>,
    /// Source pattern to match
    pub match_pattern: String,
    /// Replacement pattern
    pub replace_pattern: String,
    /// Pattern type (structural, syntactic, semantic)
    pub pattern_type: PatternType,
    /// Examples of transformations
    pub examples: Vec<PatternExample>,
    /// Constraints for when pattern should apply
    pub constraints: Vec<PatternConstraint>,
}

/// Type of transformation pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    /// Structural code patterns (loops, conditionals)
    Structural,
    /// Syntactic patterns (naming, formatting)
    Syntactic,
    /// Semantic patterns (logic, algorithms)
    Semantic,
    /// Cross-language migration patterns
    Migration,
}

/// Example of pattern transformation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternExample {
    /// Example input code
    pub input: String,
    /// Expected output code
    pub output: String,
    /// Language for this example
    pub language: String,
}

/// Constraint for pattern application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternConstraint {
    /// Only apply in specific contexts
    Context(String),
    /// Only apply if certain conditions are met
    Condition(String),
    /// Only apply to specific language versions
    LanguageVersion { language: String, min_version: String },
    /// Only apply if dependencies are present
    Dependency(String),
}

/// Language detection engine
#[derive(Debug, Clone)]
pub struct LanguageDetector {
    /// File extension mappings
    extension_map: HashMap<String, String>,
    /// Content-based detection patterns
    content_patterns: HashMap<String, Regex>,
}

/// Result of applying a pattern
#[derive(Debug, Clone)]
pub struct PatternResult {
    /// Whether pattern was applied successfully
    pub applied: bool,
    /// Number of matches found
    pub matches_found: usize,
    /// Number of replacements made
    pub replacements_made: usize,
    /// Transformed code
    pub transformed_code: String,
    /// Warnings generated
    pub warnings: Vec<String>,
    /// Errors encountered
    pub errors: Vec<String>,
}

impl MultiLanguageTransformer {
    /// Create new multi-language transformer
    pub fn new() -> Result<Self> {
        Ok(Self {
            config: MultiLanguageConfig::default(),
            pattern_library: PatternLibrary::new(),
            language_detector: LanguageDetector::new(),
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: MultiLanguageConfig) -> Self {
        Self {
            config,
            pattern_library: PatternLibrary::new(),
            language_detector: LanguageDetector::new(),
        }
    }

    /// Apply cross-language transformation
    pub async fn transform_cross_language(
        &self,
        source_code: &str,
        source_language: &str,
        target_language: &str,
        pattern_name: &str,
    ) -> Result<PatternResult> {
        // Find appropriate pattern
        let pattern = self.pattern_library.find_pattern(pattern_name, source_language, target_language)
            .ok_or_else(|| TransformError::Config(
                format!("Pattern '{}' not found for {} -> {} transformation", 
                       pattern_name, source_language, target_language)
            ))?;

        // Apply pattern
        self.apply_pattern(source_code, &pattern, target_language).await
    }

    /// Apply a specific pattern to code
    async fn apply_pattern(
        &self,
        source_code: &str,
        pattern: &TransformPattern,
        target_language: &str,
    ) -> Result<PatternResult> {
        if self.config.use_external_comby {
            self.apply_pattern_with_comby(source_code, pattern, target_language).await
        } else {
            self.apply_pattern_internal(source_code, pattern, target_language).await
        }
    }

    /// Apply pattern using external Comby tool
    async fn apply_pattern_with_comby(
        &self,
        source_code: &str,
        pattern: &TransformPattern,
        target_language: &str,
    ) -> Result<PatternResult> {
        let comby_path = self.config.comby_path.as_deref().unwrap_or("comby");

        // Create temporary files for input and pattern
        let temp_dir = std::env::temp_dir();
        let input_file = temp_dir.join("input.tmp");
        let pattern_file = temp_dir.join("pattern.tmp");

        std::fs::write(&input_file, source_code)?;
        std::fs::write(&pattern_file, &pattern.match_pattern)?;

        // Run Comby
        let output = Command::new(comby_path)
            .arg(&pattern.match_pattern)
            .arg(&pattern.replace_pattern)
            .arg(&input_file)
            .arg("-language")
            .arg(target_language)
            .output()?;

        // Clean up temporary files
        let _ = std::fs::remove_file(input_file);
        let _ = std::fs::remove_file(pattern_file);

        if output.status.success() {
            let transformed_code = String::from_utf8_lossy(&output.stdout).to_string();
            let matches_found = self.count_pattern_matches(source_code, &pattern.match_pattern)?;
            
            Ok(PatternResult {
                applied: true,
                matches_found,
                replacements_made: matches_found,
                transformed_code,
                warnings: vec![],
                errors: vec![],
            })
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
            Ok(PatternResult {
                applied: false,
                matches_found: 0,
                replacements_made: 0,
                transformed_code: source_code.to_string(),
                warnings: vec![],
                errors: vec![error_msg],
            })
        }
    }

    /// Apply pattern using internal implementation
    async fn apply_pattern_internal(
        &self,
        source_code: &str,
        pattern: &TransformPattern,
        _target_language: &str,
    ) -> Result<PatternResult> {
        // Convert Comby-style pattern to regex
        let regex_pattern = self.comby_to_regex(&pattern.match_pattern)?;
        let regex = Regex::new(&regex_pattern)?;

        let mut matches_found = 0;
        let mut replacements_made = 0;
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // Apply replacements
        let transformed_code = regex.replace_all(source_code, |caps: &regex::Captures| {
            matches_found += 1;
            
            // Process replacement pattern with captured groups
            let mut replacement = pattern.replace_pattern.clone();
            
            // Replace captured variables (:[var] -> $var)
            for i in 1..caps.len() {
                if let Some(capture) = caps.get(i) {
                    let placeholder = format!(":[{}]", i);
                    replacement = replacement.replace(&placeholder, capture.as_str());
                }
            }
            
            replacements_made += 1;
            replacement
        }).to_string();

        // Validate result if configured
        if self.config.validate_syntax {
            if let Err(e) = self.validate_syntax(&transformed_code, _target_language).await {
                errors.push(format!("Syntax validation failed: {}", e));
            }
        }

        Ok(PatternResult {
            applied: matches_found > 0,
            matches_found,
            replacements_made,
            transformed_code,
            warnings,
            errors,
        })
    }

    /// Convert Comby-style pattern to regex
    fn comby_to_regex(&self, comby_pattern: &str) -> Result<String> {
        let mut regex_pattern = comby_pattern.to_string();
        
        // Replace Comby holes with regex groups
        // :[var] -> (.+?) for any identifier
        // :[var:e] -> (.+?) for expressions
        // :[var:s] -> (.+?) for statements
        let hole_regex = Regex::new(r":\[([^\]]+)\]")?;
        regex_pattern = hole_regex.replace_all(&regex_pattern, "(.+?)").to_string();
        
        // Escape special regex characters except our groups
        regex_pattern = regex::escape(&regex_pattern)
            .replace(r"\(\.\+\?\)", "(.+?)");
        
        Ok(regex_pattern)
    }

    /// Count pattern matches in source code
    fn count_pattern_matches(&self, source_code: &str, pattern: &str) -> Result<usize> {
        let regex_pattern = self.comby_to_regex(pattern)?;
        let regex = Regex::new(&regex_pattern)?;
        Ok(regex.find_iter(source_code).count())
    }

    /// Validate syntax of transformed code
    async fn validate_syntax(&self, code: &str, language: &str) -> Result<()> {
        if let Some(settings) = self.config.language_settings.get(language) {
            if let Some(checker_cmd) = &settings.syntax_checker {
                // Create temporary file
                let temp_file = std::env::temp_dir().join(format!("validate_{}.tmp", uuid::Uuid::new_v4()));
                std::fs::write(&temp_file, code)?;

                // Run syntax checker
                let parts: Vec<&str> = checker_cmd.split_whitespace().collect();
                if !parts.is_empty() {
                    let mut cmd = Command::new(parts[0]);
                    for part in &parts[1..] {
                        cmd.arg(part);
                    }
                    cmd.arg(&temp_file);
                    
                    let output = cmd.output()?;
                    
                    // Clean up
                    let _ = std::fs::remove_file(temp_file);
                    
                    if !output.status.success() {
                        let error_msg = String::from_utf8_lossy(&output.stderr);
                        return Err(TransformError::Transform(format!("Syntax error: {}", error_msg)));
                    }
                }
            }
        }
        Ok(())
    }

    /// Detect language of source code
    pub fn detect_language(&self, source_code: &str, file_path: Option<&str>) -> String {
        self.language_detector.detect(source_code, file_path)
    }

    /// List available patterns
    pub fn list_patterns(&self) -> Vec<&TransformPattern> {
        self.pattern_library.list_all_patterns()
    }

    /// Add custom pattern
    pub fn add_pattern(&mut self, pattern: TransformPattern) {
        self.pattern_library.add_pattern(pattern);
    }
}

impl PatternLibrary {
    /// Create new pattern library with default patterns
    fn new() -> Self {
        let mut library = Self {
            patterns: HashMap::new(),
        };
        
        library.load_default_patterns();
        library
    }

    /// Load default transformation patterns
    fn load_default_patterns(&mut self) {
        // Add common cross-language patterns
        self.add_pattern(TransformPattern {
            name: "for_loop_to_foreach".to_string(),
            description: "Convert traditional for loops to foreach/for-in style".to_string(),
            languages: vec!["python".to_string(), "javascript".to_string(), "java".to_string()],
            match_pattern: "for :[i] in range(len(:[arr])):\n    :[body]".to_string(),
            replace_pattern: "for :[item] in :[arr]:\n    :[body]".to_string(),
            pattern_type: PatternType::Structural,
            examples: vec![
                PatternExample {
                    input: "for i in range(len(items)):\n    print(items[i])".to_string(),
                    output: "for item in items:\n    print(item)".to_string(),
                    language: "python".to_string(),
                }
            ],
            constraints: vec![],
        });

        self.add_pattern(TransformPattern {
            name: "null_check_to_optional".to_string(),
            description: "Convert null checks to optional/maybe patterns".to_string(),
            languages: vec!["java".to_string(), "kotlin".to_string(), "swift".to_string()],
            match_pattern: "if (:[var] != null) {\n    :[body]\n}".to_string(),
            replace_pattern: ":[var].ifPresent(value -> {\n    :[body]\n});".to_string(),
            pattern_type: PatternType::Semantic,
            examples: vec![],
            constraints: vec![],
        });

        self.add_pattern(TransformPattern {
            name: "callback_to_promise".to_string(),
            description: "Convert callback patterns to promises".to_string(),
            languages: vec!["javascript".to_string(), "typescript".to_string()],
            match_pattern: ":[func](:[args], function(:[err], :[result]) {\n    :[body]\n});".to_string(),
            replace_pattern: ":[func](:[args]).then(:[result] => {\n    :[body]\n}).catch(:[err] => {\n    // Handle error\n});".to_string(),
            pattern_type: PatternType::Structural,
            examples: vec![],
            constraints: vec![],
        });
    }

    /// Add a pattern to the library
    fn add_pattern(&mut self, pattern: TransformPattern) {
        let category = match pattern.pattern_type {
            PatternType::Structural => "structural",
            PatternType::Syntactic => "syntactic",
            PatternType::Semantic => "semantic",
            PatternType::Migration => "migration",
        };

        self.patterns.entry(category.to_string())
            .or_insert_with(Vec::new)
            .push(pattern);
    }

    /// Find pattern by name and languages
    fn find_pattern(&self, name: &str, source_lang: &str, target_lang: &str) -> Option<&TransformPattern> {
        for patterns in self.patterns.values() {
            for pattern in patterns {
                if pattern.name == name && 
                   pattern.languages.contains(&source_lang.to_string()) &&
                   pattern.languages.contains(&target_lang.to_string()) {
                    return Some(pattern);
                }
            }
        }
        None
    }

    /// List all patterns
    fn list_all_patterns(&self) -> Vec<&TransformPattern> {
        self.patterns.values()
            .flat_map(|patterns| patterns.iter())
            .collect()
    }
}

impl LanguageDetector {
    /// Create new language detector
    fn new() -> Self {
        let mut extension_map = HashMap::new();
        extension_map.insert("py".to_string(), "python".to_string());
        extension_map.insert("js".to_string(), "javascript".to_string());
        extension_map.insert("ts".to_string(), "typescript".to_string());
        extension_map.insert("java".to_string(), "java".to_string());
        extension_map.insert("cpp".to_string(), "cpp".to_string());
        extension_map.insert("rs".to_string(), "rust".to_string());
        extension_map.insert("go".to_string(), "go".to_string());

        let mut content_patterns = HashMap::new();
        content_patterns.insert("python".to_string(), Regex::new(r"def\s+\w+\s*\(").unwrap());
        content_patterns.insert("javascript".to_string(), Regex::new(r"function\s+\w+\s*\(").unwrap());
        content_patterns.insert("java".to_string(), Regex::new(r"public\s+class\s+\w+").unwrap());

        Self {
            extension_map,
            content_patterns,
        }
    }

    /// Detect language from source code and file path
    fn detect(&self, source_code: &str, file_path: Option<&str>) -> String {
        // Try file extension first
        if let Some(path) = file_path {
            if let Some(extension) = std::path::Path::new(path).extension() {
                if let Some(ext_str) = extension.to_str() {
                    if let Some(language) = self.extension_map.get(ext_str) {
                        return language.clone();
                    }
                }
            }
        }

        // Fall back to content-based detection
        for (language, pattern) in &self.content_patterns {
            if pattern.is_match(source_code) {
                return language.clone();
            }
        }

        "unknown".to_string()
    }
}

#[async_trait]
impl Transformer for MultiLanguageTransformer {
    /// Analyze code for multi-language transformation opportunities
    async fn analyze_transformation_opportunities(
        &self,
        analysis_result: &AnalysisResult,
    ) -> Result<Vec<TransformationSuggestion>> {
        // Detect source language if not specified
        let source_language = if input.language.is_empty() {
            self.detect_language(&input.source_code, Some(&input.file_path))
        } else {
            input.language.clone()
        };

        // Get transformation parameters
        let pattern_name = input.config.parameters.get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let target_language = input.config.parameters.get("target_language")
            .and_then(|v| v.as_str())
            .unwrap_or(&source_language);

        // Apply transformation
        let pattern_result = if pattern_name == "default" {
            // Apply default transformations for the language
            self.apply_default_transformations(&input.source_code, &source_language).await?
        } else {
            // Apply specific pattern
            self.transform_cross_language(&input.source_code, &source_language, target_language, pattern_name).await?
        };

        // Calculate statistics
        let original_lines = input.source_code.lines().count();
        let transformed_lines = pattern_result.transformed_code.lines().count();

        let statistics = TransformationStatistics {
            lines_processed: original_lines,
            lines_modified: pattern_result.replacements_made,
            lines_added: if transformed_lines > original_lines { 
                transformed_lines - original_lines 
            } else { 0 },
            lines_removed: if original_lines > transformed_lines { 
                original_lines - transformed_lines 
            } else { 0 },
            transformations_applied: pattern_result.replacements_made,
            complexity_before: None,
            complexity_after: None,
            issues_fixed: pattern_result.replacements_made,
            issues_introduced: pattern_result.errors.len(),
        };

        Ok(TransformationResult {
            success: pattern_result.applied && pattern_result.errors.is_empty(),
            transformed_code: if pattern_result.applied { 
                Some(pattern_result.transformed_code) 
            } else { 
                None 
            },
            modified_files: if pattern_result.applied { 
                vec![input.file_path.clone()] 
            } else { 
                vec![] 
            },
            created_files: vec![],
            backup_info: None,
            statistics,
            errors: pattern_result.errors,
            warnings: pattern_result.warnings,
            execution_time_ms: 0, // Will be set by engine
        })
    }

    fn supports_dry_run(&self) -> bool {
        true
    }

    fn supports_rollback(&self) -> bool {
        true
    }

    fn estimate_complexity(&self, input: &TransformInput) -> Result<ComplexityEstimate> {
        let lines = input.source_code.lines().count();
        let estimated_duration = (lines as f64 * 0.02).max(0.1); // 0.02 seconds per line
        
        Ok(ComplexityEstimate {
            estimated_duration_seconds: estimated_duration,
            files_to_modify: 1,
            transformation_count: 1,
            risk_level: RiskLevel::Medium, // Cross-language transformations have moderate risk
            confidence: 0.8,
            lines_affected: lines / 5, // Estimate 20% of lines affected
            requires_manual_review: true, // Cross-language changes should be reviewed
        })
    }
}

impl MultiLanguageTransformer {
    /// Apply default transformations for a language
    async fn apply_default_transformations(&self, source_code: &str, language: &str) -> Result<PatternResult> {
        // Find applicable patterns for this language
        let applicable_patterns: Vec<&TransformPattern> = self.pattern_library
            .list_all_patterns()
            .into_iter()
            .filter(|p| p.languages.contains(&language.to_string()))
            .collect();

        if applicable_patterns.is_empty() {
            return Ok(PatternResult {
                applied: false,
                matches_found: 0,
                replacements_made: 0,
                transformed_code: source_code.to_string(),
                warnings: vec![format!("No patterns available for language: {}", language)],
                errors: vec![],
            });
        }

        // Apply first applicable pattern as default
        self.apply_pattern(source_code, applicable_patterns[0], language).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_language_transformer_creation() {
        let transformer = MultiLanguageTransformer::new();
        assert!(transformer.is_ok());
    }

    #[test]
    fn test_language_detection() {
        let transformer = MultiLanguageTransformer::new().unwrap();
        let python_code = "def hello():\n    print('Hello')";
        let language = transformer.detect_language(python_code, Some("test.py"));
        assert_eq!(language, "python");
    }

    #[test]
    fn test_comby_to_regex_conversion() {
        let transformer = MultiLanguageTransformer::new().unwrap();
        let comby_pattern = "for :[i] in range(:[n]):";
        let regex_pattern = transformer.comby_to_regex(comby_pattern);
        assert!(regex_pattern.is_ok());
    }

    #[test]
    fn test_pattern_library() {
        let library = PatternLibrary::new();
        let patterns = library.list_all_patterns();
        assert!(!patterns.is_empty());
    }
}